#!/usr/bin/env python3
"""Turn `velvet vs3 strings` output into translation chunks.

usage: i18n_prep.py strings.json outdir [--only-new existing_ids.json]
"""
import hashlib
import json
import os
import re
import sys

NAME_CTX = {"nm", "nf", "sn", "odd_m", "odd_f", "NAMES_M", "NAMES_F", "SURNAMES", "PET_NAMES"}
ID_LIST_CTX = {"o"}
DROP_CTX = {"SPELLING", "WORDS", "INVITE_ACTS", "lock_is_requirement", "pressed#1"}
DROP_FILES = {"debug.vel", "i18n_data.vel"}
PIPE_CTX = {"cl", "CAREER_ROWS", "cus"}

PATH_RE = re.compile(r"^[MLHVCSQTAZmlhvcsqtaz0-9 .,\-]+$")
ID_TEMPLATE_RE = re.compile(r"^[a-z0-9_:./\-{}]+$")

MARKERS = {
    "{a}": ("GENDER", "player's gender ending: 'o' (male) or 'a' (female)"),
    "{xa}": ("GENDER", "other person's gender ending: 'o' or 'a'"),
    "{pa}": ("GENDER", "partner's gender ending: 'o' or 'a'"),
    "{pa2}": ("GENDER", "gender ending: 'o' or 'a'"),
    "{ka}": ("GENDER", "child's gender ending: 'o' or 'a'"),
    "{o}": ("GENDER", "gender ending: 'o' or 'a'"),
    "{ra}": ("GENDER", "gender ending: '' (male) or 'a' (female)"),
    "{xlo}": ("LOLA", "object pronoun for the other person: 'lo' (him) or 'la' (her)"),
    "{plo}": ("LOLA", "object pronoun for the partner: 'lo' (him) or 'la' (her)"),
    "{xel}": ("ELELLA", "'él' (he) or 'ella' (she) for the other person"),
    "{pel}": ("ELELLA", "'él' (he) or 'ella' (she) for the partner"),
    "{xun}": ("UNUNA", "'Un' or 'Una' (a man / a woman) for the other person"),
    "{x}": ("NAME", "first name of the other person"),
    "{p}": ("NAME", "partner's first name"),
    "{nombre}": ("NAME", "player's first name"),
    "{pname}": ("NAME", "a person's name"),
    "{edad}": ("NUMBER", "age in years"),
    "{monto}": ("MONEY", "money amount like $1,200"),
    "{costo}": ("MONEY", "money amount like $1,200"),
    "{meta}": ("MONEY", "money amount like $1,200"),
    "{trabajo}": ("TEXT", "player's job title (already translated)"),
    "{hij}": ("TEXT", "'hijo' or 'hija' (son/daughter), already translated"),
    "{padre}": ("TEXT", "a title like 'El Emperador', already translated"),
    "{pa_word}": ("TEXT", "'papá' or 'mamá', already translated"),
}

NAME_HINT = re.compile(r"(\.first|\.last|\.name|full_name\(|^name$|^nombre$|^first$|\.n_first|pet_name|\.pet)\b")
GENDER_HINT = re.compile(r"(^|\()oa\(|str_upper\(oa\(")
NUMBER_HINT = re.compile(r"^str\(|^len\(|^int\(|\.age\b|_pct|percent")
MONEY_HINT = re.compile(r"money_str\(|money\(|\$")


def hole_kind(hint):
    if hint in MARKERS:
        return MARKERS[hint]
    if hint.startswith("{") and hint.endswith("}"):
        return ("TEXT", f"value of marker {hint}")
    if GENDER_HINT.search(hint):
        return ("GENDER", "gender ending 'o'/'a' (uppercase 'O'/'A' if inside caps)")
    if MONEY_HINT.search(hint):
        return ("MONEY", "money amount like $1,200")
    if NAME_HINT.search(hint):
        return ("NAME", "a person's name")
    if NUMBER_HINT.search(hint):
        return ("NUMBER", "a number")
    return ("TEXT", "runtime text (translated separately; may be a word, phrase or sentence)")


def sid(key):
    return hashlib.sha1(key.encode("utf-8")).hexdigest()[:10]


def literal_parts(key):
    return re.sub(r"\{\d+\}", "", key)


def keep(e):
    key = e["key"]
    file = e["places"][0].split(":")[0]
    if file in DROP_FILES or e["context"] in DROP_CTX or e["context"] in NAME_CTX:
        return False
    if "<" in key and ">" in key:
        return False
    lits = literal_parts(key)
    if PATH_RE.match(key) and len(key) > 20:
        return False
    if e["holes"] and ID_TEMPLATE_RE.match(key) and " " not in key:
        return False
    # Ids de opciones en listas de datos ("o": ["play", "cram"]).
    if e["context"] in ID_LIST_CTX and re.match(r"^[a-z_]+$", key):
        return False
    if not any(c.isalpha() for c in lits) and not e["holes"]:
        return False
    if "font-" in key or "fill=" in key or "stroke" in key:
        return False
    return True


def main():
    src, outdir = sys.argv[1], sys.argv[2]
    only_new = None
    if "--only-new" in sys.argv:
        only_new = set(json.load(open(sys.argv[sys.argv.index("--only-new") + 1])))
    data = json.load(open(src))
    names = set()
    for e in data:
        if e["context"] in NAME_CTX and not e["holes"]:
            for n in re.split(r"[,|]", e["key"]):
                n = n.strip()
                if n:
                    names.add(n)
    entries = {}
    structural = {}
    for e in data:
        if not keep(e):
            continue
        keys = [e["key"]]
        if not e["holes"] and "|" in e["key"] and (e["context"] in PIPE_CTX or e["key"].count("|") >= 1):
            keys = [k.strip() for k in e["key"].split("|") if k.strip()]
        for key in keys:
            if not e["holes"] and not any(c.isalpha() for c in key):
                continue
            holes = [{"hint": h, "kind": hole_kind(h)[0], "note": hole_kind(h)[1]} for h in e["holes"]]
            item = {
                "id": sid(key),
                "key": key,
                "holes": holes,
                "ctx": e["context"],
                "at": e["places"][0],
                "file": e["places"][0].split(":")[0],
            }
            if e["holes"] and not any(c.isalpha() for c in literal_parts(key)):
                structural[item["id"]] = item
                continue
            entries.setdefault(item["id"], item)
    os.makedirs(outdir, exist_ok=True)
    json.dump(sorted(names), open(os.path.join(outdir, "names.json"), "w"), ensure_ascii=False)
    json.dump(list(structural.values()), open(os.path.join(outdir, "structural.json"), "w"), ensure_ascii=False, indent=0)
    allk = list(entries.values())
    json.dump(allk, open(os.path.join(outdir, "keys.json"), "w"), ensure_ascii=False, indent=0)
    todo = [e for e in allk if only_new is None or e["id"] not in only_new]
    # Chunks grouped by file, ~CHUNK_CHARS of source text each.
    CHUNK_CHARS = int(os.environ.get("CHUNK_CHARS", "16000"))
    by_file = {}
    for e in todo:
        by_file.setdefault(e["file"], []).append(e)
    chunks, cur, size = [], [], 0
    for f in sorted(by_file, key=lambda f: f):
        for e in by_file[f]:
            n = len(e["key"]) + 40 * len(e["holes"]) + 30
            if cur and size + n > CHUNK_CHARS:
                chunks.append(cur)
                cur, size = [], 0
            cur.append(e)
            size += n
    if cur:
        chunks.append(cur)
    cdir = os.path.join(outdir, "chunks")
    os.makedirs(cdir, exist_ok=True)
    for f in os.listdir(cdir):
        os.remove(os.path.join(cdir, f))
    for i, c in enumerate(chunks):
        slim = [
            {k: v for k, v in (("id", e["id"]), ("src", e["key"]), ("holes", [f"{{{j}}}={h['kind']}: {h['note']} [{h['hint']}]" for j, h in enumerate(e["holes"])]), ("ctx", e["ctx"] + " @ " + e["at"])) if v != []}
            for e in c
        ]
        json.dump(slim, open(os.path.join(cdir, f"{i:03d}.json"), "w"), ensure_ascii=False, indent=1)
    total = sum(len(e["key"]) for e in todo)
    print(f"{len(allk)} entries ({len(todo)} to translate, {total} chars) in {len(chunks)} chunks; "
          f"{len(structural)} structural; {len(names)} names")


if __name__ == "__main__":
    main()
