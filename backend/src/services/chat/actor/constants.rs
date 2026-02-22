//! Constants for ChatActor behavior

/// Maximum number of retries for transient AI engine errors
pub const MAX_AI_RETRIES: u32 = 3;
/// Initial backoff duration in milliseconds for retries
pub const RETRY_BACKOFF_MS: u64 = 1000;
/// Stream read timeout in seconds - if no data received from API within this time, consider it stalled
pub const STREAM_READ_TIMEOUT_SECS: u64 = 120;
