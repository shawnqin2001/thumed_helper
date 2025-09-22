use std::path::{Path, PathBuf};
use std::env::consts::OS;

// Get the executable name with the platform-appropriate extension
pub fn get_exe_name(name: &str) -> String {
    if OS == "windows" {
        format!("{}.exe", name)
    } else {
        name.to_string()
    }
}

// Get the path to a binary in the binary directory
pub fn get_bin_path(bin_dir: &Path, name: &str) -> PathBuf {
    bin_dir.join(get_exe_name(name))
}

// Check if we're running on Windows
pub fn is_windows() -> bool {
    OS == "windows"
}

// Check if we're running on Unix-like OS (Linux/macOS)
pub fn is_unix() -> bool {
    OS == "linux" || OS == "macos"
}

// Get the appropriate shell command for the platform
pub fn get_shell_cmd() -> &'static str {
    if is_windows() {
        "powershell"
    } else {
        "sh"
    }
}

// Get the appropriate path separator for the platform
pub fn get_path_separator() -> &'static str {
    if is_windows() {
        ";"
    } else {
        ":"
    }
}