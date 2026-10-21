use std::{fs, path::PathBuf, process::Command};

fn setup_config_dir() -> PathBuf {
    let config_dir = if cfg!(target_os = "windows") {
        std::env::var_os("APPDATA")
            .map(|path| PathBuf::from(path).join("entropick"))
            .expect("Missing %APPDATA% environment variable on Windows")
    } else {
        std::env::var_os("XDG_CONFIG_HOME")
        .map(|path| PathBuf::from(path).join("entropick"))
        .or_else(|| {
            std::env::var_os("HOME")
                .map(|path| PathBuf::from(path).join(".config").join("entropick"))
        })
        .expect("Could not determine Unix config directory (both $XDG_CONFIG_HOME and $HOME are missing)")
    };

    if config_dir.exists() {
        fs::remove_dir_all(&config_dir).unwrap();
    }
    config_dir
}

#[test]
fn test_binary_generates_patterns() {
    let binary_path = env!("CARGO_BIN_EXE_entropick");

    let mut cmd = Command::new(binary_path);
    let status = cmd
        .arg("generate")
        .args(["--pattern", "\"[[a-z]][[0-9]][[A-Z]]\""])
        .args(["-l", "10"])
        .args(["--count", "40"])
        .output()
        .expect("Failed to generate with the compiled binary");

    assert!(status.status.success(), "Binary has successful exit status");
}
#[test]
fn test_binary_creates_toml_file() {
    let config_dir = setup_config_dir();

    let binary_path = env!("CARGO_BIN_EXE_entropick");

    let mut cmd = Command::new(binary_path);
    let status = cmd
        .args(["profile", "add", "test_profile_TP"])
        .args(["--pattern", "\"[[a-z]][[0-9]][[A-Z]]\""])
        .args(["--length", "12"])
        .status()
        .expect("Failed to add a profile with the compiled binary");

    assert!(status.success(), "Binary exited with non-zero status");
    assert!(
        config_dir.is_dir(),
        "Expected config directory was not created at: {:?}",
        config_dir
    );
    let mut cmd = Command::new(binary_path);

    let output = cmd
        .args(["profile", "list"])
        .output()
        .expect("Failed to add a profile with the compiled binary");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("Error output was not valid UTF-8");
    eprintln!("LOOKHERE == {}", stdout);
    assert!(
        stdout.contains("test_profile_TP"),
        "Added profile has not been listed using the list subcommand"
    );
}
