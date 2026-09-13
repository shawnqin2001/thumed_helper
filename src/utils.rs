use crate::error::{Result, ThumedError};
use std::io;
use std::path::PathBuf;
use std::process::Command;

pub fn managed_tool_path(tool: &str) -> PathBuf {
    crate::environment::config_dir().join("bin").join(format!(
        "{}{}",
        tool,
        std::env::consts::EXE_SUFFIX
    ))
}

/// Use app-owned tools when installed; otherwise reuse the user's PATH.
pub fn command(cmd: &str) -> Command {
    if matches!(cmd, "kubectl" | "helm") {
        let installed = managed_tool_path(cmd);
        if installed.is_file() {
            return Command::new(installed);
        }
    }
    Command::new(cmd)
}

pub fn run_cmd(cmd: &str, args: &[&str]) -> Result<String> {
    run_command(command(cmd).args(args))
}

pub fn run_command(command: &mut Command) -> Result<String> {
    let cmd = command.get_program().to_string_lossy().into_owned();
    let output = command.output().map_err(|error| {
        if error.kind() == io::ErrorKind::NotFound {
            ThumedError::MissingTool(cmd.to_string())
        } else {
            ThumedError::Io(error)
        }
    })?;
    if !output.status.success() {
        let args = command
            .get_args()
            .map(|arg| arg.to_string_lossy())
            .collect::<Vec<_>>()
            .join(" ");
        return Err(ThumedError::CommandFailed {
            cmd: format!("{} {}", cmd, args).trim().to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        });
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// First non-empty line, trimmed and truncated - good enough for version output.
pub fn first_line(text: &str) -> String {
    let line = text
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("");
    line.chars().take(120).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_executable_maps_to_missing_tool() {
        let error = run_cmd("thumed_helper_definitely_missing_binary", &[]).unwrap_err();
        assert!(matches!(error, ThumedError::MissingTool(_)));
    }

    #[test]
    fn first_line_skips_blanks() {
        assert_eq!(first_line("\n  v1.2.3 \nrest"), "v1.2.3");
        assert_eq!(first_line(""), "");
    }
}
