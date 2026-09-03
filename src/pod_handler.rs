use crate::constants;
use crate::environment;
use crate::environment::DirManager;
use crate::error::{Result, ThumedError};
use crate::utils;
use std::io::Write;
use std::process::{Command, Stdio};

#[derive(Debug)]
pub struct PodConfig {
    container_name: String,
    cpu: Option<u8>,
    memory: Option<u8>,
}

impl PodConfig {
    pub fn from_values(container_name: &str, cpu: &str, memory: &str) -> Result<Self> {
        let container_name = container_name.trim();
        if container_name.is_empty()
            || !container_name
                .chars()
                .all(|character| character.is_ascii_lowercase() || character.is_ascii_digit())
        {
            return Err(ThumedError::Config(
                "Pod name must contain only lowercase letters and numbers.".to_string(),
            ));
        }

        Ok(Self {
            container_name: container_name.to_string(),
            cpu: parse_limit(cpu, "CPU cores")?,
            memory: parse_limit(memory, "Memory GB")?,
        })
    }

    fn get_cpu(&self) -> u8 {
        self.cpu.unwrap_or(constants::DEFAULT_CPU_CORES)
    }
    fn get_memory(&self) -> u8 {
        self.memory.unwrap_or(constants::DEFAULT_MEMORY_GB)
    }
    fn render_values_yaml(&self, dirman: &DirManager) -> Result<String> {
        let user_info = environment::UserInfo::load(dirman)?;
        let cpu = self.get_cpu().to_string();
        let memory = self.get_memory().to_string();

        Ok(constants::HELM_VALUES_TEMPLATE
            .replace("{container_name}", &self.container_name)
            .replace("{cpu}", &cpu)
            .replace("{memory}", &memory)
            .replace("{username}", &yaml_scalar(&user_info.user))
            .replace("{password}", &yaml_scalar(&user_info.password)))
    }

    pub fn install_pod(&self, dirman: &DirManager) -> Result<()> {
        let yaml_content = self.render_values_yaml(dirman)?;
        let mut child = Command::new("helm")
            .args([
                "install",
                &self.container_name,
                constants::HELM_CHART,
                "-f",
                "-",
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        child
            .stdin
            .as_mut()
            .expect("Helm stdin was not piped")
            .write_all(yaml_content.as_bytes())?;

        let output = child.wait_with_output()?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(ThumedError::CommandFailed {
                cmd: "helm install".to_string(),
                stderr: stderr.to_string(),
            });
        }
        Ok(())
    }
}

fn yaml_scalar(value: &str) -> String {
    let mut escaped = String::new();
    for character in value.chars() {
        match character {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            character if character.is_control() => {
                escaped.push_str(&format!("\\u{:04x}", character as u32));
            }
            character => escaped.push(character),
        }
    }
    format!("\"{}\"", escaped)
}

fn parse_limit(value: &str, label: &str) -> Result<Option<u8>> {
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    let value = value.parse::<u8>().map_err(|_| {
        ThumedError::Config(format!(
            "{} must be a whole number between 1 and 255.",
            label
        ))
    })?;
    if value == 0 {
        return Err(ThumedError::Config(format!(
            "{} must be a whole number between 1 and 255.",
            label
        )));
    }
    Ok(Some(value))
}

pub struct PodHandler {
    pub pod_list: Vec<String>,
}

impl PodHandler {
    pub fn new() -> Self {
        PodHandler {
            pod_list: Vec::new(),
        }
    }
    pub fn get_pod_list(&mut self) -> Result<()> {
        let stdout = utils::run_cmd("kubectl", &["get", "pods"])?;
        let lines: Vec<&str> = stdout.lines().collect();
        let mut pod_list = Vec::new();
        for line in lines.iter().skip(1) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if !parts.is_empty() {
                pod_list.push(parts[0].to_string());
            }
        }
        self.pod_list = pod_list;
        Ok(())
    }

    pub fn forward_pod_by_name(&self, pod_name: &str) -> Result<()> {
        if !self.pod_list.contains(&pod_name.to_string()) {
            return Err(ThumedError::PodNotFound(pod_name.to_string()));
        }
        println!("Port-forward to pod: {}...", pod_name);
        let mut child = Command::new("kubectl")
            .args(["port-forward", pod_name, "8787:8787"])
            .stdin(Stdio::null())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|e| ThumedError::CommandFailed {
                cmd: "kubectl port-forward".to_string(),
                stderr: e.to_string(),
            })?;
        println!("Port forwarding started. Press Ctrl+C to stop.");
        println!("Open http://localhost:8787 in your browser to access Rstudio.");

        match child.wait() {
            Ok(status) => {
                if !status.success() {
                    return Err(ThumedError::CommandFailed {
                        cmd: "kubectl port-forward".to_string(),
                        stderr: format!("exit status: {}", status),
                    });
                }
                Ok(())
            }
            Err(e) => Err(ThumedError::Io(e)),
        }
    }

    pub fn login_pod_by_name(&self, pod_name: &str) -> Result<()> {
        if !self.pod_list.contains(&pod_name.to_string()) {
            return Err(ThumedError::PodNotFound(pod_name.to_string()));
        }
        println!("Connecting to pod: {}...", pod_name);
        let status = Command::new("kubectl")
            .args(["exec", "-it", pod_name, "--", "sh", "/cmd.sh"])
            .status()
            .map_err(|e| ThumedError::CommandFailed {
                cmd: "kubectl exec".to_string(),
                stderr: e.to_string(),
            })?;
        if !status.success() {
            return Err(ThumedError::CommandFailed {
                cmd: "kubectl exec".to_string(),
                stderr: format!("exit status: {}", status),
            });
        }
        Ok(())
    }
    pub fn release_for_pod(&self, pod_name: &str) -> Result<String> {
        if !self.pod_list.iter().any(|pod| pod == pod_name) {
            return Err(ThumedError::PodNotFound(pod_name.to_string()));
        }
        let release = utils::run_cmd(
            "kubectl",
            &[
                "get",
                "pod",
                pod_name,
                "-o",
                "jsonpath={.metadata.labels.app\\.kubernetes\\.io/instance}",
            ],
        )?;
        let release = release.trim();
        if release.is_empty() {
            return Err(ThumedError::Config(format!(
                "Pod '{}' has no Helm release label.",
                pod_name
            )));
        }
        utils::run_cmd("helm", &["status", release])?;
        Ok(release.to_string())
    }

    pub fn uninstall_pod_release(&mut self, pod_name: &str, release: &str) -> Result<()> {
        if self.release_for_pod(pod_name)? != release {
            return Err(ThumedError::Config(
                "Pod Helm release changed; select pod again before uninstalling.".to_string(),
            ));
        }

        let output = Command::new("helm").args(["uninstall", release]).output()?;

        if output.status.success() {
            self.get_pod_list()?;
            Ok(())
        } else {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            Err(ThumedError::CommandFailed {
                cmd: "helm uninstall".to_string(),
                stderr: error_msg.to_string(),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_temp_dir(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("thumed_helper_test_{}_{}", name, nanos))
    }

    #[test]
    fn pod_config_preserves_values() {
        let config = PodConfig {
            container_name: "pod01".to_string(),
            cpu: Some(8),
            memory: Some(32),
        };

        assert_eq!(config.container_name, "pod01");
        assert_eq!(config.get_cpu(), 8);
        assert_eq!(config.get_memory(), 32);
    }

    #[test]
    fn pod_config_rejects_invalid_limits() {
        assert!(PodConfig::from_values("pod01", "0", "4").is_err());
        assert!(PodConfig::from_values("pod01", "2", "invalid").is_err());
        assert!(PodConfig::from_values("Pod01", "2", "4").is_err());
    }

    #[test]
    fn pod_config_uses_defaults_when_limits_are_missing() {
        let config = PodConfig {
            container_name: "pod01".to_string(),
            cpu: None,
            memory: None,
        };

        assert_eq!(config.get_cpu(), constants::DEFAULT_CPU_CORES);
        assert_eq!(config.get_memory(), constants::DEFAULT_MEMORY_GB);
    }

    #[test]
    fn render_values_yaml_uses_expected_values_without_writing_file() {
        let config_dir = unique_temp_dir("pod_yaml");
        std::fs::create_dir_all(&config_dir).unwrap();
        std::fs::write(config_dir.join("user.config"), "alice\nsecret\n").unwrap();
        let dirman = DirManager {
            config_dir: config_dir.clone(),
            bin_dir: unique_temp_dir("bin"),
        };
        let config = PodConfig {
            container_name: "pod01".to_string(),
            cpu: Some(4),
            memory: Some(24),
        };

        let yaml = config.render_values_yaml(&dirman).unwrap();

        assert!(yaml.contains("containerName: \"pod01\""));
        assert!(yaml.contains("mode: deployment"));
        assert!(yaml.contains("resources:\n  cpu: \"4\"\n  memory: \"24\""));
        assert!(!yaml.contains("limits:"));
        assert!(yaml.contains("username: \"alice\""));
        assert!(yaml.contains("password: \"secret\""));
        assert!(yaml.contains("- \"alice\""));
        assert!(!config_dir.join("pod01.yaml").exists());
    }

    #[test]
    fn yaml_scalar_escapes_control_and_yaml_characters() {
        assert_eq!(yaml_scalar("#\\\"\n"), "\"#\\\\\\\"\\n\"");
    }

    #[test]
    fn pod_actions_return_pod_not_found_for_unknown_name() {
        let handler = PodHandler {
            pod_list: vec!["known-pod".to_string()],
        };

        let login_error = handler.login_pod_by_name("missing-pod").unwrap_err();
        let forward_error = handler.forward_pod_by_name("missing-pod").unwrap_err();

        assert!(matches!(login_error, ThumedError::PodNotFound(name) if name == "missing-pod"));
        assert!(matches!(forward_error, ThumedError::PodNotFound(name) if name == "missing-pod"));
    }
}
