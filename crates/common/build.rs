use std::path::Path;

fn main() {
    // Tell Cargo to rerun this build script if migrations are modified
    println!("cargo:rerun-if-changed=migrations");
    
    // Also track individual migration files
    let migrations_dir = Path::new("migrations");
    if migrations_dir.exists() {
        for entry in std::fs::read_dir(migrations_dir).unwrap() {
            if let Ok(entry) = entry {
                if let Some(ext) = entry.path().extension() {
                    if ext == "sql" {
                        println!("cargo:rerun-if-changed={}", entry.path().display());
                    }
                }
            }
        }
    }
} 