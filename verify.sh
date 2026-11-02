#!/usr/bin/env bash
# AXBridge — jedna komenda, która sprawdza cały projekt.
#
# Uruchom z katalogu głównego repozytorium:
#   bash verify.sh
#
# Skrypt nie zatrzymuje się na pierwszym błędzie — zbiera pełny raport,
# bo przy pierwszym uruchomieniu chcesz zobaczyć wszystkie problemy naraz.

set -u
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$ROOT"

PASS=0
FAIL=0
SKIP=0
REPORT="verify-report.txt"
: > "$REPORT"

say() { printf '%s\n' "$*" | tee -a "$REPORT"; }
ok()   { PASS=$((PASS+1)); say "  ✓ $1"; }
bad()  { FAIL=$((FAIL+1)); say "  ✗ $1"; }
skip() { SKIP=$((SKIP+1)); say "  – $1 (pominięte)"; }

say "AXBridge — weryfikacja $(date -u '+%Y-%m-%d %H:%M UTC')"
say "Katalog: $ROOT"
say ""

# ---------------------------------------------------------------- 1. NARZĘDZIA
say "[1/5] Dostępne narzędzia"
if command -v cargo >/dev/null 2>&1; then
  ok "cargo $(cargo --version 2>/dev/null | awk '{print $2}')"
  HAVE_CARGO=1
else
  bad "brak cargo — zainstaluj Rust: https://rustup.rs"
  HAVE_CARGO=0
fi
if command -v node >/dev/null 2>&1; then
  ok "node $(node --version)"
  HAVE_NODE=1
else
  bad "brak node — zainstaluj Node 20+"
  HAVE_NODE=0
fi
if command -v npx >/dev/null 2>&1; then
  ok "npx dostępne"
else
  skip "npx niedostępne — deploy Workera pominięty"
fi
say ""

# ------------------------------------------------- 2. KOLIZJE MODUŁÓW W RUST
say "[2/5] Kolizje modułów (Rust nie przyjmuje pliku i katalogu o tej samej nazwie)"
COLLISION=0
for pair in "codegen/web.rs:codegen/web" "codegen/workers.rs:codegen/workers"; do
  FILE="${pair%%:*}"
  DIR="${pair##*:}"
  if [ -f "compiler/src/$FILE" ] && [ -d "compiler/src/$DIR" ]; then
    bad "kolizja: compiler/src/$FILE oraz katalog compiler/src/$DIR"
    say "      napraw: rm -rf compiler/src/$DIR"
    COLLISION=1
  else
    ok "brak kolizji dla $FILE"
  fi
done
if [ -f "compiler/src/a11y_validator.rs" ]; then
  bad "pozostał duplikat compiler/src/a11y_validator.rs (zastąpiony przez a11y/audit.rs)"
  COLLISION=1
else
  ok "brak duplikatu a11y_validator.rs"
fi
if [ -f "compiler/src/index.js" ]; then
  bad "pozostał drugi backend compiler/src/index.js — konflikt bindingów D1/KV/R2"
  COLLISION=1
else
  ok "jeden backend: axbridge-production/worker"
fi
say ""

# ----------------------------------------------------- 3. KOMPILATOR I TESTY
say "[3/5] Kompilator AXCode"
if [ "$HAVE_CARGO" = "1" ] && [ "$COLLISION" = "0" ]; then
  cd compiler
  say "  $ cargo fmt --check"
  if cargo fmt --all -- --check >>"$ROOT/$REPORT" 2>&1; then ok "formatowanie"; else bad "cargo fmt zgłasza zmiany"; fi
  say "  $ cargo clippy"
  if cargo clippy --all-targets -- -D warnings >>"$ROOT/$REPORT" 2>&1; then ok "clippy bez ostrzeżeń"; else bad "clippy zgłasza problemy"; fi
  say "  $ cargo test --all"
  if cargo test --all >>"$ROOT/$REPORT" 2>&1; then ok "testy jednostkowe"; else bad "testy jednostkowe nie przeszły"; fi
  say "  $ cargo build --release"
  if cargo build --release >>"$ROOT/$REPORT" 2>&1; then
    ok "build release"
    AXC="./target/release/axc"
    GOLDEN="../compiler/tests/golden/app.ax"
    say "  $ axc persona deafBlind"
    if $AXC persona deafBlind >>"$ROOT/$REPORT" 2>&1; then ok "plan persony"; else bad "plan persony nie działa"; fi
    say "  $ axc channels"
    if $AXC channels >>"$ROOT/$REPORT" 2>&1; then ok "matryca kanałów"; else bad "matryca kanałów nie działa"; fi
    say "  $ axc audit $GOLDEN"
    if $AXC audit "$GOLDEN" >>"$ROOT/$REPORT" 2>&1; then ok "audyt golden path"; else bad "audyt golden path zgłasza błędy krytyczne"; fi
    for target in web ios android workers; do
      say "  $ axc build --target $target"
      if $AXC build "$GOLDEN" --target "$target" --out "./dist/$target" >>"$ROOT/$REPORT" 2>&1; then
        ok "generowanie $target"
      else
        bad "generowanie $target"
      fi
    done
    say "  $ axc audit ../axbridge-production/app/app.ax"
    if $AXC audit ../axbridge-production/app/app.ax >>"$ROOT/$REPORT" 2>&1; then
      ok "audyt produkcyjnego app.ax"
    else
      bad "produkcyjny app.ax ma krytyczne błędy dostępności (oczekiwane przed przepisaniem na v2)"
    fi
  else
    bad "cargo build --release — zobacz $REPORT"
  fi
  cd "$ROOT"
else
  skip "kompilator (brak cargo albo nierozwiązane kolizje)"
fi
say ""

# ------------------------------------------------------- 4. RUNTIME W NODE
say "[4/5] Runtime klienta (Node 20+)"
if [ "$HAVE_NODE" = "1" ]; then
  for test in crypto morse; do
    say "  $ node compiler/tests/$test.test.js"
    if node "compiler/tests/$test.test.js" >>"$REPORT" 2>&1; then
      ok "test $test"
    else
      bad "test $test — zobacz $REPORT"
    fi
  done
  say "  $ node --check modułów Workera"
  WORKER_OK=1
  for file in axbridge-production/worker/core.js axbridge-production/worker/auth.js axbridge-production/worker/comms.js axbridge-production/worker/social.js axbridge-production/worker/worker.js; do
    if node --check "$file" >>"$REPORT" 2>&1; then :; else bad "składnia $file"; WORKER_OK=0; fi
  done
  [ "$WORKER_OK" = "1" ] && ok "składnia wszystkich modułów Workera"
else
  skip "runtime (brak node)"
fi
say ""

# ---------------------------------------------------------------- 5. PODSUMOWANIE
say "[5/5] Podsumowanie"
say "  zaliczone:   $PASS"
say "  nieudane:    $FAIL"
say "  pominięte:   $SKIP"
say "  pełny log:   $REPORT"
say ""
if [ "$FAIL" = "0" ] && [ "$COLLISION" = "0" ]; then
  say "PROJEKT GOTOWY DO TESTOWANIA"
  say ""
  say "Uruchom aplikację web:"
  say "  cd compiler/dist/web && python3 -m http.server 8787"
  say "  otwórz http://localhost:8787"
  exit 0
fi
say "WYMAGA POPRAWY — napraw pozycje oznaczone ✗ i uruchom verify.sh ponownie."
exit 1
