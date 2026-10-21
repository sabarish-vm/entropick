use std::collections::BTreeMap;
use std::fs;

use crate::config::profiles_path;
use crate::types::Profile;

// Load all profiles from the TOML file.
// Returns an empty map if the file
pub fn load_profiles() -> BTreeMap<String, Profile> {
    let path = match profiles_path() {
        Some(p) => p,
        None => {
            eprintln!("(config directory not found)");
            return BTreeMap::new();
        }
    };

    if !path.exists() {
        eprintln!(
            "no profiles file found at {}, starting fresh",
            path.display()
        );
        return BTreeMap::new();
    }
    let content = fs::read_to_string(path).unwrap_or_default();
    toml::from_str::<BTreeMap<String, Profile>>(&content).unwrap_or_else(|err| {
        eprintln!("Loading of TOML file failed: {err}");
        BTreeMap::new()
    })
}

/// Save profiles to the TOML file, creating the config directory if needed.
pub fn save_profiles(profiles: &BTreeMap<String, Profile>) -> Result<(), String> {
    let path = profiles_path().ok_or("could not determine config directory")?;

    // Create parent directory if it doesn't exist
    if let Some(parent) = path.parent()
        && let Err(e) = fs::create_dir_all(parent)
    {
        return Err(format!("failed to create config directory: {e}"));
    }

    let content = toml::to_string_pretty(profiles)
        .map_err(|e| format!("failed to serialize profiles: {e}"))?;

    fs::write(&path, content).map_err(|e| format!("failed to write profiles: {e}"))?;

    Ok(())
}

/// Add or overwrite a profile.
pub fn add_profile(name: &str, profile: Profile) -> Result<(), String> {
    if name.is_empty() {
        return Err("profile name cannot be empty".to_string());
    }

    let mut profiles = load_profiles();
    profiles.insert(name.to_string(), profile);
    save_profiles(&profiles)
}

/// Get a single profile by name.
pub fn get_profile(name: &str) -> Option<Profile> {
    let profiles = load_profiles();
    profiles.get(name).cloned()
}

/// List all profile names.
pub fn list_profiles() -> Vec<String> {
    let profiles = load_profiles();
    profiles.into_keys().collect()
}

/// Edit fields of an existing profile. Both `pattern` and `length` are
/// optional — only present fields will be updated.
pub fn edit_profile(
    name: &str,
    pattern: Option<&String>,
    length: Option<&usize>,
) -> Result<(), String> {
    let mut profiles = load_profiles();

    let entry = profiles
        .get_mut(name)
        .ok_or_else(|| format!("profile '{name}' not found"))?;

    if let Some(p) = pattern {
        entry.pattern = p.clone();
    }
    if let Some(l) = length {
        entry.length = *l;
    }

    save_profiles(&profiles)
}

/// Remove a profile by name.
pub fn remove_profile(name: &str) -> Result<(), String> {
    let mut profiles = load_profiles();

    if profiles.remove(name).is_none() {
        return Err(format!("profile '{name}' not found"));
    }

    save_profiles(&profiles)
}

/// Search profile names containing `query` (case-insensitive).
pub fn search_profiles(query: &str) -> Vec<(String, Profile)> {
    let profiles = load_profiles();
    let query = query.to_lowercase();

    profiles
        .into_iter()
        .filter(|(name, _)| name.to_lowercase().contains(&query))
        .collect()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use crate::profiles::{add_profile, edit_profile, get_profile, list_profiles, remove_profile};
    use crate::types::Profile;

    /// Creates an isolated temp directory and sets ENTROPICK_CONFIG_DIR.
    /// The `TempDir` auto-deletes when it goes out of scope.
    fn isolate() -> tempfile::TempDir {
        let dir = tempfile::TempDir::new().expect("failed to create temp dir");
        unsafe {
            std::env::set_var("ENTROPICK_CONFIG_DIR", dir.path());
        }
        dir
    }

    fn cleanup() {
        unsafe {
            std::env::remove_var("ENTROPICK_CONFIG_DIR");
        }
    }

    #[test]
    fn test_add_and_get() {
        let _ = isolate();
        let profile = Profile {
            pattern: "[[a-z]][[A-Z]]".to_string(),
            length: 16,
        };

        assert!(add_profile("test_add", profile.clone()).is_ok());
        let retrieved = get_profile("test_add");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().length, 16);
        cleanup();
    }

    #[test]
    fn test_remove() {
        let _ = isolate();
        let profile = Profile {
            pattern: "[[0-9]]".to_string(),
            length: 8,
        };

        assert!(add_profile("test_remove", profile).is_ok());
        assert!(get_profile("test_remove").is_some());
        assert!(remove_profile("test_remove").is_ok());
        assert!(get_profile("test_remove").is_none());
        cleanup();
    }

    #[test]
    fn test_edit() {
        let _ = isolate();
        let profile = Profile {
            pattern: "[[a-z]]".to_string(),
            length: 8,
        };

        assert!(add_profile("test_edit", profile).is_ok());
        assert!(edit_profile("test_edit", Some(&"[[A-Z]]".to_string()), None).is_ok());

        let retrieved = get_profile("test_edit").unwrap();
        assert_eq!(retrieved.pattern, "[[A-Z]]");
        assert_eq!(retrieved.length, 8); // unchanged
        cleanup();
    }

    #[test]
    fn test_list() {
        let _ = isolate();

        add_profile(
            "test_list_a",
            Profile {
                pattern: "[[a-z]]".to_string(),
                length: 10,
            },
        )
        .unwrap();
        add_profile(
            "test_list_b",
            Profile {
                pattern: "[[A-Z]]".to_string(),
                length: 12,
            },
        )
        .unwrap();

        let names = list_profiles();
        assert!(names.contains(&"test_list_a".to_string()));
        assert!(names.contains(&"test_list_b".to_string()));
        cleanup();
    }
}
