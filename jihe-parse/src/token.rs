use std::range::Range;

pub(super) struct Token {
    kind: Kind,
    pos: Range<usize>,
}

pub(super) enum Kind {
    // Number
    Integer,
    Fraction,
    // Name
    Identifier,
    VariableX,
    VariableY,
    // Punctuation
    BraceL,
    BraceR,
    ParentheseL,
    ParentheseR,
    Power,
    Multiply,
    Divide,
    Plus,
    Minus,
    Equal,
    Comma,
}
