use crate::ast::types::Type;
use crate::errors::diagnostic::VerunError;

use super::env::{TypeEnv, TypeEntry};

pub fn resolve_type(ty: &Type, env: &TypeEnv) -> Result<Type, VerunError> {
    match ty {
        Type::Named(name) => {
            if let Some(entry) = env.lookup_type(name) {
                match entry {
                    TypeEntry::Alias { target, .. } => Ok(target.clone()),
                    _ => Ok(ty.clone()),
                }
            } else if env.lookup_enum(name).is_some() {
                Ok(ty.clone())
            } else {
                Err(VerunError::UndefinedType {
                    name: name.clone(),
                    span: None,
                })
            }
        }
        Type::Array { element, size } => {
            let resolved = resolve_type(element, env)?;
            Ok(Type::Array {
                element: Box::new(resolved),
                size: *size,
            })
        }
        Type::Map { key, value } => {
            let resolved_key = resolve_type(key, env)?;
            let resolved_value = resolve_type(value, env)?;
            Ok(Type::Map {
                key: Box::new(resolved_key),
                value: Box::new(resolved_value),
            })
        }
        _ => Ok(ty.clone()),
    }
}

pub fn types_compatible(expected: &Type, actual: &Type) -> bool {
    match (expected, actual) {
        (Type::Int, Type::Int) => true,
        (Type::Real, Type::Real) => true,
        (Type::Bool, Type::Bool) => true,
        (Type::String, Type::String) => true,
        (Type::Named(a), Type::Named(b)) => a == b,
        (Type::Enum(a), Type::Enum(b)) => a == b,
        (Type::Named(a), Type::Enum(b)) | (Type::Enum(a), Type::Named(b)) => a == b,
        (
            Type::Array {
                element: e1,
                size: s1,
            },
            Type::Array {
                element: e2,
                size: s2,
            },
        ) => s1 == s2 && types_compatible(e1, e2),
        (
            Type::Map {
                key: k1,
                value: v1,
            },
            Type::Map {
                key: k2,
                value: v2,
            },
        ) => types_compatible(k1, k2) && types_compatible(v1, v2),
        (Type::Int, Type::Real) | (Type::Real, Type::Int) => true,
        _ => false,
    }
}
