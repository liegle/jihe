#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) struct IntSet(u32);

impl IntSet {
    pub(super) const CAPACITY: u8 = u32::BITS as u8;

    pub(super) fn new() -> Self {
        Self(0)
    }

    pub(super) fn with_values<'a, T: IntoIterator<Item = u8>>(values: T) -> Self {
        Self(
            values
                .into_iter()
                .filter_map(|value| {
                    if value >= Self::CAPACITY {
                        None
                    } else {
                        Some(1 << value)
                    }
                })
                .fold(0, |acc, item| acc | item),
        )
    }

    pub(super) fn is_empty(&self) -> bool {
        self.0 == 0
    }

    pub(super) fn insert(&mut self, value: u8) -> Option<bool> {
        if value >= Self::CAPACITY {
            None
        } else {
            let value = 1 << value;
            if self.0 & value == 0 {
                self.0 |= value;
                Some(true)
            } else {
                Some(false)
            }
        }
    }

    pub(super) fn contains(&self, value: u8) -> Option<bool> {
        if value >= Self::CAPACITY {
            None
        } else {
            Some(self.0 & (1 << value) != 0)
        }
    }
}

impl Extend<u8> for IntSet {
    fn extend<T: IntoIterator<Item = u8>>(&mut self, iter: T) {
        for value in iter {
            self.insert(value);
        }
    }
}

impl<'a> Extend<&'a u8> for IntSet {
    fn extend<T: IntoIterator<Item = &'a u8>>(&mut self, iter: T) {
        for value in iter {
            self.insert(*value);
        }
    }
}

impl IntoIterator for IntSet {
    type Item = u8;
    type IntoIter = IntoIter;

    fn into_iter(self) -> IntoIter {
        IntoIter(self.0, 0)
    }
}

pub(super) struct IntoIter(u32, u8);

impl Iterator for IntoIter {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        while let 0 = self.0 & 1 {
            if self.0 == 0 {
                return None;
            }
            self.0 >>= 1;
            self.1 += 1;
        }
        let result = Some(self.1);
        self.0 >>= 1;
        self.1 += 1;
        result
    }
}

#[cfg(test)]
mod test {
    use std::assert_matches;

    use super::*;

    #[test]
    fn test_set_basic() {
        let mut set = IntSet::new();
        assert_matches!(set.insert(32), None);
        assert_matches!(set.insert(31), Some(true));
        assert_matches!(set.contains(31), Some(true));
        assert_matches!(set.insert(31), Some(false));
        assert_matches!(set.insert(0), Some(true));
        assert_matches!(set.contains(0), Some(true));
    }

    #[test]
    fn test_set_iter() {
        let mut set = IntSet::new();
        set.insert(0);
        set.insert(9);
        set.insert(7);
        set.insert(6);
        set.extend([9, 4, 32]);
        let mut iter = set.into_iter();
        assert_matches!(iter.next(), Some(0));
        assert_matches!(iter.next(), Some(4));
        assert_matches!(iter.next(), Some(6));
        assert_matches!(iter.next(), Some(7));
        assert_matches!(iter.next(), Some(9));
        assert_matches!(iter.next(), None);
    }
}
