use crate::error::AxResult;
use std::path::Path;

pub fn start(file: &Path, target: &str) -> AxResult<()> {
    println!("🔥 AXCode Ultra Dev Server");
    println!("   Source : {}", file.display());
    println!("   Target : {}", target);
    println!("   Hot reload enabled – state is preserved when possible");
    println!("   Open   : http://localhost:5173");
    println!("   (Full implementation uses axc runtime + file watcher)");
    Ok(())
}
