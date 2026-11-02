// Weryfikacja runtime kryptografii (Node 20+ ma globalne Web Crypto).
import { strict as assert } from "node:assert";
import { Ratchet, backup, restore, safetyNumber, identity, sha256, sealWithPassphrase, openWithPassphrase } from "../runtime/crypto.js";

const ROOT = "AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8=";
const results = [];
const check = async (name, fn) => {
  try { await fn(); results.push({ name, passed: true }); }
  catch (error) { results.push({ name, passed: false, message: error.message }); }
};

await check("ciphertext nie zawiera plaintextu", async () => {
  const r = await Ratchet.create(ROOT);
  const p = await r.encrypt("Tajna wiadomość", "c1");
  assert.ok(!p.ciphertext.includes("Tajna"));
});

await check("peer odszyfrowuje wiadomość", async () => {
  const a = await Ratchet.create(ROOT), b = await Ratchet.create(ROOT);
  const p = await a.encrypt("Bezpieczna wiadomość", "c1");
  assert.equal(await b.decrypt(p, "c1"), "Bezpieczna wiadomość");
});

await check("powtórzenie pakietu jest odrzucane", async () => {
  const a = await Ratchet.create(ROOT), b = await Ratchet.create(ROOT);
  const p = await a.encrypt("raz", "c1");
  await b.decrypt(p, "c1");
  await assert.rejects(() => b.decrypt(p, "c1"), /Replay|out-of-order/);
});

await check("dostarczenie poza kolejnością jest odrzucane", async () => {
  const a = await Ratchet.create(ROOT), b = await Ratchet.create(ROOT);
  const first = await a.encrypt("pierwsza", "c");
  const second = await a.encrypt("druga", "c");
  await assert.rejects(() => b.decrypt(second, "c"));
  assert.equal(await b.decrypt(first, "c"), "pierwsza");
});

await check("błędne additionalData psuje uwierzytelnienie", async () => {
  const a = await Ratchet.create(ROOT), b = await Ratchet.create(ROOT);
  const p = await a.encrypt("treść", "A");
  await assert.rejects(() => b.decrypt(p, "B"));
});

await check("stan ratcheta można wyeksportować", async () => {
  const a = await Ratchet.create(ROOT);
  const p = await a.encrypt("pierwsza", "c");
  const saved = a.export();
  const b = await Ratchet.create(ROOT);
  assert.equal(await b.decrypt(p, "c"), "pierwsza");
  assert.equal(typeof saved.counter, "number");
  assert.ok(saved.secret.length > 0);
});

await check("safety number jest symetryczny i ma 12 grup", async () => {
  const f = await safetyNumber("klucz-A", "klucz-B");
  assert.equal(f, await safetyNumber("klucz-B", "klucz-A"));
  assert.equal(f.split(" ").length, 12);
});

await check("różne pary kluczy dają różne safety numbers", async () => {
  assert.notEqual(await safetyNumber("A", "B"), await safetyNumber("A", "C"));
});

await check("backup nie zawiera sekretu", async () => {
  const e = await backup({ apiKey: "sk-tajne-123" }, "silne-haslo");
  assert.ok(!e.includes("sk-tajne-123"));
  assert.ok(!e.includes("silne-haslo"));
});

await check("restore odtwarza dokładny payload", async () => {
  const payload = { apiKey: "sk-tajne-123", profile: { textScale: 1.2 } };
  assert.deepEqual(await restore(await backup(payload, "silne-haslo"), "silne-haslo"), payload);
});

await check("błędne hasło nie otwiera backupu", async () => {
  const e = await backup({ s: 1 }, "dobre-haslo");
  await assert.rejects(() => restore(e, "zle-haslo"));
});

await check("backup używa 210k iteracji PBKDF2", async () => {
  const p = await sealWithPassphrase({ a: 1 }, "haslo");
  assert.equal(p.iterations, 210000);
  assert.equal(p.kdf, "PBKDF2-SHA256");
  assert.deepEqual(await openWithPassphrase(p, "haslo"), { a: 1 });
});

await check("identity generuje eksportowalną parę kluczy", async () => {
  const k = await identity();
  assert.ok(k.publicKey.kty && k.privateKey.d);
});

await check("sha256 jest stabilny", async () => {
  assert.equal(await sha256("abc"), await sha256("abc"));
  assert.notEqual(await sha256("abc"), await sha256("abd"));
});

const failed = results.filter((r) => !r.passed);
for (const r of results) console.log(`${r.passed ? "✓" : "✗"} ${r.name}${r.passed ? "" : " — " + r.message}`);
console.log(`\ncrypto: ${results.length - failed.length}/${results.length}`);
if (failed.length) process.exit(1);
