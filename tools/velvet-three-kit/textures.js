// Painted canvas textures for the house. Everything that tiles is drawn nine
// times at wrapped offsets, so the seams never show; everything meant to be
// seen once (rugs, banners, paintings, signs) is painted for its own rectangle.
import * as THREE from "three";
import { withRelief, heightFromColor } from "./relief.js";

export function mulberry(seed) {
  let a = seed >>> 0;
  return () => {
    a = (a + 0x6d2b79f5) >>> 0;
    let t = a;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

export function canvas(w, h) {
  const c = document.createElement("canvas");
  c.width = w;
  c.height = h;
  return [c, c.getContext("2d")];
}

export function toTexture(c, { repeat = [1, 1], srgb = true, wrap = true, aniso = 16 } = {}) {
  const t = new THREE.CanvasTexture(c);
  t.wrapS = t.wrapT = wrap ? THREE.RepeatWrapping : THREE.ClampToEdgeWrapping;
  t.repeat.set(repeat[0], repeat[1]);
  t.colorSpace = srgb ? THREE.SRGBColorSpace : THREE.NoColorSpace;
  t.anisotropy = aniso;
  return t;
}

/** A copy of a texture turned a quarter, for grain that runs the other way. */
export function turned(texture) {
  const t = texture.clone();
  t.rotation = Math.PI / 2;
  t.center.set(0.5, 0.5);
  t.needsUpdate = true;
  t.userData = { ...texture.userData };
  if (texture.userData.normal) {
    const n = texture.userData.normal.clone();
    n.rotation = Math.PI / 2;
    n.center.set(0.5, 0.5);
    n.needsUpdate = true;
    t.userData.normal = n;
  }
  return t;
}

/** Grey for a height canvas (0 deep .. 1 proud). */
export const grey = (v) => {
  const g = Math.max(0, Math.min(255, Math.round(v * 255)));
  return `rgb(${g},${g},${g})`;
};

/** Runs `draw` at the nine wrapped offsets so the result tiles seamlessly. */
export function tiled(ctx, W, H, draw) {
  for (const dx of [-W, 0, W]) {
    for (const dy of [-H, 0, H]) {
      ctx.save();
      ctx.translate(dx, dy);
      draw();
      ctx.restore();
    }
  }
}

export function shade([r, g, b], t) {
  return `rgb(${Math.max(0, Math.min(255, Math.round(r * t)))},${Math.max(0, Math.min(255, Math.round(g * t)))},${Math.max(0, Math.min(255, Math.round(b * t)))})`;
}

export function specks(ctx, W, H, rng, count, palette, size = 2, alpha = [0.05, 0.16]) {
  for (let i = 0; i < count; i++) {
    ctx.fillStyle = palette[Math.floor(rng() * palette.length)];
    ctx.globalAlpha = alpha[0] + rng() * (alpha[1] - alpha[0]);
    ctx.fillRect(rng() * W, rng() * H, size * (0.5 + rng()), size * (0.5 + rng()));
  }
  ctx.globalAlpha = 1;
}

export function blobs(ctx, W, H, seed, count, warm, light, rMin = 40, rMax = 120) {
  tiled(ctx, W, H, () => {
    const rng = mulberry(seed);
    for (let i = 0; i < count; i++) {
      const x = rng() * W, y = rng() * H, r = rMin + rng() * (rMax - rMin);
      const g = ctx.createRadialGradient(x, y, 0, x, y, r);
      const isWarm = rng() < 0.55;
      g.addColorStop(0, isWarm ? warm.replace("A", (0.04 + rng() * 0.06).toFixed(3)) : light.replace("A", (0.04 + rng() * 0.06).toFixed(3)));
      g.addColorStop(1, "rgba(0,0,0,0)");
      ctx.fillStyle = g;
      ctx.fillRect(x - r, y - r, r * 2, r * 2);
    }
  });
}

// ------------------------------------------------------------------ plaster
/** Hand-painted cream plaster over soft blocks: flat tones, a lit top edge, a shaded foot, a few
 *  hairline cracks; the blocks stand proud of sunken joints. 1024 px = 2 m. */
export function plasterTexture() {
  const S = 1024, [c, ctx] = canvas(S, S), [hc, hx] = canvas(S, S);
  ctx.fillStyle = "#c9ab7c";
  ctx.fillRect(0, 0, S, S);
  hx.fillStyle = grey(0.2);
  hx.fillRect(0, 0, S, S);
  const draw = (g, height) => tiled(g, S, S, () => {
    const rng = mulberry(101);
    const rows = 5, rowH = S / rows;
    for (let r = 0; r < rows; r++) {
      const n = 3 + Math.floor(rng() * 2);
      const cuts = Array.from({ length: n }, () => 0.65 + rng() * 0.7);
      const sum = cuts.reduce((a, b) => a + b, 0);
      let x = rng() * S;
      const y = r * rowH;
      for (let k = 0; k < n; k++) {
        const w = (cuts[k] / sum) * S;
        const tone = 0.965 + rng() * 0.06;
        const crack = rng() < 0.3 ? [x + w * (0.2 + rng() * 0.6), Array.from({ length: 4 }, () => [(rng() - 0.5) * 26, 12 + rng() * 16])] : null;
        if (height) {
          // A soft dome per block: proud in the middle, rounding into the joint.
          const grd = g.createRadialGradient(x + w / 2, y + rowH / 2, 10, x + w / 2, y + rowH / 2, Math.max(w, rowH) * 0.7);
          grd.addColorStop(0, grey(0.95));
          grd.addColorStop(1, grey(0.72));
          g.fillStyle = grd;
          g.beginPath();
          g.roundRect(x + 5, y + 5, w - 10, rowH - 10, 18);
          g.fill();
          if (crack) {
            g.strokeStyle = grey(0.45);
            g.lineWidth = 2.5;
            let [cx, cy] = [crack[0], y + 10];
            g.beginPath();
            g.moveTo(cx, cy);
            for (const [dx, dy] of crack[1]) g.lineTo((cx += dx), (cy += dy));
            g.stroke();
          }
        } else {
          const grd = g.createLinearGradient(0, y, 0, y + rowH);
          grd.addColorStop(0, shade([246, 229, 194], tone));
          grd.addColorStop(1, shade([234, 211, 170], tone));
          g.fillStyle = grd;
          g.beginPath();
          g.roundRect(x + 4, y + 4, w - 8, rowH - 8, 18);
          g.fill();
          g.strokeStyle = "rgba(255,248,226,0.5)";
          g.lineWidth = 5;
          g.beginPath();
          g.moveTo(x + 14, y + rowH - 16);
          g.lineTo(x + 12, y + 12);
          g.lineTo(x + w - 16, y + 10);
          g.stroke();
          g.strokeStyle = "rgba(150,106,58,0.3)";
          g.lineWidth = 6;
          g.beginPath();
          g.moveTo(x + w - 10, y + 16);
          g.lineTo(x + w - 10, y + rowH - 11);
          g.lineTo(x + 16, y + rowH - 10);
          g.stroke();
          if (crack) {
            g.strokeStyle = "rgba(120,80,40,0.45)";
            g.lineWidth = 2;
            let [cx, cy] = [crack[0], y + 10];
            g.beginPath();
            g.moveTo(cx, cy);
            for (const [dx, dy] of crack[1]) g.lineTo((cx += dx), (cy += dy));
            g.stroke();
          }
        }
        x += w;
      }
    }
  });
  draw(ctx, false);
  draw(hx, true);
  const rng = mulberry(103);
  specks(ctx, S, S, rng, 700, ["#a5793f", "#fff4d8"], 3, [0.05, 0.1]);
  return withRelief(toTexture(c, { aniso: 16 }), hc, { strength: 1.6, scale: 0.8 });
}

// -------------------------------------------------------------------- floors
/**
 * One board of a painted floor or panel, into the colour and the height canvases. Boards run along
 * v (y on the canvas). The figure is one of three, as sawn timber shows it: straight quarter-sawn
 * lines, flat-sawn cathedral arches, or a lazy wave; every board has its own tone, a lit and a
 * shaded edge, a crowned face in the height map, and nails at its ends when `nails`.
 */
function board(ctx, hx, base, x, y, w, len, rng, { nails = true, px = 512 } = {}) {
  const odd = rng();
  // Most boards sit close to the base; one in eight is noticeably warmer, redder or paler.
  let tone = 0.88 + rng() * 0.2;
  let tint = [1, 1, 1];
  if (odd < 0.06) tint = [1.08, 0.96, 0.86];
  else if (odd < 0.12) tint = [1.07, 1.04, 0.97]; // a paler, honeyed board (never a cold grey one)
  else if (odd < 0.16) tone *= 0.8;
  const col = base.map((b, i) => b * tint[i]);
  const dark = shade(col, tone * 0.58), light = shade(col, tone * 1.14);
  ctx.fillStyle = shade(col, tone);
  ctx.fillRect(x, y, w, len);
  // Height: a gentle crown across the board, sinking at both ends.
  const cg = hx.createLinearGradient(x, 0, x + w, 0);
  cg.addColorStop(0, grey(0.55));
  cg.addColorStop(0.18, grey(0.82));
  cg.addColorStop(0.5, grey(0.9));
  cg.addColorStop(0.82, grey(0.82));
  cg.addColorStop(1, grey(0.55));
  hx.fillStyle = cg;
  hx.fillRect(x + 2, y + 2, w - 4, len - 4);
  // The figure.
  const figure = rng();
  ctx.save();
  ctx.beginPath();
  ctx.rect(x, y, w, len);
  ctx.clip();
  hx.save();
  hx.beginPath();
  hx.rect(x, y, w, len);
  hx.clip();
  const stroke = (pts, width, alpha) => {
    ctx.strokeStyle = dark;
    ctx.globalAlpha = alpha;
    ctx.lineWidth = width;
    ctx.beginPath();
    pts.forEach(([px_, py], i) => (i ? ctx.lineTo(px_, py) : ctx.moveTo(px_, py)));
    ctx.stroke();
    hx.strokeStyle = grey(0.7);
    hx.lineWidth = width * 0.8;
    hx.beginPath();
    pts.forEach(([px_, py], i) => (i ? hx.lineTo(px_, py) : hx.moveTo(px_, py)));
    hx.stroke();
  };
  if (figure < 0.4) {
    // Quarter-sawn: fine straight lines, slightly wandering.
    const lines = 4 + Math.floor(rng() * 3);
    for (let k = 0; k < lines; k++) {
      const gx = x + w * (0.1 + 0.8 * (k + rng() * 0.6) / lines), wob = rng() * 3, ph = rng() * 6;
      const pts = [];
      for (let t = 0; t <= 1.0001; t += 0.05) pts.push([gx + Math.sin(t * 4 + ph) * wob, y + t * len]);
      stroke(pts, 1.6, 0.28 + rng() * 0.12);
    }
  } else if (figure < 0.8) {
    // Flat-sawn: nested cathedral arches up the board, flanked by straight lines.
    const cx = x + w * (0.4 + rng() * 0.2), apex = y + len * (0.25 + rng() * 0.5);
    for (let k = 0; k < 5; k++) {
      const spread = w * (0.1 + k * 0.085), rise = px * (0.18 + k * 0.09);
      const pts = [];
      for (let t = -1; t <= 1.0001; t += 0.1) pts.push([cx + t * spread, apex + rise * (1 - Math.abs(t) ** 1.6) * -1 + rise]);
      pts.unshift([cx - spread, y + len]);
      pts.push([cx + spread, y + len]);
      stroke(pts.map(([a, b2]) => [a, Math.min(y + len, b2)]), 2, 0.3);
    }
    for (const side of [0.08, 0.92]) {
      const pts = [];
      for (let t = 0; t <= 1.0001; t += 0.1) pts.push([x + w * side, y + t * len]);
      stroke(pts, 1.4, 0.22);
    }
  } else {
    // A lazy wave.
    const lines = 3 + Math.floor(rng() * 2);
    for (let k = 0; k < lines; k++) {
      const gx = x + w * (0.15 + 0.7 * (k + 0.5) / lines), amp = w * (0.06 + rng() * 0.06), ph = rng() * 6, f = 2 + rng() * 3;
      const pts = [];
      for (let t = 0; t <= 1.0001; t += 0.03) pts.push([gx + Math.sin(t * f * (len / px) + ph) * amp, y + t * len]);
      stroke(pts, 2.2, 0.3);
    }
  }
  ctx.globalAlpha = 1;
  // A knot now and then, the grain rings around it.
  if (rng() < 0.18) {
    const kx = x + w * (0.3 + rng() * 0.4), ky = y + len * (0.2 + rng() * 0.6);
    ctx.fillStyle = shade(col, tone * 0.4);
    ctx.beginPath();
    ctx.ellipse(kx, ky, w * 0.07, w * 0.12, 0, 0, Math.PI * 2);
    ctx.fill();
    ctx.strokeStyle = dark;
    ctx.globalAlpha = 0.4;
    for (let k = 1; k <= 3; k++) {
      ctx.lineWidth = 2;
      ctx.beginPath();
      ctx.ellipse(kx, ky, w * 0.07 + k * 6, w * 0.12 + k * 11, 0, 0, Math.PI * 2);
      ctx.stroke();
    }
    ctx.globalAlpha = 1;
    hx.fillStyle = grey(0.62);
    hx.beginPath();
    hx.ellipse(kx, ky, w * 0.07, w * 0.12, 0, 0, Math.PI * 2);
    hx.fill();
  }
  ctx.restore();
  hx.restore();
  // Painted edges: lit on the left, shaded on the right.
  ctx.fillStyle = light;
  ctx.globalAlpha = 0.55;
  ctx.fillRect(x + 3, y, 3, len);
  ctx.globalAlpha = 1;
  ctx.fillStyle = "rgba(40,16,4,0.2)";
  ctx.fillRect(x + w - 7, y, 5, len);
  // Seams: a crisp dark line along and across, and a lit lip.
  ctx.fillStyle = "rgba(30,12,3,0.9)";
  ctx.fillRect(x, y, 3, len);
  ctx.fillRect(x, y, w, 3);
  ctx.fillStyle = "rgba(255,222,165,0.3)";
  ctx.fillRect(x, y + 3, w, 2);
  if (nails) {
    for (const ny of [y + 14, y + len - 14]) {
      for (const nx of [x + w * 0.25, x + w * 0.75]) {
        ctx.fillStyle = "#2a1a10";
        ctx.beginPath();
        ctx.arc(nx, ny, 3.2, 0, Math.PI * 2);
        ctx.fill();
        ctx.fillStyle = "rgba(255,230,190,0.6)";
        ctx.fillRect(nx - 1.5, ny - 2, 1.6, 1.6);
        hx.fillStyle = grey(1);
        hx.beginPath();
        hx.arc(nx, ny, 3, 0, Math.PI * 2);
        hx.fill();
      }
    }
  }
}

/** Boards of mixed widths across the canvas (summing exactly to its width) and staggered lengths
 *  along it, wrapped both ways so the floor tiles. */
function boards(ctx, hx, S, base, rng, { widths, minLen, maxLen, nails, px }) {
  const cols = [];
  let acc = 0;
  while (acc < S) {
    const w = Math.round(px * widths[Math.floor(rng() * widths.length)]);
    cols.push(w);
    acc += w;
  }
  cols[cols.length - 1] -= acc - S;
  if (cols[cols.length - 1] < px * 0.1) {
    const last = cols.pop();
    cols[cols.length - 1] += last;
  }
  let x = 0;
  for (const w of cols) {
    const lens = [];
    let y = 0;
    while (y < S) {
      let len = px * (minLen + rng() * (maxLen - minLen));
      if (S - y - len < px * minLen * 0.6) len = S - y;
      lens.push(len);
      y += len;
    }
    const start = rng() * S;
    const seeds = lens.map(() => Math.floor(rng() * 1e9));
    for (const dy of [-S, 0]) {
      let yy = start + dy;
      lens.forEach((len, k) => {
        board(ctx, hx, base, x, yy, w, len, mulberry(seeds[k]), { nails, px });
        yy += len;
      });
    }
    x += w;
  }
}

/** Hand-laid honey floor: boards of three widths and staggered lengths, each with its own figure,
 *  nailed at the ends. 2048 px = 4 m (so it repeats half as often as before). */
export function plankTexture({ base = [176, 128, 80], seed = 7 } = {}) {
  const S = 2048, [c, ctx] = canvas(S, S), [hc, hx] = canvas(S, S);
  hx.fillStyle = grey(0);
  hx.fillRect(0, 0, S, S);
  boards(ctx, hx, S, base, mulberry(seed), { widths: [0.16, 0.2, 0.24, 0.2], minLen: 0.9, maxLen: 2.4, nails: true, px: 512 });
  return withRelief(toTexture(c, { aniso: 16 }), hc, { strength: 1.4, scale: 0.9 });
}

/** Hand-painted flagstones: rounded slabs with a lit top-left rim, a shaded bottom-right rim,
 *  flat tones and the odd crack, in deep grout; each slab domes a little. 1024 px = 2 m. */
export function flagstoneTexture({ base = [140, 118, 96], seed = 5 } = {}) {
  const S = 1024, [c, ctx] = canvas(S, S), [hc, hx] = canvas(S, S);
  ctx.fillStyle = shade(base, 0.32);
  ctx.fillRect(0, 0, S, S);
  hx.fillStyle = grey(0.05);
  hx.fillRect(0, 0, S, S);
  const N = 5, cell = S / N;
  const draw = (g, height) => tiled(g, S, S, () => {
    const rng = mulberry(seed);
    for (let j = 0; j < N; j++) {
      for (let i = 0; i < N; i++) {
        const w = cell * (0.86 + rng() * 0.08), h = cell * (0.84 + rng() * 0.1);
        const cx = (i + 0.5) * cell + (rng() - 0.5) * 14 + (j % 2 ? cell * 0.25 : 0), cy = (j + 0.5) * cell + (rng() - 0.5) * 14;
        const tone = 0.84 + rng() * 0.28;
        const x = cx - w / 2, y = cy - h / 2, r = 26 + rng() * 14;
        const cracked = rng() < 0.45;
        const cr = [rng(), rng(), ...Array.from({ length: 8 }, () => rng())];
        g.save();
        g.beginPath();
        g.roundRect(x, y, w, h, r);
        if (height) {
          const grd = g.createRadialGradient(cx - w * 0.1, cy - h * 0.1, 8, cx, cy, Math.max(w, h) * 0.62);
          grd.addColorStop(0, grey(0.98));
          grd.addColorStop(0.7, grey(0.8));
          grd.addColorStop(1, grey(0.42));
          g.fillStyle = grd;
          g.fill();
        } else {
          g.fillStyle = shade(base, tone);
          g.fill();
        }
        g.clip();
        if (!height) {
          g.strokeStyle = shade(base, tone * 1.28);
          g.lineWidth = 12;
          g.beginPath();
          g.roundRect(x - 6, y - 6, w, h, r);
          g.stroke();
          g.strokeStyle = shade(base, tone * 0.62);
          g.lineWidth = 14;
          g.beginPath();
          g.roundRect(x + 7, y + 7, w, h, r);
          g.stroke();
        }
        if (cracked) {
          g.strokeStyle = height ? grey(0.35) : shade(base, tone * 0.5);
          g.lineWidth = height ? 4 : 3;
          g.beginPath();
          let px = x + w * (0.2 + cr[0] * 0.3), py = y + h * (0.2 + cr[1] * 0.2);
          g.moveTo(px, py);
          for (let k = 0; k < 4; k++) {
            px += w * (0.08 + cr[2 + k * 2] * 0.1);
            py += h * (0.06 + cr[3 + k * 2] * 0.12) * (cr[2 + k] < 0.5 ? 1 : 0.4);
            g.lineTo(px, py);
          }
          g.stroke();
        }
        g.restore();
      }
    }
  });
  draw(ctx, false);
  draw(hx, true);
  return withRelief(toTexture(c, { aniso: 16 }), hc, { strength: 2.2, scale: 1.2 });
}

/** Cut stone blocks (the fireplace); 512 px = 2 m. */
export function brickTexture() {
  const S = 512, [c, ctx] = canvas(S, S);
  ctx.fillStyle = "#62594d";
  ctx.fillRect(0, 0, S, S);
  tiled(ctx, S, S, () => {
    const rng = mulberry(61);
    const rows = 9, rh = S / rows;
    for (let r = 0; r < rows; r++) {
      const n = 4 + Math.floor(rng() * 2);
      const cuts = Array.from({ length: n }, () => 0.7 + rng() * 0.6);
      const sum = cuts.reduce((a, b) => a + b, 0);
      let x = rng() * S;
      for (let k = 0; k < n; k++) {
        const w = (cuts[k] / sum) * S;
        const tone = 0.86 + rng() * 0.26;
        ctx.fillStyle = shade([176, 168, 152], tone);
        ctx.beginPath();
        ctx.roundRect(x + 2, r * rh + 2, w - 4, rh - 4, 5);
        ctx.fill();
        ctx.fillStyle = "rgba(255,250,236,0.2)";
        ctx.fillRect(x + 5, r * rh + 3, w - 10, 3);
        ctx.fillStyle = "rgba(40,30,20,0.16)";
        ctx.fillRect(x + 5, (r + 1) * rh - 7, w - 10, 3);
        if (rng() < 0.35) {
          ctx.fillStyle = "rgba(50,40,30,0.22)";
          ctx.beginPath();
          ctx.ellipse(x + w * rng(), r * rh + rh * 0.6, 4 + rng() * 5, 2 + rng() * 3, 0, 0, Math.PI * 2);
          ctx.fill();
        }
        x += w;
      }
    }
  });
  return withRelief(toTexture(c), heightFromColor(c, 1.5), { strength: 3, scale: 1 });
}

// --------------------------------------------------------------------- wood
/** Joinery boards (grain along v) for panels, beams and furniture: mixed widths, long lengths, the
 *  same three figures as the floor, no nails. 1024 px = 2 m. */
export function woodTexture({ base = [78, 46, 26], boards: count = 6, seed = 3 } = {}) {
  const S = 1024, [c, ctx] = canvas(S, S), [hc, hx] = canvas(S, S);
  hx.fillStyle = grey(0);
  hx.fillRect(0, 0, S, S);
  const w0 = 1 / count; // metres per board at 1 m, as before
  boards(ctx, hx, S, base, mulberry(seed), { widths: [w0 * 0.8, w0, w0 * 1.25, w0], minLen: 1.2, maxLen: 2.0, nails: false, px: 512 });
  return withRelief(toTexture(c, { aniso: 16 }), hc, { strength: 1.2, scale: 0.7 });
}

// -------------------------------------------------------------------- fabric
/** Velvet: a flat pile with a soft diagonal sheen and a faint diamond of tufting. */
export function velvetTexture(base = [138, 28, 44], seed = 12) {
  const S = 256, [c, ctx] = canvas(S, S);
  const rng = mulberry(seed);
  ctx.fillStyle = shade(base, 1);
  ctx.fillRect(0, 0, S, S);
  const g = ctx.createLinearGradient(0, 0, S, S);
  g.addColorStop(0, "rgba(255,210,210,0.1)");
  g.addColorStop(0.5, "rgba(0,0,0,0)");
  g.addColorStop(1, "rgba(0,0,0,0.12)");
  ctx.fillStyle = g;
  ctx.fillRect(0, 0, S, S);
  ctx.strokeStyle = shade(base, 0.78);
  ctx.globalAlpha = 0.35;
  ctx.lineWidth = 2;
  tiled(ctx, S, S, () => {
    for (const [a, b] of [[0, 0], [S / 2, S / 2]]) {
      ctx.beginPath();
      ctx.moveTo(a, b - S / 2);
      ctx.lineTo(a + S / 2, b);
      ctx.lineTo(a, b + S / 2);
      ctx.lineTo(a - S / 2, b);
      ctx.closePath();
      ctx.stroke();
    }
  });
  ctx.globalAlpha = 1;
  specks(ctx, S, S, rng, 260, ["#000000", "#ffd0d0"], 2, [0.04, 0.08]);
  return withRelief(toTexture(c, { aniso: 16 }), heightFromColor(c, 2), { strength: 3, scale: 0.45 });
}

/** A rug painted for its own rectangle: gold border, inner line, lozenge field. */
export function carpetTexture({ field = "#7b1424", field2 = "#93192c", dark = "#520c18", gold = "#d9a94a", w = 512, h = 512, motif = "lozenge" } = {}) {
  const F = Math.round(h * 0.035);
  const [c, ctx] = canvas(w, h);
  // Knotted fringe at both short ends; the rug itself is painted inside it.
  const frng = mulberry(29);
  for (const top of [true, false]) {
    for (let x = 3; x < w; x += 7) {
      const len = F * (0.7 + frng() * 0.3);
      ctx.fillStyle = shade([232, 206, 150], 0.85 + frng() * 0.2);
      ctx.fillRect(x, top ? F - len : h - F, 3, len);
    }
  }
  ctx.save();
  ctx.translate(0, F);
  ctx.scale(1, (h - 2 * F) / h);
  ctx.fillStyle = field;
  ctx.fillRect(0, 0, w, h);
  const rng = mulberry(19);
  // Field: a tonal lozenge lattice.
  ctx.save();
  ctx.beginPath();
  ctx.rect(46, 46, w - 92, h - 92);
  ctx.clip();
  const step = Math.min(w, h) / 3.2;
  for (let y = 46 - step; y < h + step; y += step) {
    for (let x = 46 - step; x < w + step; x += step) {
      ctx.strokeStyle = field2;
      ctx.lineWidth = 7;
      ctx.beginPath();
      ctx.moveTo(x + step / 2, y);
      ctx.lineTo(x + step, y + step / 2);
      ctx.lineTo(x + step / 2, y + step);
      ctx.lineTo(x, y + step / 2);
      ctx.closePath();
      ctx.stroke();
      ctx.fillStyle = field2;
      if (motif === "lozenge") {
        ctx.beginPath();
        ctx.arc(x + step / 2, y + step / 2, step * 0.09, 0, Math.PI * 2);
        ctx.fill();
      }
      ctx.strokeStyle = "rgba(217,169,74,0.35)";
      ctx.lineWidth = 2.5;
      ctx.beginPath();
      ctx.moveTo(x + step / 2, y + step * 0.2);
      ctx.lineTo(x + step * 0.8, y + step / 2);
      ctx.lineTo(x + step / 2, y + step * 0.8);
      ctx.lineTo(x + step * 0.2, y + step / 2);
      ctx.closePath();
      ctx.stroke();
    }
  }
  ctx.restore();
  // Border bands.
  ctx.strokeStyle = dark;
  ctx.lineWidth = 26;
  ctx.strokeRect(24, 24, w - 48, h - 48);
  ctx.strokeStyle = gold;
  ctx.lineWidth = 7;
  ctx.strokeRect(11, 11, w - 22, h - 22);
  ctx.lineWidth = 3.5;
  ctx.strokeRect(38, 38, w - 76, h - 76);
  ctx.lineWidth = 2;
  ctx.strokeRect(46, 46, w - 92, h - 92);
  // Corner rosettes.
  for (const [x, y] of [[30, 30], [w - 30, 30], [30, h - 30], [w - 30, h - 30]]) {
    ctx.fillStyle = gold;
    ctx.beginPath();
    ctx.arc(x, y, 9, 0, Math.PI * 2);
    ctx.fill();
    ctx.fillStyle = dark;
    ctx.beginPath();
    ctx.arc(x, y, 4, 0, Math.PI * 2);
    ctx.fill();
  }
  // Small ornaments along the long border.
  ctx.fillStyle = gold;
  const long = Math.max(w, h);
  for (let t = 90; t < long - 90; t += 70) {
    for (const [x, y] of w >= h ? [[t, 25], [t, h - 25]] : [[25, t], [w - 25, t]]) {
      ctx.beginPath();
      ctx.moveTo(x, y - 5);
      ctx.lineTo(x + 5, y);
      ctx.lineTo(x, y + 5);
      ctx.lineTo(x - 5, y);
      ctx.closePath();
      ctx.fill();
    }
  }
  for (let y = 0; y < h; y += 4) {
    ctx.fillStyle = "rgba(0,0,0,0.045)";
    ctx.fillRect(0, y, w, 1.5);
  }
  ctx.restore();
  return withRelief(toTexture(c, { wrap: false }), heightFromColor(c, 1.2), { strength: 2.5, scale: 0.4, wrap: false });
}

/**
 * Curtain cloth for a drape's own UVs: u repeats every 0.6 m across, v runs hem (0) .. heading (1).
 * The folds come from the geometry and the toon light, so the cloth itself is flat: a woven ground,
 * a tone-on-tone damask, a weighted hem with a gold thread and a gathered heading tape.
 */
export function curtainTexture(dark = [70, 8, 20], light = [190, 42, 58]) {
  const W = 256, H = 1024, [c, ctx] = canvas(W, H);
  const mid = dark.map((d, i) => Math.round(d + (light[i] - d) * 0.55));
  const deep = dark.map((d, i) => Math.round(d + (light[i] - d) * 0.25));
  const hi = light.map((l) => Math.min(255, Math.round(l * 1.08)));
  ctx.fillStyle = `rgb(${mid})`;
  ctx.fillRect(0, 0, W, H);
  // Woven ground: fine warp lines.
  const rng = mulberry(dark[0] + light[2]);
  for (let x = 0; x < W; x += 3) {
    ctx.fillStyle = `rgba(0,0,0,${0.03 + rng() * 0.04})`;
    ctx.fillRect(x, 0, 1, H);
  }
  // Damask: a lozenge with a four-petal flower, tone on tone, in staggered rows.
  const motif = (cx, cy, s) => {
    ctx.fillStyle = `rgba(${hi},0.22)`;
    ctx.beginPath();
    ctx.moveTo(cx, cy - s);
    ctx.quadraticCurveTo(cx + s * 0.35, cy - s * 0.35, cx + s * 0.62, cy);
    ctx.quadraticCurveTo(cx + s * 0.35, cy + s * 0.35, cx, cy + s);
    ctx.quadraticCurveTo(cx - s * 0.35, cy + s * 0.35, cx - s * 0.62, cy);
    ctx.quadraticCurveTo(cx - s * 0.35, cy - s * 0.35, cx, cy - s);
    ctx.fill();
    ctx.fillStyle = `rgba(${deep},0.3)`;
    for (const [dx, dy] of [[0, -0.32], [0.22, 0], [0, 0.32], [-0.22, 0]]) {
      ctx.beginPath();
      ctx.ellipse(cx + dx * s, cy + dy * s, s * 0.13, s * 0.19, 0, 0, Math.PI * 2);
      ctx.fill();
    }
  };
  for (let row = 0; row < 12; row++) {
    const y = 110 + row * 70;
    for (const x of row % 2 ? [0, W / 2, W] : [W / 4, (W * 3) / 4]) motif(x, H - y, 30);
  }
  // Heading tape (top) and weighted hem (bottom).
  ctx.fillStyle = `rgb(${deep})`;
  ctx.fillRect(0, 0, W, 34);
  ctx.fillStyle = "rgba(0,0,0,0.25)";
  for (let x = 16; x < W; x += 32) ctx.fillRect(x, 6, 3, 22);
  ctx.fillStyle = `rgb(${deep})`;
  ctx.fillRect(0, H - 70, W, 70);
  ctx.fillStyle = "#d9a94a";
  ctx.fillRect(0, H - 78, W, 5);
  ctx.fillRect(0, H - 22, W, 3);
  const t = toTexture(c, { wrap: true });
  t.wrapT = THREE.ClampToEdgeWrapping;
  return withRelief(t, heightFromColor(c, 1), { strength: 2, scale: 0.35 });
}

/** The long runner along the way in: borders along both edges, a lozenge per 1.2 m tile (v repeats). */
export function runnerTexture({ field = "#7b1424", field2 = "#93192c", dark = "#4a0a14", gold = "#d9a94a" } = {}) {
  const W = 512, H = 512, [c, ctx] = canvas(W, H);
  ctx.fillStyle = field;
  ctx.fillRect(0, 0, W, H);
  // Pile: faint cross rows.
  for (let y = 0; y < H; y += 4) {
    ctx.fillStyle = "rgba(0,0,0,0.05)";
    ctx.fillRect(0, y, W, 1.5);
  }
  // A lozenge medallion per tile, and half medallions at the seams so it tiles.
  const lozenge = (cy) => {
    ctx.fillStyle = field2;
    ctx.beginPath();
    ctx.moveTo(W / 2, cy - 170);
    ctx.lineTo(W / 2 + 130, cy);
    ctx.lineTo(W / 2, cy + 170);
    ctx.lineTo(W / 2 - 130, cy);
    ctx.closePath();
    ctx.fill();
    ctx.strokeStyle = gold;
    ctx.lineWidth = 5;
    ctx.stroke();
    ctx.fillStyle = dark;
    ctx.beginPath();
    ctx.moveTo(W / 2, cy - 70);
    ctx.lineTo(W / 2 + 54, cy);
    ctx.lineTo(W / 2, cy + 70);
    ctx.lineTo(W / 2 - 54, cy);
    ctx.closePath();
    ctx.fill();
    ctx.fillStyle = gold;
    ctx.beginPath();
    ctx.arc(W / 2, cy, 13, 0, Math.PI * 2);
    ctx.fill();
  };
  lozenge(H / 2);
  for (const cy of [0, H]) {
    ctx.fillStyle = gold;
    for (const dx of [-60, 60]) {
      ctx.beginPath();
      ctx.arc(W / 2 + dx, cy, 8, 0, Math.PI * 2);
      ctx.fill();
    }
  }
  // Borders along both long edges: dark band, gold lines, a running chain.
  for (const side of [0, 1]) {
    const x0 = side ? W - 70 : 0;
    ctx.fillStyle = dark;
    ctx.fillRect(x0, 0, 70, H);
    ctx.fillStyle = gold;
    ctx.fillRect(side ? W - 66 : 60, 0, 6, H);
    ctx.fillRect(side ? W - 16 : 10, 0, 6, H);
    for (let y = 16; y < H; y += 32) {
      ctx.beginPath();
      const cx = side ? W - 38 : 38;
      ctx.moveTo(cx, y - 9);
      ctx.lineTo(cx + 8, y);
      ctx.lineTo(cx, y + 9);
      ctx.lineTo(cx - 8, y);
      ctx.closePath();
      ctx.fill();
    }
  }
  const t = toTexture(c, { wrap: true });
  t.wrapS = THREE.ClampToEdgeWrapping;
  return withRelief(t, heightFromColor(c, 1.2), { strength: 2.5, scale: 0.4 });
}

/** Knotted wool fringe for the ends of a runner: strands with alpha, knots at the top. */
export function fringeTexture(color = [226, 196, 140]) {
  const W = 256, H = 64, [c, ctx] = canvas(W, H);
  const rng = mulberry(9);
  for (let x = 2; x < W; x += 6) {
    const len = H * (0.72 + rng() * 0.26), tone = 0.85 + rng() * 0.25;
    ctx.fillStyle = shade(color, tone);
    ctx.fillRect(x, 0, 3, len);
    ctx.fillStyle = shade(color, tone * 0.7);
    ctx.fillRect(x + 2, 0, 1, len);
  }
  ctx.fillStyle = shade(color, 0.75);
  for (let x = 0; x < W; x += 24) ctx.fillRect(x, 6, 18, 6);
  const t = toTexture(c, { wrap: true });
  t.wrapT = THREE.ClampToEdgeWrapping;
  return t;
}

/** A plain woven cotton (sheets, pillows): a fine basket weave. */
export function weaveTexture(base = [246, 240, 228]) {
  const S = 128, [c, ctx] = canvas(S, S);
  ctx.fillStyle = shade(base, 1);
  ctx.fillRect(0, 0, S, S);
  for (let y = 0; y < S; y += 4) {
    for (let x = 0; x < S; x += 4) {
      const on = ((x + y) / 4) % 2 === 0;
      ctx.fillStyle = on ? "rgba(0,0,0,0.05)" : "rgba(255,255,255,0.06)";
      ctx.fillRect(x, y, on ? 4 : 2, on ? 2 : 4);
    }
  }
  return withRelief(toTexture(c, { repeat: [6, 6] }), heightFromColor(c, 0.6), { strength: 4, scale: 0.3 });
}

/** Crimson banner with a gold heart outline and swallow-tail (alpha cut). */
export function bannerTexture() {
  const W = 256, H = 448, [c, ctx] = canvas(W, H);
  const path = () => {
    ctx.beginPath();
    ctx.moveTo(6, 0);
    ctx.lineTo(W - 6, 0);
    ctx.lineTo(W - 6, H - 62);
    ctx.lineTo(W / 2, H - 6);
    ctx.lineTo(6, H - 62);
    ctx.closePath();
  };
  path();
  const g = ctx.createLinearGradient(0, 0, 0, H);
  g.addColorStop(0, "#a02034");
  g.addColorStop(1, "#7a1424");
  ctx.fillStyle = g;
  ctx.fill();
  ctx.save();
  path();
  ctx.clip();
  const rng = mulberry(31);
  specks(ctx, W, H, rng, 1200, ["#3a0508", "#ffb0a0"], 2, [0.04, 0.1]);
  ctx.restore();
  ctx.strokeStyle = "#e0b552";
  ctx.lineWidth = 11;
  ctx.save();
  ctx.translate(0, 0);
  path();
  ctx.stroke();
  ctx.restore();
  // Heart outline.
  const cx = W / 2, cy = 190, s = 74;
  ctx.beginPath();
  ctx.moveTo(cx, cy + s * 0.75);
  ctx.bezierCurveTo(cx - s * 1.25, cy - s * 0.05, cx - s * 0.6, cy - s * 0.95, cx, cy - s * 0.38);
  ctx.bezierCurveTo(cx + s * 0.6, cy - s * 0.95, cx + s * 1.25, cy - s * 0.05, cx, cy + s * 0.75);
  ctx.closePath();
  ctx.fillStyle = "rgba(200,60,70,0.55)";
  ctx.fill();
  ctx.strokeStyle = "#e6bd5a";
  ctx.lineWidth = 10;
  ctx.stroke();
  ctx.strokeStyle = "rgba(230,189,90,0.55)";
  ctx.lineWidth = 3;
  ctx.save();
  ctx.translate(cx, cy);
  ctx.scale(0.72, 0.72);
  ctx.translate(-cx, -cy);
  ctx.stroke();
  ctx.restore();
  return toTexture(c, { wrap: false });
}

// ------------------------------------------------------------------ pictures
export function paintingTexture(kind = "landscape") {
  const W = 320, H = 220, [c, ctx] = canvas(W, H);
  const rng = mulberry(kind.length * 17);
  if (kind === "landscape") {
    const sky = ctx.createLinearGradient(0, 0, 0, H);
    sky.addColorStop(0, "#efc287");
    sky.addColorStop(0.5, "#f7e3b4");
    sky.addColorStop(1, "#a9c1a3");
    ctx.fillStyle = sky;
    ctx.fillRect(0, 0, W, H);
    ctx.fillStyle = "rgba(230,120,50,0.9)";
    ctx.beginPath();
    ctx.arc(232, 86, 20, 0, Math.PI * 2);
    ctx.fill();
    for (const [x, w, h, col] of [[40, 150, 84, "#7a8f9e"], [130, 170, 116, "#56778a"], [230, 150, 72, "#5f8574"]]) {
      ctx.fillStyle = col;
      ctx.beginPath();
      ctx.moveTo(x - w / 2, 140);
      ctx.lineTo(x - w * 0.1, 140 - h);
      ctx.lineTo(x + w * 0.05, 140 - h * 0.86);
      ctx.lineTo(x + w / 2, 140);
      ctx.fill();
    }
    ctx.fillStyle = "#4c7d5c";
    ctx.fillRect(0, 138, W, 82);
    ctx.fillStyle = "rgba(255,236,190,0.45)";
    for (let i = 0; i < 14; i++) ctx.fillRect(150 + rng() * 120, 146 + rng() * 44, 24 + rng() * 40, 3);
    ctx.fillStyle = "#2f5a3e";
    for (const [x, h] of [[24, 60], [52, 44], [292, 66], [268, 46]]) {
      ctx.beginPath();
      ctx.moveTo(x - 20, 172);
      ctx.lineTo(x, 172 - h);
      ctx.lineTo(x + 20, 172);
      ctx.fill();
    }
  } else if (kind === "roses") {
    const bg = ctx.createRadialGradient(W / 2, 90, 10, W / 2, 110, 210);
    bg.addColorStop(0, "#6a3a30");
    bg.addColorStop(1, "#22100e");
    ctx.fillStyle = bg;
    ctx.fillRect(0, 0, W, H);
    ctx.fillStyle = "#5a3418";
    ctx.fillRect(0, 168, W, 52);
    ctx.fillStyle = "#2e5a86";
    ctx.beginPath();
    ctx.moveTo(130, 168);
    ctx.quadraticCurveTo(112, 128, 138, 100);
    ctx.lineTo(182, 100);
    ctx.quadraticCurveTo(208, 128, 190, 168);
    ctx.fill();
    ctx.fillStyle = "rgba(255,255,255,0.18)";
    ctx.fillRect(142, 108, 8, 54);
    for (let i = 0; i < 9; i++) {
      const a = (i / 9) * Math.PI * 2, r = 26 + rng() * 22;
      const x = 160 + Math.cos(a) * r * 1.25, y = 82 + Math.sin(a) * r * 0.75;
      const g = ctx.createRadialGradient(x - 4, y - 4, 2, x, y, 20);
      g.addColorStop(0, "#f06070");
      g.addColorStop(1, "#7a1626");
      ctx.fillStyle = g;
      ctx.beginPath();
      ctx.arc(x, y, 16 + rng() * 6, 0, Math.PI * 2);
      ctx.fill();
    }
    ctx.fillStyle = "#3c6a3a";
    for (let i = 0; i < 6; i++) ctx.fillRect(120 + i * 16, 96 + (i % 2) * 8, 7, 24);
  } else {
    const sky = ctx.createLinearGradient(0, 0, 0, H);
    sky.addColorStop(0, "#141c3a");
    sky.addColorStop(0.62, "#2a3a6a");
    sky.addColorStop(1, "#0e1630");
    ctx.fillStyle = sky;
    ctx.fillRect(0, 0, W, H);
    const moon = ctx.createRadialGradient(220, 66, 4, 220, 66, 60);
    moon.addColorStop(0, "rgba(255,250,220,1)");
    moon.addColorStop(0.25, "rgba(255,244,200,0.9)");
    moon.addColorStop(1, "rgba(255,244,200,0)");
    ctx.fillStyle = moon;
    ctx.fillRect(150, 0, 140, 140);
    ctx.fillStyle = "#0a1226";
    ctx.fillRect(0, 130, W, 90);
    ctx.strokeStyle = "rgba(230,230,255,0.4)";
    ctx.lineWidth = 2;
    for (let i = 0; i < 26; i++) {
      const y = 134 + rng() * 76, x = 40 + rng() * 240, l = 10 + rng() * 34;
      ctx.beginPath();
      ctx.moveTo(x, y);
      ctx.lineTo(x + l, y);
      ctx.stroke();
    }
    ctx.fillStyle = "rgba(255,244,200,0.5)";
    for (let i = 0; i < 8; i++) ctx.fillRect(206 + (rng() - 0.5) * 30, 134 + i * 9, 20 + rng() * 14, 2.5);
  }
  ctx.strokeStyle = "rgba(0,0,0,0.25)";
  ctx.lineWidth = 6;
  ctx.strokeRect(0, 0, W, H);
  return toTexture(c, { wrap: false });
}

export function doorTexture() {
  const W = 256, H = 512, [c, ctx] = canvas(W, H);
  const rng = mulberry(88);
  ctx.fillStyle = "#4e2c16";
  ctx.fillRect(0, 0, W, H);
  for (let i = 0; i < 40; i++) {
    ctx.strokeStyle = `rgba(20,8,2,${0.1 + rng() * 0.18})`;
    ctx.lineWidth = 0.8 + rng() * 1.6;
    const x = rng() * W;
    ctx.beginPath();
    ctx.moveTo(x, 0);
    ctx.bezierCurveTo(x + (rng() - 0.5) * 8, H * 0.3, x + (rng() - 0.5) * 8, H * 0.7, x, H);
    ctx.stroke();
  }
  ctx.strokeStyle = "#2a1408";
  ctx.lineWidth = 12;
  ctx.strokeRect(10, 10, W - 20, H - 20);
  const panel = (x, y, w, h) => {
    ctx.fillStyle = "rgba(20,8,2,0.28)";
    ctx.fillRect(x, y, w, h);
    ctx.strokeStyle = "#2a1408";
    ctx.lineWidth = 7;
    ctx.strokeRect(x, y, w, h);
    ctx.strokeStyle = "rgba(255,190,120,0.16)";
    ctx.lineWidth = 2;
    ctx.strokeRect(x + 6, y + 6, w - 12, h - 12);
  };
  panel(36, 34, W - 72, 200);
  panel(36, 262, W - 72, 210);
  ctx.fillStyle = "rgba(224,184,80,0.85)";
  for (const [x, y] of [[26, 26], [W - 26, 26], [26, H - 26], [W - 26, H - 26], [26, 248], [W - 26, 248]]) {
    ctx.beginPath();
    ctx.arc(x, y, 5, 0, Math.PI * 2);
    ctx.fill();
  }
  return toTexture(c, { wrap: false });
}

/** A dark slate notice board. */
export function boardTexture() {
  const S = 256, [c, ctx] = canvas(S, S);
  const rng = mulberry(120);
  const g = ctx.createRadialGradient(S / 2, S / 2, 20, S / 2, S / 2, S * 0.75);
  g.addColorStop(0, "#4b5655");
  g.addColorStop(1, "#2c3534");
  ctx.fillStyle = g;
  ctx.fillRect(0, 0, S, S);
  specks(ctx, S, S, rng, 1400, ["#111a19", "#8a9a98"], 3, [0.05, 0.14]);
  return toTexture(c, { wrap: false });
}

export function paperTexture(seed = 1) {
  const W = 128, H = 160, [c, ctx] = canvas(W, H);
  const rng = mulberry(seed * 13);
  ctx.fillStyle = "#f0e2bc";
  ctx.fillRect(0, 0, W, H);
  specks(ctx, W, H, rng, 300, ["#b89a60", "#fff6dc"], 2, [0.05, 0.12]);
  ctx.strokeStyle = "rgba(70,46,24,0.6)";
  ctx.lineWidth = 2;
  for (let l = 0; l < 8; l++) {
    ctx.beginPath();
    ctx.moveTo(14, 34 + l * 15);
    ctx.lineTo(W - 14 - rng() * 34, 34 + l * 15);
    ctx.stroke();
  }
  ctx.fillStyle = "rgba(70,46,24,0.75)";
  ctx.fillRect(14, 14, 50 + rng() * 30, 8);
  return toTexture(c, { wrap: false });
}

/** A painted sign: a plate with a border, a heart and text. */
export function signTexture(text, { w = 512, h = 160, bg = "#3a2010", fg = "#f0d08a", heart = true, fontFamily = "Georgia, serif" } = {}) {
  const [c, ctx] = canvas(w, h);
  ctx.fillStyle = bg;
  ctx.fillRect(0, 0, w, h);
  const rng = mulberry(text.length * 7);
  specks(ctx, w, h, rng, 700, ["#000000", "#7a5030"], 2.4, [0.05, 0.14]);
  ctx.strokeStyle = fg;
  ctx.lineWidth = Math.max(4, h * 0.035);
  ctx.strokeRect(h * 0.06, h * 0.06, w - h * 0.12, h * 0.88);
  ctx.lineWidth = Math.max(2, h * 0.016);
  ctx.strokeRect(h * 0.12, h * 0.12, w - h * 0.24, h * 0.76);
  const heartSpace = heart ? h * 0.7 : 0;
  const maxW = w - h * 0.4 - heartSpace;
  let size = h * 0.5;
  ctx.textAlign = "center";
  ctx.textBaseline = "middle";
  for (; size > 12; size -= 2) {
    ctx.font = `700 ${size}px ${fontFamily}`;
    if (ctx.measureText(text).width <= maxW) break;
  }
  ctx.fillStyle = fg;
  ctx.fillText(text, heartSpace + (w - heartSpace) / 2, h / 2 + size * 0.04);
  if (heart) {
    const cx = h * 0.55, cy = h / 2 + 2, s2 = h * 0.24;
    ctx.beginPath();
    ctx.moveTo(cx, cy + s2 * 0.75);
    ctx.bezierCurveTo(cx - s2 * 1.25, cy - s2 * 0.05, cx - s2 * 0.6, cy - s2 * 0.95, cx, cy - s2 * 0.38);
    ctx.bezierCurveTo(cx + s2 * 0.6, cy - s2 * 0.95, cx + s2 * 1.25, cy - s2 * 0.05, cx, cy + s2 * 0.75);
    ctx.closePath();
    ctx.fillStyle = "#b32c3c";
    ctx.fill();
    ctx.strokeStyle = fg;
    ctx.lineWidth = Math.max(3, h * 0.03);
    ctx.stroke();
  }
  return toTexture(c, { wrap: false });
}

/** A leaf: pointed ellipse, mid-rib, side veins, paler edge. Alpha-cut. */
export function leafTexture({ dark = "#2a6a36", mid = "#43974a", light = "#93cc6a", seed = 4 } = {}) {
  const W = 128, H = 256, [c, ctx] = canvas(W, H);
  const rng = mulberry(seed);
  ctx.beginPath();
  ctx.moveTo(W / 2, 2);
  ctx.bezierCurveTo(W * 1.05, H * 0.28, W * 0.98, H * 0.78, W / 2, H - 2);
  ctx.bezierCurveTo(W * 0.02, H * 0.78, -W * 0.05, H * 0.28, W / 2, 2);
  ctx.closePath();
  const g = ctx.createLinearGradient(0, 0, W, 0);
  g.addColorStop(0, dark);
  g.addColorStop(0.5, mid);
  g.addColorStop(1, dark);
  ctx.fillStyle = g;
  ctx.fill();
  ctx.save();
  ctx.clip();
  const sheen = ctx.createLinearGradient(0, 0, W, H * 0.4);
  sheen.addColorStop(0, "rgba(255,255,220,0.0)");
  sheen.addColorStop(0.5, "rgba(190,240,150,0.22)");
  sheen.addColorStop(1, "rgba(255,255,220,0.0)");
  ctx.fillStyle = sheen;
  ctx.fillRect(0, 0, W, H);
  ctx.strokeStyle = light;
  ctx.globalAlpha = 0.55;
  ctx.lineWidth = 4;
  ctx.beginPath();
  ctx.moveTo(W / 2, 4);
  ctx.lineTo(W / 2, H - 4);
  ctx.stroke();
  ctx.lineWidth = 1.6;
  for (let i = 0; i < 9; i++) {
    const y = 40 + i * 22;
    ctx.beginPath();
    ctx.moveTo(W / 2, y);
    ctx.quadraticCurveTo(W * 0.72, y - 6, W * 0.94, y - 30 - rng() * 4);
    ctx.moveTo(W / 2, y);
    ctx.quadraticCurveTo(W * 0.28, y - 6, W * 0.06, y - 30 - rng() * 4);
    ctx.stroke();
  }
  ctx.restore();
  return toTexture(c, { wrap: false });
}

/** Dark slate shingles for the roofs over the closed rooms; 512 px = 2 m. */
export function roofTexture() {
  const S = 512, [c, ctx] = canvas(S, S);
  ctx.fillStyle = "#1a1f2c";
  ctx.fillRect(0, 0, S, S);
  tiled(ctx, S, S, () => {
    const rng = mulberry(77);
    const rows = 10, rh = S / rows;
    for (let r = 0; r < rows; r++) {
      const cols = 8, cw = S / cols;
      for (let k = -1; k <= cols; k++) {
        const x = k * cw + (r % 2 ? cw / 2 : 0), y = r * rh;
        const tone = 0.8 + rng() * 0.4;
        const g = ctx.createLinearGradient(0, y, 0, y + rh);
        g.addColorStop(0, shade([58, 70, 96], tone));
        g.addColorStop(1, shade([32, 40, 60], tone));
        ctx.fillStyle = g;
        ctx.beginPath();
        ctx.moveTo(x + 1.5, y);
        ctx.lineTo(x + cw - 1.5, y);
        ctx.lineTo(x + cw - 1.5, y + rh * 0.62);
        ctx.quadraticCurveTo(x + cw / 2, y + rh * 1.16, x + 1.5, y + rh * 0.62);
        ctx.closePath();
        ctx.fill();
        ctx.strokeStyle = "rgba(6,8,16,0.7)";
        ctx.lineWidth = 1.6;
        ctx.stroke();
        ctx.fillStyle = "rgba(190,205,240,0.16)";
        ctx.fillRect(x + 4, y + 1, cw - 8, 2);
      }
    }
  });
  const rng = mulberry(78);
  specks(ctx, S, S, rng, 1400, ["#0a0d16", "#8a96b8"], 2, [0.04, 0.1]);
  return withRelief(toTexture(c), heightFromColor(c, 1.5), { strength: 3, scale: 0.9 });
}

/** A mirror: cool grey-blue glass with a diagonal streak of light. */
export function mirrorTexture() {
  const W = 128, H = 192, [c, ctx] = canvas(W, H);
  const g = ctx.createLinearGradient(0, 0, W, H);
  g.addColorStop(0, "#6f8494");
  g.addColorStop(0.5, "#4a5f70");
  g.addColorStop(1, "#33465a");
  ctx.fillStyle = g;
  ctx.fillRect(0, 0, W, H);
  ctx.fillStyle = "rgba(255,255,255,0.22)";
  ctx.beginPath();
  ctx.moveTo(W * 0.15, 0);
  ctx.lineTo(W * 0.45, 0);
  ctx.lineTo(W * 0.05, H);
  ctx.lineTo(W * -0.25, H);
  ctx.closePath();
  ctx.fill();
  ctx.fillStyle = "rgba(255,255,255,0.12)";
  ctx.fillRect(W * 0.5, 0, W * 0.07, H);
  return toTexture(c, { wrap: false });
}

/** A soft radial gradient (halos, light pools, contact shadows). */
export function radialTexture(stops = [[0, "rgba(255,255,255,1)"], [0.4, "rgba(255,255,255,0.32)"], [1, "rgba(255,255,255,0)"]], size = 128) {
  const [c, ctx] = canvas(size, size);
  const g = ctx.createRadialGradient(size / 2, size / 2, 0, size / 2, size / 2, size / 2);
  for (const [t, col] of stops) g.addColorStop(t, col);
  ctx.fillStyle = g;
  ctx.fillRect(0, 0, size, size);
  return toTexture(c, { wrap: false });
}

/** A blurred rounded rectangle: the soft footprint shadow under a piece of furniture. */
export function softBoxTexture(size = 128) {
  const [c, ctx] = canvas(size, size);
  ctx.filter = "blur(11px)";
  ctx.fillStyle = "#000";
  ctx.beginPath();
  ctx.roundRect(26, 26, size - 52, size - 52, 18);
  ctx.fill();
  return toTexture(c, { wrap: false });
}
