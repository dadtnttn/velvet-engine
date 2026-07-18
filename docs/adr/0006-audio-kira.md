# ADR 0006: kira for Audio

## Status

Accepted

## Context

Need music, SFX, voice buses, fades, and optional spatial 2D.

## Decision

Use `kira` as the default backend behind `velvet-audio` abstractions (buses: Master, Music, Voice, Effects, Ambient, UI). Tests may use a null backend.

## Consequences

- Pure-Rust stack aligns with engine goals.
- Backend trait allows replacement if needed.
