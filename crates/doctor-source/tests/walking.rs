#![allow(clippy::unwrap_used)]
use doctor_source::{analyze_directory, SourceError, SourceOptions, TargetContext};
use std::{fs, path::PathBuf};
struct Project(PathBuf);
impl Project {
    fn new() -> Self {
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "sortralis-source-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn write(&self, path: &str, source: &str) {
        let path = self.0.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, source).unwrap();
    }
}
impl Drop for Project {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
const EXPORT: &str = "#[soroban_sdk::contracttype(export = false)] struct S;";
#[test]
fn exclusions_defaults_grouping_and_exact_paths_are_respected() {
    let project = Project::new();
    for path in ["src/first.rs", "src/nested/second.rs"] {
        project.write(path, EXPORT);
    }
    for path in [
        "target/broken.rs",
        "vendor/broken.rs",
        "generated/broken.rs",
        "src/generated/broken.rs",
        ".git/broken.rs",
        "node_modules/broken.rs",
        "src/excluded/broken.rs",
        "ignored.rs",
    ] {
        project.write(path, "NOT VALID RUST {");
    }
    let options = SourceOptions {
        exclude: vec!["src/excluded".into(), "ignored.rs".into()],
        ..SourceOptions::default()
    };
    let findings = analyze_directory(&project.0, TargetContext::Sdk28, &options).unwrap();
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].evidence.len(), 2);
    for (evidence, path) in findings[0]
        .evidence
        .iter()
        .zip(["src/first.rs", "src/nested/second.rs"])
    {
        assert_eq!(evidence.path.as_ref().unwrap(), &project.0.join(path));
        assert_eq!(evidence.line.unwrap().get(), 1);
        assert_eq!(fs::read_to_string(project.0.join(path)).unwrap(), EXPORT);
    }
    assert!(!project.0.join("target/report.json").exists());
}
#[test]
fn excluded_file_can_be_included_when_custom_exclusion_is_removed() {
    let project = Project::new();
    project.write("skip/contract.rs", EXPORT);
    let options = SourceOptions {
        exclude: vec!["skip".into()],
        ..SourceOptions::default()
    };
    assert!(
        analyze_directory(&project.0, TargetContext::Sdk28, &options)
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        analyze_directory(&project.0, TargetContext::Sdk28, &SourceOptions::default()).unwrap()[0]
            .evidence
            .len(),
        1
    );
}
#[test]
fn malformed_files_and_bad_exclusions_are_not_silent_success() {
    let project = Project::new();
    project.write("broken.rs", "fn broken( {");
    assert!(matches!(
        analyze_directory(&project.0, TargetContext::Sdk28, &SourceOptions::default()),
        Err(SourceError::Parse { .. })
    ));
    for exclude in ["", "../outside", "/absolute", "."] {
        let options = SourceOptions {
            exclude: vec![exclude.into()],
            ..SourceOptions::default()
        };
        assert!(matches!(
            analyze_directory(&project.0, TargetContext::Sdk28, &options),
            Err(SourceError::InvalidExclusion(_))
        ));
    }
}
#[test]
fn cargo_dependency_aliases_can_be_supplied_without_guessing() {
    let project = Project::new();
    project.write(
        "lib.rs",
        "use sdk_alias as sdk; #[sdk::contracttype(export = false)] struct S;",
    );
    assert!(
        analyze_directory(&project.0, TargetContext::Sdk28, &SourceOptions::default())
            .unwrap()
            .is_empty()
    );
    let options = SourceOptions {
        sdk_crate_names: vec!["sdk_alias".into()],
        ..SourceOptions::default()
    };
    assert_eq!(
        analyze_directory(&project.0, TargetContext::Sdk28, &options)
            .unwrap()
            .len(),
        1
    );
}
#[cfg(unix)]
#[test]
fn symlinks_do_not_escape_the_source_root_or_duplicate_findings() {
    let project = Project::new();
    let outside = Project::new();
    outside.write("outside.rs", EXPORT);
    project.write("real.rs", EXPORT);
    std::os::unix::fs::symlink(outside.0.join("outside.rs"), project.0.join("linked.rs")).unwrap();
    std::os::unix::fs::symlink(&project.0, project.0.join("cycle")).unwrap();
    assert_eq!(
        analyze_directory(&project.0, TargetContext::Sdk28, &SourceOptions::default()).unwrap()[0]
            .evidence
            .len(),
        1
    );
}
