# AXBridge // AXCode — architektura wykonawcza

## Cel

Jedna deklaracja `.ax` opisuje produkt dostępnościowy: interfejs, stan, dane, kanały komunikacji,
bezpieczeństwo, pracę offline i testy. Kompilator zamienia ją na kod, który przechodzi walidację
dostępności i bezpieczeństwa **przed** uruchomieniem.

## Warstwy

| Warstwa | Katalog | Odpowiedzialność |
|---|---|---|
| Język | `compiler/src/{lexer,parser,ast,axir}` | Składnia, AST, IR |
| Analiza | `compiler/src/semantic`, `compiler/src/a11y` | Typy, nawigacja, dostępność |
| Kod | `compiler/src/codegen` | Web, Swift, Kotlin, Workers |
| Runtime | `compiler/runtime` | Krypto, Morse, API, outbox |
| Silnik lokalny | `compiler/src/ai` | Podpowiedzi offline, wybór providera |
| Backend | `axbridge-production/worker` | Autoryzacja, trwałość, kolejki |

## Zasada nadrzędna

**AI proponuje, człowiek zatwierdza.** Każda treść utworzona przez model — transkrypcja mowy,
rozpoznanie migu, muzyka, dekodowanie — jest kopertą z `requires_confirmation`, dopóki użytkownik
nie włączy trybu automatycznego dla konkretnego kanału. `Payload::requires_confirmation` egzekwuje
to w kodzie, a walidator dostępności zgłasza brak `confirm beforeSend` jako błąd krytyczny.

## Wielokanałowość

Jeden model `Payload` obsługuje tekst, mowę, Morse’a, język migowy, muzykę, obraz, wideo, audio,
lokalizację, alarm i reakcje emocjonalne. Każdy kanał, który nie jest tekstem, musi mieć
**reprezentację równoważną** (`redundant_representation`) — napisy, transkrypcję, alt albo opis nastroju.

## Bezpieczeństwo

- Koperty `ciphertext` + `nonce` + `mediaKey`; serwer nie widzi plaintextu.
- Ratchet z licznikiem chroni przed powtórzeniem i zmianą kolejności.
- Safety number liczony z obu kluczy w porządku sortowanym — obie strony widzą ten sam ciąg.
- Backup szyfrowany hasłem (PBKDF2-SHA256, 210 000 iteracji, AES-GCM).
- Klucze API providerów pozostają w szyfrowanym store lokalnym; backend odrzuca je kodem `API_KEY_FORBIDDEN`.

## Dostępność

`a11y::validate` sprawdza etykiety, rozmiary celów (44 pt), live regions, redukcję ruchu,
redundancję kanałów i potwierdzanie treści AI. Raport rozdziela błędy krytyczne od ostrzeżeń,
a `is_blocking()` decyduje o zatrzymaniu buildu.

## Offline

Outbox przechowuje koperty z kluczem idempotencji, ponawia z wykładniczym backoffem i wznawia
po powrocie sieci. Alarm, Morse i tekst działają bez połączenia; muzyka i wideo czekają w kolejce.
