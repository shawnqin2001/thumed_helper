mod cli;
mod constants;
mod environment;
// mod host_handler;
mod interaction;
mod platform;
mod pod_handler;
mod utils;
use clap::Parser;
use std::env;
use std::process;

use crate::pod_handler::PodConfig;

fn main() {
    if let Ok(exe_path) = env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            if let Err(e) = env::set_current_dir(exe_dir) {
                eprintln!("Failed to change directory: {}", e);
            }
        }
    }
    // check the env if first run

    let cli = cli::Cli::parse();

    // If no command is specified or interactive mode is requested, run interactive mode
    if cli.interactive || cli.command.is_none() {
        run_interactive_mode();
        return;
    }

    // Handle command-line mode
    match cli.command.unwrap() {
        cli::Commands::CheckEnv => environment::check_env(),
        cli::Commands::ListPods => {
            let mut ph = pod_handler::PodHandler::new();
            match ph.get_pod_list() {
                Ok(_) => ph.display(),
                Err(e) => {
                    eprintln!("Error getting pod list: {}", e);
                    process::exit(1);
                }
            }
        }

        cli::Commands::InstallPod { name, cpu, memory } => {
            let pod_config = match name {
                Some(pod_name) => pod_handler::PodConfig::new_with_params(pod_name, cpu, memory),
                None => pod_handler::PodConfig::new(),
            };

            if let Err(e) = pod_config.save_config_yaml() {
                eprintln!("Error saving pod configuration: {}", e);
                process::exit(1);
            }

            if let Err(e) = pod_config.install_pod() {
                eprintln!("Error installing pod: {}", e);
                process::exit(1);
            }
        }

        cli::Commands::LoginPod { name } => {
            let mut ph = pod_handler::PodHandler::new();
            if let Err(e) = ph.get_pod_list() {
                eprintln!("Error getting pod list: {}", e);
                process::exit(1);
            }

            match name {
                Some(pod_name) => {
                    if let Err(e) = ph.login_pod_by_name(&pod_name) {
                        eprintln!("Error logging into pod: {}", e);
                        process::exit(1);
                    }
                }
                None => {
                    ph.display();
                    if let Err(e) = ph.login_pod() {
                        eprintln!("Error logging into pod: {}", e);
                        process::exit(1);
                    }
                }
            }
        }

        cli::Commands::UninstallPod { name } => {
            let mut ph = pod_handler::PodHandler::new();
            if let Err(e) = ph.get_pod_list() {
                eprintln!("Error getting pod list: {}", e);
                process::exit(1);
            }

            match name {
                Some(pod_name) => {
                    if let Err(e) = ph.uninstall_pod_by_name(&pod_name) {
                        eprintln!("Error uninstalling pod: {}", e);
                        process::exit(1);
                    }
                }
                None => {
                    ph.display();
                    if let Err(e) = ph.uninstall_pod() {
                        eprintln!("Error uninstalling pod: {}", e);
                        process::exit(1);
                    }
                }
            }
        }

        cli::Commands::UpdateUser => {
            if let Err(e) = environment::UserInfo::update_user() {
                eprintln!("Error updating user info: {}", e);
                process::exit(1);
            }
        }
    }
}

fn run_interactive_mode() {
    let mut ph = pod_handler::PodHandler::new();
    println!("Welcome to {}", constants::APP_NAME);
    println!("Current: {}", constants::APP_VERSION);

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
                    if let Err(e) = pod_config.save_config_yaml() {
                        println!("Error saving pod configuration: {}", e);
                        continue;
                    }
                    if let Err(e) = pod_config.install_pod() {
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
                    if let Err(e) = environment::UserInfo::update_user() {
                        println!("Error updating user info: {}", e);
                    }
                }
                _ => println!("Invalid action"),
            },
            Err(e) => println!("Error: {}", e),
        }
    }
}
