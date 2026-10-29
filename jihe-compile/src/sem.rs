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

pub trait Table {
    fn register(&mut self, name: &str, dep: Dep) -> Result<Id, (Range<usize>, Dep)>;
    fn depend(&mut self, name: &str, dep: Dep) -> Result<Id, (Range<usize>, Dep)>;
}

#[derive(Clone, Copy)]
pub enum Dep {
    Not,
    Var,
    Fn,
}

pub struct Statement {
    id: Id,
    class: Class,
    span: Range<usize>,
}

impl Statement {
    fn parse_all<T: Table>(syn: Syn, source: &str, table: &mut T) -> Result<Vec<Self>, SemError> {
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
            let class = match tree.fields {
                Fields::Named(f) => named(f, span.clone()),
                Fields::Unnamed(f) => unnamed(f, span.clone()),
            }?;
            let name = &source[tree.name];
            let dep = class.dep();
            let id = match table.register(name, class.dep()) {
                Ok(id) => id,
                Err((prev_span, prev_dep)) => {
                    return Err(SemError::WrongDependenceType {
                        name: name.to_owned(),
                        prev_span,
                        prev_dep,
                        span,
                        dep,
                    });
                }
            };
            statements.push(Self { id, class, span });
        }
        Ok(statements)
    }
}

struct Construct {
    named: fn(HashMap<&str, Spanned<Node>>, span: Range<usize>) -> Result<Class, SemError>,
    unnamed: fn(Vec<Spanned<Node>>, span: Range<usize>) -> Result<Class, SemError>,
}

// TODO: use std::range::Range
macro_rules! enum_class {
    {
        $($class:ident: $dep:ident {
            $($field:ident: $combine:ident $(= [$($default:literal),+])?),+
        }),+
    } => {
        pub enum Class {
            $($class {
                $($field: $combine,)+
            }),+
        }

        impl Class {
            pub fn dep(&self) -> Dep {
                match self {
                    $(Self::$class { .. } => Dep::$dep),+
                }
            }
        }

        static CONSTRUCTS: LazyLock<HashMap<&'static str, Construct>> = LazyLock::new(|| {
            let mut map = HashMap::new();
            $({
                fn named(
                    mut fields: HashMap<&str, Spanned<Node>>,
                    span: Range<usize>
                ) -> Result<Class, SemError> {
                    $(let $field = {
                        let field = stringify!($field);
                        if let Some(field) = fields.remove(field)
                            .map(|(node, span)| {
                                $combine::from_node(node)
                                    .ok_or(SemError::WrongFieldDefinition {
                                        field,
                                        def: $combine::FIELD_DEFINITION,
                                        span
                                    })
                            })
                            $(.or(Some(Ok(lit_expr!($combine, $($default),+)))))? {
                            field?
                        } else {
                            return Err(SemError::FieldLost { field, span });
                        }
                    };)+
                    if let Some((_, (_, field_span))) = fields.iter().next() {
                        return Err(SemError::RedundantField { span: field_span.clone() });
                    }
                    Ok(Class::$class { $($field),+ })
                }

                fn unnamed(
                    mut fields: Vec<Spanned<Node>>,
                    span: Range<usize>
                ) -> Result<Class, SemError> {
                    fields.reverse();
                    $(let $field = {
                        let field = stringify!($field);
                        if let Some(field) = fields.pop()
                            .map(|(node, span)| {
                                $combine::from_node(node)
                                    .ok_or(SemError::WrongFieldDefinition {
                                        field,
                                        def: $combine::FIELD_DEFINITION,
                                        span
                                    })
                            })
                            $(.or(Some(Ok(lit_expr!($combine, $($default),+)))))? {
                            field?
                        } else {
                            return Err(SemError::FieldLost { field, span });
                        }
                    };)+
                    if let Some((_, span)) = fields.get(0) {
                        return Err(SemError::RedundantField { span: span.clone() });
                    }
                    Ok(Class::$class { $($field),+ })
                }

                map.insert(stringify!($class), Construct { named, unnamed });
            })+
            map
        });
    };
}

macro_rules! lit_expr {
    ($combine:ident, $($default:literal),+) => { $combine($(Expr::Literal($default)),+) }
}

enum_class! {
    Slider: Var {
        from: Single,
        to: Single
    },
    Bind: Var {
        expr: Single
    },
    Fn: Fn {
        expr: Fn
    },
    Point: Not {
        x: Single,
        y: Single,
        size: Single = [3.],
        color: Color = [0., 0., 0., 1.]
    },
    Curve: Not {
        expr: Eq,
        thickness: Single = [3.],
        color: Color = [0., 0., 0., 1.]
    }
}

trait FromNode: Sized {
    const FIELD_DEFINITION: &'static str;
    fn from_node(node: Node) -> Option<Self>;
}

pub struct Single(Expr);

impl FromNode for Single {
    const FIELD_DEFINITION: &'static str = "Any single expr without \"=\" or \"fn\" inside";

    fn from_node(node: Node) -> Option<Self> {
        todo!()
    }
}

pub struct Eq(Expr, Expr);

impl FromNode for Eq {
    const FIELD_DEFINITION: &'static str = "Two single exprs with \"=\" between";

    fn from_node(node: Node) -> Option<Self> {
        todo!()
    }
}

pub struct Fn {
    args: Vec<Id>,
    expr: Expr,
}

impl FromNode for Fn {
    const FIELD_DEFINITION: &'static str =
        "\"fn(...) = ...\" with only identifiers in fn() and single expr on the right";

    fn from_node(node: Node) -> Option<Self> {
        todo!()
    }
}

pub struct Color(Expr, Expr, Expr, Expr);

impl FromNode for Color {
    const FIELD_DEFINITION: &'static str =
        "3 or 4 single exprs representing r, g, b and a channels of the color";

    fn from_node(node: Node) -> Option<Self> {
        todo!()
    }
}
