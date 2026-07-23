# Velvet Script for VS Code

1. Install the locked extension dependencies: `pnpm install --frozen-lockfile`.
2. Build the LSP: `cargo build -p velvet-script-lsp --release`.
3. Put `target/release/velvet-script-lsp` on your `PATH`, or set
   `velvet.lsp.path` in settings.
4. Run `pnpm run check`, then open a `.vel` file.

Edition-3 files use the VS3 semantic frontend for name, scope, const, native
arity, mathematical dimensions, immutable components, and type diagnostics.
Completion automatically switches between classic story syntax and the full
VS3 surface, including vectors, matrices, quaternions, procedural noise,
statistics, and numerical methods. Native hover shows return kind, purity, and
base execution cost.

The server speaks standard LSP over stdio with `Content-Length` framing.
