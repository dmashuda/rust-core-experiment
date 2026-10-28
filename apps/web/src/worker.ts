// The calculator itself, off the main thread.
//
// It runs here and not in the page because its history is a SQLite database
// in OPFS, and the VFS that reaches OPFS needs `createSyncAccessHandle`,
// which exists only in a dedicated worker. SQLite's calls are synchronous
// too, so a worker is where they belong regardless.
//
// No calculator logic lives here either: this is a transport. Input goes to
// Rust, the returned ViewModel is copied into a plain object and posted back.
import init, {
  keyboardKeys,
  keypad,
  openCalculator,
  type CalculatorUi,
  type ViewModel,
} from "../../../bindings/wasm/calc_wasm_bindgen/calc.js";
import type { FromWorker, KeyRowData, ToWorker, ViewData } from "./protocol.js";

/** The database name inside the OPFS pool. */
const DATABASE = "calc-history.db";

const post = (message: FromWorker) => self.postMessage(message);

/** Copies a ViewModel out of wasm memory, then frees the wrappers. */
function toViewData(vm: ViewModel): ViewData {
  const history = vm.history;
  const data: ViewData = {
    expression: vm.expression,
    display: vm.display,
    isError: vm.isError,
    caption: vm.caption,
    history: history.map((line) => ({
      id: line.id,
      expression: line.expression,
      result: line.result,
    })),
  };
  for (const line of history) line.free();
  vm.free();
  return data;
}

function toKeypadData(): KeyRowData[] {
  return keypad().map((row) => {
    const keys = row.keys;
    const data = keys.map((key) => {
      const copy = { id: key.id, label: key.label, style: key.style, span: key.span };
      key.free();
      return copy;
    });
    row.free();
    return { keys: data };
  });
}

let ui: CalculatorUi | undefined;
/** Input that arrived while the wasm module was still loading. */
const pending: ToWorker[] = [];

function handle(message: ToWorker) {
  if (!ui) {
    pending.push(message);
    return;
  }
  const vm = message.kind === "press" ? ui.press(message.id) : ui.keyInput(message.key);
  // keyInput returns undefined for a key the calculator ignores. The renderer
  // decided not to send those (see keyboardKeys), but a stale one is possible.
  if (vm) post({ kind: "view", view: toViewData(vm) });
}

// Listening before the first await, so nothing sent during startup is lost.
self.addEventListener("message", (e: MessageEvent<ToWorker>) => handle(e.data));

// The .wasm sits next to this bundle; see //apps/web:site.
await init({ module_or_path: new URL("./calc_bg.wasm", import.meta.url) });
ui = await openCalculator(DATABASE);

const keys = keyboardKeys();
post({
  kind: "ready",
  keypad: toKeypadData(),
  keyboardKeys: { named: keys.named, typeable: keys.typeable },
  view: toViewData(ui.view()),
});
keys.free();

for (const message of pending.splice(0)) handle(message);
