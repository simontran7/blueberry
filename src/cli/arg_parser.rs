use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};

/// blueberry: the compiler for the blueberry programming language
#[derive(Parser)]
#[command(name = "blueberry", arg_required_else_help = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Check a program for errors
    Check(Flags),
    /// Compile a program into an executable (not implemented yet)
    Build(Flags),
    /// Build a program, then run it (not implemented yet)
    Run(Flags),
}

#[derive(Args)]
pub struct Flags {
    /// The `.bb` file to compile, whose directory is the project's root
    #[arg(long, value_parser = parse_bb_path)]
    pub path: PathBuf,
    /// Also print what the compiler produced at some of its stages, for the compiled file
    #[arg(long, value_delimiter = ',')]
    pub emit: Vec<EmitKind>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, ValueEnum)]
pub enum EmitKind {
    Tokens,
    Cst,
    Hir,
    Mir,
    #[value(hide = true)]
    LlvmIr,
}

fn parse_bb_path(s: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(s);
    if !path.is_file() {
        return Err(format!("invalid file path: {s}"));
    }
    if path.extension().and_then(|e| e.to_str()) != Some("bb") {
        return Err("invalid file extension (only `.bb` files are accepted)".to_string());
    }
    Ok(path)
}
