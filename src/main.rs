fn main() {
    match std::env::args().nth(1).as_deref() {
        Some("lsp") => blueberry::language_server::main(),
        _ => blueberry::batch::main(),
    }
}
