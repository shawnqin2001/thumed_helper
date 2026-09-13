use crate::constants;
use crate::environment::UserInfo;
use crate::error::{Invalid, Result, ThumedError};
use crate::utils::{command, run_cmd};
use std::io::Write;
use std::process::{Child, Stdio};
use std::time::{Duration, Instant};

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

    pub fn release_name(&self) -> &str {
        &self.container_name
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

#[derive(Debug, PartialEq, Eq)]
enum PodStartup {
    Waiting(String),
    Running(Vec<String>),
    Failed(String),
}

/// Wait as part of creation, not in a background worker or the TUI event loop.
pub fn wait_for_pod_running(release: &str) -> Result<Vec<String>> {
    wait_for_running(
        release,
        Duration::from_secs(30),
        Duration::from_secs(2),
        || check_pod_startup(release),
    )
}

fn wait_for_running(
    release: &str,
    timeout: Duration,
    interval: Duration,
    mut check: impl FnMut() -> Result<PodStartup>,
) -> Result<Vec<String>> {
    let started = Instant::now();
    loop {
        match check()? {
            PodStartup::Running(names) => return Ok(names),
            PodStartup::Failed(detail) => {
                return Err(ThumedError::PodStartupFailed {
                    release: release.to_string(),
                    detail,
                });
            }
            PodStartup::Waiting(detail) => {
                let remaining = timeout.saturating_sub(started.elapsed());
                if remaining.is_zero() {
                    return Err(ThumedError::PodStartupTimeout {
                        seconds: timeout.as_secs(),
                        detail,
                    });
                }
                std::thread::sleep(interval.min(remaining));
            }
        }
    }
}

/// Inspect only pods belonging to the newly installed Helm release.
/// JSONPath emits fixed columns; no human-readable kubectl table parsing.
fn check_pod_startup(release: &str) -> Result<PodStartup> {
    let selector = format!("app.kubernetes.io/instance={}", release);
    let output = run_cmd(
        "kubectl",
        &[
            "get",
            "pods",
            "--selector",
            &selector,
            "--request-timeout=5s",
            "-o",
            concat!(
                "jsonpath={range .items[*]}",
                "{.metadata.name}{\"\\t\"}{.status.phase}{\"\\t\"}",
                "{.status.reason}{\"\\t\"}{.metadata.deletionTimestamp}{\"\\t\"}",
                "{range .status.initContainerStatuses[*]}{.state.waiting.reason}{\",\"}{end}",
                "{range .status.containerStatuses[*]}{.state.waiting.reason}{\",\"}{end}{\"\\t\"}",
                "{range .status.initContainerStatuses[*]}",
                "{.state.terminated.reason}{\":\"}{.state.terminated.exitCode}{\",\"}{end}",
                "{range .status.containerStatuses[*]}",
                "{.state.terminated.reason}{\":\"}{.state.terminated.exitCode}{\",\"}{end}",
                "{\"\\n\"}{end}",
            ),
        ],
    )?;
    parse_pod_startup(&output)
}

fn parse_pod_startup(output: &str) -> Result<PodStartup> {
    let mut summary = Vec::new();
    let mut names = Vec::new();
    let mut failed = false;
    let mut all_running = true;
    for row in output.lines().filter(|row| !row.trim().is_empty()) {
        let fields: Vec<_> = row.split('\t').collect();
        if fields.len() != 6 || fields[0].is_empty() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Unexpected kubectl pod status output",
            )
            .into());
        }
        let (name, phase, reason, deleting) = (fields[0], fields[1], fields[2], fields[3]);
        let waiting: Vec<_> = fields[4].split(',').filter(|s| !s.is_empty()).collect();
        let terminated: Vec<_> = fields[5]
            .split(',')
            .filter(|s| !s.is_empty() && *s != ":")
            .collect();
        // Phase may still be Running while a container is in CrashLoopBackOff.
        failed |= matches!(phase, "Failed" | "Succeeded")
            || waiting.iter().any(|reason| {
                matches!(
                    *reason,
                    "CrashLoopBackOff"
                        | "ErrImagePull"
                        | "ImagePullBackOff"
                        | "InvalidImageName"
                        | "ErrImageNeverPull"
                        | "CreateContainerConfigError"
                        | "CreateContainerError"
                        | "RunContainerError"
                        | "StartError"
                )
            })
            || terminated.iter().any(|state| {
                state
                    .rsplit_once(':')
                    .and_then(|(_, code)| code.parse::<i32>().ok())
                    .is_some_and(|code| code != 0)
            });
        all_running &= phase == "Running" && waiting.is_empty() && deleting.is_empty();
        let details = [
            reason.to_string(),
            waiting.join(", "),
            terminated.join(", "),
        ]
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" / ");
        names.push(name.to_string());
        summary.push(
            format!("{}: {} {}", name, phase, details)
                .trim()
                .to_string(),
        );
    }
    let running = !summary.is_empty() && all_running;
    let summary = summary.join("\n");
    Ok(if failed {
        PodStartup::Failed(summary)
    } else if running {
        PodStartup::Running(names)
    } else {
        PodStartup::Waiting(summary)
    })
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
    fn synchronous_wait_returns_running_failure_timeout_or_query_error() {
        let mut states = [
            PodStartup::Waiting("Pending".into()),
            PodStartup::Running(vec!["lesson1-abc".into()]),
        ]
        .into_iter();
        let names = wait_for_running("lesson1", Duration::from_secs(1), Duration::ZERO, || {
            Ok(states.next().expect("must stop after Running"))
        })
        .unwrap();
        assert_eq!(names, vec!["lesson1-abc"]);
        assert!(states.next().is_none());

        let failed = wait_for_running("lesson1", Duration::from_secs(1), Duration::ZERO, || {
            Ok(PodStartup::Failed("CrashLoopBackOff".into()))
        });
        assert!(
            matches!(failed, Err(ThumedError::PodStartupFailed { release, detail })
            if release == "lesson1" && detail == "CrashLoopBackOff")
        );
        let timeout = wait_for_running("lesson1", Duration::ZERO, Duration::ZERO, || {
            Ok(PodStartup::Waiting("Pending".into()))
        });
        assert!(
            matches!(timeout, Err(ThumedError::PodStartupTimeout { seconds: 0, detail }) if detail == "Pending")
        );
        let error = wait_for_running("lesson1", Duration::ZERO, Duration::ZERO, || {
            Err(ThumedError::MissingTool("kubectl".into()))
        });
        assert!(matches!(error, Err(ThumedError::MissingTool(_))));
    }

    #[test]
    fn startup_checks_phase_and_container_errors() {
        fn row(phase: &str, reason: &str, waiting: &str, terminated: &str) -> String {
            format!(
                "lesson1-abc\t{}\t{}\t\t{}\t{}\n",
                phase, reason, waiting, terminated
            )
        }
        assert_eq!(
            parse_pod_startup("").unwrap(),
            PodStartup::Waiting(String::new())
        );
        let running = row("Running", "", ",,", "Completed:0,:,");
        assert_eq!(
            parse_pod_startup(&running).unwrap(),
            PodStartup::Running(vec!["lesson1-abc".to_string()])
        );
        assert_eq!(
            parse_pod_startup(&running.replace('\n', "\r\n")).unwrap(),
            parse_pod_startup(&running).unwrap()
        );
        let pending = row("Pending", "", "ContainerCreating,", ":,");
        for output in [&pending, &format!("{}{}", running, pending)] {
            assert!(matches!(
                parse_pod_startup(output),
                Ok(PodStartup::Waiting(_))
            ));
        }
        for (phase, reason, waiting, terminated, expected) in [
            ("Failed", "Evicted", "", "", "Evicted"),
            ("Running", "", "CrashLoopBackOff,", ":,", "CrashLoopBackOff"),
            ("Pending", "", "ImagePullBackOff,", "", "ImagePullBackOff"),
            ("Pending", "", "ErrImagePull,", "", "ErrImagePull"),
            (
                "Pending",
                "",
                "CreateContainerConfigError,",
                "",
                "CreateContainerConfigError",
            ),
            ("Running", "", "", "OOMKilled:137,", "OOMKilled"),
            ("Pending", "", "", "Error:1,", "Error:1"),
            ("Succeeded", "", "", "Completed:0,", "Succeeded"),
        ] {
            let output = format!("{}{}", running, row(phase, reason, waiting, terminated));
            assert!(
                matches!(parse_pod_startup(&output), Ok(PodStartup::Failed(detail)) if detail.contains(expected))
            );
        }
        assert!(matches!(
            parse_pod_startup("lesson1-abc\tRunning\t\t2026-01-01T00:00:00Z\t,\t:,\n"),
            Ok(PodStartup::Waiting(_))
        ));
        assert!(parse_pod_startup("malformed output").is_err());
    }

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
