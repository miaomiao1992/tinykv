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

#[cfg(all(not(feature = "std"), feature = "nanoserde"))]
use alloc::{
    collections::BTreeMap,
    string::{String, ToString},
    vec::Vec,
};

#[cfg(all(not(feature = "std"), not(feature = "nanoserde")))]
use alloc::{
    collections::BTreeMap,
    format,
    string::{String, ToString},
    vec::Vec,
};

#[cfg(feature = "std")]
use std::collections::HashMap;

#[cfg(feature = "std")]
use std::fs;

#[cfg(feature = "std")]
use std::io::{self, ErrorKind};

#[cfg(feature = "std")]
use std::path::{Path, PathBuf};

#[cfg(feature = "std")]
use std::time::{SystemTime, UNIX_EPOCH};

// Conditional imports based on feature flags
#[cfg(feature = "nanoserde")]
use nanoserde::{DeJson, SerJson};

#[cfg(all(not(feature = "nanoserde"), feature = "std"))]
use serde::{Deserialize, Serialize};

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
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for TinyKVError {}

// Entry struct with conditional serialization
#[cfg(feature = "nanoserde")]
#[derive(DeJson, SerJson, Debug, Clone)]
struct Entry {
    value: String, // nanoserde stores as JSON string
    #[nserde(default)]
    expires_at: Option<u64>, // UNIX timestamp (seconds)
}

#[cfg(all(not(feature = "nanoserde"), feature = "std"))]
#[derive(Serialize, Deserialize, Debug, Clone)]
struct Entry {
    value: serde_json::Value,
    #[serde(default)]
    expires_at: Option<u64>, // UNIX timestamp (seconds)
}

// For no_std without nanoserde, we use a simpler approach
#[cfg(all(not(feature = "nanoserde"), not(feature = "std")))]
#[derive(Debug, Clone)]
struct Entry {
    value: String,           // Simple string storage for no_std
    expires_at: Option<u64>, // UNIX timestamp (seconds)
}

/// A simple persistent key-value store with TTL and auto-save.
///
/// Values are stored in JSON format and must implement serialization traits.
/// Uses `serde` by default, or `nanoserde` when the feature flag is enabled.
/// In `no_std` mode, some features like file I/O are not available.
pub struct TinyKV {
    #[cfg(feature = "std")]
    path: PathBuf,
    #[cfg(feature = "std")]
    data: HashMap<String, Entry>,
    #[cfg(not(feature = "std"))]
    data: BTreeMap<String, Entry>,
    auto_save: bool,
    backup_enabled: bool,
}

impl TinyKV {
    /// Open or create a TinyKV store at the given file path.
    /// Only available with `std` feature.
    #[cfg(feature = "std")]
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, TinyKVError> {
        let path_buf = path.as_ref().to_path_buf();
        let data = match fs::read_to_string(&path_buf) {
            Ok(contents) => Self::deserialize_data(&contents)?,
            Err(e) if e.kind() == ErrorKind::NotFound => HashMap::new(),
            Err(e) => return Err(TinyKVError::Io(e)),
        };

        Ok(Self {
            path: path_buf,
            data,
            auto_save: false,
            backup_enabled: false,
        })
    }

    /// Create a new in-memory TinyKV store.
    /// Available in both `std` and `no_std` modes.
    pub fn new() -> Self {
        Self {
            #[cfg(feature = "std")]
            path: PathBuf::new(),
            #[cfg(feature = "std")]
            data: HashMap::new(),
            #[cfg(not(feature = "std"))]
            data: BTreeMap::new(),
            auto_save: false,
            backup_enabled: false,
        }
    }

    /// Create a TinyKV store from serialized data.
    /// Available in both `std` and `no_std` modes.
    pub fn from_data(data: &str) -> Result<Self, TinyKVError> {
        let data = Self::deserialize_data(data)?;
        Ok(Self {
            #[cfg(feature = "std")]
            path: PathBuf::new(),
            data,
            auto_save: false,
            backup_enabled: false,
        })
    }

    /// Serialize the store to a string.
    /// Available in both `std` and `no_std` modes.
    pub fn to_data(&self) -> Result<String, TinyKVError> {
        self.serialize_data()
    }

    /// Enables auto-saving after every set/remove operation.
    /// Only effective with `std` feature.
    pub fn with_auto_save(mut self) -> Self {
        self.auto_save = true;
        self
    }

    /// Enables or disables file backup before saving.
    /// Only effective with `std` feature.
    pub fn with_backup(mut self, enabled: bool) -> Self {
        self.backup_enabled = enabled;
        self
    }

    // Helper method for serialization
    #[cfg(all(not(feature = "nanoserde"), feature = "std"))]
    fn serialize_data(&self) -> Result<String, TinyKVError> {
        serde_json::to_string_pretty(&self.data).map_err(Into::into)
    }

    #[cfg(feature = "nanoserde")]
    fn serialize_data(&self) -> Result<String, TinyKVError> {
        Ok(self.data.serialize_json())
    }

    #[cfg(all(not(feature = "nanoserde"), not(feature = "std")))]
    fn serialize_data(&self) -> Result<String, TinyKVError> {
        // Simple JSON serialization for no_std
        let mut result = String::from("{");
        let mut first = true;

        for (key, entry) in &self.data {
            if !first {
                result.push(',');
            }
            first = false;

            result.push_str(&format!(
                r#""{}":{{"value":"{}","expires_at":{}}}"#,
                key,
                entry.value,
                match entry.expires_at {
                    Some(exp) => exp.to_string(),
                    None => "null".to_string(),
                }
            ));
        }

        result.push('}');
        Ok(result)
    }

    // Helper method for deserialization
    #[cfg(all(not(feature = "nanoserde"), feature = "std"))]
    fn deserialize_data(contents: &str) -> Result<HashMap<String, Entry>, TinyKVError> {
        if contents.trim().is_empty() {
            return Ok(HashMap::new());
        }
        serde_json::from_str(contents)
            .map_err(|e| TinyKVError::Io(io::Error::new(ErrorKind::InvalidData, e)))
    }

    #[cfg(all(feature = "nanoserde", feature = "std"))]
    fn deserialize_data(contents: &str) -> Result<HashMap<String, Entry>, TinyKVError> {
        if contents.trim().is_empty() {
            return Ok(HashMap::new());
        }
        HashMap::<String, Entry>::deserialize_json(contents)
            .map_err(|e| TinyKVError::Serialization(e.to_string()))
    }

    #[cfg(all(feature = "nanoserde", not(feature = "std")))]
    fn deserialize_data(contents: &str) -> Result<BTreeMap<String, Entry>, TinyKVError> {
        if contents.trim().is_empty() {
            return Ok(BTreeMap::new());
        }
        BTreeMap::<String, Entry>::deserialize_json(contents)
            .map_err(|e| TinyKVError::Serialization(e.to_string()))
    }

    #[cfg(all(not(feature = "nanoserde"), not(feature = "std")))]
    fn deserialize_data(_contents: &str) -> Result<BTreeMap<String, Entry>, TinyKVError> {
        // Simple deserialization for no_std (basic implementation)
        // In practice, you'd want a proper JSON parser here
        Err(TinyKVError::NoStdUnsupported(
            "JSON deserialization not implemented for no_std without nanoserde".to_string(),
        ))
    }

    #[cfg(feature = "std")]
    fn current_timestamp() -> Result<u64, TinyKVError> {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| TinyKVError::TimeError)
            .map(|d| d.as_secs())
    }

    #[cfg(not(feature = "std"))]
    #[allow(dead_code)]
    fn current_timestamp() -> Result<u64, TinyKVError> {
        Err(TinyKVError::NoStdUnsupported(
            "System time not available in no_std".to_string(),
        ))
    }

    /// Inserts a key with a value (without expiration).
    #[cfg(all(not(feature = "nanoserde"), feature = "std"))]
    pub fn set<T: Serialize>(&mut self, key: &str, value: T) -> Result<(), TinyKVError> {
        let val = serde_json::to_value(value)?;
        self.data.insert(
            key.to_string(),
            Entry {
                value: val,
                expires_at: None,
            },
        );

        if self.auto_save {
            self.save()?;
        }
        Ok(())
    }

    #[cfg(feature = "nanoserde")]
    pub fn set<T: SerJson>(&mut self, key: &str, value: T) -> Result<(), TinyKVError> {
        let json_str = value.serialize_json();
        self.data.insert(
            key.to_string(),
            Entry {
                value: json_str,
                expires_at: None,
            },
        );

        #[cfg(feature = "std")]
        if self.auto_save {
            self.save()?;
        }
        Ok(())
    }

    #[cfg(all(not(feature = "nanoserde"), not(feature = "std")))]
    pub fn set(&mut self, key: &str, value: &str) -> Result<(), TinyKVError> {
        self.data.insert(
            key.to_string(),
            Entry {
                value: value.to_string(),
                expires_at: None,
            },
        );
        Ok(())
    }

    /// Inserts a key with value and expiration (TTL in seconds).
    #[cfg(all(not(feature = "nanoserde"), feature = "std"))]
    pub fn set_with_ttl<T: Serialize>(
        &mut self,
        key: &str,
        value: T,
        ttl_secs: u64,
    ) -> Result<(), TinyKVError> {
        let val = serde_json::to_value(value)?;
        let expires_at = Some(Self::current_timestamp()? + ttl_secs);

        self.data.insert(
            key.to_string(),
            Entry {
                value: val,
                expires_at,
            },
        );

        if self.auto_save {
            self.save()?;
        }
        Ok(())
    }

    #[cfg(feature = "nanoserde")]
    pub fn set_with_ttl<T: SerJson>(
        &mut self,
        key: &str,
        value: T,
        ttl_secs: u64,
    ) -> Result<(), TinyKVError> {
        let json_str = value.serialize_json();
        let expires_at = Some(Self::current_timestamp()? + ttl_secs);

        self.data.insert(
            key.to_string(),
            Entry {
                value: json_str,
                expires_at,
            },
        );

        #[cfg(feature = "std")]
        if self.auto_save {
            self.save()?;
        }
        Ok(())
    }

    #[cfg(all(not(feature = "nanoserde"), not(feature = "std")))]
    pub fn set_with_ttl(
        &mut self,
        key: &str,
        value: &str,
        _ttl_secs: u64,
    ) -> Result<(), TinyKVError> {
        // TTL not supported in no_std without time
        self.set(key, value)
    }

    /// Retrieves the value for a given key if it exists and hasn't expired.
    #[cfg(all(not(feature = "nanoserde"), feature = "std"))]
    pub fn get<T: for<'de> Deserialize<'de>>(
        &mut self,
        key: &str,
    ) -> Result<Option<T>, TinyKVError> {
        let now = Self::current_timestamp()?;

        if let Some(entry) = self.data.get(key) {
            if let Some(expiry) = entry.expires_at {
                if now > expiry {
                    self.data.remove(key);
                    if self.auto_save {
                        self.save()?;
                    }
                    return Ok(None);
                }
            }

            let value = serde_json::from_value(entry.value.clone())?;
            return Ok(Some(value));
        }

        Ok(None)
    }

    #[cfg(feature = "nanoserde")]
    pub fn get<T: DeJson>(&mut self, key: &str) -> Result<Option<T>, TinyKVError> {
        #[cfg(feature = "std")]
        let now = Self::current_timestamp()?;

        if let Some(entry) = self.data.get(key) {
            #[cfg(feature = "std")]
            if let Some(expiry) = entry.expires_at {
                if now > expiry {
                    self.data.remove(key);
                    if self.auto_save {
                        self.save()?;
                    }
                    return Ok(None);
                }
            }

            let value = T::deserialize_json(&entry.value)
                .map_err(|e| TinyKVError::Serialization(e.to_string()))?;
            return Ok(Some(value));
        }

        Ok(None)
    }

    #[cfg(all(not(feature = "nanoserde"), not(feature = "std")))]
    pub fn get(&self, key: &str) -> Option<String> {
        self.data.get(key).map(|entry| entry.value.clone())
    }

    /// Removes a key from the store.
    pub fn remove(&mut self, key: &str) -> Result<bool, TinyKVError> {
        let removed = self.data.remove(key).is_some();

        #[cfg(feature = "std")]
        if removed && self.auto_save {
            self.save()?;
        }

        Ok(removed)
    }

    /// Checks if the store contains a given key and it's not expired.
    pub fn contains_key(&self, key: &str) -> bool {
        if let Some(_entry) = self.data.get(key) {
            #[cfg(feature = "std")]
            if let Some(expiry) = _entry.expires_at {
                let now = Self::current_timestamp().unwrap_or(0);
                return now <= expiry;
            }
            return true;
        }
        false
    }

    /// Returns a list of all unexpired keys in the store.
    pub fn keys(&self) -> Vec<String> {
        #[cfg(feature = "std")]
        let now = Self::current_timestamp().unwrap_or(0);

        self.data
            .iter()
            .filter(|(_, _entry)| {
                #[cfg(feature = "std")]
                match _entry.expires_at {
                    Some(expiry) => now <= expiry,
                    None => true,
                }
                #[cfg(not(feature = "std"))]
                true
            })
            .map(|(k, _)| k.clone())
            .collect()
    }

    /// Returns number of unexpired entries.
    pub fn len(&self) -> usize {
        #[cfg(feature = "std")]
        let now = Self::current_timestamp().unwrap_or(0);

        self.data
            .iter()
            .filter(|(_, _entry)| {
                #[cfg(feature = "std")]
                match _entry.expires_at {
                    Some(expiry) => now <= expiry,
                    None => true,
                }
                #[cfg(not(feature = "std"))]
                true
            })
            .count()
    }

    /// Returns true if the store is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Save contents to disk. Creates a `.bak` file if backup is enabled.
    /// Only available with `std` feature.
    #[cfg(feature = "std")]
    pub fn save(&self) -> Result<(), TinyKVError> {
        if self.backup_enabled && self.path.exists() {
            let backup_path = self.path.with_extension("bak");
            fs::copy(&self.path, &backup_path)?;
        }

        let json = self.serialize_data()?;
        let temp_path = self.path.with_extension("tmp");
        fs::write(&temp_path, json)?;
        fs::rename(&temp_path, &self.path)?;

        Ok(())
    }

    /// Removes all expired entries from memory.
    /// TTL checking only available with `std` feature.
    pub fn purge_expired(&mut self) -> Result<usize, TinyKVError> {
        if self.data.is_empty() {
            return Ok(0);
        }

        #[cfg(feature = "std")]
        {
            let now = Self::current_timestamp()?;
            let before = self.data.len();
            self.data.retain(|_, entry| match entry.expires_at {
                Some(expiry) => now <= expiry,
                None => true,
            });

            let removed = before - self.data.len();

            if removed > 0 && self.auto_save {
                self.save()?;
            }

            Ok(removed)
        }

        #[cfg(not(feature = "std"))]
        Ok(0) // No TTL support in no_std
    }

    /// Clears all entries from memory.
    pub fn clear(&mut self) -> Result<(), TinyKVError> {
        self.data.clear();

        #[cfg(feature = "std")]
        if self.auto_save {
            self.save()?;
        }

        Ok(())
    }

    /// Reloads the store contents from disk.
    /// Only available with `std` feature.
    #[cfg(feature = "std")]
    pub fn reload(&mut self) -> Result<(), TinyKVError> {
        let data = match fs::read_to_string(&self.path) {
            Ok(contents) => Self::deserialize_data(&contents)?,
            Err(e) if e.kind() == ErrorKind::NotFound => HashMap::new(),
            Err(e) => return Err(TinyKVError::Io(e)),
        };

        self.data = data;
        Ok(())
    }
}

impl Default for TinyKV {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "std")]
impl Drop for TinyKV {
    fn drop(&mut self) {
        if self.auto_save {
            let _ = self.save();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_std_basic_operations() {
        // Test basic operations in no_std mode
        #[cfg(all(not(feature = "nanoserde"), not(feature = "std")))]
        {
            let mut kv = TinyKV::new();
            kv.set("name", "alice").unwrap();
            let name = kv.get("name").unwrap();
            assert_eq!(name, "alice");

            assert!(kv.remove("name").unwrap());
            assert!(!kv.remove("name").unwrap());

            let name = kv.get("name");
            assert!(name.is_none());
        }

        #[cfg(feature = "nanoserde")]
        {
            let mut kv = TinyKV::new();
            kv.set("name", "alice".to_string()).unwrap();
            let name: String = kv.get("name").unwrap().unwrap();
            assert_eq!(name, "alice");
        }

        #[cfg(all(feature = "std", not(feature = "nanoserde")))]
        {
            let mut kv = TinyKV::new();
            kv.set("name", "alice").unwrap();
            let name: String = kv.get("name").unwrap().unwrap();
            assert_eq!(name, "alice");
        }
    }

    #[test]
    fn test_serialization() {
        #[cfg(feature = "nanoserde")]
        {
            let mut kv = TinyKV::new();
            kv.set("test", "value".to_string()).unwrap();
            let serialized = kv.to_data().unwrap();
            assert!(serialized.contains("test"));
            assert!(serialized.contains("value"));
        }

        #[cfg(all(feature = "std", not(feature = "nanoserde")))]
        {
            let mut kv = TinyKV::new();
            kv.set("test", "value").unwrap();
            let serialized = kv.to_data().unwrap();
            assert!(serialized.contains("test"));
            assert!(serialized.contains("value"));
        }

        #[cfg(all(not(feature = "std"), not(feature = "nanoserde")))]
        {
            let mut kv = TinyKV::new();
            kv.set("test", "value").unwrap();
            // Simple JSON serialization should work in pure no_std mode
            let serialized = kv.to_data().unwrap();
            assert!(serialized.contains("test"));
            assert!(serialized.contains("value"));
        }
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_basic_operations() {
        let temp_file = tempfile::NamedTempFile::new().unwrap();
        let mut kv = TinyKV::open(temp_file.path()).unwrap();

        // Test set and get
        kv.set("name", "alice".to_string()).unwrap();
        let name: String = kv.get("name").unwrap().unwrap();
        assert_eq!(name, "alice");

        // Test remove
        assert!(kv.remove("name").unwrap());
        assert!(!kv.remove("name").unwrap());

        let name: Option<String> = kv.get("name").unwrap();
        assert!(name.is_none());
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_ttl() {
        let temp_file = tempfile::NamedTempFile::new().unwrap();
        let mut kv = TinyKV::open(temp_file.path()).unwrap();

        // Set with 1 second TTL
        kv.set_with_ttl("temp", "value".to_string(), 1).unwrap();

        // Should exist immediately
        let val: Option<String> = kv.get("temp").unwrap();
        assert_eq!(val, Some("value".to_string()));

        // Wait for expiry
        std::thread::sleep(std::time::Duration::from_secs(2));

        // Should be expired
        let val: Option<String> = kv.get("temp").unwrap();
        assert!(val.is_none());
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_auto_save() {
        let temp_file = tempfile::NamedTempFile::new().unwrap();
        let temp_path = temp_file.path().to_path_buf();

        {
            let mut kv = TinyKV::open(&temp_path).unwrap().with_auto_save();
            kv.set("key", "value".to_string()).unwrap();
        } // kv dropped here, auto-save should trigger

        // Create new instance to test persistence
        let mut kv2 = TinyKV::open(&temp_path).unwrap();
        let val: String = kv2.get("key").unwrap().unwrap();
        assert_eq!(val, "value");
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_backup() {
        let temp_file = tempfile::NamedTempFile::new().unwrap();
        let temp_path = temp_file.path().to_path_buf();

        // First, create a file with some data
        {
            let mut kv = TinyKV::open(&temp_path).unwrap();
            kv.set("initial", "data".to_string()).unwrap();
            kv.save().unwrap();
        }

        // Now test backup functionality
        {
            let mut kv = TinyKV::open(&temp_path).unwrap().with_backup(true);
            kv.set("new", "data".to_string()).unwrap();
            kv.save().unwrap();
        }

        // Check if backup file was created
        let backup_path = temp_path.with_extension("bak");
        assert!(backup_path.exists());
    }
}
