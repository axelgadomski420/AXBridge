import { auth, audit, body, bool, communityMember, cursor, empty, fail, id, int, json, member, now, oneOf, optionalText, queryInt, rate, text } from "./core.js";

const REACTIONS = ["like", "love", "support", "bravo", "insight", "funny"];
const MEDIA_TYPES = ["text", "image", "video", "audio", "music", "sign"];

export async function profile(request, env, match) {
  const u = await auth(env, request);
  if (!match) {
    if (request.method === "GET") {
      const row = await env.AL_DB.prepare("SELECT p.*, u.persona, u.preferred_lang FROM profiles p JOIN users u ON u.id = p.user_id WHERE p.user_id = ?").bind(u.id).first();
      if (!row) fail(404, "NOT_FOUND", "Profile not created yet");
      return json(env, request, { profile: shapeProfile(row), isMe: true });
    }
    const b = await body(request);
    const handle = text(b.handle, "handle", 3, 32).toLowerCase().replace(/[^a-z0-9_.]/g, "");
    if (handle.length < 3) fail(422, "INVALID_HANDLE", "Handle must have at least 3 letters or digits");
    const languages = Array.isArray(b.languages) ? b.languages.filter((l) => typeof l === "string").slice(0, 10) : ["pl"];
    const accessibility = b.accessibility && typeof b.accessibility === "object" ? b.accessibility : {};
    const exists = await env.AL_DB.prepare("SELECT user_id FROM profiles WHERE handle = ? AND user_id != ?").bind(handle, u.id).first();
    if (exists) fail(409, "HANDLE_TAKEN", "That handle is already used");
    await env.AL_DB.prepare("INSERT INTO profiles(user_id, handle, display_name, bio, languages, accessibility, created_at) VALUES(?,?,?,?,?,?,?) ON CONFLICT(user_id) DO UPDATE SET handle=excluded.handle, display_name=excluded.display_name, bio=excluded.bio, languages=excluded.languages, accessibility=excluded.accessibility").bind(u.id, handle, text(b.displayName, "displayName", 1, 80), optionalText(b.bio, "bio", 500), JSON.stringify(languages), JSON.stringify(accessibility), now()).run();
    await audit(env, u.id, "profile.upsert", "profile", u.id);
    return json(env, request, { handle }, 201);
  }
  const row = await env.AL_DB.prepare("SELECT p.*, u.persona, u.preferred_lang FROM profiles p JOIN users u ON u.id = p.user_id WHERE p.handle = ?").bind(text(match[0], "handle", 3, 32).toLowerCase()).first();
  if (!row) fail(404, "NOT_FOUND", "Profile not found");
  const following = await env.AL_DB.prepare("SELECT 1 AS v FROM follows WHERE follower_id = ? AND followee_id = ?").bind(u.id, row.user_id).first();
  return json(env, request, { profile: shapeProfile(row), isMe: row.user_id === u.id, following: !!following });
}

export async function posts(request, env, match) {
  const u = await auth(env, request);

  if (!match && request.method === "GET") {
    const url = new URL(request.url);
    const before = cursor(url);
    const limit = queryInt(url, "limit", 20, 1, 50);
    const author = optionalText(url.searchParams.get("author"), "author", 32);
    const language = optionalText(url.searchParams.get("lang"), "lang", 8);
    const simpleOnly = url.searchParams.get("simple") === "1";

    let sql = "SELECT p.*, pr.handle, pr.display_name, pr.avatar_key, (SELECT reaction FROM post_reactions r WHERE r.post_id = p.id AND r.user_id = ?) AS my_reaction FROM posts p JOIN profiles pr ON pr.user_id = p.author_id WHERE p.status = 'published' AND p.created_at < ?";
    const binds = [u.id, before];
    if (author) { sql += " AND pr.handle = ?"; binds.push(author.toLowerCase()); }
    if (language) { sql += " AND p.language = ?"; binds.push(language); }
    if (simpleOnly) { sql += " AND p.is_simple = 1"; }
    sql += " ORDER BY p.created_at DESC LIMIT ?";
    binds.push(limit);

    const rows = await env.AL_DB.prepare(sql).bind(...binds).all();
    const today = await env.AL_DB.prepare("SELECT COUNT(*) AS c FROM posts WHERE status = 'published' AND created_at > ?").bind(new Date(Date.now() - 86400000).toISOString()).first();
    return json(env, request, { posts: rows.results.map(shapePost), nextBefore: rows.results.length === limit ? rows.results[rows.results.length - 1].created_at : null, todayCount: today.c });
  }

  if (!match && request.method === "POST") {
    await rate(env, u.id, "post", 30, 3600);
    const b = await body(request, 262144);
    const mediaType = oneOf(b.mediaType, "mediaType", MEDIA_TYPES, "text");
    const pid = id();
    const isVideo = mediaType === "video";
    if (isVideo && !optionalText(b.captions, "captions", 20000) && !optionalText(b.transcript, "transcript", 20000)) {
      fail(422, "CAPTIONS_REQUIRED", "Wideo bez napisów i bez transkrypcji jest niedostępne — dodaj jedno z dwóch");
    }
    if (mediaType === "image" && !optionalText(b.imageAlt, "imageAlt", 2000)) {
      fail(422, "ALT_REQUIRED", "Zdjęcie bez opisu alternatywnego jest niedostępne — dodaj alt");
    }
    if (mediaType === "sign" && !optionalText(b.signGlosses, "signGlosses", 2000)) {
      fail(422, "GLOSS_REQUIRED", "Wpis migowy wymaga zapisu znaków, żeby był czytelny dla osób, które nie znają migu");
    }
    await env.AL_DB.prepare("INSERT INTO posts(id, author_id, body, media_key, media_type, mood, language, captions, transcript, image_alt, sign_glosses, is_simple, created_at) VALUES(?,?,?,?,?,?,?,?,?,?,?,?,?)").bind(pid, u.id, text(b.body, "body", 1, 10000), optionalText(b.mediaKey, "mediaKey", 500), mediaType, optionalText(b.mood, "mood", 40), oneOf(b.language, "language", ["pl", "en", "de", "uk", "es", "fr"], "pl"), optionalText(b.captions, "captions", 20000), optionalText(b.transcript, "transcript", 20000), optionalText(b.imageAlt, "imageAlt", 2000), optionalText(b.signGlosses, "signGlosses", 2000), bool(b.isSimple) ? 1 : 0, now()).run();
    await env.AL_DB.prepare("UPDATE profiles SET post_count = post_count + 1 WHERE user_id = ?").bind(u.id).run();
    await audit(env, u.id, "post.create", "post", pid, { mediaType });
    return json(env, request, { id: pid }, 201);
  }

  const pid = match[0];
  const action = match[1];
  const post = await env.AL_DB.prepare("SELECT * FROM posts WHERE id = ?").bind(pid).first();
  if (!post || post.status !== "published") fail(404, "NOT_FOUND", "Post not found");

  if (action === "reaction" && request.method === "PUT") {
    const b = await body(request);
    const reaction = oneOf(b.reaction, "reaction", REACTIONS);
    await env.AL_DB.prepare("INSERT INTO post_reactions(post_id, user_id, reaction, created_at) VALUES(?,?,?,?) ON CONFLICT(post_id, user_id) DO UPDATE SET reaction=excluded.reaction").bind(pid, u.id, reaction, now()).run();
    await env.AL_DB.prepare("UPDATE posts SET like_count = (SELECT COUNT(*) FROM post_reactions WHERE post_id = ?) WHERE id = ?").bind(pid, pid).run();
    return empty(env, request);
  }
  if (action === "reaction" && request.method === "DELETE") {
    await env.AL_DB.prepare("DELETE FROM post_reactions WHERE post_id = ? AND user_id = ?").bind(pid, u.id).run();
    await env.AL_DB.prepare("UPDATE posts SET like_count = (SELECT COUNT(*) FROM post_reactions WHERE post_id = ?) WHERE id = ?").bind(pid, pid).run();
    return empty(env, request);
  }

  if (action === "comments") {
    if (request.method === "GET") {
      const rows = await env.AL_DB.prepare("SELECT c.*, pr.handle, pr.display_name, pr.avatar_key FROM post_comments c JOIN profiles pr ON pr.user_id = c.author_id WHERE c.post_id = ? AND c.status = 'visible' ORDER BY c.created_at LIMIT 200").bind(pid).all();
      return json(env, request, { comments: rows.results.map(shapeComment) });
    }
    if (request.method === "POST") {
      await rate(env, u.id, "comment", 120, 3600);
      const b = await body(request);
      if (b.parentId) {
        const parent = await env.AL_DB.prepare("SELECT id FROM post_comments WHERE id = ? AND post_id = ?").bind(b.parentId, pid).first();
        if (!parent) fail(422, "INVALID_PARENT", "Parent comment not found");
      }
      const cid = id();
      await env.AL_DB.prepare("INSERT INTO post_comments(id, post_id, author_id, parent_id, body, language, created_at) VALUES(?,?,?,?,?,?,?)").bind(cid, pid, u.id, optionalText(b.parentId, "parentId", 100), text(b.body, "body", 1, 3000), oneOf(b.language, "language", ["pl", "en", "de", "uk", "es", "fr"], "pl"), now()).run();
      await env.AL_DB.prepare("UPDATE posts SET comment_count = (SELECT COUNT(*) FROM post_comments WHERE post_id = ? AND status = 'visible') WHERE id = ?").bind(pid, pid).run();
      if (post.author_id !== u.id) {
        await env.AL_DB.prepare("INSERT INTO notifications(id, user_id, type, resource_id, payload, created_at) VALUES(?,?,?,?,?,?)").bind(id(), post.author_id, "comment", pid, JSON.stringify({ commentId: cid, from: u.publicId }), now()).run();
      }
      return json(env, request, { id: cid }, 201);
    }
  }

  if (action === "views" && request.method === "POST") {
    await env.AL_DB.prepare("INSERT OR IGNORE INTO post_views(post_id, user_id, seen_at) VALUES(?,?,?)").bind(pid, u.id, now()).run();
    return empty(env, request);
  }

  if (action === "report" && request.method === "POST") {
    const b = await body(request);
    const rid = id();
    await env.AL_DB.prepare("INSERT INTO reports(id, reporter_id, resource_type, resource_id, reason, details, status, created_at) VALUES(?,?,'post',?,?,?,'open',?)").bind(rid, u.id, pid, text(b.reason, "reason", 2, 100), optionalText(b.details, "details", 3000), now()).run();
    return json(env, request, { id: rid }, 201);
  }

  if (!action && request.method === "GET") {
    const row = await env.AL_DB.prepare("SELECT p.*, pr.handle, pr.display_name, pr.avatar_key FROM posts p JOIN profiles pr ON pr.user_id = p.author_id WHERE p.id = ?").bind(pid).first();
    return json(env, request, { post: shapePost(row) });
  }

  if (!action && request.method === "DELETE") {
    if (post.author_id !== u.id && !u.isAdmin) fail(403, "FORBIDDEN", "Not allowed");
    await env.AL_DB.prepare("UPDATE posts SET status='deleted', deleted_at=?, body='' WHERE id = ?").bind(now(), pid).run();
    await env.AL_DB.prepare("UPDATE profiles SET post_count = MAX(0, post_count - 1) WHERE user_id = ?").bind(post.author_id).run();
    await audit(env, u.id, "post.delete", "post", pid);
    return empty(env, request);
  }

  fail(405, "METHOD_NOT_ALLOWED", "Method not allowed");
}

export async function follow(request, env, match) {
  const u = await auth(env, request);
  const handle = text(match[0], "handle", 3, 32).toLowerCase();
  const target = await env.AL_DB.prepare("SELECT user_id FROM profiles WHERE handle = ?").bind(handle).first();
  if (!target) fail(404, "NOT_FOUND", "Profile not found");
  if (target.user_id === u.id) fail(422, "VALIDATION", "You cannot follow yourself");

  if (request.method === "PUT") {
    await env.AL_DB.prepare("INSERT OR IGNORE INTO follows(follower_id, followee_id, created_at) VALUES(?,?,?)").bind(u.id, target.user_id, now()).run();
    await env.AL_DB.prepare("UPDATE profiles SET follower_count = (SELECT COUNT(*) FROM follows WHERE followee_id = ?) WHERE user_id = ?").bind(target.user_id, target.user_id).run();
    await env.AL_DB.prepare("UPDATE profiles SET following_count = (SELECT COUNT(*) FROM follows WHERE follower_id = ?) WHERE user_id = ?").bind(u.id, u.id).run();
    return empty(env, request);
  }
  if (request.method === "DELETE") {
    await env.AL_DB.prepare("DELETE FROM follows WHERE follower_id = ? AND followee_id = ?").bind(u.id, target.user_id).run();
    await env.AL_DB.prepare("UPDATE profiles SET follower_count = (SELECT COUNT(*) FROM follows WHERE followee_id = ?) WHERE user_id = ?").bind(target.user_id, target.user_id).run();
    await env.AL_DB.prepare("UPDATE profiles SET following_count = (SELECT COUNT(*) FROM follows WHERE follower_id = ?) WHERE user_id = ?").bind(u.id, u.id).run();
    return empty(env, request);
  }
  fail(405, "METHOD_NOT_ALLOWED", "Method not allowed");
}

function shapeProfile(row) {
  return {
    userId: row.user_id,
    handle: row.handle,
    displayName: row.display_name,
    bio: row.bio,
    avatarKey: row.avatar_key,
    languages: safeParse(row.languages, ["pl"]),
    accessibility: safeParse(row.accessibility, {}),
    persona: row.persona,
    isVerified: !!row.is_verified,
    followerCount: row.follower_count,
    followingCount: row.following_count,
    postCount: row.post_count,
    createdAt: row.created_at,
  };
}

function shapePost(row) {
  return {
    id: row.id,
    author: { handle: row.handle, displayName: row.display_name, avatarKey: row.avatar_key },
    body: row.body,
    mediaKey: row.media_key,
    mediaType: row.media_type,
    mood: row.mood,
    language: row.language,
    captions: row.captions,
    transcript: row.transcript,
    imageAlt: row.image_alt,
    signGlosses: row.sign_glosses,
    isSimple: !!row.is_simple,
    likeCount: row.like_count,
    commentCount: row.comment_count,
    shareCount: row.share_count,
    myReaction: row.my_reaction || null,
    createdAt: row.created_at,
  };
}

function shapeComment(row) {
  return {
    id: row.id,
    postId: row.post_id,
    parentId: row.parent_id,
    author: { handle: row.handle, displayName: row.display_name, avatarKey: row.avatar_key },
    body: row.body,
    language: row.language,
    createdAt: row.created_at,
  };
}

function safeParse(value, fallback) {
  try { return JSON.parse(value); } catch { return fallback; }
}
