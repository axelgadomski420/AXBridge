export class HttpError extends Error {
  constructor(status, code, message, fields = {}) { super(message); this.status = status; this.code = code; this.fields = fields; }
}
export const fail = (status, code, message, fields) => { throw new HttpError(status, code, message, fields); };
export const now = () => new Date().toISOString();
export const id = () => crypto.randomUUID();
export const bytes = (value) => Uint8Array.from(atob(value), c => c.charCodeAt(0));
export const b64 = (value) => btoa(String.fromCharCode(...new Uint8Array(value)));
export const text = (value, name, min = 1, max = 500) => {
  if (typeof value !== "string" || value.trim().length < min || value.length > max) fail(422, "VALIDATION", `Invalid ${name}`, { field: name });
  return value.trim();
};
export const optionalText = (value, name, max = 5000) => value == null || value === "" ? null : text(value, name, 1, max);
export const bool = (value, fallback = false) => value == null ? fallback : value === true || value === 1;
export const int = (value, name, min, max, fallback) => {
  const n = value == null ? fallback : Number(value);
  if (!Number.isInteger(n) || n < min || n > max) fail(422, "VALIDATION", `Invalid ${name}`, { field: name });
  return n;
};
export const oneOf = (value, name, values, fallback) => {
  const v = value == null ? fallback : value;
  if (!values.includes(v)) fail(422, "VALIDATION", `Invalid ${name}`, { field: name });
  return v;
};
export async function body(request, max = 65536) {
  const length = Number(request.headers.get("content-length") || 0);
  if (length > max) fail(413, "PAYLOAD_TOO_LARGE", "Request body too large");
  let data;
  try { data = await request.json(); } catch { fail(400, "INVALID_JSON", "Expected a JSON body"); }
  if (!data || Array.isArray(data) || typeof data !== "object") fail(422, "VALIDATION", "Expected a JSON object");
  return data;
}
export const queryInt = (url, key, fallback, min, max) => int(url.searchParams.get(key), key, min, max, fallback);
export const cursor = (url) => optionalText(url.searchParams.get("before"), "before", 64) || "9999-12-31T23:59:59.999Z";
export function headers(env, request) {
  const origin = request?.headers.get("Origin");
  const allowed = (env.ALLOWED_ORIGINS || "http://localhost:8787").split(",").map(x => x.trim());
  const h = new Headers({ "Content-Type": "application/json; charset=utf-8", "X-Content-Type-Options": "nosniff", "Referrer-Policy": "no-referrer", "Cache-Control": "no-store", "Content-Security-Policy": "default-src 'none'; frame-ancestors 'none'", "Access-Control-Allow-Methods": "GET,POST,PUT,PATCH,DELETE,OPTIONS", "Access-Control-Allow-Headers": "Content-Type,Authorization,X-Device-Id,X-CSRF-Token", "Access-Control-Max-Age": "86400", "Vary": "Origin" });
  if (origin && allowed.includes(origin)) { h.set("Access-Control-Allow-Origin", origin); h.set("Access-Control-Allow-Credentials", "true"); }
  return h;
}
export const json = (env, request, data, status = 200) => new Response(JSON.stringify(data), { status, headers: headers(env, request) });
export const empty = (env, request, status = 204) => new Response(null, { status, headers: headers(env, request) });
export function error(env, request, e) {
  const known = e instanceof HttpError;
  if (!known) console.error("request_failed", e instanceof Error ? e.message : "unknown");
  return json(env, request, { error: { code: known ? e.code : "INTERNAL_ERROR", message: known ? e.message : "Request failed", ...(known ? e.fields : {}) } }, known ? e.status : 500);
}
export async function digest(value) { return b64(await crypto.subtle.digest("SHA-256", new TextEncoder().encode(value))); }
export async function hashPassword(password, salt = crypto.getRandomValues(new Uint8Array(16)), iterations = 210000) {
  const key = await crypto.subtle.importKey("raw", new TextEncoder().encode(password), "PBKDF2", false, ["deriveBits"]);
  const derived = await crypto.subtle.deriveBits({ name: "PBKDF2", hash: "SHA-256", salt, iterations }, key, 256);
  return `pbkdf2$${iterations}$${b64(salt)}$${b64(derived)}`;
}
export async function verifyPassword(password, encoded) {
  const [scheme, count, salt, expected] = String(encoded).split("$");
  if (scheme !== "pbkdf2" || !count || !salt || !expected) return false;
  const candidate = await hashPassword(password, bytes(salt), Number(count));
  const a = new TextEncoder().encode(candidate), b = new TextEncoder().encode(encoded);
  if (a.length !== b.length) return false;
  let diff = 0; for (let i = 0; i < a.length; i++) diff |= a[i] ^ b[i];
  return diff === 0;
}
export async function rate(env, principal, action, limit = 60, window = 60) {
  const bucket = Math.floor(Date.now() / (window * 1000));
  const key = `${principal}:${action}:${bucket}`;
  await env.AL_DB.prepare("INSERT INTO rate_limits(key,count,expires_at) VALUES(?,1,?) ON CONFLICT(key) DO UPDATE SET count=count+1").bind(key, new Date((bucket + 1) * window * 1000).toISOString()).run();
  const row = await env.AL_DB.prepare("SELECT count FROM rate_limits WHERE key=?").bind(key).first();
  if (row.count > limit) fail(429, "RATE_LIMITED", "Too many requests", { retryAfter: window });
}
export async function audit(env, actor, action, resource, resourceId, metadata = {}) {
  await env.AL_DB.prepare("INSERT INTO audit_logs(id,actor_id,action,resource_type,resource_id,metadata,created_at) VALUES(?,?,?,?,?,?,?)").bind(id(), actor || null, action, resource, resourceId || null, JSON.stringify(metadata), now()).run();
}
export function token(request) {
  const value = request.headers.get("authorization") || "";
  if (!value.startsWith("Bearer ")) fail(401, "UNAUTHENTICATED", "Authentication required");
  return value.slice(7);
}
export async function auth(env, request, { admin = false } = {}) {
  await rate(env, request.headers.get("cf-connecting-ip") || "unknown", "auth", 120);
  const hash = await digest(token(request));
  const device = text(request.headers.get("x-device-id"), "device", 8, 128);
  const session = await env.AL_DB.prepare("SELECT s.*,u.public_id,u.email,u.display_name,u.avatar_url,u.bio,u.public_key,u.persona,u.preferred_lang,u.is_admin,u.is_verified,u.profile_completed,u.created_at,u.last_seen FROM sessions s JOIN users u ON u.id=s.user_id WHERE s.access_hash=? AND s.device_id=? AND s.revoked_at IS NULL AND s.access_expires_at>? AND u.deleted_at IS NULL").bind(hash, device, now()).first();
  if (!session) fail(401, "INVALID_SESSION", "Session is invalid or expired");
  if (admin && !session.is_admin) fail(403, "FORBIDDEN", "Administrator permission required");
  return { id: session.user_id, deviceId: device, sessionId: session.id, publicId: session.public_id, email: session.email, displayName: session.display_name, isAdmin: !!session.is_admin };
}
export const member = async (env, conversationId, userId) => {
  const row = await env.AL_DB.prepare("SELECT role FROM conversation_participants WHERE conversation_id=? AND user_id=? AND left_at IS NULL").bind(conversationId, userId).first();
  if (!row) fail(403, "NOT_PARTICIPANT", "You are not a participant of this conversation");
  return row;
};
export const communityMember = async (env, communityId, userId, roles = []) => {
  const row = await env.AL_DB.prepare("SELECT role FROM community_members WHERE community_id=? AND user_id=? AND status='active'").bind(communityId, userId).first();
  if (!row) fail(403, "NOT_MEMBER", "Community membership required");
  if (roles.length && !roles.includes(row.role)) fail(403, "FORBIDDEN", "Insufficient community role");
  return row;
};
export const publicUser = row => ({ id: row.id || row.user_id, publicId: row.public_id, displayName: row.display_name, avatarUrl: row.avatar_url, bio: row.bio, publicKey: row.public_key, persona: row.persona, preferredLang: row.preferred_lang, isVerified: !!row.is_verified, profileCompleted: !!row.profile_completed, createdAt: row.created_at, lastSeen: row.last_seen });
