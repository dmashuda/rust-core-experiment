// Minimal static file server for local development (no npm dependencies).
// Usage: node serve.mjs <dir> [port]   (port 0 picks a free one)
import { createServer } from "node:http";
import { readFile, stat } from "node:fs/promises";
import { extname, join, normalize, resolve, sep } from "node:path";

const root = resolve(process.argv[2] ?? ".");
const port = Number(process.argv[3] ?? process.env.PORT ?? 8080);

const TYPES = {
  ".html": "text/html; charset=utf-8",
  ".js": "text/javascript; charset=utf-8",
  ".css": "text/css; charset=utf-8",
  // Must be application/wasm or WebAssembly.instantiateStreaming refuses it.
  ".wasm": "application/wasm",
};

const server = createServer(async (req, res) => {
  let file;
  try {
    // URL parsing folds "/../" away, but percent-encoded separators survive it
    // ("/%2e%2e%2fx" decodes to "/../x"), so the root is re-checked below.
    // decodeURIComponent also throws on a malformed escape such as "/%".
    const urlPath = decodeURIComponent(new URL(req.url, "http://x").pathname);
    file = normalize(join(root, urlPath));
  } catch {
    res.writeHead(400).end("bad request");
    return;
  }
  // The separator matters: a bare prefix test would also accept siblings of
  // the root whose names merely start with it, like "<root>-secrets".
  if (file !== root && !file.startsWith(root + sep)) {
    res.writeHead(403).end("forbidden");
    return;
  }
  try {
    if ((await stat(file)).isDirectory()) file = join(file, "index.html");
    const body = await readFile(file);
    res.writeHead(200, { "content-type": TYPES[extname(file)] ?? "application/octet-stream" });
    res.end(body);
  } catch {
    res.writeHead(404).end("not found");
  }
});

server.listen(port, "127.0.0.1", () =>
  console.log(`serving ${root} at http://127.0.0.1:${server.address().port}`),
);
