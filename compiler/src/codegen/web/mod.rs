//! Web backend (Wasm + TypeScript glue) – skeleton

use crate::error::AxResult;
use std::path::Path;

pub fn generate(_ax_file: &Path, out_dir: &Path, _release: bool) -> AxResult<()> {
    std::fs::create_dir_all(out_dir.join("web"))?;

    let index_html = r#"<!DOCTYPE html>
<html lang="pl">
<head>
  <meta charset="UTF-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover" />
  <title>AXBridge Ultra</title>
  <style>
    :root { color-scheme: light dark; }
    body { margin: 0; font-family: Inter, system-ui, sans-serif; background: #0a0a0b; color: #fafafa; }
    #root { min-height: 100dvh; }
  </style>
</head>
<body>
  <div id="root"></div>
  <script type="module" src="./main.js"></script>
</body>
</html>
"#;
    std::fs::write(out_dir.join("web/index.html"), index_html)?;

    let main_js = r#"// AXBridge Ultra – Web runtime glue
// Real implementation loads Wasm core generated from AXIR
console.log("AXBridge Ultra Web runtime ready");
document.getElementById("root").innerHTML = `
  <div style="padding:2rem;text-align:center">
    <h1>AXBridge Ultra</h1>
    <p>Web runtime placeholder – compile with full Wasm backend for production.</p>
  </div>
`;
"#;
    std::fs::write(out_dir.join("web/main.js"), main_js)?;

    println!("✓ Web backend skeleton generated");
    Ok(())
}
