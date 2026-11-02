# AXCode Ultra + AXBridge Production

Kompletny pakiet:

1. **Kompilator AXCode Ultra** (`compiler/`) – Rust
2. **Produkcyjna aplikacja AXBridge** (`axbridge-production/`)

## Szybki start

```bash
# 1. Kompilator
cd compiler
cargo build --release

# 2. Wstaw swój app.ax
cp /ścieżka/do/app.ax  ../axbridge-production/app/app.ax

# 3. Sprawdź
./target/release/axc check ../axbridge-production/app/app.ax

# 4. Generuj / odśwież Workera
./target/release/axc workers ../axbridge-production/app/app.ax -o ../axbridge-production/worker

# 5. Deploy
cd ../axbridge-production/worker
npx wrangler d1 execute AL_DB --file=./schema.sql
npx wrangler deploy
```

## Struktura

```
axcode-ultra/
├── compiler/                     ← Ultra Kompilator (Rust)
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs
│       ├── lexer/
│       ├── parser/
│       ├── semantic/
│       ├── axir/
│       ├── optimizer/
│       ├── codegen/ (web + workers)
│       ├── runtime/
│       └── error.rs
│
└── axbridge-production/          ← Aplikacja produkcyjna
    ├── app/                      ← TUTAJ WKŁADASZ app.ax
    ├── worker/                   ← Cloudflare Worker + schema + wrangler.toml
    ├── public/
    ├── shared/
    ├── docs/
    ├── scripts/
    └── .github/workflows/
```

## Admin

Na stałe: **axelgadomski@icloud.com**
