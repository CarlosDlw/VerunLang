use std::process;

use anyhow::Result;

use crate::ast::nodes::Item;
use crate::codegen::c::CTarget;
use crate::codegen::cairo::CairoTarget;
use crate::codegen::go::GoTarget;
use crate::codegen::java::JavaTarget;
use crate::codegen::move_lang::MoveTarget;
use crate::codegen::rust::RustTarget;
use crate::codegen::solidity::SolidityTarget;
use crate::codegen::target::CodeTarget;
use crate::codegen::typescript::TypeScriptTarget;
use crate::codegen::vyper::VyperTarget;
use crate::errors::diagnostic::VerunError;
use crate::errors::report::{render_error, render_errors};
use crate::parser::parse_file_with_imports;
use crate::smt::solver::Solver;
use crate::smt::verifier::Verifier;
use crate::types::checker::TypeChecker;

pub fn execute(file: &str, target: &str, output: Option<&str>) -> Result<()> {
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

    // Verify before generating — only generate from verified specs
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

    let mut has_failures = false;
    for item in &program.items {
        if let Item::State(state) = &item.node {
            let result = verifier.verify_state(state);
            for check in &result.checks {
                if !check.passed {
                    let is_warning = matches!(
                        &check.kind,
                        crate::smt::verifier::CheckKind::DeadTransition { .. }
                    );
                    if is_warning {
                        eprintln!("  [WARN] {}", check.kind);
                    } else {
                        has_failures = true;
                        eprintln!("  [FAIL] {}", check.kind);
                        if let Some(ce) = &check.counterexample {
                            eprint!("{}", ce.format_readable());
                        }
                    }
                }
            }
        }
    }

    if has_failures {
        eprintln!("\nVerification failed — refusing to generate code from unverified spec.");
        eprintln!("Fix the spec or run 'verun check {}' for details.", file);
        process::exit(1);
    }

    let code_target: Box<dyn CodeTarget> = match target {
        "rust" | "rs" => Box::new(RustTarget),
        "typescript" | "ts" => Box::new(TypeScriptTarget),
        "solidity" | "sol" => Box::new(SolidityTarget),
        "go" => Box::new(GoTarget),
        "java" => Box::new(JavaTarget),
        "c" => Box::new(CTarget),
        "move" => Box::new(MoveTarget),
        "cairo" => Box::new(CairoTarget),
        "vyper" | "vy" => Box::new(VyperTarget),
        _ => {
            eprintln!("unsupported target: {}", target);
            process::exit(1);
        }
    };

    let generated = code_target.generate(&program);

    if let Some(out_path) = output {
        std::fs::write(out_path, &generated)?;
        eprintln!("Generated {} code written to: {}", code_target.name(), out_path);
    } else {
        println!("{}", generated);
    }

    Ok(())
}
