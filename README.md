# tinykv
[![Crates.io](https://img.shields.io/crates/v/tinykv.svg)](https://crates.io/crates/tinykv)
[![Downloads](https://img.shields.io/crates/d/tinykv.svg)](https://crates.io/crates/tinykv)


**🪶 A minimal file-backed key-value store for Rust.**

`tinykv` is a lightweight, `serde`-powered key-value storage engine with optional TTL (time-to-live) support, atomic saving, and human-readable persistence via JSON.

Perfect for config storage, CLI apps, prototyping, testing, and anywhere you need a simple persistent store without setting up a database.

---

## 🏆 Why tinykv?

| Feature | tinykv | sled | pickledb | HashMap + JSON |
|---------|---------|------|----------|----------------|
| Human-readable | ✅ | ❌ | ✅ | ✅ |
| TTL Support | ✅ | ❌ | ❌ | ❌ |
| Actively maintained | ✅ | ⚠️ | ❌ | N/A |
| Learning curve | ✅ Simple | ❌ Complex | ✅ Simple | ❌ Manual |
| Serde integration | ✅ | ❌ | ✅ | ❌ |

---

## ✨ Features

- ✅ JSON file storage (human-readable)
- ✅ `serde`-based serialization
- ✅ Optional TTL (Time-to-Live) per entry
- ✅ Auto-save on every write (optional)
- ✅ Backup file creation (optional)
- ✅ Atomic write safety (`.tmp` + `rename`)
- ✅ Drop-safe saving
- ✅ Fully tested with `tempfile` isolation

---

## 🎯 Use Cases

- **CLI Tools**: Store user preferences and config
- **Desktop Apps**: Save application state and settings  
- **Games**: Simple save game systems
- **Prototyping**: Quick persistent storage without DB setup
- **IoT/Embedded**: Lightweight device configuration
- **Testing**: Mock persistent storage in tests

---

## 🚀 Quick Start

```toml
# Cargo.toml
[dependencies]
tinykv = "0.1"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

```rust
use tinykv::TinyKV;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut kv = TinyKV::open("store.json")?
        .with_auto_save()
        .with_backup(true);

    kv.set("username", "hasan")?;
    kv.set_with_ttl("token", "abc123", 10)?; // expires in 10 seconds

    if let Some(username) = kv.get::<String>("username")? {
        println!("Welcome, {username}");
    }

    kv.save()?; // not needed with auto_save, but explicit is okay
    Ok(())
}
```

## 📦 Example File Output
```json
{
  "username": {
    "value": "hasan",
    "expires_at": null
  },
  "token": {
    "value": "abc123",
    "expires_at": 1721234567
  }
}
```

## 🧪 Running Tests

```bash
cargo test
```

Tests use tempfile to avoid polluting your working directory.

## 📁 Planned (Optional) Features
- purge_on_load() builder option
- Pluggable serialization formats (YAML, Bincode) via feature flags

📚 **[Full API Documentation](https://docs.rs/tinykv)**

## 📜 License
This project is licensed under the MIT License.

```text
MIT License

Copyright (c) 2025 Hasan YILDIZ

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights  
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell      
copies of the Software, and to permit persons to whom the Software is          
furnished to do so, subject to the following conditions:                       

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.                                

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR    
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,      
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE   
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER        
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, 
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE 
SOFTWARE.
```