//! Main TinyKV store implementation - fixed version.

#[cfg(all(not(feature = "std"), not(feature = "wasm")))]
use alloc::collections::BTreeMap;

#[cfg(not(feature = "std"))]
use alloc::{
    format,
    string::{String, ToString},
    vec::Vec,
};

#[cfg(feature = "wasm")]
use wasm_bindgen::{JsValue, prelude::wasm_bindgen};

#[cfg(all(feature = "wasm", not(feature = "std")))]
use alloc::collections::BTreeMap as HashMap;

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

#[cfg(feature = "wasm")]
use crate::wasm;

// Conditional imports based on feature flags
#[cfg(feature = "nanoserde")]
use nanoserde::{DeJson, SerJson};

#[cfg(all(not(feature = "nanoserde"), feature = "std"))]
use serde::{Deserialize, Serialize};

use crate::entry::Entry;
use crate::error::TinyKVError;

/// A simple persistent key-value store with TTL and auto-save.
///
/// Values are stored in JSON format and must implement serialization traits.
/// Uses `serde` by default, or `nanoserde` when the feature flag is enabled.
/// In `no_std` mode, some features like file I/O are not available.
pub struct TinyKV {
    #[cfg(feature = "std")]
    path: PathBuf,
    #[cfg(feature = "wasm")]
    web_prefix: String,
    namespace: String,
    #[cfg(any(feature = "std", feature = "wasm"))]
    data: HashMap<String, Entry>,
    #[cfg(all(not(feature = "std"), not(feature = "wasm")))]
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
            #[cfg(feature = "wasm")]
            web_prefix: String::new(),
            namespace: String::new(),
            data,
            auto_save: false,
            backup_enabled: false,
        })
    }

    /// Create TinyKV store using browser localStorage.
    /// Only available with `wasm` feature.
    #[cfg(feature = "wasm")]
    pub fn open_localstorage(prefix: &str) -> Result<Self, TinyKVError> {
        let mut kv = Self {
            #[cfg(feature = "std")]
            path: PathBuf::new(),
            web_prefix: prefix.to_string(),
            namespace: String::new(),
            data: HashMap::new(),
            auto_save: false,
            backup_enabled: false,
        };

        kv.web_load()?;
        Ok(kv)
    }

    /// Create TinyKV store with automatic backend selection.
    /// Tries localStorage first, falls back to Error.
    /// Only available with `wasm` feature.
    #[cfg(feature = "wasm")]
    pub fn open_web_auto(prefix: &str) -> Result<Self, TinyKVError> {
        // Try localStorage first
        match Self::open_localstorage(prefix) {
            Ok(kv) => Ok(kv),
            Err(_) => Err(TinyKVError::WebStorage(
                "Failed to open localStorage".into(),
            )),
        }
    }

    /// Create a new in-memory TinyKV store.
    /// Available in both `std` and `no_std` modes.
    pub fn new() -> Self {
        Self {
            #[cfg(feature = "std")]
            path: PathBuf::new(),
            #[cfg(feature = "wasm")]
            web_prefix: String::new(),
            namespace: String::new(),
            #[cfg(any(feature = "std", feature = "wasm"))]
            data: HashMap::new(),
            #[cfg(all(not(feature = "std"), not(feature = "wasm")))]
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
            #[cfg(feature = "wasm")]
            web_prefix: String::new(),
            namespace: String::new(),
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

    /// Sets a namespace prefix for all keys.
    /// Keys will be automatically prefixed when stored and accessed.
    pub fn with_namespace(mut self, namespace: &str) -> Self {
        self.namespace = if namespace.is_empty() {
            String::new()
        } else if namespace.ends_with(':') {
            namespace.to_string()
        } else {
            format!("{}:", namespace)
        };
        self
    }

    /// Helper function to add namespace prefix to a key.
    fn namespaced_key(&self, key: &str) -> String {
        if self.namespace.is_empty() {
            key.to_string()
        } else {
            format!("{}{}", self.namespace, key)
        }
    }

    /// Helper function to remove namespace prefix from a key.
    fn strip_namespace(&self, key: &str) -> String {
        if self.namespace.is_empty() {
            key.to_string()
        } else if key.starts_with(&self.namespace) {
            key[self.namespace.len()..].to_string()
        } else {
            key.to_string()
        }
    }

    #[cfg(feature = "wasm")]
    fn web_load(&mut self) -> Result<(), TinyKVError> {
        self.load_from_localstorage()
    }

    #[cfg(feature = "wasm")]
    fn web_save(&self) -> Result<(), TinyKVError> {
        self.save_to_localstorage()
    }

    #[cfg(feature = "wasm")]
    fn load_from_localstorage(&mut self) -> Result<(), TinyKVError> {
        let data_key = format!("{}:data", self.web_prefix);

        if let Some(json_data) = wasm::ls_get_item(&data_key) {
            let data = Self::deserialize_data(&json_data)?;
            self.data = data;
        }

        Ok(())
    }

    #[cfg(feature = "wasm")]
    fn save_to_localstorage(&self) -> Result<(), TinyKVError> {
        let data_key = format!("{}:data", self.web_prefix);
        let json_data = self.serialize_data()?;

        wasm::ls_set_item(&data_key, &json_data);
        Ok(())
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

    #[cfg(all(not(feature = "nanoserde"), not(feature = "std"), feature = "wasm"))]
    fn deserialize_data(_contents: &str) -> Result<HashMap<String, Entry>, TinyKVError> {
        // Simple deserialization for WASM no_std (basic implementation)
        // In practice, you'd want a proper JSON parser here
        Err(TinyKVError::NoStdUnsupported(
            "JSON deserialization not implemented for WASM no_std without nanoserde".to_string(),
        ))
    }

    #[cfg(all(not(feature = "nanoserde"), not(feature = "std"), feature = "wasm"))]
    fn serialize_data(&self) -> Result<String, TinyKVError> {
        // Simple JSON serialization for WASM no_std
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

    #[cfg(all(not(feature = "nanoserde"), not(feature = "std"), feature = "wasm"))]
    pub fn set(&mut self, key: &str, value: &str) -> Result<(), TinyKVError> {
        self.data.insert(
            key.to_string(),
            Entry {
                value: value.to_string(),
                expires_at: None,
            },
        );

        if self.auto_save {
            self.web_save()?;
        }
        Ok(())
    }

    #[cfg(all(not(feature = "nanoserde"), not(feature = "std"), feature = "wasm"))]
    pub fn set_with_ttl(
        &mut self,
        key: &str,
        value: &str,
        ttl_secs: u64,
    ) -> Result<(), TinyKVError> {
        let expires_at = Some(Self::current_timestamp()? + ttl_secs);

        self.data.insert(
            key.to_string(),
            Entry {
                value: value.to_string(),
                expires_at,
            },
        );

        if self.auto_save {
            self.web_save()?;
        }
        Ok(())
    }

    #[cfg(all(not(feature = "nanoserde"), not(feature = "std"), feature = "wasm"))]
    pub fn get(&self, key: &str) -> Option<String> {
        let now = Self::current_timestamp().unwrap_or(0);

        if let Some(entry) = self.data.get(key) {
            if let Some(expiry) = entry.expires_at {
                if now > expiry {
                    return None;
                }
            }
            return Some(entry.value.clone());
        }
        None
    }

    #[cfg(all(
        not(feature = "nanoserde"),
        not(feature = "std"),
        not(feature = "wasm")
    ))]
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

    #[cfg(all(feature = "nanoserde", any(feature = "std", feature = "wasm")))]
    fn deserialize_data(contents: &str) -> Result<HashMap<String, Entry>, TinyKVError> {
        if contents.trim().is_empty() {
            return Ok(HashMap::new());
        }
        HashMap::<String, Entry>::deserialize_json(contents)
            .map_err(|e| TinyKVError::Serialization(e.to_string()))
    }

    #[cfg(all(feature = "nanoserde", not(feature = "std"), not(feature = "wasm")))]
    fn deserialize_data(contents: &str) -> Result<BTreeMap<String, Entry>, TinyKVError> {
        if contents.trim().is_empty() {
            return Ok(BTreeMap::new());
        }
        BTreeMap::<String, Entry>::deserialize_json(contents)
            .map_err(|e| TinyKVError::Serialization(e.to_string()))
    }

    #[cfg(all(
        not(feature = "nanoserde"),
        not(feature = "std"),
        not(feature = "wasm")
    ))]
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

    #[cfg(all(feature = "wasm", not(feature = "std")))]
    fn current_timestamp() -> Result<u64, TinyKVError> {
        Ok(wasm::current_timestamp())
    }

    #[cfg(all(not(feature = "std"), not(feature = "wasm")))]
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
        let namespaced_key = self.namespaced_key(key);
        self.data.insert(
            namespaced_key,
            Entry {
                value: val,
                expires_at: None,
            },
        );

        if self.auto_save {
            #[cfg(feature = "std")]
            self.save()?;
            #[cfg(feature = "wasm")]
            self.web_save()?;
        }
        Ok(())
    }

    #[cfg(feature = "nanoserde")]
    pub fn set<T: SerJson>(&mut self, key: &str, value: T) -> Result<(), TinyKVError> {
        let json_str = value.serialize_json();
        let namespaced_key = self.namespaced_key(key);
        self.data.insert(
            namespaced_key,
            Entry {
                value: json_str,
                expires_at: None,
            },
        );

        if self.auto_save {
            #[cfg(feature = "std")]
            self.save()?;
            #[cfg(feature = "wasm")]
            self.web_save()?;
        }
        Ok(())
    }

    #[cfg(all(
        not(feature = "nanoserde"),
        not(feature = "std"),
        not(feature = "wasm")
    ))]
    pub fn set(&mut self, key: &str, value: &str) -> Result<(), TinyKVError> {
        let namespaced_key = self.namespaced_key(key);
        self.data.insert(
            namespaced_key,
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
        let namespaced_key = self.namespaced_key(key);

        self.data.insert(
            namespaced_key,
            Entry {
                value: val,
                expires_at,
            },
        );

        if self.auto_save {
            #[cfg(feature = "std")]
            self.save()?;
            #[cfg(feature = "wasm")]
            self.web_save()?;
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
        let namespaced_key = self.namespaced_key(key);

        self.data.insert(
            namespaced_key,
            Entry {
                value: json_str,
                expires_at,
            },
        );

        if self.auto_save {
            #[cfg(feature = "std")]
            self.save()?;
            #[cfg(feature = "wasm")]
            self.web_save()?;
        }
        Ok(())
    }

    #[cfg(all(
        not(feature = "nanoserde"),
        not(feature = "std"),
        not(feature = "wasm")
    ))]
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
        let namespaced_key = self.namespaced_key(key);

        if let Some(entry) = self.data.get(&namespaced_key) {
            if let Some(expiry) = entry.expires_at {
                if now > expiry {
                    self.data.remove(&namespaced_key);
                    if self.auto_save {
                        #[cfg(feature = "std")]
                        self.save()?;
                        #[cfg(feature = "wasm")]
                        self.web_save()?;
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
        #[cfg(any(feature = "std", feature = "wasm"))]
        let now = Self::current_timestamp()?;
        let namespaced_key = self.namespaced_key(key);

        if let Some(entry) = self.data.get(&namespaced_key) {
            #[cfg(any(feature = "std", feature = "wasm"))]
            if let Some(expiry) = entry.expires_at {
                if now > expiry {
                    self.data.remove(&namespaced_key);
                    if self.auto_save {
                        #[cfg(feature = "std")]
                        self.save()?;
                        #[cfg(feature = "wasm")]
                        self.web_save()?;
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

    #[cfg(all(
        not(feature = "nanoserde"),
        not(feature = "std"),
        not(feature = "wasm")
    ))]
    pub fn get(&self, key: &str) -> Option<String> {
        let namespaced_key = self.namespaced_key(key);
        self.data.get(&namespaced_key).map(|entry| entry.value.clone())
    }

    /// Removes a key from the store.
    pub fn remove(&mut self, key: &str) -> Result<bool, TinyKVError> {
        let namespaced_key = self.namespaced_key(key);
        let removed = self.data.remove(&namespaced_key).is_some();

        if removed && self.auto_save {
            #[cfg(feature = "std")]
            self.save()?;
            #[cfg(feature = "wasm")]
            self.web_save()?;
        }

        Ok(removed)
    }

    /// Checks if the store contains a given key and it's not expired.
    pub fn contains_key(&self, key: &str) -> bool {
        let namespaced_key = self.namespaced_key(key);
        if let Some(_entry) = self.data.get(&namespaced_key) {
            #[cfg(any(feature = "std", feature = "wasm"))]
            if let Some(expiry) = _entry.expires_at {
                let now = Self::current_timestamp().unwrap_or(0);
                return now <= expiry;
            }
            return true;
        }
        false
    }

    /// Returns a list of all unexpired keys in the store.
    /// If namespace is set, returns keys with namespace prefix stripped.
    pub fn keys(&self) -> Vec<String> {
        #[cfg(any(feature = "std", feature = "wasm"))]
        let now = Self::current_timestamp().unwrap_or(0);

        self.data
            .iter()
            .filter(|(key, _entry)| {
                // If namespace is set, only include keys from this namespace
                if !self.namespace.is_empty() && !key.starts_with(&self.namespace) {
                    return false;
                }
                
                // Check expiration
                #[cfg(any(feature = "std", feature = "wasm"))]
                match _entry.expires_at {
                    Some(expiry) => now <= expiry,
                    None => true,
                }
                #[cfg(all(not(feature = "std"), not(feature = "wasm")))]
                true
            })
            .map(|(k, _)| self.strip_namespace(k))
            .collect()
    }

    /// Returns a list of all unexpired keys that start with the given prefix.
    pub fn list_keys(&self, prefix: &str) -> Vec<String> {
        #[cfg(any(feature = "std", feature = "wasm"))]
        let now = Self::current_timestamp().unwrap_or(0);

        self.data
            .iter()
            .filter(|(key, _entry)| {
                // Check prefix
                if !key.starts_with(prefix) {
                    return false;
                }
                
                // Check expiration
                #[cfg(any(feature = "std", feature = "wasm"))]
                match _entry.expires_at {
                    Some(expiry) => now <= expiry,
                    None => true,
                }
                #[cfg(all(not(feature = "std"), not(feature = "wasm")))]
                true
            })
            .map(|(k, _)| k.clone())
            .collect()
    }

    /// Returns number of unexpired entries.
    pub fn len(&self) -> usize {
        #[cfg(any(feature = "std", feature = "wasm"))]
        let now = Self::current_timestamp().unwrap_or(0);

        self.data
            .iter()
            .filter(|(_, _entry)| {
                #[cfg(any(feature = "std", feature = "wasm"))]
                match _entry.expires_at {
                    Some(expiry) => now <= expiry,
                    None => true,
                }
                #[cfg(all(not(feature = "std"), not(feature = "wasm")))]
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

        #[cfg(any(feature = "std", feature = "wasm"))]
        {
            let now = Self::current_timestamp()?;
            let before = self.data.len();
            self.data.retain(|_, entry| match entry.expires_at {
                Some(expiry) => now <= expiry,
                None => true,
            });

            let removed = before - self.data.len();

            if removed > 0 && self.auto_save {
                #[cfg(feature = "std")]
                self.save()?;
                #[cfg(feature = "wasm")]
                self.web_save()?;
            }

            Ok(removed)
        }

        #[cfg(all(not(feature = "std"), not(feature = "wasm")))]
        Ok(0) // No TTL support in no_std
    }

    /// Clears all entries from memory.
    pub fn clear(&mut self) -> Result<(), TinyKVError> {
        self.data.clear();

        if self.auto_save {
            #[cfg(feature = "std")]
            self.save()?;
            #[cfg(feature = "wasm")]
            self.web_save()?;
        }

        Ok(())
    }

    /// Removes all entries that start with the given prefix.
    pub fn clear_prefix(&mut self, prefix: &str) -> Result<usize, TinyKVError> {
        let before_count = self.data.len();
        
        self.data.retain(|key, _| !key.starts_with(prefix));
        
        let removed_count = before_count - self.data.len();

        if removed_count > 0 && self.auto_save {
            #[cfg(feature = "std")]
            self.save()?;
            #[cfg(feature = "wasm")]
            self.web_save()?;
        }

        Ok(removed_count)
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

impl Drop for TinyKV {
    fn drop(&mut self) {
        if self.auto_save {
            #[cfg(feature = "std")]
            let _ = self.save();
            #[cfg(feature = "wasm")]
            let _ = self.web_save();
        }
    }
}


/// WASM wrapper for TinyKV
#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub struct TinyKVWasm {
    inner: TinyKV,
}

#[cfg(feature = "wasm")]
#[wasm_bindgen]
impl TinyKVWasm {
    #[wasm_bindgen(constructor)]
    pub fn new() -> TinyKVWasm {
        TinyKVWasm {
            inner: TinyKV::new(),
        }
    }

    #[wasm_bindgen(js_name = "openLocalStorage")]
    pub fn open_localstorage(prefix: &str) -> Result<TinyKVWasm, JsValue> {
        TinyKV::open_localstorage(prefix)
            .map(|kv| TinyKVWasm {
                inner: kv.with_auto_save(),
            })
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    #[wasm_bindgen(js_name = "set")]
    pub fn set(&mut self, key: &str, value: &str) -> Result<(), JsValue> {
        self.inner
            .set(key, value.to_string())
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    #[wasm_bindgen(js_name = "get")]
    pub fn get(&mut self, key: &str) -> Option<String> {
        self.inner.get(key).unwrap_or(None)
    }

    #[wasm_bindgen(js_name = "remove")]
    pub fn remove(&mut self, key: &str) -> Result<bool, JsValue> {
        self.inner
            .remove(key)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    #[wasm_bindgen(js_name = "setWithTtl")]
    pub fn set_with_ttl(&mut self, key: &str, value: &str, ttl_secs: f64) -> Result<(), JsValue> {
        self.inner
            .set_with_ttl(key, value.to_string(), ttl_secs as u64)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    #[wasm_bindgen(js_name = "containsKey")]
    pub fn contains_key(&self, key: &str) -> bool {
        self.inner.contains_key(key)
    }

    #[wasm_bindgen(js_name = "clear")]
    pub fn clear(&mut self) -> Result<(), JsValue> {
        self.inner
            .clear()
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    #[wasm_bindgen(js_name = "listKeys")]
    pub fn list_keys(&self, prefix: &str) -> Vec<String> {
        self.inner.list_keys(prefix)
    }

    #[wasm_bindgen(js_name = "clearPrefix")]
    pub fn clear_prefix(&mut self, prefix: &str) -> Result<u32, JsValue> {
        self.inner
            .clear_prefix(prefix)
            .map(|count| count as u32)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }
}
