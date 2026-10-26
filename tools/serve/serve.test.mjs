// Starts the real server over a temporary directory and checks that it serves
// what is under the root and nothing else.
import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { mkdir, mkdtemp, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";

const dir = await mkdtemp(join(tmpdir(), "serve-test-"));
const root = join(dir, "site");
await mkdir(root);
await writeFile(join(root, "index.html"), "PUBLIC");
await writeFile(join(root, "app.wasm"), "WASMBYTES");
// A sibling whose name starts with the root's name: a prefix check alone
// would hand this out.
await writeFile(join(dir, "site-secrets.txt"), "SECRET");

const script = new URL("./serve.mjs", import.meta.url).pathname;
const child = spawn(process.execPath, [script, root, "0"], { stdio: ["ignore", "pipe", "inherit"] });

const url = await new Promise((resolve, reject) => {
  let out = "";
  const giveUp = setTimeout(() => reject(new Error(`server never reported a port; saw: ${out}`)), 10_000);
  giveUp.unref();
  child.stdout.on("data", (chunk) => {
    out += chunk;
    const match = out.match(/at (http:\/\/\S+)/);
    if (match) {
      clearTimeout(giveUp);
      resolve(match[1]);
    }
  });
  child.on("exit", (code) => reject(new Error(`server exited early with ${code}`)));
});

assert.match(url, /^http:\/\/127\.0\.0\.1:\d+$/, "the dev server binds loopback, not every interface");

const get = async (path) => {
  const res = await fetch(url + path);
  return { status: res.status, body: await res.text() };
};

try {
  assert.deepEqual(await get("/"), { status: 200, body: "PUBLIC" });
  assert.deepEqual(await get("/index.html"), { status: 200, body: "PUBLIC" });
  assert.equal((await fetch(url + "/app.wasm")).headers.get("content-type"), "application/wasm");
  assert.equal((await get("/nope.html")).status, 404);

  // Encoded dot-dot segments survive URL parsing, so they reach the file
  // lookup: they must not escape the root, not even into a sibling that
  // shares its name prefix.
  for (const path of [
    "/%2e%2e%2fsite-secrets.txt",
    "/%2e%2e%2f%2e%2e%2fetc%2fhosts",
    "/a%2f%2e%2e%2f%2e%2e%2fsite-secrets.txt",
  ]) {
    const { status, body } = await get(path);
    assert.equal(status, 403, `${path} should be refused, got ${status} ${body}`);
    assert.doesNotMatch(body, /SECRET/, `${path} leaked a file outside the root`);
  }
  // A malformed escape must not take the server down with it.
  assert.equal((await get("/%")).status, 400);
  assert.deepEqual(await get("/"), { status: 200, body: "PUBLIC" }, "still serving");
  console.log("serve OK");
} finally {
  child.kill();
}
