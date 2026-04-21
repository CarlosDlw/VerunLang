pub mod builder;
pub mod grammar;
pub mod imports;

use anyhow::Result;

use crate::ast::nodes::Program;
use crate::ast::span::Span;
use crate::errors::diagnostic::VerunError;
use builder::build_program;
use grammar::parse;
pub use imports::{LoadedProgram, parse_file_with_imports};

fn byte_offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let offset = offset.min(source.len());
    let before = &source[..offset];
    let line = before.bytes().filter(|&b| b == b'\n').count() + 1;
    let col = before.rfind('\n').map(|i| offset - i).unwrap_or(offset + 1);
    (line, col)
}

/// Scan from `offset` to find the actual offending token.
/// Skips horizontal whitespace only — does NOT cross newlines, so the token
/// stays on the same line as the reported error.
fn find_token_at(source: &str, offset: usize) -> (usize, String) {
    let bytes = source.as_bytes();
    let mut i = offset.min(source.len());
    // skip spaces and tabs only — not newlines
    while i < bytes.len() && (bytes[i] == b' ' || bytes[i] == b'\t' || bytes[i] == b'\r') {
        i += 1;
    }
    if i >= bytes.len() {
        return (i, "end of file".into());
    }
    // if we hit a newline, back up to the last real char on the current line
    if bytes[i] == b'\n' {
        let line_start = bytes[..i]
            .iter()
            .rposition(|&b| b == b'\n')
            .map(|p| p + 1)
            .unwrap_or(0);
        // scan backward to find last non-space char on this line
        let mut j = i.saturating_sub(1);
        while j >= line_start && (bytes[j] == b' ' || bytes[j] == b'\t' || bytes[j] == b'\r') {
            if j == 0 {
                break;
            }
            j -= 1;
        }
        i = j;
    }
    if i >= bytes.len() {
        return (i, "end of file".into());
    }
    let ch = bytes[i] as char;
    let tok = match ch {
        ',' => "','",
        ';' => "';'",
        ')' => "')'",
        '(' => "'('",
        '}' => "'}'",
        '{' => "'{'",
        ']' => "']'",
        '[' => "'['",
        ':' => "':'",
        '.' => "'.'",
        '=' => "'='",
        '!' => "'!'",
        '<' => "'<'",
        '>' => "'>'",
        '+' => "'+'",
        '-' => "'-'",
        '*' => "'*'",
        '/' => "'/'",
        '%' => "'%'",
        '&' => "'&'",
        '|' => "'|'",
        _ if ch.is_ascii_alphabetic() || ch == '_' => {
            let end = bytes[i..]
                .iter()
                .take_while(|&&b| (b as char).is_ascii_alphanumeric() || b == b'_')
                .count();
            return (i, format!("'{}'", &source[i..i + end]));
        }
        _ if ch.is_ascii_digit() => "number",
        '"' => "string literal",
        _ => "unexpected character",
    };
    (i, tok.into())
}

fn rule_to_display(rule: &grammar::Rule) -> String {
    use grammar::Rule::*;
    match rule {
        ident => "identifier".into(),
        integer_lit => "integer".into(),
        real_lit => "real number".into(),
        bool_lit => "bool literal".into(),
        string_lit => "string".into(),
        op_add => "'+'".into(),
        op_sub => "'-'".into(),
        op_mul => "'*'".into(),
        op_div => "'/'".into(),
        op_mod => "'%'".into(),
        op_eq => "'=='".into(),
        op_neq => "'!='".into(),
        op_lt => "'<'".into(),
        op_gt => "'>'".into(),
        op_lte => "'<='".into(),
        op_gte => "'>='".into(),
        op_and => "'&&'".into(),
        op_or => "'||'".into(),
        op_not | op_neg => "'!'".into(),
        op_implies => "'==>'".into(),
        op_range => "'..'".into(),
        expr => "expression".into(),
        statement => "statement".into(),
        transition_body => "transition body".into(),
        init_block => "'init' block".into(),
        where_clause => "'where' block".into(),
        ensure_clause => "'ensure' block".into(),
        assignment => "assignment".into(),
        param => "parameter".into(),
        qualified_ident => "qualified identifier".into(),
        enum_variant => "enum variant".into(),
        fn_call => "function call".into(),
        old_expr => "'old()'".into(),
        forall_expr => "'forall' expression".into(),
        exists_expr => "'exists' expression".into(),
        field_decl => "field declaration (name: type)".into(),
        invariant_decl => "'invariant' block".into(),
        transition_decl => "'transition' declaration".into(),
        const_decl => "'const' declaration".into(),
        state_def => "'state' declaration".into(),
        enum_def => "'enum' declaration".into(),
        type_def => "'type' declaration".into(),
        fn_def => "'fn' declaration".into(),
        import_decl => "'import' declaration".into(),
        let_stmt => "'let' binding".into(),
        if_stmt => "'if' statement".into(),
        match_stmt => "'match' statement".into(),
        assert_stmt => "'assert' statement".into(),
        type_expr => "type".into(),
        base_type => "type name".into(),
        array_type => "array type (T[N])".into(),
        map_type => "map type (map[K, V])".into(),
        r => format!("{:?}", r),
    }
}

const BINARY_OPS: &[grammar::Rule] = &[
    grammar::Rule::op_add,
    grammar::Rule::op_sub,
    grammar::Rule::op_mul,
    grammar::Rule::op_div,
    grammar::Rule::op_mod,
    grammar::Rule::op_eq,
    grammar::Rule::op_neq,
    grammar::Rule::op_lt,
    grammar::Rule::op_gt,
    grammar::Rule::op_lte,
    grammar::Rule::op_gte,
    grammar::Rule::op_and,
    grammar::Rule::op_or,
    grammar::Rule::op_implies,
    grammar::Rule::op_range,
];

fn format_expected(positives: &[grammar::Rule]) -> String {
    if positives.is_empty() {
        return String::new();
    }

    use grammar::Rule::*;
    let is_binary_op = |r: &grammar::Rule| BINARY_OPS.contains(r);

    // Collapse "all state body members" into a friendly phrase
    let state_body_rules = [
        const_decl,
        field_decl,
        invariant_decl,
        init_block,
        transition_decl,
    ];
    if state_body_rules.iter().all(|r| positives.contains(r)) {
        return "a state member (field, invariant, init, transition, or const)".into();
    }

    // Collapse "all top-level items" into a friendly phrase
    let top_level_rules = [
        state_def,
        enum_def,
        type_def,
        fn_def,
        import_decl,
        const_decl,
    ];
    if top_level_rules.iter().all(|r| positives.contains(r)) {
        return "a top-level declaration (state, enum, type, fn, import, or const)".into();
    }

    let all_binary = positives.iter().all(is_binary_op);
    let has_binary = positives.iter().any(is_binary_op);

    if all_binary {
        return "binary operator".into();
    }

    let mut parts: Vec<String> = Vec::new();
    if has_binary {
        parts.push("binary operator".into());
    }
    for r in positives {
        if !is_binary_op(r) {
            let s = rule_to_display(r);
            if !parts.contains(&s) {
                parts.push(s);
            }
        }
    }
    parts.join(", ")
}

pub fn parse_source(source: &str) -> Result<Program> {
    let pairs = parse(source).map_err(|e| {
        let raw_offset = match e.location {
            pest::error::InputLocation::Pos(p) => p,
            pest::error::InputLocation::Span((s, _)) => s,
        };

        let (tok_offset, tok_display) = find_token_at(source, raw_offset);
        let tok_end = (tok_offset + 1).min(source.len());
        let span = Span::new(tok_offset, tok_end);
        let (line, col) = byte_offset_to_line_col(source, tok_offset);

        let base_message = match &e.variant {
            pest::error::ErrorVariant::ParsingError {
                positives,
                negatives,
            } => {
                let expected = format_expected(positives);
                let neg_str: Vec<String> = negatives.iter().map(|r| rule_to_display(r)).collect();
                if !expected.is_empty() && !neg_str.is_empty() {
                    format!("expected {}; unexpected {}", expected, neg_str.join(", "))
                } else if !expected.is_empty() {
                    format!("expected {}, got {}", expected, tok_display)
                } else {
                    format!("unexpected {}", tok_display)
                }
            }
            pest::error::ErrorVariant::CustomError { message } => message.clone(),
        };
        let message = format!("{}:{}: {}", line, col, base_message);
        VerunError::ParseError {
            message,
            span: Some(span),
        }
    })?;
    build_program(pairs, source)
}
