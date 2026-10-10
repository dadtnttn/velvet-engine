// Painted textures for the village, in the same hand as the house's (../textures.js): flat tones,
// a lit rim on the top-left of every stone and shingle, a shaded rim on the bottom-right, and a
// height painted alongside for the relief. Tiling ones are drawn at nine wrapped offsets.
import { canvas, toTexture, mulberry, tiled, shade, specks, blobs, grey } from "../textures.js";
import { withRelief, heightFromColor } from "../relief.js";

// --------------------------------------------------------------------- grass
/** A lawn painted in strokes: broad patches of light and shade, blades in three greens, clover
 *  and the odd daisy. 1024 px = 4 m. */
export function grassTexture({ seed = 3 } = {}) {
  const S = 1024, [c, ctx] = canvas(S, S);
  ctx.fillStyle = "#5c9a3c";
  ctx.fillRect(0, 0, S, S);
  blobs(ctx, S, S, seed, 26, "rgba(214,226,120,A)", "rgba(40,92,40,A)", 80, 220);
  blobs(ctx, S, S, seed + 1, 40, "rgba(150,196,80,A)", "rgba(52,110,46,A)", 30, 90);
  tiled(ctx, S, S, () => {
    const rng = mulberry(seed + 7);
    const greens = ["#3f7d33", "#4f8f3a", "#6aa846", "#86bd52", "#9ccc5e"];
    ctx.lineCap = "round";
    for (let i = 0; i < 5200; i++) {
      const x = rng() * S, y = rng() * S, h = 7 + rng() * 13, lean = (rng() - 0.5) * 7;
      ctx.strokeStyle = greens[Math.floor(rng() * greens.length)];
      ctx.globalAlpha = 0.35 + rng() * 0.45;
      ctx.lineWidth = 1.6 + rng() * 1.6;
      ctx.beginPath();
      ctx.moveTo(x, y);
      ctx.quadraticCurveTo(x + lean * 0.4, y - h * 0.6, x + lean, y - h);
      ctx.stroke();
    }
    ctx.globalAlpha = 1;
    // Clover patches and a few flowers.
    for (let i = 0; i < 90; i++) {
      const x = rng() * S, y = rng() * S;
      ctx.fillStyle = rng() < 0.5 ? "#4f9a3e" : "#5aa646";
      for (let k = 0; k < 3; k++) {
        const a = (k / 3) * Math.PI * 2 + rng();
        ctx.beginPath();
        ctx.arc(x + Math.cos(a) * 3.2, y + Math.sin(a) * 3.2, 3.4, 0, Math.PI * 2);
        ctx.fill();
      }
    }
    for (let i = 0; i < 70; i++) {
      const x = rng() * S, y = rng() * S;
      ctx.fillStyle = rng() < 0.6 ? "#f4f0dc" : "#f2d24a";
      ctx.beginPath();
      ctx.arc(x, y, 2.2 + rng() * 1.4, 0, Math.PI * 2);
      ctx.fill();
      ctx.fillStyle = "#e0a020";
      ctx.beginPath();
      ctx.arc(x, y, 0.9, 0, Math.PI * 2);
      ctx.fill();
    }
  });
  return withRelief(toTexture(c, { aniso: 16 }), heightFromColor(c, 1.2), { strength: 1.2, scale: 0.6 });
}

// ------------------------------------------------------------------- cobbles
/**
 * The stones of a cobble tile: `rows` rows of rounded stones, each row filled exactly (its stones
 * stretched a little to fit) so the tile wraps with no seam and no stone overlapping another.
 * Pixels of an S×S tile. Each stone owns a cell (its width × the row's height, joints included):
 * the paving's mask reads these cells to set whole stones along its edges.
 */
export function cobbleLayout({ seed = 11, rows = 9, S = 1024 } = {}) {
  const rng = mulberry(seed);
  const rh = S / rows;
  const stones = [];
  for (let r = 0; r < rows; r++) {
    const widths = [];
    let sum = 0;
    while (sum < S - rh * 0.45) {
      const w = rh * (0.75 + rng() * 0.6);
      widths.push(w);
      sum += w;
    }
    const fit = S / sum;
    let x = rng() * rh;
    for (const w0 of widths) {
      const w = w0 * fit, h = rh * (0.78 + rng() * 0.18);
      stones.push({
        row: r, cx: x + w / 2, cy: r * rh + rh / 2 + (rng() - 0.5) * rh * 0.12, w, h,
        rad: Math.min(w, h) * (0.32 + rng() * 0.12), tone: 0.82 + rng() * 0.3, warm: rng(),
        stain: rng() < 0.3 ? [rng() - 0.5, rng() - 0.5, rng(), rng(), rng()] : null,
      });
      x += w;
    }
  }
  return { S, rows, stones };
}

/** Rounded cobbles packed in loose rows, sandy and warm, deep in their joints; each stone domes in
 *  the height map (the same stones: one layout draws both). 1024 px = one tile. */
export function cobbleTexture({ base = [214, 186, 140], mortar = [122, 96, 64], seed = 11, rows = 9 } = {}) {
  const S = 1024, [c, ctx] = canvas(S, S), [hc, hx] = canvas(S, S);
  const layout = cobbleLayout({ seed, rows, S });
  ctx.fillStyle = shade(mortar, 1);
  ctx.fillRect(0, 0, S, S);
  hx.fillStyle = grey(0.05);
  hx.fillRect(0, 0, S, S);
  const draw = (g, height) => tiled(g, S, S, () => {
    for (const { cx, cy, w, h, rad, tone, warm, stain } of layout.stones) {
      const col = warm < 0.18 ? [base[0] * 0.92, base[1] * 0.9, base[2] * 0.98] : warm > 0.88 ? [base[0] * 1.02, base[1] * 0.92, base[2] * 0.8] : base;
      g.save();
      g.beginPath();
      g.roundRect(cx - w / 2 + 3, cy - h / 2 + 3, w - 6, h - 6, rad);
      if (height) {
        const grd = g.createRadialGradient(cx - w * 0.12, cy - h * 0.12, 4, cx, cy, Math.max(w, h) * 0.62);
        grd.addColorStop(0, grey(1));
        grd.addColorStop(0.65, grey(0.78));
        grd.addColorStop(1, grey(0.32));
        g.fillStyle = grd;
        g.fill();
      } else {
        g.fillStyle = shade(col, tone);
        g.fill();
        g.clip();
        g.strokeStyle = shade(col, tone * 1.22);
        g.lineWidth = 9;
        g.beginPath();
        g.roundRect(cx - w / 2 - 2, cy - h / 2 - 2, w - 4, h - 4, rad);
        g.stroke();
        g.strokeStyle = shade(col, tone * 0.66);
        g.lineWidth = 10;
        g.beginPath();
        g.roundRect(cx - w / 2 + 9, cy - h / 2 + 9, w - 4, h - 4, rad);
        g.stroke();
        if (stain) {
          g.fillStyle = "rgba(90,70,40,0.18)";
          g.beginPath();
          g.ellipse(cx + stain[0] * w * 0.4, cy + stain[1] * h * 0.3, 6 + stain[2] * 8, 3 + stain[3] * 4, stain[4] * 3, 0, Math.PI * 2);
          g.fill();
        }
      }
      g.restore();
    }
  });
  draw(ctx, false);
  draw(hx, true);
  const rng = mulberry(seed + 3);
  specks(ctx, S, S, rng, 1600, ["#5a4428", "#fff0d0", "#8a6a40"], 2.2, [0.05, 0.14]);
  // Moss and weeds in the joints here and there.
  tiled(ctx, S, S, () => {
    const r2 = mulberry(seed + 9);
    for (let i = 0; i < 60; i++) {
      ctx.fillStyle = r2() < 0.5 ? "rgba(96,140,60,0.55)" : "rgba(70,116,48,0.5)";
      ctx.beginPath();
      ctx.ellipse(r2() * S, r2() * S, 4 + r2() * 9, 2 + r2() * 4, r2() * 3, 0, Math.PI * 2);
      ctx.fill();
    }
  });
  const texture = withRelief(toTexture(c, { aniso: 16 }), hc, { strength: 2.4, scale: 1.1 });
  texture.userData.layout = layout;
  return texture;
}

// -------------------------------------------------------------------- stone
/** Rubble stone in rough courses (plinths, the well, the bridge, the tower): grey-beige, rounded,
 *  deep mortar. 1024 px = 2 m. */
export function stoneWallTexture({ base = [176, 166, 148], seed = 21 } = {}) {
  const S = 1024, [c, ctx] = canvas(S, S), [hc, hx] = canvas(S, S);
  ctx.fillStyle = "#6a6052";
  ctx.fillRect(0, 0, S, S);
  hx.fillStyle = grey(0.04);
  hx.fillRect(0, 0, S, S);
  const rows = 7, rh = S / rows;
  const draw = (g, height) => tiled(g, S, S, () => {
    const rng = mulberry(seed);
    for (let r = 0; r < rows; r++) {
      let x = rng() * rh;
      while (x < S + rh) {
        const w = rh * (0.9 + rng() * 0.9), h = rh * (0.82 + rng() * 0.16);
        const cx = x + w / 2, cy = r * rh + rh / 2;
        const tone = 0.8 + rng() * 0.32;
        const tint = rng() < 0.25 ? [base[0] * 1.06, base[1] * 0.98, base[2] * 0.86] : rng() < 0.2 ? [base[0] * 0.92, base[1] * 0.96, base[2] * 1.02] : base;
        const rad = 14 + rng() * 22;
        g.save();
        g.beginPath();
        g.roundRect(cx - w / 2 + 4, cy - h / 2 + 4, w - 8, h - 8, rad);
        if (height) {
          const grd = g.createRadialGradient(cx - w * 0.1, cy - h * 0.15, 6, cx, cy, Math.max(w, h) * 0.6);
          grd.addColorStop(0, grey(0.98));
          grd.addColorStop(0.7, grey(0.74));
          grd.addColorStop(1, grey(0.3));
          g.fillStyle = grd;
          g.fill();
        } else {
          g.fillStyle = shade(tint, tone);
          g.fill();
          g.clip();
          g.strokeStyle = shade(tint, tone * 1.2);
          g.lineWidth = 10;
          g.beginPath();
          g.roundRect(cx - w / 2 - 2, cy - h / 2 - 2, w - 4, h - 4, rad);
          g.stroke();
          g.strokeStyle = shade(tint, tone * 0.62);
          g.lineWidth = 12;
          g.beginPath();
          g.roundRect(cx - w / 2 + 10, cy - h / 2 + 10, w - 4, h - 4, rad);
          g.stroke();
        }
        g.restore();
        x += w;
      }
    }
  });
  draw(ctx, false);
  draw(hx, true);
  specks(ctx, S, S, mulberry(seed + 2), 1400, ["#3a3428", "#f4ecd8"], 2.4, [0.05, 0.14]);
  tiled(ctx, S, S, () => {
    const r2 = mulberry(seed + 5);
    for (let i = 0; i < 26; i++) {
      ctx.fillStyle = r2() < 0.5 ? "rgba(98,140,64,0.5)" : "rgba(124,160,74,0.42)";
      ctx.beginPath();
      ctx.ellipse(r2() * S, r2() * S, 8 + r2() * 20, 4 + r2() * 8, r2() * 3, 0, Math.PI * 2);
      ctx.fill();
    }
  });
  return withRelief(toTexture(c, { aniso: 16 }), hc, { strength: 2.6, scale: 1.1 });
}

// -------------------------------------------------------------------- roofs
const ROOFS = {
  // Wooden shakes: long, ragged, grain streaks.
  shake: { base: [150, 96, 58], dark: [62, 36, 22], light: [196, 140, 92], scallop: false, grain: true },
  shakeDark: { base: [118, 74, 46], dark: [48, 28, 18], light: [166, 112, 72], scallop: false, grain: true },
  moss: { base: [118, 132, 64], dark: [44, 62, 28], light: [168, 186, 92], scallop: false, grain: true, moss: true },
  teal: { base: [52, 132, 124], dark: [18, 58, 56], light: [110, 190, 176], scallop: true },
  red: { base: [170, 62, 52], dark: [72, 22, 22], light: [222, 116, 92], scallop: true },
  slate: { base: [86, 96, 120], dark: [34, 38, 52], light: [140, 152, 180], scallop: false },
};

/** Roofing in rows: `kind` is shake, shakeDark, moss, teal, red or slate. u runs along the ridge,
 *  v down the slope (the bottom of every row is lit, its top in the shade of the row above).
 *  512 px = 2 m. */
export function roofingTexture(kind = "shake", seed = 31) {
  const R = ROOFS[kind];
  const S = 512, [c, ctx] = canvas(S, S);
  ctx.fillStyle = shade(R.dark, 1);
  ctx.fillRect(0, 0, S, S);
  const rows = R.scallop ? 9 : 8, rh = S / rows;
  tiled(ctx, S, S, () => {
    const rng = mulberry(seed);
    for (let r = 0; r < rows; r++) {
      const y = r * rh;
      let x = (r % 2) * rh * 0.45 - rng() * rh * 0.3;
      while (x < S + rh) {
        const w = R.scallop ? rh * 1.05 : rh * (0.55 + rng() * 0.7);
        const tone = 0.84 + rng() * 0.3;
        const col = R.moss && rng() < 0.45 ? [R.base[0] * 0.72 + 30, R.base[1] * 0.95 + 18, R.base[2] * 0.6] : R.base;
        const g = ctx.createLinearGradient(0, y, 0, y + rh * 1.15);
        g.addColorStop(0, shade(col, tone * 0.62));
        g.addColorStop(0.55, shade(col, tone));
        g.addColorStop(1, shade(col, tone * 1.12));
        ctx.fillStyle = g;
        ctx.beginPath();
        if (R.scallop) {
          ctx.moveTo(x + 1.5, y - 2);
          ctx.lineTo(x + w - 1.5, y - 2);
          ctx.lineTo(x + w - 1.5, y + rh * 0.55);
          ctx.quadraticCurveTo(x + w / 2, y + rh * 1.28, x + 1.5, y + rh * 0.55);
        } else {
          const ragged = (rng() - 0.5) * rh * 0.16;
          ctx.moveTo(x + 1.5, y - 2);
          ctx.lineTo(x + w - 1.5, y - 2);
          ctx.lineTo(x + w - 1.5, y + rh * 1.02 + ragged);
          ctx.lineTo(x + w * 0.5, y + rh * 1.06);
          ctx.lineTo(x + 1.5, y + rh * 1.0 - ragged);
        }
        ctx.closePath();
        ctx.fill();
        ctx.strokeStyle = shade(R.dark, 0.8);
        ctx.lineWidth = 2;
        ctx.stroke();
        // The lit lip at the bottom of each shingle.
        ctx.strokeStyle = shade(R.light, 1);
        ctx.globalAlpha = 0.55;
        ctx.lineWidth = 2.4;
        ctx.beginPath();
        if (R.scallop) {
          ctx.moveTo(x + 5, y + rh * 0.62);
          ctx.quadraticCurveTo(x + w / 2, y + rh * 1.18, x + w - 5, y + rh * 0.62);
        } else {
          ctx.moveTo(x + 4, y + rh * 0.95);
          ctx.lineTo(x + w - 4, y + rh * 0.97);
        }
        ctx.stroke();
        ctx.globalAlpha = 1;
        if (R.grain) {
          ctx.strokeStyle = shade(R.dark, 1.3);
          ctx.globalAlpha = 0.22;
          ctx.lineWidth = 1;
          for (let k = 0; k < 3; k++) {
            const gx = x + 4 + rng() * (w - 8);
            ctx.beginPath();
            ctx.moveTo(gx, y + 4);
            ctx.lineTo(gx + (rng() - 0.5) * 3, y + rh * 0.9);
            ctx.stroke();
          }
          ctx.globalAlpha = 1;
        }
        if (R.moss && rng() < 0.25) {
          ctx.fillStyle = "rgba(120,170,70,0.55)";
          ctx.beginPath();
          ctx.ellipse(x + w * rng(), y + rh * (0.3 + rng() * 0.5), 6 + rng() * 10, 3 + rng() * 5, 0, 0, Math.PI * 2);
          ctx.fill();
        }
        x += w;
      }
    }
  });
  specks(ctx, S, S, mulberry(seed + 4), 900, [shade(R.dark, 0.7), shade(R.light, 1.1)], 2, [0.04, 0.1]);
  const texture = withRelief(toTexture(c, { aniso: 16 }), heightFromColor(c, 1.4), { strength: 3.2, scale: 1.0 });
  // The shingled roofs lay one course of real shingles on each painted row.
  texture.userData.rows = rows;
  return texture;
}

// --------------------------------------------------------------- foliage
/** Dense painted leaves (hedges, vines, ivy, the well's roof moss). 512 px = 1 m. */
export function foliageTexture({ dark = "#2d6630", mid = "#46913f", light = "#8cc65a", seed = 41 } = {}) {
  const S = 512, [c, ctx] = canvas(S, S);
  ctx.fillStyle = dark;
  ctx.fillRect(0, 0, S, S);
  tiled(ctx, S, S, () => {
    const rng = mulberry(seed);
    for (let i = 0; i < 900; i++) {
      const x = rng() * S, y = rng() * S, r = 7 + rng() * 9, a = rng() * Math.PI * 2;
      const t = rng();
      ctx.fillStyle = t < 0.35 ? dark : t < 0.8 ? mid : light;
      ctx.globalAlpha = 0.85;
      ctx.beginPath();
      ctx.ellipse(x, y, r, r * 0.55, a, 0, Math.PI * 2);
      ctx.fill();
    }
    ctx.globalAlpha = 1;
  });
  return withRelief(toTexture(c), heightFromColor(c, 1.5), { strength: 2.2, scale: 0.8 });
}

// ------------------------------------------------------------------- signs
const ICON_PATHS = {
  beer: (g, s) => {
    g.beginPath();
    g.roundRect(-s * 0.36, -s * 0.28, s * 0.56, s * 0.72, s * 0.06);
    g.fillStyle = "#e0a640";
    g.fill();
    g.stroke();
    g.beginPath();
    g.arc(s * 0.2, s * 0.08, s * 0.16, -Math.PI / 2, Math.PI / 2);
    g.lineWidth = s * 0.07;
    g.stroke();
    g.fillStyle = "#fbf3dc";
    g.beginPath();
    for (const [x, r] of [[-0.3, 0.13], [-0.12, 0.16], [0.08, 0.14]]) g.arc(s * x, -s * 0.3, s * r, 0, Math.PI * 2);
    g.fill();
  },
  bag: (g, s) => {
    g.fillStyle = "#c9944e";
    g.beginPath();
    g.moveTo(-s * 0.14, -s * 0.28);
    g.quadraticCurveTo(-s * 0.48, s * 0.05, -s * 0.36, s * 0.34);
    g.quadraticCurveTo(0, s * 0.48, s * 0.36, s * 0.34);
    g.quadraticCurveTo(s * 0.48, s * 0.05, s * 0.14, -s * 0.28);
    g.closePath();
    g.fill();
    g.stroke();
    g.beginPath();
    g.moveTo(-s * 0.2, -s * 0.3);
    g.lineTo(s * 0.2, -s * 0.3);
    g.stroke();
    g.fillStyle = "#f2d062";
    g.beginPath();
    g.arc(0, s * 0.1, s * 0.13, 0, Math.PI * 2);
    g.fill();
    g.stroke();
  },
  heart: (g, s) => {
    g.fillStyle = "#c62a42";
    g.beginPath();
    g.moveTo(0, s * 0.38);
    g.bezierCurveTo(-s * 0.62, -s * 0.02, -s * 0.3, -s * 0.48, 0, -s * 0.18);
    g.bezierCurveTo(s * 0.3, -s * 0.48, s * 0.62, -s * 0.02, 0, s * 0.38);
    g.fill();
    g.stroke();
  },
  steam: (g, s) => {
    g.lineWidth = s * 0.08;
    g.lineCap = "round";
    for (const x of [-0.2, 0, 0.2]) {
      g.beginPath();
      g.moveTo(s * x, s * 0.05);
      g.bezierCurveTo(s * (x - 0.12), -s * 0.12, s * (x + 0.12), -s * 0.22, s * x, -s * 0.4);
      g.stroke();
    }
    g.beginPath();
    g.moveTo(-s * 0.4, s * 0.12);
    g.quadraticCurveTo(0, s * 0.55, s * 0.4, s * 0.12);
    g.stroke();
  },
  scroll: (g, s) => {
    g.fillStyle = "#f2e2b4";
    g.beginPath();
    g.roundRect(-s * 0.34, -s * 0.3, s * 0.68, s * 0.6, s * 0.06);
    g.fill();
    g.stroke();
    for (const y of [-0.12, 0.02, 0.16]) {
      g.beginPath();
      g.moveTo(-s * 0.2, s * y);
      g.lineTo(s * 0.2, s * y);
      g.stroke();
    }
  },
};

/** A hanging shop sign: a wooden board with a gold border, a painted icon and a name. */
export function signboardTexture(icon, text, { w = 512, h = 384, bg = "#5a3420", fg = "#f2d690", ink = "#2a160a" } = {}) {
  const [c, ctx] = canvas(w, h);
  const g = ctx.createLinearGradient(0, 0, 0, h);
  g.addColorStop(0, shade(hex(bg), 1.15));
  g.addColorStop(1, shade(hex(bg), 0.85));
  ctx.fillStyle = g;
  ctx.fillRect(0, 0, w, h);
  const rng = mulberry(text.length * 13 + icon.length);
  ctx.strokeStyle = "rgba(0,0,0,0.18)";
  for (let i = 0; i < 18; i++) {
    const y = rng() * h;
    ctx.lineWidth = 1 + rng() * 2;
    ctx.beginPath();
    ctx.moveTo(0, y);
    ctx.bezierCurveTo(w * 0.3, y + (rng() - 0.5) * 12, w * 0.7, y + (rng() - 0.5) * 12, w, y + (rng() - 0.5) * 8);
    ctx.stroke();
  }
  ctx.strokeStyle = fg;
  ctx.lineWidth = 10;
  ctx.strokeRect(14, 14, w - 28, h - 28);
  ctx.lineWidth = 3;
  ctx.strokeRect(30, 30, w - 60, h - 60);
  ctx.save();
  ctx.translate(w / 2, h * 0.42);
  ctx.strokeStyle = ink;
  ctx.lineWidth = 7;
  ctx.lineJoin = "round";
  ICON_PATHS[icon]?.(ctx, h * 0.5);
  ctx.restore();
  ctx.fillStyle = fg;
  ctx.textAlign = "center";
  ctx.textBaseline = "middle";
  let size = h * 0.16;
  for (; size > 12; size -= 2) {
    ctx.font = `800 ${size}px Georgia, "Times New Roman", serif`;
    if (ctx.measureText(text).width < w - 90) break;
  }
  ctx.fillText(text, w / 2, h * 0.8);
  return toTexture(c, { wrap: false });
}

function hex(s) {
  const n = parseInt(s.slice(1), 16);
  return [(n >> 16) & 255, (n >> 8) & 255, n & 255];
}

/** The clock face: cream enamel, a gold ring, roman hours, the hands at twenty to seven. */
export function clockTexture() {
  const S = 512, [c, ctx] = canvas(S, S);
  ctx.clearRect(0, 0, S, S);
  const R = S / 2;
  ctx.translate(R, R);
  ctx.fillStyle = "#c79a3a";
  ctx.beginPath();
  ctx.arc(0, 0, R - 2, 0, Math.PI * 2);
  ctx.fill();
  ctx.fillStyle = "#f6ecd2";
  ctx.beginPath();
  ctx.arc(0, 0, R * 0.86, 0, Math.PI * 2);
  ctx.fill();
  ctx.strokeStyle = "#3a2410";
  ctx.lineWidth = 4;
  ctx.beginPath();
  ctx.arc(0, 0, R * 0.8, 0, Math.PI * 2);
  ctx.stroke();
  const roman = ["XII", "I", "II", "III", "IV", "V", "VI", "VII", "VIII", "IX", "X", "XI"];
  ctx.fillStyle = "#2a180a";
  ctx.textAlign = "center";
  ctx.textBaseline = "middle";
  ctx.font = `700 ${R * 0.15}px Georgia, serif`;
  roman.forEach((t, i) => {
    const a = (i / 12) * Math.PI * 2 - Math.PI / 2;
    ctx.fillText(t, Math.cos(a) * R * 0.64, Math.sin(a) * R * 0.64);
  });
  const hand = (a, len, wid) => {
    ctx.save();
    ctx.rotate(a);
    ctx.fillStyle = "#1e1208";
    ctx.beginPath();
    ctx.moveTo(-wid, 0);
    ctx.lineTo(0, -len);
    ctx.lineTo(wid, 0);
    ctx.lineTo(0, len * 0.14);
    ctx.closePath();
    ctx.fill();
    ctx.restore();
  };
  hand(((6 + 40 / 60) / 12) * Math.PI * 2, R * 0.42, R * 0.05);
  hand((40 / 60) * Math.PI * 2, R * 0.66, R * 0.035);
  ctx.fillStyle = "#c79a3a";
  ctx.beginPath();
  ctx.arc(0, 0, R * 0.05, 0, Math.PI * 2);
  ctx.fill();
  return toTexture(c, { wrap: false });
}

/** A striped market awning with a scalloped hem (alpha-cut). u across, v down to the hem. */
export function awningTexture(a = "#c23a32", b = "#f4e6c8") {
  const W = 512, H = 256, [c, ctx] = canvas(W, H);
  ctx.clearRect(0, 0, W, H);
  const n = 8, sw = W / n;
  for (let i = 0; i < n; i++) {
    ctx.fillStyle = i % 2 ? b : a;
    ctx.fillRect(i * sw, 0, sw, H * 0.8);
    ctx.beginPath();
    ctx.arc(i * sw + sw / 2, H * 0.8, sw / 2, 0, Math.PI);
    ctx.fill();
  }
  const g = ctx.createLinearGradient(0, 0, 0, H);
  g.addColorStop(0, "rgba(0,0,0,0.18)");
  g.addColorStop(0.4, "rgba(0,0,0,0)");
  g.addColorStop(1, "rgba(0,0,0,0.12)");
  ctx.globalCompositeOperation = "source-atop";
  ctx.fillStyle = g;
  ctx.fillRect(0, 0, W, H);
  return toTexture(c, { wrap: false });
}

/** The baths' door curtain: indigo panels with the hot-spring sign in cream. */
export function norenTexture() {
  const W = 512, H = 384, [c, ctx] = canvas(W, H);
  ctx.clearRect(0, 0, W, H);
  const panels = 3, pw = W / panels;
  for (let i = 0; i < panels; i++) {
    ctx.fillStyle = "#24305a";
    ctx.fillRect(i * pw + 3, 0, pw - 6, H);
  }
  ctx.fillStyle = "rgba(0,0,0,0.25)";
  ctx.fillRect(0, 0, W, 26);
  ctx.save();
  ctx.translate(W / 2, H * 0.52);
  ctx.strokeStyle = "#f2e6c8";
  ICON_PATHS.steam(ctx, H * 0.62);
  ctx.restore();
  return toTexture(c, { wrap: false });
}

/** A long council banner: deep green with gold trim and an oak leaf. */
export function councilBannerTexture() {
  const W = 256, H = 640, [c, ctx] = canvas(W, H);
  ctx.clearRect(0, 0, W, H);
  ctx.fillStyle = "#1f5a3a";
  ctx.beginPath();
  ctx.moveTo(0, 0);
  ctx.lineTo(W, 0);
  ctx.lineTo(W, H * 0.86);
  ctx.lineTo(W / 2, H);
  ctx.lineTo(0, H * 0.86);
  ctx.closePath();
  ctx.fill();
  ctx.strokeStyle = "#e2b85a";
  ctx.lineWidth = 12;
  ctx.stroke();
  ctx.save();
  ctx.translate(W / 2, H * 0.42);
  ctx.fillStyle = "#e2b85a";
  ctx.beginPath();
  ctx.moveTo(0, -90);
  for (let i = 0; i <= 6; i++) {
    const y = -90 + i * 30, bulge = 34 + Math.sin(i) * 8;
    ctx.quadraticCurveTo(bulge + 14, y + 8, 6, y + 22);
  }
  ctx.lineTo(0, 110);
  for (let i = 6; i >= 0; i--) {
    const y = -90 + i * 30, bulge = 34 + Math.sin(i) * 8;
    ctx.quadraticCurveTo(-bulge - 14, y + 8, -6, y - 8);
  }
  ctx.closePath();
  ctx.fill();
  ctx.restore();
  return toTexture(c, { wrap: false });
}

/** The village's welcome board over the gate. */
export function gateSignTexture(text = "Villa del Claro") {
  const W = 1024, H = 256, [c, ctx] = canvas(W, H);
  const g = ctx.createLinearGradient(0, 0, 0, H);
  g.addColorStop(0, "#7a4a2a");
  g.addColorStop(1, "#5a3420");
  ctx.fillStyle = g;
  ctx.fillRect(0, 0, W, H);
  ctx.strokeStyle = "#f0d48c";
  ctx.lineWidth = 10;
  ctx.strokeRect(12, 12, W - 24, H - 24);
  ctx.fillStyle = "#f6e2a6";
  ctx.font = `800 ${H * 0.46}px Georgia, serif`;
  ctx.textAlign = "center";
  ctx.textBaseline = "middle";
  ctx.fillText(text, W / 2, H / 2 + 6);
  return toTexture(c, { wrap: false });
}

// ------------------------------------------------------------- the houses
/** Lime plaster, smooth and laid by hand: soft patches of a lighter and a darker wash, the sweeps
 *  of the trowel, a few hairline cracks and specks. Near white, so each house's own colour tints
 *  it. 1024 px = 3 m. */
export function limePlasterTexture({ seed = 61 } = {}) {
  const S = 1024, [c, ctx] = canvas(S, S);
  ctx.fillStyle = "rgb(238,232,220)";
  ctx.fillRect(0, 0, S, S);
  blobs(ctx, S, S, seed, 40, "rgba(196,184,160,A)", "rgba(255,252,244,A)", 70, 240);
  blobs(ctx, S, S, seed + 1, 60, "rgba(206,194,170,A)", "rgba(255,250,240,A)", 20, 70);
  tiled(ctx, S, S, () => {
    const rng = mulberry(seed + 2);
    // Trowel sweeps: long shallow arcs, a shade lighter or darker.
    for (let i = 0; i < 90; i++) {
      const x = rng() * S, y = rng() * S, r = 50 + rng() * 110, a = rng() * Math.PI * 2;
      ctx.strokeStyle = rng() < 0.5 ? "rgba(255,252,246,0.22)" : "rgba(180,166,140,0.12)";
      ctx.lineWidth = 6 + rng() * 14;
      ctx.beginPath();
      ctx.arc(x, y, r, a, a + 0.5 + rng() * 0.6);
      ctx.stroke();
    }
    // Hairline cracks, wandering, here and there.
    for (let i = 0; i < 7; i++) {
      let x = rng() * S, y = rng() * S, a = rng() * Math.PI * 2;
      ctx.strokeStyle = "rgba(120,104,84,0.35)";
      ctx.lineWidth = 1.3;
      ctx.beginPath();
      ctx.moveTo(x, y);
      for (let k = 0; k < 9; k++) {
        a += (rng() - 0.5) * 1.1;
        x += Math.cos(a) * (8 + rng() * 10);
        y += Math.sin(a) * (8 + rng() * 10);
        ctx.lineTo(x, y);
      }
      ctx.stroke();
    }
  });
  specks(ctx, S, S, mulberry(seed + 3), 2600, ["#b8a888", "#ffffff", "#9c8c70"], 2.2, [0.05, 0.16]);
  return withRelief(toTexture(c, { aniso: 16 }), heightFromColor(c, 2.5), { strength: 1.0, scale: 0.35 });
}

/** The face of a dressed stone: mottled, flecked, a vein or two, a spot of lichen. Neutral, so
 *  the stones' materials tint it. 512 px = 1 m. */
export function rockTexture({ seed = 71 } = {}) {
  const S = 512, [c, ctx] = canvas(S, S);
  ctx.fillStyle = "rgb(214,208,196)";
  ctx.fillRect(0, 0, S, S);
  blobs(ctx, S, S, seed, 30, "rgba(150,140,124,A)", "rgba(250,246,236,A)", 30, 110);
  blobs(ctx, S, S, seed + 1, 50, "rgba(120,112,100,A)", "rgba(255,255,250,A)", 8, 30);
  tiled(ctx, S, S, () => {
    const rng = mulberry(seed + 2);
    for (let i = 0; i < 9; i++) {
      let x = rng() * S, y = rng() * S, a = rng() * Math.PI * 2;
      ctx.strokeStyle = "rgba(96,88,76,0.3)";
      ctx.lineWidth = 1.2 + rng();
      ctx.beginPath();
      ctx.moveTo(x, y);
      for (let k = 0; k < 6; k++) {
        a += (rng() - 0.5) * 0.9;
        x += Math.cos(a) * 12;
        y += Math.sin(a) * 12;
        ctx.lineTo(x, y);
      }
      ctx.stroke();
    }
    // Lichen: little yellow-green and pale crusts.
    for (let i = 0; i < 14; i++) {
      ctx.fillStyle = rng() < 0.5 ? "rgba(176,180,96,0.45)" : "rgba(226,222,196,0.5)";
      ctx.beginPath();
      ctx.ellipse(rng() * S, rng() * S, 4 + rng() * 9, 3 + rng() * 6, rng() * 3, 0, Math.PI * 2);
      ctx.fill();
    }
  });
  specks(ctx, S, S, mulberry(seed + 3), 1800, ["#6e6658", "#ffffff", "#8a8070"], 2, [0.08, 0.22]);
  return withRelief(toTexture(c, { aniso: 16 }), heightFromColor(c, 1.2), { strength: 1.8, scale: 0.6 });
}

/** Small diamond panes of old glass in lead, each pane a little different. For the windows' map
 *  and their glow alike, so at night the panes shine and the lead stays dark. */
export function leadedGlassTexture({ seed = 81 } = {}) {
  const W = 256, H = 256, [c, ctx] = canvas(W, H);
  const rng = mulberry(seed);
  ctx.fillStyle = "#ffffff";
  ctx.fillRect(0, 0, W, H);
  // Panes: diamonds on a lattice, each its own tone, with a soft sheen across it.
  const dx = W / 4, dy = H / 4;
  for (let j = -1; j <= 4; j++) {
    for (let i = -1; i <= 4; i++) {
      const cx = i * dx + (j % 2 ? dx / 2 : 0), cy = j * dy + dy / 2;
      const tone = 0.8 + rng() * 0.2;
      const g = ctx.createLinearGradient(cx - dx / 2, cy - dy / 2, cx + dx / 2, cy + dy / 2);
      g.addColorStop(0, grey(Math.min(1, tone + 0.12)));
      g.addColorStop(1, grey(tone * 0.9));
      ctx.fillStyle = g;
      ctx.beginPath();
      ctx.moveTo(cx, cy - dy / 2);
      ctx.lineTo(cx + dx / 2, cy);
      ctx.lineTo(cx, cy + dy / 2);
      ctx.lineTo(cx - dx / 2, cy);
      ctx.closePath();
      ctx.fill();
    }
  }
  // The lead between them: two families of diagonals through the diamonds' corners.
  ctx.strokeStyle = "rgb(40,34,30)";
  ctx.lineWidth = 5;
  const slope = dy / dx; // rise per run of a diamond's edge... (dy/2) / (dx/2)
  for (let k = -10; k <= 10; k++) {
    const x0 = k * dx;
    ctx.beginPath();
    ctx.moveTo(x0, -dy / 2);
    ctx.lineTo(x0 + (H + dy) / slope, H + dy / 2);
    ctx.stroke();
    ctx.beginPath();
    ctx.moveTo(x0, -dy / 2);
    ctx.lineTo(x0 - (H + dy) / slope, H + dy / 2);
    ctx.stroke();
  }
  // A heavier band of lead round the edge.
  ctx.lineWidth = 10;
  ctx.strokeRect(0, 0, W, H);
  return toTexture(c, { wrap: false, aniso: 8 });
}

/** A shutter of four planks with dark grooves, grain and nail heads. Near white: the shutter's own
 *  paint tints it. */
export function shutterTexture({ seed = 91 } = {}) {
  const W = 256, H = 512, [c, ctx] = canvas(W, H);
  const rng = mulberry(seed);
  const planks = 4, pw = W / planks;
  for (let p = 0; p < planks; p++) {
    ctx.fillStyle = grey(0.86 + rng() * 0.12);
    ctx.fillRect(p * pw, 0, pw, H);
    ctx.strokeStyle = "rgba(60,50,40,0.18)";
    ctx.lineWidth = 1.2;
    for (let k = 0; k < 6; k++) {
      const x = p * pw + 6 + rng() * (pw - 12);
      ctx.beginPath();
      ctx.moveTo(x, 0);
      ctx.bezierCurveTo(x + (rng() - 0.5) * 8, H * 0.35, x + (rng() - 0.5) * 8, H * 0.65, x, H);
      ctx.stroke();
    }
    // The groove between planks.
    ctx.fillStyle = "rgba(30,24,18,0.75)";
    ctx.fillRect(p * pw - 2, 0, 4, H);
  }
  // Paint worn pale at the edges, and nail heads where the battens are.
  ctx.strokeStyle = "rgba(255,255,255,0.35)";
  ctx.lineWidth = 6;
  ctx.strokeRect(3, 3, W - 6, H - 6);
  ctx.fillStyle = "rgba(40,32,26,0.8)";
  for (const y of [H * 0.2, H * 0.8]) {
    for (let p = 0; p < planks; p++) {
      ctx.beginPath();
      ctx.arc(p * pw + pw / 2, y, 4, 0, Math.PI * 2);
      ctx.fill();
    }
  }
  return toTexture(c, { wrap: false, aniso: 8 });
}

/** A door of broad oak planks: grained, a knot here and there, the seams dark. */
export function plankDoorTexture({ seed = 95 } = {}) {
  const W = 256, H = 512, [c, ctx] = canvas(W, H);
  const rng = mulberry(seed);
  const planks = 5, pw = W / planks;
  for (let p = 0; p < planks; p++) {
    ctx.fillStyle = shade([112, 70, 40], 0.85 + rng() * 0.25);
    ctx.fillRect(p * pw, 0, pw, H);
    for (let k = 0; k < 9; k++) {
      const x = p * pw + 4 + rng() * (pw - 8);
      ctx.strokeStyle = `rgba(40,20,8,${(0.15 + rng() * 0.2).toFixed(3)})`;
      ctx.lineWidth = 0.8 + rng() * 1.4;
      ctx.beginPath();
      ctx.moveTo(x, 0);
      ctx.bezierCurveTo(x + (rng() - 0.5) * 10, H * 0.3, x + (rng() - 0.5) * 10, H * 0.7, x, H);
      ctx.stroke();
    }
    if (rng() < 0.6) {
      const kx = p * pw + pw * (0.3 + rng() * 0.4), ky = rng() * H;
      ctx.strokeStyle = "rgba(40,20,8,0.5)";
      ctx.lineWidth = 1.5;
      for (const r of [4, 7, 10]) {
        ctx.beginPath();
        ctx.ellipse(kx, ky, r * 0.6, r, 0, 0, Math.PI * 2);
        ctx.stroke();
      }
    }
    ctx.fillStyle = "rgba(24,12,4,0.85)";
    ctx.fillRect(p * pw - 2, 0, 4, H);
  }
  return toTexture(c, { wrap: false, aniso: 8 });
}
