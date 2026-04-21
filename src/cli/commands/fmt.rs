use std::fs;
use std::process;

use anyhow::Result;

use crate::codegen::formatter::format_program;
use crate::errors::diagnostic::VerunError;
use crate::errors::report::render_error;
use crate::parser::parse_source;

pub fn execute(file: &str, check_only: bool) -> Result<()> {
    let source = fs::read_to_string(file)?;
    let program = match parse_source(&source) {
        Ok(p) => p,
        Err(e) => {
            if let Some(parse_err) = e.downcast_ref::<VerunError>() {
                eprint!("{}", render_error(parse_err, &source, file));
            } else {
                eprintln!("Error: {}", e);
            }
            process::exit(1);
        }
    };

    let formatted = format_program(&program);

    if check_only {
        if source.trim() == formatted.trim() {
            println!("{} is already formatted", file);
        } else {
            eprintln!("{} needs formatting", file);
            process::exit(1);
        }
    } else {
        fs::write(file, &formatted)?;
        eprintln!("Formatted {}", file);
    }

    Ok(())
}
