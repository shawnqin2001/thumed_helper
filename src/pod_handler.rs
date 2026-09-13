use crate::constants;
use crate::environment::UserInfo;
use crate::error::{Invalid, Result, ThumedError};
use crate::utils::{command, run_cmd};
use std::io::Write;
use std::process::{Child, Stdio};

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
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        {
            return Err(Invalid::PodName.into());
        }
        Ok(Self {
            container_name: container_name.to_string(),
            cpu: parse_limit(cpu, Invalid::CpuValue)?,
            memory: parse_limit(memory, Invalid::MemoryValue)?,
        })
    }

    fn render_values_yaml(&self, user_info: &UserInfo) -> String {
        constants::HELM_VALUES_TEMPLATE
            .replace("{container_name}", &self.container_name)
            .replace(
                "{cpu}",
                &self.cpu.unwrap_or(constants::DEFAULT_CPU_CORES).to_string(),
            )
            .replace(
                "{memory}",
                &self
                    .memory
                    .unwrap_or(constants::DEFAULT_MEMORY_GB)
                    .to_string(),
            )
            .replace("{username}", &yaml_scalar(&user_info.user))
            .replace("{password}", &yaml_scalar(&user_info.password))
    }

    pub fn install_pod(&self) -> Result<()> {
        let yaml_content = self.render_values_yaml(&UserInfo::load()?);
        let mut child = command("helm")
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
            .spawn()
            .map_err(spawn_error("helm"))?;

        child
            .stdin
            .as_mut()
            .expect("helm stdin was piped")
            .write_all(yaml_content.as_bytes())?;

        let output = child.wait_with_output()?;
        if !output.status.success() {
            return Err(ThumedError::CommandFailed {
                cmd: "helm install".to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            });
        }
        Ok(())
    }
}

/// Map a spawn failure to a friendly "install this tool" error.
fn spawn_error(tool: &'static str) -> impl Fn(std::io::Error) -> ThumedError {
    move |error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            ThumedError::MissingTool(tool.to_string())
        } else {
            ThumedError::Io(error)
        }
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
            c if c.is_control() => escaped.push_str(&format!("\\u{:04x}", c as u32)),
            c => escaped.push(c),
        }
    }
    format!("\"{}\"", escaped)
}

fn parse_limit(value: &str, kind: Invalid) -> Result<Option<u8>> {
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    match value.parse::<u8>() {
        Ok(0) | Err(_) => Err(kind.into()),
        Ok(parsed) => Ok(Some(parsed)),
    }
}

#[derive(Default)]
pub struct PodHandler {
    pub pod_list: Vec<String>,
}

impl PodHandler {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn refresh(&mut self) -> Result<()> {
        let stdout = run_cmd("kubectl", &["get", "pods"])?;
        self.pod_list = stdout
            .lines()
            .skip(1)
            .filter_map(|line| line.split_whitespace().next())
            .map(str::to_string)
            .collect();
        Ok(())
    }

    fn ensure_known(&self, pod_name: &str) -> Result<()> {
        if self.pod_list.iter().any(|pod| pod == pod_name) {
            Ok(())
        } else {
            Err(ThumedError::PodNotFound(pod_name.to_string()))
        }
    }

    /// Start `kubectl port-forward` in the background; the caller owns the child
    /// and stops it. Output is captured so it cannot corrupt the TUI.
    pub fn start_forward(&self, pod_name: &str) -> Result<Child> {
        self.ensure_known(pod_name)?;
        let ports = format!("{0}:{0}", constants::FORWARD_PORT);
        command("kubectl")
            .args(["port-forward", pod_name, &ports])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(spawn_error("kubectl"))
    }

    /// Interactive shell; the TUI must leave the alternate screen first.
    pub fn login_pod_by_name(&self, pod_name: &str) -> Result<()> {
        self.ensure_known(pod_name)?;
        let status = command("kubectl")
            .args(["exec", "-it", pod_name, "--", "sh", "/cmd.sh"])
            .status()
            .map_err(spawn_error("kubectl"))?;
        if !status.success() {
            return Err(ThumedError::CommandFailed {
                cmd: "kubectl exec".to_string(),
                stderr: status.to_string(),
            });
        }
        Ok(())
    }

    pub fn release_for_pod(&self, pod_name: &str) -> Result<String> {
        self.ensure_known(pod_name)?;
        let release = run_cmd(
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
            return Err(Invalid::NoReleaseLabel.into());
        }
        run_cmd("helm", &["status", release])?;
        Ok(release.to_string())
    }

    pub fn uninstall_pod_release(&mut self, pod_name: &str, release: &str) -> Result<()> {
        if self.release_for_pod(pod_name)? != release {
            return Err(Invalid::ReleaseChanged.into());
        }
        run_cmd("helm", &["uninstall", release])?;
        self.refresh()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pod_config_validates_input() {
        assert!(PodConfig::from_values("pod01", "", "").is_ok());
        assert!(matches!(
            PodConfig::from_values("Pod01", "2", "4").unwrap_err(),
            ThumedError::Invalid(Invalid::PodName)
        ));
        assert!(matches!(
            PodConfig::from_values("pod01", "0", "4").unwrap_err(),
            ThumedError::Invalid(Invalid::CpuValue)
        ));
        assert!(matches!(
            PodConfig::from_values("pod01", "2", "nope").unwrap_err(),
            ThumedError::Invalid(Invalid::MemoryValue)
        ));
    }

    #[test]
    fn render_values_yaml_uses_values_and_defaults() {
        let user = UserInfo::from_username("alice").unwrap();
        let yaml = PodConfig::from_values("pod01", "4", "")
            .unwrap()
            .render_values_yaml(&user);

        assert!(yaml.contains("containerName: \"pod01\""));
        assert!(yaml.contains(&format!(
            "resources:\n  cpu: \"4\"\n  memory: \"{}\"",
            constants::DEFAULT_MEMORY_GB
        )));
        assert!(yaml.contains("username: \"alice\""));
        assert!(yaml.contains("password: \"Test1234\""));
    }

    #[test]
    fn yaml_scalar_escapes_control_and_yaml_characters() {
        assert_eq!(yaml_scalar("#\\\"\n"), "\"#\\\\\\\"\\n\"");
    }

    #[test]
    fn unknown_pod_is_rejected() {
        let handler = PodHandler {
            pod_list: vec!["known-pod".to_string()],
        };

        assert!(matches!(
            handler.login_pod_by_name("missing-pod").unwrap_err(),
            ThumedError::PodNotFound(name) if name == "missing-pod"
        ));
        assert!(matches!(
            handler.start_forward("missing-pod").unwrap_err(),
            ThumedError::PodNotFound(_)
        ));
    }
}
