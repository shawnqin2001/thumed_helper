mod cli;
mod constants;
mod environment;
mod error;
mod interaction;
mod platform;
mod pod_handler;
mod utils;

use crate::cli::Commands;
use crate::environment::{DirManager, UserInfo};
use crate::error::{Result, ThumedError};
use crate::pod_handler::{PodConfig, PodHandler};
use clap::Parser;

fn main() -> Result<()> {
    let cli = cli::Cli::parse();

    let dirman = DirManager::new("thumed_helper");
    let mut ph = PodHandler::new();

    match cli.command {
        Some(cmd) => execute_command(cmd, &dirman, &mut ph)?,
        None => run_interactive_mode(&dirman, &mut ph)?,
    }

    Ok(())
}

fn execute_command(cmd: Commands, dirman: &DirManager, ph: &mut PodHandler) -> Result<()> {
    match cmd {
        Commands::CheckEnv => {
            environment::check_env();
        }
        Commands::ListPods => {
            ph.get_pod_list()?;
            ph.display();
        }
        Commands::InstallPod { name, cpu, memory } => {
            let pod_config = if let Some(n) = name {
                PodConfig::from_args(n, cpu, memory)
            } else {
                PodConfig::new()
            };
            pod_config.save_config_yaml(dirman)?;
            pod_config.install_pod(dirman)?;
        }
        Commands::LoginPod { name } => {
            ph.get_pod_list()?;
            ph.display();
            if let Some(n) = name {
                ph.login_pod_by_name(&n)?;
            } else {
                ph.login_pod()?;
            }
        }
        Commands::ForwardPod { name } => {
            ph.get_pod_list()?;
            ph.display();
            if let Some(n) = name {
                ph.forward_pod_by_name(&n)?;
            } else {
                ph.forward_pod()?;
            }
        }
        Commands::UninstallPod { name } => {
            ph.get_pod_list()?;
            ph.display();
            if let Some(n) = name {
                ph.uninstall_pod_by_name(&n)?;
            } else {
                ph.uninstall_pod()?;
            }
        }
        Commands::UpdateUser => {
            UserInfo::update_user(dirman)?;
        }
    }
    Ok(())
}

fn run_interactive_mode(dirman: &DirManager, ph: &mut PodHandler) -> Result<()> {
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
            .map_err(ThumedError::Io)?;
        if input.trim().to_lowercase() == "y" {
            environment::check_env();
        }
    }
    let _ = environment::add_path(bin_dir);

    loop {
        match interaction::get_user_action() {
            Ok(action) => match action {
                0 => break,
                1 => environment::check_env(),
                2 => {
                    ph.get_pod_list()?;
                    ph.display();
                }
                3 => {
                    let pod_config = PodConfig::new();
                    pod_config.save_config_yaml(dirman)?;
                    pod_config.install_pod(dirman)?;
                }
                4 => {
                    ph.get_pod_list()?;
                    ph.display();
                    ph.login_pod()?;
                }
                5 => {
                    ph.get_pod_list()?;
                    ph.display();
                    ph.forward_pod()?;
                }
                6 => {
                    ph.get_pod_list()?;
                    ph.display();
                    ph.uninstall_pod()?;
                }
                7 => {
                    UserInfo::update_user(dirman)?;
                }
                _ => println!("Invalid action"),
            },
            Err(e) => println!("Error: {}", e),
        }
    }
    Ok(())
}