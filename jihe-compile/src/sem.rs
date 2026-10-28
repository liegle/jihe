use std::{collections::HashMap, ops::Range, sync::LazyLock};

use crate::{
    Spanned, Syn,
    sem::{
        error::{FieldStruct, SemError},
        expr::Expr,
    },
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
        let mut statements = Vec::new();
        for tree in syn {
            let (tree, span) = tree.map_err(SemError::SynError)?;
            let class = &source[tree.class.clone()];
            let Some(Construct { named, unnamed }) = CONSTRUCTS.get(class) else {
                return Err(SemError::ClassNotFound {
                    found: class.to_owned(),
                    span: tree.class,
                });
            };
            // 🤣
            let inner = match tree.fields {
                Fields::Named(f) => named(f, span.clone()),
                Fields::Unnamed(f) => unnamed(f, span.clone()),
            }?;
            let id = g.next_id(&source[tree.name]);
            statements.push(Self { id, inner, span });
        }
        Ok(statements)
    }
}

struct Construct {
    named: fn(HashMap<&str, Spanned<Node>>, span: Range<usize>) -> Result<Inner, SemError>,
    unnamed: fn(Vec<Spanned<Node>>, span: Range<usize>) -> Result<Inner, SemError>,
}

// TODO: use std::range::Range
macro_rules! enum_inner {
    {
        $($class:ident {
            $($function:ident($($field:ident),+) = $single:ident $(? [$($default:literal),+])?),+
        }),+
    } => {
        pub enum Inner {
            $($class {
                $($($field: Expr,)+)+
            }),+
        }

        static CONSTRUCTS: LazyLock<HashMap<&'static str, Construct>> = LazyLock::new(|| {
            let mut map = HashMap::new();
            $({
                fn named(
                    mut fields: HashMap<&str, Spanned<Node>>,
                    span: Range<usize>
                ) -> Result<Inner, SemError> {
                    $(let [$($field),+] = {
                        let field = stringify!($single);
                        fields.remove(field).map(|(node, span)| {
                            paste::paste!([<$function:lower:snake>](node))
                                .map_err(|stru| SemError::WrongFieldStruct { field, stru, span })
                        })
                        $(.or(Some(Ok(lit_expr!([$($default),+])))))?
                        .ok_or(SemError::FieldLost { field, span: span.clone() })??
                    };)+
                    Ok(Inner::$class { $($($field,)+)+ })
                }

                fn unnamed(
                    mut fields: Vec<Spanned<Node>>,
                    span: Range<usize>
                ) -> Result<Inner, SemError> {
                    fields.reverse();
                    $(let [$($field),+] = {
                        let field = stringify!($single);
                        fields.pop().map(|(node, span)| {
                            paste::paste!([<$function:lower:snake>](node))
                                .map_err(|stru| SemError::WrongFieldStruct { field, stru, span })
                        })
                        $(.or(Some(Ok(lit_expr!([$($default),+])))))?
                        .ok_or(SemError::FieldLost { field, span: span.clone() })??
                    };)+
                    Ok(Inner::$class { $($($field,)+)+ })
                }

                map.insert(stringify!($class), Construct { named, unnamed });
            })+
            map
        });
    };
}

macro_rules! lit_expr {
    ([$($default:literal),+]) => { [$(Expr::Literal($default)),+] }
}

enum_inner! {
    Slider {
         Ident(from) = from,
         Ident(to) = to
    },
    Curve {
         Eq(expr_l, expr_r) = expr,
         Ident(thickness) = thickness ? [3.],
         Color(r, g, b, a) = color ? [0., 0., 0., 1.]
    }
}

fn ident(node: Node) -> Result<[Expr; 1], FieldStruct> {
    todo!()
}

fn eq(node: Node) -> Result<[Expr; 2], FieldStruct> {
    todo!()
}

fn color(node: Node) -> Result<[Expr; 4], FieldStruct> {
    todo!()
}
