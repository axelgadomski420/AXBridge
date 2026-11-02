# AXBridge Ultra – Production Application

Kompletna produkcyjna aplikacja komunikacyjna z muzyką, tłumaczeniem na żywo, TikTak, społecznościami i pełną dostępnością.

## Struktura katalogów

```
axbridge-production/
├── app/                  ← TUTAJ WKŁADASZ SWÓJ app.ax
│   └── app.ax            ← (Twój plik – skopiuj go tutaj)
├── worker/               ← Cloudflare Worker (wygenerowany + ręcznie dopracowany)
│   ├── worker.js
│   ├── schema.sql
│   └── wrangler.toml     ← Twój istniejący config (już dopasowany)
├── public/               ← Statyczne assety (logo, ikony, fallback)
├── shared/               ← Typy + kontrakty API współdzielone
├── docs/                 ← Dokumentacja
├── scripts/              ← Skrypty pomocnicze
└── .github/workflows/    ← CI
```

## Gdzie umieścić app.ax

```bash
cp /ścieżka/do/twojego/app.ax  ./app/app.ax
```

Lub po prostu:

```
axbridge-production/app/app.ax
```

To jest **jedyny** wymagany plik źródłowy aplikacji.

## Szybki start

```bash
# 1. Kompilator
cd ../compiler
cargo build --release
export PATH="$PATH:$(pwd)/target/release"

# 2. Sprawdzenie
axc check ../axbridge-production/app/app.ax

# 3. Generowanie Workera (jeśli jeszcze nie)
axc workers ../axbridge-production/app/app.ax -o ../axbridge-production/worker

# 4. Deploy Workera
cd ../axbridge-production/worker
npx wrangler d1 execute AL_DB --file=./schema.sql
npx wrangler deploy

# 5. Dev
axc run ../axbridge-production/app/app.ax --target web
```

## Admin

Na stałe ustawiony w kompilatorze i workerze:

**axelgadomski@icloud.com**

Tylko to konto ma dostęp do `/admin` i panelu administracyjnego.

## Główne funkcje

- Tłumaczenie PL ↔ EN na żywo w rozmowach
- Muzyka (Suno v6 + Puter.js + Hugging Face) jako kanał emocji
- TikTak – krótkie zabawne treści
- Persony dostępności (głuchy, niemy, głuchoniemy, niewidomy…)
- E2EE + offline-first
- Wszystkie API w ustawieniach (przewijana lista dostawców)
- Cloudflare D1 + R2 + KV + Queue + AI
