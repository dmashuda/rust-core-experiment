import init, { Calculator } from "../../../bindings/wasm/calc_wasm_bindgen/calc.js";

// The .wasm is copied next to main.js by //apps/web:site.
await init({ module_or_path: new URL("./calc_bg.wasm", import.meta.url) });

const calc = new Calculator();
const exprEl = document.getElementById("expr") as HTMLOutputElement;
const resultEl = document.getElementById("result") as HTMLOutputElement;
const historyEl = document.getElementById("history") as HTMLOListElement;
const keysEl = document.getElementById("keys") as HTMLDivElement;

let expr = "";

const KEYS: Array<{ label: string; insert?: string; action?: () => void; cls?: string }> = [
  { label: "C", action: () => setExpr("") },
  { label: "(", insert: "(" },
  { label: ")", insert: ")" },
  { label: "÷", insert: "/", cls: "op" },
  { label: "7", insert: "7" },
  { label: "8", insert: "8" },
  { label: "9", insert: "9" },
  { label: "×", insert: "*", cls: "op" },
  { label: "4", insert: "4" },
  { label: "5", insert: "5" },
  { label: "6", insert: "6" },
  { label: "−", insert: "-", cls: "op" },
  { label: "1", insert: "1" },
  { label: "2", insert: "2" },
  { label: "3", insert: "3" },
  { label: "+", insert: "+", cls: "op" },
  { label: "0", insert: "0" },
  { label: ".", insert: "." },
  { label: "^", insert: "^" },
  { label: "⌫", action: () => setExpr(expr.slice(0, -1)) },
  { label: "ans", insert: "ans" },
  { label: "%", insert: "%" },
  { label: "=", action: () => evaluate(), cls: "op wide" },
];

for (const key of KEYS) {
  const button = document.createElement("button");
  button.textContent = key.label;
  if (key.cls) button.className = key.cls;
  button.addEventListener("click", () => (key.action ? key.action() : setExpr(expr + key.insert)));
  keysEl.append(button);
}

document.addEventListener("keydown", (e) => {
  if (e.key === "Enter" || e.key === "=") evaluate();
  else if (e.key === "Backspace") setExpr(expr.slice(0, -1));
  else if (e.key === "Escape") setExpr("");
  else if (/^[0-9a-z.+\-*/%^() ]$/.test(e.key)) setExpr(expr + e.key);
  else return;
  e.preventDefault();
});

function setExpr(next: string) {
  expr = next;
  exprEl.textContent = expr;
  resultEl.classList.remove("error");
}

function evaluate() {
  try {
    resultEl.textContent = calc.evaluate(expr);
    resultEl.classList.remove("error");
    renderHistory();
    expr = "";
  } catch (err) {
    resultEl.textContent = (err as Error).message;
    resultEl.classList.add("error");
  }
}

function renderHistory() {
  historyEl.replaceChildren(
    ...calc
      .history()
      .reverse()
      .map((entry) => {
        const li = document.createElement("li");
        const left = document.createElement("span");
        const right = document.createElement("span");
        left.textContent = entry.expression;
        right.textContent = `= ${entry.result}`;
        li.append(left, right);
        entry.free();
        return li;
      }),
  );
}
