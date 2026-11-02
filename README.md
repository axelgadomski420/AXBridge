# AXBridge

Komunikator dostępnościowy: tekst, mowa, kod Morse'a, język migowy, muzyka i alarm w jednym modelu
wiadomości. Zbudowany w AXCode — deklaratywnym języku, w którym jeden plik `.ax` opisuje interfejs,
stan, dane, kanały komunikacji, bezpieczeństwo, pracę offline i testy.

**Zasada nadrzędna: AI proponuje, człowiek zatwierdza.** Treść rozpoznana przez model — transkrypcja
mowy, rozpoznanie migu, wygenerowana muzyka — nigdy nie jest wysyłana bez potwierdzenia, chyba że
użytkownik jawnie włączył tryb automatyczny dla konkretnego kanału.

## Szybki start

    bash verify.sh

Skrypt sprawdza cały projekt w pięciu krokach: narzędzia, kolizje modułów, kompilator i testy,
runtime w Node, podsumowanie. Zapisuje pełny log do `verify-report.txt` i mówi wprost, czy projekt
jest gotowy do testowania.

### Kompilator

    cd compiler
    cargo build --release

    # Audyt dostępności bez generowania kodu
    ./target/release/axc audit tests/golden/app.ax

    # Jak aplikacja zachowa się dla osoby głuchoniewidomej
    ./target/release/axc persona deafBlind

    # Pełna matryca kanałów: co wymaga potwierdzenia, co działa offline
    ./target/release/axc channels

    # Zbuduj uruchamialną aplikację web
    ./target/release/axc build tests/golden/app.ax --target web --out ./dist/web
    cd dist/web && python3 -m http.server 8787

Target `web` generuje `ax.app.js` w czystym JavaScripcie — bez React, bez bundlera, bez `npm install`.
Otwierasz `index.html` i aplikacja działa. Target `react` produkuje komponenty React, jeśli chcesz
użyć własnego pipeline'u.

### Backend

    cd axbridge-production/worker
    npx wrangler d1 execute AL_DB --file=./migrations/0001_axbridge_v2.sql
    npx wrangler deploy

## Struktura

    compiler/                          Kompilator AXCode w Rust
    ├── src/
    │   ├── lexer, parser, ast         Składnia i drzewo programu
    │   ├── axir/                      Model przejściowy z rozwiązaną dostępnością
    │   ├── a11y/                      Walidator, plany person, audyt na bramce build
    │   ├── ai/                        Podpowiedzi lokalne, wybór providera
    │   ├── codegen/                   Web (DOM), React, iOS, Android, Workers
    │   ├── message.rs                 Model wielokanałowy + matryca zdolności
    │   ├── semantic/                  Analiza: duplikaty, nawigacja, modele
    │   └── runtime/                   Dev watcher i diagnostyka
    ├── runtime/                       Runtime klienta
    │   ├── crypto.js                  Ratchet E2EE, safety numbers, szyfrowane backupy
    │   ├── morse.js                   Pełna tabela Morse'a z diakrytykami
    │   └── api.js                     Klient HTTP z timeoutem i outbox offline
    └── tests/
        ├── golden/app.ax              Referencyjna ścieżka krytyczna
        ├── crypto.test.js
        └── morse.test.js

    axbridge-production/               Aplikacja produkcyjna
    ├── app/app.ax                     Deklaracja produktu
    ├── worker/                        JEDYNY backend: Cloudflare Workers
    │   ├── core.js                    Błędy, walidacja, ratelimit, audyt, sesje
    │   ├── auth.js                    Konta, logowanie, dostępność, RODO
    │   ├── comms.js                   Rozmowy, koperty, media, Pomoc, providerzy
    │   ├── social.js                  TikTak, społeczności, muzyka, tłumaczenie
    │   ├── worker.js                  Router, kolejka, cron
    │   └── migrations/                Wersjonowane zmiany schematu D1
    └── public/                        Statyczne assety

## Bezpieczeństwo

- Koperty `ciphertext` + `nonce` + `mediaKey`; serwer nie widzi plaintextu.
- Sesje z hashowanymi tokenami, device binding, rotacja refresh tokena.
- Hasła: PBKDF2-SHA256, 210 000 iteracji.
- Tożsamość wyłącznie z zweryfikowanej sesji — nigdy z nagłówka ani z ciała żądania.
- Klucze API providerów zostają w szyfrowanym store lokalnym; backend je odrzuca kodem `API_KEY_FORBIDDEN`.
- Rate limiting, audit log, CORS ograniczony do `ALLOWED_ORIGINS`.

## Dostępność

Włączona w kompilator, nie dodana po fakcie:

- Walidator sprawdza etykiety, rozmiary celów (44 pt), live regions, redukcję ruchu i potwierdzanie treści AI.
- `axc build` zatrzymuje się na krytycznych błędach dostępności; `--allow-a11y-violations` to obejście.
- Każdy kanał nietekstowy musi mieć reprezentację równoważną: napisy, transkrypcję, alt albo opis nastroju.
- Plany person dobierają kanały wejścia i wyjścia: głuchoniewidomy dostaje Morse'a i haptykę,
  osoba z ograniczoną sprawnością ruchową maksymalnie trzy akcje na ekran.

## Wersje

| Komponent | Wersja |
|---|---|
| Kompilator AXCode | 2.0.0 |
| AXBridge | 2.0.0-production |
| API | v1 |

## Licencja

MIT
