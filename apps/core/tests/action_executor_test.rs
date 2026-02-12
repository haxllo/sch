use std::fs;
use std::path::PathBuf;

use swiftfind_core::action_executor::{launch_path, LaunchError};

#[test]
fn rejects_empty_launch_path() {
    let result = launch_path("");
    assert_eq!(result, Err(LaunchError::EmptyPath));
}

#[test]
fn rejects_missing_launch_path() {
    let missing = format!(
        "/tmp/swiftfind-missing-path-{}-{}",
        std::process::id(),
        "action-executor"
    );

    let result = launch_path(&missing);

    assert_eq!(result, Err(LaunchError::MissingPath(PathBuf::from(&missing))));
}

#[test]
fn accepts_existing_launch_path() {
    let file_path = format!(
        "/tmp/swiftfind-existing-path-{}-{}.tmp",
        std::process::id(),
        "action-executor"
    );

    fs::write(&file_path, b"ok").expect("should create temp file");
    let result = launch_path(&file_path);
    fs::remove_file(&file_path).expect("should clean temp file");

    assert!(result.is_ok());
}
