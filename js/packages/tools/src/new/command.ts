// The `new` command: scaffolds a game folder from the template, and says what to do next.

import { resolve } from "node:path";
import { UsageError, type CommandSpec } from "../command";
import { ENGINE_JS_DIR } from "../paths";
import type { Output } from "../output";
import { scaffoldGame } from "./scaffold";

/** The `new` command. */
export const newCommand: CommandSpec = {
  name: "new",
  summary: "create a new game folder from the template (package.json, Vite, a title screen, locales, a test)",
  usage: "<dir> --id <id> --title <title> [--engine <js dir>] [--force]",
  options: [
    { name: "id", takesValue: true, valueName: "<id>", help: "the game's id: lowercase letters, digits, - or _" },
    { name: "title", takesValue: true, valueName: "<title>", help: "the title that the window and the page show" },
    {
      name: "engine",
      takesValue: true,
      valueName: "<js dir>",
      help: "the engine's js folder (default: the one this tool belongs to)",
    },
    { name: "force", takesValue: false, help: "write into a folder that is not empty, and overwrite files of the same name" },
  ],
  details: [
    "The game links the engine's packages with `link:` paths computed from its folder, so keep the engine",
    "where it is, or pass --engine again when you move the game. Then, in the new folder:",
    "  pnpm install     once, to link the packages and install Vite, TypeScript and Vitest",
    "  pnpm dev         the web side, on http://localhost:5173",
    "  pnpm test        the tests;  pnpm typecheck  the types;  pnpm check:data  the game's data",
    "Velvet Desktop runs the game in a window: from the engine's repository, `cargo run -p velvet-desktop --`",
    "with this folder and `--dev http://localhost:5173`. Release it with `velvet build --runtime js`.",
  ],
  async run(args, output: Output) {
    if (args.positional.length !== 1) {
      throw new UsageError("give one folder for the new game, such as my-game");
    }
    const id = args.values.get("id");
    const title = args.values.get("title");
    if (id === undefined) throw new UsageError("--id is required: the game's id, such as my-game");
    if (title === undefined) throw new UsageError("--title is required: the title the window shows, such as \"My Game\"");

    const folder = args.positional[0] ?? "";
    const engine = resolve(args.values.get("engine") ?? ENGINE_JS_DIR);
    const json = args.flags.has("json");
    const result = await scaffoldGame({
      folder,
      id,
      title,
      engine,
      force: args.flags.has("force"),
    });

    if (json) {
      output.out(JSON.stringify({ ok: true, folder: result.folder, id, title, files: result.files }, null, 2));
    } else {
      output.out(`created ${result.folder} with ${result.files.length} files:`);
      for (const file of result.files) output.out(`  ${file}`);
      output.out(`next: cd ${result.folder}, then pnpm install and pnpm dev (see its README.md)`);
    }
    return 0;
  },
};
