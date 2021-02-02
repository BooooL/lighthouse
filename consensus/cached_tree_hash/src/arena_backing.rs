use std::ops::Range;

pub trait ArenaBacking<T> {
    fn with_capacity(capacity: usize) -> Self;

    fn len(&self) -> usize;

    fn splice_forgetful<I: IntoIterator<Item = T>>(&mut self, range: Range<usize>, replace_with: I);

    fn get(&self, i: usize) -> Option<&T>;

    fn get_mut(&mut self, i: usize) -> Option<&mut T>;

    fn iter_range<'a>(&'a self, range: Range<usize>) -> std::slice::Iter<'a, T>;

    fn iter_range_mut<'a>(&'a mut self, range: Range<usize>) -> std::slice::IterMut<'a, T>;
}

impl<T> ArenaBacking<T> for Vec<T> {
    fn with_capacity(capacity: usize) -> Self {
        Vec::with_capacity(capacity)
    }

    fn len(&self) -> usize {
        Vec::len(self)
    }

    fn splice_forgetful<I: IntoIterator<Item = T>>(
        &mut self,
        range: Range<usize>,
        replace_with: I,
    ) {
        self.splice(range, replace_with);
    }

    fn get(&self, i: usize) -> Option<&T> {
        <Vec<T>>::get(self, i)
    }

    fn get_mut(&mut self, i: usize) -> Option<&mut T> {
        Vec::get_mut(self, i)
    }

    fn iter_range<'a>(&'a self, range: Range<usize>) -> std::slice::Iter<'a, T> {
        self[range].iter()
    }

    fn iter_range_mut<'a>(&'a mut self, range: Range<usize>) -> std::slice::IterMut<'a, T> {
        self[range].iter_mut()
    }
}
