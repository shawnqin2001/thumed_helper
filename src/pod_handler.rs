use crate::constants;
use crate::environment;
use crate::environment::DirManager;
use crate::error::{Result, ThumedError};
use crate::utils;
use std::fs;
use std::io::{self, Write};
use std::process::{Command, Stdio};

#[derive(Debug)]
pub struct PodConfig {
    container_name: String,
    cpu: Option<u8>,
    memory: Option<u8>,
}

impl PodConfig {
    /// Create a new PodConfig interactively by prompting the user
    pub fn new() -> Self {
        let mut container_name = String::new();
        loop {
            container_name.clear();
            println!("Please input the pod's name (only lowercase letters and numbers allowed):");
            io::stdin()
                .read_line(&mut container_name)
                .expect("Failed to read line");
            container_name = container_name.trim().to_string();
            if container_name
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
                && !container_name.is_empty()
            {
                break;
            } else {
                println!(
                    "Invalid input. Please enter a valid name: only lowercase letters and numbers are allowed."
                );
            }
        }
        let cpu = Self::prompt_cpu();
        let memory = Self::prompt_memory();
        PodConfig {
            container_name,
            cpu,
            memory,
        }
    }

    /// Create a PodConfig from CLI arguments (non-interactive)
    pub fn from_args(name: String, cpu: Option<u8>, memory: Option<u8>) -> Self {
        PodConfig {
            container_name: name,
            cpu,
            memory,
        }
    }

    fn prompt_cpu() -> Option<u8> {
        let mut cpu = String::new();
        println!(
            "Please input the CPU limit (in cores, default: {}):",
            constants::DEFAULT_CPU_CORES
        );
        io::stdin()
            .read_line(&mut cpu)
            .expect("Failed to read line");
        if cpu.trim().is_empty() {
            None
        } else {
            match cpu.trim().parse::<u8>() {
                Ok(cpu) => Some(cpu),
                Err(_) => {
                    println!("Invalid input. Using default.");
                    None
                }
            }
        }
    }

    fn prompt_memory() -> Option<u8> {
        let mut memory = String::new();
        println!(
            "Please input the memory limit (in GB, default: {}):",
            constants::DEFAULT_MEMORY_GB
        );
        io::stdin()
            .read_line(&mut memory)
            .expect("Failed to read line");
        if memory.trim().is_empty() {
            None
        } else {
            match memory.trim().parse::<u8>() {
                Ok(memory) => Some(memory),
                Err(_) => {
                    println!("Invalid input. Using default.");
                    None
                }
            }
        }
    }

    fn get_cpu(&self) -> u8 {
        self.cpu.unwrap_or(constants::DEFAULT_CPU_CORES)
    }
    fn get_memory(&self) -> u8 {
        self.memory.unwrap_or(constants::DEFAULT_MEMORY_GB)
    }
    pub fn save_config_yaml(&self, dirman: &DirManager) -> Result<()> {
        let user_info = environment::UserInfo::load(dirman)?;
        let yaml_content = format!(
            r#"replicaCount: 1

image:
  repository: base.med.thu/public/r-4.3
  pullPolicy: Always
  tag: "v1"

containerName: "{container_name}"

service:
  type: ClusterIP
  port: 8787

resources:
  limits:
    cpu: "{cpu}"
    memory: "{memory}"

imageCredentials:
  registry: base.med.thu
  username: {username}
  password: {password}

loadDataPath:
  public:
    - "input"
    - "lessonPublic"
  personal:
    - "{username}"

type: centos

nfs: "Aries"

transfer: false
        "#,
            container_name = self.container_name,
            cpu = self.get_cpu(),
            memory = self.get_memory(),
            username = user_info.user,
            password = user_info.password
        );
        let config_dir = &dirman.config_dir;
        let file_path = config_dir.join(format!("{}.yaml", self.container_name));
        let mut file = fs::File::create(&file_path)?;
        file.write_all(yaml_content.as_bytes())?;
        println!("Configuration saved to {}", file_path.display());
        Ok(())
    }
    pub fn install_pod(&self, dirman: &DirManager) -> Result<()> {
        let config_dir = &dirman.config_dir;
        let file_path = config_dir.join(format!("{}.yaml", self.container_name));
        if !file_path.exists() {
            return Err(ThumedError::Config(format!(
                "Configuration file not found: {}",
                file_path.display()
            )));
        }
        let output = Command::new("helm")
            .args([
                "install",
                &self.container_name,
                "med-helm/alpha",
                "-f",
                &file_path.to_string_lossy(),
            ])
            .output()?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(ThumedError::CommandFailed {
                cmd: "helm install".to_string(),
                stderr: stderr.to_string(),
            });
        }
        println!("Pod installed successfully.");
        Ok(())
    }
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

    pub fn display(&self) {
        println!("Pods:");
        for pod in &self.pod_list {
            println!("Pod ID: {}", pod);
        }
    }

    pub fn forward_pod(&self) -> Result<()> {
        println!("Select the pod name to access the web service:");
        let mut pod_name = String::new();
        io::stdin().read_line(&mut pod_name)?;
        let pod_name = pod_name.trim();
        self.forward_pod_by_name(pod_name)
    }

    pub fn forward_pod_by_name(&self, pod_name: &str) -> Result<()> {
        if !self.pod_list.contains(&pod_name.to_string()) {
            return Err(ThumedError::PodNotFound(pod_name.to_string()));
        }
        println!("Port-forward to pod: {}...", pod_name);
        let mut child = Command::new("kubectl")
            .args(["port-forward", pod_name, "8787:8787"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
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

    pub fn login_pod(&self) -> Result<()> {
        println!("Please input the pod name you want to log in:");
        let mut pod_name = String::new();
        io::stdin().read_line(&mut pod_name)?;
        let pod_name = pod_name.trim();
        self.login_pod_by_name(pod_name)
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
    pub fn uninstall_pod(&mut self) -> Result<()> {
        println!("Please input the pod name you want to uninstall:");
        let mut pod_name = String::new();
        io::stdin().read_line(&mut pod_name)?;
        let pod_name = pod_name.trim().to_string();
        self.uninstall_pod_by_name(&pod_name)
    }

    pub fn uninstall_pod_by_name(&mut self, pod_name: &str) -> Result<()> {
        if !self.pod_list.contains(&pod_name.to_string()) {
            return Err(ThumedError::PodNotFound(pod_name.to_string()));
        }

        let podname_split = pod_name.split('-').next().unwrap_or(pod_name);

        let output = Command::new("helm")
            .args(["uninstall", podname_split])
            .output()?;

        if output.status.success() {
            println!("Pod uninstalled successfully.");
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
    fn pod_config_from_args_preserves_values() {
        let config = PodConfig::from_args("pod01".to_string(), Some(8), Some(32));

        assert_eq!(config.container_name, "pod01");
        assert_eq!(config.get_cpu(), 8);
        assert_eq!(config.get_memory(), 32);
    }

    #[test]
    fn pod_config_uses_defaults_when_limits_are_missing() {
        let config = PodConfig::from_args("pod01".to_string(), None, None);

        assert_eq!(config.get_cpu(), constants::DEFAULT_CPU_CORES);
        assert_eq!(config.get_memory(), constants::DEFAULT_MEMORY_GB);
    }

    #[test]
    fn save_config_yaml_writes_expected_values() {
        let config_dir = unique_temp_dir("pod_yaml");
        std::fs::create_dir_all(&config_dir).unwrap();
        std::fs::write(config_dir.join("user.config"), "alice\nsecret\n").unwrap();
        let dirman = DirManager {
            config_dir: config_dir.clone(),
            bin_dir: unique_temp_dir("bin"),
        };
        let config = PodConfig::from_args("pod01".to_string(), Some(4), Some(24));

        config.save_config_yaml(&dirman).unwrap();

        let yaml = std::fs::read_to_string(config_dir.join("pod01.yaml")).unwrap();
        assert!(yaml.contains("containerName: \"pod01\""));
        assert!(yaml.contains("cpu: \"4\""));
        assert!(yaml.contains("memory: \"24\""));
        assert!(yaml.contains("username: alice"));
        assert!(yaml.contains("password: secret"));
        assert!(yaml.contains("- \"alice\""));
    }

    #[test]
    fn install_pod_errors_when_config_file_is_missing() {
        let config_dir = unique_temp_dir("missing_pod_yaml");
        std::fs::create_dir_all(&config_dir).unwrap();
        let dirman = DirManager {
            config_dir,
            bin_dir: unique_temp_dir("bin"),
        };
        let config = PodConfig::from_args("pod01".to_string(), None, None);

        let error = config.install_pod(&dirman).unwrap_err();

        assert!(matches!(error, ThumedError::Config(_)));
        assert!(error.to_string().contains("Configuration file not found"));
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
