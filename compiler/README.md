# AXBridge — kompilator AXCode

Kompilator zamienia jeden plik `.ax` na działającą aplikację dla wielu platform.
Dostępność jest sprawdzana **przed** wygenerowaniem kodu: build zatrzymuje się
na krytycznych błędach, a nie po wdrożeniu.

## Instalacja

```bash
cd compiler
cargo build --release
```

Binarka `axc` pojawi się w `compiler/target/release/axc`.

## Komendy

```bash
# Audyt dostępności bez generowania kodu
./target/release/axc audit ../axbridge-production/app/app.ax

# Raport jako JSON (do CI)
./target/release/axc audit ../axbridge-production/app/app.ax --json

# Jak aplikacja zachowa się dla konkretnej osoby
./target/release/axc persona deafBlind
./target/release/axc persona blind
./target/release/axc persona motor

# Pełna matryca kanałów: co wymaga potwierdzenia, co działa offline
./target/release/axc channels

# Zbuduj aplikację web (czysty JavaScript, bez npm)
./target/release/axc build ../axbridge-production/app/app.ax --target web --out ./dist/web

# Zbuduj pozostałe cele
./target/release/axc build ../axbridge-production/app/app.ax --target ios --out ./dist/ios
./target/release/axc build ../axbridge-production/app/app.ax --target android --out ./dist/android
./target/release/axc build ../axbridge-production/app/app.ax --target workers --out ./dist/workers

# Obejście bramki dostępności (tylko do debugowania)
./target/release/axc build app.ax --allow-a11y-violations
```

## Uruchomienie

```bash
cd compiler/dist/web
python3 -m http.server 8787
# otwórz http://localhost:8787
```

Target `web` generuje `ax.app.js` w czystym JavaScripcie — bez React, bez bundlera,
bez `npm install`. Otwierasz `index.html` i aplikacja działa. Target `react` produkuje
komponenty React, jeśli chcesz użyć własnego pipeline'u.

## Jak działa bramka dostępności

Walidator sprawdza dwanaście reguł: etykiety elementów interaktywnych, rozmiary celów
dotykowych (44 pt), live regions, redukcję ruchu, opisy obrazów, napisy do wideo,
redundancję kanałów i potwierdzanie treści AI.

Każde zgłoszenie ma przypisany poziom WCAG i konkretną poprawkę:

```
Accessibility: 2 critical, 1 warning, 0 info
  • [Critical] InteractiveLabel at [0, 3] — Interactive `button` has no accessible label
    fix: Add `accessibility label "…"` or a visible text child.
    wcag: 4.1.2, 2.4.6
```

Build kończy się kodem błędu, gdy `critical > 0`. Ostrzeżenia raportuje, ale nie blokuje.

## Runtime klienta

Katalog `compiler/runtime/` zawiera moduły wykonywane na urządzeniu:

| Plik | Zawartość |
|---|---|
| `crypto.js` | Ratchet z ochroną przed powtórzeniem, safety numbers, szyfrowane backupy |
| `morse.js` | Pełna tabela Morse'a, encoder, decoder, timeline, dźwięk, wibracje, tryb nauki |
| `api.js` | Klient HTTP z timeoutem, ponawianiem, single-flight refresh, outbox offline |

Testy:

```bash
node tests/crypto.test.js
node tests/morse.test.js
```

## Struktura

```
compiler/
├── src/
│   ├── lexer/          Lekser z obsługą wcięć
│   ├── parser/         Parser .ax
│   ├── ast.rs          Drzewo programu
│   ├── axir/           Model przejściowy z rozwiązaną dostępnością
│   ├── a11y/           Walidator, plany person, audyt na bramce build
│   ├── ai/             Podpowiedzi lokalne, wybór providera
│   ├── codegen/        dom.rs (Web), web.rs (React), swift.rs, kotlin.rs, workers.rs
│   ├── semantic/       Duplikaty, nawigacja, modele
│   ├── message.rs      Model wielokanałowy + matryca zdolności
│   └── runtime/        Dev watcher, diagnostyka
├── runtime/            Moduły wykonywane na urządzeniu
└── tests/              Testy runtime i golden path
```

## Golden path

`tests/golden/app.ax` to referencyjna aplikacja pokrywająca ścieżkę krytyczną:
onboarding z wyborem persony, czat z listą `aria-live`, potwierdzenie transkrypcji,
tryb Pomoc oraz ustawienia dostępności. CI buduje z niego wszystkie cztery cele.
