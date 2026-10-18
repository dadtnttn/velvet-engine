// Runs the velvet-art conformance suite, docs/formats/fixtures/art/cases.json. The suite is shared
// with other readers, so each case states its results in the format's own terms: the source and path
// of a lookup, the code of a refusal, the rows of the missing report, and the colour and label of a
// placeholder.

import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { ArtError, type ArtErrorCode } from "./error";
import { parseArtManifest } from "./manifest";
import { placeholderColor, placeholderSvg, shortLabel } from "./placeholder";
import { ArtResolver, type ArtMissingRow } from "./resolver";
import type { ArtFormat, ArtKind, ArtSource } from "./types";

const FIXTURES = new URL("../../../../docs/formats/fixtures/art/", import.meta.url);

/** One lookup of a case. The expectation is either a picture or an error code. */
interface LookupCase {
  readonly id: string;
  readonly variant?: string;
  readonly expect: {
    readonly source?: ArtSource;
    readonly path?: string | null;
    readonly from_variant?: string | null;
    readonly error?: ArtErrorCode;
  };
}

/** One scenario: a set of files, optionally a list of accepted formats, the lookups and the report. */
interface ArtCase {
  readonly name: string;
  readonly files: readonly string[];
  readonly accept?: readonly ArtFormat[];
  readonly lookups: readonly LookupCase[];
  readonly report?: readonly ArtMissingRow[];
}

/** The whole cases file. */
interface Cases {
  readonly manifest: string;
  readonly invalid: readonly { readonly file: string; readonly code: ArtErrorCode }[];
  readonly cases: readonly ArtCase[];
  readonly placeholders: readonly { readonly id: string; readonly kind: ArtKind; readonly label: string; readonly color: string }[];
}

/** Reads a file of the fixtures folder. */
function read(name: string): string {
  return readFileSync(new URL(name, FIXTURES), "utf8");
}

/** The code of the ArtError that a call throws. Fails the test when the call throws anything else, or nothing. */
function codeThrown(call: () => unknown): string {
  try {
    call();
  } catch (error) {
    if (error instanceof ArtError) return error.code;
    throw error;
  }
  throw new Error("the call did not throw");
}

const cases = JSON.parse(read("cases.json")) as Cases;
const manifest = parseArtManifest(read(cases.manifest));

describe("velvet-art conformance: manifests that are refused", () => {
  for (const testCase of cases.invalid) {
    it(`${testCase.file} is refused with "${testCase.code}"`, () => {
      const text = read(testCase.file);
      let code = "accepted";
      let issueCodes: string[] = [];
      try {
        parseArtManifest(text);
      } catch (error) {
        if (!(error instanceof ArtError)) throw error;
        code = error.code;
        issueCodes = error.issues.map((issue) => issue.code);
      }
      expect(code).toBe(testCase.code);
      expect(issueCodes).toContain(testCase.code);
    });
  }
});

describe("velvet-art conformance: lookups and reports", () => {
  for (const testCase of cases.cases) {
    it(testCase.name, () => {
      // The conformance cases check the format's rules, so the resolver runs strict.
      const resolver = new ArtResolver(manifest, testCase.files, testCase.accept === undefined ? { strict: true } : { accept: testCase.accept, strict: true });
      for (const lookup of testCase.lookups) {
        const label = lookup.variant === undefined ? lookup.id : `${lookup.id}#${lookup.variant}`;
        if (lookup.expect.error !== undefined) {
          expect(codeThrown(() => resolver.resolve(lookup.id, lookup.variant)), label).toBe(lookup.expect.error);
          continue;
        }
        const picture = resolver.resolve(lookup.id, lookup.variant);
        expect({ label, source: picture.source, path: picture.path, from_variant: picture.variant }).toEqual({
          label,
          source: lookup.expect.source,
          path: lookup.expect.path,
          from_variant: lookup.expect.from_variant,
        });
      }
      if (testCase.report !== undefined) {
        expect(resolver.missingReport().missing).toEqual(testCase.report);
      }
    });
  }
});

describe("velvet-art conformance: placeholders", () => {
  for (const item of cases.placeholders) {
    it(`${item.id} is labelled "${item.label}" and coloured ${item.color}`, () => {
      expect(shortLabel(item.id, item.kind)).toBe(item.label);
      expect(placeholderColor(item.id, item.kind)).toBe(item.color);
      const svg = placeholderSvg(item.id, item.kind, [64, 64]);
      expect(svg).toContain(`aria-label="${item.label}"`);
      // A background paints its colour into the gradient, not into a fill.
      if (item.kind !== "background") expect(svg).toContain(item.color);
    });
  }
});
