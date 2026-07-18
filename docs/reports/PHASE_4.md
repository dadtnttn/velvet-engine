```text
FASE COMPLETADA
Objetivo: Velvet Script básico — lexer, parser, AST, bytecode, compilador, VM
Cambios:
  - velvet-script-ast: Expr/Stmt/Item/Module + SourceLoc + Diagnostic
  - velvet-script-parser: recursive descent, recovery, dialogue/scene/choice/fn
  - velvet-script-bytecode: Op, Chunk, BytecodeModule, source map
  - velvet-script-compiler: AST → bytecode (functions, state, scenes, control flow)
  - velvet-script-vm: stack VM, limits (instructions/memory/recursion), stack traces
  - velvet-cli: `velvet script check` / `velvet script run`
  - examples/hello-script.vel
Crates afectados: script-*, velvet-cli
Pruebas:
  - parser: 5 tests
  - compiler: 2 tests
  - vm: 7 tests (arithmetic, while, limits, div0 location, narrative print, globals)
Comandos:
  cargo test -p velvet-script-parser -p velvet-script-compiler -p velvet-script-vm
  cargo run -p velvet-cli -- script check examples/hello-script.vel
  cargo run -p velvet-cli -- script run examples/hello-script.vel --call intro
Resultados: tests OK; scripts compile and execute
Estado:
  - compile + execute: implementado
  - file:line:column errors: implementado (lexer + diagnostics + VM maps)
  - types/HIR/LSP/format: planificado / scaffold
  - full object model / field access: no en v1
Problemas conocidos:
  - choice always takes first arm (no runtime UI yet)
  - field/index expressions rejected at compile
Siguiente fase: Velvet Story (Phase 5)
```
