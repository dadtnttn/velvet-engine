# Velvet Engine test strategy

Velvet Engine treats tests as executable product contracts, not as line-count
or compilation evidence. Every default test must have a clear reason to fail
when externally meaningful behavior regresses.

## Test classes

### Unit tests

Unit tests protect one deterministic contract in one crate. Prefer exact
outputs, explicit invariants, error codes, source locations, state transitions,
and boundary behavior.

Good examples:

- a parser preserves the valid declaration after malformed input;
- a VM error contains the failing source location and function frame;
- a dash with a raw direction preserves the configured magnitude;
- a catalog is unique and exactly synchronized with its public API.

### Contract tests

Contract tests stabilize public tables, protocols, serialization formats,
diagnostic codes, host requests, and language capabilities. They should fail
when a public contract changes without an intentional test update.

### Integration tests

Integration tests cross crate boundaries and must execute the real path being
protected. They should call exported APIs and verify the observable result,
not merely check that some function or export exists.

Examples include:

- source -> parser -> compiler -> bytecode -> VM result;
- story -> choice -> variables -> save/load;
- VS3 yield -> host ticket -> resume value;
- input -> dialogue advance -> typewriter completion.

### Manual visual evidence

Tests that write PNGs or other inspection artifacts are useful for design
review, but are not correctness tests. They must be marked `#[ignore]` and run
explicitly:

```bash
cargo test -p velvet-stakes -- --ignored
cargo test -p velvet-novella -- --ignored
```

Default CI uses geometry, pixel-region, contrast, hit-testing, layout, or
snapshot invariants instead of creating evidence files.

## Required qualities

A test in the default suite must satisfy all of these:

1. **Defined failure meaning.** A failure identifies a broken behavior or
   public contract.
2. **Deterministic setup.** Randomness uses a fixed seed and asynchronous work
   has a bounded deterministic completion rule.
3. **Specific assertion.** Prefer exact values or narrow invariants over
   permissive alternatives.
4. **Public behavior first.** Avoid asserting private implementation details
   unless the test is explicitly a low-level unit test.
5. **No duplicated proof.** Similar inputs for one behavior should use a table
   or loop unless distinct setup is necessary.
6. **Actionable diagnostics.** Failure messages include the observed value
   when the assertion is not self-explanatory.
7. **Independent state.** Tests do not depend on execution order or shared
   mutable files.

## Forbidden patterns

The quality gate rejects high-confidence non-tests such as:

- `assert!(result.is_ok() || result.is_err())`;
- empty-or-non-empty tautologies;
- `assert_eq!(value, value)` and equivalent self-comparisons;
- assertions containing `|| true`;
- PNG/evidence dump tests that run by default.

Do not replace these with a different broad assertion. State the expected
success, error, value, transition, or invariant.

## Consolidation guidance

Consolidate cases when they exercise the same control flow and differ only in
input/output data:

```rust
for (input, expected) in cases {
    assert_eq!(transform(input), expected);
}
```

Keep separate tests when setup, failure interpretation, or the protected
contract is different. Test count is not a goal by itself; signal quality and
maintenance cost are.

## Local and CI gates

Run the same gates locally that CI runs:

```bash
python tools/check_test_quality.py
pnpm --dir editors/vscode-velvet install --frozen-lockfile
pnpm --dir editors/vscode-velvet run check
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
cargo doc --workspace --all-features --no-deps
```

`tools/check_test_quality.py` is deliberately conservative. It catches only
patterns that are clearly non-protective; code review remains responsible for
semantic quality beyond those mechanical checks.
