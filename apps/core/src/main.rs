fn main() {
    if let Err(error) = swiftfind_core::runtime::run() {
        eprintln!("[swiftfind-core] runtime failed: {error}");
        std::process::exit(1);
    }
}
