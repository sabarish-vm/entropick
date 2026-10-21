use std::path::PathBuf;

/// Returns the config directory for entropick, checking the environment
/// variable `ENTROPICK_CONFIG_DIR` first. If unset, uses the platform default:
///
///  Unix    : `$XDG_CONFIG_HOME/entropick` → `$HOME/.config/entropick`
///  Windows : `%APPDATA%/entropick`
pub fn config_dir() -> Option<PathBuf> {
    // Allow override via environment variable (useful for tests)
    let env_override = std::env::var("ENTROPICK_CONFIG_DIR").ok();
    if env_override.is_some() {
        return env_override.map(PathBuf::from);
    }

    let base = if cfg!(target_os = "windows") {
        std::env::var("APPDATA").ok().map(PathBuf::from)
    } else {
        std::env::var("XDG_CONFIG_HOME")
            .ok()
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var("HOME")
                    .ok()
                    .map(|h| PathBuf::from(h).join(".config"))
            })
    };

    base.map(|p| p.join("entropick"))
}

/// Returns the full path to the profiles TOML file.
pub fn profiles_path() -> Option<PathBuf> {
    config_dir().map(|d| d.join("profiles.toml"))
}

