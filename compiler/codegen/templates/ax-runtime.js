export const state = new Proxy({
  screen: null,
  params: {},
  accessibility: {
    preferredInput: "text",
    preferredOutput: "text",
    morseWpm: 15,
    vibrationStrength: 0.7,
    textScale: 1,
    highContrast: false,
    reduceMotion: false,
    ttsVoice: "default",
    ttsRate: 1,
    ttsPitch: 1,
    autoTranslate: true,
    showOriginalAndTranslation: true,
    confirmBeforeSend: true,
    showConfidence: true,
    largeTargets: true,
    maxActions: 8,
    announceChanges: true,
    autoSendChannels: []
  },
  persona: "general",
  user: null,
  session: null,
  conversations: [],
  messages: [],
  contacts: [],
  communities: [],
  tiktak: [],
  music: [],
  providers: [],
  emergency: [],
  outbox: [],
  error: "",
  busy: false,
  loading: {},
  toast: ""
}, { set(target, key, value) { target[key] = value; if (key !== "toast") scheduleRender(); return true; } });

const listeners = new Set();
let frame = 0;

export function subscribe(listener) { listeners.add(listener); return () => listeners.delete(listener); }

function scheduleRender() {
  if (frame) return;
  frame = requestAnimationFrame(() => { frame = 0; for (const listener of listeners) listener(state); });
}

export function announce(message, politeness = "polite") {
  if (!state.accessibility.announceChanges || typeof document === "undefined") return;
  let region = document.getElementById("ax-live");
  if (!region) {
    region = document.createElement("div");
    region.id = "ax-live";
    region.className = "sr-only";
    region.setAttribute("role", "status");
    region.setAttribute("aria-live", politeness);
    document.body.appendChild(region);
  }
  region.setAttribute("aria-live", politeness);
  region.textContent = "";
  setTimeout(() => { region.textContent = message; }, 40);
}

export function applyAccessibilityProfile(profile) {
  Object.assign(state.accessibility, profile || {});
  if (typeof document !== "undefined") {
    const root = document.documentElement;
    root.style.setProperty("--scale", String(state.accessibility.textScale || 1));
    root.dataset.reduceMotion = String(!!state.accessibility.reduceMotion);
    root.dataset.contrast = state.accessibility.highContrast ? "high" : "normal";
  }
}

export function haptic(pattern, strength = state.accessibility.vibrationStrength) {
  if (typeof navigator === "undefined" || !navigator.vibrate) return false;
  navigator.vibrate(pattern.map(value => Math.round(value * (0.5 + strength))));
  return true;
}

export async function speak(text, { voice = state.accessibility.ttsVoice, rate = state.accessibility.ttsRate, pitch = state.accessibility.ttsPitch } = {}) {
  if (typeof speechSynthesis === "undefined") throw new Error("Synteza mowy niedostępna");
  return new Promise((resolve, reject) => {
    const utterance = new SpeechSynthesisUtterance(text);
    utterance.voice = speechSynthesis.getVoices().find(v => v.name === voice || v.voiceURI === voice) || null;
    utterance.rate = Math.min(3, Math.max(0.5, Number(rate) || 1));
    utterance.pitch = Math.min(2, Math.max(0, Number(pitch) || 1));
    utterance.onend = () => resolve(true);
    utterance.onerror = event => reject(new Error(`Synteza mowy: ${event.error}`));
    speechSynthesis.speak(utterance);
  });
}

export function subscribeNetwork(onChange) {
  if (typeof window === "undefined") return () => {};
  const handler = () => onChange(navigator.onLine);
  addEventListener("online", handler);
  addEventListener("offline", handler);
  return () => { removeEventListener("online", handler); removeEventListener("offline", handler); };
}

export function mount(render) {
  const root = document.getElementById("ax-root");
  if (!root) throw new Error("Brak kontenera #ax-root");
  const draw = () => { root.innerHTML = render(state); };
  subscribe(draw);
  draw();
  return root;
}
