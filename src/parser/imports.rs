use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow};

use crate::ast::nodes::{Item, Program};
use crate::ast::span::Spanned;
use crate::ast::types::{Field, Type};
use crate::errors::diagnostic::VerunError;

use super::parse_source;

#[derive(Debug)]
pub struct LoadedProgram {
    pub program: Program,
    pub root_source: String,
    pub file_count: usize,
}

pub fn parse_file_with_imports(file: &str) -> Result<LoadedProgram> {
    let root_path = PathBuf::from(file);
    let root_canonical = root_path
        .canonicalize()
        .map_err(|e| anyhow!("failed to resolve file '{}': {}", file, e))?;

    let root_source = fs::read_to_string(&root_canonical)?;

    let mut stack = Vec::new();
    let mut visited: HashSet<PathBuf> = HashSet::new();
    let mut order: Vec<PathBuf> = Vec::new();
    let mut programs: HashMap<PathBuf, Program> = HashMap::new();

    load_recursive(
        &root_canonical,
        &root_canonical,
        &mut stack,
        &mut visited,
        &mut order,
        &mut programs,
    )?;

    let mut merged_items = Vec::new();
    for path in &order {
        if let Some(program) = programs.remove(path) {
            for item in program.items {
                if !matches!(item.node, Item::Import(_)) {
                    merged_items.push(item);
                }
            }
        }
    }

    Ok(LoadedProgram {
        program: Program {
            items: merged_items,
            source: root_source.clone(),
        },
        root_source,
        file_count: order.len(),
    })
}

fn load_recursive(
    file: &Path,
    root_file: &Path,
    stack: &mut Vec<PathBuf>,
    visited: &mut HashSet<PathBuf>,
    order: &mut Vec<PathBuf>,
    programs: &mut HashMap<PathBuf, Program>,
) -> Result<()> {
    if stack.iter().any(|p| p == file) {
        let mut cycle = stack.clone();
        cycle.push(file.to_path_buf());
        let cycle_str = cycle
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join(" -> ");
        return Err(anyhow!(VerunError::ParseError {
            message: format!("import cycle detected: {}", cycle_str),
            span: None,
        }));
    }

    if visited.contains(file) {
        return Ok(());
    }

    stack.push(file.to_path_buf());

    let source = fs::read_to_string(file)?;
    let mut program = parse_source(&source).map_err(|e| {
        if file == root_file {
            e
        } else {
            anyhow!(VerunError::ParseError {
                message: format!("failed to parse imported file '{}': {}", file.display(), e),
                span: None,
            })
        }
    })?;

    let mut resolved_imports = Vec::new();
    for item in &program.items {
        if let Item::Import(import) = &item.node {
            let resolved = resolve_import_path(file, &import.path.node)?;
            load_recursive(&resolved, root_file, stack, visited, order, programs)?;
            resolved_imports.push((import.alias.as_ref().map(|a| a.node.clone()), resolved));
        }
    }

    rewrite_import_aliases(file, &mut program, &resolved_imports, programs)?;

    stack.pop();
    visited.insert(file.to_path_buf());
    programs.insert(file.to_path_buf(), program);
    order.push(file.to_path_buf());

    Ok(())
}

fn rewrite_import_aliases(
    file: &Path,
    program: &mut Program,
    resolved_imports: &[(Option<String>, PathBuf)],
    programs: &HashMap<PathBuf, Program>,
) -> Result<()> {
    let mut alias_exports: HashMap<String, HashSet<String>> = HashMap::new();

    for (alias, imported_path) in resolved_imports {
        if let Some(alias_name) = alias {
            let imported = programs.get(imported_path).ok_or_else(|| {
                anyhow!(
                    "internal error: imported program '{}' not available",
                    imported_path.display()
                )
            })?;
            alias_exports.insert(alias_name.clone(), exported_names(imported));
        }
    }

    for item in &mut program.items {
        rewrite_item(file, &alias_exports, item)?;
    }

    Ok(())
}

fn exported_names(program: &Program) -> HashSet<String> {
    let mut names = HashSet::new();
    for item in &program.items {
        match &item.node {
            Item::EnumDef(e) => {
                names.insert(e.name.node.clone());
            }
            Item::TypeDef(t) => {
                names.insert(t.name.node.clone());
            }
            Item::ConstDef(c) => {
                names.insert(c.name.node.clone());
            }
            Item::State(s) => {
                names.insert(s.name.node.clone());
            }
            Item::Function(f) => {
                names.insert(f.name.node.clone());
            }
            Item::Import(_) => {}
        }
    }
    names
}

fn rewrite_item(
    file: &Path,
    alias_exports: &HashMap<String, HashSet<String>>,
    item: &mut Spanned<Item>,
) -> Result<()> {
    match &mut item.node {
        Item::Import(_) => {}
        Item::EnumDef(_) => {}
        Item::TypeDef(t) => {
            for field in &mut t.fields {
                rewrite_field(file, alias_exports, field)?;
            }
            if let Some(alias) = &mut t.alias {
                rewrite_type(file, alias_exports, &mut alias.node)?;
            }
            if let Some(refinement) = &mut t.refinement {
                rewrite_expr(file, alias_exports, &mut refinement.node)?;
            }
        }
        Item::ConstDef(c) => {
            rewrite_type(file, alias_exports, &mut c.ty.node)?;
            rewrite_expr(file, alias_exports, &mut c.value.node)?;
        }
        Item::State(s) => {
            for c in &mut s.constants {
                rewrite_type(file, alias_exports, &mut c.ty.node)?;
                rewrite_expr(file, alias_exports, &mut c.value.node)?;
            }
            for field in &mut s.fields {
                rewrite_field(file, alias_exports, field)?;
            }
            for inv in &mut s.invariants {
                rewrite_expr(file, alias_exports, &mut inv.condition.node)?;
            }
            if let Some(init) = &mut s.init {
                for a in &mut init.assignments {
                    rewrite_expr(file, alias_exports, &mut a.value.node)?;
                }
            }
            for t in &mut s.transitions {
                for p in &mut t.params {
                    rewrite_type(file, alias_exports, &mut p.ty.node)?;
                }
                for pre in &mut t.preconditions {
                    rewrite_expr(file, alias_exports, &mut pre.node)?;
                }
                for stmt in &mut t.body {
                    rewrite_statement(file, alias_exports, &mut stmt.node)?;
                }
                for post in &mut t.postconditions {
                    rewrite_expr(file, alias_exports, &mut post.node)?;
                }
                for emit in &mut t.emits {
                    for arg in &mut emit.node.args {
                        rewrite_expr(file, alias_exports, &mut arg.node)?;
                    }
                }
            }
        }
        Item::Function(f) => {
            for p in &mut f.params {
                rewrite_type(file, alias_exports, &mut p.ty.node)?;
            }
            rewrite_type(file, alias_exports, &mut f.return_type.node)?;
            if let Some(body) = &mut f.body {
                for stmt in body {
                    rewrite_statement(file, alias_exports, &mut stmt.node)?;
                }
            }
        }
    }
    Ok(())
}

fn rewrite_field(
    file: &Path,
    alias_exports: &HashMap<String, HashSet<String>>,
    field: &mut Field,
) -> Result<()> {
    rewrite_type(file, alias_exports, &mut field.ty.node)
}

fn rewrite_type(
    file: &Path,
    alias_exports: &HashMap<String, HashSet<String>>,
    ty: &mut Type,
) -> Result<()> {
    match ty {
        Type::Named(name) | Type::Enum(name) => {
            if let Some(rewritten) = rewrite_qualified_name(file, alias_exports, name)? {
                *name = rewritten;
            }
        }
        Type::Array { element, .. } => rewrite_type(file, alias_exports, element)?,
        Type::Map { key, value } => {
            rewrite_type(file, alias_exports, key)?;
            rewrite_type(file, alias_exports, value)?;
        }
        Type::Int | Type::Real | Type::Bool | Type::String => {}
    }
    Ok(())
}

fn rewrite_statement(
    file: &Path,
    alias_exports: &HashMap<String, HashSet<String>>,
    stmt: &mut crate::ast::nodes::Statement,
) -> Result<()> {
    match stmt {
        crate::ast::nodes::Statement::Assign(a) => {
            rewrite_expr(file, alias_exports, &mut a.value.node)?
        }
        crate::ast::nodes::Statement::CompoundAssign { value, .. } => {
            rewrite_expr(file, alias_exports, &mut value.node)?
        }
        crate::ast::nodes::Statement::IndexedAssign { index, value, .. } => {
            rewrite_expr(file, alias_exports, &mut index.node)?;
            rewrite_expr(file, alias_exports, &mut value.node)?;
        }
        crate::ast::nodes::Statement::IndexedCompoundAssign { index, value, .. } => {
            rewrite_expr(file, alias_exports, &mut index.node)?;
            rewrite_expr(file, alias_exports, &mut value.node)?;
        }
        crate::ast::nodes::Statement::Assert { condition } => {
            rewrite_expr(file, alias_exports, &mut condition.node)?
        }
        crate::ast::nodes::Statement::If {
            condition,
            then_body,
            else_body,
        } => {
            rewrite_expr(file, alias_exports, &mut condition.node)?;
            for s in then_body {
                rewrite_statement(file, alias_exports, &mut s.node)?;
            }
            if let Some(else_body) = else_body {
                for s in else_body {
                    rewrite_statement(file, alias_exports, &mut s.node)?;
                }
            }
        }
        crate::ast::nodes::Statement::Let { ty, value, .. } => {
            if let Some(ty) = ty {
                rewrite_type(file, alias_exports, &mut ty.node)?;
            }
            rewrite_expr(file, alias_exports, &mut value.node)?;
        }
        crate::ast::nodes::Statement::Match { expr, arms } => {
            rewrite_expr(file, alias_exports, &mut expr.node)?;
            for arm in arms {
                if let crate::ast::nodes::MatchPattern::EnumVariant { enum_name, .. } =
                    &mut arm.pattern.node
                {
                    if let Some(rewritten) = rewrite_qualified_name(file, alias_exports, enum_name)?
                    {
                        *enum_name = rewritten;
                    }
                }
                for s in &mut arm.body {
                    rewrite_statement(file, alias_exports, &mut s.node)?;
                }
            }
        }
    }
    Ok(())
}

fn rewrite_expr(
    file: &Path,
    alias_exports: &HashMap<String, HashSet<String>>,
    expr: &mut crate::ast::nodes::Expr,
) -> Result<()> {
    use crate::ast::nodes::Expr;

    match expr {
        Expr::Ident(name) => {
            if let Some(rewritten) = rewrite_qualified_name(file, alias_exports, name)? {
                *name = rewritten;
            }
        }
        Expr::FieldAccess { object, .. } => rewrite_expr(file, alias_exports, &mut object.node)?,
        Expr::IndexAccess { object, index } => {
            rewrite_expr(file, alias_exports, &mut object.node)?;
            rewrite_expr(file, alias_exports, &mut index.node)?;
        }
        Expr::MapAccess { map, key } => {
            rewrite_expr(file, alias_exports, &mut map.node)?;
            rewrite_expr(file, alias_exports, &mut key.node)?;
        }
        Expr::UnaryOp { operand, .. } => rewrite_expr(file, alias_exports, &mut operand.node)?,
        Expr::BinaryOp { left, right, .. } => {
            rewrite_expr(file, alias_exports, &mut left.node)?;
            rewrite_expr(file, alias_exports, &mut right.node)?;
        }
        Expr::FnCall { name, args } => {
            if let Some(rewritten) = rewrite_qualified_name(file, alias_exports, &name.node)? {
                name.node = rewritten;
            }
            for arg in args {
                rewrite_expr(file, alias_exports, &mut arg.node)?;
            }
        }
        Expr::Old(inner) => rewrite_expr(file, alias_exports, &mut inner.node)?,
        Expr::Forall { domain, body, .. } | Expr::Exists { domain, body, .. } => {
            rewrite_expr(file, alias_exports, &mut domain.node)?;
            rewrite_expr(file, alias_exports, &mut body.node)?;
        }
        Expr::Range { start, end } => {
            rewrite_expr(file, alias_exports, &mut start.node)?;
            rewrite_expr(file, alias_exports, &mut end.node)?;
        }
        Expr::EnumVariant { enum_name, variant } => {
            let mut replace_with_ident = None;

            if let Some(exports) = alias_exports.get(enum_name.as_str()) {
                if exports.contains(variant.as_str()) {
                    replace_with_ident = Some(variant.clone());
                } else {
                    return Err(anyhow!(VerunError::ParseError {
                        message: format!(
                            "symbol '{}' is not exported by alias '{}' in '{}'",
                            variant,
                            enum_name,
                            file.display()
                        ),
                        span: None,
                    }));
                }
            } else if let Some(rewritten) = rewrite_qualified_name(file, alias_exports, enum_name)?
            {
                *enum_name = rewritten;
            }

            if let Some(name) = replace_with_ident {
                *expr = Expr::Ident(name);
            }
        }
        Expr::IntLit(_) | Expr::RealLit(_) | Expr::BoolLit(_) | Expr::StringLit(_) => {}
    }

    Ok(())
}

fn rewrite_qualified_name(
    file: &Path,
    alias_exports: &HashMap<String, HashSet<String>>,
    name: &str,
) -> Result<Option<String>> {
    let mut parts = name.split("::");
    let first = parts.next().unwrap_or_default();
    let second = parts.next();

    if second.is_none() {
        return Ok(None);
    }

    if let Some(exports) = alias_exports.get(first) {
        let exported = second.unwrap();
        if !exports.contains(exported) {
            return Err(anyhow!(VerunError::ParseError {
                message: format!(
                    "symbol '{}' is not exported by alias '{}' in '{}'",
                    exported,
                    first,
                    file.display()
                ),
                span: None,
            }));
        }
        let remainder = name[first.len() + 2..].to_string();
        return Ok(Some(remainder));
    }

    Ok(None)
}

fn resolve_import_path(current_file: &Path, import_path: &str) -> Result<PathBuf> {
    let parent = current_file.parent().ok_or_else(|| {
        anyhow!(
            "cannot resolve parent directory for '{}'",
            current_file.display()
        )
    })?;
    let joined = parent.join(import_path);
    let canonical = joined.canonicalize().map_err(|e| {
        anyhow!(
            "failed to resolve import '{}' from '{}': {}",
            import_path,
            current_file.display(),
            e
        )
    })?;
    Ok(canonical)
}
