use std::cell::RefCell;
use std::collections::HashMap;

use z3::ast::{Array as Z3Array, Ast, Bool, Dynamic, Int, Real as Z3Real};
use z3::{Context, FuncDecl, Sort};

use crate::ast::nodes::*;
use crate::ast::span::Spanned;
use crate::ast::types::{EnumDef, Type};

pub struct Encoder<'ctx> {
    pub ctx: &'ctx Context,
    enum_variants: HashMap<String, HashMap<String, i64>>,
    functions: HashMap<String, FuncDecl<'ctx>>,
    warnings: RefCell<Vec<String>>,
}

impl<'ctx> Encoder<'ctx> {
    pub fn new(ctx: &'ctx Context) -> Self {
        Self {
            ctx,
            enum_variants: HashMap::new(),
            functions: HashMap::new(),
            warnings: RefCell::new(Vec::new()),
        }
    }

    pub fn take_warnings(&self) -> Vec<String> {
        self.warnings.borrow_mut().drain(..).collect()
    }

    fn warn(&self, msg: String) {
        self.warnings.borrow_mut().push(msg);
    }

    pub fn register_enum(&mut self, def: &EnumDef) {
        let mut variants = HashMap::new();
        for (i, variant) in def.variants.iter().enumerate() {
            variants.insert(variant.node.clone(), i as i64);
        }
        self.enum_variants.insert(def.name.node.clone(), variants);
    }

    pub fn register_function(&mut self, func: &crate::ast::nodes::FnDef) {
        let param_sorts: Vec<Sort> = func
            .params
            .iter()
            .map(|p| self.encode_type(&p.ty.node))
            .collect();
        let param_sort_refs: Vec<&Sort> = param_sorts.iter().collect();
        let return_sort = self.encode_type(&func.return_type.node);
        let decl = FuncDecl::new(
            self.ctx,
            func.name.node.as_str(),
            &param_sort_refs,
            &return_sort,
        );
        self.functions.insert(func.name.node.clone(), decl);
    }

    pub fn encode_type(&self, ty: &Type) -> Sort<'ctx> {
        match ty {
            Type::Int => Sort::int(self.ctx),
            Type::Real => Sort::real(self.ctx),
            Type::Bool => Sort::bool(self.ctx),
            _ => Sort::int(self.ctx),
        }
    }

    pub fn encode_state_var(&self, name: &str, ty: &Type, prefix: &str) -> Dynamic<'ctx> {
        let full_name = format!("{}_{}", prefix, name);
        match ty {
            Type::Int => Dynamic::from_ast(&Int::new_const(self.ctx, full_name.as_str())),
            Type::Real => Dynamic::from_ast(&Z3Real::new_const(self.ctx, full_name.as_str())),
            Type::Bool => Dynamic::from_ast(&Bool::new_const(self.ctx, full_name.as_str())),
            Type::Array { element, .. } => {
                let elem_sort = self.encode_type(element);
                Dynamic::from_ast(&Z3Array::new_const(
                    self.ctx,
                    full_name.as_str(),
                    &Sort::int(self.ctx),
                    &elem_sort,
                ))
            }
            Type::Map { key, value } => {
                let key_sort = self.encode_type(key);
                let val_sort = self.encode_type(value);
                Dynamic::from_ast(&Z3Array::new_const(
                    self.ctx,
                    full_name.as_str(),
                    &key_sort,
                    &val_sort,
                ))
            }
            _ => Dynamic::from_ast(&Int::new_const(self.ctx, full_name.as_str())),
        }
    }

    pub fn encode_expr(
        &self,
        expr: &Spanned<Expr>,
        vars: &std::collections::HashMap<String, Dynamic<'ctx>>,
    ) -> Option<Dynamic<'ctx>> {
        match &expr.node {
            Expr::IntLit(val) => Some(Dynamic::from_ast(&Int::from_i64(self.ctx, *val))),
            Expr::RealLit(val) => {
                let scaled = (*val * 1_000_000.0).round() as i64;
                let r = Z3Real::from_real(self.ctx, scaled as i32, 1_000_000);
                Some(Dynamic::from_ast(&r))
            }
            Expr::BoolLit(val) => Some(Dynamic::from_ast(&Bool::from_bool(self.ctx, *val))),
            Expr::Ident(name) => vars.get(name).cloned(),
            Expr::UnaryOp { op, operand } => {
                let inner = self.encode_expr(operand, vars)?;
                match op {
                    UnaryOp::Neg => {
                        if let Some(i) = inner.as_int() {
                            Some(Dynamic::from_ast(&i.unary_minus()))
                        } else if let Some(r) = inner.as_real() {
                            Some(Dynamic::from_ast(&r.unary_minus()))
                        } else {
                            None
                        }
                    }
                    UnaryOp::Not => {
                        let b = inner.as_bool()?;
                        Some(Dynamic::from_ast(&b.not()))
                    }
                }
            }
            Expr::BinaryOp { left, op, right } => {
                let l = self.encode_expr(left, vars)?;
                let r = self.encode_expr(right, vars)?;
                self.encode_binary_op(&l, op, &r)
            }
            Expr::Old(inner) => {
                let mut old_vars = vars.clone();
                for (name, _) in vars {
                    if let Some(stripped) = name.strip_prefix("pre_") {
                        old_vars.insert(stripped.to_string(), vars.get(name).unwrap().clone());
                    }
                }
                self.encode_expr(inner, &old_vars)
            }
            Expr::Forall { var, domain, body } => {
                self.encode_quantifier(true, var, domain, body, vars)
            }
            Expr::Exists { var, domain, body } => {
                self.encode_quantifier(false, var, domain, body, vars)
            }
            Expr::EnumVariant { enum_name, variant } => {
                let val = self.enum_variants.get(enum_name)?.get(variant)?;
                Some(Dynamic::from_ast(&Int::from_i64(self.ctx, *val)))
            }
            Expr::IndexAccess { object, index } => {
                let obj = self.encode_expr(object, vars)?;
                let idx = self.encode_expr(index, vars)?;
                let arr = obj.as_array()?;
                Some(arr.select(&idx))
            }
            Expr::MapAccess { map, key } => {
                let obj = self.encode_expr(map, vars)?;
                let k = self.encode_expr(key, vars)?;
                let arr = obj.as_array()?;
                Some(arr.select(&k))
            }
            Expr::FieldAccess { object, field } => {
                if let Expr::Ident(obj_name) = &object.node {
                    let flat_key = format!("{}_{}", obj_name, field.node);
                    vars.get(&flat_key)
                        .cloned()
                        .or_else(|| vars.get(&format!("{}.{}", obj_name, field.node)).cloned())
                } else {
                    None
                }
            }
            Expr::FnCall { name, args } => self
                .encode_builtin_call(&name.node, args, vars)
                .or_else(|| {
                    if let Some(decl) = self.functions.get(&name.node) {
                        let encoded_args: Vec<Dynamic<'ctx>> = args
                            .iter()
                            .filter_map(|a| self.encode_expr(a, vars))
                            .collect();
                        if encoded_args.len() == args.len() {
                            let arg_refs: Vec<&dyn z3::ast::Ast> = encoded_args
                                .iter()
                                .map(|a| a as &dyn z3::ast::Ast)
                                .collect();
                            Some(decl.apply(&arg_refs))
                        } else {
                            None
                        }
                    } else {
                        self.warn(format!("unknown function '{}' in SMT encoder", name.node));
                        None
                    }
                }),
            Expr::StringLit(_) => {
                self.warn("string literals not yet supported in SMT encoder".to_string());
                None
            }
            Expr::Range { .. } => None,
        }
    }

    fn encode_binary_op(
        &self,
        left: &Dynamic<'ctx>,
        op: &BinaryOp,
        right: &Dynamic<'ctx>,
    ) -> Option<Dynamic<'ctx>> {
        match op {
            BinaryOp::Add => self.encode_arith(left, right, |a, b| a + b, |a, b| a + b),
            BinaryOp::Sub => self.encode_arith(left, right, |a, b| a - b, |a, b| a - b),
            BinaryOp::Mul => self.encode_arith(left, right, |a, b| a * b, |a, b| a * b),
            BinaryOp::Div => self.encode_arith(left, right, |a, b| a / b, |a, b| a / b),
            BinaryOp::Mod => {
                let l = left.as_int()?;
                let r = right.as_int()?;
                Some(Dynamic::from_ast(&l.modulo(&r)))
            }
            BinaryOp::Eq => self
                .encode_comparison(left, right, |a, b| a._eq(b), |a, b| a._eq(b))
                .or_else(|| {
                    left.as_array()
                        .and_then(|la| right.as_array().map(|ra| Dynamic::from_ast(&la._eq(&ra))))
                }),
            BinaryOp::Neq => {
                let eq = self.encode_binary_op(left, &BinaryOp::Eq, right)?;
                let b = eq.as_bool()?;
                Some(Dynamic::from_ast(&b.not()))
            }
            BinaryOp::Lt => self.encode_comparison(left, right, |a, b| a.lt(b), |a, b| a.lt(b)),
            BinaryOp::Gt => self.encode_comparison(left, right, |a, b| a.gt(b), |a, b| a.gt(b)),
            BinaryOp::Lte => self.encode_comparison(left, right, |a, b| a.le(b), |a, b| a.le(b)),
            BinaryOp::Gte => self.encode_comparison(left, right, |a, b| a.ge(b), |a, b| a.ge(b)),
            BinaryOp::And => {
                let l = left.as_bool()?;
                let r = right.as_bool()?;
                Some(Dynamic::from_ast(&Bool::and(self.ctx, &[&l, &r])))
            }
            BinaryOp::Or => {
                let l = left.as_bool()?;
                let r = right.as_bool()?;
                Some(Dynamic::from_ast(&Bool::or(self.ctx, &[&l, &r])))
            }
            BinaryOp::Implies => {
                let l = left.as_bool()?;
                let r = right.as_bool()?;
                Some(Dynamic::from_ast(&l.implies(&r)))
            }
        }
    }

    fn encode_arith<F, G>(
        &self,
        left: &Dynamic<'ctx>,
        right: &Dynamic<'ctx>,
        int_op: F,
        real_op: G,
    ) -> Option<Dynamic<'ctx>>
    where
        F: FnOnce(&Int<'ctx>, &Int<'ctx>) -> Int<'ctx>,
        G: FnOnce(&Z3Real<'ctx>, &Z3Real<'ctx>) -> Z3Real<'ctx>,
    {
        if let (Some(l), Some(r)) = (left.as_int(), right.as_int()) {
            Some(Dynamic::from_ast(&int_op(&l, &r)))
        } else if let (Some(l), Some(r)) = (left.as_real(), right.as_real()) {
            Some(Dynamic::from_ast(&real_op(&l, &r)))
        } else {
            None
        }
    }

    fn encode_comparison<F, G>(
        &self,
        left: &Dynamic<'ctx>,
        right: &Dynamic<'ctx>,
        int_op: F,
        real_op: G,
    ) -> Option<Dynamic<'ctx>>
    where
        F: FnOnce(&Int<'ctx>, &Int<'ctx>) -> Bool<'ctx>,
        G: FnOnce(&Z3Real<'ctx>, &Z3Real<'ctx>) -> Bool<'ctx>,
    {
        if let (Some(l), Some(r)) = (left.as_int(), right.as_int()) {
            Some(Dynamic::from_ast(&int_op(&l, &r)))
        } else if let (Some(l), Some(r)) = (left.as_real(), right.as_real()) {
            Some(Dynamic::from_ast(&real_op(&l, &r)))
        } else if let (Some(l), Some(r)) = (left.as_bool(), right.as_bool()) {
            Some(Dynamic::from_ast(&l._eq(&r)))
        } else {
            None
        }
    }

    fn encode_builtin_call(
        &self,
        name: &str,
        args: &[Spanned<Expr>],
        vars: &std::collections::HashMap<String, Dynamic<'ctx>>,
    ) -> Option<Dynamic<'ctx>> {
        match name {
            "abs" if args.len() == 1 => {
                let x = self.encode_expr(&args[0], vars)?;
                if let Some(i) = x.as_int() {
                    let zero = Int::from_i64(self.ctx, 0);
                    let neg = i.unary_minus();
                    Some(Dynamic::from_ast(&i.ge(&zero).ite(&i, &neg)))
                } else if let Some(r) = x.as_real() {
                    let zero = Z3Real::from_real(self.ctx, 0, 1);
                    let neg = r.unary_minus();
                    Some(Dynamic::from_ast(&r.ge(&zero).ite(&r, &neg)))
                } else {
                    None
                }
            }
            "min" if args.len() == 2 => {
                let a = self.encode_expr(&args[0], vars)?;
                let b = self.encode_expr(&args[1], vars)?;
                if let (Some(ai), Some(bi)) = (a.as_int(), b.as_int()) {
                    Some(Dynamic::from_ast(&ai.le(&bi).ite(&ai, &bi)))
                } else if let (Some(ar), Some(br)) = (a.as_real(), b.as_real()) {
                    Some(Dynamic::from_ast(&ar.le(&br).ite(&ar, &br)))
                } else {
                    None
                }
            }
            "max" if args.len() == 2 => {
                let a = self.encode_expr(&args[0], vars)?;
                let b = self.encode_expr(&args[1], vars)?;
                if let (Some(ai), Some(bi)) = (a.as_int(), b.as_int()) {
                    Some(Dynamic::from_ast(&ai.ge(&bi).ite(&ai, &bi)))
                } else if let (Some(ar), Some(br)) = (a.as_real(), b.as_real()) {
                    Some(Dynamic::from_ast(&ar.ge(&br).ite(&ar, &br)))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn encode_quantifier(
        &self,
        is_forall: bool,
        var: &Spanned<String>,
        domain: &Spanned<Expr>,
        body: &Spanned<Expr>,
        vars: &std::collections::HashMap<String, Dynamic<'ctx>>,
    ) -> Option<Dynamic<'ctx>> {
        if let Expr::Range { start, end } = &domain.node {
            let bound_var = Int::new_const(self.ctx, var.node.as_str());
            let start_val = self.encode_expr(start, vars)?.as_int()?;
            let end_val = self.encode_expr(end, vars)?.as_int()?;

            let range_constraint = Bool::and(
                self.ctx,
                &[&bound_var.ge(&start_val), &bound_var.lt(&end_val)],
            );

            let mut inner_vars = vars.clone();
            inner_vars.insert(var.node.clone(), Dynamic::from_ast(&bound_var));
            let body_encoded = self.encode_expr(body, &inner_vars)?.as_bool()?;

            let pattern = &[];
            let bound_dyn = Dynamic::from_ast(&bound_var);
            let bound: &[&dyn Ast] = &[&bound_dyn];

            if is_forall {
                let implies = range_constraint.implies(&body_encoded);
                Some(Dynamic::from_ast(&z3::ast::forall_const(
                    self.ctx, bound, pattern, &implies,
                )))
            } else {
                let conj = Bool::and(self.ctx, &[&range_constraint, &body_encoded]);
                Some(Dynamic::from_ast(&z3::ast::exists_const(
                    self.ctx, bound, pattern, &conj,
                )))
            }
        } else {
            None
        }
    }
}
