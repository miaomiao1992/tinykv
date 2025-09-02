//! # TinyKV
//!
//! A minimal JSON-based persistent key-value store for Rust projects.
//! Supports optional TTL (expiration), auto-saving, backup, and serialization via `serde` or `nanoserde`.
//! Works in `no_std` environments with `alloc`.
//!
//! ## Features
//! - File-based storage using pretty-formatted JSON
//! - Optional TTL expiration support per key
//! - Auto-saving changes on modification
//! - Backup support with `.bak` files
//! - Simple interface with `serde` (default) or `nanoserde` (feature flag)
//! - `no_std` support with `alloc`
//!
//! ## Feature Flags
//! - `default`: Uses `serde` for serialization (maximum compatibility) and `std`
//! - `nanoserde`: Uses `nanoserde` for minimal binary size and faster compilation
//! - `std`: Enables `std` library (enabled by default)
//!
//! ## Example
//!
//! ```rust
//! use tinykv::TinyKV;
//!
//! # #[cfg(feature = "std")]
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let mut kv = TinyKV::open("mydata.json")?.with_auto_save();
//!     kv.set("username", "hasan".to_string())?;
//!     kv.set_with_ttl("session_token", "abc123".to_string(), 60)?; // 60 seconds TTL
//!     let user: Option<String> = kv.get("username")?;
//!     println!("User: {:?}", user);
//!     Ok(())
//! }
//!
//! # #[cfg(not(feature = "std"))]
//! # fn main() -> Result<(), tinykv::TinyKVError> {
//! #     let mut kv = TinyKV::new();
//! #     kv.set("username", "hasan".to_string())?;
//! #     let user: Option<String> = kv.get("username")?;
//! #     println!("User: {:?}", user);
//! #     Ok(())
//! # }
//! ```

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
extern crate alloc;

// Module declarations
mod entry;
mod error;
mod store;

// WASM bindings module
#[cfg(feature = "wasm")]
mod wasm;

#[cfg(test)]
mod tests;

// Public exports - only the essential ones from original
pub use error::TinyKVError;
pub use store::TinyKV;

// Re-export WASM types for convenience
#[cfg(feature = "wasm")]
pub use wasm::WebStorageBackend;

// Re-export important traits for users - exactly as in original
#[cfg(feature = "nanoserde")]
pub use nanoserde::{DeJson, SerJson};

#[cfg(all(not(feature = "nanoserde"), feature = "std"))]
pub use serde::{Deserialize, Serialize};
