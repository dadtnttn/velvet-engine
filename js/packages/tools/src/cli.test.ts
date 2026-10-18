// The command line: argument parsing, the help texts, the exit codes and the JSON error form.

import { describe, expect, it } from "vitest";
import { helpText, parseArgs, UsageError, type OptionSpec } from "./command";
import { COMMANDS, main } from "./cli";
import { bufferedOutput } from "./output";
import { tempFolder } from "./test-support";

const OPTIONS: readonly OptionSpec[] = [
  { name: "id", takesValue: true, help: "the id" },
  { name: "title", takesValue: true, help: "the title" },
  { name: "force", takesValue: false, help: "overwrite" },
];

describe("parseArgs", () => {
  it("reads positional arguments, values in both forms, and flags", () => {
    const parsed = parseArgs(["dir", "--id", "x", "--title=My Game", "--force", "--json"], OPTIONS);
    expect(parsed.positional).toEqual(["dir"]);
    expect(parsed.values.get("id")).toBe("x");
    expect(parsed.values.get("title")).toBe("My Game");
    expect(parsed.flags.has("force")).toBe(true);
    expect(parsed.flags.has("json")).toBe(true);
  });

  it("reads -h as --help, and keeps everything after -- as positional", () => {
    const parsed = parseArgs(["-h", "--", "--not-an-option"], OPTIONS);
    expect(parsed.flags.has("help")).toBe(true);
    expect(parsed.positional).toEqual(["--not-an-option"]);
  });

  it("refuses an unknown option, a missing value and a value given to a flag", () => {
    expect(() => parseArgs(["--nope"], OPTIONS)).toThrow(UsageError);
    expect(() => parseArgs(["--id"], OPTIONS)).toThrow(/needs a value/u);
    expect(() => parseArgs(["--id", "--force"], OPTIONS)).toThrow(/needs a value/u);
    expect(() => parseArgs(["--force=yes"], OPTIONS)).toThrow(/is a flag/u);
  });
});

describe("helpText", () => {
  it("shows the usage, the options and the built-in options", () => {
    const text = helpText({
      name: "demo",
      summary: "does a demo",
      usage: "<dir> --id <id>",
      options: OPTIONS,
      details: ["an extra line"],
      run: async () => 0,
    }).join("\n");
    expect(text).toContain("Usage: velvet-js demo <dir> --id <id>");
    expect(text).toContain("--id <value>");
    expect(text).toContain("--json");
    expect(text).toContain("--help");
    expect(text).toContain("an extra line");
  });
});

describe("main", () => {
  it("prints the program help to standard error with no command, and exits 1", async () => {
    const out = bufferedOutput();
    expect(await main([], out.output)).toBe(1);
    expect(out.stderr[0]).toBe("Usage: velvet-js <command> [options]");
  });

  it("prints the program help with --help, and exits 0", async () => {
    const out = bufferedOutput();
    expect(await main(["--help"], out.output)).toBe(0);
    for (const command of COMMANDS) expect(out.stdout.join("\n")).toContain(command.name);
  });

  it("refuses an unknown command", async () => {
    const out = bufferedOutput();
    expect(await main(["explode"], out.output)).toBe(1);
    expect(out.stderr.join("\n")).toContain('unknown command "explode"');
  });

  it("shows a command's own help", async () => {
    for (const command of COMMANDS) {
      const out = bufferedOutput();
      expect(await main([command.name, "--help"], out.output)).toBe(0);
      expect(out.stdout[0]).toBe(`Usage: velvet-js ${command.name} ${command.usage}`.trimEnd());
    }
  });

  it("reports a usage error with a hint, and exits 1", async () => {
    const out = bufferedOutput();
    expect(await main(["check"], out.output)).toBe(1);
    expect(out.stderr[0]).toContain("give one game folder");
    expect(out.stderr[1]).toContain('Run "velvet-js check --help"');
  });

  it("prints a usage error as JSON when --json is given", async () => {
    const out = bufferedOutput();
    expect(await main(["check", "--json"], out.output)).toBe(1);
    const parsed: unknown = JSON.parse(out.stdout.join("\n"));
    expect(parsed).toMatchObject({ ok: false, command: "check" });
  });

  it("refuses two folders for check, and an option that does not exist", async () => {
    const out = bufferedOutput();
    expect(await main(["check", "a", "b"], out.output)).toBe(1);
    expect(await main(["check", "--where"], bufferedOutput().output)).toBe(1);
  });

  it("fails a game folder that has no manifest, with a located error", async () => {
    const folder = await tempFolder();
    const out = bufferedOutput();
    expect(await main(["check", folder], out.output)).toBe(1);
    expect(out.stdout.join("\n")).toContain("velvet-game.json: (file): error: velvet-game.json is missing");
  });
});
