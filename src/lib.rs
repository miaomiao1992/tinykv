use std::collections::HashMap;
use std::fs;
use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json;

/// Errors that can occur while using the TinyKV store.
#[derive(Debug)]
pub enum TinyKVError {
    Io(io::Error),
    Serialization(serde_json::Error),
    TimeError,
}

impl From<io::Error> for TinyKVError {
    fn from(err: io::Error) -> Self {
        TinyKVError::Io(err)
    }
}

impl From<serde_json::Error> for TinyKVError {
    fn from(err: serde_json::Error) -> Self {
        TinyKVError::Serialization(err)
    }
}

impl std::fmt::Display for TinyKVError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TinyKVError::Io(e) => write!(f, "IO error: {}", e),
            TinyKVError::Serialization(e) => write!(f, "Serialization error: {}", e),
            TinyKVError::TimeError => write!(f, "Time error"),
        }
    }
}

impl std::error::Error for TinyKVError {}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Entry {
    value: serde_json::Value,
    #[serde(default)]
    expires_at: Option<u64>, // UNIX timestamp (seconds)
}

/// A minimal JSON-based key-value store with TTL and auto-save support.
///
/// Stores key-value pairs in a file and supports expiration (TTL), optional auto-saving,
/// and optional backup file creation.
pub struct TinyKV {
    path: PathBuf,
    data: HashMap<String, Entry>,
    auto_save: bool,
    backup_enabled: bool,
}

impl TinyKV {
    /// Opens an existing store or creates a new one at the given path.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, TinyKVError> {
        let path_buf = path.as_ref().to_path_buf();
        let data = match fs::read_to_string(&path_buf) {
            Ok(contents) => serde_json::from_str(&contents)
                .map_err(|e| TinyKVError::Io(io::Error::new(ErrorKind::InvalidData, e)))?,
            Err(e) if e.kind() == ErrorKind::NotFound => HashMap::new(),
            Err(e) => return Err(TinyKVError::Io(e)),
        };

        Ok(TinyKV {
            path: path_buf,
            data,
            auto_save: false,
            backup_enabled: false,
        })
    }

    /// Enables auto-saving on every modification.
    pub fn with_auto_save(mut self) -> Self {
        self.auto_save = true;
        self
    }

    /// Enables or disables backup file creation during save.
    pub fn with_backup(mut self, enabled: bool) -> Self {
        self.backup_enabled = enabled;
        self
    }

    /// Inserts a value into the store without TTL.
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

    /// Inserts a value into the store with a TTL (in seconds).
    pub fn set_with_ttl<T: Serialize>(
        &mut self,
        key: &str,
        value: T,
        ttl_secs: u64,
    ) -> Result<(), TinyKVError> {
        let val = serde_json::to_value(value)?;
        let expires_at = Some(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|_| TinyKVError::TimeError)?
                .as_secs()
                + ttl_secs,
        );

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

    /// Retrieves a value by key if it exists and has not expired.
    pub fn get<T: for<'de> Deserialize<'de>>(
        &mut self,
        key: &str,
    ) -> Result<Option<T>, TinyKVError> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| TinyKVError::TimeError)?
            .as_secs();

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

    /// Removes a key from the store.
    pub fn remove(&mut self, key: &str) -> Result<bool, TinyKVError> {
        let removed = self.data.remove(key).is_some();
        if removed && self.auto_save {
            self.save()?;
        }
        Ok(removed)
    }

    /// Checks if a key exists and is not expired.
    pub fn contains_key(&self, key: &str) -> bool {
        if let Some(entry) = self.data.get(key) {
            if let Some(expiry) = entry.expires_at {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                return now <= expiry;
            }
            return true;
        }
        false
    }

    /// Returns all non-expired keys.
    pub fn keys(&self) -> Vec<String> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        self.data
            .iter()
            .filter(|(_, entry)| match entry.expires_at {
                Some(expiry) => now <= expiry,
                None => true,
            })
            .map(|(k, _)| k.clone())
            .collect()
    }

    /// Returns the count of non-expired entries.
    pub fn len(&self) -> usize {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        self.data
            .iter()
            .filter(|(_, entry)| match entry.expires_at {
                Some(expiry) => now <= expiry,
                None => true,
            })
            .count()
    }

    /// Returns true if there are no non-expired entries.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the backup file path derived from the main path.
    fn backup_path(&self) -> PathBuf {
        let mut backup = self.path.clone();
        backup.set_extension("bak");
        backup
    }

    /// Saves the store to disk in JSON format.
    /// Creates a backup and uses atomic write (via .tmp) if enabled.
    pub fn save(&self) -> Result<(), TinyKVError> {
        if self.backup_enabled && self.path.exists() {
            let backup_path = self.backup_path();
            fs::copy(&self.path, &backup_path)?;
        }

        let json = serde_json::to_string_pretty(&self.data)?;

        let temp_path = self.path.with_extension("tmp");
        fs::write(&temp_path, json)?;
        fs::rename(&temp_path, &self.path)?;

        Ok(())
    }

    /// Removes all expired keys from the store.
    pub fn purge_expired(&mut self) -> Result<usize, TinyKVError> {
        if self.data.is_empty() {
            return Ok(0);
        }

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| TinyKVError::TimeError)?
            .as_secs();

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

    /// Clears all entries from the store.
    pub fn clear(&mut self) -> Result<(), TinyKVError> {
        self.data.clear();
        if self.auto_save {
            self.save()?;
        }
        Ok(())
    }

    /// Reloads the store from disk.
    pub fn reload(&mut self) -> Result<(), TinyKVError> {
        let data = match fs::read_to_string(&self.path) {
            Ok(contents) => serde_json::from_str(&contents)
                .map_err(|e| TinyKVError::Io(io::Error::new(ErrorKind::InvalidData, e)))?,
            Err(e) if e.kind() == ErrorKind::NotFound => HashMap::new(),
            Err(e) => return Err(TinyKVError::Io(e)),
        };

        self.data = data;
        Ok(())
    }
}

impl Drop for TinyKV {
    /// Automatically saves the store if `auto_save` is enabled.
    fn drop(&mut self) {
        if self.auto_save {
            let _ = self.save();
        }
    }
}
