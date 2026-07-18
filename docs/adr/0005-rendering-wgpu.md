# ADR 0005: wgpu for 2D Rendering

## Status

Accepted

## Context

Need portable GPU access for sprites, post-processing, and editor views.

## Decision

Use `wgpu` + `winit` for desktop. Implement a 2D-focused renderer with batching, cameras, virtual resolution, and named quality profiles (Visual Novel, Pixel Art, Top-down RPG, Top-down Action, Cinematic 2D).

## Consequences

- Modern API learning curve.
- Excellent long-term portability (including future WASM).
- Requires careful testing on host GPU drivers.
