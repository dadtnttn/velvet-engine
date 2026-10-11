import sys, os, numpy as np
sys.path.insert(0, os.path.dirname(__file__))
from synth_lib import *

OUT = sys.argv[1]
ONLY = sys.argv[2:] or None
os.makedirs(OUT, exist_ok=True)

def semis(note, k):
    return hz(note) * 2 ** (k / 12)

def chord(tr, bus, beat, notes, dur, vel, inst=epiano, gain=1.0, strum=0.0):
    for i, n in enumerate(notes):
        tr.add(bus, beat + i * strum, inst(n, tr.beats_to_s(dur), vel), gain)

def mixdown(tr, gains, sends, wet=0.3, verb=None, hi=None, lo=None, peak=0.7, drive=1.1):
    n = tr.n
    dry = np.zeros(n); send = np.zeros(n)
    for bus, sig in tr.buses.items():
        g = gains.get(bus, 1.0)
        dry += sig * g
        send += sig * g * sends.get(bus, 0.0)
    v = verb or {}
    wetsig = reverb_loop(send, wet=1.0, **v) - send * 0.5
    x = dry + wetsig * wet
    if hi or lo: x = fft_filter(x, lo=lo, hi=hi, slope=0.5)
    return master(x, peak=peak, drive=drive)

def save(name, x):
    d = write_wav(os.path.join(OUT, name + ".wav"), x)
    print(f"{name:10s} {d:5.2f}s")

def want(name): return ONLY is None or name in ONLY

# =========================================================== PORTADA (Persona)
if want("bgm_title"):
    tr = Track(104, 8, swing=0.12)
    prog = [
        ("D2", [["F3", "A3", "C4", "E4"]], None),
        ("G2", [["F3", "B3", "E4", "A4"]], None),
        ("C2", [["E3", "G3", "B3", "D4"]], None),
        ("A1", [["G3", "C#4", "F4", "B4"]], None),
        ("D2", [["F3", "A3", "C4", "E4"]], None),
        ("G2", [["F3", "B3", "E4", "A4"]], None),
        ("Bb1", [["A3", "D4", "F4", "C5"]], None),
        ("A1", [["G3", "D4", "E4", "A4"], ["G3", "C#4", "F4", "Bb4"]], None),
    ]
    roots = [p[0] for p in prog]
    for b, (root, vs, _) in enumerate(prog):
        o = b * 4
        # piano eléctrico: golpes sincopados
        if len(vs) == 1:
            for beat, dur, vel in [(0, 1.2, 0.75), (1.5, 0.45, 0.55), (2.75, 1.0, 0.65)]:
                chord(tr, "keys", o + beat, vs[0], dur, vel)
        else:
            for beat, dur, vel, v in [(0, 1.2, 0.75, vs[0]), (1.5, 0.45, 0.55, vs[0]), (2.0, 0.8, 0.7, vs[1]), (2.75, 1.0, 0.65, vs[1])]:
                chord(tr, "keys", o + beat, v, dur, vel)
        # bajo funky
        nxt = roots[(b + 1) % len(roots)]
        approach = semis(nxt, -1) if b % 2 == 0 else semis(nxt, 1)
        f5 = semis(root, 7); octv = semis(root, 12); b7 = semis(root, 10)
        for beat, f, dur, vel in [(0, root, 0.45, 0.9), (0.75, root, 0.2, 0.45), (1.5, octv, 0.25, 0.7), (2.0, f5, 0.4, 0.8),
                                  (2.75, root, 0.2, 0.55), (3.0, b7, 0.4, 0.7), (3.5, approach, 0.4, 0.6)]:
            tr.add("bass", o + beat, bass(f, tr.beats_to_s(dur), vel, 1.0))
        # batería
        for beat, vel in [(0, 1.0), (1.5, 0.6), (2.5, 0.85)] + ([(3.75, 0.55)] if b % 4 == 3 else []):
            tr.add("drums", o + beat, kick(vel))
        for beat, vel in [(1, 0.9), (3, 0.9), (1.75, 0.22), (2.25, 0.18), (3.75, 0.25)]:
            tr.add("drums", o + beat, snare(vel))
        if b >= 4:
            for beat in (1, 3): tr.add("drums", o + beat, clap(0.5))
        for s in range(16):
            beat = s / 4
            if beat == 3.5:
                tr.add("hats", o + beat, hat(0.45, True))
            else:
                tr.add("hats", o + beat, hat(0.5 if s % 2 == 0 else 0.22))
    # melodía de vibráfono en la segunda mitad
    mel = [
        (4, [("D5", 0, .5), ("F5", .5, .5), ("A5", 1, 1.25), ("G5", 2.5, .5), ("F5", 3, .5), ("E5", 3.5, .5)]),
        (5, [("D5", 0, 1.5), ("E5", 1.5, .5), ("F5", 2, .5), ("E5", 2.5, .5), ("D5", 3, 1)]),
        (6, [("F5", 0, .5), ("A5", .5, .5), ("C6", 1, 1), ("A5", 2, .5), ("F5", 2.5, 1.5)]),
        (7, [("E5", 0, 1), ("C#5", 1, .5), ("E5", 1.5, .5), ("G5", 2, 1), ("A5", 3, 1)]),
    ]
    for bar, notes in mel:
        for n, beat, dur in notes:
            tr.add("lead", bar * 4 + beat, vibes(n, tr.beats_to_s(dur), 0.75))
    for bar, n in [(4, ["D4", "A4"]), (5, ["D4", "B4"]), (6, ["D4", "F4"]), (7, ["C#4", "E4"])]:
        for m in n: tr.add("pad", bar * 4, pad(m, tr.beats_to_s(4), 0.35, 0.6, 0.6, 1500))
    x = mixdown(tr, {"keys": 0.42, "bass": 0.62, "drums": 0.55, "hats": 0.32, "lead": 0.45, "pad": 0.25},
                {"keys": 0.35, "lead": 0.5, "pad": 0.6, "drums": 0.08}, wet=0.35, verb=dict(dur=1.6, decay=0.45), hi=10000)
    save("bgm_title", x)

# =========================================================== VIDA (neo-soul)
if want("bgm_life"):
    tr = Track(86, 8, swing=0.45)
    prog = [
        [("F2", ["E3", "A3", "C4", "G4"], 0, 4)],
        [("E2", ["G3", "B3", "D4", "F#4"], 0, 2), ("A1", ["G3", "C#4", "F#4", "B4"], 2, 2)],
        [("D2", ["F3", "A3", "C4", "E4"], 0, 4)],
        [("C2", ["Eb3", "G3", "Bb3", "D4"], 0, 2), ("F2", ["Eb3", "A3", "D4", "G4"], 2, 2)],
        [("Bb1", ["A3", "C4", "D4", "F4"], 0, 4)],
        [("Bb1", ["Db4", "F4", "G4", "C5"], 0, 4)],
        [("A1", ["G3", "C4", "D4", "E4"], 0, 4)],
        [("G1", ["F3", "Bb3", "D4", "A4"], 0, 2), ("C2", ["Bb3", "E4", "A4", "D5"], 2, 2)],
    ]
    for b, bar in enumerate(prog):
        o = b * 4
        for root, v, start, length in bar:
            if length == 4:
                hits = [(0, 1.6, 0.6), (1.75, 0.6, 0.45), (3.0, 0.9, 0.5)]
                bl = [(0, root, 1.2, 0.8), (1.5, semis(root, 7), 0.4, 0.6), (2.5, root, 0.4, 0.7), (3.5, semis(root, 12), 0.4, 0.55)]
            else:
                hits = [(0, 1.0, 0.6), (1.25, 0.6, 0.45)]
                bl = [(0, root, 0.9, 0.8), (1.5, semis(root, 7), 0.4, 0.6)]
            for beat, dur, vel in hits:
                chord(tr, "keys", o + start + beat, v, dur, vel, strum=0.02)
            for beat, f, dur, vel in bl:
                tr.add("bass", o + start + beat, bass(f, tr.beats_to_s(dur), vel, 0.45))
        for beat, vel in [(0, 0.9), (2.25, 0.55), (2.5, 0.8)]:
            tr.add("drums", o + beat, kick(vel))
        for beat in (1, 3):
            tr.add("drums", o + beat, snare(0.6)); tr.add("drums", o + beat, rim(0.35))
        for s in range(8):
            tr.add("hats", o + s / 2, hat(0.4 if s % 2 == 0 else 0.25))
        for s in range(16):
            tr.add("hats", o + s / 4, shaker(0.18 if s % 2 else 0.1))
    mel = [
        (4, [("D5", 0, 1.5), ("C5", 1.5, .5), ("A4", 2, 2)]),
        (5, [("Db5", 0, 1.5), ("C5", 1.5, .5), ("Bb4", 2, 1), ("F4", 3, 1)]),
        (6, [("E5", 0, 2), ("D5", 2, .5), ("C5", 2.5, 1.5)]),
        (7, [("D5", 0, 1), ("Bb4", 1, 1), ("A4", 2, 1), ("G4", 3, 1)]),
    ]
    for bar, notes in mel:
        for n, beat, dur in notes:
            tr.add("lead", bar * 4 + beat, lead(n, tr.beats_to_s(dur) * 0.95, 0.6))
    # crujido de vinilo muy suave
    r = np.random.default_rng(3)
    crackle = np.zeros(tr.n)
    pos = r.integers(0, tr.n, 900)
    crackle[pos] = r.uniform(-1, 1, 900) * r.uniform(0.2, 1, 900)
    crackle = fft_filter(crackle, lo=1500, hi=7000)
    tr.buses["vinyl"] = crackle
    x = mixdown(tr, {"keys": 0.5, "bass": 0.6, "drums": 0.42, "hats": 0.3, "lead": 0.38, "vinyl": 0.6},
                {"keys": 0.3, "lead": 0.45}, wet=0.3, verb=dict(dur=1.4, decay=0.4, bright=5000), hi=7000, drive=1.2)
    save("bgm_life", x)

# =========================================================== INFANCIA
if want("bgm_child"):
    tr = Track(112, 8, swing=0.0)
    prog = [("C3", ["E4", "G4", "C5"]), ("A2", ["E4", "A4", "C5"]), ("F2", ["F4", "A4", "C5"]), ("G2", ["D4", "G4", "B4"]),
            ("C3", ["E4", "G4", "C5"]), ("A2", ["E4", "A4", "C5"]), ("D3", ["D4", "F4", "A4"]), ("C3", ["E4", "G4", "C5"])]
    for b, (root, tri) in enumerate(prog):
        o = b * 4
        if b == 6:  # Dm7 -> G7
            tr.add("bass", o, pizz(root, 0.3, 0.9)); tr.add("bass", o + 2, pizz("G2", 0.3, 0.9))
            chord(tr, "keys", o + 1, ["D4", "F4", "C5"], 0.4, 0.55, marimba)
            chord(tr, "keys", o + 3, ["D4", "F4", "B4"], 0.4, 0.55, marimba)
        else:
            tr.add("bass", o, pizz(root, 0.3, 0.9)); tr.add("bass", o + 2, pizz(semis(root, 7), 0.3, 0.75))
            for beat in (1, 3): chord(tr, "keys", o + beat, tri, 0.4, 0.5, marimba)
        for s in range(8): tr.add("perc", o + s / 2, shaker(0.35 if s % 2 else 0.2))
        if b >= 4:
            for beat in (1, 3): tr.add("perc", o + beat, clap(0.35))
        tr.add("perc", o, kick(0.45))
    mel = [
        [("E5", 0, .5), ("G5", .5, .5), ("C6", 1, .5), ("G5", 1.5, .5), ("A5", 2, 1), ("G5", 3, 1)],
        [("E5", 0, .5), ("C5", .5, .5), ("E5", 1, .5), ("A5", 1.5, .5), ("G5", 2, 1.5), ("E5", 3.5, .5)],
        [("F5", 0, .5), ("A5", .5, .5), ("C6", 1, 1), ("A5", 2, .5), ("F5", 2.5, .5), ("A5", 3, 1)],
        [("G5", 0, 1.5), ("F5", 1.5, .5), ("E5", 2, .5), ("D5", 2.5, .5), ("B4", 3, 1)],
        [("C5", 0, .5), ("E5", .5, .5), ("G5", 1, .5), ("C6", 1.5, .5), ("E6", 2, 1), ("D6", 3, 1)],
        [("C6", 0, .5), ("A5", .5, .5), ("E5", 1, 1), ("A5", 2, .5), ("B5", 2.5, .5), ("C6", 3, 1)],
        [("D6", 0, .5), ("C6", .5, .5), ("A5", 1, 1), ("G5", 2, .5), ("F5", 2.5, .5), ("D5", 3, 1)],
        [("E5", 0, 1), ("G5", 1, 1), ("C6", 2, 2)],
    ]
    for bar, notes in enumerate(mel):
        for n, beat, dur in notes:
            tr.add("lead", bar * 4 + beat, glock(n, tr.beats_to_s(dur), 0.7))
    x = mixdown(tr, {"keys": 0.38, "bass": 0.6, "perc": 0.35, "lead": 0.5},
                {"keys": 0.25, "lead": 0.4}, wet=0.3, verb=dict(dur=1.2, decay=0.35), hi=10000)
    save("bgm_child", x)

# =========================================================== NOCHE / PRISIÓN
if want("bgm_night"):
    tr = Track(72, 4)
    prog = [("A1", ["A3", "E4", "B4", "C5"]), ("F1", ["A3", "E4", "B4", "C5"]), ("A1", ["A3", "E4", "B4", "C5"]), ("E1", ["E3", "A3", "D4", "B4"])]
    for b, (root, arp) in enumerate(prog):
        o = b * 4
        tr.add("bass", o, bass(root, tr.beats_to_s(3.8), 0.8, 0.35))
        for i, (n, beat) in enumerate(zip(arp, [0, 1, 2, 2.5])):
            tr.add("keys", o + beat, epiano(n, tr.beats_to_s(1.2), 0.5, 0.6))
        if b == 3:
            tr.add("keys", o + 3, epiano("G#4", tr.beats_to_s(1.0), 0.45, 0.6))
            tr.add("keys", o + 3.5, epiano("F4", tr.beats_to_s(0.5), 0.4, 0.6))
        for beat in range(4):
            tr.add("perc", o + beat, rim(0.16 if beat % 2 == 0 else 0.1))
        tr.add("perc", o, kick(0.5)); tr.add("perc", o + 2.5, kick(0.35))
    for b in range(4):
        for n in (["A2", "E3"] if b != 3 else ["E2", "B2"]):
            tr.add("pad", b * 4, pad(n, tr.beats_to_s(4.2), 0.4, 1.2, 1.5, 900))
    x = mixdown(tr, {"keys": 0.5, "bass": 0.7, "perc": 0.35, "pad": 0.4},
                {"keys": 0.6, "pad": 0.5, "perc": 0.2}, wet=0.45, verb=dict(dur=2.4, decay=0.8, bright=4000), hi=6000)
    save("bgm_night", x)

# =========================================================== DESPEDIDA
if want("bgm_death"):
    tr = Track(64, 4)
    arps = [
        ("D2", ["D3", "A3", "E4", "F4", "A4", "F4", "E4", "A3"]),
        ("Bb1", ["Bb2", "F3", "A3", "D4", "F4", "D4", "A3", "F3"]),
        ("A1", ["A2", "F3", "C4", "E4", "A4", "E4", "C4", "F3"]),
        ("G1", ["G2", "D3", "F3", "Bb3", "A2", "E3", "G3", "C#4"]),
    ]
    for b, (root, notes) in enumerate(arps):
        for i, n in enumerate(notes):
            tr.add("piano", b * 4 + i * 0.5, piano(n, tr.beats_to_s(1.2), 0.45))
        tr.add("pad", b * 4, pad(root, tr.beats_to_s(4.0), 0.3, 1.0, 1.2, 700))
    strings = [["F3", "A3", "D4"], ["F3", "A3", "D4"], ["F3", "A3", "C4"], ["F3", "Bb3", "D4"]]
    for b, ch in enumerate(strings):
        for n in ch: tr.add("pad", b * 4, pad(n, tr.beats_to_s(4.1), 0.22, 1.2, 1.2, 1400))
    mel = [
        [("D5", 0, 1), ("A5", 1, 2), ("G5", 3, .5), ("F5", 3.5, .5)],
        [("D5", 0, 3), ("C5", 3, 1)],
        [("C5", 0, 1.5), ("A4", 1.5, .5), ("C5", 2, 2)],
        [("Bb4", 0, 2), ("A4", 2, 1), ("C#5", 3, 1)],
    ]
    for bar, notes in enumerate(mel):
        for n, beat, dur in notes:
            tr.add("lead", bar * 4 + beat, piano(n, tr.beats_to_s(dur), 0.6))
    x = mixdown(tr, {"piano": 0.55, "pad": 0.4, "lead": 0.55},
                {"piano": 0.45, "pad": 0.5, "lead": 0.5}, wet=0.45, verb=dict(dur=2.6, decay=0.9, bright=5000), hi=9000)
    save("bgm_death", x)
