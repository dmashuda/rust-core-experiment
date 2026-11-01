// Drives the real bundled renderer (//apps/web:bundle) against a minimal DOM,
// so the wiring main.ts does -- building the keypad once, delegating clicks,
// and freeing the wasm wrappers it is handed -- is covered by a test.
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";

class El {
  constructor(tag) {
    this.tagName = tag.toUpperCase();
    this.children = [];
    this.parent = null;
    this.dataset = {};
    this.style = {};
    this.textContent = "";
    this.className = "";
    this.classes = new Set();
    this.listeners = {};
    this.classList = {
      toggle: (name, on) => (on ? this.classes.add(name) : this.classes.delete(name)),
      contains: (name) => this.classes.has(name),
    };
  }
  adopt(kids) {
    for (const k of kids) k.parent = this;
    return kids;
  }
  append(...kids) {
    this.children.push(...this.adopt(kids));
  }
  replaceChildren(...kids) {
    this.children = this.adopt(kids);
  }
  addEventListener(type, fn) {
    (this.listeners[type] ??= []).push(fn);
  }
  closest(selector) {
    for (let n = this; n; n = n.parent) if (n.tagName === selector.toUpperCase()) return n;
    return null;
  }
  get text() {
    return this.textContent || this.children.map((c) => c.text).join(" ");
  }
}

/// Bubbles an event from `target` up through the ancestors, like the real thing.
function dispatch(target, type, event = {}) {
  for (let n = target; n; n = n.parent) {
    for (const fn of n.listeners[type] ?? []) fn({ target, preventDefault() {}, ...event });
  }
}

const ids = ["expr", "result", "keys", "history", "caption"];
const els = Object.fromEntries(ids.map((id) => [id, new El("div")]));
const doc = new El("document");
globalThis.document = {
  getElementById: (id) => els[id],
  createElement: (tag) => new El(tag),
  addEventListener: (type, fn) => doc.addEventListener(type, fn),
};
// The bundle loads the wasm by URL; Node's fetch does not do file: URLs.
globalThis.fetch = async (url) =>
  new Response(await readFile(url), { headers: { "content-type": "application/wasm" } });

await import(new URL("./site/main.js", import.meta.url));

const { expr, result, keys, history, caption } = els;

// The keypad is built once, from the static keypad() call.
assert.equal(keys.children.length, 23, "every key has a button");
assert.deepEqual(keys.children.at(0).textContent, "C");
assert.equal(keys.children.at(-1).dataset.id, "evaluate");
assert.equal(keys.children.at(-1).style.gridColumn, "span 2");
assert.match(caption.textContent, /WebAssembly/);
const buttonsBefore = [...keys.children];
const press = (id) => dispatch(keys.children.find((b) => b.dataset.id === id), "click");

// Clicks arrive through the one delegated listener on #keys.
for (const id of ["6", "*", "7"]) press(id);
assert.equal(expr.textContent, "6*7");
press("evaluate");
assert.equal(result.textContent, "42");
assert.equal(history.children.length, 1);
assert.equal(history.children[0].text, "6*7 = 42");

// Re-rendering must not rebuild the 23 buttons, nor the unchanged history.
assert.deepEqual(
  keys.children.map((b, i) => b === buttonsBefore[i]),
  buttonsBefore.map(() => true),
  "the keypad buttons are the same nodes after a render",
);
const historyRow = history.children[0];
press("1");
assert.equal(history.children[0], historyRow, "an unchanged history list is left alone");

// An operator after "=" continues from the answer; errors clear on edit.
press("backspace");
for (const id of ["+", "1", "evaluate"]) press(id);
assert.equal(result.textContent, "43");
for (const id of ["1", "/", "0", "evaluate"]) press(id);
assert.equal(result.textContent, "division by zero");
assert.ok(result.classList.contains("error"));
press("backspace");
assert.equal(result.textContent, "43", "the error text does not linger as a result");
assert.ok(!result.classList.contains("error"));

// Hardware keys go through the same path.
dispatch(doc, "keydown", { key: "Escape", metaKey: false, ctrlKey: false, altKey: false });
assert.equal(expr.textContent, "");
for (const key of "8*8") dispatch(doc, "keydown", { key, metaKey: false, ctrlKey: false, altKey: false });
dispatch(doc, "keydown", { key: "Enter", metaKey: false, ctrlKey: false, altKey: false });
assert.equal(result.textContent, "64");

// Many presses, each handed a fresh ViewModel that render() frees: if a free
// were wrong or missing a pointer would be reused and wasm would throw here.
for (let i = 0; i < 500; i++) press("1");
assert.equal(expr.textContent.length, 500);

console.log("web renderer OK");
