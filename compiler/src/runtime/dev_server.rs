use crate::error::AxResult;
use std::path::Path;

pub fn start(file: &Path, target: &str) -> AxResult<()> {
    println!("[axc dev] Watching {} for {} (Ctrl+C to stop)", file.display(), target);
    println!("[axc dev] On file change, recompiles to ./dist");
    println!("[axc dev] NOTE: hot reload w tej wersji to prosty rebuild — pełny HMR w planach.");
    let mut last = std::fs::metadata(file)?.modified()?;
    loop {
        std::thread::sleep(std::time::Duration::from_secs(1));
        if let Ok(m) = std::fs::metadata(file) {
            if let Ok(mtime) = m.modified() {
                if mtime != last {
                    println!("[axc dev] Zmiana wykryta → rebuild");
                    let _ = crate::codegen::build(file, target, Path::new("./dist"), false, true);
                    last = mtime;
                }
            }
        }
    }
}
