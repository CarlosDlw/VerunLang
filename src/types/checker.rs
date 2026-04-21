use crate::ast::nodes::*;
use crate::ast::span::Spanned;
use crate::ast::types::Type;
use crate::errors::diagnostic::VerunError;

use super::env::{FunctionSig, TypeEntry, TypeEnv};
use super::resolver::{resolve_type, types_compatible};

pub struct TypeChecker {
    env: TypeEnv,
    errors: Vec<VerunError>,
    in_postcondition: bool,
    param_names: std::collections::HashSet<String>,
    state_field_names: std::collections::HashSet<String>,
}

impl TypeChecker {
    pub fn new() -> Self {
        Self {
            env: TypeEnv::new(),
            errors: Vec::new(),
            in_postcondition: false,
            param_names: std::collections::HashSet::new(),
            state_field_names: std::collections::HashSet::new(),
        }
    }

    pub fn check(&mut self, program: &Program) -> Vec<VerunError> {
        let mut seen_items: std::collections::HashSet<String> = std::collections::HashSet::new();

        for item in &program.items {
            match &item.node {
                Item::EnumDef(e) => {
                    if !seen_items.insert(e.name.node.clone()) {
                        self.errors.push(VerunError::DuplicateDefinition {
                            name: e.name.node.clone(),
                            span: Some(e.name.span),
                        });
                    }
                    let mut seen_variants: std::collections::HashSet<String> =
                        std::collections::HashSet::new();
                    for v in &e.variants {
                        if !seen_variants.insert(v.node.clone()) {
                            self.errors.push(VerunError::DuplicateDefinition {
                                name: format!("{}::{}", e.name.node, v.node),
                                span: Some(v.span),
                            });
                        }
                    }
                    self.register_enum(e);
                }
                Item::TypeDef(t) => {
                    if !seen_items.insert(t.name.node.clone()) {
                        self.errors.push(VerunError::DuplicateDefinition {
                            name: t.name.node.clone(),
                            span: Some(t.name.span),
                        });
                    }
                    self.register_type_def(t);
                }
                _ => {}
            }
        }

        for item in &program.items {
            if let Item::Function(func) = &item.node {
                if !seen_items.insert(func.name.node.clone()) {
                    self.errors.push(VerunError::DuplicateDefinition {
                        name: func.name.node.clone(),
                        span: Some(func.name.span),
                    });
                }
                self.register_function_signature(func);
            }
        }

        for item in &program.items {
            match &item.node {
                Item::ConstDef(c) => {
                    if !seen_items.insert(c.name.node.clone()) {
                        self.errors.push(VerunError::DuplicateDefinition {
                            name: c.name.node.clone(),
                            span: Some(c.name.span),
                        });
                    }
                    self.register_const(c);
                }
                Item::State(state) => {
                    if !seen_items.insert(state.name.node.clone()) {
                        self.errors.push(VerunError::DuplicateDefinition {
                            name: state.name.node.clone(),
                            span: Some(state.name.span),
                        });
                    }
                    self.check_state(state);
                }
                Item::Function(func) => self.check_function(func),
                _ => {}
            }
        }

        std::mem::take(&mut self.errors)
    }

    fn register_enum(&mut self, e: &crate::ast::types::EnumDef) {
        let variants: Vec<String> = e.variants.iter().map(|v| v.node.clone()).collect();
        self.env.define_enum(&e.name.node, variants.clone());
        self.env
            .define_type(&e.name.node, TypeEntry::Enum(variants));
    }

    fn register_type_def(&mut self, t: &crate::ast::types::TypeDef) {
        if let Some(alias) = &t.alias {
            if let Some(ref refinement) = t.refinement {
                self.env.push_scope();
                self.env.define_var("value", alias.node.clone());
                let ref_ty = self.infer_expr(refinement);
                if let Some(ty) = &ref_ty
                    && *ty != Type::Bool
                {
                    self.errors.push(VerunError::TypeMismatch {
                        expected: "bool".to_string(),
                        found: format!("{:?}", ty),
                        span: Some(refinement.span),
                    });
                }
                self.env.pop_scope();
            }
            self.env.define_type(
                &t.name.node,
                TypeEntry::Alias {
                    target: alias.node.clone(),
                    refinement: t.refinement.clone(),
                },
            );
        } else {
            let mut fields = std::collections::HashMap::new();
            for field in &t.fields {
                fields.insert(field.name.node.clone(), field.ty.node.clone());
            }
            self.env
                .define_type(&t.name.node, TypeEntry::Struct(fields));
        }
    }

    fn register_const(&mut self, c: &ConstDef) {
        let expected = match resolve_type(&c.ty.node, &self.env) {
            Ok(resolved) => resolved,
            Err(e) => {
                self.errors.push(e);
                c.ty.node.clone()
            }
        };
        if let Some(val_ty) = self.infer_expr(&c.value)
            && !types_compatible(&expected, &val_ty)
        {
            self.errors.push(VerunError::TypeMismatch {
                expected: format!("{:?}", expected),
                found: format!("{:?}", val_ty),
                span: Some(c.value.span),
            });
        }
        self.env.define_var(&c.name.node, expected);
    }

    fn register_function_signature(&mut self, func: &FnDef) {
        let params = func
            .params
            .iter()
            .map(|p| match resolve_type(&p.ty.node, &self.env) {
                Ok(resolved) => resolved,
                Err(e) => {
                    self.errors.push(e);
                    p.ty.node.clone()
                }
            })
            .collect::<Vec<_>>();

        let return_type = match resolve_type(&func.return_type.node, &self.env) {
            Ok(resolved) => resolved,
            Err(e) => {
                self.errors.push(e);
                func.return_type.node.clone()
            }
        };

        self.env.define_function(
            &func.name.node,
            FunctionSig {
                params,
                return_type,
            },
        );
    }

    fn check_state(&mut self, state: &StateDef) {
        self.env.push_scope();

        for c in &state.constants {
            self.register_const(c);
        }

        self.state_field_names.clear();
        let mut seen_fields: std::collections::HashSet<String> = std::collections::HashSet::new();
        for field in &state.fields {
            if !seen_fields.insert(field.name.node.clone()) {
                self.errors.push(VerunError::DuplicateDefinition {
                    name: field.name.node.clone(),
                    span: Some(field.name.span),
                });
            }
            match resolve_type(&field.ty.node, &self.env) {
                Ok(resolved) => self.env.define_var(&field.name.node, resolved),
                Err(e) => {
                    self.errors.push(e);
                    self.env.define_var(&field.name.node, field.ty.node.clone());
                }
            }
            self.state_field_names.insert(field.name.node.clone());
        }

        let mut seen_invariants: std::collections::HashSet<String> =
            std::collections::HashSet::new();
        for inv in &state.invariants {
            if let Some(name) = &inv.name
                && !seen_invariants.insert(name.node.clone())
            {
                self.errors.push(VerunError::DuplicateDefinition {
                    name: name.node.clone(),
                    span: Some(name.span),
                });
            }
            if let Some(ty) = self.infer_expr(&inv.condition)
                && ty != Type::Bool
            {
                self.errors.push(VerunError::TypeMismatch {
                    expected: "bool".to_string(),
                    found: format!("{:?}", ty),
                    span: Some(inv.condition.span),
                });
            }
            self.check_idents_in_expr(&inv.condition);
        }

        if let Some(init) = &state.init {
            let mut initialized: std::collections::HashSet<String> =
                std::collections::HashSet::new();
            for assign in &init.assignments {
                if self.env.lookup_var(&assign.target.node).is_none() {
                    self.errors.push(VerunError::UndefinedVariable {
                        name: assign.target.node.clone(),
                        span: Some(assign.target.span),
                    });
                } else {
                    if let Some(field_ty) = self.env.lookup_var(&assign.target.node).cloned()
                        && let Some(val_ty) = self.infer_expr(&assign.value)
                        && !types_compatible(&field_ty, &val_ty)
                    {
                        self.errors.push(VerunError::TypeMismatch {
                            expected: format!("{:?}", field_ty),
                            found: format!("{:?}", val_ty),
                            span: Some(assign.value.span),
                        });
                    }
                }
                initialized.insert(assign.target.node.clone());
            }
            for field in &state.fields {
                if !initialized.contains(&field.name.node) {
                    match &field.ty.node {
                        Type::Array { .. } | Type::Map { .. } => {}
                        _ => {
                            self.errors.push(VerunError::UninitializedField {
                                name: field.name.node.clone(),
                                span: Some(init.span),
                            });
                        }
                    }
                }
            }
        } else if !state.fields.is_empty() {
            self.errors.push(VerunError::MissingInit {
                name: state.name.node.clone(),
                span: Some(state.name.span),
            });
        }

        let mut seen_transitions: std::collections::HashSet<String> =
            std::collections::HashSet::new();
        for transition in &state.transitions {
            if !seen_transitions.insert(transition.name.node.clone()) {
                self.errors.push(VerunError::DuplicateDefinition {
                    name: transition.name.node.clone(),
                    span: Some(transition.name.span),
                });
            }
            self.check_transition(transition);
        }

        self.env.pop_scope();
    }

    fn check_transition(&mut self, transition: &Transition) {
        self.env.push_scope();
        self.param_names.clear();

        if transition.body.is_empty() && transition.postconditions.is_empty() {
            self.errors.push(VerunError::EmptyTransitionBody {
                name: transition.name.node.clone(),
                span: Some(transition.name.span),
            });
        }

        for param in &transition.params {
            match resolve_type(&param.ty.node, &self.env) {
                Ok(resolved) => self.env.define_var(&param.name.node, resolved),
                Err(e) => {
                    self.errors.push(e);
                    self.env.define_var(&param.name.node, param.ty.node.clone());
                }
            }
            self.param_names.insert(param.name.node.clone());
        }

        for pre in &transition.preconditions {
            self.check_expr_no_old(pre);
            self.check_idents_in_expr(pre);
            if let Some(ty) = self.infer_expr(pre)
                && ty != Type::Bool
            {
                self.errors.push(VerunError::TypeMismatch {
                    expected: "bool".to_string(),
                    found: format!("{:?}", ty),
                    span: Some(pre.span),
                });
            }
        }

        for stmt in &transition.body {
            self.check_statement(stmt);
        }

        self.in_postcondition = true;
        for post in &transition.postconditions {
            self.check_idents_in_expr(post);
            if let Some(ty) = self.infer_expr(post)
                && ty != Type::Bool
            {
                self.errors.push(VerunError::TypeMismatch {
                    expected: "bool".to_string(),
                    found: format!("{:?}", ty),
                    span: Some(post.span),
                });
            }
        }
        self.in_postcondition = false;

        self.param_names.clear();
        self.env.pop_scope();
    }

    fn check_expr_no_old(&mut self, expr: &Spanned<Expr>) {
        match &expr.node {
            Expr::Old(_) => {
                self.errors.push(VerunError::OldOutsideEnsure {
                    span: Some(expr.span),
                });
            }
            Expr::BinaryOp { left, right, .. } => {
                self.check_expr_no_old(left);
                self.check_expr_no_old(right);
            }
            Expr::UnaryOp { operand, .. } => {
                self.check_expr_no_old(operand);
            }
            Expr::FnCall { args, .. } => {
                for a in args {
                    self.check_expr_no_old(a);
                }
            }
            Expr::Forall { body, .. } | Expr::Exists { body, .. } => {
                self.check_expr_no_old(body);
            }
            _ => {}
        }
    }

    fn check_idents_in_expr(&mut self, expr: &Spanned<Expr>) {
        match &expr.node {
            Expr::Ident(name) if self.env.lookup_var(name).is_none() => {
                self.errors.push(VerunError::UndefinedVariable {
                    name: name.clone(),
                    span: Some(expr.span),
                });
            }
            Expr::BinaryOp { left, right, .. } => {
                self.check_idents_in_expr(left);
                self.check_idents_in_expr(right);
            }
            Expr::UnaryOp { operand, .. } => {
                self.check_idents_in_expr(operand);
            }
            Expr::Old(inner) => {
                self.check_idents_in_expr(inner);
            }
            Expr::FnCall { args, .. } => {
                for a in args {
                    self.check_idents_in_expr(a);
                }
            }
            Expr::Forall { var, domain, body } | Expr::Exists { var, domain, body } => {
                self.env.push_scope();
                if let Some(resolved) = self.env.lookup_var(&var.node).cloned().or({
                    // try resolving domain as a type — for set/range quantifiers the type
                    // comes from the domain expression; just register as Int for now
                    None
                }) {
                    self.env.define_var(&var.node, resolved);
                } else {
                    // For range quantifiers like `forall x in 0..n`, x is an integer index
                    self.env.define_var(&var.node, crate::ast::types::Type::Int);
                }
                self.check_idents_in_expr(domain);
                self.check_idents_in_expr(body);
                self.env.pop_scope();
            }
            Expr::FieldAccess { object, .. } => {
                self.check_idents_in_expr(object);
            }
            Expr::IndexAccess { object, index } => {
                self.check_idents_in_expr(object);
                self.check_idents_in_expr(index);
            }
            Expr::MapAccess { map, key } => {
                self.check_idents_in_expr(map);
                self.check_idents_in_expr(key);
            }
            _ => {}
        }
    }

    fn check_statement(&mut self, stmt: &Spanned<Statement>) {
        match &stmt.node {
            Statement::Assign(assign) => {
                if let Some(field_ty) = self.env.lookup_var(&assign.target.node).cloned() {
                    if let Some(val_ty) = self.infer_expr(&assign.value)
                        && !types_compatible(&field_ty, &val_ty)
                    {
                        self.errors.push(VerunError::TypeMismatch {
                            expected: format!("{:?}", field_ty),
                            found: format!("{:?}", val_ty),
                            span: Some(assign.value.span),
                        });
                    }
                } else {
                    self.errors.push(VerunError::UndefinedVariable {
                        name: assign.target.node.clone(),
                        span: Some(assign.target.span),
                    });
                }
                self.check_expr_no_old(&assign.value);
            }
            Statement::CompoundAssign { target, value, .. } => {
                if let Some(field_ty) = self.env.lookup_var(&target.node).cloned() {
                    if let Some(val_ty) = self.infer_expr(value)
                        && !types_compatible(&field_ty, &val_ty)
                    {
                        self.errors.push(VerunError::TypeMismatch {
                            expected: format!("{:?}", field_ty),
                            found: format!("{:?}", val_ty),
                            span: Some(value.span),
                        });
                    }
                } else {
                    self.errors.push(VerunError::UndefinedVariable {
                        name: target.node.clone(),
                        span: Some(target.span),
                    });
                }
                self.check_expr_no_old(value);
            }
            Statement::IndexedAssign {
                target,
                index,
                value,
            } => {
                if let Some(field_ty) = self.env.lookup_var(&target.node).cloned() {
                    match &field_ty {
                        Type::Array { element, .. } => {
                            if let Some(val_ty) = self.infer_expr(value)
                                && !types_compatible(element, &val_ty)
                            {
                                self.errors.push(VerunError::TypeMismatch {
                                    expected: format!("{:?}", element),
                                    found: format!("{:?}", val_ty),
                                    span: Some(value.span),
                                });
                            }
                            if let Some(idx_ty) = self.infer_expr(index)
                                && idx_ty != Type::Int
                            {
                                self.errors.push(VerunError::TypeMismatch {
                                    expected: "Int".to_string(),
                                    found: format!("{:?}", idx_ty),
                                    span: Some(index.span),
                                });
                            }
                        }
                        Type::Map { key, value: val_ty } => {
                            if let Some(idx_ty) = self.infer_expr(index)
                                && !types_compatible(key, &idx_ty)
                            {
                                self.errors.push(VerunError::TypeMismatch {
                                    expected: format!("{:?}", key),
                                    found: format!("{:?}", idx_ty),
                                    span: Some(index.span),
                                });
                            }
                            if let Some(v_ty) = self.infer_expr(value)
                                && !types_compatible(val_ty, &v_ty)
                            {
                                self.errors.push(VerunError::TypeMismatch {
                                    expected: format!("{:?}", val_ty),
                                    found: format!("{:?}", v_ty),
                                    span: Some(value.span),
                                });
                            }
                        }
                        _ => {
                            self.errors.push(VerunError::TypeMismatch {
                                expected: "array or map".to_string(),
                                found: format!("{:?}", field_ty),
                                span: Some(target.span),
                            });
                        }
                    }
                } else {
                    self.errors.push(VerunError::UndefinedVariable {
                        name: target.node.clone(),
                        span: Some(target.span),
                    });
                }
                self.check_expr_no_old(index);
                self.check_expr_no_old(value);
            }
            Statement::IndexedCompoundAssign {
                target,
                index,
                value,
                ..
            } => {
                if let Some(field_ty) = self.env.lookup_var(&target.node).cloned() {
                    match &field_ty {
                        Type::Array { element, .. } => {
                            if let Some(val_ty) = self.infer_expr(value)
                                && !types_compatible(element, &val_ty)
                            {
                                self.errors.push(VerunError::TypeMismatch {
                                    expected: format!("{:?}", element),
                                    found: format!("{:?}", val_ty),
                                    span: Some(value.span),
                                });
                            }
                        }
                        Type::Map { value: val_ty, .. } => {
                            if let Some(v_ty) = self.infer_expr(value)
                                && !types_compatible(val_ty, &v_ty)
                            {
                                self.errors.push(VerunError::TypeMismatch {
                                    expected: format!("{:?}", val_ty),
                                    found: format!("{:?}", v_ty),
                                    span: Some(value.span),
                                });
                            }
                        }
                        _ => {
                            self.errors.push(VerunError::TypeMismatch {
                                expected: "array or map".to_string(),
                                found: format!("{:?}", field_ty),
                                span: Some(target.span),
                            });
                        }
                    }
                } else {
                    self.errors.push(VerunError::UndefinedVariable {
                        name: target.node.clone(),
                        span: Some(target.span),
                    });
                }
                self.check_expr_no_old(index);
                self.check_expr_no_old(value);
            }
            Statement::Assert { condition } => {
                if let Some(ty) = self.infer_expr(condition)
                    && ty != Type::Bool
                {
                    self.errors.push(VerunError::TypeMismatch {
                        expected: "bool".to_string(),
                        found: format!("{:?}", ty),
                        span: Some(condition.span),
                    });
                }
                self.check_expr_no_old(condition);
            }
            Statement::If {
                condition,
                then_body,
                else_body,
            } => {
                if let Some(ty) = self.infer_expr(condition)
                    && ty != Type::Bool
                {
                    self.errors.push(VerunError::TypeMismatch {
                        expected: "bool".to_string(),
                        found: format!("{:?}", ty),
                        span: Some(condition.span),
                    });
                }
                for s in then_body {
                    self.check_statement(s);
                }
                if let Some(else_stmts) = else_body {
                    for s in else_stmts {
                        self.check_statement(s);
                    }
                }
            }
            Statement::Let { name, ty, value } => {
                let inferred = self.infer_expr(value);
                if let Some(explicit_ty) = ty {
                    let resolved = match resolve_type(&explicit_ty.node, &self.env) {
                        Ok(r) => r,
                        Err(e) => {
                            self.errors.push(e);
                            explicit_ty.node.clone()
                        }
                    };
                    if let Some(ref val_ty) = inferred
                        && !types_compatible(&resolved, val_ty)
                    {
                        self.errors.push(VerunError::TypeMismatch {
                            expected: format!("{:?}", resolved),
                            found: format!("{:?}", val_ty),
                            span: Some(value.span),
                        });
                    }
                    self.env.define_var(&name.node, resolved);
                } else if let Some(val_ty) = inferred {
                    self.env.define_var(&name.node, val_ty);
                }
                self.check_expr_no_old(value);
            }
            Statement::Match { expr, arms } => {
                let expr_ty = self.infer_expr(expr);
                for arm in arms {
                    for s in &arm.body {
                        self.check_statement(s);
                    }
                }
                if let Some(ty) = &expr_ty
                    && let Type::Named(name) | Type::Enum(name) = ty
                    && let Some(variants) = self.env.lookup_enum(name)
                {
                    let mut has_wildcard = false;
                    let mut covered: std::collections::HashSet<String> =
                        std::collections::HashSet::new();
                    for arm in arms {
                        match &arm.pattern.node {
                            MatchPattern::Wildcard => has_wildcard = true,
                            MatchPattern::EnumVariant { variant, .. } => {
                                covered.insert(variant.clone());
                            }
                            _ => {}
                        }
                    }
                    if !has_wildcard {
                        for v in variants {
                            if !covered.contains(v) {
                                self.errors.push(VerunError::NonExhaustiveMatch {
                                    missing: (*v).clone(),
                                    span: Some(expr.span),
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    fn check_function(&mut self, func: &FnDef) {
        self.env.push_scope();

        for param in &func.params {
            match resolve_type(&param.ty.node, &self.env) {
                Ok(resolved) => self.env.define_var(&param.name.node, resolved),
                Err(e) => {
                    self.errors.push(e);
                    self.env.define_var(&param.name.node, param.ty.node.clone());
                }
            }
        }

        if let Some(body) = &func.body {
            for stmt in body {
                self.check_statement(stmt);
            }
        }

        self.env.pop_scope();
    }

    pub fn infer_expr(&mut self, expr: &Spanned<Expr>) -> Option<Type> {
        match &expr.node {
            Expr::IntLit(_) => Some(Type::Int),
            Expr::RealLit(_) => Some(Type::Real),
            Expr::BoolLit(_) => Some(Type::Bool),
            Expr::StringLit(_) => Some(Type::String),
            Expr::Ident(name) => self.env.lookup_var(name).cloned(),
            Expr::UnaryOp { op, operand } => {
                let inner = self.infer_expr(operand)?;
                match op {
                    UnaryOp::Neg => Some(inner),
                    UnaryOp::Not => Some(Type::Bool),
                }
            }
            Expr::BinaryOp { left, op, right } => {
                let left_ty = self.infer_expr(left)?;
                let _right_ty = self.infer_expr(right)?;
                match op {
                    BinaryOp::Eq
                    | BinaryOp::Neq
                    | BinaryOp::Lt
                    | BinaryOp::Gt
                    | BinaryOp::Lte
                    | BinaryOp::Gte
                    | BinaryOp::And
                    | BinaryOp::Or
                    | BinaryOp::Implies => Some(Type::Bool),
                    _ => Some(left_ty),
                }
            }
            Expr::Old(inner) => {
                if !self.in_postcondition {
                    self.errors.push(VerunError::OldOutsideEnsure {
                        span: Some(expr.span),
                    });
                } else if let Expr::Ident(name) = &inner.node
                    && self.param_names.contains(name.as_str())
                {
                    self.errors.push(VerunError::OldOnParameter {
                        name: name.clone(),
                        span: Some(expr.span),
                    });
                }
                self.infer_expr(inner)
            }
            Expr::Forall { .. } | Expr::Exists { .. } => Some(Type::Bool),
            Expr::FnCall { name, args } => self.infer_call(expr.span, &name.node, args),
            Expr::FieldAccess { object, field } => {
                let obj_ty = self.infer_expr(object)?;
                if let Type::Named(name) = &obj_ty
                    && let Some(TypeEntry::Struct(fields)) = self.env.lookup_type(name)
                {
                    return fields.get(&field.node).cloned();
                }
                None
            }
            Expr::EnumVariant { enum_name, variant } => {
                if !self.env.is_valid_enum_variant(enum_name, variant) {
                    if self.env.lookup_enum(enum_name).is_some() {
                        self.errors.push(VerunError::InvalidEnumVariant {
                            enum_name: enum_name.clone(),
                            variant: variant.clone(),
                            span: Some(expr.span),
                        });
                    } else {
                        self.errors.push(VerunError::UndefinedType {
                            name: enum_name.clone(),
                            span: Some(expr.span),
                        });
                    }
                }
                Some(Type::Enum(enum_name.clone()))
            }
            Expr::Range { .. } => None,
            Expr::IndexAccess { object, .. } => {
                let obj_ty = self.infer_expr(object)?;
                match obj_ty {
                    Type::Array { element, .. } => Some(*element),
                    Type::Map { value, .. } => Some(*value),
                    _ => None,
                }
            }
            Expr::MapAccess { map, .. } => {
                let map_ty = self.infer_expr(map)?;
                if let Type::Map { value, .. } = map_ty {
                    Some(*value)
                } else {
                    None
                }
            }
        }
    }
    fn infer_call(
        &mut self,
        span: crate::ast::span::Span,
        name: &str,
        args: &[Spanned<Expr>],
    ) -> Option<Type> {
        if let Some(ty) = self.infer_builtin_call(name, args) {
            return Some(ty);
        }

        let sig = match self.env.lookup_function(name) {
            Some(sig) => sig.clone(),
            None => {
                self.errors.push(VerunError::UndefinedVariable {
                    name: name.to_string(),
                    span: Some(span),
                });
                return None;
            }
        };

        if sig.params.len() != args.len() {
            self.errors.push(VerunError::TypeMismatch {
                expected: format!("{} argument(s)", sig.params.len()),
                found: format!("{}", args.len()),
                span: Some(span),
            });
            for arg in args {
                let _ = self.infer_expr(arg);
            }
            return Some(sig.return_type);
        }

        for (arg_expr, param_ty) in args.iter().zip(sig.params.iter()) {
            if let Some(arg_ty) = self.infer_expr(arg_expr)
                && !types_compatible(param_ty, &arg_ty)
            {
                self.errors.push(VerunError::TypeMismatch {
                    expected: format!("{:?}", param_ty),
                    found: format!("{:?}", arg_ty),
                    span: Some(arg_expr.span),
                });
            }
        }

        Some(sig.return_type)
    }

    fn infer_builtin_call(&mut self, name: &str, args: &[Spanned<Expr>]) -> Option<Type> {
        match name {
            "abs" => {
                if args.len() == 1 {
                    self.infer_expr(&args[0])
                } else {
                    None
                }
            }
            "min" | "max" => {
                if args.len() == 2 {
                    self.infer_expr(&args[0])
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

impl Default for TypeChecker {
    fn default() -> Self {
        Self::new()
    }
}
