#!/usr/bin/env node
// velvet-js: the command-line tools of Velvet JS (new, check, i18n). Run it from js/ with
// `pnpm velvet-js <command>`, or directly: `node packages/tools/bin/velvet-js.mjs <command>`.
//
// The packages are TypeScript source with no build step, and they import each other without file
// extensions (the bundler rule that Vite follows). Node 25 strips the types of a .ts file by itself,
// but it does not add a missing extension, so this file installs a resolver hook first: a relative
// import that finds no file is tried again with `.ts`, then with `/index.ts`. The hook is part of
// Node's own module API (`module.registerHooks`), so the tools need no dependency and no build.

import { registerHooks } from "node:module";

registerHooks({
  resolve(specifier, context, nextResolve) {
    try {
      return nextResolve(specifier, context);
    } catch (error) {
      const relative = specifier.startsWith("./") || specifier.startsWith("../");
      if (!relative || context.parentURL === undefined) throw error;
      for (const candidate of [`${specifier}.ts`, `${specifier}/index.ts`]) {
        try {
          return nextResolve(candidate, context);
        } catch {
          // Not this one: try the next candidate.
        }
      }
      throw error;
    }
  },
});

const { main } = await import(new URL("../src/cli.ts", import.meta.url).href);
process.exitCode = await main(process.argv.slice(2));
