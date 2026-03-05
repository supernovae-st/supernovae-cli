//! Key format validation and masking utilities.

use crate::find_provider;
use core::fmt;

/// Result of key format validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationResult {
    /// Key format is valid.
    Valid,
    /// Key is empty.
    Empty,
    /// Key is too short (minimum 8 characters).
    TooShort {
        /// Actual length
        actual: usize,
        /// Minimum required length
        minimum: usize,
    },
    /// Key has invalid prefix.
    InvalidPrefix {
        /// Expected prefix
        expected: String,
        /// Actual prefix found
        actual: String,
    },
    /// Provider not found.
    UnknownProvider {
        /// The provider ID that wasn't found
        provider: String,
    },
}

impl ValidationResult {
    /// Returns true if the validation passed.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        matches!(self, ValidationResult::Valid)
    }
}

impl fmt::Display for ValidationResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Valid => write!(f, "Valid"),
            Self::Empty => write!(f, "Key is empty"),
            Self::TooShort { actual, minimum } => {
                write!(f, "Key too short ({} chars, minimum {})", actual, minimum)
            }
            Self::InvalidPrefix { expected, actual } => {
                write!(
                    f,
                    "Invalid prefix (expected '{}', got '{}')",
                    expected, actual
                )
            }
            Self::UnknownProvider { provider } => {
                write!(f, "Unknown provider: {}", provider)
            }
        }
    }
}

/// Validate an API key format for a specific provider.
///
/// This performs format validation only (prefix, length).
/// It does NOT make API calls to verify the key works.
///
/// # Example
///
/// ```
/// use spn_core::{validate_key_format, ValidationResult};
///
/// // Valid Anthropic key
/// let result = validate_key_format("anthropic", "sk-ant-api03-xxxxx");
/// assert!(result.is_valid());
///
/// // Invalid prefix
/// let result = validate_key_format("anthropic", "sk-wrong-key");
/// assert!(matches!(result, ValidationResult::InvalidPrefix { .. }));
///
/// // Too short
/// let result = validate_key_format("anthropic", "short");
/// assert!(matches!(result, ValidationResult::TooShort { .. }));
/// ```
#[must_use]
pub fn validate_key_format(provider_id: &str, key: &str) -> ValidationResult {
    // Check for empty key
    if key.is_empty() {
        return ValidationResult::Empty;
    }

    // Check minimum length
    const MIN_KEY_LENGTH: usize = 8;
    if key.len() < MIN_KEY_LENGTH {
        return ValidationResult::TooShort {
            actual: key.len(),
            minimum: MIN_KEY_LENGTH,
        };
    }

    // Find provider
    let Some(provider) = find_provider(provider_id) else {
        return ValidationResult::UnknownProvider {
            provider: provider_id.to_string(),
        };
    };

    // Check prefix if required
    if let Some(expected_prefix) = provider.key_prefix {
        if !key.starts_with(expected_prefix) {
            // Get actual prefix (same length as expected)
            let actual_prefix: String = key.chars().take(expected_prefix.len()).collect();
            return ValidationResult::InvalidPrefix {
                expected: expected_prefix.to_string(),
                actual: actual_prefix,
            };
        }
    }

    ValidationResult::Valid
}

/// Mask an API key for safe display.
///
/// Shows the prefix (if identifiable) followed by bullets.
/// Never exposes more than the first 7 characters.
///
/// # Example
///
/// ```
/// use spn_core::mask_key;
///
/// assert_eq!(mask_key("sk-ant-api03-secret-key"), "sk-ant-••••••••");
/// assert_eq!(mask_key("ghp_xxxxxxxxxxxxxxxxxxxx"), "ghp_xxx••••••••");
/// assert_eq!(mask_key("short"), "short••••••••");
/// assert_eq!(mask_key(""), "••••••••");
/// ```
#[must_use]
pub fn mask_key(key: &str) -> String {
    const MASK: &str = "••••••••";
    const MAX_VISIBLE: usize = 7;

    if key.is_empty() {
        return MASK.to_string();
    }

    // Show up to MAX_VISIBLE characters, then mask
    let visible_len = key.len().min(MAX_VISIBLE);
    let visible: String = key.chars().take(visible_len).collect();

    format!("{}{}", visible, MASK)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_empty() {
        assert_eq!(
            validate_key_format("anthropic", ""),
            ValidationResult::Empty
        );
    }

    #[test]
    fn test_validate_too_short() {
        let result = validate_key_format("anthropic", "short");
        assert!(matches!(
            result,
            ValidationResult::TooShort {
                actual: 5,
                minimum: 8
            }
        ));
    }

    #[test]
    fn test_validate_invalid_prefix() {
        let result = validate_key_format("anthropic", "sk-wrong-key-here");
        assert!(matches!(result, ValidationResult::InvalidPrefix { .. }));

        if let ValidationResult::InvalidPrefix { expected, actual } = result {
            assert_eq!(expected, "sk-ant-");
            assert_eq!(actual, "sk-wron");
        }
    }

    #[test]
    fn test_validate_valid_anthropic() {
        let result = validate_key_format("anthropic", "sk-ant-api03-xxxxxxxxxxxxxxxx");
        assert!(result.is_valid());
    }

    #[test]
    fn test_validate_valid_openai() {
        let result = validate_key_format("openai", "sk-xxxxxxxxxxxxxxxxxxxxxxxx");
        assert!(result.is_valid());
    }

    #[test]
    fn test_validate_valid_groq() {
        let result = validate_key_format("groq", "gsk_xxxxxxxxxxxxxxxxxxxx");
        assert!(result.is_valid());
    }

    #[test]
    fn test_validate_valid_github() {
        let result = validate_key_format("github", "ghp_xxxxxxxxxxxxxxxxxxxx");
        assert!(result.is_valid());
    }

    #[test]
    fn test_validate_no_prefix_required() {
        // Mistral doesn't have a required prefix
        let result = validate_key_format("mistral", "any-valid-key-format");
        assert!(result.is_valid());
    }

    #[test]
    fn test_validate_unknown_provider() {
        let result = validate_key_format("unknown_provider", "some-key");
        assert!(matches!(result, ValidationResult::UnknownProvider { .. }));
    }

    #[test]
    fn test_mask_key() {
        assert_eq!(mask_key("sk-ant-api03-secret"), "sk-ant-••••••••");
        assert_eq!(mask_key("ghp_xxxxxxxxxxxx"), "ghp_xxx••••••••");
        assert_eq!(mask_key("short"), "short••••••••");
        assert_eq!(mask_key(""), "••••••••");
    }

    #[test]
    fn test_mask_key_max_visible() {
        // Should show at most 7 characters
        let long_key = "1234567890123456789";
        let masked = mask_key(long_key);
        assert_eq!(masked, "1234567••••••••");
    }
}
