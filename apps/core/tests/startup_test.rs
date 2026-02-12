#[cfg(not(target_os = "windows"))]
#[test]
fn startup_service_reports_unsupported_platform_off_windows() {
    let check = swiftfind_core::startup::is_enabled();
    assert!(matches!(
        check,
        Err(swiftfind_core::startup::StartupError::UnsupportedPlatform)
    ));

    let set = swiftfind_core::startup::set_enabled(false, std::path::Path::new("/tmp/test"));
    assert!(matches!(
        set,
        Err(swiftfind_core::startup::StartupError::UnsupportedPlatform)
    ));
}
