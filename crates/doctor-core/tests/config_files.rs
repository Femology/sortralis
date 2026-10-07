#![allow(clippy::unwrap_used)]

use doctor_core::{Config, ConfigError, CONFIG_FILE_NAME};
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "sortralis-config-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn optional_missing_file_uses_defaults_without_creating_a_file() {
    let directory = TestDirectory::new();
    let path = directory.0.join(CONFIG_FILE_NAME);
    assert_eq!(Config::load_optional(&path).unwrap(), Config::default());
    assert!(!path.exists());
}

#[test]
fn existing_file_is_read_and_parse_errors_include_its_path() {
    let directory = TestDirectory::new();
    let path = directory.0.join(CONFIG_FILE_NAME);
    fs::write(&path, "run_tests = false").unwrap();
    assert!(!Config::load_optional(&path).unwrap().run_tests);
    fs::write(&path, "fail_on = [\"NOT_A_SEVERITY\"]").unwrap();
    let error = Config::load_optional(&path).unwrap_err();
    assert!(matches!(error, ConfigError::Parse { .. }));
    assert!(error.to_string().contains("NOT_A_SEVERITY"));
    assert!(error.to_string().contains(CONFIG_FILE_NAME));
}

#[test]
fn a_directory_is_a_read_error_instead_of_silently_using_defaults() {
    let directory = TestDirectory::new();
    let error = Config::load_optional(&directory.0).unwrap_err();
    assert!(matches!(error, ConfigError::Read { .. }));
    assert!(std::error::Error::source(&error).is_some());
}
