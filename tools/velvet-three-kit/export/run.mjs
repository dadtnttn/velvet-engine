// The export driver every prototype shares. It serves the prototype (a Vite server of its own, only
// while it exports), opens it in headless Chrome, waits for its debug hook to be ready, asks it to
// export and writes what the page hands back: GLB files (their meshes named after their nodes, as
// the game reads them) and plan JSON files.
//
//   import { exportGame } from "../../../tools/velvet-three-kit/export/run.mjs";
//   await exportGame({ prototype: new URL("..", import.meta.url), debug: "castleDebug", port: 5259,
//                      glb: "data/assets/3d/castle", data: "data/castle" });
//
// The page side is the prototype's: `window[debug].ready` once its scene is built, and
// `window[debug].exportGame()`, which calls `saveGameFile(name, base64)` for each file and returns
// their sizes. VELVET_EXPORT_ROOT writes somewhere else than the game (demos/demon-queen), to compare
// two exports.
import { chromium } from "playwright-core";
import { createServer } from "vite";
import { mkdirSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";

const CHROME = "C:/Program Files/Google/Chrome/Application/chrome.exe";

/** GLTFExporter names nodes but not meshes; the game reads mesh names, so copy them over. */
export function nameMeshes(glb) {
  if (glb.length < 20) {
    console.log("short glb", glb.length);
    return glb;
  }
  const jsonLength = glb.readUInt32LE(12);
  const json = JSON.parse(glb.subarray(20, 20 + jsonLength).toString("utf8"));
  for (const node of json.nodes ?? []) if (node.mesh != null && node.name && !json.meshes[node.mesh].name) json.meshes[node.mesh].name = node.name;
  let text = Buffer.from(JSON.stringify(json), "utf8");
  const pad = (4 - (text.length % 4)) % 4;
  text = Buffer.concat([text, Buffer.alloc(pad, 0x20)]);
  const rest = glb.subarray(20 + jsonLength);
  const header = Buffer.alloc(20);
  header.writeUInt32LE(0x46546c67, 0);
  header.writeUInt32LE(2, 4);
  header.writeUInt32LE(20 + text.length + rest.length, 8);
  header.writeUInt32LE(text.length, 12);
  header.writeUInt32LE(0x4e4f534a, 16);
  return Buffer.concat([header, text, rest]);
}

/**
 * Exports a prototype into the game.
 * - `prototype`: the prototype's folder (a file URL or a path).
 * - `debug`: the name of its debug hook on `window`.
 * - `port`: where to serve it while it exports.
 * - `glb`, `data`: where the GLB files and the JSON files go, inside the game.
 * - `screenshot`: a PNG of the page once it is ready (to see it started), if given.
 * - `prepare(page)`: run before the page loads (a test can make the export repeatable with it).
 * - `page`: the prototype's page to open ("/" by default; a second page such as "/combate.html").
 * Returns the files' sizes and the page's errors.
 */
export async function exportGame({ prototype, debug, port, glb, data, screenshot = null, prepare = null, page: pagePath = "/" }) {
  const root = prototype instanceof URL ? fileURLToPath(prototype) : resolve(prototype);
  const game = process.env.VELVET_EXPORT_ROOT ? resolve(process.env.VELVET_EXPORT_ROOT) : resolve(root, "../demon-queen");
  const glbDir = resolve(game, glb), dataDir = resolve(game, data);
  mkdirSync(glbDir, { recursive: true });
  mkdirSync(dataDir, { recursive: true });
  const errors = [];
  const browser = await chromium.launch({ executablePath: CHROME, headless: true, args: ["--use-angle=d3d11", "--enable-gpu", "--ignore-gpu-blocklist"] });
  const server = await createServer({ root, configFile: resolve(root, "vite.config.js"), logLevel: "error", server: { host: "127.0.0.1", port, strictPort: true } });
  try {
    const page = await browser.newPage({ viewport: { width: 1280, height: 720 } });
    page.on("pageerror", (e) => {
      errors.push(e.message);
      console.log("PAGEERROR", e.message);
    });
    page.on("console", (m) => {
      if (m.type() === "error") {
        errors.push(m.text().slice(0, 300));
        console.log("console", m.text().slice(0, 300));
      }
    });
    await page.exposeFunction("saveGameFile", (name, b64) => {
      const to = name.endsWith(".json") ? resolve(dataDir, name) : resolve(glbDir, name);
      const bytes = Buffer.from(b64, "base64");
      console.log("save", name, bytes.length);
      writeFileSync(to, name.endsWith(".glb") ? nameMeshes(bytes) : bytes);
    });
    if (prepare) await prepare(page);
    await server.listen();
    await page.goto(`http://127.0.0.1:${port}${pagePath}?debug=1`);
    await page.waitForFunction((hook) => window[hook]?.ready === true, debug, { timeout: 180000 });
    if (screenshot) await page.screenshot({ path: screenshot });
    const t0 = Date.now();
    const sizes = await page.evaluate((hook) => window[hook].exportGame(), debug);
    let total = 0;
    for (const [n, s] of Object.entries(sizes)) {
      console.log(n.padEnd(34), (s / 1e6).toFixed(2), "MB");
      total += s;
    }
    console.log("total", (total / 1e6).toFixed(1), "MB in", ((Date.now() - t0) / 1000).toFixed(1), "s ->", glbDir);
    return { sizes, errors };
  } finally {
    await browser.close();
    await server.close();
  }
}
