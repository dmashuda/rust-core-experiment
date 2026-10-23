// Loads the real wasm-bindgen output in Node and exercises the API, proving
// the Rust -> wasm -> JS path without a browser.
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import init, { Calculator, evaluate } from "./calc_wasm_bindgen/calc.js";

const wasm = await readFile(new URL("./calc_wasm_bindgen/calc_bg.wasm", import.meta.url));
await init({ module_or_path: wasm });

assert.equal(evaluate("1 + 2 * 3"), "7");
assert.equal(evaluate("0.1 + 0.2"), "0.3");
assert.throws(() => evaluate("1 / 0"), /division by zero/);

const calc = new Calculator();
assert.equal(calc.evaluate("6 * 7"), "42");
assert.equal(calc.evaluate("ans / 2"), "21");
assert.deepEqual(
  calc.history().map((e) => [e.expression, e.result]),
  [["6 * 7", "42"], ["ans / 2", "21"]],
);
console.log("wasm bindings OK");
