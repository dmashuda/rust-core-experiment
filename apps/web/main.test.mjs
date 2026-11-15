// Drives the real bundled renderer (//apps/web:bundle) against a minimal DOM:
// first the shared flows (//core/calc_flows:flows.json), clicking the real
// buttons and reading what the page shows, then what only the web renderer
// does -- building the keypad once, delegating clicks, and freeing the wasm
// wrappers it is handed.
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

// The bundle loads the wasm by URL; Node's fetch does not do file: URLs.
globalThis.fetch = async (url) =>
  new Response(await readFile(url), { headers: { "content-type": "application/wasm" } });

/// A fresh page: new elements, and the bundle run again over them (a new
/// module instance, so a new wasm instance and a new calculator).
let loads = 0;
async function load() {
  const ids = ["expr", "result", "keys", "history", "caption"];
  const els = Object.fromEntries(ids.map((id) => [id, new El("div")]));
  const doc = new El("document");
  globalThis.document = {
    getElementById: (id) => els[id],
    createElement: (tag) => new El(tag),
    addEventListener: (type, fn) => doc.addEventListener(type, fn),
  };
  await import(new URL(`./site/main.js?load=${loads++}`, import.meta.url));
  const press = (id) => {
    const button = els.keys.children.find((b) => b.dataset.id === id);
    if (button) dispatch(button, "click");
    return button !== undefined;
  };
  /// Whether the page took the key (called preventDefault).
  const key = (key) => {
    let taken = false;
    dispatch(doc, "keydown", { key, metaKey: false, ctrlKey: false, altKey: false, preventDefault: () => (taken = true) });
    return taken;
  };
  return { ...els, press, key };
}

// ------------------------------------------------------------ the flows --

/// What the page shows for a flow's `field`.
function read(page, field) {
  switch (field) {
    case "expression":
      return page.expr.textContent;
    case "display":
      return page.result.textContent;
    case "error":
      return String(page.result.classList.contains("error"));
    case "history":
      return page.history.children.map((li) => li.text).join("\n");
    default:
      return `unknown field ${field}`;
  }
}

async function play(flow) {
  const page = await load();
  for (const [index, step] of flow.steps.entries()) {
    const place = `"${flow.name}", step ${index + 1} (${JSON.stringify(step)})`;
    switch (step.op) {
      case "press":
        assert.ok(page.press(step.id), `${place}: no such button`);
        break;
      case "key":
        assert.ok(page.key(step.key), `${place}: the page ignored it`);
        break;
      case "ignored":
        assert.ok(!page.key(step.key), `${place}: the page took it`);
        break;
      case "expect":
        assert.equal(read(page, step.field), step.value, place);
        break;
      default:
        assert.fail(`${place}: unknown step`);
    }
  }
}

const flows = JSON.parse(await readFile(new URL("../../core/calc_flows/flows.json", import.meta.url), "utf8"));
assert.ok(flows.length > 0);
for (const flow of flows) await play(flow);

// --------------------------------------------------------- only the web --

const { keys, history, caption, expr, press } = await load();

// The keypad is built once, from the static keypad() call.
assert.equal(keys.children.length, 23, "every key has a button");
assert.deepEqual(keys.children.at(0).textContent, "C");
assert.equal(keys.children.at(-1).dataset.id, "evaluate");
assert.equal(keys.children.at(-1).style.gridColumn, "span 2");
assert.equal(keys.style.gridTemplateColumns, "repeat(4, 1fr)", "the grid width comes from Rust, not CSS");
assert.match(caption.textContent, /WebAssembly/);
const buttonsBefore = [...keys.children];

// Re-rendering must not rebuild the 23 buttons, nor the unchanged history.
for (const id of ["6", "*", "7", "evaluate"]) press(id);
assert.deepEqual(
  keys.children.map((b, i) => b === buttonsBefore[i]),
  buttonsBefore.map(() => true),
  "the keypad buttons are the same nodes after a render",
);
const historyRow = history.children[0];
press("1");
assert.equal(history.children[0], historyRow, "an unchanged history list is left alone");

// Many presses, each handed a fresh ViewModel that render() frees: if a free
// were wrong or missing a pointer would be reused and wasm would throw here.
for (let i = 0; i < 500; i++) press("1");
assert.equal(expr.textContent.length, 501);

console.log(`web renderer OK: ${flows.length} flows`);
