#!/usr/bin/env python3
"""Batch translation helper (translations written by hand, line by line).

  tx.py init <prepdir>          start a work folder from i18n_prep.py output and
                                adopt what the catalogs already translate
  tx.py adopt                   take translations from assets/i18n/*.json and
                                i18n_data.vel into the work folder
  tx.py salvage                 merge glossaries + partial outputs into done/<lang>.json
  tx.py plan [chars]            freeze the global batch partition (batches/index.json)
  tx.py todo <lang>             batches with entries missing in lang ("batch:count")
  tx.py show <k> [lang]         compact view of batch k (only entries missing in lang)
  tx.py put <lang> <k> <file>   ingest "n|translation" lines for batch k
  tx.py stats                   progress per language
  tx.py build                   write ../../assets/i18n/<lang>.json + ../../i18n_data.vel
  tx.py delta <keys.json> [chars]  append new keys as extra batches (old batches untouched);
                                   also records which keys the code still has (live.json)
                                   and takes structural.json/names.json from the same folder

The work folder is ./trabajo next to this script, or $VSF_I18N.
"""
import fcntl
import hashlib
import json
import os
import re
import shutil
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
GAME = os.path.dirname(os.path.dirname(HERE))
I = os.environ.get("VSF_I18N") or os.path.join(HERE, "trabajo")
LANGS = ["en", "pt-BR", "de", "fr", "it", "pl", "ru", "zh-Hans", "zh-Hant", "ja", "ko", "he", "hi", "th"]

sys.path.insert(0, HERE)
from i18n_check import check  # noqa: E402

KIND_SHORT = {"NAME": "N", "GENDER": "G", "LOLA": "LL", "ELELLA": "EE", "UNUNA": "UU",
              "NUMBER": "#", "MONEY": "$", "TEXT": "T"}
UI_CTX = re.compile(r"(button|btn|tag|tab|title|menu|label|cats|toggle|word|ctext|head|item#[0-9]|names|chip)", re.I)


def sid(key):
    return hashlib.sha1(key.encode("utf-8")).hexdigest()[:10]


def load(path, default):
    try:
        with open(path) as f:
            return json.load(f)
    except (OSError, ValueError):
        return default


def save(path, data):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    tmp = path + ".tmp"
    with open(tmp, "w") as f:
        json.dump(data, f, ensure_ascii=False, indent=0)
    os.replace(tmp, path)


def keys():
    return load(os.path.join(I, "keys.json"), [])


def by_id():
    return {e["id"]: e for e in keys()}


def entry_for_check(e):
    def kind(h):
        return "PLURAL" if "plural(" in h.get("hint", "") else h["kind"]
    return {"id": e["id"], "src": e["key"],
            "holes": [f"{{{j}}}={kind(h)}: x" for j, h in enumerate(e["holes"])]}


def valid(e, tgt, lang):
    return not check(entry_for_check(e), tgt, lang)


def done_path(lang):
    return os.path.join(I, "done", f"{lang}.json")


def cmd_salvage():
    ents = by_id()
    key_to_id = {e["key"]: e["id"] for e in ents.values()}
    for lang in LANGS:
        done = load(done_path(lang), {})
        bad = 0
        d = os.path.join(I, "out", lang)
        sources = []
        g = load(os.path.join(d, "_glossary.json"), {})
        sources.append({key_to_id[k]: v for k, v in g.items() if k in key_to_id})
        if os.path.isdir(d):
            for f in sorted(os.listdir(d)):
                if f.endswith(".json") and not f.startswith("_"):
                    data = load(os.path.join(d, f), {})
                    if isinstance(data, dict):
                        sources.append(data)
        for src in sources:
            for i, t in src.items():
                e = ents.get(i)
                if e is None or not isinstance(t, str):
                    continue
                if valid(e, t, lang):
                    done.setdefault(i, t)
                else:
                    bad += 1
        save(done_path(lang), done)
        print(f"{lang:8} {len(done):6} done  ({bad} invalid skipped)")


def cmd_plan(chars=12000):
    ks = keys()
    batches, cur, size = [], [], 0
    for e in ks:
        n = len(e["key"]) + 12 * len(e["holes"]) + 6
        if cur and size + n > chars:
            batches.append(cur)
            cur, size = [], 0
        cur.append(e["id"])
        size += n
    if cur:
        batches.append(cur)
    save(os.path.join(I, "batches", "index.json"), batches)
    print(f"{len(batches)} batches of ~{chars} chars")


def batches():
    return load(os.path.join(I, "batches", "index.json"), [])


def cmd_delta(path, chars=12000):
    """Append keys from a fresh i18n_prep keys.json as NEW batches (old ones untouched)."""
    ks = keys()
    have = {e["id"] for e in ks}
    current = load(path, [])
    # Templates the code no longer builds must leave the catalogs: an old
    # "Ingreso {0} · vida…" would still match the new line and win.
    save(os.path.join(I, "live.json"), sorted(e["id"] for e in current))
    for side in ("structural.json", "names.json"):
        src = os.path.join(os.path.dirname(os.path.abspath(path)), side)
        if os.path.exists(src):
            shutil.copyfile(src, os.path.join(I, side))
    new = [e for e in current if e["id"] not in have]
    if not new:
        print("no new keys")
        return
    b = batches()
    start = len(b)
    out, cur, size = [], [], 0
    for e in new:
        n = len(e["key"]) + 12 * len(e["holes"]) + 6
        if cur and size + n > chars:
            out.append(cur)
            cur, size = [], 0
        cur.append(e["id"])
        size += n
    if cur:
        out.append(cur)
    # Keys first, then the index: readers never see a batch id without its key.
    save(os.path.join(I, "keys.json"), ks + new)
    save(os.path.join(I, "batches", "index.json"), b + out)
    print(f"+{len(new)} keys in batches {start}..{start + len(out) - 1}")


def cmd_init(prep):
    """A fresh work folder: the keys of the code now, one batch partition, and
    whatever the catalogs of the repo already translate."""
    os.makedirs(I, exist_ok=True)
    for name in ("keys.json", "structural.json", "names.json"):
        shutil.copyfile(os.path.join(prep, name), os.path.join(I, name))
    ks = keys()
    save(os.path.join(I, "live.json"), sorted(e["id"] for e in ks))
    en = catalog("en")
    # Whether a hole holds a name was decided when the key was first
    # translated ({0!} keeps it as is); the hint alone can't tell a person's
    # `name` from a job's.
    for e in ks:
        t = en.get(e["key"], "")
        for j, h in enumerate(e["holes"]):
            if "{%d!}" % j in t:
                h["kind"] = "NAME"
            elif "{%d}" % j in t and h["kind"] == "NAME":
                h["kind"] = "TEXT"
    # Plain texts the code no longer has stay in the catalogs: old saves
    # still show them in the life story.
    have = {e["id"] for e in ks}
    names = set(load(os.path.join(I, "names.json"), []))
    for k in en:
        if not re.search(r"\{\d", k) and k not in names and sid(k) not in have:
            ks.append({"id": sid(k), "key": k, "holes": [], "ctx": "retired", "at": "", "file": ""})
            have.add(sid(k))
    save(os.path.join(I, "keys.json"), ks)
    cmd_plan()
    cmd_adopt()


def catalog(lang):
    return load(os.path.join(GAME, "assets", "i18n", f"{lang}.json"), {}).get("msgs", {})


def cmd_adopt():
    """The repo keeps only the built catalogs: a key a catalog translates is
    done, except the ones build filled in with English ("fallback")."""
    ents = keys()
    for lang in LANGS:
        cat = load(os.path.join(GAME, "assets", "i18n", f"{lang}.json"), {})
        msgs = cat.get("msgs", {})
        english = set(cat.get("fallback", []))
        done = load(done_path(lang), {})
        n = 0
        for e in ents:
            t = msgs.get(e["key"])
            if t is None or e["id"] in done or e["key"] in english:
                continue
            done[e["id"]] = re.sub(r"\{(\d+)!\}", r"{\1}", t)
            n += 1
        save(done_path(lang), done)
        print(f"{lang:8} +{n} from the catalog ({len(done)} done)")
    # The minigame words live only in i18n_data.vel.
    row = re.compile(r'^\s*"([\w-]+)": (\{.*\}),?$')
    try:
        with open(os.path.join(GAME, "i18n_data.vel"), encoding="utf-8") as f:
            for line in f:
                m = row.match(line)
                if m:
                    path = os.path.join(I, "out", m.group(1), "_minigames.json")
                    if not os.path.exists(path):
                        save(path, json.loads(m.group(2)))
    except OSError:
        pass


def hole_tag(e):
    if not e["holes"]:
        return ""
    parts = []
    for j, h in enumerate(e["holes"]):
        k = KIND_SHORT.get(h["kind"], "T")
        if k == "T":
            hint = h["hint"]
            if hint.startswith("{") and hint.endswith("}"):
                hint = hint[1:-1]
            k = f"T={hint[:28]}"
        parts.append(f"{j}:{k}")
    return " ‹" + " ".join(parts) + "›"


def cmd_show(k, lang=None):
    ents = by_id()
    done = load(done_path(lang), {}) if lang else {}
    ref = load(done_path("en"), {}) if lang and lang != "en" else {}
    ids = batches()[k]
    shown = 0
    for n, i in enumerate(ids, 1):
        if i in done:
            continue
        e = ents[i]
        src = e["key"].replace("\n", "⏎")
        ui = " ◆" if (UI_CTX.search(e["ctx"]) and len(e["key"]) <= 40) else ""
        en = ref.get(i)
        hint = f"  ⟪EN: {en.replace(chr(10), '⏎')}⟫" if en is not None else ""
        print(f"{n}|{src}{hole_tag(e)}{ui}{hint}")
        shown += 1
    print(f"# batch {k}: {shown} of {len(ids)} entries" + (f" missing in {lang}" if lang else ""))


def cmd_put(lang, k, path):
    os.makedirs(os.path.dirname(done_path(lang)), exist_ok=True)
    with open(done_path(lang) + ".lock", "w") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        _put(lang, k, path)


def _put(lang, k, path):
    ents = by_id()
    ids = batches()[k]
    done = load(done_path(lang), {})
    ok, probs = 0, []
    with open(path) as f:
        for line in f:
            line = line.rstrip("\n")
            if not line.strip() or line.startswith("#"):
                continue
            m = re.match(r"^\s*(\d+)\s*\|(.*)$", line)
            if not m:
                probs.append(f"unparsed line: {line[:60]}")
                continue
            n, t = int(m.group(1)), m.group(2).replace("⏎", "\n")
            if n < 1 or n > len(ids):
                probs.append(f"{n}: out of range")
                continue
            e = ents[ids[n - 1]]
            # "N|=" written twice ("12|N|=", "12|12|=") also means "keep it".
            t = re.sub(r"^(?:N|\d+)\|(?==$)", "", t)
            if t == "=":
                t = e["key"]
            if re.match(r"^(?:N|\d+)\|", t):
                probs.append(f"{n}: starts with a line number again | got={t[:60]!r}")
                continue
            # Marks copied from the show view are never part of a translation.
            if "◆" not in e["key"]:
                t = t.replace(" ◆", "").replace("◆", "")
            if "⟪" not in e["key"]:
                t = re.sub(r"\s*⟪[^⟫]*⟫", "", t)
            p = check(entry_for_check(e), t, lang)
            if p:
                probs.append(f"{n}: {'; '.join(p)} | src={e['key'][:60]!r} | got={t[:60]!r}")
                continue
            done[e["id"]] = t
            ok += 1
    save(done_path(lang), done)
    missing = [n for n, i in enumerate(ids, 1) if i not in done]
    print(f"{lang} batch {k}: +{ok} ok, {len(probs)} problems, {len(missing)} still missing" +
          (f" {missing[:30]}" if missing else ""))
    for p in probs:
        print("  " + p)


def cmd_todo(lang):
    done = load(done_path(lang), {})
    out = []
    for k, ids in enumerate(batches()):
        miss = sum(1 for i in ids if i not in done)
        if miss:
            out.append(f"{k}:{miss}")
    print(" ".join(out))


def cmd_stats():
    ks = keys()
    total = len(ks)
    tchars = sum(len(e["key"]) for e in ks)
    for lang in LANGS:
        done = load(done_path(lang), {})
        dchars = sum(len(e["key"]) for e in ks if e["id"] in done)
        print(f"{lang:8} {len(done):6}/{total}  {100 * dchars // max(1, tchars):3}% of text")
    b = batches()
    print(f"{len(b)} batches")


def name_holes(e):
    return {j for j, h in enumerate(e["holes"]) if h["kind"] == "NAME"}


def mark_raw(target, holes):
    def rep(m):
        n = int(m.group(1))
        return "{%d!}" % n if n in holes else m.group(0)
    return re.sub(r"\{(\d+)\}", rep, target)


# Plantillas estructurales con puntuación propia del español: cada idioma la
# escribe a su manera ("*" vale para los demás).
STRUCT_LANG = {
    "¡{0}!": {"zh-Hans": "{0}！", "zh-Hant": "{0}！", "ja": "{0}！", "*": "{0}!"},
    "{0}¿?": {"*": "{0}???"},
    "«{0}»": {"en": "“{0}”", "pt-BR": "“{0}”", "de": "„{0}“", "pl": "„{0}”", "ja": "「{0}」",
              "zh-Hant": "「{0}」", "zh-Hans": "“{0}”", "ko": "“{0}”", "hi": "“{0}”", "th": "“{0}”",
              "he": '"{0}"', "*": "«{0}»"},
}


def struct_target(key, lang):
    forms = STRUCT_LANG.get(key)
    if forms is None:
        return key
    return forms.get(lang, forms["*"])


AMOUNT = re.compile(r"\$\d{1,3}(?:,\d{3})*")


def money_templates(msgs):
    """The game shows local prices: «Te quitó $500 y el celular.» becomes
    «Te quitó $770…». Each text with amounts (kept verbatim by its
    translation) also gets a template with the amounts as holes."""
    out = {}
    for key, val in msgs.items():
        amounts = AMOUNT.findall(key)
        if not amounts or len(set(amounts)) != len(amounts):
            continue
        if not all(a in val for a in amounts):
            continue
        holes = [int(h) for h in re.findall(r"\{(\d+)", key)]
        nxt = max(holes) + 1 if holes else 0
        k, v = key, val
        for j, a in enumerate(amounts):
            k = k.replace(a, "{%d}" % (nxt + j), 1)
            v = v.replace(a, "{%d}" % (nxt + j))
        out[k] = v
    return out


def plural_pairs():
    """(singular, plural) words the game passes to plural(): «año»/«años»."""
    pairs = set()
    for name in os.listdir(GAME):
        if name.endswith(".vel") and name != "debug.vel":
            text = open(os.path.join(GAME, name), encoding="utf-8").read()
            pairs.update(re.findall(r'plural\([^,()]*, "([^"]+)", "([^"]+)"\)', text))
    return sorted(p for p in pairs if p[0] != p[0].upper())


def singular_templates(msgs, pairs):
    """«Hace {0} años» gets «Hace {0} año» too (for 1): the plural word of
    the translation becomes the singular one, or a selector is reused."""
    out = {}
    for sing, plur in pairs:
        t_sing, t_plur = msgs.get(sing), msgs.get(plur)
        for key, val in msgs.items():
            if "{0} " + plur not in key:
                continue
            k = key.replace("{0} " + plur, "{0} " + sing)
            if k in msgs:
                continue
            if "{0|#" in val:
                out[k] = val
            elif t_sing and t_plur and t_plur in val:
                out[k] = val.replace(t_plur, t_sing)
    return out


def cmd_build():
    ks = keys()
    ents = {e["id"]: e for e in ks}
    structural = load(os.path.join(I, "structural.json"), [])
    names = load(os.path.join(I, "names.json"), [])
    data_vel = ["// @edition 3",
                "// Generado por tx.py: datos de minijuegos por idioma (no editar a mano).",
                "", "const I18N_GAME = {"]
    rows = []
    # Lo que falte en un idioma se muestra en inglés (el predeterminado), no en español.
    en_done = load(done_path("en"), {})
    live = set(load(os.path.join(I, "live.json"), []))

    def stale(e):
        # Old plain texts stay (old saves still show them); old templates go.
        return bool(live) and bool(e["holes"]) and e["id"] not in live

    for lang in LANGS:
        done = load(done_path(lang), {})
        msgs = {}
        for i, t in done.items():
            e = ents.get(i)
            if e is None or stale(e):
                continue
            # "¿?" (unknown) is Spanish punctuation: other languages show "???".
            msgs[e["key"]] = mark_raw(t.replace("¿?", "???"), name_holes(e))
        english = []
        if lang != "en":
            for i, t in en_done.items():
                e = ents.get(i)
                if e is None or stale(e) or e["key"] in msgs:
                    continue
                msgs[e["key"]] = mark_raw(t.replace("¿?", "???"), name_holes(e))
                english.append(e["key"])
        fallback = len(english)
        for e in structural:
            msgs.setdefault(e["key"], mark_raw(struct_target(e["key"], lang), name_holes(e)))
        money = money_templates(msgs)
        for k, v in money.items():
            msgs.setdefault(k, v)
        for k, v in singular_templates(msgs, plural_pairs()).items():
            msgs.setdefault(k, v)
        lower = {k.lower() for k in msgs}
        for nm in names:
            if nm.lower() in lower and nm not in msgs:
                msgs[nm] = nm
        out = {"lang": lang, "msgs": msgs}
        if english:
            # Not translated yet: adopt must not take these as translations.
            out["fallback"] = english
        path = os.path.join(GAME, "assets", "i18n", f"{lang}.json")
        os.makedirs(os.path.dirname(path), exist_ok=True)
        with open(path, "w") as f:
            json.dump(out, f, ensure_ascii=False, separators=(",", ":"))
        mg = load(os.path.join(I, "out", lang, "_minigames.json"), {})
        if mg:
            rows.append(f'    "{lang}": ' + json.dumps({"title": mg.get("title", ""),
                                                        "spelling": mg.get("spelling", []),
                                                        "typing": mg.get("typing", [])},
                                                       ensure_ascii=False))
        print(f"{lang:8} {len(msgs)} msgs ({fallback} en inglés) -> {os.path.getsize(path) // 1024} KB")
    data_vel.append(",\n".join(rows))
    data_vel.append("}")
    with open(os.path.join(GAME, "i18n_data.vel"), "w") as f:
        f.write("\n".join(data_vel) + "\n")


def main():
    a = sys.argv[1:]
    if not a:
        print(__doc__)
        return
    c = a[0]
    if c == "init":
        cmd_init(a[1])
    elif c == "adopt":
        cmd_adopt()
    elif c == "salvage":
        cmd_salvage()
    elif c == "plan":
        cmd_plan(int(a[1]) if len(a) > 1 else 12000)
    elif c == "show":
        cmd_show(int(a[1]), a[2] if len(a) > 2 else None)
    elif c == "put":
        cmd_put(a[1], int(a[2]), a[3])
    elif c == "stats":
        cmd_stats()
    elif c == "todo":
        cmd_todo(a[1])
    elif c == "build":
        cmd_build()
    elif c == "delta":
        cmd_delta(a[1], int(a[2]) if len(a) > 2 else 12000)
    else:
        print(__doc__)


if __name__ == "__main__":
    main()
