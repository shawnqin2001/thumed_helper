use crate::constants;
use crate::environment;
use crate::environment::DirManager;
use crate::utils;
// use crate::host_handler;
use std::env;
use std::error::Error;
use std::fs;
use std::io;
use std::io::Write;
use std::process::{Command, Stdio};

#[derive(Debug)]
pub struct PodConfig {
    container_name: String,
    cpu: Option<u8>,
    memory: Option<u8>,
}

impl PodConfig {
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
        let mut cpu = String::new();
        println!(
            "Please input the CPU limit (in cores, default: {}):",
            constants::DEFAULT_CPU_CORES
        );
        io::stdin()
            .read_line(&mut cpu)
            .expect("Failed to read line");
        let cpu = if cpu.trim().is_empty() {
            None
        } else {
            match cpu.trim().parse::<u8>() {
                Ok(cpu) => Some(cpu),
                Err(_) => {
                    println!("Invalid input. Please enter a valid number.");
                    None
                }
            }
        };

        let mut memory = String::new();
        println!(
            "Please input the memory limit (in GB, default: {}):",
            constants::DEFAULT_MEMORY_GB
        );
        io::stdin()
            .read_line(&mut memory)
            .expect("Failed to read line");
        let memory = if memory.trim().is_empty() {
            None
        } else {
            match memory.trim().parse::<u8>() {
                Ok(memory) => Some(memory),
                Err(_) => {
                    println!("Invalid input. Please enter a valid number.");
                    None
                }
            }
        };
        PodConfig {
            container_name,
            cpu,
            memory,
        }
    }

    fn get_cpu(&self) -> u8 {
        self.cpu.unwrap_or(constants::DEFAULT_CPU_CORES)
    }
    fn get_memory(&self) -> u8 {
        self.memory.unwrap_or(constants::DEFAULT_MEMORY_GB)
    }
    pub fn save_config_yaml(&self, dirman: &DirManager) -> io::Result<()> {
        let user_info = environment::UserInfo::load(dirman).unwrap();
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
        let config_dir = env::current_dir()?.join("config");
        fs::create_dir_all(&config_dir)?;
        let file_path = config_dir.join(format!("{}.yaml", self.container_name));
        let mut file = fs::File::create(&file_path)?;
        file.write_all(yaml_content.as_bytes())?;
        println!("Configuration saved to {}", file_path.display());
        Ok(())
    }
    pub fn install_pod(&self) -> Result<(), Box<dyn Error>> {
        let config_dir = env::current_dir()?.join("config");
        let file_path = config_dir.join(format!("{}.yaml", self.container_name));
        // let use_portforward = true;
        if !file_path.exists() {
            eprintln!("Configuration file not found: {}", file_path.display());
            return Ok(());
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
            return Err(format!("Error in installing: \n{}", stderr).into());
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
    pub fn get_pod_list(&mut self) -> Result<(), Box<dyn Error>> {
        match utils::run_cmd("kubectl", &["get", "pods"]) {
            Ok(stdout) => {
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
            Err(e) => {
                eprintln!("Failed to get pod list: {}", e);
                Err(e)
            }
        }
    }

    pub fn display(&self) {
        println!("Pods:");
        for pod in &self.pod_list {
            println!("Pod ID: {};", pod);
        }
    }

    pub fn forward_pod(&self) -> Result<(), Box<dyn Error>> {
        println!("Select the pod name to access the web service:");
        let mut pod_name = String::new();
        io::stdin().read_line(&mut pod_name)?;
        let pod_name = pod_name.trim();
        self.forward_pod_by_name(pod_name)
    }

    pub fn forward_pod_by_name(&self, pod_name: &str) -> Result<(), Box<dyn Error>> {
        if self.pod_list.contains(&pod_name.to_string()) {
            println!("Port-forward to pod: {}...", pod_name);
            let mut child = Command::new("kubectl")
                .args(["port-forward", pod_name, "8787:8787"])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .expect("Failed to start kubectl port-forward command");
            println!("Port forwarding started. Press Ctrl+C to stop.");
            println!("Open http://localhost:8787 in your browser to access Rstudio.");

            match child.wait() {
                Ok(status) => {
                    if !status.success() {
                        eprintln!("Error: kubectl command failed with status: {}", status);
                        return Err(
                            format!("kubectl command failed with status: {}", status).into()
                        );
                    }
                    Ok(())
                }
                Err(e) => {
                    eprintln!("Failed to execute kubectl command: {}", e);
                    Err(e.into())
                }
            }
        } else {
            Err(format!("Pod {} not found in the list", pod_name).into())
        }
    }

    pub fn login_pod(&self) -> Result<(), Box<dyn Error>> {
        println!("Please input the pod name you want to log in:");
        let mut pod_name = String::new();
        io::stdin().read_line(&mut pod_name)?;
        let pod_name = pod_name.trim();

        self.login_pod_by_name(pod_name)
    }

    // Login to a pod by its name (for CLI usage)
    pub fn login_pod_by_name(&self, pod_name: &str) -> Result<(), Box<dyn Error>> {
        if self.pod_list.contains(&pod_name.to_string()) {
            println!("Connecting to pod: {}...", pod_name);
            // Use Command::status to run interactively instead of output
            match Command::new("kubectl")
                .args(["exec", "-it", pod_name, "--", "sh", "/cmd.sh"])
                .status()
            {
                Ok(status) => {
                    if !status.success() {
                        eprintln!("Error: kubectl command failed with status: {}", status);
                        return Err(
                            format!("kubectl command failed with status: {}", status).into()
                        );
                    }
                    Ok(())
                }
                Err(e) => {
                    eprintln!("Failed to execute kubectl command: {}", e);
                    Err(e.into())
                }
            }
        } else {
            Err(format!("Pod {} not found in the list", pod_name).into())
        }
    }
    pub fn uninstall_pod(&mut self) -> Result<(), Box<dyn Error>> {
        println!("Please input the pod name you want to uninstall:");
        let mut pod_name = String::new();
        io::stdin().read_line(&mut pod_name)?;
        pod_name = pod_name.trim().to_string();

        self.uninstall_pod_by_name(&pod_name)
    }

    // Uninstall a pod by its name (for CLI usage)
    pub fn uninstall_pod_by_name(&mut self, pod_name: &str) -> Result<(), Box<dyn Error>> {
        if !self.pod_list.contains(&pod_name.to_string()) {
            eprintln!("Pod {} not found in the list.", pod_name);
            return Err(format!("Pod {} not found in the list", pod_name).into());
        }

        let podname_split = pod_name.split('-').next().unwrap_or(pod_name);

        match Command::new("helm")
            .args(["uninstall", podname_split])
            .output()
        {
            Ok(output) => {
                if output.status.success() {
                    println!("Pod uninstalled successfully.");
                    self.get_pod_list()?;
                    Ok(())
                } else {
                    let error_msg = String::from_utf8_lossy(&output.stderr);
                    eprintln!("Error uninstalling pod: {}", error_msg);
                    Err(format!("Failed to uninstall pod: {}", error_msg).into())
                }
            }
            Err(e) => {
                eprintln!("Failed to run helm uninstall command: {}", e);
                Err(e.into())
            }
        }
    }
}
