use crate::{
    Lex,
    lex::{Kind, KindSet, Token},
    syn::{ExpectToken, Prependable, Syn, SynError},
};

#[derive(Clone)]
pub(crate) enum Expr<'src> {
    Num { integer: u32, decimal: u32 },
    Param(&'src str),
    Fn(&'src str, Vec<Expr<'src>>),
    VarX,
    VarY,
    Paren(Box<Expr<'src>>),
    Neg(Box<Expr<'src>>),
    Pow(Box<Expr<'src>>, Box<Expr<'src>>),
    Mul(Box<Expr<'src>>, Box<Expr<'src>>),
    Div(Box<Expr<'src>>, Box<Expr<'src>>),
    Add(Box<Expr<'src>>, Box<Expr<'src>>),
    Sub(Box<Expr<'src>>, Box<Expr<'src>>),
    Eq(Box<Expr<'src>>, Box<Expr<'src>>),
}

impl<'src> Syn<'src> for Expr<'src> {
    fn parse(lex: &mut Prependable<Lex<'src>>) -> Result<Self, SynError> {
        let accu = Self::unit(lex, true)?;
        loop {
            match lex.next_token()? {
                Token {
                    kind: Kind::BraceR | Kind::ParenR | Kind::Comma,
                    ..
                } => return Ok(accu),
                token => lex.prepend(Ok(token)),
            }
            let mid = lex.next_token()?;
            todo!()
        }
    }
}

impl<'src> Expr<'src> {
    fn unit(lex: &mut Prependable<Lex<'src>>, is_start: bool) -> Result<Self, SynError> {
        let token = lex.next_token()?;
        match token.kind {
            Kind::Num => {
                let (integer, decimal) = num(token.string)?;
                Ok(Self::Num { integer, decimal })
            }
            Kind::Ident => {
                todo!("Param or Fn")
            }
            Kind::VarX => Ok(Self::VarX),
            Kind::VarY => Ok(Self::VarY),
            Kind::ParenL => {
                let inner = Self::parse(lex)?;
                let _ = lex.next_kind(Kind::ParenR)?;
                Ok(Self::Paren(Box::new(inner)))
            }
            Kind::Sub if is_start => {
                let inner = Self::parse(lex)?;
                Ok(Self::Neg(Box::new(inner)))
            }
            Kind::Sub => Err(SynError::NegInsideExpr { cursor: token.range.start }),
            _ => Err(SynError::UnexpectedToken {
                expected: KindSet::with_values([]),
                found: token.string.to_owned(),
                range: token.range,
            }),
        }
    }
}

fn num<'src>(string: &'src str) -> Result<(u32, u32), SynError> {
    todo!()
}
