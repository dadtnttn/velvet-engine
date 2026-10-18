// The entry of velvet-js: reads the command name, finds the command and runs it. The bin file calls
// `main` with the arguments after the program name.

import { helpText, parseArgs, UsageError, type CommandSpec } from "./command";
import { checkCommand } from "./check/command";
import { i18nCommand } from "./i18n/command";
import { newCommand } from "./new/command";
import { terminalOutput, type Output } from "./output";

/** The commands, in the order the help text lists them. */
export const COMMANDS: readonly CommandSpec[] = [newCommand, checkCommand, i18nCommand];

/** The help for the whole program: the commands and how to ask for each one's help. */
function programHelp(): string[] {
  const width = Math.max(...COMMANDS.map((command) => command.name.length)) + 4;
  return [
    "Usage: velvet-js <command> [options]",
    "",
    "The command-line tools of Velvet JS. Run them from js/ with `pnpm velvet-js <command>`.",
    "",
    "Commands:",
    ...COMMANDS.map((command) => `  ${command.name.padEnd(width)}${command.summary}`),
    "",
    "Run `velvet-js <command> --help` for a command's options.",
    "Exit code: 0 on success, 1 when a command fails (for example, a game has errors).",
  ];
}

/**
 * Runs one velvet-js command line.
 * @param argv - the arguments after the program name, such as `["check", "my-game"]`.
 * @param output - where the text goes. Defaults to the terminal.
 * @returns the exit code.
 */
export async function main(argv: readonly string[], output: Output = terminalOutput): Promise<number> {
  const [name, ...rest] = argv;
  if (name === undefined) {
    for (const line of programHelp()) output.err(line);
    return 1;
  }
  if (name === "--help" || name === "-h" || name === "help") {
    for (const line of programHelp()) output.out(line);
    return 0;
  }
  const command = COMMANDS.find((candidate) => candidate.name === name);
  if (command === undefined) {
    const names = COMMANDS.map((candidate) => candidate.name).join(", ");
    output.err(`velvet-js: unknown command "${name}". The commands are: ${names}.`);
    return 1;
  }

  let json = false;
  try {
    const args = parseArgs(rest, command.options);
    json = args.flags.has("json");
    if (args.flags.has("help")) {
      for (const line of helpText(command)) output.out(line);
      return 0;
    }
    return await command.run(args, output);
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    if (json) {
      output.out(JSON.stringify({ ok: false, command: command.name, error: message }, null, 2));
    } else {
      output.err(`velvet-js ${command.name}: ${message}`);
      if (error instanceof UsageError) {
        output.err(`Run "velvet-js ${command.name} --help" for the options.`);
      }
    }
    return 1;
  }
}
