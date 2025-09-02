//! Entry struct definitions - directly from original code.

#[cfg(not(feature = "std"))]
use alloc::string::String;

// Conditional imports based on feature flags
#[cfg(feature = "nanoserde")]
use nanoserde::{DeJson, SerJson};

#[cfg(all(not(feature = "nanoserde"), feature = "std"))]
use serde::{Deserialize, Serialize};

// Entry struct with conditional serialization
#[cfg(feature = "nanoserde")]
#[derive(DeJson, SerJson, Debug, Clone)]
pub struct Entry {
    pub value: String, // nanoserde stores as JSON string
    #[nserde(default)]
    pub expires_at: Option<u64>, // UNIX timestamp (seconds)
}

#[cfg(all(not(feature = "nanoserde"), feature = "std"))]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Entry {
    pub value: serde_json::Value,
    #[serde(default)]
    pub expires_at: Option<u64>, // UNIX timestamp (seconds)
}

// For no_std without nanoserde, we use a simpler approach
#[cfg(all(not(feature = "nanoserde"), not(feature = "std")))]
#[derive(Debug, Clone)]
pub struct Entry {
    pub value: String,           // Simple string storage for no_std
    pub expires_at: Option<u64>, // UNIX timestamp (seconds)
}
