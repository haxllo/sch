fn main() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os != "windows" {
        return;
    }

    let icon_path = "../assets/swiftfinder.ico";
    if !std::path::Path::new(icon_path).exists() {
        panic!("missing Windows icon file: {icon_path}");
    }

    let mut res = winres::WindowsResource::new();
    res.set_icon(icon_path);
    res.compile()
        .expect("failed to compile Windows resources");
}
