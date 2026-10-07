//! Read-only Cargo workspace discovery using format-version 1 metadata.
use cargo_metadata::{DependencyKind, Metadata};
use doctor_core::{CommandStatus, DetectedPackage, Evidence};
use doctor_runner::{CapturedCommand, CommandRunner, CommandSpec, RunnerError, SystemRunner};
use std::{
    collections::BTreeSet,
    error::Error,
    fmt, fs, io,
    path::{Path, PathBuf},
    time::Duration,
};

#[derive(Debug)]
pub struct ContractCandidate {
    pub package: DetectedPackage,
    pub sdk_dependencies: Vec<SdkDependency>,
    pub crate_types: Vec<String>,
    /// Source directories of normal library/binary targets (not build scripts,
    /// integration tests, examples or benchmarks), from exact metadata fields.
    pub source_roots: Vec<PathBuf>,
    pub evidence: Vec<Evidence>,
}

#[derive(Debug)]
pub struct SdkDependency {
    pub requirement: String,
    pub alias: Option<String>,
    pub optional: bool,
    pub target: Option<String>,
}

#[derive(Debug)]
pub struct CargoAnalysis {
    pub repository_path: PathBuf,
    pub workspace_root: PathBuf,
    pub packages: Vec<DetectedPackage>,
    pub contracts: Vec<ContractCandidate>,
    /// Includes raw stdout/stderr, even when the external command fails.
    pub command: CapturedCommand,
}

#[derive(Debug)]
pub enum CargoError {
    Path {
        path: PathBuf,
        source: io::Error,
    },
    NoManifest {
        path: PathBuf,
    },
    Runner(RunnerError),
    MetadataFailed(Box<CapturedCommand>),
    InvalidMetadata {
        source: serde_json::Error,
        command: Box<CapturedCommand>,
    },
}
impl fmt::Display for CargoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Path { path, source } => write!(f, "cannot access {}: {source}", path.display()),
            Self::NoManifest { path } => write!(
                f,
                "no Cargo.toml at {}; supply a Cargo workspace/package directory or its Cargo.toml",
                path.display()
            ),
            Self::Runner(error) => write!(f, "{error}"),
            Self::MetadataFailed(command) => write!(
                f,
                "Cargo metadata failed ({:?}): {}",
                command.record.status,
                command.record.stderr.trim()
            ),
            Self::InvalidMetadata { source, .. } => {
                write!(f, "invalid Cargo metadata JSON: {source}")
            }
        }
    }
}
impl Error for CargoError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Path { source, .. } => Some(source),
            Self::Runner(source) => Some(source),
            Self::InvalidMetadata { source, .. } => Some(source),
            _ => None,
        }
    }
}

/// Analyze a manifest or the directory containing it. Cargo itself discovers
/// workspace members and inherited dependencies. No resolution, builds, or
/// repository scripts are run; no resolved SDK version is claimed.
pub fn analyze(repository: &Path, timeout: Duration) -> Result<CargoAnalysis, CargoError> {
    analyze_with(repository, timeout, &SystemRunner)
}
pub fn analyze_with(
    repository: &Path,
    timeout: Duration,
    runner: &impl CommandRunner,
) -> Result<CargoAnalysis, CargoError> {
    let path = fs::canonicalize(repository).map_err(|source| CargoError::Path {
        path: repository.to_path_buf(),
        source,
    })?;
    let manifest = if path.is_dir() {
        path.join("Cargo.toml")
    } else {
        path.clone()
    };
    if manifest.file_name().is_none_or(|name| name != "Cargo.toml") || !manifest.is_file() {
        return Err(CargoError::NoManifest { path });
    }
    let directory = manifest
        .parent()
        .ok_or_else(|| CargoError::NoManifest { path: path.clone() })?
        .to_path_buf();
    let command = runner
        .execute(&CommandSpec {
            program: "cargo".into(),
            args: vec![
                "metadata".into(),
                "--format-version".into(),
                "1".into(),
                "--no-deps".into(),
                "--offline".into(),
                "--manifest-path".into(),
                manifest.into_os_string(),
            ],
            working_directory: directory.to_path_buf(),
            timeout,
        })
        .map_err(CargoError::Runner)?;
    if command.record.status != (CommandStatus::Exited { code: 0 }) {
        return Err(CargoError::MetadataFailed(Box::new(command)));
    }
    // Verified against cargo_metadata 0.23.1's Metadata/Dependency/Target types
    // and Cargo's documented format-version 1 schema (see docs/cargo-discovery.md).
    let metadata: Metadata = serde_json::from_slice(&command.stdout_bytes).map_err(|source| {
        CargoError::InvalidMetadata {
            source,
            command: Box::new(command.clone()),
        }
    })?;
    let mut packages = Vec::new();
    let mut contracts = Vec::new();
    for package in metadata.workspace_packages() {
        let sdk_dependencies: Vec<_> = package
            .dependencies
            .iter()
            .filter(|dependency| {
                dependency.name == "soroban-sdk" && dependency.kind == DependencyKind::Normal
            })
            .map(|dependency| SdkDependency {
                requirement: dependency.req.to_string(),
                alias: dependency.rename.clone(),
                optional: dependency.optional,
                target: dependency.target.as_ref().map(ToString::to_string),
            })
            .collect();
        let requirements: BTreeSet<_> = sdk_dependencies
            .iter()
            .map(|sdk| sdk.requirement.clone())
            .collect();
        let detected = DetectedPackage {
            name: package.name.to_string(),
            manifest_path: package.manifest_path.clone().into_std_path_buf(),
            soroban_sdk_requirement: if requirements.is_empty() {
                None
            } else {
                Some(requirements.into_iter().collect::<Vec<_>>().join(", "))
            },
        };
        packages.push(detected.clone());
        if !sdk_dependencies.is_empty() {
            let crate_types: BTreeSet<_> = package
                .targets
                .iter()
                .flat_map(|target| target.crate_types.iter().map(ToString::to_string))
                .collect();
            let evidence = sdk_dependencies.iter().map(|sdk| Evidence {
                path: Some(detected.manifest_path.clone()), line: None,
                message: format!("Cargo metadata reports a normal soroban-sdk dependency requested as {} (optional: {}, target: {}); crate types: {}", sdk.requirement, sdk.optional, sdk.target.as_deref().unwrap_or("all"), crate_types.iter().cloned().collect::<Vec<_>>().join(", ")),
                command: Some(command.record.clone()),
            }).collect();
            contracts.push(ContractCandidate {
                package: detected,
                sdk_dependencies,
                crate_types: crate_types.into_iter().collect(),
                source_roots: package
                    .targets
                    .iter()
                    .filter(|target| {
                        !target.is_custom_build()
                            && !target.is_test()
                            && !target.is_example()
                            && !target.is_bench()
                    })
                    .filter_map(|target| {
                        target
                            .src_path
                            .parent()
                            .map(|path| path.to_path_buf().into_std_path_buf())
                    })
                    .collect::<BTreeSet<_>>()
                    .into_iter()
                    .collect(),
                evidence,
            });
        }
    }
    packages.sort_by(|a, b| a.manifest_path.cmp(&b.manifest_path));
    contracts.sort_by(|a, b| a.package.manifest_path.cmp(&b.package.manifest_path));
    Ok(CargoAnalysis {
        repository_path: path,
        workspace_root: metadata.workspace_root.into_std_path_buf(),
        packages,
        contracts,
        command,
    })
}
