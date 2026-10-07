#![allow(clippy::unwrap_used)]
use doctor_cargo::{analyze, CargoError};
use std::{path::PathBuf, time::Duration};
fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures")
        .join(name)
        .canonicalize()
        .unwrap()
}
#[test]
fn discovers_virtual_workspace_multiple_contracts_and_exact_paths() {
    let path = fixture("healthy-v28");
    let result = analyze(&path, Duration::from_secs(10)).unwrap();
    assert_eq!(result.workspace_root, path);
    assert_eq!(result.packages.len(), 2);
    assert_eq!(result.contracts.len(), 2);
    for contract in &result.contracts {
        assert_eq!(
            contract.package.soroban_sdk_requirement.as_deref(),
            Some("=28.0.0")
        );
        assert_eq!(
            contract.package.manifest_path,
            path.join(&contract.package.name).join("Cargo.toml")
        );
        assert_eq!(
            contract.evidence[0].path.as_ref(),
            Some(&contract.package.manifest_path)
        );
        assert!(contract.crate_types.contains(&"cdylib".to_string()));
    }
}
#[test]
fn inherited_dependency_and_member_path_find_workspace() {
    let root = fixture("workspace-inherited");
    let result = analyze(&root.join("nested/contract"), Duration::from_secs(10)).unwrap();
    assert_eq!(result.workspace_root, root);
    assert_eq!(result.contracts.len(), 1);
    assert_eq!(
        result.contracts[0]
            .package
            .soroban_sdk_requirement
            .as_deref(),
        Some("=28.0.0")
    );
}
#[test]
fn cdylib_without_sdk_is_not_a_soroban_contract() {
    let result = analyze(&fixture("non-soroban"), Duration::from_secs(10)).unwrap();
    assert_eq!(result.packages.len(), 1);
    assert!(result.contracts.is_empty());
}

fn snapshot(root: &std::path::Path) -> std::collections::BTreeMap<PathBuf, Vec<u8>> {
    fn visit(
        root: &std::path::Path,
        path: &std::path::Path,
        files: &mut std::collections::BTreeMap<PathBuf, Vec<u8>>,
    ) {
        for entry in std::fs::read_dir(path).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                visit(root, &path, files);
            } else {
                files.insert(
                    path.strip_prefix(root).unwrap().to_path_buf(),
                    std::fs::read(path).unwrap(),
                );
            }
        }
    }
    let mut files = std::collections::BTreeMap::new();
    visit(root, root, &mut files);
    files
}
#[test]
fn fixtures_are_not_modified_and_no_lockfile_or_build_is_created() {
    for name in ["healthy-v28", "non-soroban", "workspace-inherited"] {
        let path = fixture(name);
        let before = snapshot(&path);
        analyze(&path.join("Cargo.toml"), Duration::from_secs(10)).unwrap();
        assert_eq!(snapshot(&path), before);
        assert!(!path.join("Cargo.lock").exists());
        assert!(!path.join("target").exists());
    }
}
struct TemporaryProject(PathBuf);
impl TemporaryProject {
    fn new() -> Self {
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "sortralis-cargo-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn manifest(&self, dependency: &str) {
        std::fs::create_dir(self.0.join("src")).unwrap();
        std::fs::write(self.0.join("src/lib.rs"), "pub const VALUE: u32 = 1;\n").unwrap();
        std::fs::write(self.0.join("Cargo.toml"), format!("[package]\nname = \"candidate\"\nversion = \"0.1.0\"\nedition = \"2021\"\n[workspace]\n{dependency}\n")).unwrap();
    }
}
impl Drop for TemporaryProject {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
#[test]
fn non_cargo_directory_does_not_fall_back_to_an_ancestor_manifest() {
    let project = TemporaryProject::new();
    project.manifest("");
    let nested = project.0.join("unrelated");
    std::fs::create_dir(&nested).unwrap();
    assert!(matches!(
        analyze(&nested, Duration::from_secs(10)),
        Err(CargoError::NoManifest { .. })
    ));
}
#[test]
fn malformed_manifest_preserves_failure_output() {
    let project = TemporaryProject::new();
    std::fs::write(project.0.join("Cargo.toml"), "[package\n").unwrap();
    let before = snapshot(&project.0);
    match analyze(&project.0, Duration::from_secs(10)).unwrap_err() {
        CargoError::MetadataFailed(command) => {
            assert!(!command.stderr_bytes.is_empty());
            assert!(command.record.stderr.contains("Cargo.toml"));
            assert_ne!(
                command.record.status,
                doctor_core::CommandStatus::Exited { code: 0 }
            );
        }
        other => panic!("unexpected error: {other}"),
    }
    assert_eq!(snapshot(&project.0), before);
}
#[test]
fn dev_and_build_dependencies_alone_do_not_indicate_a_contract() {
    for kind in ["dev-dependencies", "build-dependencies"] {
        let project = TemporaryProject::new();
        project.manifest(&format!("[{kind}]\nsoroban-sdk = \"=28.0.0\""));
        assert!(analyze(&project.0, Duration::from_secs(10))
            .unwrap()
            .contracts
            .is_empty());
    }
}
#[test]
fn normal_optional_targeted_sdk_is_evidence_even_without_cdylib() {
    let project = TemporaryProject::new();
    project.manifest("[target.'cfg(target_arch = \"wasm32\")'.dependencies]\nsdk = { package = \"soroban-sdk\", version = \"=28.0.0\", optional = true }");
    std::fs::write(
        project.0.join("build.rs"),
        "compile_error!(\"Discovery must not execute build scripts\");\n",
    )
    .unwrap();
    let before = snapshot(&project.0);
    let analysis = analyze(&project.0, Duration::from_secs(10)).unwrap();
    assert_eq!(analysis.contracts.len(), 1);
    let sdk = &analysis.contracts[0].sdk_dependencies[0];
    assert_eq!(sdk.alias.as_deref(), Some("sdk"));
    assert!(sdk.optional);
    assert_eq!(sdk.target.as_deref(), Some("cfg(target_arch = \"wasm32\")"));
    assert_eq!(snapshot(&project.0), before);
}
#[test]
fn missing_path_returns_typed_error() {
    let project = TemporaryProject::new();
    assert!(matches!(
        analyze(&project.0.join("absent"), Duration::from_secs(10)),
        Err(CargoError::Path { .. })
    ));
}
