from pathlib import Path

for rel in [
    "crates/velvet-script-hir/src/lib.rs",
    "crates/velvet-script-types/src/lib.rs",
]:
    p = Path(rel)
    out = []
    for line in p.read_text(encoding="utf-8").splitlines(True):
        if "format!(" in line and "scene s" in line and "{}" in line:
            # literal braces in format strings must be doubled
            line = line.replace("{}", "{{}}")
        out.append(line)
    p.write_text("".join(out), encoding="utf-8", newline="\n")
    print("fixed", rel)
