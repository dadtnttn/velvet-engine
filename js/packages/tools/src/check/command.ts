// The `check` command: prints the problems of a game folder, then a summary, and exits 1 on an error.

import { resolve } from "node:path";
import { UsageError, type CommandSpec } from "../command";
import { formatIssue } from "../issues";
import type { Output } from "../output";
import { checkGame, type CheckResult } from "./check";

/** The lines of the human summary of a check. */
function summaryLines(result: CheckResult): string[] {
  const c = result.counts;
  const parts = [
    `${c.catalogs} catalog${c.catalogs === 1 ? "" : "s"}`,
    `${c.cases} case${c.cases === 1 ? "" : "s"}`,
    `${c.stat_schemas} stat schema${c.stat_schemas === 1 ? "" : "s"}`,
    `${c.characters} character sheet${c.characters === 1 ? "" : "s"}`,
    `${c.art_manifests} art manifest${c.art_manifests === 1 ? "" : "s"}`,
    `${c.audio_manifests} audio manifest${c.audio_manifests === 1 ? "" : "s"}`,
    `${c.achievement_definitions} achievement definition${c.achievement_definitions === 1 ? "" : "s"}`,
    `${c.tile_maps} tile map${c.tile_maps === 1 ? "" : "s"}`,
    `${c.tilesets} tileset${c.tilesets === 1 ? "" : "s"}`,
    `${c.artprep} artprep document${c.artprep === 1 ? "" : "s"}`,
    `${c.guiones} ${c.guiones === 1 ? "guion" : "guiones"}`,
    `${c.locale_files} locale file${c.locale_files === 1 ? "" : "s"} in ${c.languages} language${c.languages === 1 ? "" : "s"}`,
  ];
  const verdict = result.errors === 0 ? "ok" : "failed";
  return [
    `checked ${result.game}: ${parts.join(", ")}`,
    `${result.game}: ${verdict}, ${result.errors} error${result.errors === 1 ? "" : "s"}, ${result.warnings} warning${result.warnings === 1 ? "" : "s"}`,
  ];
}

/** The `check` command. */
export const checkCommand: CommandSpec = {
  name: "check",
  summary: "check a game's data against every format, and the links between its files",
  usage: "<game dir>",
  options: [],
  details: [
    "Finds the data by its format, not by its place: stacks catalogs and cases, stat schemas and character",
    "sheets, art manifests, audio manifests, achievement definitions, Tiled maps and tilesets, artprep documents,",
    "guiones and locale catalogs. Each file is read by the package that owns its format. Then the links are",
    "checked: art ids and variants, the icons of achievements, identities of the characters a shot shows, the id",
    "keys of the data (which must exist in the source language), and the files that the data names: the sound",
    "files of an audio manifest and the pictures of the tilesets, which must exist in the game.",
    "The discovery rules are in packages/tools/README.md, under \"Where the data is\".",
    "Exit code 0 when there are no errors (warnings do not fail the check), 1 when there is an error.",
  ],
  async run(args, output: Output) {
    if (args.positional.length !== 1) {
      throw new UsageError("give one game folder, such as my-game");
    }
    const folder = args.positional[0] ?? "";
    const result = await checkGame(resolve(folder));
    const json = args.flags.has("json");

    if (json) {
      output.out(
        JSON.stringify(
          {
            ok: result.errors === 0,
            game: result.game,
            folder,
            counts: result.counts,
            errors: result.errors,
            warnings: result.warnings,
            issues: result.issues,
          },
          null,
          2,
        ),
      );
    } else {
      for (const issue of result.issues) output.out(formatIssue(issue));
      for (const line of summaryLines(result)) output.out(line);
    }
    return result.errors === 0 ? 0 : 1;
  },
};
