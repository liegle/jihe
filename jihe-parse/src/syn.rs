use std::{collections::HashSet, mem};

pub(super) use crate::syn::error::SynError;
use crate::{
    lex::{Kind, Token}, syn::tree::{Class, Statement, Tree},
};

mod error;
mod tree;

pub(super) struct Syn<'src> {
    tree: Tree<'src>,
    stage: StatementStage<'src>,
}

impl<'src> Syn<'src> {
    pub(super) fn new() -> Self {
        Self {
            tree: Tree { statements: Vec::new() },
            stage: StatementStage::None,
        }
    }

    pub(super) fn input(&mut self, token: Token<'src>) -> Result<(), SynError> {
        if let Some(statement) = self.stage.step(token)? {
            self.tree.statements.push(statement);
        }
        Ok(())
    }

    pub(super) fn output(self) -> Result<Tree<'src>, SynError> {
        // TODO
        Ok(self.tree)
    }
}

trait Stage<'src, T: 'src> {
    fn step(&mut self, token: Token<'src>) -> Result<Option<T>, SynError>;
    fn expected(&self) -> HashSet<Kind>;
}

enum StatementStage<'src> {
    None,
    Name(&'src str),
    Colon(&'src str),
    Kind(&'src str, ClassStage),
    BraceL(&'src str, ClassStage),
    Inner(&'src str, Class<'src>),
}

impl<'src> Stage<'src, Statement<'src>> for StatementStage<'src> {
    fn step(
        &mut self,
        Token { kind, string }: Token<'src>,
    ) -> Result<Option<Statement<'src>>, SynError> {
        *self = match (mem::replace(self, StatementStage::None), kind) {
            (StatementStage::None, Kind::Identifier) => StatementStage::Name(string),
            (StatementStage::Name(name), Kind::Colon) => StatementStage::Colon(name),
            (StatementStage::Colon(name), Kind::Identifier) => {
                StatementStage::Kind(name, ClassStage::from_str(string)?)
            }
            (StatementStage::Kind(name, inner), Kind::BraceL) => {
                StatementStage::BraceL(name, inner)
            }
            (StatementStage::BraceL(name, mut inner), kind) => {
                if let Some(result) = inner.step(Token { kind, string })? {
                    StatementStage::Inner(name, result)
                } else {
                    StatementStage::BraceL(name, inner)
                }
            }
            (StatementStage::Inner(name, kind), Kind::BraceR) => {
                return Ok(Some(Statement { name, kind }));
            }
            (stage, _) => {
                return Err(SynError::UnexpectedToken {
                    expected: stage.expected(),
                    found: string.to_owned(),
                });
            }
        };
        Ok(None)
    }

    fn expected(&self) -> HashSet<Kind> {
        let mut expected = HashSet::new();
        expected.insert(match self {
            StatementStage::None => Kind::Identifier,
            StatementStage::Name(..) => Kind::Colon,
            StatementStage::Colon(..) => Kind::Identifier,
            StatementStage::Kind(..) => Kind::BraceL,
            StatementStage::BraceL(_, inner) => {
                return inner.expected();
            }
            StatementStage::Inner(..) => Kind::BraceR,
        });
        expected
    }
}

enum ClassStage {
    Param,
    Var,
    Point,
    Curve,
}

impl ClassStage {
    fn from_str(string: &str) -> Result<Self, SynError> {
        match string {
            "Param" => Ok(Self::Param),
            "Var" => Ok(Self::Var),
            "Point" => Ok(Self::Point),
            "Curve" => Ok(Self::Curve),
            _ => Err(SynError::UndefinedStatementKind { found: string.to_owned() }),
        }
    }
}

impl<'src> Stage<'src, Class<'src>> for ClassStage {
    fn step(&mut self, token: Token<'src>) -> Result<Option<Class<'src>>, SynError> {
        // TODO
        Ok(None)
    }

    fn expected(&self) -> HashSet<Kind> {
        HashSet::new()
    }
}

enum ClassVarietyStage<const N: usize> {
    None,
}
