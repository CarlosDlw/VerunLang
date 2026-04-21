use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use verun::errors::diagnostic::Severity;
use verun::parser::parse_file_with_imports;
use verun::types::checker::TypeChecker;

fn unique_temp_dir() -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time before unix epoch")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("verun-import-tests-{}", nanos));
    fs::create_dir_all(&dir).expect("failed to create temp dir");
    dir
}

#[test]
fn resolve_relative_import_and_typecheck() {
    let dir = unique_temp_dir();

    let dep = dir.join("types.verun");
    let root = dir.join("root.verun");

    fs::write(
        &dep,
        r#"
enum Phase { Draft, Approved }
"#,
    )
    .expect("failed to write dep module");

    fs::write(
        &root,
        r#"
import "./types.verun"

state Workflow {
    phase: Phase

    init { phase = Phase::Draft }

    transition approve() {
        phase = Phase::Approved
        ensure { phase == Phase::Approved }
    }
}
"#,
    )
    .expect("failed to write root module");

    let loaded = parse_file_with_imports(root.to_str().expect("invalid root path"))
        .expect("failed to resolve imports");

    let mut checker = TypeChecker::new();
    let diags = checker.check(&loaded.program);
    let errors: Vec<_> = diags
        .iter()
        .filter(|d| d.severity() == Severity::Error)
        .collect();

    assert!(errors.is_empty(), "unexpected type errors: {:?}", errors);
}

#[test]
fn detect_import_cycle() {
    let dir = unique_temp_dir();

    let a = dir.join("a.verun");
    let b = dir.join("b.verun");

    fs::write(&a, "import \"./b.verun\"\n").expect("failed to write a.verun");
    fs::write(&b, "import \"./a.verun\"\n").expect("failed to write b.verun");

    let err = parse_file_with_imports(a.to_str().expect("invalid path for a"))
        .expect_err("expected import cycle error");
    let msg = format!("{}", err);

    assert!(msg.contains("import cycle detected"), "unexpected error: {}", msg);
}

#[test]
fn resolve_import_alias_namespace() {
    let dir = unique_temp_dir();

    let dep = dir.join("types.verun");
    let root = dir.join("root.verun");

    fs::write(
        &dep,
        r#"
enum Mode { A, B }
const LIMIT: int = 10
"#,
    )
    .expect("failed to write dep");

    fs::write(
        &root,
        r#"
import "./types.verun" as common

state S {
    mode: common::Mode
    x: int

    init {
        mode = common::Mode::A
        x = 0
    }

    transition set_a() {
        where { common::LIMIT >= 0 }
        mode = common::Mode::A
        ensure { mode == common::Mode::A }
    }
}
"#,
    )
    .expect("failed to write root");

    let loaded = parse_file_with_imports(root.to_str().expect("invalid root path"))
        .expect("failed to resolve imports with alias");

    let mut checker = TypeChecker::new();
    let diags = checker.check(&loaded.program);
    let errors: Vec<_> = diags
        .iter()
        .filter(|d| d.severity() == Severity::Error)
        .collect();

    assert!(errors.is_empty(), "unexpected type errors: {:?}", errors);
}

#[test]
fn reject_unknown_symbol_in_alias_namespace() {
    let dir = unique_temp_dir();

    let dep = dir.join("types.verun");
    let root = dir.join("root.verun");

    fs::write(&dep, "enum Mode { A, B }\n").expect("failed to write dep");
    fs::write(
        &root,
        r#"
import "./types.verun" as common

state S {
    mode: common::Missing

    init { mode = common::Mode::A }

    transition t() {
        mode = common::Mode::A
        ensure { mode == common::Mode::A }
    }
}
"#,
    )
    .expect("failed to write root");

    let err = parse_file_with_imports(root.to_str().expect("invalid root path"))
        .expect_err("expected unknown symbol in alias namespace");
    let msg = format!("{}", err);

    assert!(
        msg.contains("is not exported by alias"),
        "unexpected error: {}",
        msg
    );
}

#[test]
fn resolve_import_alias_function_call_and_match() {
    let dir = unique_temp_dir();

    let dep = dir.join("common.verun");
    let root = dir.join("root.verun");

    fs::write(
        &dep,
        r#"
enum Mode { A, B }
fn normalize(x: int) -> int
"#,
    )
    .expect("failed to write dep");

    fs::write(
        &root,
        r#"
import "./common.verun" as common

state S {
    mode: common::Mode
    x: int

    init {
        mode = common::Mode::A
        x = 0
    }

    transition step(delta: int) {
        let normalized: int = common::normalize(delta)
        x = normalized

        let next: common::Mode = common::Mode::B
        match next {
            common::Mode::A => {
                mode = common::Mode::A
            }
            common::Mode::B => {
                mode = common::Mode::B
            }
        }

        ensure {
            mode == common::Mode::B
        }
    }
}
"#,
    )
    .expect("failed to write root");

    let loaded = parse_file_with_imports(root.to_str().expect("invalid root path"))
        .expect("failed to resolve imports with alias");

    let mut checker = TypeChecker::new();
    let diags = checker.check(&loaded.program);
    let errors: Vec<_> = diags
        .iter()
        .filter(|d| d.severity() == Severity::Error)
        .collect();

    assert!(errors.is_empty(), "unexpected type errors: {:?}", errors);
}
