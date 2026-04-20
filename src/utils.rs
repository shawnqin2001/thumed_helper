use crate::{constants, error::{Result, ThumedError}, platform};
use std::path::Path;
use std::process::Command;

pub fn run_cmd(cmd: &str, args: &[&str]) -> Result<String> {
    let mut command = Command::new(cmd);
    command.args(args);
    let output = command.output()?;

    if !output.status.success() {
        let error_message = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(ThumedError::CommandFailed {
            cmd: cmd.to_string(),
            stderr: error_message,
        });
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

pub fn download_file(url: &str, output_path: &Path) -> Result<()> {
    println!("Downloading from: {}", url);

    if platform::is_windows() {
        let status = Command::new("powershell")
            .arg("-Command")
            .arg(format!(
                "Invoke-WebRequest -Uri '{}' -OutFile '{}'",
                url,
                output_path.to_string_lossy()
            ))
            .status()?;

        if !status.success() {
            return Err(ThumedError::CommandFailed {
                cmd: "powershell Invoke-WebRequest".to_string(),
                stderr: format!("Failed to download from {}", url),
            });
        }
    } else {
        let status = Command::new("curl")
            .args(["-L", "-o", &output_path.to_string_lossy(), url])
            .status()?;

        if !status.success() {
            return Err(ThumedError::CommandFailed {
                cmd: "curl".to_string(),
                stderr: format!("Failed to download from {}", url),
            });
        }
    }

    if platform::is_unix() {
        let status = Command::new("chmod")
            .args(["+x", &output_path.to_string_lossy()])
            .status()?;

        if !status.success() {
            return Err(ThumedError::CommandFailed {
                cmd: "chmod".to_string(),
                stderr: "Failed to make file executable".to_string(),
            });
        }
    }

    println!("Download complete: {}", output_path.display());
    Ok(())
}

fn get_os_and_arch() -> Result<(String, String)> {
    let os = if platform::is_windows() {
        "windows".to_string()
    } else if cfg!(target_os = "macos") {
        "darwin".to_string()
    } else if cfg!(target_os = "linux") {
        "linux".to_string()
    } else {
        return Err(ThumedError::Config("Unsupported operating system".to_string()));
    };

    let arch = if cfg!(target_arch = "x86_64") {
        "amd64".to_string()
    } else if cfg!(target_arch = "aarch64") {
        "arm64".to_string()
    } else {
        return Err(ThumedError::Config("Unsupported architecture".to_string()));
    };

    Ok((os, arch))
}

pub fn download_kubectl(bin_dir: &Path) -> Result<()> {
    let kubectl_path = platform::get_bin_path(bin_dir, "kubectl");

    if kubectl_path.exists() {
        println!("kubectl already exists, skipping download");
        return Ok(());
    }

    println!("Downloading kubectl...");

    let (os, arch) = get_os_and_arch()?;
    let version = constants::KUBECTL_VERSION;

    let download_url = if platform::is_windows() {
        format!(
            "https://dl.k8s.io/release/{}/bin/windows/{}/kubectl.exe",
            version, arch
        )
    } else {
        format!(
            "https://dl.k8s.io/release/{}/bin/{}/{}/kubectl",
            version, os, arch
        )
    };

    download_file(&download_url, &kubectl_path)?;
    println!("kubectl downloaded successfully");
    Ok(())
}

pub fn download_helm(bin_dir: &Path) -> Result<()> {
    let helm_path = platform::get_bin_path(bin_dir, "helm");

    if helm_path.exists() {
        println!("helm already exists, skipping download");
        return Ok(());
    }

    println!("Downloading helm...");

    let (os, arch) = get_os_and_arch()?;
    let version = constants::HELM_VERSION;

    let helm_os = match os.as_str() {
        "darwin" => "darwin",
        "linux" => "linux",
        "windows" => "windows",
        _ => return Err(ThumedError::Config(format!("Unsupported OS: {}", os))),
    };

    let helm_arch = match arch.as_str() {
        "amd64" => "amd64",
        "arm64" => "arm64",
        _ => return Err(ThumedError::Config(format!("Unsupported architecture: {}", arch))),
    };

    let filename = format!("helm-{}-{}-{}", version, helm_os, helm_arch);
    let download_url = format!("https://get.helm.sh/{}.tar.gz", filename);

    let temp_file = bin_dir.join(format!("{}.tar.gz", filename));
    download_file(&download_url, &temp_file)?;

    extract_gz_file(&temp_file, &helm_path)?;
    let extracted_dir = bin_dir.join(format!("{}-{}", helm_os, helm_arch));
    let extracted_file = extracted_dir.join(if platform::is_windows() {
        "helm.exe"
    } else {
        "helm"
    });
    std::fs::rename(extracted_file, &helm_path)?;
    println!("helm downloaded successfully");
    Ok(())
}

fn extract_gz_file(gz_path: &Path, output_path: &Path) -> Result<()> {
    println!("Extracting: {}", gz_path.display());

    let extract_dir = gz_path.parent().unwrap();

    if platform::is_windows() {
        let status = Command::new("powershell")
            .arg("-Command")
            .arg(format!(
                "tar -xzf '{}' -C '{}'",
                gz_path.to_string_lossy(),
                extract_dir.to_string_lossy()
            ))
            .status()?;

        if !status.success() {
            return Err(ThumedError::CommandFailed {
                cmd: "tar".to_string(),
                stderr: format!("Failed to extract {}", gz_path.display()),
            });
        }
    } else {
        let status = Command::new("tar")
            .args([
                "-xzf",
                &gz_path.to_string_lossy(),
                "-C",
                &extract_dir.to_string_lossy(),
            ])
            .status()?;

        if !status.success() {
            return Err(ThumedError::CommandFailed {
                cmd: "tar".to_string(),
                stderr: format!("Failed to extract {}", gz_path.display()),
            });
        }
    }

    std::fs::remove_file(gz_path)?;

    println!("Extraction complete to: {}", output_path.display());
    Ok(())
}
