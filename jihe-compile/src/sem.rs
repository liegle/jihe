use std::{
    collections::{HashMap, hash_map::Entry},
    range::Range,
    sync::LazyLock,
};

use crate::{
    Spanned, Syn,
    sem::{error::SemError, expr::Expr},
    syn::{self, Fields, Node},
};

mod error;
mod expr;

#[derive(Clone, Copy)]
pub struct Id(u32);

enum RegisterError {
    Dep(Range<usize>, Dep),
    Dup(Range<usize>),
}

pub struct GlobalTable<'src> {
    source: &'src str,
    symbols: HashMap<String, (Dep, Id, Range<usize>, bool)>,
    prev_id: u32,
}

impl<'src> GlobalTable<'src> {
    fn register(&mut self, name: Range<usize>, dep: Dep) -> Result<Id, RegisterError> {
        if let Some((d, id, span, registered)) = self.symbols.get_mut(&self.source[name]) {
            if *d == dep {
                if !*registered {
                    *span = name;
                    *registered = true;
                    Ok(*id)
                } else {
                    Err(RegisterError::Dup(*span))
                }
            } else {
                Err(RegisterError::Dep(*span, *d))
            }
        } else {
            self.prev_id += 1;
            let id = self.prev_id;
            self.symbols
                .insert(self.source[name].to_owned(), (dep, Id(id), name, true));
            Ok(Id(id))
        }
    }

    fn depend(&mut self, name: Range<usize>, dep: Dep) -> Result<Id, (Range<usize>, Dep)> {
        if let Some((d, id, span, _)) = self.symbols.get_mut(&self.source[name]) {
            if *d == dep { Ok(*id) } else { Err((*span, *d)) }
        } else {
            self.prev_id += 1;
            let id = self.prev_id;
            self.symbols
                .insert(self.source[name].to_owned(), (dep, Id(id), name, false));
            Ok(Id(id))
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
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
    fn analyze(syn: Syn, source: &str, table: &mut GlobalTable) -> Result<Vec<Self>, SemError> {
        let mut statements = Vec::new();
        for tree in syn {
            let (tree, span) = tree.map_err(SemError::SynError)?;
            let class = &source[tree.class];
            let Some(Construct { named, unnamed }) = CONSTRUCTS.get(class) else {
                return Err(SemError::ClassNotFound {
                    found: class.to_owned(),
                    span: tree.class,
                });
            };
            let class = match tree.fields {
                Fields::Named(f) => named(f, span, table),
                Fields::Unnamed(f) => unnamed(f, span, table),
            }?;
            let dep = class.dep();
            let id = match table.register(tree.name, dep) {
                Ok(id) => id,
                Err(e) => match e {
                    RegisterError::Dep(prev_span, prev_dep) => {
                        return Err(SemError::WrongDependenceType {
                            prev_span,
                            prev_dep,
                            span,
                            dep,
                        });
                    }
                    RegisterError::Dup(prev_span) => {
                        return Err(SemError::DuplicatedStatementName { prev_span, span });
                    }
                },
            };
            statements.push(Self { id, class, span });
        }
        Ok(statements)
    }
}

struct Construct {
    named:
        fn(HashMap<&str, Spanned<Node>>, Range<usize>, &mut GlobalTable) -> Result<Class, SemError>,
    unnamed: fn(Vec<Spanned<Node>>, Range<usize>, &mut GlobalTable) -> Result<Class, SemError>,
}

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
                    span: Range<usize>,
                    table: &mut GlobalTable,
                ) -> Result<Class, SemError> {
                    $(let $field = {
                        let field = stringify!($field);
                        if let Some(field) = fields.remove(field)
                            .map(|(node, span)| $combine::from_node(node, table)
                                .map_err(|e| e.unwrap_or(SemError::WrongFieldDefinition {
                                    field,
                                    def: $combine::FIELD_DEFINITION,
                                    span
                                }))
                            )
                            $(.or(Some(Ok(lit_expr!($combine, $($default),+)))))? {
                            field?
                        } else {
                            return Err(SemError::FieldLost { field, span });
                        }
                    };)+
                    if let Some((_, (_, field_span))) = fields.iter().next() {
                        return Err(SemError::RedundantField { span: *field_span });
                    }
                    Ok(Class::$class { $($field),+ })
                }

                fn unnamed(
                    mut fields: Vec<Spanned<Node>>,
                    span: Range<usize>,
                    table: &mut GlobalTable,
                ) -> Result<Class, SemError> {
                    fields.reverse();
                    $(let $field = {
                        let field = stringify!($field);
                        if let Some(field) = fields.pop()
                            .map(|(node, span)| $combine::from_node(node, table)
                                .map_err(|e| e.unwrap_or(SemError::WrongFieldDefinition {
                                    field,
                                    def: $combine::FIELD_DEFINITION,
                                    span
                                }))
                            )
                            $(.or(Some(Ok(lit_expr!($combine, $($default),+)))))? {
                            field?
                        } else {
                            return Err(SemError::FieldLost { field, span });
                        }
                    };)+
                    if let Some((_, span)) = fields.get(0) {
                        return Err(SemError::RedundantField { span: *span });
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
    fn from_node(node: Node, table: &mut GlobalTable) -> Result<Self, Option<SemError>>;
}

pub struct Single(Expr);

impl FromNode for Single {
    const FIELD_DEFINITION: &'static str = "Any single expr without \"=\" or \"fn\" inside";

    fn from_node(node: Node, table: &mut GlobalTable) -> Result<Self, Option<SemError>> {
        match Expr::new(node, |name, dep| table.depend(name, dep)) {
            Ok(expr) => Ok(Self(expr)),
            Err(e) => Err(Some(e)),
        }
    }
}

pub struct Eq(Expr, Expr);

impl FromNode for Eq {
    const FIELD_DEFINITION: &'static str = "Two single exprs with \"=\" between";

    fn from_node(node: Node, table: &mut GlobalTable) -> Result<Self, Option<SemError>> {
        let syn::Node::Binary {
            binary: syn::Binary::Eq,
            pos,
            left,
            right,
        } = node
        else {
            return Err(None);
        };
        Ok(Self(
            Expr::new(*left, |name, dep| table.depend(name, dep)).map_err(Some)?,
            Expr::new(*right, |name, dep| table.depend(name, dep)).map_err(Some)?,
        ))
    }
}

pub struct Fn {
    args: Vec<String>,
    expr: Expr,
}

impl FromNode for Fn {
    const FIELD_DEFINITION: &'static str =
        "\"fn(...) = ...\" with only identifiers in fn() and single expr on the right";

    fn from_node(node: Node, table: &mut GlobalTable) -> Result<Self, Option<SemError>> {
        let syn::Node::Binary {
            binary: syn::Binary::Eq,
            pos,
            left,
            right,
        } = node
        else {
            return Err(None);
        };
        let syn::Node::Function { name, args, .. } = *left else {
            return Err(None);
        };
        if &table.source[name] != "fn" {
            return Err(None);
        }
        let mut names = Vec::new();
        let mut map = HashMap::new();
        for arg in args {
            let syn::Node::Primary { primary: syn::Primary::Var(span) } = arg else {
                return Err(None);
            };
            table.prev_id += 1;
            let name = table.source[span].to_owned();
            names.push(name.clone());
            map.insert(name, (Id(table.prev_id), span));
        }
        Ok(Self {
            args: names,
            expr: Expr::new(*right, |name, dep| {
                if let Some((id, span)) = map.get(&table.source[name]) {
                    if dep == Dep::Var {
                        Ok(*id)
                    } else {
                        Err((*span, Dep::Var))
                    }
                } else {
                    table.depend(name, dep)
                }
            })
            .map_err(Some)?,
        })
    }
}

pub struct Color(Expr, Expr, Expr, Expr);

impl FromNode for Color {
    const FIELD_DEFINITION: &'static str =
        "3 or 4 single exprs representing r, g, b and a channels of the color";

    fn from_node(node: Node, table: &mut GlobalTable) -> Result<Self, Option<SemError>> {
        todo!()
    }
}
