# ADR 0002: Plugin Architecture

## Status

Accepted

## Context

Modules must compose without a god-object engine type that knows about visual novels and combat.

## Decision

Adopt a Bevy-inspired but independent `Plugin` trait on `App`:

- Named plugins with declared dependencies.
- Topological initialization with cycle detection.
- `build` / `finish` hooks.
- Systems registered into named schedules.

Core remains free of Story/Play knowledge.

## Consequences

- Uniform extension model for first-party and third-party modules.
- Requires careful ordering documentation.
- Slight runtime cost for registration (startup only).
