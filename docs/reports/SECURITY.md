# Security Notes

Velvet Engine is a **game toolkit**, not a hardened multi-tenant sandbox product.
Still, several design choices matter for safety.

## Script VM

- The VM applies instruction/step **limits** (`VmLimits`) to reduce infinite loops.
- It is **not** a security boundary against hostile scripts with host-injected native functions.
- Do not run untrusted `.vel` with privileged host callbacks.

## Filesystem & tooling

- CLI pack/export walk user-supplied directories; treat paths as trusted project content.
- Export may invoke `cargo build` — same trust model as running Cargo yourself.
- Studio and CLI read/write project files without sandboxing.

## Deserialization

- `velvet.project` uses RON via `serde`. Only open projects you trust.
- Save games and asset manifests are similarly trusted-data formats.

## Dependencies

- Review `deny.toml` / `cargo deny` in CI when enabled.
- GPU/audio native stacks (wgpu, etc.) inherit upstream CVEs — keep toolchains updated.

## Reporting

- Prefer private disclosure for vulnerabilities that enable remote code execution via crafted assets.
- There is no paid bug bounty program as of this writing.

## Explicit non-goals

- Secure multiplayer authority
- Browser-grade XSS model for UI markup
- Cryptographic verification of asset packs (checksums are integrity helpers, not signatures)
