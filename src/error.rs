use std::fmt;
use std::io;

/// Validation / configuration problems that get shown to the user.
/// Kept as a key (not a message) so `i18n` can translate it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Invalid {
    KubeUser,
    SavedCredentials,
    SetupPrerequisite,
    UnsupportedPlatform,
    DownloadVerification,
    PodName,
    CpuValue,
    MemoryValue,
    NoReleaseLabel,
    ReleaseChanged,
}

#[derive(Debug)]
pub enum ThumedError {
    Io(io::Error),
    /// Executable not found on PATH (kubectl / helm).
    MissingTool(String),
    /// kubeconfig itself is absent.
    MissingKubeconfig(String),
    CommandFailed {
        cmd: String,
        stderr: String,
    },
    PodNotFound(String),
    PodStartupFailed {
        release: String,
        detail: String,
    },
    PodStartupTimeout {
        seconds: u64,
        detail: String,
    },
    Invalid(Invalid),
}

impl Invalid {
    /// English fallback; `i18n::error_text` provides the localized version.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::KubeUser => "The active kubeconfig context has no valid username.",
            Self::SavedCredentials => "Invalid saved account or password. Update user information.",
            Self::SetupPrerequisite => "Skipped because a prerequisite check failed.",
            Self::UnsupportedPlatform => {
                "Automatic installation supports macOS, Linux and Windows on x86_64/arm64."
            }
            Self::DownloadVerification => "Download verification failed; no tool was installed.",
            Self::PodName => "Pod name must contain only lowercase letters and numbers.",
            Self::CpuValue => "CPU cores must be a whole number between 1 and 255.",
            Self::MemoryValue => "Memory GB must be a whole number between 1 and 255.",
            Self::NoReleaseLabel => "Pod has no Helm release label.",
            Self::ReleaseChanged => "Pod Helm release changed; select the pod again.",
        }
    }
}

impl fmt::Display for ThumedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "IO error: {}", e),
            Self::MissingTool(tool) => write!(f, "'{}' was not found on PATH", tool),
            Self::MissingKubeconfig(path) => write!(f, "kubeconfig '{}' was not found", path),
            Self::CommandFailed { cmd, stderr } => {
                write!(f, "Command '{}' failed: {}", cmd, stderr.trim())
            }
            Self::PodNotFound(name) => write!(f, "Pod '{}' not found", name),
            Self::PodStartupFailed { release, detail } => {
                write!(
                    f,
                    "Pod startup failed for release '{}': {}",
                    release, detail
                )
            }
            Self::PodStartupTimeout { seconds, detail } => {
                write!(
                    f,
                    "Pod Running was not confirmed within {} seconds: {}",
                    seconds, detail
                )
            }
            Self::Invalid(kind) => write!(f, "{}", kind.as_str()),
        }
    }
}

impl std::error::Error for ThumedError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<io::Error> for ThumedError {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<Invalid> for ThumedError {
    fn from(kind: Invalid) -> Self {
        Self::Invalid(kind)
    }
}

pub type Result<T> = std::result::Result<T, ThumedError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_formats_errors() {
        assert_eq!(
            ThumedError::from(Invalid::PodName).to_string(),
            "Pod name must contain only lowercase letters and numbers."
        );
        assert_eq!(
            ThumedError::MissingTool("helm".to_string()).to_string(),
            "'helm' was not found on PATH"
        );
        assert_eq!(
            ThumedError::CommandFailed {
                cmd: "helm install".to_string(),
                stderr: "release exists\n".to_string(),
            }
            .to_string(),
            "Command 'helm install' failed: release exists"
        );
    }

    #[test]
    fn io_error_keeps_source() {
        let error = ThumedError::from(io::Error::new(io::ErrorKind::NotFound, "missing file"));

        assert!(std::error::Error::source(&error).is_some());
        assert!(error.to_string().contains("IO error: missing file"));
    }
}
