// Runs the velvet-artprep fixtures, docs/formats/fixtures/artprep/. The suite is shared with other readers
// of the format, so each case states its results in the format's own terms: the codes of a refusal, the
// composed prompts and jobs of a profile, and the checks of the gallery page.

import { readFileSync } from "node:fs";
import { parseArtManifest } from "@velvet/assets";
import { describe, expect, it } from "vitest";
import {
  ArtPrepError,
  buildJobs,
  galleryHtml,
  parseIdentity,
  parseProfiles,
  parseResults,
  parseShots,
  parseStyle,
  type ArtPrepErrorCode,
  type ArtPrepProfile,
  type CharacterIdentity,
  type Shot,
  type StyleLayer,
} from "./index";

const FIXTURES = new URL("../../../../docs/formats/fixtures/artprep/", import.meta.url);

/** A shot in a case: either the id of a shot of shots.json, or a shot written out in the case. */
type ShotItem = string | Record<string, unknown>;

/** A build that must give the jobs of an expected file. */
interface BuildCase {
  readonly name: string;
  readonly profile: string;
  readonly shots: readonly ShotItem[];
  readonly expect: string;
}

/** A build that must be refused with an error code. */
interface RefusedCase {
  readonly name: string;
  readonly profile: string;
  readonly shots: readonly ShotItem[];
  readonly seeds?: readonly number[];
  readonly error: ArtPrepErrorCode;
}

/** The checks of the gallery page built from a results document. */
interface GalleryCase {
  readonly results: string;
  readonly contains: readonly string[];
  readonly not_contains: readonly string[];
  readonly counts: Readonly<Record<string, number>>;
}

/** The whole cases file. */
interface Cases {
  readonly profiles: string;
  readonly style: string;
  readonly manifest: string;
  readonly identities: readonly string[];
  readonly shots: string;
  readonly builds: readonly BuildCase[];
  readonly refused: readonly RefusedCase[];
  readonly gallery: GalleryCase;
}

/** One document that must be refused, with the reader it is read by. */
interface InvalidCase {
  readonly name: string;
  readonly as: "profiles" | "style" | "identity" | "shots" | "results";
  readonly text?: string;
  readonly document?: unknown;
  readonly error: ArtPrepErrorCode;
}

/** Reads a file of the fixtures folder. */
function read(path: string): string {
  return readFileSync(new URL(path, FIXTURES), "utf8");
}

/** Reads a JSON file of the fixtures folder. */
function readJson(path: string): unknown {
  return JSON.parse(read(path));
}

/** The codes of the problems that a call reports. Fails the test when the call throws anything else, or nothing. */
function issueCodes(call: () => unknown): ArtPrepErrorCode[] {
  try {
    call();
  } catch (error) {
    if (error instanceof ArtPrepError) return error.issues.map((issue) => issue.code);
    throw error;
  }
  throw new Error("the call did not refuse");
}

const cases = readJson("cases.json") as Cases;
const profiles = parseProfiles(read(cases.profiles));
const style: StyleLayer = parseStyle(read(cases.style));
const identities: readonly CharacterIdentity[] = cases.identities.map((path) => parseIdentity(read(path)));
const manifest = parseArtManifest(read(cases.manifest));
const shots = parseShots(read(cases.shots));
const shotById = new Map<string, Shot>(shots.map((shot) => [shot.id, shot]));

/** The profile with an id. Fails the test when the fixtures have no such profile. */
function profileById(id: string): ArtPrepProfile {
  const profile = profiles.find((candidate) => candidate.id === id);
  if (profile === undefined) throw new Error(`no profile named ${id}`);
  return profile;
}

/** The shots of a case, in their order. A written-out shot is read by the shot reader, as a file would be. */
function shotsOf(items: readonly ShotItem[]): Shot[] {
  return items.map((item) => {
    if (typeof item === "string") {
      const shot = shotById.get(item);
      if (shot === undefined) throw new Error(`no shot named ${item}`);
      return shot;
    }
    const [shot] = parseShots(JSON.stringify({ format: "velvet-artprep", version: 1, kind: "shots", shots: [item] }));
    if (shot === undefined) throw new Error("a written-out shot did not read");
    return shot;
  });
}

describe("velvet-artprep conformance: the documents of the project", () => {
  it("reads the four profiles, in their order", () => {
    expect(profiles.map((profile) => profile.id)).toEqual(["krea2-niji-base", "krea2-reference", "krea2-edit", "generic-sd"]);
  });

  it("reads the style layer and the two identities", () => {
    expect(style.phrases).toHaveLength(4);
    expect(identities.map((one) => one.id)).toEqual(["ada", "bex"]);
  });

  it("reads the nine shots of the fixtures", () => {
    expect(shots.map((shot) => shot.id)).toEqual([
      "ada-master",
      "bex-master",
      "ada-portrait-neutral",
      "ada-portrait-happy",
      "bex-portrait-neutral",
      "pair-morning",
      "basket",
      "ada-wave-edit",
      "ada-scarf-edit",
    ]);
  });
});

describe("velvet-artprep conformance: the composed prompts and jobs of each profile", () => {
  for (const build of cases.builds) {
    it(build.name, () => {
      const jobs = buildJobs(manifest.entries, identities, shotsOf(build.shots), profileById(build.profile), undefined, style);
      expect(jobs).toEqual(readJson(build.expect));
    });
  }
});

describe("velvet-artprep conformance: builds that are refused", () => {
  for (const refused of cases.refused) {
    it(`${refused.name} is refused with "${refused.error}"`, () => {
      const codes = issueCodes(() =>
        buildJobs(manifest.entries, identities, shotsOf(refused.shots), profileById(refused.profile), refused.seeds, style),
      );
      expect(codes).toContain(refused.error);
    });
  }
});

describe("velvet-artprep conformance: documents that are refused", () => {
  const readers = {
    profiles: parseProfiles,
    style: parseStyle,
    identity: parseIdentity,
    shots: parseShots,
    results: parseResults,
  } as const;
  const invalid = readJson("invalid.json") as readonly InvalidCase[];
  for (const testCase of invalid) {
    it(`${testCase.name} is refused with "${testCase.error}"`, () => {
      const text = testCase.text ?? JSON.stringify(testCase.document);
      const reader = readers[testCase.as];
      expect(issueCodes(() => reader(text))).toContain(testCase.error);
    });
  }
});

describe("velvet-artprep conformance: the selection gallery", () => {
  it("shows the candidates in one self-contained page, with every text escaped", () => {
    const results = parseResults(read(cases.gallery.results));
    const html = galleryHtml(results.shots, { title: results.title ?? undefined });
    for (const text of cases.gallery.contains) expect(html).toContain(text);
    for (const text of cases.gallery.not_contains) expect(html).not.toContain(text);
    for (const [pattern, count] of Object.entries(cases.gallery.counts)) {
      expect(html.split(pattern).length - 1, pattern).toBe(count);
    }
    expect(html).not.toMatch(/<script[^>]*\bsrc=/);
    expect(html).not.toMatch(/<link\b/);
  });
});
