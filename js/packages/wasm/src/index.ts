// @velvet/wasm: loads velvet-wasm, the Rust crates compiled to WebAssembly, and hands out its
// typed exports. The module loads once. Browsers and Velvet Desktop fetch it from the `gen/`
// folder next to this package; Node (the tests) reads the same file from disk.
//
// The module is built by `pnpm build:wasm` (run from js/). Import from the package root:
//   import { loadVelvetWasm } from "@velvet/wasm";

/** The typed exports of the module: the functions and classes of `crates/velvet-wasm`. */
export type VelvetWasm = typeof import("../gen/velvet_wasm.js");

/**
 * The handles of the guion API, as types: a library of guiones, a conversation in progress, and
 * the portraits of the dialogue box.
 */
export type { GuionConversation, GuionLibrary, GuionPortraits } from "../gen/velvet_wasm.js";

/** Where the module's bytes come from: a URL of the `.wasm` file, or the bytes themselves. */
export type WasmSource = URL | Uint8Array | ArrayBuffer;

/** Raised when the module cannot be loaded, usually because it has not been built. */
export class WasmLoadError extends Error {
  /**
   * @param message - what went wrong, and what to do about it.
   * @param options - an optional `cause`, the error that the loader caught.
   */
  constructor(message: string, options?: ErrorOptions) {
    super(message, options);
    this.name = "WasmLoadError";
  }
}

/** The part of `fs/promises` that the loader uses in Node. */
interface NodeFsPromises {
  readFile(path: string): Promise<Uint8Array>;
}

/** The part of `node:url` that the loader uses in Node. */
interface NodeUrl {
  /** Node's own URL class: a test environment may replace the global one. */
  readonly URL: new (input: string, base: string) => object;
  fileURLToPath(url: string): string;
}

/** The module names, kept in variables so that browser bundles do not try to resolve them. */
const NODE_FS_PROMISES = "node:fs/promises";
const NODE_URL = "node:url";

/** The load in progress or done; cleared when it fails, so that a later call can retry. */
let loading: Promise<VelvetWasm> | undefined;

/**
 * Loads the module once and returns its typed exports. Later calls return the same exports, and
 * their `source` is ignored.
 *
 * @param source - the bytes or the URL of the `.wasm` file. Default: `gen/velvet_wasm_bg.wasm`
 *   next to this package.
 * @returns the exports of the module.
 * @throws {WasmLoadError} when the module is missing or cannot be started.
 */
export function loadVelvetWasm(source?: WasmSource): Promise<VelvetWasm> {
  if (loading === undefined) {
    loading = instantiate(source).catch((error: unknown) => {
      loading = undefined;
      throw new WasmLoadError(
        "Velvet WebAssembly could not be loaded. Build it with `pnpm build:wasm` from js/ " +
          `(it needs the wasm32-unknown-unknown target and wasm-bindgen-cli 0.2.126): ${describe(error)}`,
        { cause: error },
      );
    });
  }
  return loading;
}

/** Imports the generated glue and starts the module from `source`, or from its default place. */
async function instantiate(source?: WasmSource): Promise<VelvetWasm> {
  const generated = await import("../gen/velvet_wasm.js");
  const input = await resolveSource(source);
  await generated.default({ module_or_path: input });
  return generated;
}

/**
 * The bytes or the URL to start the module from. The default is the file next to this package.
 * A browser fetches a URL. Node cannot fetch a file URL, so there the file is read from disk, by
 * path: the URL is written as text and turned into a path, which works for any URL class.
 *
 * The default address is made with Node's own URL class. A test environment may replace the global
 * one, and its relative addresses do not resolve against this module (happy-dom's do not).
 */
async function resolveSource(source?: WasmSource): Promise<WasmSource> {
  if (!runsInNode()) return source ?? new URL("../gen/velvet_wasm_bg.wasm", import.meta.url);
  if (source instanceof Uint8Array || source instanceof ArrayBuffer) return source;
  const url = (await import(/* @vite-ignore */ NODE_URL)) as NodeUrl;
  const fs = (await import(/* @vite-ignore */ NODE_FS_PROMISES)) as NodeFsPromises;
  const address = source ?? new url.URL("../gen/velvet_wasm_bg.wasm", import.meta.url);
  return fs.readFile(url.fileURLToPath(String(address)));
}

/** True when this code runs in Node.js (which has a `process` with a `node` version). */
function runsInNode(): boolean {
  const proc: unknown = Reflect.get(globalThis, "process");
  if (typeof proc !== "object" || proc === null) return false;
  const versions: unknown = Reflect.get(proc, "versions");
  return (
    typeof versions === "object" &&
    versions !== null &&
    typeof Reflect.get(versions, "node") === "string"
  );
}

/** A message for an error of any kind. */
function describe(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}
