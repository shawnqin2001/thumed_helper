mod constants;
mod environment;
mod error;
mod pod_handler;
mod tui;
mod utils;

use crate::environment::DirManager;
use crate::error::Result;
use crate::pod_handler::PodHandler;

fn main() -> Result<()> {
    let dirman = DirManager::new("thumed_helper");
    let mut pod_handler = PodHandler::new();
    tui::run(&dirman, &mut pod_handler)
}
