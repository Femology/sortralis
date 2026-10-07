#![allow(clippy::unwrap_used)]
use doctor_git::GitRepository;
use std::{fs, path::Path, process::Command, time::Duration};

fn git(root: &Path, args: &[&str]) -> Vec<u8> {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    output.stdout
}
fn repo() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    git(dir.path(), &["init", "-b", "main"]);
    fs::write(dir.path().join("tracked.txt"), b"committed bytes").unwrap();
    git(dir.path(), &["add", "tracked.txt"]);
    git(
        dir.path(),
        &[
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.invalid",
            "commit",
            "-m",
            "baseline",
        ],
    );
    dir
}

#[test]
fn dirty_active_tree_head_branch_index_and_sentinel_are_preserved() {
    let dir = repo();
    assert!(!GitRepository::open(dir.path(), Duration::from_secs(10))
        .unwrap()
        .is_dirty()
        .unwrap());
    let sentinel = dir.path().join("uncommitted-sentinel.bin");
    let bytes = b"sentinel\0\xffuser bytes\n";
    fs::write(&sentinel, bytes).unwrap();
    fs::write(dir.path().join("tracked.txt"), b"dirty tracked bytes").unwrap();
    let head = git(dir.path(), &["rev-parse", "HEAD"]);
    let branch = git(dir.path(), &["symbolic-ref", "HEAD"]);
    let worktrees = git(dir.path(), &["worktree", "list", "--porcelain"]);
    let index = fs::read(dir.path().join(".git/index")).unwrap();
    let repository = GitRepository::open(dir.path(), Duration::from_secs(10)).unwrap();
    assert!(repository.is_dirty().unwrap());
    let snapshots = repository.snapshots("HEAD", "main").unwrap();
    let temporary = snapshots.root().to_path_buf();
    assert!(temporary.starts_with(std::env::temp_dir()));
    assert_eq!(
        fs::read(snapshots.from_path().join("tracked.txt")).unwrap(),
        b"committed bytes"
    );
    assert!(!snapshots
        .from_path()
        .join("uncommitted-sentinel.bin")
        .exists());
    snapshots.close().unwrap();
    assert!(!temporary.exists());
    assert_eq!(git(dir.path(), &["rev-parse", "HEAD"]), head);
    assert_eq!(git(dir.path(), &["symbolic-ref", "HEAD"]), branch);
    assert_eq!(
        git(dir.path(), &["worktree", "list", "--porcelain"]),
        worktrees
    );
    assert_eq!(fs::read(dir.path().join(".git/index")).unwrap(), index);
    assert_eq!(fs::read(sentinel).unwrap(), bytes);
    assert_eq!(
        fs::read(dir.path().join("tracked.txt")).unwrap(),
        b"dirty tracked bytes"
    );
}

#[test]
fn invalid_refs_and_drop_cleanup_are_safe() {
    let dir = repo();
    let repository = GitRepository::open(dir.path(), Duration::from_secs(10)).unwrap();
    let head = git(dir.path(), &["rev-parse", "HEAD"]);
    for reference in ["", "--all", "missing", "HEAD;touch SENTINEL", "HEAD\n"] {
        assert!(repository.snapshots("HEAD", reference).is_err());
    }
    assert!(!dir.path().join("SENTINEL").exists());
    assert_eq!(git(dir.path(), &["rev-parse", "HEAD"]), head);
    let path = {
        let snapshot = repository.snapshots("HEAD", "HEAD").unwrap();
        snapshot.root().to_path_buf()
    };
    assert!(!path.exists());
}

#[test]
fn dirty_detection_does_not_use_git_clean_filters() {
    let dir = repo();
    fs::write(
        dir.path().join(".gitattributes"),
        "tracked.txt filter=never-run\n",
    )
    .unwrap();
    git(dir.path(), &["add", ".gitattributes"]);
    git(
        dir.path(),
        &[
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.invalid",
            "commit",
            "-m",
            "attributes",
        ],
    );
    git(
        dir.path(),
        &["config", "filter.never-run.clean", "touch FILTER_EXECUTED"],
    );
    git(dir.path(), &["config", "filter.never-run.required", "true"]);
    // Same length as the committed file, forcing content inspection.
    fs::write(dir.path().join("tracked.txt"), b"dirty raw bytes").unwrap();
    let repository = GitRepository::open(dir.path(), Duration::from_secs(10)).unwrap();
    assert!(repository.is_dirty().unwrap());
    assert!(!dir.path().join("FILTER_EXECUTED").exists());
}

#[cfg(unix)]
#[test]
fn partial_snapshot_error_cleans_directory_and_never_follows_symlink() {
    use std::os::unix::fs::symlink;
    let dir = repo();
    let baseline = String::from_utf8(git(dir.path(), &["rev-parse", "HEAD"])).unwrap();
    symlink("/etc/passwd", dir.path().join("unsafe-link")).unwrap();
    git(dir.path(), &["add", "unsafe-link"]);
    git(
        dir.path(),
        &[
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.invalid",
            "commit",
            "-m",
            "link",
        ],
    );
    let repository = GitRepository::open(dir.path(), Duration::from_secs(10)).unwrap();
    let head = git(dir.path(), &["rev-parse", "HEAD"]);
    match repository.snapshots(baseline.trim(), "HEAD") {
        Err(doctor_git::GitError::Snapshot { path, .. }) => assert!(!path.exists()),
        _ => panic!("expected cleaned unsupported-entry error"),
    }
    assert_eq!(git(dir.path(), &["rev-parse", "HEAD"]), head);
    assert!(dir.path().join("unsafe-link").is_symlink());
}

#[test]
fn promisor_repository_is_rejected_without_remote_access() {
    let dir = repo();
    git(dir.path(), &["config", "remote.origin.promisor", "true"]);
    assert!(matches!(
        GitRepository::open(dir.path(), Duration::from_secs(10)),
        Err(doctor_git::GitError::UnsupportedEntry(_))
    ));
}

#[test]
fn target_tool_selection_is_omitted_without_modifying_originals() {
    let dir = repo();
    fs::create_dir(dir.path().join(".cargo")).unwrap();
    let config = "[build]\nrustc-wrapper = \"./untrusted\"\n";
    let toolchain = "[toolchain]\nchannel = \"nonexistent-test-toolchain\"\n";
    fs::write(dir.path().join(".cargo/config.toml"), config).unwrap();
    fs::write(dir.path().join("rust-toolchain.toml"), toolchain).unwrap();
    git(
        dir.path(),
        &["add", ".cargo/config.toml", "rust-toolchain.toml"],
    );
    git(
        dir.path(),
        &[
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.invalid",
            "commit",
            "-m",
            "tool configuration",
        ],
    );
    let repo = GitRepository::open(dir.path(), Duration::from_secs(10)).unwrap();
    let snapshots = repo.snapshots("HEAD", "HEAD").unwrap();
    assert!(!snapshots.from_path().join(".cargo").exists());
    assert!(!snapshots.to_path().join("rust-toolchain.toml").exists());
    snapshots.close().unwrap();
    assert_eq!(
        fs::read_to_string(dir.path().join(".cargo/config.toml")).unwrap(),
        config
    );
    assert_eq!(
        fs::read_to_string(dir.path().join("rust-toolchain.toml")).unwrap(),
        toolchain
    );
}
