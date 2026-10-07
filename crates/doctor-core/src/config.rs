use crate::Severity;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    error::Error,
    fmt, fs, io,
    path::{Path, PathBuf},
};

/// Kept as specified in the build plan despite the product rename.
pub const CONFIG_FILE_NAME: &str = "upgrade-doctor.toml";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "RawConfig")]
pub struct Config {
    pub fail_on: BTreeSet<Severity>,
    pub run_tests: bool,
    pub build_contracts: bool,
    /// Relative paths are interpreted relative to the target repository by callers.
    pub report_dir: PathBuf,
    /// Path exclusions are data only; matching belongs to later analyzers.
    pub exclude: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            fail_on: BTreeSet::from([Severity::Breaking]),
            run_tests: true,
            build_contracts: true,
            report_dir: PathBuf::from("doctor-reports"),
            exclude: vec!["target".into(), "vendor".into()],
        }
    }
}

#[derive(Deserialize)]
#[serde(default, deny_unknown_fields)]
struct RawConfig {
    fail_on: BTreeSet<Severity>,
    run_tests: bool,
    build_contracts: bool,
    report_dir: PathBuf,
    exclude: Vec<String>,
}

impl Default for RawConfig {
    fn default() -> Self {
        let config = Config::default();
        Self {
            fail_on: config.fail_on,
            run_tests: config.run_tests,
            build_contracts: config.build_contracts,
            report_dir: config.report_dir,
            exclude: config.exclude,
        }
    }
}

impl TryFrom<RawConfig> for Config {
    type Error = ConfigValidationError;

    fn try_from(raw: RawConfig) -> Result<Self, Self::Error> {
        if raw.report_dir.as_os_str().is_empty() {
            return Err(ConfigValidationError::EmptyReportDirectory);
        }
        if raw.exclude.iter().any(|path| path.trim().is_empty()) {
            return Err(ConfigValidationError::EmptyExclusion);
        }
        Ok(Self {
            fail_on: raw.fail_on,
            run_tests: raw.run_tests,
            build_contracts: raw.build_contracts,
            report_dir: raw.report_dir,
            exclude: raw.exclude,
        })
    }
}

impl Config {
    pub fn from_toml(source: &str) -> Result<Self, ConfigError> {
        toml::from_str(source).map_err(|source| ConfigError::Parse { path: None, source })
    }

    /// Only a missing optional file falls back to defaults. No files are written
    /// and no paths or target code are executed.
    pub fn load_optional(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let path = path.as_ref();
        let contents = match fs::read_to_string(path) {
            Ok(contents) => contents,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(source) => {
                return Err(ConfigError::Read {
                    path: path.to_owned(),
                    source,
                })
            }
        };
        toml::from_str(&contents).map_err(|source| ConfigError::Parse {
            path: Some(path.to_owned()),
            source,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigValidationError {
    EmptyReportDirectory,
    EmptyExclusion,
}

impl fmt::Display for ConfigValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyReportDirectory => f.write_str("report_dir must not be empty"),
            Self::EmptyExclusion => f.write_str("exclude entries must not be empty"),
        }
    }
}

impl Error for ConfigValidationError {}

#[derive(Debug)]
pub enum ConfigError {
    Read {
        path: PathBuf,
        source: io::Error,
    },
    Parse {
        path: Option<PathBuf>,
        source: toml::de::Error,
    },
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read { path, source } => {
                write!(f, "cannot read configuration {}: {source}", path.display())
            }
            Self::Parse {
                path: Some(path),
                source,
            } => write!(f, "invalid configuration {}: {source}", path.display()),
            Self::Parse { path: None, source } => write!(f, "invalid configuration: {source}"),
        }
    }
}

impl Error for ConfigError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Read { source, .. } => Some(source),
            Self::Parse { source, .. } => Some(source),
        }
    }
}
