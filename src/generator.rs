use rand::rngs::ChaCha20Rng;
use rand::{RngExt, seq::SliceRandom};
use secrecy::SecretString;
use zeroize::Zeroize;
use zxcvbn::zxcvbn;

use crate::types::{GenerateConfig, GeneratedPassword};

type RngUsed = ChaCha20Rng;

/// Generate a password from a parsed `GenerateConfig`.
///
/// 1. At least one character from each `[[…]]` block appears in the
///    final output.
/// 2. ChaCha20 RNG is used to draw random characters from the ppool
/// 3. A Fisher-Yates shuffle is applied
/// 4. The generated password is wrapped in `SecretString`
///    and intermediate String buffers are zeroed on drop.
///    This can be an overkill, but it is not a lot of work to implement it
pub fn generate_password(config: &GenerateConfig) -> Result<GeneratedPassword, String> {
    let mandatory_count = config.mandatory_count;
    let target_length = config.length;

    if target_length < mandatory_count {
        return Err(format!(
            "target length ({target_length}) is less than the number of \
             mandatory blocks ({mandatory_count})",
        ));
    }

    // 1. Pick one guaranteed character from each mandatory block
    let mut chars: Vec<char> = Vec::with_capacity(target_length);
    for pool in &config.pool.mandatory {
        let mut rng: RngUsed = rand::make_rng();
        let idx = rng.random_range(0..pool.len());
        chars.push((*pool)[idx]);
    }

    // 2. Build a unified pool from every block (mandatory + optional)
    let mut unified: Vec<char> = Vec::new();
    for pool in &config.pool.mandatory {
        unified.extend(pool);
    }
    for pool in &config.pool.optional {
        unified.extend(pool);
    }

    if unified.is_empty() {
        return Err("internal error: character pool is empty".to_string());
    }

    // 3. Fill remaining positions from the unified pool
    let remaining = target_length - mandatory_count;
    for _ in 0..remaining {
        let mut rng: RngUsed = rand::make_rng();
        let idx = rng.random_range(0..unified.len());
        chars.push(unified[idx]);
    }

    // 4. Fisher-Yates shuffle — eliminates positional bias
    {
        let mut rng: RngUsed = rand::make_rng();
        chars.shuffle(&mut rng);
    }
    // 5. Build the output string and secure-wrap it
    let password_string = chars.iter().collect::<String>();

    // Zeroize intermediate buffers before they are dropped
    chars.zeroize();
    unified.zeroize();
    let entropy = zxcvbn(password_string.as_str(), &[]);
    Ok(GeneratedPassword {
        value: SecretString::new(password_string.into()),
        entropy,
    })
}

/// Convenience wrapper: parse the template and generate in one call.
pub fn generate_from_pattern(template: &str, length: usize) -> Result<GeneratedPassword, String> {
    use crate::parser::parse_pattern;

    let pool = match parse_pattern(template) {
        Ok(p) => p,
        Err(e) => {
            return Err(format!("parse error: {e}"));
        }
    };

    let mandatory_count = pool.mandatory.len();
    let config = GenerateConfig {
        pool,
        mandatory_count,
        length,
    };

    generate_password(&config)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use secrecy::ExposeSecret;

    use crate::generator::{generate_from_pattern, generate_password};
    use crate::types::{CharPool, GenerateConfig};

    /// Helper: build a simple config from two mandatory pools.
    fn make_config() -> GenerateConfig {
        GenerateConfig {
            pool: CharPool {
                mandatory: [
                    "abcdefghijklmnopqrstuvwxyz".chars().collect::<Vec<char>>(),
                    "ABCDEFGHIJKLMNOPQRSTUVWXYZ".chars().collect::<Vec<char>>(),
                ]
                .to_vec(),
                optional: ["!@#$%".chars().collect::<Vec<char>>()].to_vec(),
            },
            mandatory_count: 2,
            length: 16,
        }
    }

    // -- structural tests ---------------------------------------------------

    #[test]
    fn test_generates_correct_length() {
        let config = make_config();
        let result = generate_password(&config);
        assert!(result.is_ok());
        let password = result.unwrap();
        let exposed = password.value.expose_secret();
        assert_eq!(exposed.len(), 16);
    }

    #[test]
    fn test_length_too_short_returns_error() {
        let config = GenerateConfig {
            length: 1,
            ..make_config()
        };
        let result = generate_password(&config);
        assert!(result.is_err());
    }

    // -- guarantee tests ----------------------------------------------------

    #[test]
    fn test_every_mandatory_block_contributes() {
        let config = make_config();
        let lowercase_set = "abcdefghijklmnopqrstuvwxyz";
        let uppercase_set = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";

        for _ in 0..1000 {
            let result = generate_password(&config);
            assert!(result.is_ok());
            let generated = result.unwrap();
            let s = generated.value.expose_secret();

            let has_lower = s.chars().any(|c| lowercase_set.contains(c));
            let has_upper = s.chars().any(|c| uppercase_set.contains(c));

            assert!(has_lower, "missing lowercase character in {s}");
            assert!(has_upper, "missing uppercase character in {s}");
        }
    }

    #[test]
    fn test_single_mandatory_block() {
        let config = GenerateConfig {
            pool: CharPool {
                mandatory: ["0123456789".chars().collect::<Vec<char>>()].to_vec(),
                optional: Vec::new(),
            },
            mandatory_count: 1,
            length: 10,
        };

        for _ in 0..100 {
            let result = generate_password(&config);
            assert!(result.is_ok());
            let generated = result.unwrap();
            let s = generated.value.expose_secret();
            assert_eq!(s.len(), 10);
            for c in s.chars() {
                assert!(c.is_ascii_digit(), "non-digit character '{c}' in {s}");
            }
        }
    }

    #[test]
    fn test_not_all_identical() {
        let config = make_config();
        let mut seen: Vec<String> = Vec::new();

        for _ in 0..20 {
            let result = generate_password(&config);
            assert!(result.is_ok());
            let generated = result.unwrap();
            let s = generated.value.expose_secret();

            let s_str = s.to_string();
            if !seen.contains(&s_str) {
                seen.push(s_str);
            }
        }

        assert!(
            seen.len() > 1,
            "all generated passwords were identical — RNG may be broken",
        );
    }

    // -- integration via template -------------------------------------------

    #[test]
    fn test_generate_from_template() {
        let result = generate_from_pattern("[[a-z]][[A-Z]][[0-9]]((!@#$%^&*))", 16);
        assert!(result.is_ok());
        let generated = result.unwrap();
        let s = generated.value.expose_secret();
        assert_eq!(s.len(), 16);
        let has_lower = s.chars().any(|c| c.is_ascii_lowercase());
        let has_upper = s.chars().any(|c| c.is_ascii_uppercase());
        let has_digit = s.chars().any(|c| c.is_ascii_digit());

        assert!(has_lower, "no lowercase in {s}");
        assert!(has_upper, "no uppercase in {s}");
        assert!(has_digit, "no digit in {s}");
    }
}
