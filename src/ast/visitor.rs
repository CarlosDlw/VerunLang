use super::nodes::*;
use super::span::Spanned;

pub trait Visitor {
    fn visit_program(&mut self, program: &Program) {
        for item in &program.items {
            self.visit_item(item);
        }
    }

    fn visit_item(&mut self, item: &Spanned<Item>) {
        match &item.node {
            Item::Import(import) => self.visit_import(import),
            Item::EnumDef(enum_def) => self.visit_enum_def(enum_def),
            Item::TypeDef(type_def) => self.visit_type_def(type_def),
            Item::State(state) => self.visit_state(state),
            Item::Function(func) => self.visit_function(func),
            Item::ConstDef(_) => {}
        }
    }

    fn visit_import(&mut self, _import: &Import) {}
    fn visit_enum_def(&mut self, _enum_def: &super::types::EnumDef) {}
    fn visit_type_def(&mut self, _type_def: &super::types::TypeDef) {}

    fn visit_state(&mut self, state: &StateDef) {
        for inv in &state.invariants {
            self.visit_invariant(inv);
        }
        if let Some(init) = &state.init {
            self.visit_init(init);
        }
        for transition in &state.transitions {
            self.visit_transition(transition);
        }
    }

    fn visit_invariant(&mut self, inv: &Invariant) {
        self.visit_expr(&inv.condition);
    }

    fn visit_init(&mut self, init: &InitBlock) {
        for assign in &init.assignments {
            self.visit_assignment(assign);
        }
    }

    fn visit_transition(&mut self, transition: &Transition) {
        for pre in &transition.preconditions {
            self.visit_expr(pre);
        }
        for stmt in &transition.body {
            self.visit_statement(stmt);
        }
        for post in &transition.postconditions {
            self.visit_expr(post);
        }
    }

    fn visit_assignment(&mut self, assign: &Assignment) {
        self.visit_expr(&assign.value);
    }

    fn visit_statement(&mut self, stmt: &Spanned<Statement>) {
        match &stmt.node {
            Statement::Assign(assign) => self.visit_assignment(assign),
            Statement::CompoundAssign { value, .. } => self.visit_expr(value),
            Statement::IndexedAssign { index, value, .. } => {
                self.visit_expr(index);
                self.visit_expr(value);
            }
            Statement::IndexedCompoundAssign { index, value, .. } => {
                self.visit_expr(index);
                self.visit_expr(value);
            }
            Statement::Assert { condition } => self.visit_expr(condition),
            Statement::If {
                condition,
                then_body,
                else_body,
            } => {
                self.visit_expr(condition);
                for s in then_body {
                    self.visit_statement(s);
                }
                if let Some(else_stmts) = else_body {
                    for s in else_stmts {
                        self.visit_statement(s);
                    }
                }
            }
            Statement::Let { value, .. } => self.visit_expr(value),
            Statement::Match { expr, arms } => {
                self.visit_expr(expr);
                for arm in arms {
                    for s in &arm.body {
                        self.visit_statement(s);
                    }
                }
            }
        }
    }

    fn visit_expr(&mut self, expr: &Spanned<Expr>) {
        match &expr.node {
            Expr::UnaryOp { operand, .. } => self.visit_expr(operand),
            Expr::BinaryOp { left, right, .. } => {
                self.visit_expr(left);
                self.visit_expr(right);
            }
            Expr::FnCall { args, .. } => {
                for arg in args {
                    self.visit_expr(arg);
                }
            }
            Expr::Old(inner) => self.visit_expr(inner),
            Expr::Forall { domain, body, .. } | Expr::Exists { domain, body, .. } => {
                self.visit_expr(domain);
                self.visit_expr(body);
            }
            Expr::FieldAccess { object, .. } => self.visit_expr(object),
            Expr::IndexAccess { object, index } => {
                self.visit_expr(object);
                self.visit_expr(index);
            }
            Expr::MapAccess { map, key } => {
                self.visit_expr(map);
                self.visit_expr(key);
            }
            Expr::Range { start, end } => {
                self.visit_expr(start);
                self.visit_expr(end);
            }
            _ => {}
        }
    }

    fn visit_function(&mut self, func: &FnDef) {
        if let Some(body) = &func.body {
            for stmt in body {
                self.visit_statement(stmt);
            }
        }
    }
}
