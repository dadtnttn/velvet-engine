// Builds velvet-wasm for Velvet JS: cargo compiles the crate to WebAssembly, and wasm-bindgen
// writes the JavaScript glue and the typings into gen/ (git-ignored).
//
// Run from js/ with `pnpm build:wasm` (always builds) or `node packages/wasm/build.mjs --if-stale`
// (builds only when gen/ is missing or older than the Rust sources: what `pnpm test` and
// `pnpm typecheck` run first). Needs the wasm32-unknown-unknown Rust target and wasm-bindgen-cli
// 0.2.126 (`cargo install wasm-bindgen-cli --version 0.2.126`).

import { spawnSync } from "node:child_process";
import { existsSync, mkdirSync, readdirSync, statSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, "../../..");
const gen = join(here, "gen");
const target = "wasm32-unknown-unknown";

/**
 * Runs a command in the repository root, showing its output. Throws when it cannot start or
 * exits with an error.
 * @param {string} program
 * @param {string[]} args
 */
function run(program, args) {
  const result = spawnSync(program, args, { cwd: root, stdio: "inherit" });
  if (result.error) {
    const hint =
      program === "wasm-bindgen"
        ? " Install it with `cargo install wasm-bindgen-cli --version 0.2.126`."
        : "";
    throw new Error(`could not run ${program}: ${result.error.message}.${hint}`);
  }
  if (result.status !== 0) {
    throw new Error(`${program} ${args.join(" ")} failed (exit ${result.status})`);
  }
}

/**
 * The directory where cargo puts its output (the workspace's target directory).
 * @returns {string}
 */
function cargoTargetDirectory() {
  const result = spawnSync("cargo", ["metadata", "--no-deps", "--format-version", "1"], {
    cwd: root,
    encoding: "utf8",
  });
  if (result.status !== 0) {
    throw new Error(`cargo metadata failed: ${result.stderr}`);
  }
  return JSON.parse(result.stdout).target_directory;
}

/**
 * A byte count for people: "1.23 MB (1234567 bytes)".
 * @param {number} bytes
 */
function describeSize(bytes) {
  const mb = bytes / (1024 * 1024);
  return `${mb.toFixed(2)} MB (${bytes} bytes)`;
}

/**
 * The Rust inputs of the module: the crates it is built from, and the workspace manifests. Add a
 * crate here when velvet-wasm starts to depend on it.
 */
const RUST_INPUTS = [
  "Cargo.toml",
  "Cargo.lock",
  "crates/velvet-wasm",
  "crates/velvet-guion",
  "crates/velvet-stacks",
];

/**
 * The newest modification time of `path`: the file itself, or any file under the folder.
 * @param {string} path
 * @returns {number}
 */
function newestModification(path) {
  if (!existsSync(path)) return 0;
  const stats = statSync(path);
  if (!stats.isDirectory()) return stats.mtimeMs;
  let newest = 0;
  for (const entry of readdirSync(path)) {
    if (entry === "target" || entry === "node_modules") continue;
    newest = Math.max(newest, newestModification(join(path, entry)));
  }
  return newest;
}

/**
 * True when the module is missing, or was built before the newest Rust input changed.
 * @returns {boolean}
 */
export function needsBuild() {
  const wasm = join(gen, "velvet_wasm_bg.wasm");
  if (!existsSync(join(gen, "velvet_wasm.js")) || !existsSync(wasm)) return true;
  const built = statSync(wasm).mtimeMs;
  return RUST_INPUTS.some((input) => newestModification(join(root, input)) > built);
}

/** Builds the module into gen/. Throws when a step fails. */
export function build() {
  run("cargo", ["build", "-p", "velvet-wasm", "--target", target, "--profile", "release"]);
  const built = join(cargoTargetDirectory(), target, "release", "velvet_wasm.wasm");
  mkdirSync(gen, { recursive: true });
  run("wasm-bindgen", [
    "--target",
    "web",
    "--out-dir",
    gen,
    "--out-name",
    "velvet_wasm",
    built,
  ]);
  const size = statSync(join(gen, "velvet_wasm_bg.wasm")).size;
  console.log(`velvet_wasm_bg.wasm: ${describeSize(size)} (in ${gen})`);
}

// Run as a program (not imported by the test setup).
if (process.argv[1] !== undefined && import.meta.url === pathToFileURL(process.argv[1]).href) {
  try {
    if (!process.argv.includes("--if-stale") || needsBuild()) build();
  } catch (error) {
    console.error(`pnpm build:wasm: ${error instanceof Error ? error.message : String(error)}`);
    process.exit(1);
  }
}
