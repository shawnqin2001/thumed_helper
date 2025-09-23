mod cli;
mod constants;
mod environment;
// mod host_handler;
mod interaction;
mod platform;
mod pod_handler;
mod utils;

use crate::environment::{add_path, DirManager};
use crate::pod_handler::PodHandler;

fn main() {
    let mut ph = PodHandler::new();
    let dirman = DirManager::new("thumed_helper");
    println!("Welcome to {}", constants::APP_NAME);
    println!("Current: {}", constants::APP_VERSION);

    let bin_dir = &dirman.bin_dir;
    if !bin_dir.exists() {
        println!(
            "Seems like this is your first time running the helper. \n Run initialization? (y/n)"
        );
        let mut input = String::new();
        std::io::stdin()
            .read_line(&mut input)
            .expect("Failed to read line");
        if input.trim().to_lowercase() == "y" {
            environment::check_env();
        }
    }
    let _ = add_path(bin_dir);
    loop {
        match interaction::get_user_action() {
            Ok(action) => match action {
                0 => break,
                1 => environment::check_env(),
                2 => {
                    if let Err(e) = ph.get_pod_list() {
                        println!("Error getting pod list: {}", e);
                        continue;
                    }
                    ph.display();
                }
                3 => {
                    let pod_config = pod_handler::PodConfig::new();
                    if let Err(e) = pod_config.save_config_yaml(&dirman) {
                        println!("Error saving pod configuration: {}", e);
                        continue;
                    }
                    if let Err(e) = pod_config.install_pod(&dirman) {
                        println!("Error installing pod: {}", e);
                    }
                }
                4 => {
                    if let Err(e) = ph.get_pod_list() {
                        println!("Error getting pod list: {}", e);
                        continue;
                    }
                    ph.display();
                    if let Err(e) = ph.login_pod() {
                        println!("Error logging into pod: {}", e);
                    }
                }
                5 => {
                    if let Err(e) = ph.get_pod_list() {
                        println!("Error getting pod list: {}", e);
                        continue;
                    }
                    ph.display();
                    if let Err(e) = ph.forward_pod() {
                        println!("Error logging into pod: {}", e)
                    }
                }
                6 => {
                    if let Err(e) = ph.get_pod_list() {
                        println!("Error getting pod list: {}", e);
                        continue;
                    }
                    ph.display();
                    if let Err(e) = ph.uninstall_pod() {
                        println!("Error uninstalling pod: {}", e);
                    }
                }
                7 => {
                    if let Err(e) = environment::UserInfo::update_user(&dirman) {
                        println!("Error updating user info: {}", e);
                    }
                }
                _ => println!("Invalid action"),
            },
            Err(e) => println!("Error: {}", e),
        }
    }
}
