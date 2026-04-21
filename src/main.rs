use clap::Parser;

use verun::cli::app::{Cli, Commands};
use verun::cli::commands;

fn main() {
    let cli = Cli::parse();

    let result = match &cli.command {
        Commands::Check {
            file,
            verbose,
            format,
        } => commands::check::execute(file, *verbose, format),

        Commands::Run {
            file,
            transition,
            show_state,
        } => commands::run::execute(file, transition.as_deref(), *show_state),

        Commands::Gen {
            file,
            target,
            output,
        } => commands::generate::execute(file, target, output.as_deref()),

        Commands::Ast { file, format } => commands::ast::execute(file, format),

        Commands::Fmt { file, check } => commands::fmt::execute(file, *check),

        Commands::Init { name, output } => commands::init::execute(name, output.as_deref()),
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
