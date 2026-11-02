//! Multi-backend code generation

pub mod workers;
pub mod web;

use crate::error::AxResult;
use std::path::Path;

pub fn build(file: &Path, target: &str, out: &Path, release: bool, verbose: bool) -> AxResult<()> {
    std::fs::create_dir_all(out)?;

    match target {
        "web" => web::generate(file, out, release)?,
        "workers" => workers::generate(file, out)?,
        "ios" | "android" => {
            println!("Native {} backend is generated via separate toolchain (SwiftUI / Compose).", target);
            // In full implementation this would emit Xcode / Gradle projects
        }
        _ => {
            return Err(crate::error::AxError::Codegen(format!("Unknown target: {}", target)));
        }
    }

    if verbose {
        println!("Build finished → {}", out.display());
    }
    Ok(())
}
