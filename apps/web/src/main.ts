// Renderer only: draws the Rust ViewModel and forwards input. Keypad layout,
// editing, evaluation and history all live in //core/calc_ui.
import init, { CalculatorUi, KeyStyle, type ViewModel } from "../../../bindings/wasm/calc_wasm_bindgen/calc.js";

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

function render(vm: ViewModel) {
  exprEl.textContent = vm.expression;
  resultEl.textContent = vm.display;
  resultEl.classList.toggle("error", vm.isError);
  captionEl.textContent = `${vm.caption} (WebAssembly)`;

  keysEl.replaceChildren(
    ...vm.keypad.flatMap((row) =>
      row.keys.map((key) => {
        const button = document.createElement("button");
        button.textContent = key.label;
        button.className = STYLE_CLASS[key.style];
        button.style.gridColumn = `span ${key.span}`;
        button.addEventListener("click", () => render(ui.press(key.id)));
        return button;
      }),
    ),
  );

  historyEl.replaceChildren(
    ...vm.history.map((line) => {
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

document.addEventListener("keydown", (e) => {
  if (e.metaKey || e.ctrlKey || e.altKey) return;
  const vm = ui.keyInput(e.key);
  if (vm) {
    e.preventDefault();
    render(vm);
  }
});

render(ui.view());
