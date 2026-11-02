-- AXBridge Ultra – D1 production schema
-- Run: npx wrangler d1 execute AL_DB --file=./schema.sql

PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS users (
  id            TEXT PRIMARY KEY,
  public_id     TEXT UNIQUE NOT NULL,
  email         TEXT,
  display_name  TEXT NOT NULL,
  avatar_url    TEXT,
  public_key    TEXT NOT NULL,
  persona       TEXT DEFAULT 'general',
  preferred_lang TEXT DEFAULT 'pl',
  is_admin      INTEGER DEFAULT 0,
  bio           TEXT,
  created_at    TEXT NOT NULL,
  last_seen     TEXT
);

CREATE TABLE IF NOT EXISTS contacts (
  id                TEXT PRIMARY KEY,
  owner_id          TEXT NOT NULL,
  contact_user_id   TEXT NOT NULL,
  display_name      TEXT,
  verified          INTEGER DEFAULT 0,
  safety_number     TEXT,
  can_receive_alarm INTEGER DEFAULT 0,
  preferred_lang    TEXT,
  added_at          TEXT NOT NULL,
  UNIQUE(owner_id, contact_user_id)
);

CREATE TABLE IF NOT EXISTS conversations (
  id          TEXT PRIMARY KEY,
  type        TEXT DEFAULT 'direct',
  title       TEXT,
  created_at  TEXT NOT NULL,
  updated_at  TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS conversation_participants (
  conversation_id TEXT NOT NULL,
  user_id         TEXT NOT NULL,
  joined_at       TEXT NOT NULL,
  PRIMARY KEY (conversation_id, user_id)
);

CREATE TABLE IF NOT EXISTS messages (
  id               TEXT PRIMARY KEY,
  conversation_id  TEXT NOT NULL,
  sender_id        TEXT NOT NULL,
  type             TEXT NOT NULL,
  ciphertext       BLOB NOT NULL,
  nonce            BLOB,
  original_lang    TEXT,
  translated_lang  TEXT,
  created_at       TEXT NOT NULL,
  status           TEXT DEFAULT 'sent',
  expires_at       TEXT
);

CREATE TABLE IF NOT EXISTS message_receipts (
  message_id  TEXT NOT NULL,
  user_id     TEXT NOT NULL,
  status      TEXT NOT NULL,
  updated_at  TEXT NOT NULL,
  PRIMARY KEY (message_id, user_id)
);

CREATE TABLE IF NOT EXISTS tiktak_posts (
  id          TEXT PRIMARY KEY,
  author_id   TEXT NOT NULL,
  caption     TEXT,
  media_url   TEXT,
  media_type  TEXT,
  mood        TEXT,
  is_funny    INTEGER DEFAULT 1,
  likes       INTEGER DEFAULT 0,
  comments    INTEGER DEFAULT 0,
  created_at  TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS communities (
  id            TEXT PRIMARY KEY,
  name          TEXT NOT NULL,
  description   TEXT,
  cover_url     TEXT,
  owner_id      TEXT NOT NULL,
  member_count  INTEGER DEFAULT 0,
  is_private    INTEGER DEFAULT 0,
  created_at    TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS community_members (
  community_id TEXT NOT NULL,
  user_id      TEXT NOT NULL,
  role         TEXT DEFAULT 'member',
  joined_at    TEXT NOT NULL,
  PRIMARY KEY (community_id, user_id)
);

CREATE TABLE IF NOT EXISTS music_cache (
  id           TEXT PRIMARY KEY,
  title        TEXT,
  provider     TEXT,
  prompt       TEXT,
  mood         TEXT,
  r2_key       TEXT,
  duration_ms  INTEGER,
  user_id      TEXT,
  created_at   TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS emergency_events (
  id                TEXT PRIMARY KEY,
  initiator_id      TEXT NOT NULL,
  message           TEXT,
  location_shared   INTEGER DEFAULT 0,
  latitude          REAL,
  longitude         REAL,
  expires_at        TEXT,
  status            TEXT DEFAULT 'active',
  created_at        TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS audit_logs (
  id            INTEGER PRIMARY KEY AUTOINCREMENT,
  actor_id      TEXT,
  action        TEXT NOT NULL,
  resource_type TEXT,
  resource_id   TEXT,
  created_at    TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_messages_conversation ON messages(conversation_id, created_at);
CREATE INDEX IF NOT EXISTS idx_tiktak_created ON tiktak_posts(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_users_email ON users(email);
CREATE INDEX IF NOT EXISTS idx_users_public_id ON users(public_id);
