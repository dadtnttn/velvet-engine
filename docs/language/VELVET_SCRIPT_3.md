# Velvet Script 3

Advanced mathematics contract: [VS3_MATH_SPEC.md](VS3_MATH_SPEC.md).

Evolution plan: [VS3_ROADMAP.md](VS3_ROADMAP.md). Engine integrations must
follow [VS3_ENGINE_RULES.md](../architecture/VS3_ENGINE_RULES.md).

Velvet Script 3 (VS3) is the official general-purpose game-logic language in
Velvet Engine. A source file opts in with `// @edition 3`. Classic `.vel` story
files remain a separate, supported product path.

| Property | Current contract |
|---|---|
| Edition | `// @edition 3` |
| Extension | `.vel` |
| Status | Usable alpha; real compiler, VM, CLI, LSP, tests, and host ABI |
| Scope | General deterministic logic; product features arrive through host services |
| Runtime | Sandboxed by default, bounded by instruction/memory/stack limits |

VS3 does not encode a novel, card game, RPG, renderer, or network stack into
the language. Those systems are libraries and host services. This keeps game
rules portable and testable.

## Current language surface

### Declarations and state

```velvet
// @edition 3

state {
    runs: int = 0
}

function register_run(score: int) {
    runs += 1
    return {"runs": runs, "score": score}
}
```

- Functions: `function name(parameters) { ... }` or `fn`.
- Mutable bindings: `let name = value`.
- Immutable bindings: `const name = value`.
- Persistent module state: `state { name: type = value }`.
- Parameter, local, and state annotations: `any`, `null`, `bool`, `int`/`i64`,
  `float`/`f64`, `string`/`str`, `list`, and `map`.
- A `Vs3Session` initializes state once and preserves it across calls.
  `Vs3Module::call` is intentionally isolated and creates a fresh session.

Unknown names, duplicate definitions, invalid assignment targets, writes to a
`const`, wrong arity, unknown types, and statically visible type mismatches are
compile errors with source locations. Annotated function parameters are also
checked at the Rust/CLI call boundary.

### Values and operators

- Values: `null`, booleans, signed 64-bit integers, 64-bit floats, strings,
  mutable lists, and mutable string-keyed maps.
- List literals: `[1, 2, 3]`.
- Map literals: `{"name": "Ada", "score": 42}`; identifier keys are accepted
  and the formatter emits quoted keys.
- Indexing and mutation: `items[0]`, `profile["score"] = 42`. Map fields also
  support record-style sugar: `profile.score += 1`.
- Arithmetic: `+ - * / %` and unary `-`.
- Compound assignment: `+= -= *= /=`, including indexed values.
- Comparison: `== != < <= > >=`.
- Logic: `&& || !` with short-circuit execution.

Integer overflow, division overflow, and division by zero are runtime errors;
they never panic the Rust host. Maps use deterministic key order. Cyclic list
or map values are safe to compare, format, and account against memory limits.

### Control flow

```velvet
function sum_even(values: list) {
    let total = 0
    for value in values {
        if value % 2 != 0 { continue }
        total += value
        if total >= 100 { break }
    }
    return total
}
```

VS3 supports `if`/`else`, `while`, `for value in collection`, `break`,
`continue`, block scopes, and `return`.

Narrative declarations and commands such as `scene`, `character`, dialogue,
`choice`, and `background` are rejected in edition 3. Use the classic story
runtime or a host service. This prevents silent print/no-op lowering from being
mistaken for general-language semantics.

### Standard library

| Area | Functions |
|---|---|
| Scalar math | Trigonometry, logarithms, powers, interpolation, range conversion, finite checks, GCD/LCM and checked integer powers |
| Linear algebra | `vec2`/`vec3`/`vec4`, `mat3`/`mat4`, `quat`, operators, transforms, projection and geometry |
| Procedural | Seeded PCG streams, distributions, value/gradient noise, fBm, turbulence and domain warp |
| Statistics | Aggregates, quantiles, variance, covariance, correlation, histograms and smoothing |
| Numerical | Polynomial evaluation/root solving and sample-based trapezoid/Simpson integration |
| Values | `len`, `concat`, `str`, `type_of` |
| Collections | `list_push`, `list_pop`, `map_has`, `map_keys` |
| Errors | `assert(condition, message?)`, `fail(message)` |
| Data tools | `hash_sha256`, `hex_encode`, `base64_encode` |
| Debug output | `print` (captured by the VM, not written directly by the runtime) |

See the complete contracts, signatures, failure modes, and examples in
[VS3 Advanced Mathematics](VS3_MATH_SPEC.md).

The legacy presentation adapter (`present_show`, `present_hide`, `set_bg`,
`ui_flag`, `ui_flag_get`) requires `sandbox: false`. New integrations should
prefer the generic host ABI.

## Cooperative tasks and host ABI

`yield(value)` is an expression. It is valid only in a cooperative task created
with `Vs3Module::start`; a normal call returns a clear runtime error instead of
silently continuing.

The canonical host request is `[service, payload]`:

```velvet
// @edition 3

function load_profile(user_id: int) {
    let profile = yield(["storage.profile.load", user_id])
    return profile
}
```

A `Vs3Host` returns:

- `HostOutcome::Ready(value)` for an immediate response;
- `HostOutcome::Pending { ticket }` for later completion;
- `HostOutcome::Failed(message)` for a controlled error.

Use `drive_host_with_policy` and `Vs3HostPolicy` in production. Policies can
grant exact services (`storage.profile.load`) or namespaces (`storage.*`) and
bound the number of immediate requests per drive call. `Vs3HostPolicy::default`
denies all services. Service names are validated and limited to 128 ASCII
letters, digits, dots, underscores, or hyphens.

## CLI

```powershell
# Validate and list exports
velvet vs3 check samples/vs3-logic/game_rules.vel

# Validate every edition-3 module in a directory as a package
velvet vs3 check samples/vs3-logic

# Format in place; comments and the edition marker are preserved
velvet vs3 fmt samples/vs3-logic/game_rules.vel

# One isolated call
velvet vs3 run samples/vs3-logic/game_rules.vel `
  --call can_play_card --arg i:5 --arg i:3 --arg i:3

# Structured arguments: float, null, or JSON list/map
velvet vs3 run logic.vel --call score `
  --arg f:0.75 --arg null --arg 'j:{"difficulty":"hard"}'

# Observe and optionally answer consecutive yields
velvet vs3 run samples/vs3-logic/host_services.vel `
  --call load_profile --arg i:7 --cooperative --resume 'j:{"name":"Ada"}'
```

Argument prefixes are `i:`, `f:`, `b:`, `s:`, and `j:`. Bare integers,
floats, booleans, `null`, and strings are also accepted.

## Rust API

```rust
use velvet_script_vs3::{compile, int, Vs3HostPolicy};

let module = compile(source, Some("rules.vel"))?;

// Persistent game/tool instance.
let mut session = module.session()?;
let first = session.call("register_run", &[int(100)])?;
let second = session.call("register_run", &[int(200)])?;

// Cooperative task with an explicit host capability policy.
let mut task = module.start("load_profile", &[int(7)])?;
let policy = Vs3HostPolicy::deny_all().allow("storage.profile.*");
let status = task.drive_host_with_policy(&mut host, &policy)?;
```

Runtime errors retain their source location and script stack trace. `VmLimits`
is re-exported by the VS3 crate for explicit instruction, memory, recursion,
stack, and sandbox configuration.

### Multi-file source bundles and nominal modules

An unaliased import keeps the compatibility behavior and composes source
fragments into the current module's shared namespace:

```velvet
// @edition 3
import "state.vel"
import "combat/weapons.vel"

function start() { return new_player() }
```

An aliased import creates a nominal module:

```velvet
// @edition 3
import "combat.vel" as combat
import "inventory.vel" as inventory

function attack() {
    let weapon = inventory.active_weapon()
    return combat.resolve_hit(weapon)
}
```

Nominal modules have isolated names and persistent state. Functions are called
with `module.function(...)`; state and top-level bindings are private and must
be exposed through functions. Different modules may therefore define the same
function or state names without collision.

Public APIs can be declared explicitly:

```velvet
import "combat.vel" as combat

export function attack(target: map) {
    return combat.resolve_hit(target)
}

function normalize_target(target: map) {
    return target
}
```

As soon as a source module declares at least one `export function`, only its
exported functions are visible to other modules, Rust hosts, and the CLI.
Unexported functions remain callable inside their own module. For compatibility,
a module with no `export` declarations keeps the older all-functions-public
behavior, allowing existing projects to migrate one file at a time.

Imports may be nested, are loaded once, and cycles, missing sources, invalid
aliases, ambiguous ownership, private function calls, and attempts to access
private state are diagnosed before bytecode generation.

`compile_bundle` resolves embedded source graphs and `compile_path` resolves
files from disk. Filesystem resolution canonicalizes every path and rejects
imports that escape the root directory, including through symlinks. The CLI
commands `velvet vs3 check` and `velvet vs3 run` use the same resolver.

The public Rust/CLI names of functions imported directly by the root are
`module.function`. Internal bytecode symbols are deterministically mangled and
are not part of the source or host API. The older `Vs3Package` API remains
available for host-owned collections of separately compiled modules using
`module::function`; source modules use the dot syntax.

## Tooling

- `velvet-script-lsp` uses the VS3 semantic frontend for edition-3 files.
- Completions are edition-aware and do not suggest narrative syntax in VS3.
- Rename ignores comments and string contents.
- Semantic tokens include comments, VS3 control flow, types, literals, and
  identifiers.
- The VS Code extension under `editors/vscode-velvet` declares and locks its
  language-client dependency with pnpm.

## Boundaries and next compatible extensions

The current alpha provides shared source fragments, nominal module namespaces,
and explicit function exports, but deliberately does not yet claim package
manifests, imports by package identity, structs/enums, pattern matching, generics,
return-type syntax, or a borrow checker. Records and tagged variants are represented
with maps and dispatched with ordinary conditionals. Future features should extend
the same semantic frontend and bytecode versioning instead of creating a parallel
language pipeline.

Engine concepts remain external modules. New scene, ECS, render, audio, input,
asset, physics, or UI integrations must not add edition-3 keywords or global
natives. The roadmap introduces typed, registrable modules while preserving the
generic capability-limited host ABI.

Classic product path remains:

```text
.vel / .vstory -> StoryProgram -> VnSession
```

VS3 logic path is:

```text
.vel (@edition 3)
  -> shared lexer/parser AST
  -> VS3 semantic validation (names, scopes, const, arity, annotations)
  -> bytecode v2
  -> bounded VM / persistent session / cooperative task
  -> capability-limited host services
```

## Samples and related documents

- `samples/vs3-logic/game_rules.vel`
- `samples/vs3-logic/host_services.vel`
- `samples/vs3-logic/present_bridge.vel`
- `docs/adr/0008-velvet-script-3-official.md`
- `docs/language/VS3_ROADMAP.md`
- `docs/architecture/VS3_ENGINE_RULES.md`
- `docs/language/VELVET_SCRIPT.md`
- `docs/language/VELVET_SCRIPT_2.md`
