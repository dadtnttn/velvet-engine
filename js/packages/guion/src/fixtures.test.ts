// The shared guion fixtures (docs/formats/fixtures/guion) through this package. The Rust test in
// crates/velvet-wasm reads the same files, so both runtimes must give the same results.

import { readdir, readFile } from "node:fs/promises";
import { describe, expect, it } from "vitest";
import {
  GuionError,
  GuionLibrary,
  parseGuion,
  type CastSpec,
  type Conversation,
  type GuionFile,
  type GuionVars,
  type Shown,
} from "./index";

/** The folder of the shared fixtures, found from this file (src, guion, packages, js, root). */
const FIXTURES = new URL("../../../../docs/formats/fixtures/guion/", import.meta.url);

/** An expected entry lookup. */
interface EntryCase {
  readonly who: string | null;
  readonly place: string | null;
  readonly vars: GuionVars;
  readonly scene: string | null;
}

/** One step of a run: an action (none for the first box), and what the box shows after it. */
interface StepCase {
  readonly do?: "next" | { readonly choose: number };
  readonly expect: Shown | { readonly code: string };
}

/** A run: a scene started with some variables, its steps, and the variables at the end. */
interface RunCase {
  readonly scene: string;
  readonly vars: GuionVars;
  readonly steps: readonly StepCase[];
  readonly varsAfter: GuionVars;
}

/** One `<name>.expected.json` file, with the guion it names. */
interface Fixture {
  readonly file: string;
  readonly cast?: CastSpec;
  readonly parse: GuionFile;
  readonly entries?: readonly EntryCase[];
  readonly runs: readonly RunCase[];
}

/** A fixture, loaded: its name, its expected output and the text of its guion. */
interface LoadedFixture {
  readonly name: string;
  readonly fixture: Fixture;
  readonly text: string;
}

/** Reads every fixture in the folder, in name order. */
async function loadFixtures(): Promise<LoadedFixture[]> {
  const names = (await readdir(FIXTURES)).filter((name) => name.endsWith(".expected.json")).sort();
  return Promise.all(
    names.map(async (name) => {
      const fixture = JSON.parse(await readFile(new URL(name, FIXTURES), "utf8")) as Fixture;
      const text = await readFile(new URL(fixture.file, FIXTURES), "utf8");
      return { name, fixture, text };
    }),
  );
}

const fixtures = await loadFixtures();

/** A library with the fixture's cast and its one guion file. */
async function libraryFor(loaded: LoadedFixture): Promise<GuionLibrary> {
  const library = await GuionLibrary.create(loaded.fixture.cast);
  library.add(loaded.fixture.file, loaded.text);
  return library;
}

/** The error that `act` throws, or undefined when it returns. */
function thrownBy(act: () => unknown): unknown {
  try {
    act();
  } catch (error) {
    return error;
  }
  return undefined;
}

/** Checks one step: the box equals the expected box, or the action fails with the expected code. */
function checkStep(label: string, act: () => Shown, expected: StepCase["expect"]): void {
  if ("code" in expected) {
    const error = thrownBy(act);
    expect(error, label).toBeInstanceOf(GuionError);
    expect((error as GuionError).code, label).toBe(expected.code);
  } else {
    expect(act(), label).toEqual(expected);
  }
}

/** Plays one run and checks every step, then the variables. */
function playRun(library: GuionLibrary, run: RunCase): void {
  const conversation: Conversation = library.start(run.scene, run.vars);
  run.steps.forEach((step, index) => {
    const label = `${run.scene}, step ${index}`;
    if (step.do === undefined) {
      expect(index, label).toBe(0);
      checkStep(label, () => conversation.current(), step.expect);
    } else if (step.do === "next") {
      checkStep(label, () => conversation.next(), step.expect);
    } else {
      const choice = step.do.choose;
      checkStep(label, () => conversation.choose(choice), step.expect);
    }
  });
  expect(conversation.vars, `${run.scene}: the variables after the run`).toEqual(run.varsAfter);
}

describe.each(fixtures)("guion fixture $name", ({ fixture, text }) => {
  it("parses to the expected summary", async () => {
    const parsed = await parseGuion(text, { file: fixture.file, cast: fixture.cast });
    expect(parsed).toEqual(fixture.parse);
  });

  it("finds the expected scene for each entry", async () => {
    const library = await libraryFor({ name: fixture.file, fixture, text });
    for (const entry of fixture.entries ?? []) {
      const found = library.entry(
        { who: entry.who ?? undefined, place: entry.place ?? undefined },
        entry.vars,
      );
      expect(found, JSON.stringify(entry)).toBe(entry.scene);
    }
  });

  for (const [index, run] of fixture.runs.entries()) {
    it(`plays run ${index + 1}, from ${run.scene}, as the expected boxes`, async () => {
      const library = await libraryFor({ name: fixture.file, fixture, text });
      playRun(library, run);
    });
  }
});
