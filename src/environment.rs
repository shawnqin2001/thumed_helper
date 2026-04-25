use crate::{constants, error::ThumedError, platform, utils};
use std::env;
use std::fs::{File, OpenOptions};
use std::io::{self, BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

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

    fn read_input(prompt: &str) -> crate::error::Result<String> {
        println!("{}", prompt);
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        Ok(input.trim().to_string())
    }

    fn get_credentials(
        dirman: &DirManager,
        show_current: bool,
    ) -> crate::error::Result<(String, String)> {
        let config_path = Self::get_config_path(dirman)?;

        if config_path.exists() {
            let mut file = File::open(&config_path)?;
            let mut contents = String::new();
            file.read_to_string(&mut contents)?;

            let lines: Vec<&str> = contents.lines().collect();
            if lines.len() < 2 {
                return Err(ThumedError::Config(
                    "Config file format is invalid (should contain username and password)"
                        .to_string(),
                ));
            }

            let user = lines[0].to_string();
            let password = lines[1].to_string();

            if show_current {
                println!("Current User: {}", user);
                println!("Current Password: {}", password);
            }

            return Ok((user, password));
        }

        if !show_current {
            println!("No user configuration found. Please enter credentials:");
        }
        let user = Self::read_input("Username: (Your full name)")?;
        let mut password = Self::read_input("Password: (Default: Test1234)")?;
        if password.is_empty() {
            password = "Test1234".to_string();
        }
        Ok((user, password))
    }

    pub fn update_user(dirman: &DirManager) -> crate::error::Result<Self> {
        let _ = Self::get_credentials(dirman, true);
        let (user, password) = Self::read_input_credentials()?;
        let user_info = UserInfo::new(user, password);
        user_info.save(dirman)?;
        Ok(user_info)
    }

    fn read_input_credentials() -> crate::error::Result<(String, String)> {
        let user = Self::read_input("Username: (Your Fullname)")?;
        let password = Self::read_input("Password: (Default: Test1234)")?;
        Ok((user, password))
    }

    pub fn load(dirman: &DirManager) -> crate::error::Result<Self> {
        let config_path = Self::get_config_path(dirman)?;

        if config_path.exists() {
            let (user, password) = Self::get_credentials(dirman, false)?;
            Ok(UserInfo::new(user, password))
        } else {
            println!("No user configuration found. Please enter credentials:");
            let (user, password) = Self::read_input_credentials()?;
            let user_info = UserInfo::new(user, password);
            user_info.save(dirman)?;
            Ok(user_info)
        }
    }

    fn save(&self, dirman: &DirManager) -> crate::error::Result<()> {
        let config_path = Self::get_config_path(dirman)?;
        let mut file = File::create(&config_path)?;
        writeln!(file, "{}", self.user)?;
        writeln!(file, "{}", self.password)?;
        println!("User credentials saved successfully.");
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

pub fn add_user_path(path: &Path) -> crate::error::Result<()> {
    let bin_path = path.canonicalize()?.to_string_lossy().to_string();
    if cfg!(windows) {
        let current_path = Command::new("powershell")
            .args([
                "-Command",
                "[Environment]::GetEnvironmentVariable('PATH', 'User')",
            ])
            .output()?;
        let current_path_str = String::from_utf8_lossy(&current_path.stdout);
        if !current_path_str.split(";").any(|p| p == bin_path) {
            Command::new("powershell")
                .args([
                    "-Command",
                    &format!("[Environment]::SetEnvironmentVariable('PATH', '{};'+[Environment]::GetEnvironmentVariable('PATH', 'User'), 'User')", bin_path)
                ])
                .status()?;
            println!("Added {} to user PATH (Windows)", bin_path);
        } else {
            println!("{} already in user path (Windows)", bin_path);
        }
    } else {
        // Unix system path adding
        let home = dirs::home_dir().expect("Cannot find home directory");
        let shell = std::env::var("SHELL").unwrap_or_default();
        let rc_file = if shell.contains("zsh") {
            home.join(".zshrc")
        } else {
            home.join(".bashrc")
        };

        // Read current lines to check duplicates
        let mut already_exists = false;
        if rc_file.exists() {
            let file = std::fs::File::open(&rc_file)?;
            let reader = BufReader::new(file);
            for line in reader.lines() {
                let line = line?;
                if line.contains(&bin_path) {
                    already_exists = true;
                    break;
                }
            }
        }
        if !already_exists {
            let mut file = OpenOptions::new()
                .append(true)
                .create(true)
                .open(&rc_file)?;
            writeln!(
                file,
                "\n# Added by THU-Med Helper\nexport PATH=\"{}:$PATH\"",
                bin_path
            )?;
            println!("Added {} to PATH in {:?}", bin_path, rc_file);
        } else {
            println!("{} already in PATH in {:?}", bin_path, rc_file);
        }
    }
    Ok(())
}

pub fn ensure_tools_available(dirman: &DirManager) -> crate::error::Result<()> {
    let kubectl_ok = utils::run_cmd("kubectl", &["version", "--client"]).is_ok();
    let helm_ok = utils::run_cmd("helm", &["version"]).is_ok();
    if kubectl_ok && helm_ok {
        println!("Tools are ok");
        return Ok(());
    }
    let bin_dir = &dirman.bin_dir;
    if !bin_dir.exists() {
        println!("Creating bin directory...");
        std::fs::create_dir_all(bin_dir)?;
    }
    add_path(bin_dir)?;
    add_user_path(bin_dir)?;

    let kubectl_path = platform::get_bin_path(bin_dir, "kubectl");
    let helm_path = platform::get_bin_path(bin_dir, "helm");

    let kubectl_exists = kubectl_path.exists();
    let helm_exists = helm_path.exists();

    if !kubectl_exists || !helm_exists {
        println!("Some required tools are missing. Attempting to download:");

        if !kubectl_exists {
            match utils::download_kubectl(bin_dir) {
                Ok(_) => println!("Successfully downloaded kubectl"),
                Err(e) => println!(
                    "Failed to download kubectl: {}. Please download it manually.",
                    e
                ),
            }
        }

        if !helm_exists {
            match utils::download_helm(bin_dir) {
                Ok(_) => println!("Successfully downloaded helm"),
                Err(e) => println!(
                    "Failed to download helm: {}. Please download it manually.",
                    e
                ),
            }
        }
    } else {
        println!("All required tools found in bin directory.");
    }

    let kubectl_exists = kubectl_path.exists();
    let helm_exists = helm_path.exists();

    if kubectl_exists {
        match utils::run_cmd("kubectl", &["version", "--client"]) {
            Ok(_) => println!("kubectl is working correctly"),
            Err(e) => println!("Warning: kubectl may not be working: {}", e),
        }
    } else {
        println!("kubectl is still missing. Please download it manually from:");
        println!("kubectl: kubernetes.io/docs/tasks/tools/");
    }

    if helm_exists {
        match utils::run_cmd("helm", &["version"]) {
            Ok(_) => println!("helm is working correctly"),
            Err(e) => println!("Warning: helm may not be working: {}", e),
        }
    } else {
        println!("helm is still missing. Please download it manually from:");
        println!("helm: https://github.com/helm/helm/releases");
    }
    Ok(())
}

fn init_helm() -> crate::error::Result<()> {
    let helm_list = utils::run_cmd("helm", &["repo", "list"]).unwrap_or_default();

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
        println!("Added {} repository", constants::HELM_REPO_NAME);
    } else {
        println!("{} repository already exists", constants::HELM_REPO_NAME);
    }
    let helm_update = utils::run_cmd("helm", &["repo", "update"])?;
    println!("{}", helm_update);
    Ok(())
}

pub fn check_env() {
    println!("Checking environment...");
    let dir_manager = DirManager::new("thumed_helper");
    match UserInfo::load(&dir_manager) {
        Ok(user_info) => {
            println!("User: {}", user_info.user);
            println!("Password: {}", user_info.password);
        }
        Err(e) => {
            println! {"Failed to load user information:\n {}", e};
            return;
        }
    }
    match ensure_tools_available(&dir_manager) {
        Ok(_) => println!("Tool directory setup complete"),
        Err(e) => println!("{}", e),
    }
    match init_helm() {
        Ok(_) => println!("Helm initialized successfully"),
        Err(e) => println!("{}", e),
    }
    println!("Environment check completed!");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;
    use std::time::{SystemTime, UNIX_EPOCH};

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
        let _guard = PathGuard::new("/usr/bin:/bin");
        let new_path = Path::new("/tmp/thumed-bin");

        add_path(new_path).unwrap();

        let paths: Vec<PathBuf> = env::split_paths(&env::var_os("PATH").unwrap()).collect();
        assert_eq!(paths.first().unwrap(), new_path);
    }

    #[test]
    fn add_path_does_not_duplicate_existing_path() {
        let _guard = PathGuard::new("/tmp/thumed-bin:/usr/bin:/bin");
        let existing_path = Path::new("/tmp/thumed-bin");

        add_path(existing_path).unwrap();

        let paths: Vec<PathBuf> = env::split_paths(&env::var_os("PATH").unwrap()).collect();
        let matches = paths.iter().filter(|path| path.as_path() == existing_path).count();
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
