import { empty, error, fail, headers, json } from "./core.js";
import { accessibility, contacts, deleteAccount, exportData, login, logout, me, refresh, register, updateMe } from "./auth.js";
import { conversations, emergency, media, providers } from "./comms.js";
import { follow, posts, profile } from "./social.js";
import { admin, communities, music, tiktak, translate } from "./social_legacy.js";

const routes = [
  ["POST",/^\/api\/auth\/register$/,(q,e)=>register(q,e)],
  ["POST",/^\/api\/auth\/login$/,(q,e)=>login(q,e)],
  ["POST",/^\/api\/auth\/refresh$/,(q,e)=>refresh(q,e)],
  ["POST",/^\/api\/auth\/logout$/,logout],
  ["GET",/^\/api\/me$/,me],["PATCH",/^\/api\/me$/,updateMe],["DELETE",/^\/api\/me$/,deleteAccount],
  ["GET",/^\/api\/me\/export$/,exportData],["GET",/^\/api\/me\/accessibility$/,accessibility],["PUT",/^\/api\/me\/accessibility$/,accessibility],
  ["GET",/^\/api\/contacts$/,contacts],["POST",/^\/api\/contacts$/,contacts],["PATCH",/^\/api\/contacts\/([^/]+)$/,contacts],["DELETE",/^\/api\/contacts\/([^/]+)$/,contacts],
  ["GET",/^\/api\/conversations$/,conversations],["POST",/^\/api\/conversations$/,conversations],
  ["GET",/^\/api\/conversations\/([^/]+)\/messages$/,conversations],["POST",/^\/api\/conversations\/([^/]+)\/messages$/,conversations],
  ["POST",/^\/api\/conversations\/([^/]+)\/messages\/receipts$/,conversations],
  ["PATCH",/^\/api\/conversations\/([^/]+)\/messages\/([^/]+)$/,conversations],["DELETE",/^\/api\/conversations\/([^/]+)\/messages\/([^/]+)$/,conversations],
  ["PATCH",/^\/api\/conversations\/([^/]+)$/,conversations],["DELETE",/^\/api\/conversations\/([^/]+)$/,conversations],["POST",/^\/api\/conversations\/([^/]+)\/participants$/,conversations],
  ["POST",/^\/api\/media$/,media],["GET",/^\/api\/media\/(.+)$/,media],["DELETE",/^\/api\/media\/(.+)$/,media],
  ["GET",/^\/api\/emergency$/,emergency],["POST",/^\/api\/emergency$/,emergency],["DELETE",/^\/api\/emergency\/([^/]+)$/,emergency],
  ["GET",/^\/api\/providers$/,providers],["POST",/^\/api\/providers$/,providers],["PATCH",/^\/api\/providers\/([^/]+)$/,providers],["DELETE",/^\/api\/providers\/([^/]+)$/,providers],
  ["GET",/^\/api\/me\/profile$/,profile],["PUT",/^\/api\/me\/profile$/,profile],
  ["GET",/^\/api\/profiles\/([A-Za-z0-9_.]+)$/,profile],
  ["PUT",/^\/api\/profiles\/([A-Za-z0-9_.]+)\/follow$/,follow],["DELETE",/^\/api\/profiles\/([A-Za-z0-9_.]+)\/follow$/,follow],
  ["GET",/^\/api\/posts$/,posts],["POST",/^\/api\/posts$/,posts],
  ["GET",/^\/api\/posts\/([^/]+)$/,posts],["DELETE",/^\/api\/posts\/([^/]+)$/,posts],
  ["PUT",/^\/api\/posts\/([^/]+)\/reaction$/,posts],["DELETE",/^\/api\/posts\/([^/]+)\/reaction$/,posts],
  ["GET",/^\/api\/posts\/([^/]+)\/comments$/,posts],["POST",/^\/api\/posts\/([^/]+)\/comments$/,posts],
  ["POST",/^\/api\/posts\/([^/]+)\/views$/,posts],["POST",/^\/api\/posts\/([^/]+)\/report$/,posts],
  ["GET",/^\/api\/tiktak$/,tiktak],["POST",/^\/api\/tiktak$/,tiktak],
  ["POST",/^\/api\/tiktak\/([^/]+)\/(like|comments|report|share)$/,tiktak],["DELETE",/^\/api\/tiktak\/([^/]+)\/like$/,tiktak],["GET",/^\/api\/tiktak\/([^/]+)\/comments$/,tiktak],["DELETE",/^\/api\/tiktak\/([^/]+)\/comments\/([^/]+)$/,tiktak],["DELETE",/^\/api\/tiktak\/([^/]+)$/,tiktak],
  ["GET",/^\/api\/communities$/,communities],["POST",/^\/api\/communities$/,communities],["GET",/^\/api\/communities\/([^/]+)$/,communities],
  ["POST",/^\/api\/communities\/([^/]+)\/(join|leave|invite)$/,communities],
  ["GET",/^\/api\/communities\/([^/]+)\/posts$/,communities],["POST",/^\/api\/communities\/([^/]+)\/posts$/,communities],
  ["POST",/^\/api\/communities\/([^/]+)\/posts\/([^/]+)\/(comments|like|report)$/,communities],["DELETE",/^\/api\/communities\/([^/]+)\/posts\/([^/]+)\/like$/,communities],["GET",/^\/api\/communities\/([^/]+)\/posts\/([^/]+)\/comments$/,communities],["DELETE",/^\/api\/communities\/([^/]+)\/posts\/([^/]+)\/comments\/([^/]+)$/,communities],["DELETE",/^\/api\/communities\/([^/]+)\/posts\/([^/]+)$/,communities],
  ["GET",/^\/api\/music$/,music],["POST",/^\/api\/music$/,music],["DELETE",/^\/api\/music\/([^/]+)$/,music],
  ["POST",/^\/api\/translate$/,translate],
  ["GET",/^\/api\/admin\/(stats|reports|audit)$/,admin],["PATCH",/^\/api\/admin\/(reports)\/([^/]+)$/,admin]
];

export default {
  async fetch(request, env, ctx) {
    if (request.method === "OPTIONS") return new Response(null, { status: 204, headers: headers(env, request) });
    try {
      const path = new URL(request.url).pathname;
      if (path === "/api/health") return json(env, request, { status: "ok", service: "axbridge", version: "3.0.0", apiVersion: "v1" });
      for (const [method, pattern, handler] of routes) {
        const match = path.match(pattern);
        if (match && method === request.method) return await handler(request, env, match.slice(1));
      }
      if (path.startsWith("/api/")) fail(404, "NOT_FOUND", "Endpoint not found");
      if (env.ASSETS) return env.ASSETS.fetch(request);
      fail(404, "NOT_FOUND", "Not found");
    } catch (e) {
      return error(env, request, e);
    }
  },
  async queue(batch, env) {
    for (const message of batch.messages) {
      try {
        if (message.body.type !== "emergency") throw new Error("Unsupported queue message");
        const { eventId, recipients } = message.body;
        const event = await env.AL_DB.prepare("SELECT * FROM emergency_events WHERE id=? AND status='active'").bind(eventId).first();
        if (event) for (const userId of recipients) await env.AL_DB.prepare("INSERT INTO notifications(id,user_id,type,resource_id,payload,created_at) VALUES(?,?,'emergency',?,?,?)").bind(crypto.randomUUID(), userId, eventId, JSON.stringify({ expiresAt: event.expires_at }), new Date().toISOString()).run();
        message.ack();
      } catch (e) {
        message.retry({ delaySeconds: 30 });
      }
    }
  },
  async scheduled(event, env, ctx) {
    ctx.waitUntil(Promise.all([
      env.AL_DB.prepare("DELETE FROM sessions WHERE refresh_expires_at<? OR revoked_at<?").bind(new Date().toISOString(), new Date(Date.now() - 2592000000).toISOString()).run(),
      env.AL_DB.prepare("UPDATE emergency_events SET status='expired',location_shared=0,latitude=NULL,longitude=NULL WHERE status='active' AND expires_at<?").bind(new Date().toISOString()).run(),
      env.AL_DB.prepare("DELETE FROM rate_limits WHERE expires_at<?").bind(new Date().toISOString()).run(),
      env.AL_DB.prepare("UPDATE posts SET status='archived' WHERE status='published' AND created_at < ?").bind(new Date(Date.now() - 7776000000).toISOString()).run()
    ]));
  }
};
