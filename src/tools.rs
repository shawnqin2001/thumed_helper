use crate::error::{Invalid, Result, ThumedError};
use crate::utils::{command, first_line, managed_tool_path, run_cmd, run_command};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SetupStep {
    Check,
    FetchVersion,
    DownloadBinary,
    DownloadChecksum,
    VerifyChecksum,
    Extract,
    VerifyBinary,
    InstallBinary,
    Configure,
    Update,
}

/// Reuse working tools. Missing tools are installed per-user, never with sudo.
pub fn ensure(tool: &str, progress: &mut dyn FnMut(SetupStep) -> Result<()>) -> Result<String> {
    progress(SetupStep::Check)?;
    let args = version_args(tool);
    let version = match run_cmd(tool, args) {
        Ok(version) => version,
        Err(ThumedError::MissingTool(_)) => {
            install(tool, progress)?;
            run_cmd(tool, args)?
        }
        Err(error) => return Err(error),
    };
    let program = command(tool).get_program().to_string_lossy().into_owned();
    let location = if Path::new(&program).is_absolute() {
        program
    } else {
        format!("PATH: {}", program)
    };
    Ok(format!("{}\n{}", first_line(&version), location))
}

fn version_args(tool: &str) -> &'static [&'static str] {
    match tool {
        "kubectl" => &["version", "--client"],
        "helm" => &["version", "--short"],
        _ => unreachable!("only kubectl and helm are managed"),
    }
}

fn platform(os: &str, arch: &str) -> Result<(&'static str, &'static str)> {
    let os = match os {
        "macos" => "darwin",
        "linux" => "linux",
        "windows" => "windows",
        _ => return Err(Invalid::UnsupportedPlatform.into()),
    };
    let arch = match arch {
        "x86_64" => "amd64",
        "aarch64" => "arm64",
        _ => return Err(Invalid::UnsupportedPlatform.into()),
    };
    Ok((os, arch))
}

fn release_version(text: &str, major: &str) -> Result<String> {
    let version = text.trim();
    let parts: Vec<_> = version.strip_prefix('v').unwrap_or("").split('.').collect();
    if parts.len() != 3
        || parts[0] != major
        || parts
            .iter()
            .any(|part| part.is_empty() || !part.bytes().all(|c| c.is_ascii_digit()))
    {
        return Err(Invalid::DownloadVerification.into());
    }
    Ok(version.to_string())
}

struct InstallDir(PathBuf);

impl Drop for InstallDir {
    fn drop(&mut self) {
        // Only the unique staging directory created by this installation.
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn install(tool: &str, progress: &mut dyn FnMut(SetupStep) -> Result<()>) -> Result<()> {
    let (os, arch) = platform(std::env::consts::OS, std::env::consts::ARCH)?;
    let destination = managed_tool_path(tool);
    let bin_dir = destination.parent().expect("managed tools have a parent");
    fs::create_dir_all(bin_dir)?;
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let stage = bin_dir.join(format!(
        ".install-{}-{}-{}",
        tool,
        std::process::id(),
        nonce
    ));
    fs::create_dir(&stage)?;
    let stage = InstallDir(stage);

    let (version_url, major) = match tool {
        "kubectl" => ("https://dl.k8s.io/release/stable.txt", "1"),
        // Stay on Helm 3 for compatibility with the existing cluster/chart.
        "helm" => ("https://get.helm.sh/helm3-latest-version", "3"),
        _ => unreachable!(),
    };
    let metadata = stage.0.join("version");
    progress(SetupStep::FetchVersion)?;
    download(version_url, &metadata)?;
    let version = release_version(&fs::read_to_string(metadata)?, major)?;
    let executable = format!("{}{}", tool, std::env::consts::EXE_SUFFIX);
    let url = if tool == "kubectl" {
        format!(
            "https://dl.k8s.io/release/{}/bin/{}/{}/{}",
            version, os, arch, executable
        )
    } else {
        format!(
            "https://get.helm.sh/helm-{}-{}-{}.tar.gz",
            version, os, arch
        )
    };
    let payload = stage.0.join("download");
    let checksum = stage.0.join("checksum");
    progress(SetupStep::DownloadBinary)?;
    download(&url, &payload)?;
    progress(SetupStep::DownloadChecksum)?;
    download(
        &format!(
            "{}.{}",
            url,
            if tool == "kubectl" {
                "sha256"
            } else {
                "sha256sum"
            }
        ),
        &checksum,
    )?;
    progress(SetupStep::VerifyChecksum)?;
    verify_checksum(&payload, &fs::read_to_string(checksum)?)?;

    let candidate = if tool == "kubectl" {
        // Windows needs the .exe suffix even for a staged version check.
        let candidate = stage.0.join(&executable);
        fs::rename(payload, &candidate)?;
        candidate
    } else {
        let member = format!("{}-{}/{}", os, arch, executable);
        progress(SetupStep::Extract)?;
        // Extract only the expected executable, not an arbitrary archive tree.
        run_command(
            Command::new("tar")
                .arg("-xzf")
                .arg(&payload)
                .arg("-C")
                .arg(&stage.0)
                .arg("--")
                .arg(&member),
        )?;
        stage.0.join(member)
    };
    publish(&candidate, &destination, tool, progress)
}

fn download(url: &str, output: &Path) -> Result<()> {
    // curl and tar ship with current macOS/Windows and common Linux systems.
    // Never execute a downloaded installer script or disable TLS verification.
    run_command(
        Command::new("curl")
            .args([
                "--fail",
                "--silent",
                "--show-error",
                "--location",
                "--proto",
                "=https",
                "--proto-redir",
                "=https",
                "--tlsv1.2",
                "--connect-timeout",
                "15",
                "--max-time",
                "120",
                "--output",
            ])
            .arg(output)
            .arg(url),
    )?;
    Ok(())
}

fn verify_checksum(path: &Path, expected: &str) -> Result<()> {
    let expected = expected.split_whitespace().next().unwrap_or("");
    if expected.len() != 64 || !expected.bytes().all(|c| c.is_ascii_hexdigit()) {
        return Err(Invalid::DownloadVerification.into());
    }
    let actual = if cfg!(windows) {
        run_command(
            Command::new("powershell.exe")
                .args([
                    "-NoLogo",
                    "-NoProfile",
                    "-NonInteractive",
                    "-Command",
                    "(Get-FileHash -Algorithm SHA256 -LiteralPath $env:THUMED_HASH_FILE).Hash",
                ])
                .env("THUMED_HASH_FILE", path),
        )?
    } else if cfg!(target_os = "macos") {
        run_command(Command::new("shasum").args(["-a", "256"]).arg(path))?
    } else {
        run_command(Command::new("sha256sum").arg(path))?
    };
    if !actual
        .split_whitespace()
        .next()
        .unwrap_or("")
        .eq_ignore_ascii_case(expected)
    {
        return Err(Invalid::DownloadVerification.into());
    }
    Ok(())
}

fn publish(
    candidate: &Path,
    destination: &Path,
    tool: &str,
    progress: &mut dyn FnMut(SetupStep) -> Result<()>,
) -> Result<()> {
    if !fs::symlink_metadata(candidate)?.file_type().is_file() {
        return Err(Invalid::DownloadVerification.into());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(candidate, fs::Permissions::from_mode(0o755))?;
    }
    progress(SetupStep::VerifyBinary)?;
    run_command(Command::new(candidate).args(version_args(tool)))?;
    // Do not replace an existing executable (including one installed concurrently).
    if destination.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            destination.display().to_string(),
        )
        .into());
    }
    progress(SetupStep::InstallBinary)?;
    fs::rename(candidate, destination)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platforms_and_version_metadata_are_validated() {
        for (os, expected) in [
            ("macos", "darwin"),
            ("linux", "linux"),
            ("windows", "windows"),
        ] {
            assert_eq!(platform(os, "x86_64").unwrap(), (expected, "amd64"));
            assert_eq!(platform(os, "aarch64").unwrap(), (expected, "arm64"));
        }
        assert!(platform("linux", "unknown").is_err());
        assert!(platform("unknown", "x86_64").is_err());
        assert_eq!(release_version("v3.22.0\n", "3").unwrap(), "v3.22.0");
        for invalid in ["", "v3/../../file", "v4.0.0", "v3.1", "v3.1.2;exit"] {
            assert!(release_version(invalid, "3").is_err());
        }
    }

    #[test]
    fn checksum_failure_does_not_publish_a_binary() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("thumed-checksum-{}", nonce));
        fs::create_dir(&dir).unwrap();
        let _stage = InstallDir(dir.clone());
        let payload = dir.join("download");
        fs::write(&payload, b"abc").unwrap();
        verify_checksum(
            &payload,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad  download",
        )
        .unwrap();
        assert!(verify_checksum(&payload, &"0".repeat(64)).is_err());
        assert!(verify_checksum(&payload, "not-a-checksum").is_err());
        assert!(!dir.join("kubectl").exists());
    }
}
