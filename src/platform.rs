use std::env::consts::OS;
use std::path::{Path, PathBuf};

// Get the executable name with the platform-appropriate extension
pub fn get_exe_name(name: &str) -> String {
    if is_windows() {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_exe_name_uses_platform_extension() {
        let exe_name = get_exe_name("kubectl");

        if is_windows() {
            assert_eq!(exe_name, "kubectl.exe");
        } else {
            assert_eq!(exe_name, "kubectl");
        }
    }

    #[test]
    fn get_bin_path_joins_directory_and_executable_name() {
        let bin_dir = Path::new("/tmp/thumed-bin");
        let path = get_bin_path(bin_dir, "helm");

        assert_eq!(path, bin_dir.join(get_exe_name("helm")));
    }
}
