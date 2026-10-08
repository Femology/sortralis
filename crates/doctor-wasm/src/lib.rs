//! Local inspection through the verified Stellar CLI adapter. JSON entries are
//! retained as opaque CLI data, without guessing XDR or custom-section layouts.
use doctor_core::{CommandResult, CommandStatus, REPORT_SCHEMA_VERSION};
use doctor_runner::{
    parse_version, CommandRunner, InfoOperation, StellarCli, StellarError, SystemRunner,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    error::Error,
    fmt, fs,
    io::{self, Read},
    path::{Path, PathBuf},
    time::Duration,
};

pub const WASM_SCOPE: &str = "Local CLI inspection only. Interface/meta/env-meta JSON is preserved as CLI data. Remote build attestations are skipped. No network artifact lookup, code execution, security audit or deployment-safety guarantee.";
#[derive(Debug)]
pub enum OutputError {
    Json(serde_json::Error),
    Shape,
    Hash,
    Version,
}
impl fmt::Display for OutputError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json(e) => write!(f, "invalid JSON: {e}"),
            Self::Shape => f.write_str("expected a JSON array of object entries"),
            Self::Hash => f.write_str("expected one 64-character hexadecimal SHA-256 hash"),
            Self::Version => f.write_str("unrecognized Stellar CLI version output"),
        }
    }
}
impl Error for OutputError {}
pub fn parse_entries(bytes: &[u8]) -> Result<Vec<Value>, OutputError> {
    let values: Vec<Value> = serde_json::from_slice(bytes).map_err(OutputError::Json)?;
    if values.iter().any(|v| !v.is_object()) {
        return Err(OutputError::Shape);
    }
    Ok(values)
}
pub fn parse_hash(bytes: &[u8]) -> Result<String, OutputError> {
    let value = std::str::from_utf8(bytes)
        .map_err(|_| OutputError::Hash)?
        .trim();
    if value.len() != 64 || !value.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(OutputError::Hash);
    }
    Ok(value.to_ascii_lowercase())
}
#[derive(Debug)]
pub enum WasmError {
    File {
        path: PathBuf,
        source: io::Error,
    },
    InvalidFile(PathBuf),
    Stellar(StellarError),
    Output {
        operation: &'static str,
        source: OutputError,
        command: Box<CommandResult>,
    },
    InvalidInterface(String),
}
impl WasmError {
    pub fn exit_code(&self) -> u8 {
        match self {
            Self::File { .. } | Self::InvalidFile(_) => 2,
            Self::Stellar(_) => 3,
            Self::Output { .. } | Self::InvalidInterface(_) => 4,
        }
    }
}
impl fmt::Display for WasmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::File { path, source } => write!(f, "cannot read Wasm {}: {source}", path.display()),
            Self::InvalidFile(path) => write!(f, "{} must be a regular local WebAssembly v1 file (full validation is delegated to Stellar CLI)", path.display()),
            Self::Stellar(e) => write!(f, "{e}"),
            Self::Output { operation, source, command } => write!(f, "malformed Stellar {operation} output: {source}; stdout={:?}; stderr={:?}", command.stdout, command.stderr),
            Self::InvalidInterface(msg) => write!(f, "invalid contract interface: {msg}"),
        }
    }
}
impl Error for WasmError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::File { source, .. } => Some(source),
            Self::Stellar(e) => Some(e),
            Self::Output { source, .. } => Some(source),
            _ => None,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SectionStatus {
    Present,
    Absent,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SectionInspection {
    pub status: SectionStatus,
    pub entries: Vec<Value>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BuildInfoStatus {
    SkippedNetworkPolicy,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildInfo {
    pub status: BuildInfoStatus,
    pub reason: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WasmInspection {
    pub schema_version: String,
    pub tool_version: String,
    pub stellar_cli_version: String,
    pub path: PathBuf,
    pub interface: SectionInspection,
    pub meta: SectionInspection,
    pub env_meta: SectionInspection,
    pub build_info: BuildInfo,
    pub hash: String,
    pub commands: Vec<CommandResult>,
    pub scope: String,
}
fn file_error(path: &Path, source: io::Error) -> WasmError {
    WasmError::File {
        path: path.into(),
        source,
    }
}
fn validate(path: &Path) -> Result<PathBuf, WasmError> {
    let path = fs::canonicalize(path).map_err(|e| file_error(path, e))?;
    if !fs::metadata(&path)
        .map_err(|e| file_error(&path, e))?
        .is_file()
    {
        return Err(WasmError::InvalidFile(path));
    }
    let mut file = fs::File::open(&path).map_err(|e| file_error(&path, e))?;
    let mut header = [0; 8];
    if let Err(e) = file.read_exact(&mut header) {
        return if e.kind() == io::ErrorKind::UnexpectedEof {
            Err(WasmError::InvalidFile(path))
        } else {
            Err(file_error(&path, e))
        };
    }
    // Standard Wasm v1 header, confirmed in the inspected SDK fixture. We do
    // not attempt to parse/validate sections; the real CLI handles that.
    if header != *b"\0asm\x01\0\0\0" {
        return Err(WasmError::InvalidFile(path));
    }
    Ok(path)
}
/// CLI 27 exact missing-section diagnostic, recorded from local minimal Wasm.
/// Other failures (including near-matching diagnostics) stay external errors.
pub fn is_absent_metadata(stderr: &str) -> bool {
    stderr
        .lines()
        .last()
        .is_some_and(|line| line.trim() == "❌ error: no meta present in provided WASM file")
}
fn section<R: CommandRunner>(
    cli: &StellarCli<'_, R>,
    operation: InfoOperation,
    name: &'static str,
    path: &Path,
    cwd: &Path,
    commands: &mut Vec<CommandResult>,
) -> Result<SectionInspection, WasmError> {
    let capture = match cli.info(operation, path, cwd) {
        Ok(c) => c,
        Err(StellarError::Failed(c))
            if matches!(operation, InfoOperation::Meta | InfoOperation::EnvMeta)
                && matches!(c.record.status, CommandStatus::Exited { code: 1 })
                && is_absent_metadata(&c.record.stderr) =>
        {
            // Exact CLI 27 diagnostic, verified in meta.rs/env_meta.rs.
            commands.push(c.record);
            return Ok(SectionInspection {
                status: SectionStatus::Absent,
                entries: vec![],
            });
        }
        Err(e) => return Err(WasmError::Stellar(e)),
    };
    let entries = parse_entries(&capture.stdout_bytes).map_err(|source| WasmError::Output {
        operation: name,
        source,
        command: Box::new(capture.record.clone()),
    })?;
    commands.push(capture.record);
    Ok(SectionInspection {
        status: SectionStatus::Present,
        entries,
    })
}
pub fn inspect(path: &Path, directory: &Path) -> Result<WasmInspection, WasmError> {
    inspect_with(
        path,
        directory,
        &StellarCli::new(&SystemRunner, Duration::from_secs(30)),
    )
}
pub fn inspect_with<R: CommandRunner>(
    path: &Path,
    directory: &Path,
    cli: &StellarCli<'_, R>,
) -> Result<WasmInspection, WasmError> {
    let path = validate(path)?;
    let version = cli.version(directory).map_err(WasmError::Stellar)?;
    let stellar_cli_version = parse_version("stellar", &version.record.stdout)
        .or_else(|| parse_version("stellar", &version.record.stderr))
        .ok_or_else(|| WasmError::Output {
            operation: "version",
            source: OutputError::Version,
            command: Box::new(version.record.clone()),
        })?
        .to_string();
    let mut commands = vec![version.record];
    let interface = section(
        cli,
        InfoOperation::Interface,
        "interface",
        &path,
        directory,
        &mut commands,
    )?;
    let meta = section(
        cli,
        InfoOperation::Meta,
        "meta",
        &path,
        directory,
        &mut commands,
    )?;
    let env_meta = section(
        cli,
        InfoOperation::EnvMeta,
        "env-meta",
        &path,
        directory,
        &mut commands,
    )?;
    let hash_command = cli
        .info(InfoOperation::Hash, &path, directory)
        .map_err(WasmError::Stellar)?;
    let hash = parse_hash(&hash_command.stdout_bytes).map_err(|source| WasmError::Output {
        operation: "hash",
        source,
        command: Box::new(hash_command.record.clone()),
    })?;
    commands.push(hash_command.record);
    let reason = match cli.info(InfoOperation::Build, &path, directory) {
        Err(StellarError::NetworkForbidden) => StellarError::NetworkForbidden.to_string(),
        Err(e) => return Err(WasmError::Stellar(e)),
        Ok(_) => return Err(WasmError::Stellar(StellarError::NetworkForbidden)),
    };
    Ok(WasmInspection {
        schema_version: REPORT_SCHEMA_VERSION.into(),
        tool_version: env!("CARGO_PKG_VERSION").into(),
        stellar_cli_version,
        path,
        interface,
        meta,
        env_meta,
        build_info: BuildInfo {
            status: BuildInfoStatus::SkippedNetworkPolicy,
            reason,
        },
        hash,
        commands,
        scope: WASM_SCOPE.into(),
    })
}

pub mod interface;
pub use interface::{
    diff_interfaces, format_spec_type, interface_from_source, parse_contract_interface,
};

pub fn inspect_interface(
    path: &Path,
    directory: &Path,
) -> Result<doctor_core::interface::ContractInterface, WasmError> {
    inspect_interface_with(
        path,
        directory,
        &StellarCli::new(&SystemRunner, Duration::from_secs(30)),
    )
}

pub fn inspect_interface_with<R: CommandRunner>(
    path: &Path,
    directory: &Path,
    cli: &StellarCli<'_, R>,
) -> Result<doctor_core::interface::ContractInterface, WasmError> {
    let path = validate(path)?;
    let mut commands = Vec::new();
    let section_inspection = section(
        cli,
        InfoOperation::Interface,
        "interface",
        &path,
        directory,
        &mut commands,
    )?;
    parse_contract_interface(&section_inspection.entries)
}
