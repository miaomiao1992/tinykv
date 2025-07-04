use std::collections::HashMap;
use std::fs;
use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json;

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

pub struct TinyKV {
    path: PathBuf,
    data: HashMap<String, Entry>,
    auto_save: bool,
    backup_enabled: bool,
}

impl TinyKV {
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

    pub fn with_auto_save(mut self) -> Self {
        self.auto_save = true;
        self
    }

    pub fn with_backup(mut self, enabled: bool) -> Self {
        self.backup_enabled = enabled;
        self
    }

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

    pub fn get<T: for<'de> Deserialize<'de>>(
        &mut self,
        key: &str,
    ) -> Result<Option<T>, TinyKVError> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| TinyKVError::TimeError)?
            .as_secs();

        if let Some(entry) = self.data.get(key) {
            // Check if expired
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

    pub fn remove(&mut self, key: &str) -> Result<bool, TinyKVError> {
        let removed = self.data.remove(key).is_some();
        if removed && self.auto_save {
            self.save()?;
        }
        Ok(removed)
    }

    pub fn contains_key(&self, key: &str) -> bool {
        // Check if key exists and is not expired
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

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn backup_path(&self) -> PathBuf {
        let mut backup = self.path.clone();
        backup.set_extension("bak");
        backup
    }

    pub fn save(&self) -> Result<(), TinyKVError> {
        // Create backup if enabled
        if self.backup_enabled && self.path.exists() {
            let backup_path = self.backup_path();
            fs::copy(&self.path, &backup_path)?;
        }

        let json = serde_json::to_string_pretty(&self.data)?;

        // Atomic write
        let temp_path = self.path.with_extension("tmp");
        fs::write(&temp_path, json)?;
        fs::rename(&temp_path, &self.path)?;

        Ok(())
    }

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

    pub fn clear(&mut self) -> Result<(), TinyKVError> {
        self.data.clear();
        if self.auto_save {
            self.save()?;
        }
        Ok(())
    }

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
    fn drop(&mut self) {
        if self.auto_save {
            let _ = self.save();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_basic_operations() {
        let temp_file = tempfile::NamedTempFile::new().unwrap();
        let mut kv = TinyKV::open(temp_file.path()).unwrap();

        // Test set and get
        kv.set("name", "alice").unwrap();
        let name: String = kv.get("name").unwrap().unwrap();
        assert_eq!(name, "alice");

        // Test remove
        assert!(kv.remove("name").unwrap());
        assert!(!kv.remove("name").unwrap());

        let name: Option<String> = kv.get("name").unwrap();
        assert!(name.is_none());
    }

    #[test]
    fn test_ttl() {
        let temp_file = tempfile::NamedTempFile::new().unwrap();
        let mut kv = TinyKV::open(temp_file.path()).unwrap();

        // Set with 1 second TTL
        kv.set_with_ttl("temp", "value", 1).unwrap();

        // Should exist immediately
        let val: Option<String> = kv.get("temp").unwrap();
        assert_eq!(val, Some("value".to_string()));

        // Wait for expiry
        thread::sleep(Duration::from_secs(2));

        // Should be expired
        let val: Option<String> = kv.get("temp").unwrap();
        assert!(val.is_none());
    }

    #[test]
    fn test_auto_save() {
        let temp_file = tempfile::NamedTempFile::new().unwrap();
        let temp_path = temp_file.path().to_path_buf();

        {
            let mut kv = TinyKV::open(&temp_path).unwrap().with_auto_save();
            kv.set("key", "value").unwrap();
        } // kv dropped here, auto-save should trigger

        // Create new instance to test persistence
        let mut kv2 = TinyKV::open(&temp_path).unwrap();
        let val: String = kv2.get("key").unwrap().unwrap();
        assert_eq!(val, "value");
    }

    #[test]
    fn test_backup() {
        let temp_file = tempfile::NamedTempFile::new().unwrap();
        let temp_path = temp_file.path().to_path_buf();

        // First, create a file with some data
        {
            let mut kv = TinyKV::open(&temp_path).unwrap();
            kv.set("initial", "data").unwrap();
            kv.save().unwrap();
        }

        // Now test backup functionality
        {
            let mut kv = TinyKV::open(&temp_path).unwrap().with_backup(true);
            kv.set("new", "data").unwrap();
            kv.save().unwrap();
        }

        // Check if backup file was created
        let backup_path = temp_path.with_extension("bak");
        assert!(backup_path.exists());
    }
}
