use crate::{constants, error::ThumedError, utils};
use std::env;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
#[cfg(unix)]
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

pub struct DirManager {
    pub config_dir: PathBuf,
    pub bin_dir: PathBuf,
}

pub struct UserInfo {
    pub user: String,
    pub password: String,
}

impl DirManager {
    pub fn new(app_name: &str) -> Self {
        let home = dirs::home_dir().unwrap();
        let config_dir = dirs::config_local_dir().unwrap_or_else(|| home.join(".config"));
        let config_dir = config_dir.join(app_name);
        let bin_dir = dirs::data_local_dir().unwrap_or_else(|| {
            if cfg!(target_os = "windows") {
                home.join("AppData/Local")
            } else {
                home.join(".local/bin")
            }
        });
        let bin_dir = bin_dir.join(app_name);
        DirManager {
            config_dir,
            bin_dir,
        }
    }
}

impl UserInfo {
    pub fn new(user: String, password: String) -> Self {
        UserInfo { user, password }
    }

    fn get_config_path(dirman: &DirManager) -> crate::error::Result<PathBuf> {
        let config_dir = &dirman.config_dir;
        if !config_dir.exists() {
            std::fs::create_dir_all(config_dir)?;
        }
        Ok(config_dir.join("user.config"))
    }

    pub fn load(dirman: &DirManager) -> crate::error::Result<Self> {
        let config_path = Self::get_config_path(dirman)?;
        if !config_path.exists() {
            return Err(ThumedError::Config(
                "No user configuration found. Select Update user information.".to_string(),
            ));
        }

        let mut contents = String::new();
        File::open(config_path)?.read_to_string(&mut contents)?;
        let mut lines = contents.lines();
        let user = lines.next().unwrap_or_default().trim();
        let password = lines.next().unwrap_or_default().trim();
        if user.is_empty() || password.is_empty() {
            return Err(ThumedError::Config(
                "Config file format is invalid (username and password are required).".to_string(),
            ));
        }
        Ok(UserInfo::new(user.to_string(), password.to_string()))
    }

    pub fn save(&self, dirman: &DirManager) -> crate::error::Result<()> {
        if self.user.trim().is_empty() || self.password.trim().is_empty() {
            return Err(ThumedError::Config(
                "Username and password are required.".to_string(),
            ));
        }
        let config_path = Self::get_config_path(dirman)?;
        #[cfg(unix)]
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(&config_path)?;
        #[cfg(not(unix))]
        let mut file = File::create(&config_path)?;
        #[cfg(unix)]
        std::fs::set_permissions(&config_path, std::fs::Permissions::from_mode(0o600))?;
        writeln!(file, "{}", self.user)?;
        writeln!(file, "{}", self.password)?;
        Ok(())
    }
}
pub fn add_path(path: &Path) -> crate::error::Result<()> {
    let path_str = path.display().to_string();
    let paths = env::var("PATH")?;

    let mut path_vec: Vec<String> = env::split_paths(&paths)
        .map(|p| p.to_string_lossy().to_string())
        .collect();

    if !path_vec.contains(&path_str) {
        path_vec.insert(0, path_str);
    }

    let new_path = env::join_paths(path_vec)?;
    env::set_var("PATH", new_path);

    Ok(())
}

pub fn ensure_tools_available(_dirman: &DirManager) -> crate::error::Result<()> {
    utils::run_cmd("kubectl", &["version", "--client"])?;
    utils::run_cmd("helm", &["version"])?;
    Ok(())
}

fn init_helm() -> crate::error::Result<()> {
    let helm_list = utils::run_cmd("helm", &["repo", "list"])?;
    if !helm_list.contains(constants::HELM_REPO_NAME) {
        utils::run_cmd(
            "helm",
            &[
                "repo",
                "add",
                constants::HELM_REPO_NAME,
                constants::HELM_REPO_URL,
            ],
        )?;
    }
    utils::run_cmd("helm", &["repo", "update"])?;
    Ok(())
}

pub fn check_env(dirman: &DirManager) -> crate::error::Result<()> {
    UserInfo::load(dirman)?;
    ensure_tools_available(dirman)?;
    init_helm()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;
    use std::sync::Mutex;
    use std::time::{SystemTime, UNIX_EPOCH};

    static PATH_LOCK: Mutex<()> = Mutex::new(());

    fn unique_temp_dir(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("thumed_helper_test_{}_{}", name, nanos))
    }

    struct PathGuard {
        original: Option<OsString>,
    }

    impl PathGuard {
        fn new(value: &str) -> Self {
            let original = env::var_os("PATH");
            env::set_var("PATH", value);
            Self { original }
        }
    }

    impl Drop for PathGuard {
        fn drop(&mut self) {
            if let Some(original) = &self.original {
                env::set_var("PATH", original);
            } else {
                env::remove_var("PATH");
            }
        }
    }

    #[test]
    fn add_path_prepends_missing_path() {
        let _lock = PATH_LOCK.lock().unwrap();
        let _guard = PathGuard::new("/usr/bin:/bin");
        let new_path = Path::new("/tmp/thumed-bin");

        add_path(new_path).unwrap();

        let paths: Vec<PathBuf> = env::split_paths(&env::var_os("PATH").unwrap()).collect();
        assert_eq!(paths.first().unwrap(), new_path);
    }

    #[test]
    fn add_path_does_not_duplicate_existing_path() {
        let _lock = PATH_LOCK.lock().unwrap();
        let _guard = PathGuard::new("/tmp/thumed-bin:/usr/bin:/bin");
        let existing_path = Path::new("/tmp/thumed-bin");

        add_path(existing_path).unwrap();

        let paths: Vec<PathBuf> = env::split_paths(&env::var_os("PATH").unwrap()).collect();
        let matches = paths
            .iter()
            .filter(|path| path.as_path() == existing_path)
            .count();
        assert_eq!(matches, 1);
    }

    #[test]
    fn user_info_load_reads_existing_config() {
        let config_dir = unique_temp_dir("user_info_load");
        std::fs::create_dir_all(&config_dir).unwrap();
        std::fs::write(config_dir.join("user.config"), "alice\nsecret\n").unwrap();
        let dirman = DirManager {
            config_dir,
            bin_dir: unique_temp_dir("bin"),
        };

        let user_info = UserInfo::load(&dirman).unwrap();

        assert_eq!(user_info.user, "alice");
        assert_eq!(user_info.password, "secret");
    }

    #[test]
    fn user_info_load_rejects_invalid_config() {
        let config_dir = unique_temp_dir("invalid_user_info");
        std::fs::create_dir_all(&config_dir).unwrap();
        std::fs::write(config_dir.join("user.config"), "alice\n").unwrap();
        let dirman = DirManager {
            config_dir,
            bin_dir: unique_temp_dir("bin"),
        };

        let error = match UserInfo::load(&dirman) {
            Ok(_) => panic!("expected invalid config to fail"),
            Err(error) => error,
        };

        assert!(matches!(error, ThumedError::Config(_)));
        assert!(error.to_string().contains("Config file format is invalid"));
    }
}
