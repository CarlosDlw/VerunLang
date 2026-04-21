use serde::{Deserialize, Serialize};

use super::span::{Span, Spanned};
use super::types::{EnumDef, Field, Type, TypeDef};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Program {
    pub items: Vec<Spanned<Item>>,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Item {
    Import(Import),
    EnumDef(EnumDef),
    TypeDef(TypeDef),
    ConstDef(ConstDef),
    State(StateDef),
    Function(FnDef),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConstDef {
    pub name: Spanned<String>,
    pub ty: Spanned<Type>,
    pub value: Spanned<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Import {
    pub path: Spanned<String>,
    pub alias: Option<Spanned<String>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StateDef {
    pub name: Spanned<String>,
    pub fields: Vec<Field>,
    pub constants: Vec<ConstDef>,
    pub invariants: Vec<Invariant>,
    pub init: Option<InitBlock>,
    pub transitions: Vec<Transition>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Invariant {
    pub name: Option<Spanned<String>>,
    pub condition: Spanned<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InitBlock {
    pub assignments: Vec<Assignment>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Assignment {
    pub target: Spanned<String>,
    pub value: Spanned<Expr>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Transition {
    pub name: Spanned<String>,
    pub params: Vec<Param>,
    pub preconditions: Vec<Spanned<Expr>>,
    pub body: Vec<Spanned<Statement>>,
    pub postconditions: Vec<Spanned<Expr>>,
    pub emits: Vec<Spanned<Emit>>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Param {
    pub name: Spanned<String>,
    pub ty: Spanned<Type>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Statement {
    Assign(Assignment),
    CompoundAssign {
        target: Spanned<String>,
        op: CompoundOp,
        value: Spanned<Expr>,
    },
    IndexedAssign {
        target: Spanned<String>,
        index: Spanned<Expr>,
        value: Spanned<Expr>,
    },
    IndexedCompoundAssign {
        target: Spanned<String>,
        index: Spanned<Expr>,
        op: CompoundOp,
        value: Spanned<Expr>,
    },
    Assert {
        condition: Spanned<Expr>,
    },
    If {
        condition: Spanned<Expr>,
        then_body: Vec<Spanned<Statement>>,
        else_body: Option<Vec<Spanned<Statement>>>,
    },
    Let {
        name: Spanned<String>,
        ty: Option<Spanned<Type>>,
        value: Spanned<Expr>,
    },
    Match {
        expr: Spanned<Expr>,
        arms: Vec<MatchArm>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MatchArm {
    pub pattern: Spanned<MatchPattern>,
    pub body: Vec<Spanned<Statement>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MatchPattern {
    EnumVariant { enum_name: String, variant: String },
    IntLit(i64),
    BoolLit(bool),
    StringLit(String),
    Wildcard,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompoundOp {
    Add,
    Sub,
    Mul,
    Div,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Emit {
    pub event_name: Spanned<String>,
    pub args: Vec<Spanned<Expr>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Expr {
    IntLit(i64),
    RealLit(f64),
    BoolLit(bool),
    StringLit(String),

    Ident(String),
    FieldAccess {
        object: Box<Spanned<Expr>>,
        field: Spanned<String>,
    },
    IndexAccess {
        object: Box<Spanned<Expr>>,
        index: Box<Spanned<Expr>>,
    },
    MapAccess {
        map: Box<Spanned<Expr>>,
        key: Box<Spanned<Expr>>,
    },

    UnaryOp {
        op: UnaryOp,
        operand: Box<Spanned<Expr>>,
    },
    BinaryOp {
        left: Box<Spanned<Expr>>,
        op: BinaryOp,
        right: Box<Spanned<Expr>>,
    },

    FnCall {
        name: Spanned<String>,
        args: Vec<Spanned<Expr>>,
    },

    Old(Box<Spanned<Expr>>),

    Forall {
        var: Spanned<String>,
        domain: Box<Spanned<Expr>>,
        body: Box<Spanned<Expr>>,
    },
    Exists {
        var: Spanned<String>,
        domain: Box<Spanned<Expr>>,
        body: Box<Spanned<Expr>>,
    },

    Range {
        start: Box<Spanned<Expr>>,
        end: Box<Spanned<Expr>>,
    },

    EnumVariant {
        enum_name: String,
        variant: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnaryOp {
    Neg,
    Not,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,

    Eq,
    Neq,
    Lt,
    Gt,
    Lte,
    Gte,

    And,
    Or,
    Implies,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FnDef {
    pub name: Spanned<String>,
    pub params: Vec<Param>,
    pub return_type: Spanned<Type>,
    pub body: Option<Vec<Spanned<Statement>>>,
    pub is_extern: bool,
    pub span: Span,
}
