use serde::{Deserialize, Serialize};

use super::span::Spanned;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Type {
    Int,
    Real,
    Bool,
    String,
    Named(String),
    Array {
        element: Box<Type>,
        size: usize,
    },
    Map {
        key: Box<Type>,
        value: Box<Type>,
    },
    Enum(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Field {
    pub name: Spanned<String>,
    pub ty: Spanned<Type>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnumDef {
    pub name: Spanned<String>,
    pub variants: Vec<Spanned<String>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TypeDef {
    pub name: Spanned<String>,
    pub fields: Vec<Field>,
    pub alias: Option<Spanned<Type>>,
    pub refinement: Option<Spanned<super::nodes::Expr>>,
}
