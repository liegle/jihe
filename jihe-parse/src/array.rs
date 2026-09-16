use std::ops::{Index, IndexMut, Range};

#[derive(Debug)]
pub struct DynArray<T, const N: usize> {
    data: [T; N],
    size: usize,
}

impl<T: Default + Copy, const N: usize> DynArray<T, N> {
    pub fn new() -> Self {
        Self {
            data: [Default::default(); _],
            size: 0,
        }
    }

    pub fn push(&mut self, value: T) {
        if let Some(hole) = self.data.get_mut(self.size) {
            *hole = value;
        }
    }

    pub fn clear(&mut self) {
        self.size = 0;
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        self.data.get(index)
    }
}

impl<T, I, const N: usize> Index<I> for DynArray<T, N>
where
    [T]: Index<I> + Index<Range<usize>, Output = [T]>,
{
    type Output = <[T] as Index<I>>::Output;

    fn index(&self, index: I) -> &Self::Output {
        Index::index(&self.data[0..self.size], index)
    }
}

impl<T, I, const N: usize> IndexMut<I> for DynArray<T, N>
where
    [T]: IndexMut<I> + IndexMut<Range<usize>, Output = [T]>,
{
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        IndexMut::index_mut(&mut self.data[0..self.size], index)
    }
}
