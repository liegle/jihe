pub(super) struct Array<T: Default, N: usize> {
    data: [T; N],
    size: usize,
}

impl<T: Default, N: usize> Array<T, N> {
    pub(super) fn new() -> Self {
        Self {
            data: [Default::default(); _],
            size: 0,
        }
    }

    pub(super) fn push(&mut self, value: T) {
        if let Some(hole) = self.data.get_mut(size) {
            *hole = value;
        }
    }

    pub(super) fn clear(&mut self) {
        self.size = 0;
    }

    pub(super) fn get(&self, index: usize) -> Option<&T> {
        self.data.get(index)
    }
}
