fn main() {
    match std::env::args().nth(1).as_deref() {
        Some("lsp") => blueberry::lsp::main(),
        _ => blueberry::batch::main(),
    }
}
