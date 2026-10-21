use secrecy::SecretString;
use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, ZeroizeOnDrop};
use zxcvbn::Entropy;

/// The fully parsed template, with every character class expanded into
/// concrete character pools ready for sampling.
///
/// Memory is automatically zeroed when this value is dropped.
#[derive(Debug, Zeroize, ZeroizeOnDrop)]
pub struct CharPool {
    /// One inner `Vec<char>` per `[[...]]` block.
    pub mandatory: Vec<Vec<char>>,
    /// One inner `Vec<char>` per `((...))` block.
    pub optional: Vec<Vec<char>>,
}

/// A saved profile containing the template pattern and target length.
///
/// Stored in TOML under `~/.config/entropick/profiles.toml`.
/// Only the rules are stored — **never** a generated password.
#[derive(Serialize, Deserialize, Clone)]
pub struct Profile {
    /// The bracket-format template string (e.g. `"[[a-z]][[A-Z]][[0-9]]((!@#$%^&*))"`).
    pub pattern: String,
    pub length: usize,
}

pub struct GenerateConfig {
    /// The expanded character pools.
    pub pool: CharPool,
    /// How many mandatory blocks exist (derived from `pool.mandatory.len()`).
    pub mandatory_count: usize,
    /// The target length for the generated password.
    pub length: usize,
}

/// The result of a password generation.
///
/// Wraps the password in `SecretString` to prevent accidental
/// logging, screen capture, or memory inspection.
pub struct GeneratedPassword {
    /// The generated password, protected by `secrecy`.
    pub value: SecretString,
    pub entropy: Entropy,
}
