// Weryfikacja modułu Morse'a.
import { strict as assert } from "node:assert";
import { encodeMorse, decodeMorse, morseTimeline, learnMorse, quizMorse, MORSE_TABLE } from "../runtime/morse.js";

const results = [];
const check = (name, fn) => {
  try { fn(); results.push({ name, passed: true }); }
  catch (error) { results.push({ name, passed: false, message: error.message }); }
};

check("SOS AX koduje się kanonicznie", () => assert.equal(encodeMorse("SOS AX"), "... --- ... / .- -..-"));
check("dekodowanie odtwarza tekst", () => assert.equal(decodeMorse("... --- ... / .- -..-"), "SOS AX"));
check("diakrytyki przechodzą round trip", () => assert.equal(decodeMorse(encodeMorse("ZAŻÓŁĆ")), "ZAŻÓŁĆ"));
check("kropka trwa jedną jednostkę przy 20 WPM", () => assert.equal(morseTimeline(".", 20)[0].duration, 60));
check("kreska trwa trzy jednostki przy 20 WPM", () => assert.equal(morseTimeline("-", 20)[0].duration, 180));
check("WPM poza 1–60 jest odrzucane", () => {
  assert.throws(() => morseTimeline(".", 0));
  assert.throws(() => morseTimeline(".", 120));
});
check("niepoprawny zapis Morse'a jest odrzucany", () => {
  assert.throws(() => decodeMorse("ABC"));
  assert.throws(() => decodeMorse("..--"));
});
check("tabela pokrywa litery, cyfry i interpunkcję", () => {
  assert.ok(Object.keys(MORSE_TABLE).length >= 60, `tylko ${Object.keys(MORSE_TABLE).length} wpisów`);
  for (const l of "ABCDEFGHIJKLMNOPQRSTUVWXYZ") assert.ok(MORSE_TABLE[l], `brak ${l}`);
  for (const d of "0123456789") assert.ok(MORSE_TABLE[d], `brak ${d}`);
});
check("tryb nauki objaśnia znak", () => {
  const lesson = learnMorse("s");
  assert.equal(lesson.character, "S");
  assert.equal(lesson.code, "...");
  assert.ok(lesson.mnemonic.includes("di"));
});
check("test wiedzy weryfikuje odpowiedzi", () => {
  const quiz = quizMorse(() => 0);
  assert.equal(quiz.character, "A");
  assert.equal(quiz.verify(".-"), true);
  assert.equal(quiz.verify("-."), false);
});
check("każdy wpis tabeli jest odwracalny", () => {
  for (const [character, code] of Object.entries(MORSE_TABLE)) {
    assert.equal(decodeMorse(code), character, `round trip padł dla ${character}`);
  }
});
check("separator słów jest zachowany", () => {
  assert.equal(encodeMorse("A B"), ".- / -...");
  assert.equal(decodeMorse(".- / -..."), "A B");
});

const failed = results.filter((r) => !r.passed);
for (const r of results) console.log(`${r.passed ? "✓" : "✗"} ${r.name}${r.passed ? "" : " — " + r.message}`);
console.log(`\nmorse: ${results.length - failed.length}/${results.length}`);
if (failed.length) process.exit(1);
