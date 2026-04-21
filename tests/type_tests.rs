use verun::parser::parse_source;
use verun::types::checker::TypeChecker;

fn check_source(source: &str) -> Vec<verun::errors::diagnostic::VerunError> {
    let program = parse_source(source).unwrap();
    let mut checker = TypeChecker::new();
    checker.check(&program)
}

#[test]
fn type_check_counter() {
    let source = include_str!("../examples/counter.verun");
    let program = parse_source(source).unwrap();
    let mut checker = TypeChecker::new();
    let errors = checker.check(&program);
    assert!(errors.is_empty(), "Type errors: {:?}", errors);
}

#[test]
fn type_check_token() {
    let source = include_str!("../examples/token.verun");
    let program = parse_source(source).unwrap();
    let mut checker = TypeChecker::new();
    let errors = checker.check(&program);
    assert!(errors.is_empty(), "Type errors: {:?}", errors);
}

#[test]
fn type_check_voting() {
    let source = include_str!("../examples/voting.verun");
    let program = parse_source(source).unwrap();
    let mut checker = TypeChecker::new();
    let errors = checker.check(&program);
    assert!(errors.is_empty(), "Type errors: {:?}", errors);
}

#[test]
fn type_check_undefined_variable() {
    let source = r#"
        state Bad {
            x: int

            init {
                x = 0
            }

            transition broken() {
                y = 10
            }
        }
    "#;
    let program = parse_source(source).unwrap();
    let mut checker = TypeChecker::new();
    let errors = checker.check(&program);
    assert!(!errors.is_empty(), "Should detect undefined variable 'y'");
}

#[test]
fn type_check_invariant_must_be_bool() {
    let source = r#"
        state Bad {
            x: int

            invariant not_bool {
                x + 1
            }

            init {
                x = 0
            }
        }
    "#;
    let program = parse_source(source).unwrap();
    let mut checker = TypeChecker::new();
    let errors = checker.check(&program);
    assert!(!errors.is_empty(), "Should detect non-boolean invariant");
}

#[test]
fn type_check_auction() {
    let source = include_str!("../examples/auction.verun");
    let errors = check_source(source);
    assert!(errors.is_empty(), "Type errors: {:?}", errors);
}

#[test]
fn type_check_type_mismatch_in_assign() {
    let source = r#"
        state Bad {
            x: bool
            init { x = 42 }
        }
    "#;
    let errors = check_source(source);
    assert!(
        !errors.is_empty(),
        "Should detect type mismatch: bool vs int"
    );
}

#[test]
fn type_check_invalid_enum_variant() {
    let source = r#"
        enum Color { Red, Green, Blue }
        state Bad {
            c: Color
            init { c = Color::Yellow }
        }
    "#;
    let errors = check_source(source);
    assert!(
        errors.iter().any(|e| matches!(
            e,
            verun::errors::diagnostic::VerunError::InvalidEnumVariant { .. }
        )),
        "Should detect invalid enum variant 'Yellow': {:?}",
        errors
    );
}

#[test]
fn type_check_duplicate_field() {
    let source = r#"
        state Bad {
            x: int
            x: bool
            init { x = 0 }
        }
    "#;
    let errors = check_source(source);
    assert!(
        errors.iter().any(|e| matches!(
            e,
            verun::errors::diagnostic::VerunError::DuplicateDefinition { .. }
        )),
        "Should detect duplicate field: {:?}",
        errors
    );
}

#[test]
fn type_check_uninitialized_field() {
    let source = r#"
        state Bad {
            x: int
            y: int
            init { x = 0 }
        }
    "#;
    let errors = check_source(source);
    assert!(
        errors.iter().any(|e| matches!(
            e,
            verun::errors::diagnostic::VerunError::UninitializedField { .. }
        )),
        "Should detect uninitialized field 'y': {:?}",
        errors
    );
}

#[test]
fn type_check_old_outside_ensure() {
    let source = r#"
        state Bad {
            x: int
            invariant bad_inv { old(x) > 0 }
            init { x = 1 }
        }
    "#;
    let errors = check_source(source);
    assert!(
        errors.iter().any(|e| matches!(
            e,
            verun::errors::diagnostic::VerunError::OldOutsideEnsure { .. }
        )),
        "Should detect old() outside ensure: {:?}",
        errors
    );
}

#[test]
fn type_check_old_in_ensure_is_ok() {
    let source = r#"
        state Good {
            x: int
            init { x = 0 }
            transition inc() {
                x = x + 1
                ensure { x == old(x) + 1 }
            }
        }
    "#;
    let errors = check_source(source);
    assert!(
        errors.is_empty(),
        "old() in ensure should be valid: {:?}",
        errors
    );
}

#[test]
fn type_check_compound_assign_type_mismatch() {
    let source = r#"
        state Bad {
            x: bool
            init { x = true }
            transition go() { x += 1 }
        }
    "#;
    let errors = check_source(source);
    assert!(
        !errors.is_empty(),
        "Should detect type mismatch in compound assign: {:?}",
        errors
    );
}

#[test]
fn type_check_missing_init_warning() {
    let source = r#"
        state NoInit {
            x: int
            transition go() { x = 1 }
        }
    "#;
    let errors = check_source(source);
    assert!(
        errors
            .iter()
            .any(|e| matches!(e, verun::errors::diagnostic::VerunError::MissingInit { .. })),
        "Should warn about missing init: {:?}",
        errors
    );
    assert!(
        errors.iter().all(
            |e| e.severity() != verun::errors::diagnostic::Severity::Error
                || matches!(e, verun::errors::diagnostic::VerunError::MissingInit { .. })
        ),
        "MissingInit should be a warning, not an error"
    );
}

#[test]
fn type_check_empty_transition_body_warning() {
    let source = r#"
        state Empty {
            x: int
            init { x = 0 }
            transition noop() {}
        }
    "#;
    let errors = check_source(source);
    assert!(
        errors.iter().any(|e| matches!(
            e,
            verun::errors::diagnostic::VerunError::EmptyTransitionBody { .. }
        )),
        "Should warn about empty transition body: {:?}",
        errors
    );
}

#[test]
fn type_check_parse_error_has_span() {
    let source = "this is not valid verun code !!!";
    let result = verun::parser::parse_source(source);
    assert!(result.is_err());
    let err = result.unwrap_err();
    let parse_err = err.downcast_ref::<verun::errors::diagnostic::VerunError>();
    assert!(parse_err.is_some(), "Should be a VerunError::ParseError");
    if let Some(verun::errors::diagnostic::VerunError::ParseError { span, .. }) = parse_err {
        assert!(span.is_some(), "Parse error should have a span");
    }
}

#[test]
fn type_check_let_and_match_ok() {
    let source = r#"
        enum Status { Open, Closed }
        const LIMIT: int = 7

        state Ticket {
            s: Status
            n: int

            init {
                s = Status::Open
                n = 0
            }

            transition set_next(next: Status) {
                let cap: int = LIMIT
                match next {
                    Status::Open => { n = cap },
                    Status::Closed => { n = 0 }
                }
                s = next
            }
        }
    "#;
    let errors = check_source(source);
    assert!(
        errors.is_empty(),
        "let/match should type-check: {:?}",
        errors
    );
}

#[test]
fn type_check_non_exhaustive_match_on_enum() {
    let source = r#"
        enum Status { Open, Closed }

        state Ticket {
            s: Status
            n: int

            init {
                s = Status::Open
                n = 0
            }

            transition only_open(next: Status) {
                match next {
                    Status::Open => { n = 1 }
                }
            }
        }
    "#;
    let errors = check_source(source);
    assert!(
        errors.iter().any(|e| matches!(
            e,
            verun::errors::diagnostic::VerunError::NonExhaustiveMatch { .. }
        )),
        "Should detect non-exhaustive enum match: {:?}",
        errors
    );
}
