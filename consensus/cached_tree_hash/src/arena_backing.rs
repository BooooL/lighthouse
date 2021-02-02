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

#[derive(Default)]
pub struct AnonMmap(Option<MmapMut>);

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
        if let Some(mmap) = &self.0 {
            buf.extend_from_slice(&mmap[..])
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
        Ok(AnonMmap(Some(mmap)))
    }
}

impl ArenaBacking for AnonMmap {
    fn with_capacity(capacity: usize) -> Self {
        if capacity == 0 {
            AnonMmap(None)
        } else {
            AnonMmap(Some(new_non_empty_mmap(capacity)))
        }
    }

    fn len(&self) -> usize {
        self.0
            .as_ref()
            .map(|mmap| mmap.len() / HASHSIZE)
            .unwrap_or(0)
    }

    fn splice_forgetful(&mut self, range: Range<usize>, replace_with: &[Hash256]) {
        let range = bytes_range(range);
        let replace_with_bytes = replace_with.len() * HASHSIZE;

        macro_rules! slices {
            () => {
                (
                    self.0
                        .as_ref()
                        .map(|mmap| &mmap[..range.start])
                        .unwrap_or_else(|| &[]),
                    self.0
                        .as_ref()
                        .map(|mmap| &mmap[range.end..])
                        .unwrap_or_else(|| &[]),
                )
            };
        };

        let apply_middle_bytes = |mmap: &mut MmapMut| {
            dbg!(replace_with.len() * HASHSIZE);
            for (i, hash) in replace_with.iter().enumerate() {
                let start = range.start + i * HASHSIZE;
                let end = range.start + (i + 1) * HASHSIZE;

                assert!(end - start == HASHSIZE);

                mmap[start..end].copy_from_slice(hash.as_bytes());
            }
        };

        let new_len = {
            let slices = slices!();
            slices.0.len() + replace_with_bytes + slices.1.len()
        };

        let mut new_mmap = if new_len == 0 {
            mem::swap(&mut self.0, &mut None);
            return;
        } else if let Some(mmap) = self.0.as_mut() {
            if mmap.len() == new_len {
                apply_middle_bytes(mmap);
                return;
            } else {
                new_non_empty_mmap(new_len)
            }
        } else {
            new_non_empty_mmap(new_len)
        };

        let (start, end) = slices!();

        dbg!(start.len());
        new_mmap[..range.start].copy_from_slice(start);

        apply_middle_bytes(&mut new_mmap);

        dbg!(end.len());
        new_mmap[range.start + replace_with_bytes..].copy_from_slice(end);

        assert!(new_mmap.len() % HASHSIZE == 0);

        mem::swap(&mut self.0, &mut Some(new_mmap));
    }

    fn get(&self, i: usize) -> Option<Hash256> {
        self.0.as_ref().and_then(|mmap| {
            mmap.deref()
                .get(i * HASHSIZE..(i + 1) * HASHSIZE)
                .map(Hash256::from_slice)
        })
    }

    fn get_mut(&mut self, i: usize) -> Option<&mut [u8]> {
        if let Some(mmap) = &mut self.0 {
            mmap.deref_mut().get_mut(i * HASHSIZE..(i + 1) * HASHSIZE)
        } else {
            None
        }
    }

    fn iter_range<'a>(&'a self, range: Range<usize>) -> Box<dyn Iterator<Item = Hash256> + 'a> {
        match &self.0 {
            Some(mmap) => Box::new(
                mmap[bytes_range(range)]
                    .chunks(HASHSIZE)
                    .map(Hash256::from_slice),
            ),
            None => Box::new(iter::empty()),
        }
    }

    fn iter_range_mut<'a>(
        &'a mut self,
        range: Range<usize>,
    ) -> Box<dyn Iterator<Item = &'a mut [u8]> + 'a> {
        match &mut self.0 {
            Some(mmap) => Box::new(mmap[bytes_range(range)].chunks_mut(HASHSIZE)),
            None => Box::new(iter::empty()),
        }
    }
}

fn bytes_range(range: Range<usize>) -> Range<usize> {
    (range.start * HASHSIZE)..(range.end * HASHSIZE)
}

fn new_non_empty_mmap(capacity: usize) -> MmapMut {
    MmapOptions::new().len(capacity).map_anon().expect("FIXME")
}
