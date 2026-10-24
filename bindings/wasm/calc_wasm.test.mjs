// Loads the real wasm-bindgen output in Node and exercises the API, proving
// the Rust -> wasm -> JS path without a browser.
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import init, { CalculatorUi, KeyStyle } from "./calc_wasm_bindgen/calc.js";

const wasm = await readFile(new URL("./calc_wasm_bindgen/calc_bg.wasm", import.meta.url));
await init({ module_or_path: wasm });

const ui = new CalculatorUi();
let vm = ui.view();
assert.equal(vm.display, "0");
assert.equal(vm.keypad[0].keys[0].label, "C");
assert.equal(vm.keypad.at(-1).keys.at(-1).style, KeyStyle.Operator);
assert.equal(vm.keypad.at(-1).keys.at(-1).span, 2);

for (const id of ["6", "*", "7"]) vm = ui.press(id);
assert.equal(vm.expression, "6*7");
vm = ui.press("evaluate");
assert.equal(vm.display, "42");
assert.deepEqual(vm.history.map((h) => [h.expression, h.result]), [["6*7", "42"]]);

for (const id of ["1", "/", "0", "evaluate"]) vm = ui.press(id);
assert.equal(vm.isError, true);
assert.equal(vm.display, "division by zero");

assert.equal(ui.keyInput("Tab"), undefined);
ui.keyInput("Escape");
ui.keyInput("9");
assert.equal(ui.keyInput("Enter").display, "9");
console.log("wasm bindings OK");
