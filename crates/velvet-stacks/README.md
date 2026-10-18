# velvet-stacks

Cards on a board, in the style of Stacklands: cards lie on a table and stack on each other, and a
stack whose cards match a recipe starts a timed job that uses up some cards and makes others.

A tool crate, not a game (see `docs/architecture/TOOLS.md`). It is pure logic: no rendering, no
files, no threads, no clock. The caller passes the time that went by. The only randomness is a
seeded xorshift generator kept in the board's state, so the same catalog, seed and actions give the
same events on every platform. The rules are meant to run in WebAssembly for the JavaScript runtime.

## Parts

- `Catalog`, `CardDef`, `Selector`, `StackRule`, `Recipe`, `Ingredient`, `Output`: the definitions,
  read from JSON and checked once when the catalog is built.
- `Board`: `spawn`, `move_stack`, `drop_on`, `split`, `remove`, `tick`, `progress`,
  `running_recipe`, `take_events`, and `save` / `load` (or `to_json` / `from_json`).
- `BoardEvent`: what happened (`job_started`, `job_finished`, `card_spawned`, and so on), for
  animation and logs.
- `BoardSave`: the board's state as serde data.
- `StacksError`: every refusal, with the names listed in the format.

## Documents

The format, the matching rules and the save layout are in `docs/formats/STACKS.md`. The shared
conformance fixtures are in `docs/formats/fixtures/stacks/` (`catalog.json` and `cases/*.json`); the
test `tests/fixtures.rs` plays every case.

## Tests

    cargo test -p velvet-stacks
    cargo clippy -p velvet-stacks --all-targets -- -D warnings
