#!/usr/bin/env python3
"""Candidatos a revisar por concordancia con el género del jugador.

Uso: gender_candidates.py <lang> [outfile]
Marca entradas cuyo original le habla o describe al jugador (2.ª persona) y
cuya traducción tiene formas con género fuera de {@g|m=..|f=..}.
"""
import json, os, re, sys
I = os.environ.get("VSF_I18N") or os.path.join(os.path.dirname(os.path.abspath(__file__)), "trabajo")
# El jugador hace o es algo: pretérito de tú, «estás/eres», o su terminación {a}.
PLAYER = re.compile(r"\w(aste|iste)\b|\b(estás|eres|estabas|serás|estarás|fuiste|quedaste|sentiste|volviste)\b|\{a\}", re.I)
PAT = {
    "ru": re.compile(r"\b\w{2,}(л|лся)\b|\b(готов|рад|должен|один|сам|уверен|счастлив|свободен|виноват|нужен|занят|знаком)\b", re.I),
    "pl": re.compile(r"\w(łeś|łaś|łbyś|łabyś)\b|\b(sam|sama|gotowy|gotowa|pewny|pewna|zmęczony|zmęczona)\b", re.I),
    "he": re.compile(r"(^|\s)(אתה|את)(\s|$)"),
    "hi": re.compile(r"(गए|गया|थे|रहे|सकते|हुए|चुके|लगे|पाए|गई|गयी|थीं|थी|रही|सकती|हुई|चुकी)(\s|।|$|,)"),
    "fr": re.compile(r"\b(tu es|t'es|tu étais|tu seras|tu as été|tu te sens|tu te sentais|tu restes|tu deviens)\s+\w+", re.I),
    "pt-BR": re.compile(r"\b(está|é|ficou|estava|fica|anda|parece|sente-se|se sente|se sentiu)\s+\w+[oa]s?\b", re.I),
    "it": re.compile(r"\b(sei|eri|sarai|resti|rimani|diventi|ti senti|ti sentivi)\s+\w+[oa]\b|\bsei\s+(stato|stata|andato|andata|tornato|tornata)\b", re.I),
}


def strip_sel(t):
    while True:
        u = re.sub(r"\{[^{}]*\|[^{}]*\}", " ", t)
        if u == t:
            return u
        t = u


def main():
    lang = sys.argv[1]
    keys = json.load(open(os.path.join(I, "keys.json")))
    live = set(json.load(open(os.path.join(I, "live.json"))))
    done = json.load(open(os.path.join(I, "done", lang + ".json")))
    batches = json.load(open(os.path.join(I, "batches", "index.json")))
    pos = {i: (k, n) for k, ids in enumerate(batches) for n, i in enumerate(ids, 1)}
    out = []
    for e in keys:
        i = e["id"]
        if i not in live or i not in done or i not in pos:
            continue
        if not PLAYER.search(e["key"]):
            continue
        t = done[i]
        if PAT[lang].search(strip_sel(t)):
            k, n = pos[i]
            out.append(f"{k}|{n}|{e['key']}|{t}".replace("\n", "⏎"))
    text = "\n".join(out) + "\n"
    if len(sys.argv) > 2:
        open(sys.argv[2], "w").write(text)
    print(lang, len(out))


if __name__ == "__main__":
    main()
