use std::collections::HashMap;

use crate::ast::nodes::Expr;
use crate::ast::span::Spanned;
use crate::ast::types::Type;

#[derive(Debug, Clone)]
pub struct TypeEnv {
    scopes: Vec<HashMap<String, Type>>,
    types: HashMap<String, TypeEntry>,
    enums: HashMap<String, Vec<String>>,
    functions: HashMap<String, FunctionSig>,
}

#[derive(Debug, Clone)]
pub struct FunctionSig {
    pub params: Vec<Type>,
    pub return_type: Type,
}

#[derive(Debug, Clone)]
pub enum TypeEntry {
    Struct(HashMap<String, Type>),
    Enum(Vec<String>),
    Alias {
        target: Type,
        refinement: Option<Spanned<Expr>>,
    },
}

impl TypeEnv {
    pub fn new() -> Self {
        Self {
            scopes: vec![HashMap::new()],
            types: HashMap::new(),
            enums: HashMap::new(),
            functions: HashMap::new(),
        }
    }

    pub fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    pub fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    pub fn define_var(&mut self, name: &str, ty: Type) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.to_string(), ty);
        }
    }

    pub fn lookup_var(&self, name: &str) -> Option<&Type> {
        for scope in self.scopes.iter().rev() {
            if let Some(ty) = scope.get(name) {
                return Some(ty);
            }
        }
        None
    }

    pub fn define_type(&mut self, name: &str, entry: TypeEntry) {
        self.types.insert(name.to_string(), entry);
    }

    pub fn lookup_type(&self, name: &str) -> Option<&TypeEntry> {
        self.types.get(name)
    }

    pub fn define_enum(&mut self, name: &str, variants: Vec<String>) {
        self.enums.insert(name.to_string(), variants);
    }

    pub fn lookup_enum(&self, name: &str) -> Option<&Vec<String>> {
        self.enums.get(name)
    }

    pub fn is_valid_enum_variant(&self, enum_name: &str, variant: &str) -> bool {
        self.enums
            .get(enum_name)
            .is_some_and(|variants| variants.contains(&variant.to_string()))
    }

    pub fn define_function(&mut self, name: &str, sig: FunctionSig) {
        self.functions.insert(name.to_string(), sig);
    }

    pub fn lookup_function(&self, name: &str) -> Option<&FunctionSig> {
        self.functions.get(name)
    }
}

impl Default for TypeEnv {
    fn default() -> Self {
        Self::new()
    }
}
