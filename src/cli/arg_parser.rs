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
        let mut args = std::env::args().skip(1);
        let subcommand = args.next().unwrap_or_else(|| usage_error("missing command"));

        let mut path = None;
        let mut emit = Vec::new();
        for arg in args {
            let Some((name, value)) = arg.split_once('=') else {
                usage_error(&format!("expected --flag=value, found `{arg}`"));
            };
            match name {
                "--path" => path = Some(parse_crw_path(value)),
                "--emit" => emit.extend(value.split(',').map(parse_emit_kind)),
                other => usage_error(&format!("unknown flag `{other}`")),
            }
        }
        let path = path.unwrap_or_else(|| usage_error("missing required flag --path"));
        let flags = Flags { path, emit };

        match subcommand.as_str() {
            "build" => Command::Build(flags),
            "run" => Command::Run(flags),
            "check" => Command::Check(flags),
            other => usage_error(&format!("unknown command `{other}`")),
        }
    }
}

fn parse_crw_path(s: &str) -> PathBuf {
    let path = PathBuf::from(s);
    if !path.is_file() {
        usage_error(&format!("invalid file path: {s}"));
    }
    if path.extension().and_then(|e| e.to_str()) != Some("crw") {
        usage_error("invalid file extension (expected .crw)");
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
        "dot" => EmitKind::Dot,
        other => usage_error(&format!("unknown --emit value `{other}`")),
    }
}

fn usage_error(message: &str) -> ! {
    eprintln!("error: {message}");
    eprintln!(
        "usage: crawfish <build|run|check> --path=<path.crw> [--emit=tokens,cst,hir,mir,llvm-ir,dot]"
    );
    std::process::exit(2);
}
