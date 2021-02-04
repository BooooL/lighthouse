use crate::Hash256;
use memmap::{MmapMut, MmapOptions};
use ssz::{Decode, Encode};
use std::iter;
use std::mem;
use std::ops::{Deref, DerefMut, Range};
use tree_hash::HASHSIZE;

pub trait ArenaBacking: Encode + Decode {
    fn with_capacity(capacity: usize) -> Self;

    fn len(&self) -> usize;

    fn extend_capacity(&mut self, capacity: usize);

    fn splice_forgetful(&mut self, range: Range<usize>, replace_with: &[Hash256]);

    fn get(&self, i: usize) -> Option<Hash256>;

    fn get_mut(&mut self, i: usize) -> Option<&mut [u8]>;

    fn iter_range<'a>(&'a self, range: Range<usize>) -> Box<dyn Iterator<Item = Hash256> + 'a>;

    fn iter_range_mut<'a>(
        &'a mut self,
        range: Range<usize>,
    ) -> Box<dyn Iterator<Item = &'a mut [u8]> + 'a>;
}

impl ArenaBacking for Vec<Hash256> {
    fn with_capacity(capacity: usize) -> Self {
        Vec::with_capacity(capacity)
    }

    fn len(&self) -> usize {
        Vec::len(self)
    }

    fn extend_capacity(&mut self, capacity: usize) {
        if let Some(additional) = capacity.checked_sub(self.capacity()) {
            self.reserve(additional)
        }
    }

    fn splice_forgetful(&mut self, range: Range<usize>, replace_with: &[Hash256]) {
        self.splice(range, replace_with.iter().copied().collect::<Vec<_>>());
    }

    fn get(&self, i: usize) -> Option<Hash256> {
        self.deref().get(i).copied()
    }

    fn get_mut(&mut self, i: usize) -> Option<&mut [u8]> {
        self.deref_mut().get_mut(i).map(|s| s.as_bytes_mut())
    }

    fn iter_range<'a>(&'a self, range: Range<usize>) -> Box<dyn Iterator<Item = Hash256> + 'a> {
        Box::new(self[range].iter().copied())
    }

    fn iter_range_mut<'a>(
        &'a mut self,
        range: Range<usize>,
    ) -> Box<dyn Iterator<Item = &'a mut [u8]> + 'a> {
        Box::new(self[range].iter_mut().map(Hash256::as_bytes_mut))
    }
}

#[derive(Default, Debug)]
pub struct AnonMmap {
    mmap: Option<MmapMut>,
    len: usize,
}

impl AnonMmap {
    pub fn capacity_bytes(&self) -> usize {
        self.mmap.as_ref().map_or(0, |mmap| mmap.len())
    }

    pub fn len_bytes(&self) -> usize {
        self.len
    }
}

impl PartialEq for AnonMmap {
    fn eq(&self, other: &AnonMmap) -> bool {
        match (&self.mmap, &other.mmap) {
            (Some(a), Some(b)) if a[..self.len] == b[..other.len] => true,
            (None, None) => true,
            _ => false,
        }
    }
}

impl Clone for AnonMmap {
    fn clone(&self) -> Self {
        match &self.mmap {
            Some(mmap) => {
                let mut clone = new_non_empty_mmap(mmap.len());
                clone.copy_from_slice(&mmap[..]);
                AnonMmap {
                    mmap: Some(clone),
                    len: mmap.len(),
                }
            }
            None => Self::default(),
        }
    }
}

impl Encode for AnonMmap {
    fn is_ssz_fixed_len() -> bool {
        <Vec<u8> as Encode>::is_ssz_fixed_len()
    }

    fn ssz_fixed_len() -> usize {
        <Vec<u8> as Encode>::ssz_fixed_len()
    }

    fn ssz_bytes_len(&self) -> usize {
        self.len()
    }

    fn ssz_append(&self, buf: &mut Vec<u8>) {
        if let Some(mmap) = &self.mmap {
            buf.extend_from_slice(&mmap[..mmap.len()])
        }
    }
}

impl Decode for AnonMmap {
    fn is_ssz_fixed_len() -> bool {
        <Vec<u8> as Decode>::is_ssz_fixed_len()
    }

    fn ssz_fixed_len() -> usize {
        <Vec<u8> as Decode>::ssz_fixed_len()
    }

    fn from_ssz_bytes(bytes: &[u8]) -> Result<Self, ssz::DecodeError> {
        let mut mmap = new_non_empty_mmap(bytes.len());
        mmap[..].copy_from_slice(bytes);
        Ok(AnonMmap {
            mmap: Some(mmap),
            len: bytes.len(),
        })
    }
}

impl ArenaBacking for AnonMmap {
    fn with_capacity(capacity: usize) -> Self {
        let len = capacity * HASHSIZE;

        let mmap = if capacity == 0 {
            None
        } else {
            Some(new_non_empty_mmap(len))
        };

        Self { mmap, len }
    }

    fn len(&self) -> usize {
        self.len / HASHSIZE
    }

    fn extend_capacity(&mut self, capacity: usize) {
        let capacity = capacity * HASHSIZE;

        if let Some(mmap) = self.mmap.as_mut() {
            if capacity > mmap.len() {
                let mut new_mmap = new_non_empty_mmap(capacity);
                new_mmap[0..self.len].copy_from_slice(&mmap[0..self.len]);

                mem::swap(&mut self.mmap, &mut Some(new_mmap));
            }
        }
    }

    fn splice_forgetful(&mut self, range: Range<usize>, replace_with: &[Hash256]) {
        let range = bytes_range(range);
        let range_bytes = range
            .start
            .checked_sub(range.end)
            .expect("start of range greater then end");
        let replace_with_bytes = replace_with.len() * HASHSIZE;
        let old_len = self.len;

        // Determine the new length of `self`, after the splice.
        let new_len = if replace_with_bytes > range_bytes {
            self.len + (replace_with_bytes - range_bytes)
        } else {
            self.len - (range_bytes - replace_with_bytes)
        };

        macro_rules! new_with_start_and_end_bytes {
            () => {{
                let mut new = new_non_empty_mmap(new_len);
                if let Some(old) = &self.mmap {
                    new[..range.start].copy_from_slice(&old[..range.start]);
                    new[range.start + replace_with_bytes..].copy_from_slice(&old[range.end..]);
                }
                new
            }};
        };

        if new_len == 0 {
            self.len = 0;
            mem::swap(&mut self.mmap, &mut None);
            return;
        }

        // If the existing mmap is large enough, use it. Otherwise, create a new one.
        let mut mmap = if new_len > self.capacity_bytes() {
            new_with_start_and_end_bytes!()
        } else {
            let mmap = mem::replace(&mut self.mmap, None);
            if let Some(mut mmap) = mmap {
                // Shift/copy the end bytes to the right, if required.
                let first_end_byte = range.start + replace_with_bytes;
                if range.end != first_end_byte && first_end_byte < mmap.len() {
                    mmap.as_mut()
                        .copy_within(range.end..old_len, first_end_byte);
                }
                mmap
            } else {
                new_with_start_and_end_bytes!()
            }
        };

        for (i, hash) in replace_with.iter().enumerate() {
            let start = range.start + i * HASHSIZE;
            let end = start + HASHSIZE;

            assert!(end - start == HASHSIZE);

            mmap[start..end].copy_from_slice(hash.as_bytes());
        }

        self.len = new_len;
        mem::swap(&mut self.mmap, &mut Some(mmap))

        /*
        let range = bytes_range(range);
        assert!(range.end <= self.len, "range.end out of bounds");

        let replace_with_bytes = replace_with.len() * HASHSIZE;

        if let Some(mmap) = &self.mmap {
            assert_eq!(mmap.len() % HASHSIZE, 0, "existing mmap");
        }

        macro_rules! slices {
            () => {{
                let start = self
                    .mmap
                    .as_ref()
                    .and_then(|mmap| mmap.get(..range.start))
                    .unwrap_or_else(|| &[]);
                let end = self
                    .mmap
                    .as_ref()
                    .and_then(|mmap| mmap.get(range.end..))
                    .unwrap_or_else(|| &[]);

                (start, end)
            }};
        };

        let apply_middle_bytes = |mmap: &mut MmapMut| {
            for (i, hash) in replace_with.iter().enumerate() {
                let start = range.start + i * HASHSIZE;
                let end = start + HASHSIZE;

                assert!(end - start == HASHSIZE);

                mmap[start..end].copy_from_slice(hash.as_bytes());
            }
        };

        let new_len = {
            let slices = slices!();
            assert_eq!(slices.0.len() % HASHSIZE, 0, "slices.0");
            assert_eq!(slices.1.len() % HASHSIZE, 0, "slices.1");
            slices.0.len() + replace_with_bytes + slices.1.len()
        };

        self.len = new_len;

        assert_eq!(new_len % HASHSIZE, 0, "new_len");

        let mut new_mmap = if new_len == 0 {
            mem::swap(&mut self.mmap, &mut None);
            return;
        } else if let Some(mmap) = self.mmap.as_mut() {
            if self.len() == new_len {
                apply_middle_bytes(mmap);
                return;
            } else if new_len <= self.capacity() {
                let (start, end) = slices!();

                mmap[..range.start].copy_from_slice(start);

                apply_middle_bytes(&mut mmap);

                mmap[range.start + replace_with_bytes..].copy_from_slice(end);

                return;
            } else {
                new_non_empty_mmap(new_len)
            }
        } else {
            new_non_empty_mmap(new_len)
        };

        let (start, end) = slices!();

        new_mmap[..range.start].copy_from_slice(start);

        apply_middle_bytes(&mut new_mmap);

        new_mmap[range.start + replace_with_bytes..].copy_from_slice(end);

        assert_eq!(new_mmap.len() % HASHSIZE, 0);

        mem::swap(&mut self.mmap, &mut Some(new_mmap));
        */
    }

    fn get(&self, i: usize) -> Option<Hash256> {
        self.mmap.as_ref().and_then(|mmap| {
            mmap.deref()
                .get(i * HASHSIZE..(i + 1) * HASHSIZE)
                .map(Hash256::from_slice)
        })
    }

    fn get_mut(&mut self, i: usize) -> Option<&mut [u8]> {
        if let Some(mmap) = &mut self.mmap {
            mmap.deref_mut().get_mut(i * HASHSIZE..(i + 1) * HASHSIZE)
        } else {
            None
        }
    }

    fn iter_range<'a>(&'a self, range: Range<usize>) -> Box<dyn Iterator<Item = Hash256> + 'a> {
        let range = bytes_range(range);
        assert!(range.end <= self.len, "range.end out of bounds");

        match &self.mmap {
            Some(mmap) => Box::new(mmap[range].chunks(HASHSIZE).map(Hash256::from_slice)),
            None => Box::new(iter::empty()),
        }
    }

    fn iter_range_mut<'a>(
        &'a mut self,
        range: Range<usize>,
    ) -> Box<dyn Iterator<Item = &'a mut [u8]> + 'a> {
        let range = bytes_range(range);
        assert!(range.end <= self.len, "range.end out of bounds");

        match &mut self.mmap {
            Some(mmap) => Box::new(mmap[range].chunks_mut(HASHSIZE)),
            None => Box::new(iter::empty()),
        }
    }
}

fn bytes_range(range: Range<usize>) -> Range<usize> {
    (range.start * HASHSIZE)..(range.end * HASHSIZE)
}

fn new_non_empty_mmap(capacity: usize) -> MmapMut {
    println!("new mmap");
    MmapOptions::new().len(capacity).map_anon().expect("FIXME")
}
