use std::env::{JoinPathsError, VarError};
use std::fmt;
use std::io;

#[derive(Debug)]
pub enum ThumedError {
    Io(io::Error),
    Config(String),
    PodNotFound(String),
    CommandFailed { cmd: String, stderr: String },
    EnvVar(String),
}

impl fmt::Display for ThumedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ThumedError::Io(e) => write!(f, "IO error: {}", e),
            ThumedError::Config(msg) => write!(f, "Configuration error: {}", msg),
            ThumedError::PodNotFound(name) => write!(f, "Pod '{}' not found", name),
            ThumedError::CommandFailed { cmd, stderr } => {
                write!(f, "Command '{}' failed: {}", cmd, stderr)
            }
            ThumedError::EnvVar(msg) => write!(f, "Environment variable error: {}", msg),
        }
    }
}

impl std::error::Error for ThumedError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ThumedError::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<io::Error> for ThumedError {
    fn from(e: io::Error) -> Self {
        ThumedError::Io(e)
    }
}

impl From<VarError> for ThumedError {
    fn from(e: VarError) -> Self {
        ThumedError::EnvVar(e.to_string())
    }
}

impl From<JoinPathsError> for ThumedError {
    fn from(e: JoinPathsError) -> Self {
        ThumedError::EnvVar(e.to_string())
    }
}

impl From<String> for ThumedError {
    fn from(e: String) -> Self {
        ThumedError::Config(e)
    }
}

impl From<&str> for ThumedError {
    fn from(e: &str) -> Self {
        ThumedError::Config(e.to_string())
    }
}

pub type Result<T> = std::result::Result<T, ThumedError>;