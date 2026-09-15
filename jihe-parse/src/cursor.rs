use std::fmt::{self, Display, Formatter};

#[derive(Clone, Copy, Debug, Default)]
pub struct Cursor {
    pub(super) line: usize,
    pub(super) col: usize,
}

impl Cursor {
    pub(super) fn step(&mut self, is_line: bool) {
        if is_line {
            self.line += 1;
            self.col = 0;
        } else {
            self.col += 1;
        }
    }
}

impl Display for Cursor {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.line, self.col)
    }
}
