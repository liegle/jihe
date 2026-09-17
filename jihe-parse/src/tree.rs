pub(super) struct Tree<'src> {
    statements: Vec<Statement<'src>>,
}

pub(super) struct Statement<'src> {
    name: &'src str,
    kind: Kind<'src>,
}

pub(super) enum Kind<'src> {
    Point { x: Expr<'src>, y: Expr<'src> },
    Curve { l: Expr<'src>, r: Expr<'src> },
}

pub(super) enum Expr<'src> {
    Integer(u32),
    Fraction(u32, u32),
    Identifier(&'src str),
    VariableX,
    VariableY,
    Function(&'src str, Vec<Expr<'src>>),
    Parenthese(Box<Expr<'src>>),
    Power(Box<Expr<'src>>, Box<Expr<'src>>),
    Multiply(Box<Expr<'src>>, Box<Expr<'src>>),
    Divide(Box<Expr<'src>>, Box<Expr<'src>>),
    Plus(Box<Expr<'src>>, Box<Expr<'src>>),
    Minus(Box<Expr<'src>>, Box<Expr<'src>>),
    Negative(Box<Expr<'src>>),
}
