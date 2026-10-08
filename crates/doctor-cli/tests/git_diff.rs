#![allow(clippy::unwrap_used)]
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};
fn git(root: &Path, args: &[&str]) -> Vec<u8> {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    output.stdout
}
fn stage_exact_file(root: &Path, path: &str) {
    let hash = String::from_utf8(git(root, &["hash-object", "-w", "--", path]))
        .unwrap()
        .trim()
        .to_owned();
    git(
        root,
        &[
            "update-index",
            "--add",
            "--cacheinfo",
            "100644",
            &hash,
            path,
        ],
    );
}

fn commit(root: &Path, message: &str) {
    // Stage exact blob contents rather than relying on Git's stat-cache shortcut.
    // The before/after Cargo.toml fixtures have the same size and can otherwise
    // appear unchanged on filesystems with coarse/racy timestamp behavior.
    stage_exact_file(root, "Cargo.toml");
    stage_exact_file(root, "src/lib.rs");
    stage_exact_file(root, "build.rs");
    git(
        root,
        &[
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.invalid",
            "commit",
            "-m",
            message,
        ],
    );
}
fn repository() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    git(root, &["init", "-b", "main"]);
    fs::create_dir(root.join("src")).unwrap();
    let fixtures = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/git-diff");
    for (fixture, reference) in [("before", "before"), ("after", "after")] {
        for file in ["Cargo.toml", "src/lib.rs", "build.rs"] {
            fs::copy(fixtures.join(fixture).join(file), root.join(file)).unwrap();
        }
        commit(root, fixture);
        git(root, &["tag", reference]);
    }
    dir
}
fn diff(root: &Path, from: &str, to: &str, json: bool) -> std::process::Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_sud"));
    command
        .arg("diff")
        .arg("--repo")
        .arg(root)
        .arg("--from")
        .arg(from)
        .arg("--to")
        .arg(to);
    if json {
        command.arg("--json");
    }
    command.output().unwrap()
}

#[test]
fn real_cli_dirty_sentinel_git_state_and_deterministic_json_gate() {
    let dir = repository();
    let root = dir.path();
    // Explicit uncommitted sentinel: diff must preserve every byte.
    let sentinel = root.join("uncommitted-sentinel.bin");
    let bytes = b"DO NOT CHANGE\0\xff\n";
    fs::write(&sentinel, bytes).unwrap();
    let tracked = root.join("src/lib.rs");
    let dirty = b"this deliberately invalid Rust is an uncommitted edit";
    fs::write(&tracked, dirty).unwrap();
    let head = git(root, &["rev-parse", "HEAD"]);
    let branch = git(root, &["symbolic-ref", "HEAD"]);
    let worktrees = git(root, &["worktree", "list", "--porcelain"]);
    let index = fs::read(root.join(".git/index")).unwrap();
    let first = diff(root, "before", "after", true);
    assert!(first.status.success(), "{first:?}");
    let second = diff(root, "before", "after", true);
    assert!(second.status.success(), "{second:?}");
    assert_eq!(
        first.stdout, second.stdout,
        "temporary names/timing must not affect diff output"
    );
    let result: doctor_core::git_diff::GitDiff = serde_json::from_slice(&first.stdout).unwrap();
    assert!(result.dirty_active_worktree);
    assert_eq!(result.sdk_changes.len(), 1, "{result:#?}");
    assert_eq!(result.functions.len(), 3);
    assert_eq!(result.events.len(), 1);
    assert!(!result.types.is_empty());
    assert!(result.storage.iter().any(|s| {
        s.diff.records.iter().any(|r| {
            r.classification == doctor_core::storage::StorageClassification::MigrationLikely
        })
    }));
    assert!(result
        .findings
        .iter()
        .any(|f| f.before.is_empty() && f.after[0].id.as_str() == "SDK28_REMOVED_EXPORT_ARGUMENT"));
    let text = diff(root, "before", "after", false);
    assert!(text.status.success(), "{text:?}");
    let text = String::from_utf8(text.stdout).unwrap();
    for label in [
        "SDK",
        "Functions added",
        "Functions removed",
        "Functions changed",
        "Types",
        "Events",
        "Storage",
        "Findings introduced",
        "Findings resolved",
    ] {
        assert!(text.contains(label), "{text}");
    }
    assert_eq!(git(root, &["rev-parse", "HEAD"]), head);
    assert_eq!(git(root, &["symbolic-ref", "HEAD"]), branch);
    assert_eq!(git(root, &["worktree", "list", "--porcelain"]), worktrees);
    assert_eq!(fs::read(root.join(".git/index")).unwrap(), index);
    assert_eq!(fs::read(sentinel).unwrap(), bytes);
    assert_eq!(fs::read(tracked).unwrap(), dirty);
}

#[test]
fn invalid_ref_and_source_errors_preserve_active_files() {
    let dir = repository();
    let root = dir.path();
    fs::write(root.join("sentinel"), b"user data").unwrap();
    let bad = diff(root, "before", "--all", false);
    assert_eq!(bad.status.code(), Some(2));
    assert!(bad.stdout.is_empty());
    fs::write(root.join("src/lib.rs"), "fn {").unwrap();
    commit(root, "invalid-source");
    git(root, &["tag", "invalid"]);
    let error = diff(root, "before", "invalid", false);
    assert_eq!(error.status.code(), Some(2), "{error:?}");
    assert!(String::from_utf8(error.stderr).unwrap().contains("parse"));
    assert_eq!(fs::read(root.join("sentinel")).unwrap(), b"user data");
}

#[test]
fn identical_refs_and_reverse_findings_are_conservative() {
    let dir = repository();
    let same = diff(dir.path(), "after", "after", true);
    assert!(same.status.success(), "{same:?}");
    let same: doctor_core::git_diff::GitDiff = serde_json::from_slice(&same.stdout).unwrap();
    assert!(same.sdk_changes.is_empty());
    assert!(same.functions.is_empty());
    assert!(same.types.is_empty());
    assert!(same.events.is_empty());
    assert!(same.findings.is_empty());
    assert!(same.storage.iter().all(|s| s.diff.records.is_empty()));
    let reverse = diff(dir.path(), "after", "before", true);
    assert!(reverse.status.success(), "{reverse:?}");
    let reverse: doctor_core::git_diff::GitDiff = serde_json::from_slice(&reverse.stdout).unwrap();
    assert!(
        reverse
            .findings
            .iter()
            .any(|f| f.after.is_empty()
                && f.before[0].id.as_str() == "SDK28_REMOVED_EXPORT_ARGUMENT"),
        "{reverse:#?}"
    );
}

#[test]
fn external_manifest_paths_are_rejected_and_analysis_errors_clean_snapshots() {
    let dir = repository();
    let root = dir.path();
    fs::write(root.join("sentinel"), b"preserve bytes").unwrap();
    fs::write(root.join("src/lib.rs"), "fn {").unwrap();
    commit(root, "parse-error");
    let error = doctor_cli::diff::git_diff(root, "before", "HEAD").unwrap_err();
    match error {
        doctor_cli::diff::DiffError::AnalysisSnapshot { path, source } => {
            assert!(!path.exists());
            assert!(matches!(
                *source,
                doctor_cli::diff::DiffError::Source(doctor_source::SourceError::Parse { .. })
            ));
        }
        e => panic!("unexpected error: {e}"),
    }
    let manifest = fs::read_to_string(root.join("Cargo.toml"))
        .unwrap()
        .replace(
            "[dependencies]",
            "[dependencies]\nexternal = { path = \"../outside\" }",
        );
    fs::write(root.join("Cargo.toml"), manifest).unwrap();
    commit(root, "external-path");
    let head = git(root, &["rev-parse", "HEAD"]);
    let error = doctor_cli::diff::git_diff(root, "before", "HEAD").unwrap_err();
    assert!(
        matches!(error, doctor_cli::diff::DiffError::AnalysisSnapshot { source, .. } if matches!(*source, doctor_cli::diff::DiffError::OutsideSnapshot(_)))
    );
    assert_eq!(git(root, &["rev-parse", "HEAD"]), head);
    assert_eq!(fs::read(root.join("sentinel")).unwrap(), b"preserve bytes");
}
