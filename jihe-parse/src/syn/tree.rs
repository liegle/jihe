pub(crate) struct Tree<'src> {
    pub(super) statements: Vec<Statement<'src>>,
}

pub(crate) struct Statement<'src> {
    pub(super) name: &'src str,
    pub(super) kind: Class<'src>,
}

pub(crate) enum Class<'src> {
    Param {
        f: Number,
        t: Number,
    },
    Var {
        v: Expr<'src>,
    },
    Point {
        x: Expr<'src>,
        y: Expr<'src>,
        thickness: Number,
        color: Color,
    },
    Curve {
        l: Expr<'src>,
        r: Expr<'src>,
        size: Number,
        color: Color,
    },
}

pub(crate) enum Expr<'src> {
    Number(Number),
    Parameter(&'src str),
    VariableX,
    VariableY,
    FunctionCall(&'src str, Vec<Expr<'src>>),
    Parenthese(Box<Expr<'src>>),
    Power(Box<Expr<'src>>, Box<Expr<'src>>),
    Multiply(Box<Expr<'src>>, Box<Expr<'src>>),
    Divide(Box<Expr<'src>>, Box<Expr<'src>>),
    Plus(Box<Expr<'src>>, Box<Expr<'src>>),
    Minus(Box<Expr<'src>>, Box<Expr<'src>>),
    Negative(Box<Expr<'src>>),
}

pub(crate) enum Number {
    Integer(u32),
    Fraction(u32, u32),
}

pub(crate) enum Color {
    Rgb(Number, Number, Number),
    Rgba(Number, Number, Number, Number),
}
