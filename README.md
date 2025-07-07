# tinykv
[![Crates.io](https://img.shields.io/crates/v/tinykv.svg)](https://crates.io/crates/tinykv)
[![Downloads](https://img.shields.io/crates/d/tinykv.svg)](https://crates.io/crates/tinykv)

A minimal file-backed key-value store for Rust with no_std support.

## Why I built this

I was working on **Tazı** (named after the Turkish sighthound), a JS/TS test runner and Jest alternative, when I needed simple persistent storage for test configurations and app settings. 

I tried existing solutions:
- sled felt like overkill for storing simple config
- pickledb looked good but seemed unmaintained  
- Rolling my own JSON persistence was getting repetitive

So I built tinykv - the simple KV store I wish existed. Turns out other Rust developers had the same problem.

## Features

- JSON file storage (human-readable, git-friendly)
- Optional TTL (expiration) per key
- Auto-save and backup options
- Atomic writes (no corruption)
- Simple serde integration
- no_std support for embedded systems
- Works in WASM environments

## Feature Flags

- **default**: Uses `serde` for maximum compatibility
- **std**: Enables standard library features (file I/O, TTL)
- **nanoserde**: Uses `nanoserde` for smaller binaries and faster compilation

## Usage

```toml
# Default (std + serde)
tinykv = "0.3"

# Embedded systems (no_std + nanoserde)
tinykv = { version = "0.3", default-features = false, features = ["nanoserde"] }

# Ultra-minimal (pure no_std)
tinykv = { version = "0.3", default-features = false }
```

### Standard usage (with file I/O)

```rust
use tinykv::TinyKV;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut kv = TinyKV::open("settings.json")?
        .with_auto_save();

    kv.set("theme", "dark")?;
    kv.set_with_ttl("session", "abc123", 3600)?; // 1 hour

    let theme: String = kv.get("theme")?.unwrap_or("light".to_string());
    println!("Using {} theme", theme);
    
    Ok(())
}
```

The file looks like this:
```json
{
  "theme": {
    "value": "dark",
    "expires_at": null
  },
  "session": {
    "value": "abc123", 
    "expires_at": 1721234567
  }
}
```

### Embedded/WASM usage (no_std)

```rust
#![no_std]
extern crate alloc;
use tinykv::TinyKV;

fn embedded_main() -> Result<(), tinykv::TinyKVError> {
    let mut kv = TinyKV::new(); // In-memory store
    
    kv.set("device_id", "ESP32_001")?;
    kv.set("sample_rate", "1000")?;
    
    // Serialize to string for flash storage
    let data = kv.to_data()?;
    // flash_write(&data)?;
    
    // Load from serialized data
    let mut kv2 = TinyKV::from_data(&data)?;
    let device_id = kv2.get("device_id");
    
    Ok(())
}
```

## When to use tinykv

**Good for:**
- CLI tool configuration
- Game save files  
- Application settings
- Test data that needs persistence
- Prototyping without database setup
- Embedded systems and IoT devices
- WASM applications

**Not for:**
- High-performance applications
- Complex queries or relationships
- Multi-user concurrent access
- Large datasets

## Platform Support
tinykv works across different environments:

- Desktop applications: Full features with file I/O, TTL, backups
- Embedded systems: Memory-efficient with nanoserde serialization
- WASM projects: Browser-compatible with minimal footprint
- IoT devices: Ultra-minimal string-based storage


## API Documentation

https://docs.rs/tinykv

## License

MIT License - see the full text in the repository.