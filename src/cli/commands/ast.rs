use std::process;

use anyhow::Result;

use crate::errors::diagnostic::VerunError;
use crate::errors::report::render_error;
use crate::parser::parse_file_with_imports;

pub fn execute(file: &str, format: &str) -> Result<()> {
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

    match format {
        "json" => {
            let json = serde_json::to_string_pretty(&program)?;
            println!("{}", json);
        }
        _ => {
            println!("{:#?}", program);
        }
    }

    Ok(())
}
