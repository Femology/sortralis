//! Contract interface comparison orchestration.
//! Prefers built Wasm contract specs; falls back to labeled source approximation.
use doctor_core::interface::{AnalysisSource, ContractInterface, InterfaceDiff};
use doctor_runner::{CommandRunner, StellarCli, SystemRunner};
use std::{
    error::Error,
    fmt, io,
    path::{Path, PathBuf},
    time::Duration,
};

#[derive(Debug)]
pub enum CompareError {
    PathNotFound(PathBuf),
    Wasm(doctor_wasm::WasmError),
    Source(doctor_source::SourceError),
    Cargo(doctor_cargo::CargoError),
    Io { path: PathBuf, source: io::Error },
}

impl fmt::Display for CompareError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PathNotFound(p) => write!(f, "path not found: {}", p.display()),
            Self::Wasm(e) => write!(f, "{e}"),
            Self::Source(e) => write!(f, "{e}"),
            Self::Cargo(e) => write!(f, "{e}"),
            Self::Io { path, source } => write!(f, "cannot access {}: {source}", path.display()),
        }
    }
}

impl Error for CompareError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Wasm(e) => Some(e),
            Self::Source(e) => Some(e),
            Self::Cargo(e) => Some(e),
            Self::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

impl CompareError {
    pub fn exit_code(&self) -> u8 {
        match self {
            Self::PathNotFound(_) => 2,
            Self::Wasm(e) => e.exit_code(),
            Self::Source(_) => 2,
            Self::Cargo(_) => 2,
            Self::Io { .. } => 2,
        }
    }
}

/// Load a normalized contract interface from a Wasm file or project directory.
/// Prefers built Wasm; falls back to source-derived approximation.
pub fn load_interface(path: &Path) -> Result<ContractInterface, CompareError> {
    if !path.exists() {
        return Err(CompareError::PathNotFound(path.to_path_buf()));
    }

    // 1. Direct Wasm file
    if path.is_file() {
        let parent = path.parent().unwrap_or_else(|| Path::new("."));
        return doctor_wasm::inspect_interface(path, parent).map_err(CompareError::Wasm);
    }

    // 2. Directory: try to discover and build Wasm contract spec first
    if path.is_dir() {
        if let Ok(analysis) = doctor_cargo::analyze(path, Duration::from_secs(30)) {
            let soroban_cdylib_pkg = analysis
                .contracts
                .iter()
                .find(|c| c.crate_types.iter().any(|t| t == "cdylib"));

            if let Some(contract) = soroban_cdylib_pkg {
                let cli = StellarCli::new(&SystemRunner, Duration::from_secs(60));
                // Check if Stellar CLI is available
                if let Ok(version_res) = cli.version(path) {
                    if version_res.record.status == (doctor_core::CommandStatus::Exited { code: 0 })
                    {
                        if let Ok(temp_dir) = tempfile::tempdir() {
                            let build_req = StellarCli::<SystemRunner>::build_to_request(
                                &analysis.workspace_root,
                                Duration::from_secs(120),
                                &contract.package.name,
                                temp_dir.path(),
                            );
                            if let Ok(build_res) = SystemRunner.execute(&build_req) {
                                if build_res.record.status
                                    == (doctor_core::CommandStatus::Exited { code: 0 })
                                {
                                    let wasm_name =
                                        format!("{}.wasm", contract.package.name.replace('-', "_"));
                                    let wasm_path = temp_dir.path().join(wasm_name);
                                    if wasm_path.is_file() {
                                        if let Ok(iface) = doctor_wasm::inspect_interface(
                                            &wasm_path,
                                            temp_dir.path(),
                                        ) {
                                            return Ok(iface);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Fallback: source-derived approximation
        let source_opts = doctor_source::SourceOptions::default();
        let source_inv = doctor_source::inventory_interface_directory(path, &source_opts)
            .map_err(CompareError::Source)?;
        let mut iface = doctor_wasm::interface_from_source(&source_inv);
        iface.analysis_source = AnalysisSource::SourceApproximation;
        return Ok(iface);
    }

    Err(CompareError::PathNotFound(path.to_path_buf()))
}

/// Compare contract interfaces between two paths (Wasm artifacts or directories).
pub fn compare(before: &Path, after: &Path) -> Result<InterfaceDiff, CompareError> {
    let before_iface = load_interface(before)?;
    let after_iface = load_interface(after)?;
    Ok(doctor_wasm::diff_interfaces(&before_iface, &after_iface))
}
