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

