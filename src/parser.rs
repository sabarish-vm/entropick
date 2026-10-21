use crate::types::CharPool;
use std::fmt;

/// Whether a parsed block is mandatory `[[…]]` or optional `((…))`.
#[derive(Debug, Clone, PartialEq)]
pub enum BlockType {
    Mandatory,
    Optional,
}

/// A raw token: the block type and the inner string extracted from the brackets.
#[derive(Debug, Clone)]
pub struct Token {
    pub block_type: BlockType,
    pub inner: String,
    pub start: usize,
}

/// Errors that can occur during template parsing.
#[derive(Debug, Clone)]
pub enum ParseError {
    /// An opening bracket was never closed.
    UnmatchedOpen(usize),
    /// A block contained no characters at all: `[[]]` or `(())`.
    EmptyBlock(usize),
    /// An invalid character class range was found (e.g. `z-a` or `a-Z`).
    InvalidRange(usize, char, char),
    /// Multiple dashes in a character class (e.g. `a-c-f`), likely a typo.
    MultipleRanges(usize),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ParseError::UnmatchedOpen(pos) => {
                write!(f, "unmatched opening bracket at position {pos}")
            }
            ParseError::EmptyBlock(pos) => {
                write!(f, "empty block at position {pos}")
            }
            ParseError::InvalidRange(pos, start, end) => {
                write!(
                    f,
                    "invalid character range '{start}-{end}' at position {pos}"
                )
            }
            ParseError::MultipleRanges(pos) => {
                write!(
                    f,
                    "multiple ranges in a single block at position {pos} — \
                     each range must be in its own [[…]] block"
                )
            }
        }
    }
}

/// Tokenize a template string into a sequence of `Token` values.
pub fn tokenize(template: &str) -> Result<Vec<Token>, ParseError> {
    let mut tokens: Vec<Token> = Vec::new();
    let chars = template.chars().collect::<Vec<char>>();
    let mut i = 0;
    let len = chars.len();

    while i < len {
        // Check for mandatory block [[…]]
        if i + 1 < len && chars[i] == '[' && chars[i + 1] == '[' {
            let start = i;
            i += 2;
            let (close, _) = find_closing(&chars, i, ']', ']');
            if close == usize::MAX {
                return Err(ParseError::UnmatchedOpen(start));
            }
            let inner = chars[i..close].iter().collect::<String>();
            let trimmed = inner.trim();
            if trimmed.is_empty() {
                return Err(ParseError::EmptyBlock(start));
            }
            tokens.push(Token {
                block_type: BlockType::Mandatory,
                inner: trimmed.to_string(),
                start,
            });
            i = close + 2;
            continue;
        }

        // Check for optional block ((…))
        if i + 1 < len && chars[i] == '(' && chars[i + 1] == '(' {
            let start = i;
            i += 2;
            let (close, _) = find_closing(&chars, i, ')', ')');
            if close == usize::MAX {
                return Err(ParseError::UnmatchedOpen(start));
            }
            let inner = chars[i..close].iter().collect::<String>();
            let trimmed = inner.trim();
            if trimmed.is_empty() {
                return Err(ParseError::EmptyBlock(start));
            }
            tokens.push(Token {
                block_type: BlockType::Optional,
                inner: trimmed.to_string(),
                start,
            });
            i = close + 2;
            continue;
        }

        // Characters outside brackets – skip silently
        i += 1;
    }

    Ok(tokens)
}

/// Search for a two-character delimiter starting at position `start`.
fn find_closing(
    chars: &[char],
    start: usize,
    delim_first: char,
    delim_second: char,
) -> (usize, bool) {
    let mut i = start;
    while i + 1 < chars.len() {
        if chars[i] == delim_first && chars[i + 1] == delim_second {
            return (i, true);
        }
        i += 1;
    }
    (usize::MAX, false)
}

/// Expand a single character-class string into a flat `Vec<char>`.
///
/// Supports ASCII ranges (`a-z`, `A-Z`, `0-9`) and literal character sets.
/// Multiple dashes in one block (e.g. `a-c-f` or `a-zA-Z`) return an error.
pub fn expand_class(inner: &str, pos: usize) -> Result<Vec<char>, ParseError> {
    let chars = inner.chars().collect::<Vec<char>>();

    // Multiple dashes is likely a typo (e.g. "a-c-f" or "a-zA-Z").
    let dash_count = chars.iter().filter(|&&c| c == '-').count();
    if dash_count > 1 {
        return Err(ParseError::MultipleRanges(pos));
    }

    let mut result: Vec<char> = Vec::new();
    let mut i = 0;

    while i < chars.len() {
        if i + 2 < chars.len() && chars[i + 1] == '-' {
            let start = chars[i];
            let end = chars[i + 2];
            if is_valid_ascii_range(start, end) {
                for c in start..=end {
                    result.push(c);
                }
                i += 3;
                continue;
            } else {
                return Err(ParseError::InvalidRange(i, start, end));
            }
        }

        // Literal character (including lone dashes)
        result.push(chars[i]);
        i += 1;
    }

    Ok(result)
}

/// Returns `true` if the pair forms a valid ASCII range (`a-z`, `A-Z`, or `0-9`).
fn is_valid_ascii_range(start: char, end: char) -> bool {
    if start > end {
        return false;
    }
    let in_range = |c: char, lo: char, hi: char| -> bool { c >= lo && c <= hi };

    in_range(start, 'a', 'z') && in_range(end, 'a', 'z')
        || in_range(start, 'A', 'Z') && in_range(end, 'A', 'Z')
        || in_range(start, '0', '9') && in_range(end, '0', '9')
}

/// Parse a full template string into a `CharPool`.
pub fn parse_pattern(template: &str) -> Result<CharPool, ParseError> {
    let tokens = tokenize(template)?;

    let mut mandatory: Vec<Vec<char>> = Vec::new();
    let mut optional: Vec<Vec<char>> = Vec::new();

    for token in tokens {
        let pool = expand_class(&token.inner, token.start)?;
        if pool.is_empty() {
            return Err(ParseError::EmptyBlock(token.start));
        }
        match token.block_type {
            BlockType::Mandatory => {
                mandatory.push(pool);
            }
            BlockType::Optional => {
                optional.push(pool);
            }
        }
    }

    if mandatory.is_empty() {
        return Err(ParseError::EmptyBlock(0));
    }

    Ok(CharPool {
        mandatory,
        optional,
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use crate::parser::{BlockType, ParseError, expand_class, parse_pattern, tokenize};

    // -- tokenize -----------------------------------------------------------

    #[test]
    fn test_tokenize_simple() {
        let tokens = tokenize("[[a-z]][[A-Z]]");
        assert!(tokens.is_ok());
        let t = tokens.unwrap();
        assert_eq!(t.len(), 2);
        assert_eq!(t[0].block_type, BlockType::Mandatory);
        assert_eq!(t[0].inner, "a-z");
        assert_eq!(t[1].block_type, BlockType::Mandatory);
        assert_eq!(t[1].inner, "A-Z");
    }

    #[test]
    fn test_tokenize_mixed() {
        let tokens = tokenize("[[a-z]]((!@#))");
        assert!(tokens.is_ok());
        let t = tokens.unwrap();
        assert_eq!(t.len(), 2);
        assert_eq!(t[0].block_type, BlockType::Mandatory);
        assert_eq!(t[0].inner, "a-z");
        assert_eq!(t[1].block_type, BlockType::Optional);
        assert_eq!(t[1].inner, "!@#");
    }

    #[test]
    fn test_tokenize_realistic() {
        let tokens = tokenize("[[a-z]][[A-Z]][[0-9]]((!@#$%^&*))");
        assert!(tokens.is_ok());
        let t = tokens.unwrap();
        assert_eq!(t.len(), 4);
    }

    #[test]
    fn test_tokenize_unmatched_open() {
        let result = tokenize("[[a-z]");
        assert!(result.is_err());
        let err = result.unwrap_err();
        match err {
            ParseError::UnmatchedOpen(_) => { /* expected */ }
            _ => panic!("expected UnmatchedOpen, got {:?}", err),
        }
    }

    #[test]
    fn test_tokenize_empty_block() {
        let result = tokenize("[[]]");
        assert!(result.is_err());
    }

    #[test]
    fn test_tokenize_empty_optional() {
        let result = tokenize("(())");
        assert!(result.is_err());
    }

    #[test]
    fn test_tokenize_skips_outside_chars() {
        let tokens = tokenize("abc[[a-z]]def");
        assert!(tokens.is_ok());
        let t = tokens.unwrap();
        assert_eq!(t.len(), 1);
        assert_eq!(t[0].inner, "a-z");
    }

    // -- expand_class -------------------------------------------------------

    #[test]
    fn test_expand_lowercase() {
        let expanded = expand_class("a-z", 0).unwrap();
        assert_eq!(expanded.len(), 26);
        assert_eq!(expanded[0], 'a');
        assert_eq!(expanded[25], 'z');
    }

    #[test]
    fn test_expand_uppercase() {
        let expanded = expand_class("A-Z", 0).unwrap();
        assert_eq!(expanded.len(), 26);
        assert_eq!(expanded[0], 'A');
        assert_eq!(expanded[25], 'Z');
    }

    #[test]
    fn test_expand_digits() {
        let expanded = expand_class("0-9", 0).unwrap();
        assert_eq!(expanded.len(), 10);
        assert_eq!(expanded[0], '0');
        assert_eq!(expanded[9], '9');
    }

    #[test]
    fn test_expand_literal_chars() {
        let expanded = expand_class("!@#$%", 0).unwrap();
        assert_eq!(expanded.len(), 5);
        assert_eq!(expanded, ['!', '@', '#', '$', '%']);
    }

    #[test]
    fn test_expand_multiple_dashes_errors() {
        let result = expand_class("a-zA-Z", 0);
        assert!(result.is_err());
        match result.unwrap_err() {
            ParseError::MultipleRanges(_) => { /* expected */ }
            _ => panic!("expected MultipleRanges"),
        }
    }

    #[test]
    fn test_expand_multiple_dashes_surrounding_errors() {
        let result = expand_class("a-c-f", 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_expand_lone_dash_is_literal() {
        let expanded = expand_class("a-", 0).unwrap();
        assert_eq!(expanded.len(), 2);
        assert_eq!(expanded, ['a', '-']);
    }

    #[test]
    fn test_expand_leading_dash_is_literal() {
        let expanded = expand_class("-a", 0).unwrap();
        assert_eq!(expanded.len(), 2);
        assert_eq!(expanded, ['-', 'a']);
    }

    #[test]
    fn test_expand_invalid_range_throws_error() {
        let expanded = expand_class("z-a", 0);
        assert!(expanded.is_err())
    }

    #[test]
    fn test_expand_empty() {
        let expanded = expand_class("", 0).unwrap();
        assert!(expanded.is_empty());
    }

    // -- parse_template -----------------------------------------------------

    #[test]
    fn test_parse_template_full() {
        let pool = parse_pattern("[[a-z]][[A-Z]][[0-9]]((!@#$%^&*))");
        assert!(pool.is_ok());
        let pool = pool.unwrap();
        assert_eq!(pool.mandatory.len(), 3);
        assert_eq!(pool.optional.len(), 1);
        assert_eq!(pool.mandatory[0].len(), 26);
        assert_eq!(pool.mandatory[1].len(), 26);
        assert_eq!(pool.mandatory[2].len(), 10);
        assert_eq!(pool.optional[0].len(), 8);
    }

    #[test]
    fn test_parse_template_no_optional() {
        let pool = parse_pattern("[[a-z]][[A-Z]]").unwrap();
        assert_eq!(pool.mandatory.len(), 2);
        assert!(pool.optional.is_empty());
    }

    #[test]
    fn test_parse_template_only_mandatory() {
        let pool = parse_pattern("[[0-9]]");
        assert!(pool.is_ok());
        assert_eq!(pool.unwrap().mandatory.len(), 1);
    }

    #[test]
    fn test_parse_template_no_mandatory_is_error() {
        let result = parse_pattern("((a-z))");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_template_empty_template_is_error() {
        let result = parse_pattern("");
        assert!(result.is_err());
    }
}
