#!/usr/bin/env python3
"""Validate a translated chunk: i18n_check.py chunk_in.json out.json lang

Prints problems (one per line) and exits 1, or prints OK.
"""
import json
import re
import sys

MUST_KEEP = ("NAME", "NUMBER", "MONEY", "TEXT")
PLACE = re.compile(r"\{([^{}]*)\}")


def holes_of(entry):
    out = []
    for h in entry.get("holes", []):
        m = re.match(r"\{(\d+)\}=(\w+)", h)
        if m:
            out.append((int(m.group(1)), m.group(2)))
    return out


def check(src_entry, tgt, lang):
    probs = []
    if not isinstance(tgt, str) or not tgt.strip():
        return ["empty translation"]
    holes = dict(holes_of(src_entry))
    used = {}
    selected = set()
    depth = 0
    for ch in tgt:
        if ch == "{":
            depth += 1
            if depth > 1:
                return ["nested braces"]
        elif ch == "}":
            depth -= 1
            if depth < 0:
                return ["unbalanced '}'"]
    if depth != 0:
        return ["unbalanced '{'"]
    for m in PLACE.finditer(tgt):
        inner = m.group(1)
        if inner.startswith("@"):
            name = inner[1:].split("|")[0]
            if name.lower() != "g":
                probs.append(f"unknown variable @{name} (only @g exists)")
            if "|" not in inner:
                probs.append("@g needs options like {@g|m=..|f=..}")
            continue
        head = inner.split("|")[0].rstrip("!")
        if not head.isdigit():
            probs.append(f"bad placeholder {{{inner}}}")
            continue
        n = int(head)
        if n not in holes:
            probs.append(f"placeholder {{{n}}} not in source")
            continue
        if "|" in inner:
            opts = inner.split("|")[1:]
            if any("=" not in o for o in opts):
                probs.append(f"select {{{inner}}} needs key=value options")
            selected.add(n)
        else:
            used[n] = used.get(n, 0) + 1
    for n, kind in holes.items():
        if kind in MUST_KEEP and used.get(n, 0) == 0 and not (kind == "TEXT" and n in selected):
            probs.append(f"missing {{{n}}} ({kind})")
    return probs


def main():
    src = json.load(open(sys.argv[1]))
    lang = sys.argv[3] if len(sys.argv) > 3 else ""
    try:
        out = json.load(open(sys.argv[2]))
    except Exception as e:  # noqa: BLE001
        print(f"invalid JSON: {e}")
        sys.exit(1)
    if not isinstance(out, dict):
        print("output must be a JSON object {id: translation}")
        sys.exit(1)
    bad = 0
    for e in src:
        if e["id"] not in out:
            print(f"{e['id']}: missing")
            bad += 1
            continue
        for p in check(e, out[e["id"]], lang):
            print(f"{e['id']}: {p}  | src={e['src'][:70]!r}")
            bad += 1
    extra = set(out) - {e["id"] for e in src}
    for x in sorted(extra)[:20]:
        print(f"{x}: unknown id (remove it)")
        bad += 1
    if bad:
        print(f"{bad} problem(s)")
        sys.exit(1)
    print(f"OK {len(src)} entries")


if __name__ == "__main__":
    main()
