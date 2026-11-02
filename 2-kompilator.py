#!/usr/bin/env python3
# ============================================================
# AXCode Ultra Compiler — JEDEN PLIK
# Użycie:
#   python3 2-kompilator.py check 1-app.ax
#   python3 2-kompilator.py build  1-app.ax
#   python3 2-kompilator.py workers 1-app.ax
#   python3 2-kompilator.py run    1-app.ax
# ============================================================

import sys, re, os, json, http.server, socketserver, threading
from pathlib import Path

VERSION = "0.9.0-ultra"
ADMIN = "axelgadomski@icloud.com"

def read(path):
    return Path(path).read_text(encoding="utf-8")

def parse(source):
    """Minimalny parser AXCode — wyciąga meta + ekrany + funkcje + store'y."""
    name = re.search(r'app\s+"([^"]+)"', source)
    version = re.search(r'version\s+"([^"]+)"', source)
    screens = re.findall(r'^screen\s+(\w+)', source, re.M)
    components = re.findall(r'^component\s+(\w+)', source, re.M)
    functions = re.findall(r'^function\s+(\w+)', source, re.M)
    stores = re.findall(r'^store\s+(\w+)', source, re.M)
    return {
        "name": name.group(1) if name else "AXBridge",
        "version": version.group(1) if version else "1.0.0",
        "screens": screens,
        "components": components,
        "functions": functions,
        "stores": stores,
        "lines": source.count("\n") + 1,
    }

def cmd_check(path):
    src = read(path)
    info = parse(src)
    print(f"→ Checking {path}")
    print(f"  App       : {info['name']}")
    print(f"  Version   : {info['version']}")
    print(f"  Lines     : {info['lines']}")
    print(f"  Screens   : {len(info['screens'])}")
    print(f"  Components: {len(info['components'])}")
    print(f"  Functions : {len(info['functions'])}")
    print(f"  Stores    : {len(info['stores'])}")
    if not info["screens"]:
        print("✗ Brak ekranów (screen)")
        sys.exit(1)
    required = ["Onboarding", "Login", "Register", "Conversations", "Chat"]
    for r in required:
        if r not in info["screens"]:
            print(f"  ⚠ Brakuje ekranu: {r}")
    print(f"  Screens   : {info['screens']}")
    print("✓ No errors found")

def make_html(info):
    screens_js = json.dumps(info["screens"], ensure_ascii=False)
    cards = "\n".join(
        f'      <div class="card" onclick="alert(\'{s}\')"><h3>{s}</h3></div>'
        for s in info["screens"]
    )
    return f"""<!DOCTYPE html>
<html lang="pl">
<head>
  <meta charset="UTF-8"/>
  <meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover"/>
  <meta name="theme-color" content="#7C3AED"/>
  <title>{info['name']} {info['version']}</title>
  <style>
    :root {{ color-scheme: dark; --p:#7C3AED; --bg:#0a0a0b; --s:#18181b; --t:#fafafa; --m:#a1a1aa; }}
    * {{ box-sizing:border-box; margin:0; padding:0; }}
    body {{ font-family:Inter,system-ui,sans-serif; background:var(--bg); color:var(--t); min-height:100dvh; }}
    header {{ padding:1rem 1.25rem; display:flex; align-items:center; gap:.75rem; border-bottom:1px solid #27272a; background:var(--s); }}
    h1 {{ font-size:1.25rem; }}
    .badge {{ font-size:.7rem; padding:.2rem .55rem; border-radius:999px; background:#7c3aed33; color:#c4b5fd; }}
    main {{ padding:1.5rem 1.25rem; max-width:640px; margin:0 auto; }}
    .hero {{ text-align:center; padding:2rem 0 1.5rem; }}
    .hero h2 {{ font-size:1.75rem; margin-bottom:.5rem; }}
    .hero p {{ color:var(--m); line-height:1.5; max-width:360px; margin:0 auto 1.5rem; }}
    .btn {{ display:inline-block; padding:.85rem 1.5rem; border-radius:14px; font-weight:600; border:none; cursor:pointer; background:var(--p); color:#fff; }}
    .grid {{ display:grid; gap:.6rem; margin-top:1.5rem; }}
    .card {{ background:var(--s); border:1px solid #27272a; border-radius:12px; padding:.85rem 1rem; cursor:pointer; }}
    .card:hover {{ border-color:var(--p); }}
    .meta {{ color:var(--m); font-size:.85rem; margin-top:1.5rem; }}
    code {{ background:#27272a; padding:.1rem .35rem; border-radius:4px; }}
  </style>
</head>
<body>
  <header>
    <h1>{info['name']}</h1>
    <span class="badge">v{info['version']}</span>
    <span class="badge">AXCode</span>
  </header>
  <main>
    <div class="hero">
      <h2>Komunikator dla każdego</h2>
      <p>Tekst · Mowa · Morse · Migowy · Muzyka · TikTak · Tłumaczenie PL↔EN</p>
      <button class="btn" onclick="document.getElementById('sc').style.display='grid'">Pokaż {len(info['screens'])} ekranów</button>
    </div>
    <div class="grid" id="sc" style="display:none">{cards}</div>
    <p class="meta">Zbudowano z <code>1-app.ax</code> ({info['lines']} linii) · Admin: <code>{ADMIN}</code></p>
    <p class="meta">API: <code id="h">…</code></p>
  </main>
  <script>
    const screens = {screens_js};
    fetch('/api/health').then(r=>r.json()).then(d=>{{
      document.getElementById('h').textContent = d.status + ' · ' + (d.version||'');
    }}).catch(()=>{{ document.getElementById('h').textContent = 'offline'; }});
  </script>
</body>
</html>"""

def make_worker(info):
    html = make_html(info).replace("\\", "\\\\").replace("`", "\\`").replace("${", "\\${")
    return f'''/**
 * AXBridge Worker — wygenerowany przez 2-kompilator.py
 * Admin: {ADMIN}
 */
const HTML = `{html}`;

export default {{
  async fetch(request, env) {{
    const url = new URL(request.url);
    const path = url.pathname;
    if (request.method === "OPTIONS") {{
      return new Response(null, {{ headers: cors() }});
    }}
    if (path === "/api/health") {{
      return json({{ status: "ok", service: "axbridge-ultra", version: "{info['version']}", admin: "{ADMIN}" }});
    }}
    if (path.startsWith("/api/admin")) {{
      const email = request.headers.get("X-User-Email") || "";
      if (email !== "{ADMIN}") return json({{ error: "Admin access denied" }}, 403);
      return json({{ admin: true, email }});
    }}
    if (path === "/api/translate" && request.method === "POST") {{
      const body = await request.json();
      const t = body.targetLang === "en" ? "[EN] " + (body.text||"") : "[PL] " + (body.text||"");
      return json({{ original: body.text, translated: t, cached: false }});
    }}
    if (path === "/api/music/generate" && request.method === "POST") {{
      return json({{ status: "queued" }});
    }}
    return new Response(HTML, {{
      status: 200,
      headers: {{ "Content-Type": "text/html; charset=utf-8", "Cache-Control": "no-cache", ...cors() }},
    }});
  }},
}};

function cors() {{
  return {{
    "Access-Control-Allow-Origin": "*",
    "Access-Control-Allow-Methods": "GET,POST,OPTIONS",
    "Access-Control-Allow-Headers": "Content-Type, Authorization, X-User-Email",
  }};
}}
function json(data, status = 200) {{
  return new Response(JSON.stringify(data), {{
    status,
    headers: {{ "Content-Type": "application/json", ...cors() }},
  }});
}}
'''

def cmd_build(path, out="dist"):
    src = read(path)
    info = parse(src)
    print(f"→ Building {path}")
    outp = Path(out)
    outp.mkdir(parents=True, exist_ok=True)
    html = make_html(info)
    (outp / "index.html").write_text(html, encoding="utf-8")
    print(f"  wrote {outp/'index.html'} ({len(info['screens'])} screens)")
    print(f"✓ Build OK → {outp}")

def cmd_workers(path, out="worker-out"):
    src = read(path)
    info = parse(src)
    print(f"→ Generating Worker from {path}")
    outp = Path(out)
    outp.mkdir(parents=True, exist_ok=True)
    (outp / "worker.js").write_text(make_worker(info), encoding="utf-8")
    schema = """CREATE TABLE IF NOT EXISTS users (
  id TEXT PRIMARY KEY, public_id TEXT UNIQUE, email TEXT,
  display_name TEXT NOT NULL, is_admin INTEGER DEFAULT 0, created_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS messages (
  id TEXT PRIMARY KEY, conversation_id TEXT NOT NULL, sender_id TEXT NOT NULL,
  type TEXT NOT NULL, ciphertext BLOB NOT NULL, created_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS tiktak_posts (
  id TEXT PRIMARY KEY, author_id TEXT NOT NULL, caption TEXT,
  media_url TEXT, is_funny INTEGER DEFAULT 1, created_at TEXT NOT NULL
);
"""
    (outp / "schema.sql").write_text(schema, encoding="utf-8")
    wrangler = f'''name = "2100axlbridge"
main = "worker.js"
compatibility_date = "2026-10-30"
workers_dev = true
'''
    (outp / "wrangler.toml").write_text(wrangler, encoding="utf-8")
    print(f"  wrote worker.js, schema.sql, wrangler.toml")
    print(f"✓ Worker → {outp}")
    print(f"  Deploy:  cd {outp} && npx wrangler deploy")

def cmd_run(path, port=4173):
    cmd_build(path, "dist")
    os.chdir("dist")
    print(f"→ http://127.0.0.1:{port}")
    print("  Ctrl+C to stop")
    with socketserver.TCPServer(("", port), http.server.SimpleHTTPRequestHandler) as httpd:
        httpd.serve_forever()

def main():
    if len(sys.argv) < 2:
        print(f"""AXCode Ultra Compiler (1 plik) {VERSION}

Użycie:
  python3 2-kompilator.py check   1-app.ax
  python3 2-kompilator.py build   1-app.ax
  python3 2-kompilator.py workers 1-app.ax
  python3 2-kompilator.py run     1-app.ax
""")
        return
    cmd = sys.argv[1]
    path = sys.argv[2] if len(sys.argv) > 2 else "1-app.ax"
    if cmd == "check":
        cmd_check(path)
    elif cmd == "build":
        cmd_build(path)
    elif cmd == "workers":
        cmd_workers(path)
    elif cmd == "run":
        cmd_run(path)
    else:
        print(f"Nieznana komenda: {cmd}")
        sys.exit(1)

if __name__ == "__main__":
    main()
