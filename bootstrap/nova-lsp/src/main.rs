fn main() {
    if let Err(error) = nova_lsp::run_stdio() {
        eprintln!("nova-lsp: {error}");
        std::process::exit(1);
    }
}
