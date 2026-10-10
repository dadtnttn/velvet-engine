// Relief for painted textures: a height canvas (painted alongside the colour, or derived from its
// light and dark) turned into a tangent-space normal map, so seams sink, boards crown, stones dome
// and grain catches the light. Normal maps are sampled with the colour texture's UVs, and three.js
// builds the tangent frame from screen-space derivatives, so world-mapped UVs work as they are.
import * as THREE from "three";

/** Heights (0..1) from a canvas: its red channel (height canvases are painted in grey). */
function heights(canvas) {
  const { width: w, height: h } = canvas;
  const data = canvas.getContext("2d").getImageData(0, 0, w, h).data;
  const out = new Float32Array(w * h);
  for (let i = 0; i < w * h; i++) out[i] = data[i * 4] / 255;
  return out;
}

/** A grey height canvas from a colour canvas: its luminance, softened by `blur` pixels. */
export function heightFromColor(colorCanvas, blur = 1.5) {
  const c = document.createElement("canvas");
  c.width = colorCanvas.width;
  c.height = colorCanvas.height;
  const ctx = c.getContext("2d");
  ctx.filter = `grayscale(1) blur(${blur}px)`;
  ctx.drawImage(colorCanvas, 0, 0);
  return c;
}

/**
 * A normal map from a grey height canvas. `strength` scales the slopes; `wrap` samples across the
 * edges (for tiling textures). OpenGL convention (green = +v), as three.js expects.
 */
export function normalFromHeight(heightCanvas, { strength = 2, wrap = true } = {}) {
  const w = heightCanvas.width, h = heightCanvas.height;
  const H = heights(heightCanvas);
  const c = document.createElement("canvas");
  c.width = w;
  c.height = h;
  const ctx = c.getContext("2d");
  const img = ctx.createImageData(w, h);
  const px = img.data;
  const at = (x, y) => {
    if (wrap) {
      x = (x + w) % w;
      y = (y + h) % h;
    } else {
      x = x < 0 ? 0 : x >= w ? w - 1 : x;
      y = y < 0 ? 0 : y >= h ? h - 1 : y;
    }
    return H[y * w + x];
  };
  for (let y = 0; y < h; y++) {
    for (let x = 0; x < w; x++) {
      // Sobel: slope along u (right) and along v (up = the canvas row above).
      const tl = at(x - 1, y - 1), t = at(x, y - 1), tr = at(x + 1, y - 1);
      const l = at(x - 1, y), r = at(x + 1, y);
      const bl = at(x - 1, y + 1), b = at(x, y + 1), br = at(x + 1, y + 1);
      const du = (tr + 2 * r + br - tl - 2 * l - bl) * strength;
      const dv = (tl + 2 * t + tr - bl - 2 * b - br) * strength;
      let nx = -du, ny = -dv, nz = 1;
      const len = Math.hypot(nx, ny, nz);
      nx /= len;
      ny /= len;
      nz /= len;
      const i = (y * w + x) * 4;
      px[i] = (nx * 0.5 + 0.5) * 255;
      px[i + 1] = (ny * 0.5 + 0.5) * 255;
      px[i + 2] = (nz * 0.5 + 0.5) * 255;
      px[i + 3] = 255;
    }
  }
  ctx.putImageData(img, 0, 0);
  return c;
}

/** Attaches a normal map to a colour texture (materials.js picks it up as `normalMap`). */
export function withRelief(texture, heightCanvas, { strength = 2, scale = 1, wrap = true } = {}) {
  const n = new THREE.CanvasTexture(normalFromHeight(heightCanvas, { strength, wrap }));
  n.colorSpace = THREE.NoColorSpace;
  n.wrapS = texture.wrapS;
  n.wrapT = texture.wrapT;
  n.repeat.copy(texture.repeat);
  n.anisotropy = texture.anisotropy;
  texture.userData.normal = n;
  texture.userData.normalScale = scale;
  return texture;
}
