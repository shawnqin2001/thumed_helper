mod constants;
mod environment;
mod error;
mod i18n;
mod pod_handler;
mod tools;
mod tui;
mod utils;

use crate::i18n::{error_text, Lang};
use crate::pod_handler::PodHandler;

fn main() {
    let config_dir = environment::config_dir();
    let mut pod_handler = PodHandler::new();
    if let Err(error) = tui::run(&config_dir, &mut pod_handler) {
        // TUI is already torn down here, so plain stderr is the right channel.
        eprintln!("{}", error_text(&error, Lang::Zh));
        std::process::exit(1);
    }
}
