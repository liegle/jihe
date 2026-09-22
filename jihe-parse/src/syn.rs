use std::{collections::HashSet, mem};

pub(super) use crate::syn::error::SynError;
use crate::{
    lex::{self, Token},
    syn::tree::{Kind, Statement, Tree},
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
    fn expected(&self) -> HashSet<lex::Kind>;
}

enum StatementStage<'src> {
    None,
    Name(&'src str),
    Colon(&'src str),
    Kind(&'src str, KindStage),
    BraceL(&'src str, KindStage),
    Inner(&'src str, Kind<'src>),
}

impl<'src> Stage<'src, Statement<'src>> for StatementStage<'src> {
    fn step(
        &mut self,
        Token { kind, string }: Token<'src>,
    ) -> Result<Option<Statement<'src>>, SynError> {
        *self = match (mem::replace(self, StatementStage::None), kind) {
            (StatementStage::None, lex::Kind::Identifier) => StatementStage::Name(string),
            (StatementStage::Name(name), lex::Kind::Colon) => StatementStage::Colon(name),
            (StatementStage::Colon(name), lex::Kind::Identifier) => {
                StatementStage::Kind(name, KindStage::from_str(string)?)
            }
            (StatementStage::Kind(name, inner), lex::Kind::BraceL) => {
                StatementStage::BraceL(name, inner)
            }
            (StatementStage::BraceL(name, mut inner), kind) => {
                if let Some(result) = inner.step(Token { kind, string })? {
                    StatementStage::Inner(name, result)
                } else {
                    StatementStage::BraceL(name, inner)
                }
            }
            (StatementStage::Inner(name, kind), lex::Kind::BraceR) => {
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

    fn expected(&self) -> HashSet<lex::Kind> {
        let mut expected = HashSet::new();
        expected.insert(match self {
            StatementStage::None => lex::Kind::Identifier,
            StatementStage::Name(..) => lex::Kind::Colon,
            StatementStage::Colon(..) => lex::Kind::Identifier,
            StatementStage::Kind(..) => lex::Kind::BraceL,
            StatementStage::BraceL(_, inner) => {
                return inner.expected();
            }
            StatementStage::Inner(..) => lex::Kind::BraceR,
        });
        expected
    }
}

enum KindStage {
    Parameter,
    Variable,
    Point,
    Curve,
}

impl KindStage {
    fn from_str(string: &str) -> Result<Self, SynError> {
        match string {
            "Para" => Ok(Self::Parameter),
            "Var" => Ok(Self::Variable),
            "Point" => Ok(Self::Point),
            "Curve" => Ok(Self::Curve),
            _ => Err(SynError::UndefinedStatementType(string.to_owned())),
        }
    }
}

impl<'src> Stage<'src, Kind<'src>> for KindStage {
    fn step(&mut self, token: Token<'src>) -> Result<Option<Kind<'src>>, SynError> {
        // TODO
        Ok(None)
    }

    fn expected(&self) -> HashSet<lex::Kind> {
        HashSet::new()
    }
}
