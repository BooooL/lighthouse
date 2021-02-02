use crate::Hash256;
use memmap::{MmapMut, MmapOptions};
use ssz::{Decode, Encode};
use std::ops::{Deref, DerefMut, Range};

pub trait ArenaBacking: Encode + Decode {
    fn with_capacity(capacity: usize) -> Self;

    fn len(&self) -> usize;

    fn splice_forgetful<I: IntoIterator<Item = Hash256>>(
        &mut self,
        range: Range<usize>,
        replace_with: I,
    );

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

    fn splice_forgetful<I: IntoIterator<Item = Hash256>>(
        &mut self,
        range: Range<usize>,
        replace_with: I,
    ) {
        self.splice(range, replace_with);
    }

    fn get(&self, i: usize) -> Option<Hash256> {
        todo!();
    }

    fn get_mut(&mut self, i: usize) -> Option<&mut [u8]> {
        todo!();
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
