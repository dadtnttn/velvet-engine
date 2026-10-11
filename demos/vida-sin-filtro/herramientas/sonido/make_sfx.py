import sys, os, numpy as np
sys.path.insert(0, os.path.dirname(__file__))
from synth_lib import *

OUT = sys.argv[1]
os.makedirs(OUT, exist_ok=True)

def place(total, items):
    """items: [(segundos, señal, ganancia)] -> mezcla de duración `total`."""
    n = int(total * SR)
    for s0, sig, g in items:
        n = max(n, int(s0 * SR) + len(sig))
    out = np.zeros(n)
    for s0, sig, g in items:
        i = int(s0 * SR)
        # cada pieza termina con un fundido corto: nada de clics al cortar
        sig = sig * tail(len(sig), min(0.04, len(sig) / SR * 0.25))
        out[i:i + len(sig)] += sig * g
    return out

def swoosh(dur, f0, f1, curve=1.0, gain=1.0):
    x = noise(dur)
    t = tt(dur)
    y = onepole_sweep(x, f0, f1, curve)
    y = y - onepole_sweep(y, 200, 200)  # quita graves
    env = np.sin(np.pi * np.minimum(1, t / dur)) ** 1.5
    return y * env * gain

def bell_fm(f, dur, ratio=3.5, index=2.0, dec=0.25, vel=1.0):
    f = hz(f); t = tt(dur)
    idx = index * np.exp(-t / (dec * 0.4))
    x = np.sin(2 * np.pi * f * t + idx * np.sin(2 * np.pi * f * ratio * t))
    return x * np.exp(-t / dec) * attack(len(t), 0.001) * vel * tail(len(t), 0.01)

def blip(f0, f1, dur, vel=1.0, shape="sine"):
    t = tt(dur)
    fr = hz(f0) * (hz(f1) / hz(f0)) ** (t / dur)
    ph = 2 * np.pi * np.cumsum(fr) / SR
    if shape == "sine": x = np.sin(ph)
    else: x = np.sign(np.sin(ph)) * 0.5 + 0.5 * np.sin(ph)
    return x * np.exp(-t / (dur * 0.45)) * attack(len(t), 0.002) * vel * tail(len(t), 0.01)

MAXLEN = {"sfx_death": 3.2, "sfx_love": 1.3, "sfx_good": 1.0, "sfx_rank": 1.2, "sfx_age": 1.2, "sfx_break": 1.1, "sfx_bad": 0.8}

def save(name, x, peak=0.85, verb=None):
    if verb: x = reverb(x, **verb)
    x = x - np.mean(x)
    # recorta la cola cuando ya casi no se oye (-40 dB) o al largo máximo
    thr = np.max(np.abs(x)) * 0.01
    idx = np.where(np.abs(x) > thr)[0]
    if len(idx): x = x[:idx[-1] + int(0.01 * SR)]
    x = x[:int(MAXLEN.get(name, 0.7) * SR)]
    x = x * tail(len(x), min(0.12, len(x) / SR * 0.3))
    d = write_wav(os.path.join(OUT, name + ".wav"), x, peak)
    print(f"{name:12s} {d:5.2f}s")

# 1. Abrir menú: barrido brillante + campanita (estilo Persona)
save("sfx_open", place(0.35, [
    (0.0, swoosh(0.16, 900, 7000, 0.7), 0.55),
    (0.05, bell_fm("E6", 0.3, 3.0, 1.2, 0.09), 0.5),
    (0.075, bell_fm("B6", 0.3, 3.0, 1.0, 0.08), 0.35),
]), 0.8, dict(wet=0.18, dur=0.4, decay=0.1))

# 2. Volver/cerrar: barrido hacia abajo + blip grave
save("sfx_back", place(0.25, [
    (0.0, swoosh(0.13, 6000, 900, 0.7), 0.5),
    (0.03, blip("G5", "D5", 0.09), 0.45),
]), 0.7, dict(wet=0.12, dur=0.3, decay=0.08))

# 3. Tic (cursor / cambio de pestaña): clic corto y limpio
t = tt(0.035)
tick = np.sin(2 * np.pi * 2900 * t) * np.exp(-t / 0.006) + 0.35 * fft_filter(noise(0.035), lo=3000, hi=9000) * np.exp(-t / 0.002)
save("sfx_tick", tick, 0.55)

# 4. Confirmar: dos notas que suben
save("sfx_ok", place(0.4, [
    (0.0, bell_fm("C6", 0.25, 2.0, 1.4, 0.08), 0.6),
    (0.065, bell_fm("G6", 0.35, 2.0, 1.2, 0.12), 0.55),
    (0.0, rim(0.6), 0.4),
]), 0.75, dict(wet=0.15, dur=0.4, decay=0.1))

# 5. Algo bueno: arpegio mayor de vibráfono con brillo
save("sfx_good", place(0.9, [
    (0.00, vibes("C5", 0.25, 0.8), 0.55),
    (0.06, vibes("E5", 0.25, 0.8), 0.55),
    (0.12, vibes("G5", 0.25, 0.8), 0.55),
    (0.18, vibes("C6", 0.45, 0.9), 0.6),
    (0.18, glock("G6", 0.3, 0.5), 0.25),
]), 0.8, dict(wet=0.25, dur=0.8, decay=0.2))

# 6. Algo malo: dos notas que caen (menor) con golpe sordo
def sour(f, dur):
    t = tt(dur)
    fr = hz(f) * (1 - 0.03 * (t / dur) ** 2)
    ph = 2 * np.pi * np.cumsum(fr) / SR
    x = sum(np.sin(k * ph) / k for k in range(1, 9))
    x = fft_filter(x, hi=1600)
    return x * attack(len(t), 0.01) * np.exp(-t / (dur * 0.6)) * tail(len(t), 0.03)
thud = np.sin(2 * np.pi * np.cumsum(90 * np.exp(-tt(0.25) / 0.05) + 45) / SR) * np.exp(-tt(0.25) / 0.07)
save("sfx_bad", place(0.7, [
    (0.0, sour("Eb4", 0.22), 0.5),
    (0.0, sour("Gb4", 0.22), 0.3),
    (0.2, sour("B3", 0.45), 0.55),
    (0.2, sour("D4", 0.45), 0.3),
    (0.2, thud, 0.5),
]), 0.75, dict(wet=0.15, dur=0.5, decay=0.12))

# 7. Amor: glissando de arpa + acorde suave
harp = []
for i, n in enumerate(["C5", "D5", "E5", "G5", "A5", "C6", "D6", "E6"]):
    harp.append((i * 0.032, pluck_ks(n, 0.5, 0.6, 0.995), 0.35))
harp.append((0.05, pad("E5", 0.5, 0.4, 0.15, 0.4, 2500), 0.35))
harp.append((0.05, pad("G5", 0.5, 0.4, 0.15, 0.4, 2500), 0.3))
harp.append((0.25, glock("C7", 0.3, 0.5), 0.18))
save("sfx_love", place(1.0, harp), 0.75, dict(wet=0.3, dur=1.0, decay=0.25))

# 8. Dinero: golpe metálico + moneda
def metal(dur):
    t = tt(dur)
    x = sum(a * np.sin(2 * np.pi * f * t) * np.exp(-t / d) for f, a, d in [(2350, 1, 0.12), (3120, 0.7, 0.09), (4410, 0.5, 0.06), (5650, 0.3, 0.04)])
    return x * attack(len(t), 0.0005)
def coin(f, dur):
    t = tt(dur)
    ph = 2 * np.pi * hz(f) * t
    x = 0.6 * np.sign(np.sin(ph)) + 0.4 * np.sin(ph)
    x = fft_filter(x, hi=7000)
    return x * np.exp(-t / (dur * 0.5)) * tail(len(t), 0.01)
save("sfx_money", place(0.6, [
    (0.0, metal(0.3), 0.35),
    (0.0, fft_filter(noise(0.05), lo=2000, hi=9000) * np.exp(-tt(0.05) / 0.008), 0.3),
    (0.05, coin("B5", 0.08), 0.25),
    (0.12, coin("E6", 0.35), 0.25),
]), 0.75, dict(wet=0.15, dur=0.5, decay=0.12))

# 9. Aviso (toast): burbuja que sube + clic
t = tt(0.16)
fr = 420 * (1500 / 420) ** np.minimum(1, t / 0.05)
bub = np.sin(2 * np.pi * np.cumsum(fr) / SR) * np.exp(-t / 0.05) * attack(len(t), 0.002)
save("sfx_toast", place(0.25, [(0.0, bub, 0.6), (0.0, rim(0.5), 0.25), (0.05, bell_fm("A6", 0.15, 2.0, 0.8, 0.05), 0.2)]), 0.6,
     dict(wet=0.12, dur=0.3, decay=0.08))

# 10. Cumpleaños / nuevo año: barrido + golpe de acorde (cambio de día estilo Persona)
stab = np.zeros(1)
chord = place(0.9, [(0, epiano(n, 0.25, 0.9, 1.3), 0.3) for n in ["D4", "F4", "A4", "C5", "E5"]])
sub = np.sin(2 * np.pi * np.cumsum(60 + 60 * np.exp(-tt(0.5) / 0.03)) / SR) * np.exp(-tt(0.5) / 0.18)
save("sfx_age", place(1.1, [
    (0.0, swoosh(0.24, 500, 9000, 1.3), 0.45),
    (0.22, chord, 0.8),
    (0.22, sub, 0.55),
    (0.22, crash(0.6)[:int(0.7 * SR)], 0.16),
    (0.22, clap(0.8), 0.35),
]), 0.85, dict(wet=0.22, dur=0.9, decay=0.2))

# 11. Muerte: campana grave que dobla
def church_bell(f0, dur):
    t = tt(dur)
    x = np.zeros(len(t))
    for r, a, d in [(0.5, 0.6, 2.4), (1.0, 1.0, 1.8), (1.19, 0.45, 1.2), (1.5, 0.35, 1.0), (2.0, 0.5, 0.9),
                    (2.52, 0.25, 0.6), (2.66, 0.2, 0.5), (3.01, 0.18, 0.4), (4.0, 0.12, 0.3), (5.2, 0.06, 0.15)]:
        x += a * np.sin(2 * np.pi * f0 * r * t + r) * np.exp(-t / d)
    strike = fft_filter(noise(dur), lo=500, hi=4000) * np.exp(-t / 0.01) * 0.2
    return (x + strike) * attack(len(t), 0.001)
save("sfx_death", place(3.2, [
    (0.0, church_bell(196.0, 3.0), 0.7),
    (0.0, pad("D3", 2.0, 0.35, 0.6, 1.0, 900), 0.4),
    (0.0, pad("A3", 2.0, 0.3, 0.6, 1.0, 900), 0.3),
]), 0.8, dict(wet=0.35, dur=2.0, decay=0.6))

# 12. Sello: golpe seco + papel
t = tt(0.3)
th = np.sin(2 * np.pi * np.cumsum(55 + 110 * np.exp(-t / 0.02)) / SR) * np.exp(-t / 0.08)
paper = fft_filter(noise(0.3), lo=900, hi=5000, slope=0.5) * np.exp(-t / 0.025)
save("sfx_stamp", place(0.35, [(0.0, np.tanh(2 * th), 0.8), (0.0, paper, 0.5), (0.03, clap(0.5), 0.3)]), 0.85,
     dict(wet=0.12, dur=0.4, decay=0.08))

# 13. Error / bloqueado: dos zumbidos graves
def buzz(dur):
    t = tt(dur)
    ph = 2 * np.pi * 140 * t
    x = fft_filter(np.sign(np.sin(ph)) * 0.7 + 0.3 * np.sin(2 * ph), hi=1800)
    return x * attack(len(t), 0.004) * tail(len(t), 0.015)
save("sfx_error", place(0.25, [(0.0, buzz(0.07), 0.45), (0.11, buzz(0.09), 0.45)]), 0.6)

# 14. Celular: marimba de dos notas
save("sfx_phone", place(0.6, [(0.0, marimba("A5", 0.2, 0.9), 0.6), (0.1, marimba("E6", 0.2, 0.9), 0.6), (0.2, marimba("A6", 0.2, 0.6), 0.35)]), 0.7,
     dict(wet=0.15, dur=0.4, decay=0.1))

# 15. Subir de rango: fanfarria corta de metales sintéticos
def brass(f, dur, vel=0.8):
    f = hz(f); t = tt(dur + 0.15)
    x = saw_bl(f, t, 18)
    # el filtro se abre al atacar (metal)
    x = onepole_sweep(x, 4500, 900, 0.5)
    env = attack(len(t), 0.02) * np.where(t < dur, 1.0, np.exp(-(t - dur) / 0.05))
    return x * env * vel * tail(len(t), 0.02)
save("sfx_rank", place(1.0, [
    (0.00, brass("G4", 0.09), 0.35), (0.00, brass("C5", 0.09), 0.3),
    (0.12, brass("G4", 0.09), 0.35), (0.12, brass("C5", 0.09), 0.3),
    (0.24, brass("C5", 0.5), 0.35), (0.24, brass("E5", 0.5), 0.3), (0.24, brass("G5", 0.5), 0.3),
    (0.24, glock("C7", 0.3, 0.6), 0.18), (0.24, crash(0.5), 0.2),
]), 0.8, dict(wet=0.22, dur=0.8, decay=0.18))

# 16. Corazón roto / ruptura: nota que se desafina hacia abajo
t = tt(0.8)
fr = hz("E5") * (1 - 0.12 * (t / 0.8) ** 1.5)
br = np.sin(2 * np.pi * np.cumsum(fr) / SR) + 0.3 * np.sin(2 * np.pi * np.cumsum(2 * fr) / SR)
br = br * attack(len(t), 0.02) * np.exp(-t / 0.4)
save("sfx_break", place(0.9, [(0.0, glock("B5", 0.2, 0.7), 0.35), (0.08, br, 0.45)]), 0.7, dict(wet=0.3, dur=0.9, decay=0.25))

# 17. Pasar el mes: hoja de calendario + nota suave
t = tt(0.12)
flip = fft_filter(noise(0.12), lo=1200, hi=6500, slope=0.5)
env = np.exp(-t / 0.018) + 0.6 * np.where(t > 0.035, np.exp(-(t - 0.035) / 0.02), 0)
save("sfx_month", place(0.3, [(0.0, flip * env, 0.5), (0.03, marimba("D5", 0.2, 0.6), 0.3)]), 0.55,
     dict(wet=0.1, dur=0.3, decay=0.08))
