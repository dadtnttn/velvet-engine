# @velvet/assets

The art of a Velvet JS game, in the `velvet-art` format (`docs/formats/ART_MANIFEST.md`). A game asks
for an **art id** (`character.lyra.portrait`), never for a file. The manifest says what each id is, the
files in `art/` show it, and an id with no file yet shows a generated placeholder. Provisional art can
therefore be swapped for real images later without touching the code.

## A game

```ts
import { ArtResolver, artElement, parseArtManifest } from "@velvet/assets";

// The manifest, and the list of art files as paths relative to art/. The list can come from the
// desktop bridge, a fetched index, or a Vite glob:
//   const files = Object.keys(import.meta.glob("/art/**/*.{avif,webp,png,svg}")).map((p) => p.slice("/art/".length));
const manifest = parseArtManifest(await (await fetch("art/manifest.json")).text());
const files = ["portrait/character.lyra.portrait.happy.webp", "card/card.wood.png", "background/bg.forest.webp"];

const art = new ArtResolver(manifest, files, { baseUrl: "art/" });

// Decode what the first scene needs. The callback drives a loading bar.
const { broken } = await art.preload(
  ["bg.forest", { id: "character.lyra.portrait", variant: "happy" }],
  ({ done, total }) => setProgress(done / total), // setProgress is the game's own function
);
if (broken.length > 0) console.warn("could not decode:", broken);

// A picture as data: its source, its logical size and its pivot.
const picture = art.resolve("character.lyra.portrait", "happy");
console.log(picture.source, picture.url); // "webp" "art/portrait/character.lyra.portrait.happy.webp"

// An element for the page, sized in logical pixels. The 16:9 stage scales it with the rest.
board.append(artElement(art, "card.wood")); // board is the game's own element

// What still needs art, in the report format of the specification.
console.log(JSON.stringify(art.missingReport(), null, 2));
```

## API

| Call | What it does |
|---|---|
| `parseArtManifest(json)` | Reads the text of `art/manifest.json` into its normal form. Throws `ArtError`; its `issues` list every problem, each with a path. |
| `new ArtResolver(manifest, files, options?)` | Builds the resolver from a manifest and the paths of the art files. Options: `baseUrl` (default `art/`), `accept` (the formats the reader can show; default all four), `strict`, `onUnknown`, and `placeholder` (a runtime's own placeholder pictures, see below). Throws `ArtError` for a repeated id or file name. |
| `resolve(id, variant?)` | `{ id, variant, kind, size, anchor, source, url, path }`. `source` is `avif`, `webp`, `png`, `svg` or `placeholder`. By default an unknown id draws a 128×128 placeholder and an unknown variant falls back to the base, each reported once through `onUnknown` (console.warn); with `{ strict: true }` they throw `unknown-id` / `unknown-variant`. |
| `preload(requests, onProgress?)` | Decodes the pictures of some ids (strings, or `{ id, variant }`) with `Image.decode()`. Resolves to `{ broken }`, the paths that could not be decoded. Without a browser nothing is decoded, and every request counts as done. |
| `markBroken(path)` | Marks a file that the browser cannot show. Later resolutions skip it and move on to the next format. |
| `missingReport()` | The missing-art report: each id with no base file, and each id with a variant that has no file of its own. |
| `artElement(resolver, id, variant?)` | An `<img>`, or an inline `<svg>` for a placeholder, sized in logical pixels. Its pivot is the CSS transform origin. A refused file falls back to the next format, and then to the placeholder. |
| `placeholderSvg(id, kind, size, label?)` | The placeholder picture of an id, as SVG text. |
| `placeholderColor(id, kind)`, `shortLabel(id, kind)` | The placeholder's colour and label, as the format defines them. |

The package also exports the format constants (`ART_FORMAT`, `ART_VERSION`, `ART_REPORT_FORMAT`,
`ART_KINDS`, `ART_FORMATS`, `ART_MAX_SIDE`), `isArtKind`, and the types `ArtManifest`, `ArtEntry`,
`ArtKind`, `ArtFormat`, `ArtSource`, `ArtRequest`, `ResolvedArt`, `ArtMissingReport`, `ArtMissingRow`,
`PreloadProgress`, `PreloadResult` and `ArtError`.

## Rules to know

- A variant falls back to its base id, never the other way round. A variant's own file beats a better
  base file.
- The format order is AVIF, WebP, PNG, then a hand-made SVG. A placeholder comes last, and it is the
  same for every variant of an id.
- Art elements are sized in logical pixels with CSS, and they scale smoothly. They never set
  `image-rendering: pixelated`.
- A placeholder is made from the id: the same id always gets the same colour and label.
- A runtime may draw its own placeholders with the `placeholder` option. The hook gets the entry and the variant
  asked for (null for the base picture), and returns SVG text, or null for the generic placeholder. The answer is
  cached, and it is still a placeholder: source `placeholder`, no path. `artElement` shows the same SVG.

## Tests

From `js/`, run `pnpm test` and `pnpm typecheck`. The DOM tests run in happy-dom. The conformance
fixtures in `docs/formats/fixtures/art/` are run by `fixtures.test.ts`.
