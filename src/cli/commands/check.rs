use std::process;

use anyhow::Result;

use crate::ast::nodes::Item;
use crate::errors::diagnostic::VerunError;
use crate::errors::report::{render_error, render_errors};
use crate::parser::parse_file_with_imports;
use crate::smt::solver::Solver;
use crate::smt::verifier::{CheckKind, Verifier};
use crate::types::checker::TypeChecker;

pub fn execute(file: &str, verbose: bool, format: &str) -> Result<()> {
    let loaded = match parse_file_with_imports(file) {
        Ok(p) => p,
        Err(e) => {
            if let Some(parse_err) = e.downcast_ref::<VerunError>() {
                eprint!("{}", render_error(parse_err, "", file));
            } else {
                eprintln!("Error: {}", e);
            }
            process::exit(1);
        }
    };
    let program = loaded.program;
    let source = loaded.root_source;

    let mut checker = TypeChecker::new();
    let diagnostics = checker.check(&program);

    let errors: Vec<_> = diagnostics.iter()
        .filter(|e| e.severity() == crate::errors::diagnostic::Severity::Error)
        .collect();
    let warnings: Vec<_> = diagnostics.iter()
        .filter(|e| e.severity() == crate::errors::diagnostic::Severity::Warning)
        .collect();

    for w in &warnings {
        eprint!("{}", render_error(w, &source, file));
    }

    if !errors.is_empty() {
        let report = render_errors(&errors.iter().map(|e| (*e).clone()).collect::<Vec<_>>(), &source, file);
        eprint!("{}", report);
        process::exit(1);
    }

    if verbose {
        eprintln!("[verun] Type checking passed");
    }

    let solver = Solver::new();
    let ctx = solver.create_context();
    let mut verifier = Verifier::new(&ctx);

    for item in &program.items {
        match &item.node {
            Item::EnumDef(e) => verifier.register_enum(e),
            Item::Function(f) => verifier.register_function(f),
            Item::TypeDef(t) => verifier.register_type_def(t),
            Item::ConstDef(c) => verifier.register_const(c),
            _ => {}
        }
    }

    let mut all_passed = true;
    let mut total_checks = 0;
    let mut passed_checks = 0;
    let mut warning_checks = 0;

    for item in &program.items {
        if let Item::State(state) = &item.node {
            let result = verifier.verify_state(state);

            if verbose {
                eprintln!(
                    "\n[verun] Verifying state '{}'...",
                    result.state_name
                );
            }

            for check in &result.checks {
                total_checks += 1;
                let is_dead_transition = matches!(&check.kind, CheckKind::DeadTransition { .. });

                if check.passed {
                    passed_checks += 1;
                    if verbose {
                        eprintln!("  [PASS] {}", check.kind);
                    }
                } else if is_dead_transition {
                    warning_checks += 1;
                    eprintln!("  [WARN] {}", check.kind);
                    if verbose {
                        if let Some(ce) = &check.counterexample {
                            eprint!("{}", ce.format_readable());
                        }
                    }
                } else {
                    all_passed = false;
                    eprintln!("  [FAIL] {}", check.kind);
                    if let Some(ce) = &check.counterexample {
                        eprint!("{}", ce.format_readable());
                    }
                }
            }
        }
    }

    match format {
        "json" => {
            let output = serde_json::json!({
                "file": file,
                "total_checks": total_checks,
                "passed": passed_checks,
                "warnings": warning_checks,
                "failed": total_checks - passed_checks - warning_checks,
                "all_passed": all_passed,
            });
            println!("{}", serde_json::to_string_pretty(&output)?);
        }
        _ => {
            let mut summary = format!(
                "\n{}/{} checks passed",
                passed_checks,
                total_checks - warning_checks
            );
            if warning_checks > 0 {
                summary.push_str(&format!(", {} warning(s)", warning_checks));
            }
            if all_passed {
                summary.push_str(" — all verified");
            }
            println!("{}", summary);
        }
    }

    if !all_passed {
        process::exit(1);
    }

    Ok(())
}
