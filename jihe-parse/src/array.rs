//! "Dyn" means half dynamic

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
            self.size = usize::min(N, self.size + 1);
        }
    }

    pub fn clear(&mut self) {
        self.size = 0;
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        if index >= self.size {
            None
        } else {
            self.data.get(index)
        }
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_basic() {
        let mut arr = DynArray::<usize, 10>::new();
        assert_eq!(arr.get(0), None);

        arr.push(1);
        assert_eq!(arr.size, 1);
        assert_eq!(arr.get(0), Some(&1));
        assert_eq!(arr.get(1), None);

        arr.clear();
        assert_eq!(arr.size, 0);
        assert_eq!(arr.get(0), None);
        assert_eq!(arr.get(1), None);
    }

    #[test]
    fn test_index() {
        let mut arr = DynArray::<usize, 10>::new();
        arr.push(1);
        arr.push(2);
        assert_eq!(arr[0], 1);
        assert_eq!(&arr[0..2], &[1, 2]);
        assert_eq!(&arr[0..=1], &[1, 2]);
        assert_eq!(&arr[..], &[1, 2]);
        assert_eq!(&arr[0..], &[1, 2]);
        assert_eq!(&arr[..2], &[1, 2]);
        assert_eq!(&arr[..=1], &[1, 2]);
    }

    #[test]
    #[should_panic]
    fn test_index_panic() {
        let mut arr = DynArray::<usize, 10>::new();
        arr.push(1);
        let _ = arr[1];
    }

    #[test]
    fn test_overflow() {
        let mut arr = DynArray::<usize, 2>::new();
        arr.push(1);
        arr.push(2);
        arr.push(3);
        assert_eq!(arr.size, 2);
        assert_eq!(arr[0], 1);
        assert_eq!(arr[1], 2);
    }
}
