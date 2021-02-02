use crate::Hash256;
use memmap::{MmapMut, MmapOptions};
use ssz::{Decode, Encode};
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

struct AnonMmap(MmapMut);

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
        buf.extend_from_slice(&self.0[..])
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
        let mut s = AnonMmap::with_capacity(bytes.len());
        s.0[..].copy_from_slice(bytes);
        Ok(s)
    }
}

impl ArenaBacking for AnonMmap {
    fn with_capacity(capacity: usize) -> Self {
        AnonMmap(MmapOptions::new().len(capacity).map_anon().expect("FIXME"))
    }

    fn len(&self) -> usize {
        self.0.deref().len() / HASHSIZE
    }

    fn splice_forgetful(&mut self, range: Range<usize>, replace_with: &[Hash256]) {
        let old_span = range.end.saturating_sub(range.start) * HASHSIZE;
        let new_span = replace_with.len() * HASHSIZE;

        let new_len = if new_span > old_span {
            self.len() + (new_span - old_span)
        } else {
            self.len() - (old_span - new_span)
        };

        if new_len == self.len() {
            for i in range.clone() {
                let start = i * HASHSIZE;
                let end = (i + 1) * HASHSIZE;
                self.0[start..end].copy_from_slice(replace_with[i].as_bytes());
            }
        } else {
            let mut new = Self::with_capacity(new_len);

            new.0[0..range.start].copy_from_slice(&self.0[0..range.start * HASHSIZE]);

            for i in range.clone() {
                let start = i * HASHSIZE;
                let end = (i + 1) * HASHSIZE;
                new.0[start..end].copy_from_slice(replace_with[i].as_bytes());
            }

            new.0[range.end..].copy_from_slice(&self.0[range.start * HASHSIZE..]);

            std::mem::swap(self, &mut new);
        }
    }

    fn get(&self, i: usize) -> Option<Hash256> {
        self.0
            .deref()
            .get(i * HASHSIZE..(i + 1) * HASHSIZE)
            .map(Hash256::from_slice)
    }

    fn get_mut(&mut self, i: usize) -> Option<&mut [u8]> {
        self.0.deref_mut().get_mut(i * HASHSIZE..(i + 1) * HASHSIZE)
    }

    fn iter_range<'a>(&'a self, range: Range<usize>) -> Box<dyn Iterator<Item = Hash256> + 'a> {
        Box::new(self.0[range].chunks(HASHSIZE).map(Hash256::from_slice))
    }

    fn iter_range_mut<'a>(
        &'a mut self,
        range: Range<usize>,
    ) -> Box<dyn Iterator<Item = &'a mut [u8]> + 'a> {
        let iter = self.0[range.start * HASHSIZE..range.end * HASHSIZE].chunks_mut(HASHSIZE);
        Box::new(iter)
    }
}
