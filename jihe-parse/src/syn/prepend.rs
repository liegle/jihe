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
        self.pre.pop().or_else(|| self.iter.next())
    }

    // TODO: maybe reimpl fns like Peekable does?
}

#[cfg(test)]
mod test {
    use std::assert_matches;

    use super::*;

    #[test]
    fn test_basic() {
        let mut p = Prependable::from_iter(0..5);
        assert_matches!(p.next(), Some(0));
        assert_matches!(p.next(), Some(1));
        assert_matches!(p.next(), Some(2));
        assert_matches!(p.next(), Some(3));
        assert_matches!(p.next(), Some(4));
        assert_matches!(p.next(), None);
    }

    #[test]
    fn test_prepend() {
        let mut p = Prependable::from_iter(0..5);
        assert_matches!(p.next(), Some(0));
        assert_matches!(p.next(), Some(1));

        p.prepend(100);
        p.prepend(200);
        p.prepend(300);
        assert_matches!(p.next(), Some(300));
        assert_matches!(p.next(), Some(200));
        assert_matches!(p.next(), Some(100));
        assert_matches!(p.next(), Some(2));
        assert_matches!(p.next(), Some(3));
        assert_matches!(p.next(), Some(4));
        assert_matches!(p.next(), None);
    }

    #[test]
    fn test_next_if() {
        let mut p = Prependable::from_iter(0..5);
        assert_matches!(p.next_if(|v| *v > 0), None);
        assert_matches!(p.next_if(|v| *v < 0), None);
        assert_matches!(p.next_if(|v| *v == 0), Some(0));
        assert_matches!(p.next_if(|v| *v == 1), Some(1));
    }
}
