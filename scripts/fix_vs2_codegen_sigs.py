#!/usr/bin/env python3
from pathlib import Path
import re

p = Path("crates/velvet-script-compiler/src/vs2_codegen.rs")
t = p.read_text(encoding="utf-8")
t = t.replace(
    "fn lower_stmt(unit: &mut Vs2Unit, ctx: &mut LowerCtx, st: &HirStmt) {",
    "fn lower_stmt(unit: &mut Vs2Unit, ctx: &mut LowerCtx, st: &HirStmt, file: &str) {",
)
t = t.replace(
    "fn lower_expr(unit: &mut Vs2Unit, ctx: &mut LowerCtx, e: &HirExpr) {",
    "fn lower_expr(unit: &mut Vs2Unit, ctx: &mut LowerCtx, e: &HirExpr, file: &str) {",
)
t = re.sub(r"lower_expr\(unit, ctx, ([^)]+)\)", r"lower_expr(unit, ctx, \1, file)", t)
t = re.sub(r"lower_stmt\(unit, ctx, ([^)]+)\)", r"lower_stmt(unit, ctx, \1, file)", t)
t = t.replace(", file, file)", ", file)")

old = """        HirExpr::Field { base, .. } => {
            lower_expr(unit, ctx, base, file);
            // field access: leave base; host may refine later
        }"""
new = """        HirExpr::Field { base, field, span, .. } => {
            unit.push_diag(Vs2Diag::unsupported(
                file,
                *span,
                &format!(\"field access `.{field}`\"),
                \"field\",
            ));
            // Do not silently compile as base-only.
            lower_expr(unit, ctx, base, file);
            unit.emit(Vs2Instr::new(OpVs2::Pop));
            let z = unit.pool.intern(\"0\");
            unit.emit(Vs2Instr::with_a(OpVs2::LoadConst, z));
        }"""
if old not in t:
    raise SystemExit("field block not found:\n" + t[t.find("HirExpr::Field") : t.find("HirExpr::Field") + 250])
t = t.replace(old, new)
p.write_text(t, encoding="utf-8")
print("ok", t.count("\n"))
