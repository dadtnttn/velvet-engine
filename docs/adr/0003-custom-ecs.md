# ADR 0003: Custom ECS (Initial)

## Status

Accepted (revisitable)

## Context

Options: depend on `bevy_ecs` / `hecs` / `specs`, or implement a focused 2D ECS.

## Decision

Implement `velvet-ecs` tailored to:

- Serializable components for scenes/saves.
- Tight Velvet Script interop.
- Simple single-threaded schedules first.

Do not invent exotic archetype optimizations until benchmarks demand them. Sparse-set or archetype hybrid is acceptable; start with archetype-inspired storage for cache-friendly iteration of common queries.

## Consequences

- Full control of API and save format.
- More engineering cost than vendoring hecs.
- Escape hatch: façade traits if we swap storage later.
