# velvet-artbuild

Fills a Velvet JS game's `art/` folder from source pictures: AI generations, drawings and photographs of
any size. Each picture is fitted to the box of its art id in the art manifest, written at the output
size as PNG, and as WebP and AVIF when ffmpeg is installed. A report lists what was built, what is
missing and what needs a look.

A tool crate, not a game. The formats are in `docs/formats/ART_MANIFEST.md` (the manifest the game reads)
and `docs/formats/ART_BUILD.md` (the optional build config, the source rules, the stamps and the report).

## Usage

From the game folder (the one with `art/manifest.json`), with the sources in `art-src/`:

    velvet art check
    velvet art build
    velvet art build --formats png --force --json

- `check` reports the missing sources, orphan files, aspect warnings, heavy files, skipped outputs and
  failures, and writes nothing. It exits with 1 when it finds any problem.
- `build` writes the outputs that are new or changed, and their stamps. It skips the outputs that are up
  to date (`--force` rebuilds them all). It exits with 1 when a source or an output failed.
- `--game <dir>` names the game folder, `--src <dir>` the source folder, `--formats avif,webp,png` the
  formats, and `--json` prints the report as JSON.

WebP and AVIF need `ffmpeg` on the PATH, with the `libwebp` and `libaom-av1` encoders. Without them those
formats are skipped and reported, and PNG is still written.

## Parts

- `ArtArgs`, `ArtCommand`, `run`: the command surface, for the `velvet` CLI to flatten into its `art`
  subcommand.
- `build`, `check`, `ArtBuildOptions`: the same work from Rust, returning an `ArtReport`.
- `parse_manifest`, `parse_config`, `Manifest`, `Entry`, `Kind`, `BuildConfig`, `IdOverride`, `Quality`:
  the documents, read and checked once.
- `Fit`, `OutputFormat`: how a source fills its box, and the formats that are written.
- `ArtReport` and its items (`OutputRecord`, `MissingSource`, `AspectWarning`, `HeavyOutput`,
  `SkippedOutput`, `Failure`): the report, with `to_text` and `to_json` in the module `report`.
- `ArtBuildError`, `Refusal`, `ErrorCode`, `Issue`: the errors. A refused document lists every problem.

## Tests

    cargo test -p velvet-artbuild --offline

The manifest fixtures of `docs/formats/fixtures/art/` are checked in `tests/fixtures.rs`. The other tests
generate their pictures in a temporary folder and remove it afterwards. The WebP and AVIF tests print a
note and pass when ffmpeg with `libwebp` and `libaom-av1` is not on the PATH.
