// Drives the real bundled renderer (//apps/web:bundle) against a minimal DOM,
// so the wiring main.ts does -- building the keypad once, delegating clicks,
// deciding which keys to swallow, and redrawing history only when it changed
// -- is covered by a test.
//
// The worker is real too: `Worker` below loads //apps/web:worker_bundle in
// this process with a stubbed `self`, so a press still goes all the way into
// Rust, through SQLite, and back. Replies arrive asynchronously, as they do in
// a browser, which is why the presses here are awaited.
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
// The worker loads the wasm by URL; Node's fetch does not do file: URLs.
globalThis.fetch = async (url) =>
  new Response(await readFile(url), { headers: { "content-type": "application/wasm" } });

/// The worker bundle, loaded in this process. `self` is what it posts through.
let theWorker;
globalThis.Worker = class {
  #pageListeners = [];
  #workerListeners = [];
  #queued = [];
  /** How many messages the worker has sent the page, to await a render on. */
  delivered = 0;

  constructor(url) {
    theWorker = this;
    globalThis.self = {
      postMessage: (message) => {
        queueMicrotask(() => {
          this.delivered += 1;
          for (const fn of this.#pageListeners) fn({ data: message });
        });
      },
      addEventListener: (type, fn) => {
        if (type !== "message") return;
        this.#workerListeners.push(fn);
        for (const message of this.#queued.splice(0)) queueMicrotask(() => fn({ data: message }));
      },
    };
    this.loaded = import(url.href ?? url);
  }

  postMessage(message) {
    if (this.#workerListeners.length === 0) {
      this.#queued.push(message);
      return;
    }
    for (const fn of this.#workerListeners) queueMicrotask(() => fn({ data: message }));
  }

  addEventListener(type, fn) {
    if (type === "message") this.#pageListeners.push(fn);
  }
};

/// Polls, because a worker reply arrives whenever it arrives.
async function until(condition, what) {
  for (let i = 0; i < 400; i++) {
    if (condition()) return;
    await new Promise((resolve) => setTimeout(resolve, 5));
  }
  throw new Error(`timed out waiting for ${what}`);
}

await import(new URL("./site/main.js", import.meta.url));

const { expr, result, keys, history, caption } = els;

// Nothing is on screen until the worker has loaded the wasm and opened its
// history; the page does not block on it.
await until(() => keys.children.length > 0, "the worker to be ready");

// The keypad is built once, from the static keypad() the worker sent.
assert.equal(keys.children.length, 23, "every key has a button");
assert.deepEqual(keys.children.at(0).textContent, "C");
assert.equal(keys.children.at(-1).dataset.id, "evaluate");
assert.equal(keys.children.at(-1).style.gridColumn, "span 2");
assert.match(caption.textContent, /WebAssembly/);
// Node has no OPFS, so the worker says which SQLite it ended up with.
assert.match(caption.textContent, /history in SQLite 3\./, caption.textContent);

const buttonsBefore = [...keys.children];

/// Clicks the key and waits for the view the worker sends back.
async function press(...pressed) {
  const before = theWorker.delivered;
  for (const id of pressed) {
    dispatch(
      keys.children.find((b) => b.dataset.id === id),
      "click",
    );
  }
  await until(() => theWorker.delivered >= before + pressed.length, `a view after ${pressed}`);
}

/// Types a hardware key through the document listener, as a browser would.
function typeKey(key) {
  dispatch(doc, "keydown", { key, metaKey: false, ctrlKey: false, altKey: false });
}

/// Types a key the calculator is expected to act on, and waits for the view.
async function type(key) {
  const before = theWorker.delivered;
  typeKey(key);
  await until(() => theWorker.delivered > before, `a view after ${key}`);
}

// Clicks arrive through the one delegated listener on #keys.
await press("6", "*", "7");
assert.equal(expr.textContent, "6*7");
await press("evaluate");
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
await press("1");
assert.equal(history.children[0], historyRow, "an unchanged history list is left alone");

// An operator after "=" continues from the answer; errors clear on edit.
await press("backspace");
await press("+", "1", "evaluate");
assert.equal(result.textContent, "43");
await press("1", "/", "0", "evaluate");
assert.equal(result.textContent, "division by zero");
assert.ok(result.classList.contains("error"));
await press("backspace");
assert.equal(result.textContent, "43", "the error text does not linger as a result");
assert.ok(!result.classList.contains("error"));

// Hardware keys go through the same path.
await type("Escape");
assert.equal(expr.textContent, "");
for (const key of "8*8") await type(key);
await type("Enter");
assert.equal(result.textContent, "64");

// A key the calculator does not use is not sent at all, so it keeps whatever
// meaning the browser gives it. (Rust decides which: keyboardKeys().)
const beforeTab = theWorker.delivered;
typeKey("Tab");
typeKey("A");
await new Promise((resolve) => setTimeout(resolve, 20));
assert.equal(theWorker.delivered, beforeTab, "Tab and uppercase keys are left to the browser");

// Many presses, each answered with its own view: ordering holds and nothing
// is dropped, which is the part a worker could get wrong.
await press(...Array.from({ length: 500 }, () => "1"));
assert.equal(expr.textContent.length, 500);

console.log("web renderer OK");
