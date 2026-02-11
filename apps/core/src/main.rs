fn main() {
    match swiftfind_core::hotkey::parse_hotkey("Alt+Space") {
        Ok(h) => println!("swiftfind-core hotkey={}+{}", h.modifiers.join("+"), h.key),
        Err(e) => eprintln!("hotkey parse error: {e}"),
    }
}
