#!/usr/bin/env bash
set -euo pipefail

echo "→ AXBridge Ultra deploy"

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT/worker"

echo "→ Applying D1 schema…"
npx wrangler d1 execute AL_DB --file=./schema.sql

echo "→ Deploying Worker…"
npx wrangler deploy

echo "✓ Done"
echo "  Health: https://al2100beta.<your-subdomain>.workers.dev/api/health"
