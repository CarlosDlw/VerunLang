use serde::{Deserialize, Serialize};

use crate::ast::span::Span;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Counterexample {
    pub description: String,
    pub values: Vec<CounterexampleValue>,
    pub span: Option<Span>,
    pub expression: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CounterexampleValue {
    pub name: String,
    pub value: String,
}

impl Counterexample {
    pub fn new(description: String, values: Vec<(String, String)>, span: Option<Span>) -> Self {
        Self {
            description,
            values: values
                .into_iter()
                .map(|(name, value)| CounterexampleValue { name, value })
                .collect(),
            span,
            expression: None,
        }
    }

    pub fn with_expression(mut self, expr: String) -> Self {
        self.expression = Some(expr);
        self
    }

    pub fn format_readable(&self) -> String {
        let mut out = format!("  {}\n", self.description);

        if let Some(expr) = &self.expression {
            out.push_str(&format!("    expression: {}\n", expr));
        }

        if self.values.is_empty() {
            return out;
        }

        let mut pre_vals: Vec<(&str, &str)> = Vec::new();
        let mut post_vals: Vec<(&str, &str)> = Vec::new();
        let mut param_vals: Vec<(&str, &str)> = Vec::new();
        let mut other_vals: Vec<(&str, &str)> = Vec::new();
        let mut seen_fields: std::collections::HashSet<String> = std::collections::HashSet::new();

        for val in &self.values {
            if let Some(name) = val.name.strip_prefix("pre_") {
                pre_vals.push((name, &val.value));
                seen_fields.insert(name.to_string());
            } else if let Some(name) = val.name.strip_prefix("post_") {
                post_vals.push((name, &val.value));
                seen_fields.insert(name.to_string());
            } else if let Some(name) = val.name.strip_prefix("param_") {
                param_vals.push((name, &val.value));
            } else if !seen_fields.contains(&val.name) {
                other_vals.push((&val.name, &val.value));
            }
        }

        if !param_vals.is_empty() {
            out.push_str("    Parameters:\n");
            for (name, value) in &param_vals {
                out.push_str(&format!("      {} = {}\n", name, value));
            }
        }

        if !pre_vals.is_empty() || !post_vals.is_empty() {
            let post_map: std::collections::HashMap<&str, &str> =
                post_vals.iter().copied().collect();
            let mut transitions: Vec<String> = Vec::new();
            for (name, pre_val) in &pre_vals {
                if let Some(post_val) = post_map.get(name) {
                    if pre_val != post_val {
                        transitions.push(format!("      {}: {} -> {}", name, pre_val, post_val));
                    } else {
                        transitions.push(format!("      {} = {} (unchanged)", name, pre_val));
                    }
                } else {
                    transitions.push(format!("      {} = {} (pre)", name, pre_val));
                }
            }
            for (name, post_val) in &post_vals {
                if !pre_vals.iter().any(|(n, _)| n == name) {
                    transitions.push(format!("      {} = {} (post)", name, post_val));
                }
            }
            if !transitions.is_empty() {
                out.push_str("    State:\n");
                for t in &transitions {
                    out.push_str(t);
                    out.push('\n');
                }
            }
        }

        if !other_vals.is_empty() {
            for (name, value) in &other_vals {
                out.push_str(&format!("    {} = {}\n", name, value));
            }
        }

        out
    }
}
