use clap::{Parser, Subcommand};

/// THU Med Login Helper CLI
#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize or check environment and tools
    CheckEnv,

    /// List pods and website addresses
    ListPods,

    /// Install a new pod
    InstallPod {
        /// Pod name (lowercase letters and numbers only)
        #[arg(short, long)]
        name: Option<String>,

        /// CPU cores (default: 16)
        #[arg(short, long)]
        cpu: Option<u8>,

        /// Memory in GB (default: 50)
        #[arg(short, long)]
        memory: Option<u8>,
    },

    /// Login to a pod in the terminal
    LoginPod {
        /// Pod name to login to
        #[arg(short, long)]
        name: Option<String>,
    },

    /// Forward port to access web service (RStudio)
    ForwardPod {
        /// Pod name to forward
        #[arg(short, long)]
        name: Option<String>,
    },

    /// Uninstall a pod
    UninstallPod {
        /// Pod name to uninstall
        #[arg(short, long)]
        name: Option<String>,
    },

    /// Update user information
    UpdateUser,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn parses_install_pod_with_all_options() {
        let cli = Cli::parse_from([
            "thumed_helper",
            "install-pod",
            "--name",
            "pod01",
            "--cpu",
            "8",
            "--memory",
            "32",
        ]);

        match cli.command {
            Some(Commands::InstallPod { name, cpu, memory }) => {
                assert_eq!(name, Some("pod01".to_string()));
                assert_eq!(cpu, Some(8));
                assert_eq!(memory, Some(32));
            }
            _ => panic!("expected install-pod command"),
        }
    }

    #[test]
    fn parses_login_pod_name() {
        let cli = Cli::parse_from(["thumed_helper", "login-pod", "--name", "pod01"]);

        match cli.command {
            Some(Commands::LoginPod { name }) => {
                assert_eq!(name, Some("pod01".to_string()));
            }
            _ => panic!("expected login-pod command"),
        }
    }

    #[test]
    fn parses_no_command_as_none() {
        let cli = Cli::parse_from(["thumed_helper"]);

        assert!(cli.command.is_none());
    }
}
