// ============================================================
// AXBridge Backend — Cloudflare Worker
// Konta (D1) · Muzyka (MiniMax Music) · AI Chat z wyszukiwarką
// ============================================================

const SESSION_TTL = 60 * 60 * 24 * 30; // 30 dni

// ---------- pomocnicze ----------
const json = (data, status = 200) =>
  new Response(JSON.stringify(data), {
    status,
    headers: { 'content-type': 'application/json', 'access-control-allow-origin': '*' },
  });

const uid = () => crypto.randomUUID();

async function hashPassword(password, saltHex) {
  const enc = new TextEncoder();
  const salt = saltHex ? hexToBytes(saltHex) : crypto.getRandomValues(new Uint8Array(16));
  const key = await crypto.subtle.importKey('raw', enc.encode(password), 'PBKDF2', false, ['deriveBits']);
  const bits = await crypto.subtle.deriveBits(
    { name: 'PBKDF2', salt, iterations: 120_000, hash: 'SHA-256' },
    key,
    256
  );
  return { hash: bytesToHex(new Uint8Array(bits)), salt: bytesToHex(salt) };
}

const hexToBytes = (hex) => new Uint8Array(hex.match(/.{1,2}/g).map((b) => parseInt(b, 16)));
const bytesToHex = (bytes) => [...bytes].map((b) => b.toString(16).padStart(2, '0')).join('');

async function verifyPassword(password, storedHash, saltHex) {
  const { hash } = await hashPassword(password, saltHex);
  return hash === storedHash;
}

// ---------- sesje ----------
async function createSession(env, userId) {
  const token = uid() + uid();
  await env.SESSIONS.put(`sess:${token}`, userId, { expirationTtl: SESSION_TTL });
  return token;
}
async function getSession(env, request) {
  const auth = request.headers.get('Authorization') || '';
  const token = auth.replace('Bearer ', '').trim();
  if (!token) return null;
  const userId = await env.SESSIONS.get(`sess:${token}`);
  if (!userId) return null;
  return { userId, token };
}
async function destroySession(env, token) {
  if (token) await env.SESSIONS.delete(`sess:${token}`);
}

// ---------- inicjalizacja D1 ----------
async function ensureSchema(env) {
  const sql = `
    CREATE TABLE IF NOT EXISTS users (
      id TEXT PRIMARY KEY,
      email TEXT UNIQUE NOT NULL,
      display_name TEXT NOT NULL,
      password_hash TEXT NOT NULL,
      password_salt TEXT NOT NULL,
      created_at TEXT NOT NULL
    );
    CREATE TABLE IF NOT EXISTS ai_history (
      id TEXT PRIMARY KEY,
      user_id TEXT NOT NULL,
      role TEXT NOT NULL,
      content TEXT NOT NULL,
      sources TEXT,
      created_at TEXT NOT NULL
    );
    CREATE TABLE IF NOT EXISTS songs (
      id TEXT PRIMARY KEY,
      user_id TEXT NOT NULL,
      prompt TEXT NOT NULL,
      lyrics TEXT,
      audio_url TEXT,
      created_at TEXT NOT NULL
    );
    CREATE INDEX IF NOT EXISTS idx_ai_user ON ai_history(user_id, created_at);
    CREATE INDEX IF NOT EXISTS idx_songs_user ON songs(user_id, created_at);
  `;
  // D1 nie ma batch z wieloma statementami w jednym prepare – dzielimy
  for (const stmt of sql.split(';').map((s) => s.trim()).filter(Boolean)) {
    await env.DB.prepare(stmt).run().catch(() => {});
  }
}

// ---------- wyszukiwarka (multi-source) ----------
async function webSearch(query) {
  const results = [];

  // 1. DuckDuckGo Instant Answer
  try {
    const r = await fetch(
      `https://api.duckduckgo.com/?q=${encodeURIComponent(query)}&format=json&no_html=1&skip_disambig=1`
    );
    const d = await r.json();
    if (d.AbstractText) {
      results.push({
        title: d.Heading || 'DuckDuckGo',
        snippet: d.AbstractText,
        url: d.AbstractURL || `https://duckduckgo.com/?q=${encodeURIComponent(query)}`,
      });
    }
    (d.RelatedTopics || []).slice(0, 3).forEach((t) => {
      if (t.Text && t.FirstURL) results.push({ title: t.Text.slice(0, 60), snippet: t.Text, url: t.FirstURL });
    });
  } catch (_) {}

  // 2. Wikipedia PL
  try {
    const r = await fetch(
      `https://pl.wikipedia.org/w/api.php?action=query&list=search&srsearch=${encodeURIComponent(
        query
      )}&format=json&origin=*&srlimit=3`
    );
    const d = await r.json();
    (d.query?.search || []).forEach((x) =>
      results.push({
        title: x.title,
        snippet: x.snippet.replace(/<[^>]+>/g, ''),
        url: `https://pl.wikipedia.org/wiki/${encodeURIComponent(x.title)}`,
      })
    );
  } catch (_) {}

  // 3. Wikipedia EN (fallback)
  if (results.length === 0) {
    try {
      const r = await fetch(
        `https://en.wikipedia.org/w/api.php?action=query&list=search&srsearch=${encodeURIComponent(
          query
        )}&format=json&origin=*&srlimit=3`
      );
      const d = await r.json();
      (d.query?.search || []).forEach((x) =>
        results.push({
          title: x.title,
          snippet: x.snippet.replace(/<[^>]+>/g, ''),
          url: `https://en.wikipedia.org/wiki/${encodeURIComponent(x.title)}`,
        })
      );
    } catch (_) {}
  }

  return results.slice(0, 5);
}

// ============================================================
// GŁÓWNY HANDLER
// ============================================================
export default {
  async fetch(request, env, ctx) {
    const url = new URL(request.url);
    const path = url.pathname;

    if (request.method === 'OPTIONS') {
      return new Response(null, {
        headers: {
          'access-control-allow-origin': '*',
          'access-control-allow-headers': 'Authorization,Content-Type',
          'access-control-allow-methods': 'GET,POST,PUT,DELETE,OPTIONS',
        },
      });
    }

    await ensureSchema(env);

    // ---------------- AUTH ----------------
    if (path === '/api/auth/register' && request.method === 'POST') {
      const { email, password, displayName } = await request.json();
      if (!email || !password || !displayName) return json({ error: 'Brak danych' }, 400);
      if (password.length < 6) return json({ error: 'Hasło min. 6 znaków' }, 400);

      const exists = await env.DB.prepare('SELECT id FROM users WHERE email = ?').bind(email).first();
      if (exists) return json({ error: 'Konto już istnieje' }, 409);

      const { hash, salt } = await hashPassword(password);
      const id = uid();
      await env.DB.prepare(
        'INSERT INTO users (id, email, display_name, password_hash, password_salt, created_at) VALUES (?, ?, ?, ?, ?, ?)'
      )
        .bind(id, email, displayName, hash, salt, new Date().toISOString())
        .run();

      const token = await createSession(env, id);
      return json({ token, user: { id, email, displayName } });
    }

    if (path === '/api/auth/login' && request.method === 'POST') {
      const { email, password } = await request.json();
      const user = await env.DB.prepare('SELECT * FROM users WHERE email = ?').bind(email).first();
      if (!user) return json({ error: 'Nieprawidłowe dane' }, 401);
      const ok = await verifyPassword(password, user.password_hash, user.password_salt);
      if (!ok) return json({ error: 'Nieprawidłowe dane' }, 401);
      const token = await createSession(env, user.id);
      return json({
        token,
        user: { id: user.id, email: user.email, displayName: user.display_name },
      });
    }

    if (path === '/api/auth/me' && request.method === 'GET') {
      const sess = await getSession(env, request);
      if (!sess) return json({ error: 'Brak sesji' }, 401);
      const user = await env.DB.prepare('SELECT id, email, display_name FROM users WHERE id = ?')
        .bind(sess.userId)
        .first();
      if (!user) return json({ error: 'Nie znaleziono' }, 404);
      return json({ user: { id: user.id, email: user.email, displayName: user.display_name } });
    }

    if (path === '/api/auth/logout' && request.method === 'POST') {
      const sess = await getSession(env, request);
      if (sess) await destroySession(env, sess.token);
      return json({ ok: true });
    }

    // ---------------- AI CHAT ----------------
    if (path === '/api/ai/chat' && request.method === 'POST') {
      const sess = await getSession(env, request);
      if (!sess) return json({ error: 'Zaloguj się' }, 401);

      const { message } = await request.json();
      if (!message) return json({ error: 'Brak wiadomości' }, 400);

      const sources = await webSearch(message);

      // Zapisz wiadomość użytkownika
      await env.DB.prepare(
        'INSERT INTO ai_history (id, user_id, role, content, created_at) VALUES (?, ?, ?, ?, ?)'
      )
        .bind(uid(), sess.userId, 'user', message, new Date().toISOString())
        .run();

      // Pobierz ostatnie 6 wiadomości jako kontekst
      const past = await env.DB.prepare(
        'SELECT role, content FROM ai_history WHERE user_id = ? ORDER BY created_at DESC LIMIT 6'
      )
        .bind(sess.userId)
        .all();

      const context = (past.results || [])
        .reverse()
        .map((m) => `${m.role === 'user' ? 'Użytkownik' : 'Asystent'}: ${m.content}`)
        .join('\n');

      let systemPrompt = 'Jesteś pomocnym asystentem AXBridge. Odpowiadaj zwięźle po polsku.';
      if (sources.length > 0) {
        systemPrompt += `\n\nKontekst z wyszukiwarki:\n${sources
          .map((s, i) => `${i + 1}. ${s.title}: ${s.snippet}`)
          .join('\n')}\n\nUżyj tego kontekstu, jeśli jest istotny.`;
      }

      let answer = '';
      try {
        const aiResp = await env.AI.run('@cf/meta/llama-3.1-8b-instruct', {
          messages: [
            { role: 'system', content: systemPrompt },
            { role: 'user', content: `${context}\n\nPytanie: ${message}` },
          ],
          max_tokens: 800,
        });
        answer = aiResp.response || 'Brak odpowiedzi.';
      } catch (e) {
        answer = `⚠️ Błąd AI: ${e.message}. Spróbuj ponownie.`;
      }

      // Zapisz odpowiedź
      await env.DB.prepare(
        'INSERT INTO ai_history (id, user_id, role, content, sources, created_at) VALUES (?, ?, ?, ?, ?, ?)'
      )
        .bind(uid(), sess.userId, 'assistant', answer, JSON.stringify(sources), new Date().toISOString())
        .run();

      return json({ answer, sources });
    }

    if (path === '/api/ai/history' && request.method === 'GET') {
      const sess = await getSession(env, request);
      if (!sess) return json({ error: 'Zaloguj się' }, 401);
      const rows = await env.DB.prepare(
        'SELECT role, content, sources, created_at FROM ai_history WHERE user_id = ? ORDER BY created_at ASC LIMIT 100'
      )
        .bind(sess.userId)
        .all();
      return json({ history: rows.results || [] });
    }

    // ---------------- MUZYKA (MiniMax Music 2.6) ----------------
    if (path === '/api/music/generate' && request.method === 'POST') {
      const sess = await getSession(env, request);
      if (!sess) return json({ error: 'Zaloguj się' }, 401);

      const { prompt, lyrics } = await request.json();
      if (!prompt) return json({ error: 'Podaj opis stylu/mood' }, 400);

      try {
        const audioResp = await env.AI.run('minimax/music-2.6', {
          prompt: prompt.slice(0, 300),
          lyrics: lyrics ? lyrics.slice(0, 3000) : undefined,
        });

        // Model zwraca binarny MP3 – zapisz do R2
        const audioBuffer = audioResp instanceof ArrayBuffer ? audioResp : audioResp.audio;
        if (!audioBuffer) throw new Error('Brak danych audio');

        const key = `songs/${sess.userId}/${uid()}.mp3`;
        await env.AUDIO.put(key, audioBuffer, {
          httpMetadata: { contentType: 'audio/mpeg' },
        });

        // Zapisz metadane w D1
        const songId = uid();
        await env.DB.prepare(
          'INSERT INTO songs (id, user_id, prompt, lyrics, audio_url, created_at) VALUES (?, ?, ?, ?, ?, ?)'
        )
          .bind(songId, sess.userId, prompt, lyrics || '', key, new Date().toISOString())
          .run();

        return json({ ok: true, songId, audioKey: key });
      } catch (e) {
        return json({ error: `Błąd generowania muzyki: ${e.message}` }, 500);
      }
    }

    // Pobieranie wygenerowanego utworu z R2
    if (path.startsWith('/api/music/audio/') && request.method === 'GET') {
      const key = decodeURIComponent(path.replace('/api/music/audio/', ''));
      const obj = await env.AUDIO.get(key);
      if (!obj) return new Response('Not found', { status: 404 });
      return new Response(obj.body, {
        headers: { 'content-type': 'audio/mpeg', 'cache-control': 'public, max-age=86400' },
      });
    }

    if (path === '/api/music/list' && request.method === 'GET') {
      const sess = await getSession(env, request);
      if (!sess) return json({ error: 'Zaloguj się' }, 401);
      const rows = await env.DB.prepare(
        'SELECT id, prompt, audio_url, created_at FROM songs WHERE user_id = ? ORDER BY created_at DESC LIMIT 50'
      )
        .bind(sess.userId)
        .all();
      return json({ songs: rows.results || [] });
    }

    // ---------------- HEALTH ----------------
    if (path === '/api/health') {
      return json({ ok: true, app: 'AXBridge', version: '2.0' });
    }

    // ---------------- ASSETS ----------------
    if (env.ASSETS) return env.ASSETS.fetch(request);
    return new Response('Not found', { status: 404 });
  },
};
