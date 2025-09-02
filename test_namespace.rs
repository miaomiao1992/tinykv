use std::path::Path;

// Include the library directly for testing
include!("src/lib.rs");

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Test namespace functionality
    let mut store = TinyKV::open("test_namespace.json")?
        .with_namespace("app1");
    
    store.set("username", "alice")?;
    store.set("count", 42)?;
    
    println!("Keys: {:?}", store.keys());
    
    let username: Option<String> = store.get("username")?;
    println!("Username: {:?}", username);
    
    // Test prefix operations
    let app_keys = store.list_keys("user");
    println!("Keys with 'user' prefix: {:?}", app_keys);
    
    Ok(())
}