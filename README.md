# tinykv
[![Crates.io](https://img.shields.io/crates/v/tinykv.svg)](https://crates.io/crates/tinykv)
[![Downloads](https://img.shields.io/crates/d/tinykv.svg)](https://crates.io/crates/tinykv)

A minimal file-backed key-value store for Rust.

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

## Feature Flags

- **default**: Uses `serde` for maximum compatibility
- **nanoserde**: Uses `nanoserde` for smaller binaries and faster compilation

## Usage

```toml
# Default (serde)
tinykv = "0.2"

# Minimal (nanoserde)
tinykv = { version = "0.2", default-features = false, features = ["nanoserde"] }
```

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

## When to use tinykv

**Good for:**
- CLI tool configuration
- Game save files  
- Application settings
- Test data that needs persistence
- Prototyping without database setup

**Not for:**
- High-performance applications
- Complex queries or relationships
- Multi-user concurrent access
- Large datasets

## Comparison

| | tinykv | sled | pickledb |
|---|---|---|---|
| Setup complexity | Zero | Low | Zero |
| File format | Human-readable JSON | Binary | JSON |
| TTL support | Yes | No | No |
| Maintenance | Active | Stalled | Unmaintained |
| Learning curve | Minutes | Hours | Minutes |

## API Documentation

https://docs.rs/tinykv

## License

MIT License - see the full text in the repository.