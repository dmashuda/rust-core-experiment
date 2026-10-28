// Renderer only: draws the ViewModel the worker sends and forwards input.
// Keypad layout, editing, evaluation and history all live in //core/calc_ui;
// the calculator and its SQLite history run in //apps/web/src/worker.ts.
import type { FromWorker, KeyboardKeysData, KeyRowData, ToWorker, ViewData } from "./protocol.js";

const worker = new Worker(new URL("./worker.js", import.meta.url), { type: "module" });
const send = (message: ToWorker) => worker.postMessage(message);

const exprEl = document.getElementById("expr") as HTMLOutputElement;
const resultEl = document.getElementById("result") as HTMLOutputElement;
const keysEl = document.getElementById("keys") as HTMLDivElement;
const historyEl = document.getElementById("history") as HTMLOListElement;
const captionEl = document.getElementById("caption") as HTMLParagraphElement;

/** `KeyStyle` from Rust, in declaration order. */
const STYLE_CLASS = ["input", "function", "op"];

// The keypad is static, so the buttons are built once. One delegated listener
// serves all of them, so no per-render closures either.
function buildKeypad(rows: KeyRowData[]) {
  keysEl.replaceChildren(
    ...rows.flatMap((row) =>
      row.keys.map((key) => {
        const button = document.createElement("button");
        button.textContent = key.label;
        button.className = STYLE_CLASS[key.style] ?? "input";
        button.style.gridColumn = `span ${key.span}`;
        button.dataset.id = key.id;
        return button;
      }),
    ),
  );
}

keysEl.addEventListener("click", (e) => {
  const id = (e.target as HTMLElement).closest("button")?.dataset.id;
  if (id !== undefined) send({ kind: "press", id });
});

// Whether the calculator wants a key has to be decided while the event is
// being dispatched, so the renderer cannot ask the worker. Rust publishes the
// answer as static data instead, fetched once with the keypad.
function listenForKeys(keys: KeyboardKeysData) {
  const named = new Set(keys.named);
  document.addEventListener("keydown", (e) => {
    if (e.metaKey || e.ctrlKey || e.altKey) return;
    if (!named.has(e.key) && !(e.key.length === 1 && keys.typeable.includes(e.key))) return;
    e.preventDefault();
    send({ kind: "key", key: e.key });
  });
}

// History only ever gains a newest line (and drops the oldest at the cap), so
// its length plus newest id is enough to know whether the list needs redrawing.
let historySignature = "";

function render(vm: ViewData) {
  exprEl.textContent = vm.expression;
  resultEl.textContent = vm.display;
  resultEl.classList.toggle("error", vm.isError);
  captionEl.textContent = `${vm.caption} (WebAssembly)`;

  const history = vm.history;
  const signature = `${history.length}:${history[0]?.id ?? 0n}`;
  if (signature !== historySignature) {
    historySignature = signature;
    historyEl.replaceChildren(
      ...history.map((line) => {
        const li = document.createElement("li");
        const left = document.createElement("span");
        const right = document.createElement("span");
        left.textContent = line.expression;
        right.textContent = `= ${line.result}`;
        li.append(left, right);
        return li;
      }),
    );
  }
}

worker.addEventListener("message", (e: MessageEvent<FromWorker>) => {
  const message = e.data;
  if (message.kind === "ready") {
    buildKeypad(message.keypad);
    listenForKeys(message.keyboardKeys);
  }
  render(message.view);
});
