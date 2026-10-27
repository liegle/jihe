use std::{collections::HashMap, ops::Range, sync::LazyLock};

use crate::{
    Spanned, Syn,
    sem::{error::SemError, expr::Expr},
    syn::{Fields, Node},
};

mod error;
mod expr;

#[derive(Clone, Copy)]
pub struct Id(u32);

pub trait GenId {
    fn next_id(&mut self, name: &str) -> Id;
}

pub struct Statement {
    id: Id,
    inner: Inner,
    span: Range<usize>,
}

impl Statement {
    fn parse_all<G: GenId>(syn: Syn, source: &str, g: &mut G) -> Result<Vec<Self>, SemError> {
        let mut v = Vec::new();
        for tree in syn {
            let (tree, span) = tree.map_err(SemError::SynError)?;
            let class = &source[tree.class.clone()];
            let Some(Construct { named, unnamed }) = CONSTRUCTS.get(class) else {
                return Err(SemError::ClassNotFound {
                    found: class.to_owned(),
                    span: tree.class,
                });
            };
            let inner = match tree.fields {
                Fields::Named(f) => named(f),
                Fields::Unnamed(f) => unnamed(f),
            }?;
            let id = g.next_id(&source[tree.name]);
            v.push(Self { id, inner, span });
        }
        Ok(v)
    }
}

struct Construct {
    named: fn(HashMap<&str, Spanned<Node>>) -> Result<Inner, SemError>,
    unnamed: fn(Vec<Spanned<Node>>) -> Result<Inner, SemError>,
}

static CONSTRUCTS: LazyLock<HashMap<&'static str, Construct>> = LazyLock::new(|| {
    let mut map = HashMap::new();
    map.insert("Slide", todo!());
    map
});

pub enum Inner {
    Slide {
        from: Expr,
        to: Expr,
    },
    Curve {
        expr_l: Expr,
        expr_r: Expr,
        thickness: Expr,
        r: Expr,
        g: Expr,
        b: Expr,
        a: Expr,
    },
}
