use crate::{constants, platform, utils};
use dirs;
use std::env;
use std::error::Error;
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::PathBuf;

pub struct PathManager {
    pub config_path: PathBuf,
    pub bin_path: PathBuf,
}

impl PathManager {
    pub fn new(app_name: &str) {}
}

pub struct UserInfo {
    pub user: String,
    pub password: String,
}

impl UserInfo {
    pub fn new(user: String, password: String) -> Self {
        UserInfo { user, password }
    }

    // Get the path to the config file
    fn get_config_path() -> Result<PathBuf, Box<dyn Error>> {
        let config_dir = env::current_dir()?.join("config");
        if !config_dir.exists() {
            std::fs::create_dir_all(&config_dir)?;
        }
        Ok(config_dir.join("user.config"))
    }

    // Read user input with provided prompt
    fn read_input(prompt: &str) -> Result<String, Box<dyn Error>> {
        println!("{}", prompt);
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        Ok(input.trim().to_string())
    }

    // Read user credentials from file or prompt user
    fn get_credentials(show_current: bool) -> Result<(String, String), Box<dyn Error>> {
        let config_path = Self::get_config_path()?;

        if config_path.exists() {
            let mut file = File::open(&config_path)?;
            let mut contents = String::new();
            file.read_to_string(&mut contents)?;

            let lines: Vec<&str> = contents.lines().collect();
            if lines.len() < 2 {
                return Err(
                    "Config file format is invalid (should contain username and password)".into(),
                );
            }

            let user = lines[0].to_string();
            let password = lines[1].to_string();

            if show_current {
                println!("Current User: {}", user);
                // Note: Printing password is generally not recommended
                println!("Current Password: {}", password);
            }

            return Ok((user, password));
        }

        if !show_current {
            println!("No user configuration found. Please enter credentials:");
        }

        let user = Self::read_input("Username: (Your Fullname)")?;
        let password = Self::read_input("Password: (Default: Test1234)")?;

        Ok((user, password))
    }

    pub fn update_user() -> Result<Self, Box<dyn Error>> {
        // Show current values before updating
        let _ = Self::get_credentials(true);

        // Always prompt for new credentials when updating
        let (user, password) = Self::read_input_credentials()?;
        let user_info = UserInfo::new(user, password);
        user_info.save()?;

        Ok(user_info)
    }

    // Helper method to read username and password
    fn read_input_credentials() -> Result<(String, String), Box<dyn Error>> {
        let user = Self::read_input("Username: (Your Fullname)")?;
        let password = Self::read_input("Password: (Default: Test1234)")?;
        Ok((user, password))
    }

    pub fn load() -> Result<Self, Box<dyn Error>> {
        let config_path = Self::get_config_path()?;

        if config_path.exists() {
            let (user, password) = Self::get_credentials(false)?;
            Ok(UserInfo::new(user, password))
        } else {
            println!("No user configuration found. Please enter credentials:");
            let (user, password) = Self::read_input_credentials()?;
            let user_info = UserInfo::new(user, password);
            user_info.save()?;
            Ok(user_info)
        }
    }

    fn save(&self) -> Result<(), Box<dyn Error>> {
        let config_path = Self::get_config_path()?;
        let mut file = File::create(&config_path)?;
        writeln!(file, "{}", self.user)?;
        writeln!(file, "{}", self.password)?;
        println!("User credentials saved successfully.");
        Ok(())
    }
}
// Add Path to Environment Variable
pub fn add_path(path: &str) -> Result<(), Box<dyn Error>> {
    let path_str = path.to_string();
    let paths = env::var("PATH")?;
    let mut path_vec: Vec<String> = env::split_paths(&paths)
        .map(|p| p.to_string_lossy().to_string())
        .collect();
    if !path_vec.contains(&path_str) {
        path_vec.insert(0, path_str);
        let new_path = env::join_paths(path_vec)?;
        if platform::is_windows() {
            utils::run_cmd(
                "setx",
                &["path", new_path.to_string_lossy().to_string().as_str()],
            )?;
        } else {
            unsafe {
                env::set_var("PATH", new_path);
            }
        };
    };
    Ok(())
}
pub fn ensure_tools_available() -> Result<(), Box<dyn Error>> {
    let bin_dir = std::env::current_dir()?.join("bin");

    if !bin_dir.exists() {
        println!("Creating bin directory...");
        std::fs::create_dir_all(&bin_dir)?;
    }

    add_path(bin_dir.to_string_lossy().as_ref())?;

    let kubectl_path = platform::get_bin_path(&bin_dir, "kubectl");
    let helm_path = platform::get_bin_path(&bin_dir, "helm");

    let kubectl_exists = kubectl_path.exists();
    let helm_exists = helm_path.exists();

    if !kubectl_exists || !helm_exists {
        println!("Some required tools are missing. Will attempt to download them:");

        if !kubectl_exists {
            match utils::download_kubectl(&bin_dir) {
                Ok(_) => println!("Successfully downloaded kubectl"),
                Err(e) => println!(
                    "Failed to download kubectl: {}. Please download it manually.",
                    e
                ),
            }
        }

        if !helm_exists {
            match utils::download_helm(&bin_dir) {
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

    // Re-check after potential downloads
    let kubectl_exists = kubectl_path.exists();
    let helm_exists = helm_path.exists();

    if kubectl_exists {
        let bin_kubectl = kubectl_path.to_string_lossy().to_string();
        match utils::run_cmd(&bin_kubectl, &["version", "--client"]) {
            Ok(_) => println!("kubectl is working correctly"),
            Err(e) => println!("Warning: kubectl may not be working: {}", e),
        }
    } else {
        println!("kubectl is still missing. Please download it manually from:");
        println!("kubectl: kubernetes.io/docs/tasks/tools/");
    }

    if helm_exists {
        let bin_helm = helm_path.to_string_lossy().to_string();
        match utils::run_cmd(&bin_helm, &["version"]) {
            Ok(_) => println!("helm is working correctly"),
            Err(e) => println!("Warning: helm may not be working: {}", e),
        }
    } else {
        println!("helm is still missing. Please download it manually from:");
        println!("helm: https://github.com/helm/helm/releases");
    }

    Ok(())
}

fn init_helm() -> Result<(), Box<dyn Error>> {
    // Check if med-helm repo already exists
    let helm_list = utils::run_cmd("helm", &["repo", "list"])?;

    if !helm_list.contains(constants::HELM_REPO_NAME) {
        let _helm_init = utils::run_cmd(
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
    match UserInfo::load() {
        Ok(user_info) => {
            println!("User: {}", user_info.user);
            println!("Password: {}", user_info.password);
        }
        Err(e) => {
            println! {"Error loading user info:\n {}", e};
            return;
        }
    }
    match ensure_tools_available() {
        Ok(_) => println!("Tool directory setup complete"),
        Err(e) => println!("{}", e),
    }
    match init_helm() {
        Ok(_) => println!("Helm initialized successfully"),
        Err(e) => println!("{}", e),
    }
    println!("Environment check completed!");
}
