// Loads the real wasm-bindgen output in Node and exercises the API, proving
// the Rust -> wasm -> JS path without a browser.
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import init, { CalculatorUi, KeyStyle, keypad, openCalculator } from "./calc_wasm_bindgen/calc.js";

const wasm = await readFile(new URL("./calc_wasm_bindgen/calc_bg.wasm", import.meta.url));
await init({ module_or_path: wasm });

// The keypad is static: one call, not part of the per-keystroke ViewModel.
const rows = keypad();
assert.equal(rows[0].keys[0].label, "C");
assert.equal(rows.at(-1).keys.at(-1).style, KeyStyle.Operator);
assert.equal(rows.at(-1).keys.at(-1).span, 2);

const ui = new CalculatorUi();
let vm = ui.view();
assert.equal(vm.display, "0");
assert.equal(vm.keypad, undefined, "the ViewModel no longer carries the keypad");

for (const id of ["6", "*", "7"]) vm = ui.press(id);
assert.equal(vm.expression, "6*7");
vm = ui.press("evaluate");
assert.equal(vm.display, "42");
assert.deepEqual(
  vm.history.map((h) => [h.id, h.expression, h.result]),
  [[1n, "6*7", "42"]],
);

// An operator straight after "=" continues from the answer.
assert.equal(ui.press("+").expression, "ans+");
ui.press("1");
assert.equal(ui.press("evaluate").display, "43");

// An error is view state, and editing it away restores the previous result.
for (const id of ["1", "/", "0", "evaluate"]) vm = ui.press(id);
assert.equal(vm.isError, true);
assert.equal(vm.display, "division by zero");
vm = ui.press("backspace");
assert.equal(vm.isError, false);
assert.equal(vm.display, "43", "the error text must not linger as a result");

// A result too small for 10 fractional digits is not shown as 0.
const tiny = new CalculatorUi();
for (const id of ["1", "/", "1", "0", "0", "0", "0", "0", "0", "0", "0", "0", "0", "0", "evaluate"]) {
  vm = tiny.press(id);
}
assert.equal(vm.display, "1e-11");

// Nesting deeper than the parser's limit is an error, not a crash.
const deep = new CalculatorUi();
for (let i = 0; i < 5000; i++) deep.press("(");
deep.press("1");
assert.equal(deep.press("evaluate").display, "expression nested too deeply");

assert.equal(ui.keyInput("Tab"), undefined);
ui.keyInput("Escape");
ui.keyInput("9");
assert.equal(ui.keyInput("Enter").display, "9");
// openCalculator() keeps history in SQLite. Node has no OPFS, so this
// exercises the degraded path: still SQLite, still compiled to wasm, just
// held in memory -- and the caption has to say so rather than imply the
// history is being kept.
const sqlite = await openCalculator("calc-history.db");
let sv = sqlite.view();
assert.match(sv.caption, /^Evaluated by Rust, history in SQLite 3\./, sv.caption);
assert.match(sv.caption, /memory, no OPFS: /, sv.caption);
for (const id of ["6", "*", "7", "evaluate"]) sv = sqlite.press(id);
assert.equal(sv.display, "42");
assert.deepEqual(
  sv.history.map((h) => [h.id, h.expression, h.result]),
  [[1n, "6*7", "42"]],
  "the row came back out of SQLite, with the id SQLite assigned",
);
assert.equal(sqlite.press("+").expression, "ans+");

console.log("wasm bindings OK");
