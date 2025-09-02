//! Test modules - exactly from original code

#[cfg(all(not(feature = "std"), feature = "nanoserde"))]
use alloc::string::{String, ToString};

use crate::TinyKV;

#[test]
fn test_namespace_functionality() {
    #[cfg(feature = "std")]
    {
        use tempfile::NamedTempFile;
        let temp_file = NamedTempFile::new().unwrap();
        
        let mut store = TinyKV::open(temp_file.path()).unwrap()
            .with_namespace("app1");
        
        store.set("username", "alice".to_string()).unwrap();
        store.set("count", 42).unwrap();
        
        let keys = store.keys();
        println!("Keys: {:?}", keys);
        
        let username: String = store.get("username").unwrap().unwrap();
        assert_eq!(username, "alice");
        
        let prefix_keys = store.list_keys("user");
        println!("Prefix keys: {:?}", prefix_keys);
        
        // Actual assertions
        assert!(keys.contains(&"username".to_string()));
        assert!(keys.contains(&"count".to_string()));
    }
}

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
