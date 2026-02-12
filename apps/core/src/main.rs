fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let options = match swiftfind_core::runtime::parse_cli_args(&args) {
        Ok(options) => options,
        Err(error) => {
            eprintln!("[swiftfind-core] {error}");
            std::process::exit(2);
        }
    };

    if let Err(error) = swiftfind_core::runtime::run_with_options(options) {
        eprintln!("[swiftfind-core] runtime failed: {error}");
        std::process::exit(1);
    }
}
