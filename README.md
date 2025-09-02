# TinyKV

A minimal JSON-based key-value store for Rust with TTL support and multi-platform compatibility.

## Overview

TinyKV provides a simple persistent key-value store that works across different Rust environments - from standard desktop applications to embedded systems and WebAssembly. Data is stored in human-readable JSON format with optional automatic expiration.

## Features

- JSON file-based persistence with atomic writes
- TTL (time-to-live) expiration for keys
- Auto-save functionality
- Backup support with .bak files
- Multi-platform: std, no_std, and WebAssembly
- Flexible serialization: serde or nanoserde
- Thread-safe operations

## Installation

Add to your `Cargo.toml`:

```toml
# Default configuration (std + serde)
tinykv = "0.4"

# For embedded systems (no_std + nanoserde)
tinykv = { version = "0.4", default-features = false, features = ["nanoserde"] }

# Minimal configuration (no_std only)
tinykv = { version = "0.4", default-features = false }

# WebAssembly support
tinykv = { version = "0.4", features = ["wasm", "nanoserde"] }
```

## Quick Start

### Basic Usage

```rust
use tinykv::TinyKV;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create or open a store with namespace
    let mut store = TinyKV::open("data.json")?
        .with_auto_save()
        .with_namespace("app1");
    
    // Store some data (automatically prefixed with "app1:")
    store.set("username", "alice")?;
    store.set("count", 42)?;
    
    // Store with expiration (1 hour)
    store.set_with_ttl("session_token", "abc123", 3600)?;
    
    // Retrieve data
    let username: Option<String> = store.get("username")?;
    let count: Option<i32> = store.get("count")?;
    
    // List keys (returns ["username", "count", "session_token"])
    let keys = store.keys();
    
    println!("User: {:?}, Count: {:?}", username, count);
    Ok(())
}
```

### Embedded Systems (no_std)

```rust
#![no_std]
extern crate alloc;
use tinykv::TinyKV;

fn main() -> Result<(), tinykv::TinyKVError> {
    let mut store = TinyKV::new();
    
    store.set("device_id", "ESP32_001")?;
    store.set("config", "production")?;
    
    // Serialize for external storage
    let serialized = store.to_data()?;
    
    // Later, restore from serialized data
    let restored_store = TinyKV::from_data(&serialized)?;
    
    Ok(())
}
```

### WebAssembly

```typescript
import { TinyKVWasm } from 'tinykv';

// Use browser localStorage
const store = TinyKVWasm.openLocalStorage('myapp');
store.set('theme', 'dark');
store.setWithTtl('session', 'abc123', 3600); // 1 hour

// Prefix operations
const userKeys = store.listKeys('user:');     // ['user:123', 'user:456']
const deleted = store.clearPrefix('temp:');   // returns count
```

**Note:** For optimal performance, serve WASM files with `Content-Type: application/wasm`. TinyKV will automatically fallback to slower instantiation if the MIME type is incorrect.

## Data Format

TinyKV stores data in a structured JSON format:

```json
{
  "username": {
    "value": "alice",
    "expires_at": null
  },
  "session_token": {
    "value": "abc123",
    "expires_at": 1725300000
  }
}
```

## Feature Flags

- `std` (default): Enables file I/O, TTL, and standard library features
- `serde` (default): Uses serde for serialization (maximum compatibility)
- `nanoserde`: Uses nanoserde for faster compilation and smaller binaries
- `wasm`: Enables WebAssembly support with localStorage backend

## API Reference

### Core Methods

- `TinyKV::open(path)` - Open or create file-based store
- `TinyKV::new()` - Create in-memory store
- `set(key, value)` - Store a value
- `set_with_ttl(key, value, seconds)` - Store with expiration
- `get(key)` - Retrieve a value
- `remove(key)` - Delete a key
- `contains_key(key)` - Check if key exists
- `keys()` - List all keys
- `list_keys(prefix)` - List keys with prefix
- `clear()` - Remove all entries
- `clear_prefix(prefix)` - Remove entries with prefix
- `save()` - Manually save to disk

### Configuration

- `with_auto_save()` - Enable automatic saving
- `with_backup(enabled)` - Enable/disable backup files  
- `with_namespace(prefix)` - Set key namespace prefix
- `purge_expired()` - Remove expired entries

## Platform Compatibility

| Platform | File I/O | TTL | Auto-save | Serialization |
|----------|----------|-----|-----------|---------------|
| std      | ✓        | ✓   | ✓         | serde/nanoserde |
| no_std   | ✗        | ✗   | ✗         | nanoserde/manual |
| WASM     | localStorage | ✓ | ✓       | nanoserde |

## Use Cases

**Ideal for:**
- Application configuration files
- Game save data
- CLI tool settings
- Test data persistence
- Embedded device configuration
- Browser-based applications
- Rapid prototyping

**Not recommended for:**
- High-performance databases
- Complex relational queries
- Concurrent multi-user access
- Large datasets (>100MB)

## Documentation

Full API documentation is available at [docs.rs/tinykv](https://docs.rs/tinykv).

## Changelog

### Version 0.4.0
- **BREAKING**: WASM `setWithTtl` now accepts `number` instead of `bigint`
- Added namespace support with `with_namespace(prefix)` method
- Added prefix operations: `list_keys(prefix)` and `clear_prefix(prefix)`
- Enhanced WASM bindings with `listKeys()` and `clearPrefix()` methods
- Modernized package.json with `exports` field and `sideEffects: false`
- Improved WASM performance documentation

### Version 0.3.0
- Enhanced no_std support
- Updated documentation and examples
- Improved JSON output formatting

### Version 0.2.0
- Added nanoserde support for minimal binary size
- Improved compilation speed
- Enhanced embedded systems compatibility

### Version 0.1.0
- Initial release
- Core key-value operations with TTL
- File-based JSON persistence
- Auto-save and backup support

## License

MIT License