# @velvet/artprep

Tooling that prepares the pictures of a game for a diffusion pipeline, from data. It is not part of
the game's runtime. A project writes its generation profiles, its style, one identity per character and
its shots; this package composes each picture's prompt from three layers (style, identity, variation),
writes one job per shot for an external batch runner, and writes the selection gallery where a person
chooses the candidates. It runs no model and writes no art file. The rules are in
`docs/formats/ART_PREP.md`.

## A project

```ts
import { parseArtManifest } from "@velvet/assets";
import {
  buildJobs,
  galleryHtml,
  parseIdentity,
  parseProfiles,
  parseResults,
  parseShots,
  parseStyle,
} from "@velvet/artprep";

// The documents, as text: read them from the project's files.
const profile = parseProfiles(profilesText).find((candidate) => candidate.id === "krea2-reference");
const style = parseStyle(styleText);
const identities = [adaText, bexText].map((text) => parseIdentity(text));
const manifest = parseArtManifest(manifestText);
const shots = parseShots(shotsText);
if (profile === undefined) throw new Error("no such profile");

// One job per shot. Six seeds per shot by default; the style is the project's.
const jobs = buildJobs(manifest.entries, identities, shots, profile, undefined, style);
for (const job of jobs) {
  // The batch runner reads job.prompt, job.negative, job.references (art ids), job.size and
  // job.outputs, and writes one file per output: job.outputs[i].name plus the extension.
  console.log(job.shot, job.prompt);
}

// After the runner: its results, as a results document, become the gallery page.
const results = parseResults(resultsText);
const page = galleryHtml(results.shots, { title: results.title ?? undefined });
// Save `page` as an HTML file next to the candidate pictures.
```

A composed prompt, for the shot `ada-master` of the fixtures with the profile `krea2-niji-base`:

```text
@NJSW33T Soft-edged illustration, warm morning light, flat painted colours, clean ink outlines. Ada, human, slim build, light olive skin, short curly dark brown hair tied back with a red ribbon, green eyes, a small silver ring on her left hand, a cream apron over a blue linen dress with rolled sleeves, no horns, no tail, matte dry skin. Full body, standing, arms relaxed at her sides, plain white background, even soft studio light, cream, blue and brown.
```

## API

| Call | What it does |
|---|---|
| `parseProfiles(json)` | Reads a profiles document into its profiles, with the defaults filled in. |
| `parseStyle(json)` | Reads the style layer of a project. |
| `parseIdentity(json)` | Reads the identity layer of one character. Its id is the character's id. |
| `parseShots(json)` | Reads the shots: the pictures to make. |
| `parseResults(json)` | Reads a runner's results: the candidates of each shot. |
| `composePrompt(style, identity, variation, profile)` | Composes one picture's prompt and negative prompt. `identity` is one identity or a list of them. |
| `buildJobs(entries, identities, shots, profile, seeds?, style?)` | Builds one job per shot, against the art manifest's entries. `seeds` defaults to 101 to 106. |
| `outputName(art, variant, seed)` | The name of one output: `<art-id>[.<variant>]_<seed>`. |
| `jobSize(box, rule)` | The size in pixels of a job, from the art box and the profile's size rule. |
| `galleryHtml(shots, options?)` | One self-contained HTML page: the candidates of each shot, each with its code and a copy button. |

Every reader and builder throws `ArtPrepError` with `code` (the first problem's rule) and `issues` (every
problem, each with its path). The codes are listed in `docs/formats/ART_PREP.md`.

The package also exports the format constants (`ARTPREP_FORMAT`, `ARTPREP_VERSION`, `ARTPREP_KINDS`,
`DEFAULT_SEEDS`, `MAX_SEEDS`, `PROMPT_LAYERS`, `VARIATION_FIELDS`) and the types of the documents.

## Rules to know

- Prose profiles write sentences, with no quality tags and no weights. Tag profiles write a list, and
  may write weights with the paren syntax.
- A lacked feature (for example horns) is written as `no horns` in the text when the profile takes no
  negative prompt, and as a negative when it does. It is left out of the negative when another character
  in the same picture has it.
- Only the `prompt`, `style` and `negative` names of the art manifest's `meta` are read. The `notes` are
  for people.
- A shot makes six candidates by default. The gallery's codes are the output names: the file name
  without the extension.
- An image source in the gallery must be a relative path, an https address or a data:image URL.

## Tests

From `js/`, run `pnpm test` and `pnpm typecheck`. `fixtures.test.ts` runs every file of
`docs/formats/fixtures/artprep/`: the documents, the composed prompts and jobs of each profile, the
refused builds and documents, and the gallery checks. The other tests cover the composition rules, the
job builder and the gallery one rule at a time.
