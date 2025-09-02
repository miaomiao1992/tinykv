//! Error types for TinyKV operations.

#[cfg(feature = "std")]
use std::io;

#[cfg(not(feature = "std"))]
use alloc::string::String;

/// Errors that can occur while using the TinyKV store.
#[derive(Debug)]
pub enum TinyKVError {
    /// File system related error (only available with std)
    #[cfg(feature = "std")]
    Io(io::Error),
    /// Serialization or deserialization failure
    Serialization(String),
    /// System time is before the UNIX epoch (only available with std)
    #[cfg(feature = "std")]
    TimeError,
    /// Feature not available in no_std mode
    #[cfg(not(feature = "std"))]
    NoStdUnsupported(String),
    /// Web storage related error (only available with wasm)
    #[cfg(feature = "wasm")]
    WebStorage(String),
}

#[cfg(feature = "std")]
impl From<io::Error> for TinyKVError {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}

#[cfg(all(not(feature = "nanoserde"), feature = "std"))]
impl From<serde_json::Error> for TinyKVError {
    fn from(err: serde_json::Error) -> Self {
        Self::Serialization(err.to_string())
    }
}

impl core::fmt::Display for TinyKVError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            #[cfg(feature = "std")]
            Self::Io(e) => write!(f, "IO error: {e}"),
            Self::Serialization(e) => write!(f, "Serialization error: {e}"),
            #[cfg(feature = "std")]
            Self::TimeError => write!(f, "Time error"),
            #[cfg(not(feature = "std"))]
            Self::NoStdUnsupported(msg) => write!(f, "Feature not available in no_std: {msg}"),
            #[cfg(feature = "wasm")]
            Self::WebStorage(msg) => write!(f, "Web storage error: {msg}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for TinyKVError {}
