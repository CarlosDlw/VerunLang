use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "verun")]
#[command(about = "Verun — Programming by Executable Invariants")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Verify a .verun spec using SMT solver
    Check {
        /// Path to the .verun file
        file: String,

        /// Show verbose output
        #[arg(short, long)]
        verbose: bool,

        /// Output format (text, json)
        #[arg(short, long, default_value = "text")]
        format: String,
    },

    /// Execute a .verun spec with the runtime engine
    Run {
        /// Path to the .verun file
        file: String,

        /// Transition to execute (format: transition_name(arg1,arg2))
        #[arg(short, long)]
        transition: Option<String>,

        /// Show state after execution
        #[arg(short, long)]
        show_state: bool,
    },

    /// Generate code from a verified .verun spec
    Gen {
        /// Path to the .verun file
        file: String,

        /// Target language (rust, typescript, solidity, java, go, c, move, cairo, vyper)
        #[arg(short, long)]
        target: String,

        /// Output file path
        #[arg(short, long)]
        output: Option<String>,
    },

    /// Parse and dump AST (debug)
    Ast {
        /// Path to the .verun file
        file: String,

        /// Output format (json, pretty)
        #[arg(short, long, default_value = "pretty")]
        format: String,
    },

    /// Format a .verun file
    Fmt {
        /// Path to the .verun file
        file: String,

        /// Check formatting without writing
        #[arg(short, long)]
        check: bool,
    },

    /// Scaffold a new .verun spec
    Init {
        /// Name of the state machine
        name: String,

        /// Output file path
        #[arg(short, long)]
        output: Option<String>,
    },
}
