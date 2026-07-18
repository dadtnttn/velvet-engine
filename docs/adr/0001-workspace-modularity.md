# ADR 0001: Cargo Workspace Modularity

## Status

Accepted

## Context

Velvet Engine spans core runtime, rendering, scripting, narrative, gameplay, and tooling. A single crate would couple unrelated concerns and slow compile times.

## Decision

Use a Cargo workspace under `crates/` with fine-grained crates. Gameplay modules (Story, Play, RPG, Action) depend on core/media crates but not on each other unless explicitly needed (RPG/Action may depend on Play).

## Consequences

- Longer initial setup; clearer dependency boundaries.
- Games enable only needed features via Cargo features and plugins.
- CI builds the full workspace; publish story can remain multi-crate later.
