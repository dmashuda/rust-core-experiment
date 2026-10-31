// What crosses the worker boundary.
//
// The wasm objects themselves cannot: wasm-bindgen hands out wrappers around
// pointers into the worker's own memory. So the worker converts each one into
// a plain structured-cloneable object, which is also the only place in the web
// app that knows the wasm API exists.

/** One keypad key. `style` is the numeric `KeyStyle` from Rust. */
export type KeyData = { id: string; label: string; style: number; span: number };

export type KeyRowData = { keys: KeyData[] };

/** `id` is a Rust u64, so it arrives as a BigInt. */
export type HistoryLineData = { id: bigint; expression: string; result: string };

export type ViewData = {
  expression: string;
  display: string;
  isError: boolean;
  history: HistoryLineData[];
  caption: string;
};

/** The keys the calculator acts on; see `keyboardKeys()` in Rust. */
export type KeyboardKeysData = { named: string[]; typeable: string };

export type ToWorker = { kind: "press"; id: string } | { kind: "key"; key: string };

export type FromWorker =
  /** Sent once, when the wasm module and its SQLite history are open. */
  | { kind: "ready"; keypad: KeyRowData[]; keyboardKeys: KeyboardKeysData; view: ViewData }
  | { kind: "view"; view: ViewData };
