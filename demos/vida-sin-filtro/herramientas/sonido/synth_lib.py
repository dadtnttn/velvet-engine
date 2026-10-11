"""Sintetizador mínimo (numpy) para los sonidos de Vida Sin Filtro."""
import numpy as np, wave

SR = 22050
_rng = np.random.default_rng(1234)

def secs(n): return n / SR
def tt(dur): return np.arange(int(round(dur * SR))) / SR

def noise(dur): return _rng.standard_normal(int(round(dur * SR)))

NOTE = {"C": -9, "C#": -8, "Db": -8, "D": -7, "D#": -6, "Eb": -6, "E": -5, "F": -4, "F#": -3,
        "Gb": -3, "G": -2, "G#": -1, "Ab": -1, "A": 0, "A#": 1, "Bb": 1, "B": 2}

def hz(name):
    """'A4' -> 440, 'C#5', 'Bb2'..."""
    if isinstance(name, (int, float)): return float(name)
    p = name[:-1]; o = int(name[-1])
    return 440.0 * 2 ** ((NOTE[p] + (o - 4) * 12) / 12)

def tail(n, rel):
    """Fundido final para que nada truene al cortar."""
    e = np.ones(n)
    k = min(n, int(rel * SR))
    if k > 0: e[n - k:] = np.linspace(1, 0, k) ** 2
    return e

def attack(n, a):
    e = np.ones(n)
    k = min(n, max(1, int(a * SR)))
    e[:k] = np.linspace(0, 1, k)
    return e

# ------------------------------------------------------------- filtros (FFT)
def fft_filter(x, lo=None, hi=None, slope=0.25):
    """Filtro de fase cero con caída suave (en octavas)."""
    n = len(x)
    if n == 0: return x
    X = np.fft.rfft(x)
    f = np.fft.rfftfreq(n, 1 / SR)
    g = np.ones_like(f)
    with np.errstate(divide="ignore", invalid="ignore"):
        if hi is not None:
            oct_ = np.log2(np.maximum(f, 1e-9) / hi)
            g *= np.where(oct_ <= 0, 1.0, np.exp(-(oct_ / slope) ** 2))
        if lo is not None:
            oct_ = np.log2(lo / np.maximum(f, 1e-9))
            g *= np.where(oct_ <= 0, 1.0, np.exp(-(oct_ / slope) ** 2))
    return np.fft.irfft(X * g, n)

def onepole_sweep(x, f_from, f_to, curve=1.0):
    """Paso bajo de un polo con frecuencia que se mueve (bucle: solo para cosas cortas)."""
    n = len(x); y = np.zeros(n); s = 0.0
    fr = f_from * (f_to / f_from) ** (np.linspace(0, 1, n) ** curve)
    a = 1 - np.exp(-2 * np.pi * fr / SR)
    for i in range(n):
        s += a[i] * (x[i] - s); y[i] = s
    return y

# ------------------------------------------------------------- reverb
def ir(dur=1.2, decay=0.35, bright=6000, seed=5, predelay=0.012):
    r = np.random.default_rng(seed)
    n = int(dur * SR)
    t = np.arange(n) / SR
    h = r.standard_normal(n) * np.exp(-t / decay)
    h = fft_filter(h, lo=180, hi=bright, slope=0.6)
    pd = int(predelay * SR)
    h = np.concatenate([np.zeros(pd), h])
    return h / np.sqrt(np.sum(h ** 2))

def reverb(x, wet=0.25, **kw):
    h = ir(**kw)
    n = len(x) + len(h)
    m = 1 << (n - 1).bit_length()
    y = np.fft.irfft(np.fft.rfft(x, m) * np.fft.rfft(h, m), m)[:len(x) + len(h) - 1]
    out = np.zeros(len(y)); out[:len(x)] += x
    return out * (1 - wet * 0.5) + y * wet

def reverb_loop(x, wet=0.25, **kw):
    """Reverb circular: la cola vuelve al principio (para bucles sin costura)."""
    h = ir(**kw)
    n = len(x)
    hh = np.zeros(n)
    k = min(n, len(h)); hh[:k] = h[:k]
    if len(h) > n:  # IR más largo que el bucle: se dobla
        rest = h[n:]
        for i in range(0, len(rest), n):
            seg = rest[i:i + n]; hh[:len(seg)] += seg
    y = np.fft.irfft(np.fft.rfft(x) * np.fft.rfft(hh), n)
    return x * (1 - wet * 0.5) + y * wet

# ------------------------------------------------------------- instrumentos
def epiano(f, dur, vel=0.8, bright=1.0):
    """Piano eléctrico tipo Rhodes por FM."""
    f = hz(f); d = dur + 0.6
    t = tt(d)
    idx = (1.4 * bright * vel) * np.exp(-t / 0.28) + 0.18
    car = np.sin(2 * np.pi * f * t + idx * np.sin(2 * np.pi * f * t))
    tine = 0.10 * vel * bright * np.sin(2 * np.pi * f * 7.0 * t) * np.exp(-t / 0.025)
    amp = np.exp(-t / 1.6) * attack(len(t), 0.002)
    # al soltar la tecla, se apaga más rápido
    rel = np.where(t < dur, 1.0, np.exp(-(t - dur) / 0.12))
    trem = 1 + 0.07 * np.sin(2 * np.pi * 4.6 * t)
    return (car * amp * trem + tine) * rel * vel * tail(len(t), 0.02)

def piano(f, dur, vel=0.8):
    """Piano acústico aproximado (aditivo con inarmonía + martillo)."""
    f = hz(f); d = dur + 1.2
    t = tt(d)
    x = np.zeros(len(t))
    B = 0.0004
    for k in range(1, 9):
        fk = f * k * np.sqrt(1 + B * k * k)
        if fk > SR / 2 - 500: break
        a = (1.0 / k ** 1.25) * (0.6 + 0.4 * vel)
        dk = 2.4 / (1 + 0.45 * k) * (220 / max(f, 110)) ** 0.35
        x += a * np.sin(2 * np.pi * fk * t + k) * np.exp(-t / dk)
    hammer = fft_filter(noise(d), lo=800, hi=4000) * np.exp(-t / 0.006) * 0.15
    rel = np.where(t < dur, 1.0, np.exp(-(t - dur) / 0.25))
    return (x + hammer) * attack(len(t), 0.002) * rel * vel * 0.5 * tail(len(t), 0.03)

def vibes(f, dur, vel=0.8):
    """Vibráfono (FM 1:4 suave con trémolo)."""
    f = hz(f); d = dur + 1.0
    t = tt(d)
    idx = 0.9 * np.exp(-t / 0.08) + 0.05
    x = np.sin(2 * np.pi * f * t + idx * np.sin(2 * np.pi * 4 * f * t))
    x += 0.25 * np.sin(2 * np.pi * f * 4 * t) * np.exp(-t / 0.15)
    trem = 1 + 0.18 * np.sin(2 * np.pi * 5.5 * t)
    amp = np.exp(-t / 1.1) * attack(len(t), 0.001)
    rel = np.where(t < dur, 1.0, np.exp(-(t - dur) / 0.3))
    return x * amp * trem * rel * vel * tail(len(t), 0.03)

def glock(f, dur, vel=0.8):
    """Campanitas / caja de música."""
    f = hz(f); d = max(dur, 0.2) + 1.2
    t = tt(d)
    x = np.zeros(len(t))
    for r, a, dec in [(1, 1.0, 0.9), (2.76, 0.35, 0.3), (5.40, 0.18, 0.12), (8.93, 0.08, 0.05)]:
        if f * r < SR / 2 - 300:
            x += a * np.sin(2 * np.pi * f * r * t) * np.exp(-t / dec)
    return x * attack(len(t), 0.0008) * vel * 0.6 * tail(len(t), 0.05)

def marimba(f, dur, vel=0.8):
    f = hz(f); d = 0.6
    t = tt(d)
    x = np.sin(2 * np.pi * f * t) * np.exp(-t / 0.22)
    x += 0.3 * np.sin(2 * np.pi * f * 3.93 * t) * np.exp(-t / 0.03)
    x += 0.12 * np.sin(2 * np.pi * f * 9.3 * t) * np.exp(-t / 0.01)
    return x * attack(len(t), 0.0008) * vel * tail(len(t), 0.03)

def bass(f, dur, vel=0.8, tone=1.0):
    f = hz(f); d = dur + 0.08
    t = tt(d)
    x = np.sin(2 * np.pi * f * t) + 0.35 * tone * np.sin(2 * np.pi * 2 * f * t + 0.4) + 0.12 * tone * np.sin(2 * np.pi * 3 * f * t + 1.0)
    env = attack(len(t), 0.004) * np.exp(-t / 0.6)
    rel = np.where(t < dur, 1.0, np.exp(-(t - dur) / 0.03))
    pluck = 1 + 0.6 * tone * np.exp(-t / 0.03)
    return np.tanh(1.4 * x * env * pluck) * 0.75 * rel * vel * tail(len(t), 0.01)

def pizz(f, dur, vel=0.8):
    f = hz(f); d = 0.35
    t = tt(d)
    x = np.sin(2 * np.pi * f * t) + 0.3 * np.sin(2 * np.pi * 2 * f * t) + 0.1 * np.sin(2 * np.pi * 3 * f * t)
    return x * np.exp(-t / 0.09) * attack(len(t), 0.002) * vel * 0.8 * tail(len(t), 0.02)

def saw_bl(f, t, kmax=24):
    x = np.zeros(len(t))
    for k in range(1, kmax + 1):
        if f * k > SR / 2 - 400: break
        x += np.sin(2 * np.pi * f * k * t) / k
    return x

def pad(f, dur, vel=0.5, att=0.5, rel=0.8, cutoff=1800):
    f = hz(f); d = dur + rel
    t = tt(d)
    x = sum(saw_bl(f * c, t, 14) for c in (0.996, 1.0, 1.005)) / 3
    x = fft_filter(x, lo=80, hi=cutoff, slope=0.5)
    env = np.minimum(1, t / att)
    env = np.where(t < dur, env, env * np.exp(-(t - dur) / (rel / 3)))
    return x * env * vel * tail(len(t), 0.05)

def lead(f, dur, vel=0.6, vib=0.006):
    """Flauta/sintetizador suave (triangular + vibrato tardío)."""
    f = hz(f); d = dur + 0.12
    t = tt(d)
    v = 1 + vib * np.sin(2 * np.pi * 5.2 * t) * np.minimum(1, t / 0.25)
    ph = 2 * np.pi * np.cumsum(f * v) / SR
    x = (2 / np.pi) * np.arcsin(np.sin(ph)) * 0.8 + 0.25 * np.sin(2 * ph)
    env = attack(len(t), 0.03)
    env = np.where(t < dur, env, np.exp(-(t - dur) / 0.04))
    return x * env * vel * 0.5 * tail(len(t), 0.02)

def pluck_ks(f, dur, vel=0.7, damp=0.996):
    """Cuerda pulsada (Karplus–Strong)."""
    f = hz(f); n = int((dur + 0.4) * SR)
    p = max(2, int(SR / f))
    buf = _rng.uniform(-1, 1, p)
    out = np.zeros(n)
    for i in range(n):
        j = i % p
        out[i] = buf[j]
        buf[j] = damp * 0.5 * (buf[j] + buf[(j + 1) % p])
    return out * vel * tail(n, 0.05)

# ------------------------------------------------------------- batería
def kick(vel=1.0):
    t = tt(0.4)
    fr = 44 + 100 * np.exp(-t / 0.028)
    x = np.sin(2 * np.pi * np.cumsum(fr) / SR) * np.exp(-t / 0.16)
    click = fft_filter(noise(0.4), lo=1500, hi=6000) * np.exp(-t / 0.0025) * 0.25
    return np.tanh(1.6 * (x + click)) * vel * 0.9 * tail(len(t), 0.02)

def snare(vel=1.0):
    t = tt(0.3)
    nz = fft_filter(noise(0.3), lo=1400, hi=7000, slope=0.5) * np.exp(-t / 0.075)
    tone = np.sin(2 * np.pi * (175 + 40 * np.exp(-t / 0.01)) * t) * np.exp(-t / 0.045)
    return (0.55 * nz + 0.55 * tone) * vel * tail(len(t), 0.02)

def rim(vel=1.0):
    t = tt(0.08)
    x = np.sin(2 * np.pi * 1650 * t) * np.exp(-t / 0.012) + 0.4 * fft_filter(noise(0.08), lo=2000, hi=8000) * np.exp(-t / 0.004)
    return x * vel * 0.5

def clap(vel=1.0):
    t = tt(0.25)
    nz = fft_filter(noise(0.25), lo=900, hi=5000, slope=0.5)
    env = np.zeros(len(t))
    for o in (0.0, 0.011, 0.022):
        env += np.where(t >= o, np.exp(-(t - o) / 0.008), 0)
    env += np.where(t >= 0.03, 0.6 * np.exp(-(t - 0.03) / 0.06), 0)
    return nz * env * vel * 0.45

def hat(vel=1.0, open_=False):
    d = 0.32 if open_ else 0.07
    t = tt(d)
    nz = fft_filter(noise(d), lo=6500, hi=10500, slope=0.4)
    dec = 0.11 if open_ else 0.016
    return nz * np.exp(-t / dec) * vel * 0.45 * tail(len(t), 0.01)

def shaker(vel=1.0):
    t = tt(0.09)
    nz = fft_filter(noise(0.09), lo=5000, hi=10000, slope=0.4)
    env = np.minimum(1, t / 0.02) * np.exp(-t / 0.03)
    return nz * env * vel * 0.4

def crash(vel=1.0):
    t = tt(1.6)
    nz = fft_filter(noise(1.6), lo=3500, hi=10500, slope=0.6)
    return nz * np.exp(-t / 0.5) * vel * 0.3

# ------------------------------------------------------------- mezcla
class Track:
    """Partitura en tiempo de pulsos; se renderiza en un búfer circular (bucle)."""
    def __init__(self, bpm, bars, beats=4, swing=0.0):
        self.bpm = bpm; self.beats = beats; self.bars = bars; self.swing = swing
        self.spb = 60.0 / bpm
        self.n = int(round(bars * beats * self.spb * SR))
        self.buses = {}

    def at(self, beat):
        # swing de semicorcheas: la segunda de cada par se retrasa
        if self.swing:
            q = beat * 4
            if abs(q - round(q)) < 1e-6 and int(round(q)) % 2 == 1:
                beat += self.swing / 4
        return int(round(beat * self.spb * SR))

    def add(self, bus, beat, sig, gain=1.0):
        b = self.buses.setdefault(bus, np.zeros(self.n))
        s = self.at(beat) % self.n
        sig = sig * gain
        end = s + len(sig)
        if end <= self.n:
            b[s:end] += sig
        else:
            first = self.n - s
            b[s:] += sig[:first]
            rest = sig[first:]
            while len(rest) > 0:
                k = min(len(rest), self.n)
                b[:k] += rest[:k]
                rest = rest[k:]

    def beats_to_s(self, b): return b * self.spb

def master(x, peak=0.72, drive=1.15):
    x = x - np.mean(x)
    x = np.tanh(drive * x / max(1e-9, np.max(np.abs(x)))) 
    return x / max(1e-9, np.max(np.abs(x))) * peak

def write_wav(path, x, peak=None):
    if peak is not None:
        x = x / max(1e-9, np.max(np.abs(x))) * peak
    y = np.clip(x, -1, 1)
    pcm = (y * 32767).astype("<i2")
    with wave.open(path, "wb") as w:
        w.setnchannels(1); w.setsampwidth(2); w.setframerate(SR)
        w.writeframes(pcm.tobytes())
    return len(pcm) / SR
