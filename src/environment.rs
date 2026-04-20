use crate::{constants, error::ThumedError, platform, utils};
use std::env;
use std::fs::File;
use std::io::{self, Read, Write};
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

pub fn ensure_tools_available(dirman: &DirManager) -> crate::error::Result<()> {
    let bin_dir = &dirman.bin_dir;
    add_path(bin_dir)?;
    if !bin_dir.exists() {
        println!("Creating bin directory...");
        std::fs::create_dir_all(bin_dir)?;
    }

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
