//! String utilities for safe text handling

/// Maximum preview length for text in logs
pub const MAX_PREVIEW_LEN: usize = 100;

/// Creates a safe UTF-8 preview of a string, respecting character boundaries.
///
/// Unlike byte slicing (`&s[..n]`), this function will never panic on
/// multi-byte UTF-8 characters (e.g., Vietnamese, Chinese, emoji).
///
/// # Example
/// ```
/// let text = "chiến thắng";
/// let preview = safe_preview(text, 5); // "chiến..."
/// ```
pub fn safe_preview(text: &str, max_chars: usize) -> String {
    let preview: String = text.chars().take(max_chars).collect();
    // Use nth() for O(1) check instead of count() which is O(N)
    if text.chars().nth(max_chars).is_some() {
        format!("{}...", preview)
    } else {
        preview
    }
}

/// Creates a safe UTF-8 preview with ellipsis if truncated.
/// Returns the original string if it fits within max_chars.
pub fn truncate_safe(text: &str, max_chars: usize) -> &str {
    if text.chars().count() <= max_chars {
        text
    } else {
        // Find the byte index after max_chars characters
        let byte_idx = text.char_indices()
            .nth(max_chars)
            .map(|(idx, _)| idx)
            .unwrap_or(text.len());
        &text[..byte_idx]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safe_preview_ascii() {
        assert_eq!(safe_preview("hello world", 5), "hello...");
        assert_eq!(safe_preview("hi", 5), "hi");
    }

    #[test]
    fn test_safe_preview_utf8() {
        // Vietnamese: "chiến thắng" - each char is multi-byte
        let vietnamese = "chiến thắng";
        let preview = safe_preview(vietnamese, 5);
        assert!(preview.starts_with("chiến"));
        assert!(preview.ends_with("..."));

        // Should not panic on emoji
        let emoji = "Hello 🎉 World 🌍";
        let preview = safe_preview(emoji, 8);
        // First 8 chars: "Hello 🎉 " (H e l l o space 🎉 space), then "..."
        assert_eq!(preview, "Hello 🎉 ...");
    }

    #[test]
    fn test_safe_preview_empty() {
        assert_eq!(safe_preview("", 10), "");
    }

    #[test]
    fn test_truncate_safe() {
        assert_eq!(truncate_safe("hello", 10), "hello");
        assert_eq!(truncate_safe("hello world", 5), "hello");

        // UTF-8 safety
        let vietnamese = "chiến thắng";
        let truncated = truncate_safe(vietnamese, 5);
        assert_eq!(truncated, "chiến");
    }
}
