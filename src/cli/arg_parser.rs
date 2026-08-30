use std::path::PathBuf;

pub enum Command {
    Build(Flags),
    Run(Flags),
    Check(Flags),
}

pub struct Flags {
    pub path: PathBuf,
    pub emit: Vec<EmitKind>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EmitKind {
    Tokens,
    Cst,
    Hir,
    Mir,
    LlvmIr,
    Dot,
}

impl Command {
    pub fn parse() -> Self {
        let mut arguments = std::env::args().skip(1);
        let subcommand = arguments
            .next()
            .unwrap_or_else(|| print_usage_error("missing command"));

        let mut path = None;
        let mut emit = Vec::new();
        for argument in arguments {
            let Some((name, value)) = argument.split_once('=') else {
                print_usage_error(&format!("expected `--flag=value`, found `{argument}`"));
            };
            match name {
                "--path" => path = Some(parse_crw_path(value)),
                "--emit" => emit.extend(value.split(',').map(parse_emit_kind)),
                other => print_usage_error(&format!("unknown flag `{other}`")),
            }
        }
        let path = path.unwrap_or_else(|| print_usage_error("missing required flag --path"));
        let flags = Flags { path, emit };

        match subcommand.as_str() {
            "build" => Command::Build(flags),
            "run" => Command::Run(flags),
            "check" => Command::Check(flags),
            other => print_usage_error(&format!("unknown command `{other}`")),
        }
    }
}

fn parse_crw_path(s: &str) -> PathBuf {
    let path = PathBuf::from(s);
    if !path.is_file() {
        print_usage_error(&format!("invalid file path: {s}"));
    }
    if path.extension().and_then(|e| e.to_str()) != Some("crw") {
        print_usage_error("invalid file extension (only `.crw` files are accepted)");
    }
    path
}

fn parse_emit_kind(s: &str) -> EmitKind {
    match s {
        "tokens" => EmitKind::Tokens,
        "cst" => EmitKind::Cst,
        "hir" => EmitKind::Hir,
        "mir" => EmitKind::Mir,
        "llvm-ir" => EmitKind::LlvmIr,
        other => print_usage_error(&format!("unknown --emit value `{other}`")),
    }
}

fn print_usage_error(message: &str) -> ! {
    eprintln!("error: {message}");
    std::process::exit(2);
}
