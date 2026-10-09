pub(super) struct Prependable<I: Iterator> {
    pre: Vec<<I as Iterator>::Item>,
    iter: I,
}

impl<I: Iterator> Prependable<I> {
    pub(super) fn from_iter(iter: I) -> Self {
        Self { pre: Vec::new(), iter }
    }

    pub(super) fn inner(&self) -> &I {
        &self.iter
    }

    pub(super) fn prepend(&mut self, value: <I as Iterator>::Item) {
        self.pre.push(value);
    }

    pub(super) fn next_if(
        &mut self,
        func: impl FnOnce(&<I as Iterator>::Item) -> bool,
    ) -> Option<<I as Iterator>::Item> {
        match self.next() {
            Some(matched) if func(&matched) => Some(matched),
            Some(other) => {
                assert!(self.pre.is_empty());
                self.pre.push(other);
                None
            }
            _ => None,
        }
    }
}

impl<I: Iterator> Iterator for Prependable<I> {
    type Item = <I as Iterator>::Item;

    fn next(&mut self) -> Option<Self::Item> {
        match self.pre.len() {
            0 => self.iter.next(),
            _ => self.pre.pop(),
        }
    }

    // TODO: maybe reimpl fns like Peekable does?
}
