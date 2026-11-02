/**
 * AXBridge Ultra – Cloudflare Worker (production)
 * Admin: axelgadomski@icloud.com
 * Bindings: AL_DB, AL_KV, AL_BUCKET, AL_QUEUE (optional), AI, ASSETS
 */

export default {
  async fetch(request, env, ctx) {
    const url = new URL(request.url);
    const path = url.pathname;

    if (request.method === "OPTIONS") {
      return new Response(null, { headers: cors() });
    }

    try {
      // Admin hard-guard
      if (path.startsWith("/admin") || path.startsWith("/api/admin")) {
        const email = request.headers.get("X-User-Email") || "";
        if (email !== "axelgadomski@icloud.com") {
          return json({ error: "Admin access denied" }, 403);
        }
      }

      if (path === "/api/health") {
        return json({ status: "ok", service: "axbridge-ultra", version: "1.2.0" });
      }

      if (path === "/api/translate" && request.method === "POST") {
        return await translate(request, env);
      }

      if (path === "/api/music/generate" && request.method === "POST") {
        return await queueMusic(request, env);
      }

      if (path.startsWith("/api/tiktak")) {
        return await tiktak(request, env, path);
      }

      if (path.startsWith("/api/messages")) {
        return await messages(request, env);
      }

      if (path === "/api/upload" && request.method === "POST") {
        return await upload(request, env);
      }

      if (path.startsWith("/api/admin")) {
        return await admin(request, env, path);
      }

      // Static
      if (env.ASSETS) {
        return env.ASSETS.fetch(request);
      }

      return json({ error: "Not found" }, 404);
    } catch (e) {
      console.error(e);
      return json({ error: "Internal Server Error", detail: String(e) }, 500);
    }
  },

  async queue(batch, env) {
    for (const msg of batch.messages) {
      try {
        const { prompt, mood, provider, userId } = msg.body;
        console.log(`[music] generating via ${provider}: ${prompt}`);
        // Tutaj wywołanie Suno / Puter / HF
        // Wynik → R2 + wpis w music_cache
        msg.ack();
      } catch (err) {
        console.error("Queue error", err);
        msg.retry();
      }
    }
  }
};

function cors() {
  return {
    "Access-Control-Allow-Origin": "*",
    "Access-Control-Allow-Methods": "GET,POST,PUT,DELETE,OPTIONS",
    "Access-Control-Allow-Headers": "Content-Type, Authorization, X-User-Email, X-Device-Id",
  };
}

function json(data, status = 200) {
  return new Response(JSON.stringify(data), {
    status,
    headers: { "Content-Type": "application/json", ...cors() },
  });
}

async function translate(request, env) {
  const { text, targetLang = "en", sourceLang = "auto" } = await request.json();
  const key = `tr:${sourceLang}:${targetLang}:${simpleHash(text)}`;
  const cached = await env.AL_KV.get(key);
  if (cached) return json({ original: text, translated: cached, cached: true });

  // Placeholder – podmień na prawdziwy provider (DeepL / Puter / Workers AI)
  const translated = targetLang === "en" ? `[EN] ${text}` : `[PL] ${text}`;
  await env.AL_KV.put(key, translated, { expirationTtl: 604800 });
  return json({ original: text, translated, cached: false });
}

async function queueMusic(request, env) {
  const body = await request.json();
  if (!env.AL_QUEUE) {
    // Kolejka nie jest zbindowana – nie wywalamy requestu, tylko oznaczamy status
    return json({ status: "queued-local", note: "AL_QUEUE not bound; music marked without queue processing" });
  }
  await env.AL_QUEUE.send({
    prompt: body.prompt,
    mood: body.mood || "happy",
    provider: body.provider || "suno",
    userId: body.userId || null,
  });
  return json({ status: "queued" });
}

async function tiktak(request, env, path) {
  if (request.method === "GET") {
    const { results } = await env.AL_DB.prepare(
      `SELECT id, author_id, caption, media_url, media_type, is_funny, likes, comments, created_at
       FROM tiktak_posts ORDER BY created_at DESC LIMIT 40`
    ).all();
    return json({ posts: results || [] });
  }

  if (request.method === "POST") {
    const b = await request.json();
    const id = crypto.randomUUID();
    await env.AL_DB.prepare(
      `INSERT INTO tiktak_posts (id, author_id, caption, media_url, media_type, is_funny, created_at)
       VALUES (?, ?, ?, ?, ?, ?, ?)`
    ).bind(id, b.authorId, b.caption, b.mediaUrl || null, b.mediaType || "video", b.isFunny ? 1 : 0, new Date().toISOString()).run();
    return json({ id, status: "ok" });
  }

  return json({ error: "Method not allowed" }, 405);
}

async function messages(request, env) {
  // Serwer nigdy nie widzi plaintextu – tylko ciphertext
  if (request.method === "POST") {
    const b = await request.json();
    await env.AL_DB.prepare(
      `INSERT INTO messages (id, conversation_id, sender_id, type, ciphertext, nonce, created_at, status)
       VALUES (?, ?, ?, ?, ?, ?, ?, 'sent')`
    ).bind(b.id, b.conversationId, b.senderId, b.type, b.ciphertext, b.nonce || null, new Date().toISOString()).run();
    return json({ status: "ok" });
  }
  return json({ error: "Use client-side E2EE decryption" }, 400);
}

async function upload(request, env) {
  const form = await request.formData();
  const file = form.get("file");
  if (!file) return json({ error: "file required" }, 400);

  const key = `media/${Date.now()}-${crypto.randomUUID()}-${file.name || "file"}`;
  await env.AL_BUCKET.put(key, file.stream(), {
    httpMetadata: { contentType: file.type || "application/octet-stream" },
  });
  return json({ key, url: `https://media.axbridge.app/${key}` });
}

async function admin(request, env, path) {
  if (path === "/api/admin/stats") {
    const users = await env.AL_DB.prepare("SELECT COUNT(*) as c FROM users").first();
    const posts = await env.AL_DB.prepare("SELECT COUNT(*) as c FROM tiktak_posts").first();
    return json({ users: users?.c || 0, tiktakPosts: posts?.c || 0 });
  }
  return json({ error: "Unknown admin route" }, 404);
}

function simpleHash(s) {
  let h = 0;
  for (let i = 0; i < s.length; i++) h = (Math.imul(31, h) + s.charCodeAt(i)) | 0;
  return (h >>> 0).toString(16);
}
