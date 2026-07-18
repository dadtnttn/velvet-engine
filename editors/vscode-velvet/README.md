# Velvet Script for VS Code

1. Build the LSP: `cargo build -p velvet-script-lsp --release`
2. Put `target/release/velvet-script-lsp` on your `PATH`, or set `velvet.lsp.path` in settings.
3. Install deps: `npm install vscode-languageclient` in this folder (or use a published extension later).
4. Open a `.vel` file — diagnostics, completion, and symbols use the real analysis pipeline.

The server speaks standard **LSP over stdio** (`Content-Length` framing).
