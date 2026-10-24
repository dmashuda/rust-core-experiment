// Renderer only: draws the Rust ViewModel and forwards input. Keypad layout,
// editing, evaluation and history all live in //core/calc_ui.
import init, {
  CalculatorUi,
  KeyStyle,
  keypad,
  type ViewModel,
} from "../../../bindings/wasm/calc_wasm_bindgen/calc.js";

// The .wasm is copied next to main.js by //apps/web:site.
await init({ module_or_path: new URL("./calc_bg.wasm", import.meta.url) });

const ui = new CalculatorUi();
const exprEl = document.getElementById("expr") as HTMLOutputElement;
const resultEl = document.getElementById("result") as HTMLOutputElement;
const keysEl = document.getElementById("keys") as HTMLDivElement;
const historyEl = document.getElementById("history") as HTMLOListElement;
const captionEl = document.getElementById("caption") as HTMLParagraphElement;

const STYLE_CLASS: Record<KeyStyle, string> = {
  [KeyStyle.Input]: "input",
  [KeyStyle.Function]: "function",
  [KeyStyle.Operator]: "op",
};

// The keypad is static, so the buttons are built once. One delegated listener
// serves all of them, so no per-render closures either.
function buildKeypad() {
  const rows = keypad();
  keysEl.replaceChildren(
    ...rows.flatMap((row) => {
      const keys = row.keys;
      const buttons = keys.map((key) => {
        const button = document.createElement("button");
        button.textContent = key.label;
        button.className = STYLE_CLASS[key.style];
        button.style.gridColumn = `span ${key.span}`;
        button.dataset.id = key.id;
        key.free();
        return button;
      });
      row.free();
      return buttons;
    }),
  );
}

keysEl.addEventListener("click", (e) => {
  const id = (e.target as HTMLElement).closest("button")?.dataset.id;
  if (id !== undefined) render(ui.press(id));
});

// History only ever gains a newest line (and drops the oldest at the cap), so
// its length plus newest id is enough to know whether the list needs redrawing.
let historySignature = "";

function render(vm: ViewModel) {
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
  // wasm-bindgen hands out owning wrappers; drop them now rather than waiting
  // for the FinalizationRegistry to get around to it.
  for (const line of history) line.free();
  vm.free();
}

document.addEventListener("keydown", (e) => {
  if (e.metaKey || e.ctrlKey || e.altKey) return;
  const vm = ui.keyInput(e.key);
  if (vm) {
    e.preventDefault();
    render(vm);
  }
});

buildKeypad();
render(ui.view());
