use crate::constants;
use crate::error::{Invalid, Result, ThumedError};
use crate::tools::{self, SetupStep};
use crate::utils::run_cmd;
use std::ffi::OsStr;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn config_dir() -> PathBuf {
    dirs::config_local_dir()
        .or_else(dirs::home_dir)
        .unwrap_or_else(|| PathBuf::from("."))
        .join(constants::APP_DIR)
}

/// Only the lecture TUI deliberately displays the account/password. Never
/// derive Debug or include kubeconfig private keys/tokens in a report.
pub struct UserInfo {
    pub user: String,
    pub password: String,
    pub context: String,
    pub server: String,
    pub namespace: String,
    pub customized: bool,
    config_user: String,
}

impl UserInfo {
    pub fn load() -> Result<Self> {
        Self::from_kubeconfig()?.load_override(&config_dir())
    }

    pub fn from_kubeconfig() -> Result<Self> {
        // Flatten validates referenced cert/key files. Only safe metadata is
        // selected: no --raw output, private key material or authentication token.
        let output = run_cmd(
            "kubectl",
            &[
                "config",
                "view",
                "--minify",
                "--flatten",
                "-o",
                concat!("jsonpath={.current-context}{\"\\n\"}{.contexts[0].context.user}",
                "{\"\\n\"}{.clusters[0].cluster.server}{\"\\n\"}{.contexts[0].context.namespace}"),
            ],
        )?;
        Self::from_config_view(&output)
    }

    fn from_config_view(output: &str) -> Result<Self> {
        let fields: Vec<_> = output.split('\n').collect();
        if fields.len() != 4
            || fields[0].trim().is_empty()
            || fields[2].trim().is_empty()
            || fields
                .iter()
                .any(|field| field.chars().any(char::is_control))
        {
            return Err(Invalid::KubeUser.into());
        }
        let mut info = Self::from_username(fields[1])?;
        info.context = fields[0].trim().to_string();
        info.server = fields[2].trim().to_string();
        if !fields[3].trim().is_empty() {
            info.namespace = fields[3].trim().to_string();
        }
        Ok(info)
    }

    pub fn from_username(user: &str) -> Result<Self> {
        let user = user.trim();
        if user.is_empty()
            || matches!(user, "." | "..")
            || !user
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || "_.-@".contains(c))
        {
            return Err(Invalid::KubeUser.into());
        }
        Ok(Self {
            user: user.to_string(),
            password: constants::LECTURE_PASSWORD.to_string(),
            context: String::new(),
            server: String::new(),
            namespace: "default".to_string(),
            customized: false,
            config_user: user.to_string(),
        })
    }

    fn scope(&self) -> String {
        format!("{}\t{}\t{}", self.context, self.server, self.config_user)
    }

    fn validate_credentials(&self) -> Result<()> {
        Self::from_username(&self.user).map_err(|_| Invalid::SavedCredentials)?;
        if self.password.trim().is_empty() || self.password.chars().any(char::is_control) {
            return Err(Invalid::SavedCredentials.into());
        }
        Ok(())
    }

    fn load_override(mut self, dir: &Path) -> Result<Self> {
        let text = match std::fs::read_to_string(dir.join("lecture-user.config")) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(self),
            Err(error) => return Err(error.into()),
        };
        let fields: Vec<_> = text.lines().collect();
        // A saved account must never leak into a different context/cluster/user.
        if fields.first().copied() != Some(self.scope().as_str()) {
            return Ok(self);
        }
        if fields.len() != 3 {
            return Err(Invalid::SavedCredentials.into());
        }
        self.user = fields[1].trim().to_string();
        self.password = fields[2].to_string();
        self.validate_credentials()?;
        self.customized = true;
        Ok(self)
    }

    pub fn save(&self, dir: &Path) -> Result<()> {
        self.validate_credentials()?;
        if self.context.is_empty() || self.server.is_empty() {
            return Err(Invalid::KubeUser.into());
        }
        std::fs::create_dir_all(dir)?;
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let temporary = dir.join(format!(
            ".lecture-user-{}-{}.tmp",
            std::process::id(),
            nonce
        ));
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(&temporary)?;
        let result = (|| -> Result<()> {
            writeln!(
                file,
                "{}\n{}\n{}",
                self.scope(),
                self.user.trim(),
                self.password
            )?;
            file.sync_all()?;
            drop(file);
            // Replace only after writing succeeds, retaining the old file on error.
            std::fs::rename(&temporary, dir.join("lecture-user.config"))?;
            Ok(())
        })();
        if result.is_err() {
            let _ = std::fs::remove_file(&temporary);
        }
        result
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CheckItem {
    Kubeconfig,
    Kubectl,
    Credentials,
    Helm,
    Cluster,
    Repo,
}

impl CheckItem {
    pub const ALL: [Self; 6] = [
        Self::Kubeconfig,
        Self::Kubectl,
        Self::Credentials,
        Self::Helm,
        Self::Cluster,
        Self::Repo,
    ];
}

pub enum CheckDetail {
    Text(String),
    Account(UserInfo),
}

pub type CheckResult = (CheckItem, Result<CheckDetail>);

pub fn is_initialized(config_dir: &Path) -> bool {
    matches!(std::fs::read(config_dir.join("initialized")), Ok(contents) if contents == b"2\n")
}

pub fn record_initialization(config_dir: &Path, report: &[CheckResult]) -> Result<bool> {
    if !CheckItem::ALL.iter().all(|item| {
        report
            .iter()
            .any(|(key, result)| key == item && result.is_ok())
    }) {
        return Ok(false);
    }
    std::fs::create_dir_all(config_dir)?;
    std::fs::write(config_dir.join("initialized"), b"2\n")?;
    Ok(true)
}

/// Synchronous setup with a repaint before each operation/substep. Failures
/// remain in the report; dependent checks are explicitly marked as skipped.
pub fn check_env(
    mut progress: impl FnMut(&[CheckResult], CheckItem, SetupStep) -> Result<()>,
) -> Result<Vec<CheckResult>> {
    let mut report = Vec::new();
    progress(&report, CheckItem::Kubeconfig, SetupStep::Check)?;
    let config = check_kubeconfig(&kubeconfig_path());
    let config_exists = config.is_ok();
    report.push((CheckItem::Kubeconfig, config.map(CheckDetail::Text)));
    if !config_exists {
        for item in CheckItem::ALL.into_iter().skip(1) {
            report.push((item, Err(Invalid::SetupPrerequisite.into())));
        }
        return Ok(report);
    }

    let kubectl = tools::ensure("kubectl", &mut |step| {
        progress(&report, CheckItem::Kubectl, step)
    });
    let kubectl_ready = kubectl.is_ok();
    report.push((CheckItem::Kubectl, kubectl.map(CheckDetail::Text)));

    progress(&report, CheckItem::Credentials, SetupStep::Check)?;
    let credentials = if kubectl_ready {
        UserInfo::load()
    } else {
        Err(Invalid::SetupPrerequisite.into())
    };
    let credentials_ready = credentials.is_ok();
    report.push((
        CheckItem::Credentials,
        credentials.map(CheckDetail::Account),
    ));

    let helm = tools::ensure("helm", &mut |step| progress(&report, CheckItem::Helm, step));
    let helm_ready = helm.is_ok();
    report.push((CheckItem::Helm, helm.map(CheckDetail::Text)));

    progress(&report, CheckItem::Cluster, SetupStep::Check)?;
    let cluster = if credentials_ready {
        run_cmd(
            "kubectl",
            &["get", "pods", "--request-timeout=10s", "-o", "name"],
        )
        .map(|out| {
            format!(
                "{} Pod\n{}",
                out.lines().filter(|line| !line.is_empty()).count(),
                out.trim()
            )
        })
    } else {
        Err(Invalid::SetupPrerequisite.into())
    };
    report.push((CheckItem::Cluster, cluster.map(CheckDetail::Text)));

    let repo = if helm_ready {
        init_helm_repo(&mut |step| progress(&report, CheckItem::Repo, step))
    } else {
        Err(Invalid::SetupPrerequisite.into())
    };
    report.push((CheckItem::Repo, repo.map(CheckDetail::Text)));
    Ok(report)
}

/// kubectl performs the actual config merging; this path is for the prompt.
pub fn kubeconfig_path() -> PathBuf {
    kubeconfig_path_from(
        std::env::var_os("KUBECONFIG").as_deref(),
        &dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")),
    )
}

fn kubeconfig_path_from(value: Option<&OsStr>, home: &Path) -> PathBuf {
    if let Some(value) = value {
        let paths: Vec<_> = std::env::split_paths(value)
            .filter(|path| !path.as_os_str().is_empty())
            .collect();
        if let Some(path) = paths
            .iter()
            .find(|path| path.is_file())
            .or_else(|| paths.first())
        {
            return path.clone();
        }
    }
    home.join(".kube").join("config")
}

pub fn check_kubeconfig(path: &Path) -> Result<String> {
    if !path.is_file() {
        return Err(ThumedError::MissingKubeconfig(path.display().to_string()));
    }
    std::fs::File::open(path)?;
    Ok(path.display().to_string())
}

fn init_helm_repo(progress: &mut dyn FnMut(SetupStep) -> Result<()>) -> Result<String> {
    progress(SetupStep::Configure)?;
    run_cmd(
        "helm",
        &[
            "repo",
            "add",
            constants::HELM_REPO_NAME,
            constants::HELM_REPO_URL,
            "--force-update",
        ],
    )?;
    progress(SetupStep::Update)?;
    run_cmd("helm", &["repo", "update", constants::HELM_REPO_NAME])?;
    Ok(format!(
        "{}\n{}\n{}",
        constants::HELM_REPO_NAME,
        constants::HELM_REPO_URL,
        constants::HELM_CHART
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unique_temp_dir(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("thumed_helper_test_{}_{}", name, nanos))
    }

    #[test]
    fn initialization_is_remembered_only_after_success() {
        let dir = unique_temp_dir("initialization");
        assert!(!is_initialized(&dir));
        assert!(!record_initialization(&dir, &[]).unwrap());
        let failed = vec![(CheckItem::Credentials, Err(Invalid::KubeUser.into()))];
        assert!(!record_initialization(&dir, &failed).unwrap());
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("initialized"), b"1\n").unwrap();
        assert!(!is_initialized(&dir));
        let passed = CheckItem::ALL
            .into_iter()
            .map(|item| (item, Ok(CheckDetail::Text(String::new()))))
            .collect::<Vec<_>>();
        assert!(record_initialization(&dir, &passed).unwrap());
        assert!(is_initialized(&dir));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn lecture_credentials_use_config_username_and_default_password() {
        let info = UserInfo::from_username("student01\n").unwrap();
        assert_eq!(info.user, "student01");
        assert_eq!(info.password, "Test1234");
        for invalid in ["", " \n", "../user", "student\nother", ".."] {
            assert!(matches!(
                UserInfo::from_username(invalid),
                Err(ThumedError::Invalid(Invalid::KubeUser))
            ));
        }
    }

    #[test]
    fn saved_credentials_round_trip_and_are_scoped_to_context() {
        let dir = unique_temp_dir("credentials");
        let snapshot = "lesson\nstudent01\nhttps://fixture.invalid\nclass";
        let mut info = UserInfo::from_config_view(snapshot).unwrap();
        info.user = "edited".into();
        info.password = " secret value ".into();
        info.save(&dir).unwrap();
        let loaded = UserInfo::from_config_view(snapshot)
            .unwrap()
            .load_override(&dir)
            .unwrap();
        assert_eq!(loaded.user, "edited");
        assert_eq!(loaded.password, " secret value ");
        assert!(loaded.customized);
        let different = UserInfo::from_config_view("other\nstudent02\nhttps://fixture.invalid\n")
            .unwrap()
            .load_override(&dir)
            .unwrap();
        assert_eq!(different.user, "student02");
        assert_eq!(different.password, "Test1234");
        info.password = "updated".into();
        info.save(&dir).unwrap();
        assert_eq!(
            UserInfo::from_config_view(snapshot)
                .unwrap()
                .load_override(&dir)
                .unwrap()
                .password,
            "updated"
        );
        let before = std::fs::read(dir.join("lecture-user.config")).unwrap();
        info.password = "bad\nvalue".into();
        assert!(info.save(&dir).is_err());
        assert_eq!(
            std::fs::read(dir.join("lecture-user.config")).unwrap(),
            before
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(dir.join("lecture-user.config"))
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o777,
                0o600
            );
        }
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 1);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn kubeconfig_prompt_respects_home_and_platform_path_lists() {
        let dir = unique_temp_dir("kubeconfig");
        let default = dir.join(".kube").join("config");
        assert_eq!(kubeconfig_path_from(None, &dir), default);
        assert!(matches!(
            check_kubeconfig(&default),
            Err(ThumedError::MissingKubeconfig(_))
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let custom = dir.join("custom.config");
        std::fs::write(&custom, "fixture").unwrap();
        let paths = std::env::join_paths([dir.join("missing"), custom.clone()]).unwrap();
        assert_eq!(kubeconfig_path_from(Some(&paths), &dir), custom);
        assert!(check_kubeconfig(&custom).is_ok());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
