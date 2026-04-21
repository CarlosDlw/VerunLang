use crate::ast::nodes::*;
use crate::ast::types::{EnumDef, Field, Type, TypeDef};

pub fn format_program(program: &Program) -> String {
    let mut out = String::new();
    let mut first = true;

    for item in &program.items {
        if !first {
            out.push('\n');
        }
        first = false;

        match &item.node {
            Item::Import(import) => out.push_str(&format_import(import)),
            Item::EnumDef(e) => out.push_str(&format_enum(e)),
            Item::TypeDef(t) => out.push_str(&format_type_def(t)),
            Item::State(s) => out.push_str(&format_state(s)),
            Item::Function(f) => out.push_str(&format_function(f)),
            Item::ConstDef(c) => out.push_str(&format_const_def(c)),
        }
    }

    out
}

fn format_import(import: &Import) -> String {
    let mut out = format!("import \"{}\"", import.path.node);
    if let Some(alias) = &import.alias {
        out.push_str(&format!(" as {}", alias.node));
    }
    out.push('\n');
    out
}

fn format_enum(e: &EnumDef) -> String {
    let mut out = format!("enum {} {{\n", e.name.node);
    for (i, variant) in e.variants.iter().enumerate() {
        out.push_str(&format!("    {}", variant.node));
        if i < e.variants.len() - 1 {
            out.push(',');
        }
        out.push('\n');
    }
    out.push_str("}\n");
    out
}

fn format_type_def(t: &TypeDef) -> String {
    if let Some(alias) = &t.alias {
        let mut out = format!("type {} = {}", t.name.node, format_type(&alias.node));
        if let Some(refinement) = &t.refinement {
            out.push_str(&format!(" where {}", format_expr(&refinement.node)));
        }
        out.push('\n');
        return out;
    }
    let mut out = format!("type {} {{\n", t.name.node);
    for field in &t.fields {
        out.push_str(&format_field(field, 1));
    }
    out.push_str("}\n");
    out
}

fn format_field(field: &Field, indent: usize) -> String {
    let pad = "    ".repeat(indent);
    format!(
        "{}{}: {}\n",
        pad,
        field.name.node,
        format_type(&field.ty.node)
    )
}

fn format_type(ty: &Type) -> String {
    match ty {
        Type::Int => "int".to_string(),
        Type::Real => "real".to_string(),
        Type::Bool => "bool".to_string(),
        Type::String => "string".to_string(),
        Type::Named(name) | Type::Enum(name) => name.clone(),
        Type::Array { element, size } => format!("{}[{}]", format_type(element), size),
        Type::Map { key, value } => format!("map[{}, {}]", format_type(key), format_type(value)),
    }
}

fn format_state(state: &StateDef) -> String {
    let mut out = format!("state {} {{\n", state.name.node);

    for constant in &state.constants {
        out.push_str(&format!(
            "    {}",
            format_const_def(constant).trim_end_matches('\n')
        ));
        out.push('\n');
    }

    if !state.constants.is_empty() && !state.fields.is_empty() {
        out.push('\n');
    }

    for field in &state.fields {
        out.push_str(&format_field_decl(field));
    }

    if !state.fields.is_empty() && !state.invariants.is_empty() {
        out.push('\n');
    }

    for inv in &state.invariants {
        out.push_str(&format_invariant(inv));
    }

    if !state.invariants.is_empty() && state.init.is_some() {
        out.push('\n');
    }

    if let Some(init) = &state.init {
        out.push_str(&format_init(init));
    }

    if state.init.is_some() && !state.transitions.is_empty() {
        out.push('\n');
    }

    for (i, transition) in state.transitions.iter().enumerate() {
        out.push_str(&format_transition(transition));
        if i < state.transitions.len() - 1 {
            out.push('\n');
        }
    }

    out.push_str("}\n");
    out
}

fn format_field_decl(field: &Field) -> String {
    format!("    {}: {}\n", field.name.node, format_type(&field.ty.node))
}

fn format_invariant(inv: &Invariant) -> String {
    let mut out = String::from("    invariant");
    if let Some(name) = &inv.name {
        out.push_str(&format!(" {}", name.node));
    }
    out.push_str(" {\n");
    out.push_str(&format!("        {}\n", format_expr(&inv.condition.node)));
    out.push_str("    }\n");
    out
}

fn format_init(init: &InitBlock) -> String {
    let mut out = String::from("    init {\n");
    for (i, assign) in init.assignments.iter().enumerate() {
        out.push_str(&format!(
            "        {} = {}",
            assign.target.node,
            format_expr(&assign.value.node)
        ));
        if i < init.assignments.len() - 1 {
            out.push(',');
        }
        out.push('\n');
    }
    out.push_str("    }\n");
    out
}

fn format_transition(t: &Transition) -> String {
    let mut out = String::from("    transition ");
    out.push_str(&t.name.node);
    out.push('(');

    let params: Vec<String> = t
        .params
        .iter()
        .map(|p| format!("{}: {}", p.name.node, format_type(&p.ty.node)))
        .collect();
    out.push_str(&params.join(", "));
    out.push_str(") {\n");

    if !t.preconditions.is_empty() {
        out.push_str("        where {\n");
        for (i, pre) in t.preconditions.iter().enumerate() {
            out.push_str(&format!("            {}", format_expr(&pre.node)));
            if i < t.preconditions.len() - 1 {
                out.push(',');
            }
            out.push('\n');
        }
        out.push_str("        }\n");
    }

    for stmt in &t.body {
        out.push_str(&format_statement(&stmt.node, 2));
    }

    if !t.postconditions.is_empty() {
        out.push_str("        ensure {\n");
        for (i, post) in t.postconditions.iter().enumerate() {
            out.push_str(&format!("            {}", format_expr(&post.node)));
            if i < t.postconditions.len() - 1 {
                out.push(',');
            }
            out.push('\n');
        }
        out.push_str("        }\n");
    }

    for emit in &t.emits {
        let args: Vec<String> = emit
            .node
            .args
            .iter()
            .map(|a| format_expr(&a.node))
            .collect();
        out.push_str(&format!(
            "        emit {}({})\n",
            emit.node.event_name.node,
            args.join(", ")
        ));
    }

    out.push_str("    }\n");
    out
}

fn format_statement(stmt: &Statement, indent: usize) -> String {
    let pad = "    ".repeat(indent);
    match stmt {
        Statement::Assign(assign) => {
            format!(
                "{}{} = {}\n",
                pad,
                assign.target.node,
                format_expr(&assign.value.node)
            )
        }
        Statement::CompoundAssign { target, op, value } => {
            let op_str = match op {
                CompoundOp::Add => "+=",
                CompoundOp::Sub => "-=",
                CompoundOp::Mul => "*=",
                CompoundOp::Div => "/=",
            };
            format!(
                "{}{} {} {}\n",
                pad,
                target.node,
                op_str,
                format_expr(&value.node)
            )
        }
        Statement::If {
            condition,
            then_body,
            else_body,
        } => {
            let mut out = format!("{}if {} {{\n", pad, format_expr(&condition.node));
            for s in then_body {
                out.push_str(&format_statement(&s.node, indent + 1));
            }
            if let Some(else_stmts) = else_body {
                if else_stmts.len() == 1
                    && let Statement::If { .. } = &else_stmts[0].node
                {
                    out.push_str(&format!("{}}} else ", pad));
                    let nested = format_statement(&else_stmts[0].node, indent);
                    out.push_str(nested.trim_start());
                    return out;
                }
                out.push_str(&format!("{}}} else {{\n", pad));
                for s in else_stmts {
                    out.push_str(&format_statement(&s.node, indent + 1));
                }
            }
            out.push_str(&format!("{}}}\n", pad));
            out
        }
        Statement::IndexedAssign {
            target,
            index,
            value,
        } => {
            format!(
                "{}{}[{}] = {}\n",
                pad,
                target.node,
                format_expr(&index.node),
                format_expr(&value.node)
            )
        }
        Statement::IndexedCompoundAssign {
            target,
            index,
            op,
            value,
        } => {
            let op_str = match op {
                CompoundOp::Add => "+=",
                CompoundOp::Sub => "-=",
                CompoundOp::Mul => "*=",
                CompoundOp::Div => "/=",
            };
            format!(
                "{}{}[{}] {} {}\n",
                pad,
                target.node,
                format_expr(&index.node),
                op_str,
                format_expr(&value.node)
            )
        }
        Statement::Assert { condition } => {
            format!("{}assert {}\n", pad, format_expr(&condition.node))
        }
        Statement::Let { name, ty, value } => {
            if let Some(t) = ty {
                format!(
                    "{}let {}: {} = {}\n",
                    pad,
                    name.node,
                    format_type(&t.node),
                    format_expr(&value.node)
                )
            } else {
                format!("{}let {} = {}\n", pad, name.node, format_expr(&value.node))
            }
        }
        Statement::Match { expr, arms } => {
            let mut out = format!("{}match {} {{\n", pad, format_expr(&expr.node));
            for arm in arms {
                out.push_str(&format!(
                    "{}    {} => {{\n",
                    pad,
                    format_match_pattern(&arm.pattern.node)
                ));
                for s in &arm.body {
                    out.push_str(&format_statement(&s.node, indent + 2));
                }
                out.push_str(&format!("{}    }},\n", pad));
            }
            out.push_str(&format!("{}}}\n", pad));
            out
        }
    }
}

fn format_const_def(c: &ConstDef) -> String {
    format!(
        "const {}: {} = {}\n",
        c.name.node,
        format_type(&c.ty.node),
        format_expr(&c.value.node)
    )
}

fn format_match_pattern(p: &MatchPattern) -> String {
    match p {
        MatchPattern::EnumVariant { enum_name, variant } => format!("{}::{}", enum_name, variant),
        MatchPattern::IntLit(v) => format!("{}", v),
        MatchPattern::BoolLit(v) => format!("{}", v),
        MatchPattern::StringLit(v) => format!("\"{}\"", v),
        MatchPattern::Wildcard => "_".to_string(),
    }
}

pub fn format_expr(expr: &Expr) -> String {
    match expr {
        Expr::IntLit(v) => format!("{}", v),
        Expr::RealLit(v) => {
            let s = format!("{}", v);
            if s.contains('.') {
                s
            } else {
                format!("{}.0", s)
            }
        }
        Expr::BoolLit(v) => format!("{}", v),
        Expr::StringLit(v) => format!("\"{}\"", v),
        Expr::Ident(name) => name.clone(),
        Expr::EnumVariant { enum_name, variant } => format!("{}::{}", enum_name, variant),
        Expr::Old(inner) => format!("old({})", format_expr(&inner.node)),
        Expr::UnaryOp { op, operand } => {
            let inner = format_expr(&operand.node);
            match op {
                UnaryOp::Neg => format!("-{}", inner),
                UnaryOp::Not => format!("!{}", inner),
            }
        }
        Expr::BinaryOp { left, op, right } => {
            let l = format_expr(&left.node);
            let r = format_expr(&right.node);
            let op_str = match op {
                BinaryOp::Add => "+",
                BinaryOp::Sub => "-",
                BinaryOp::Mul => "*",
                BinaryOp::Div => "/",
                BinaryOp::Mod => "%",
                BinaryOp::Eq => "==",
                BinaryOp::Neq => "!=",
                BinaryOp::Lt => "<",
                BinaryOp::Gt => ">",
                BinaryOp::Lte => "<=",
                BinaryOp::Gte => ">=",
                BinaryOp::And => "&&",
                BinaryOp::Or => "||",
                BinaryOp::Implies => "==>",
            };
            format!("{} {} {}", l, op_str, r)
        }
        Expr::FieldAccess { object, field } => {
            format!("{}.{}", format_expr(&object.node), field.node)
        }
        Expr::IndexAccess { object, index } => {
            format!(
                "{}[{}]",
                format_expr(&object.node),
                format_expr(&index.node)
            )
        }
        Expr::MapAccess { map, key } => {
            format!("{}[{}]", format_expr(&map.node), format_expr(&key.node))
        }
        Expr::FnCall { name, args } => {
            let arg_strs: Vec<String> = args.iter().map(|a| format_expr(&a.node)).collect();
            format!("{}({})", name.node, arg_strs.join(", "))
        }
        Expr::Forall { var, domain, body } => {
            format!(
                "forall {} in {}: {}",
                var.node,
                format_expr(&domain.node),
                format_expr(&body.node)
            )
        }
        Expr::Exists { var, domain, body } => {
            format!(
                "exists {} in {}: {}",
                var.node,
                format_expr(&domain.node),
                format_expr(&body.node)
            )
        }
        Expr::Range { start, end } => {
            format!("{}..{}", format_expr(&start.node), format_expr(&end.node))
        }
    }
}

fn format_function(func: &FnDef) -> String {
    let mut out = String::new();
    if func.is_extern {
        out.push_str("extern ");
    }
    out.push_str("fn ");
    out.push_str(&func.name.node);
    out.push('(');

    let params: Vec<String> = func
        .params
        .iter()
        .map(|p| format!("{}: {}", p.name.node, format_type(&p.ty.node)))
        .collect();
    out.push_str(&params.join(", "));
    out.push_str(&format!(") -> {}", format_type(&func.return_type.node)));

    if let Some(body) = &func.body {
        out.push_str(" {\n");
        for stmt in body {
            out.push_str(&format_statement(&stmt.node, 1));
        }
        out.push('}');
    }
    out.push('\n');
    out
}
