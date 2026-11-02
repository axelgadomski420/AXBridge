// Shared contracts between client (AXCode runtime) and Cloudflare Worker

export type Persona =
  | "deaf"
  | "mute"
  | "deafMute"
  | "blind"
  | "deafBlind"
  | "motor"
  | "general";

export interface User {
  id: string;
  publicId: string;
  email?: string;
  displayName: string;
  avatarUrl?: string;
  publicKey: string;
  persona: Persona;
  preferredLang: "pl" | "en" | "auto";
  isAdmin: boolean;
  createdAt: string;
}

export interface MessageEnvelope {
  id: string;
  conversationId: string;
  senderId: string;
  type: "text" | "voice" | "morse" | "sign" | "image" | "video" | "music" | "alarm" | "system";
  ciphertext: string; // base64
  nonce: string;
  originalLang?: string;
  translatedLang?: string;
  createdAt: string;
}

export interface MusicGenerateRequest {
  prompt: string;
  mood?: "happy" | "calm" | "energetic" | "sad" | "funny" | "love" | "angry";
  provider?: "suno" | "puter" | "huggingface";
  userId?: string;
}

export interface TikTakPost {
  id: string;
  authorId: string;
  caption: string;
  mediaUrl?: string;
  mediaType: "video" | "image" | "music" | "audio";
  isFunny: boolean;
  likes: number;
  comments: number;
  createdAt: string;
}

export interface TranslateRequest {
  text: string;
  targetLang: "pl" | "en";
  sourceLang?: string;
}

export interface TranslateResponse {
  original: string;
  translated: string;
  cached: boolean;
}
