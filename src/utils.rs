use crate::error::{Result, ThumedError};
use std::process::Command;

pub fn run_cmd(cmd: &str, args: &[&str]) -> Result<String> {
    let output = Command::new(cmd).args(args).output()?;
    if !output.status.success() {
        return Err(ThumedError::CommandFailed {
            cmd: cmd.to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        });
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}
