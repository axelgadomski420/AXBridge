# AXBridge Ultra – Architecture

## High-level

```
┌─────────────────────────────────────────────────────────────┐
│                        Client (AXCode)                      │
│  iOS / Android / Web  – single app.ax source                │
│  E2EE · Offline · Persona · Music · Live Translate · TikTak │
└────────────────────────────┬────────────────────────────────┘
                             │ HTTPS + WebSocket (signaling)
                             ▼
┌─────────────────────────────────────────────────────────────┐
│              Cloudflare Worker (al2100beta)                 │
│  worker.js                                                  │
│  ├── AL_DB   (D1)      – users, messages (ciphertext), …    │
│  ├── AL_KV   (KV)      – translation cache, sessions        │
│  ├── AL_BUCKET (R2)    – avatars, music, TikTak media       │
│  ├── AL_QUEUE          – async music generation             │
│  └── AI                – optional on-worker inference       │
└─────────────────────────────────────────────────────────────┘
```

## Security

- All message bodies are end-to-end encrypted on the client.
- Server stores only ciphertext + minimal metadata.
- Admin routes require header `X-User-Email: axelgadomski@icloud.com`.
- API keys for external providers (Suno, HF, DeepL…) live only on the client (encrypted in local store).

## Music flow

1. Client calls `/api/music/generate`
2. Worker enqueues job on `AL_QUEUE`
3. Queue consumer calls Suno / Puter / HF
4. Result stored in R2 + `music_cache` table
5. Client receives notification / polls and plays

## Translation flow

1. Client sends text + target language
2. Worker checks KV cache
3. On miss → external provider or Workers AI
4. Result cached in KV (7 days)
