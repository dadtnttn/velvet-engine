// What the exporters of scenes merged into one mesh per material share (the castle, the springs):
// pieces tidied to the attributes the game reads, all in plain float arrays so they merge; texture
// transforms baked into the UVs; back faces as real triangles; skinned meshes frozen in their pose.
import * as THREE from "three";

/** Only the attributes the game reads, all present (white COLOR_0 when there is none). */
export function tidy(geometry) {
  let g = geometry.index ? geometry.toNonIndexed() : geometry.clone();
  for (const name of Object.keys(g.attributes)) if (!["position", "normal", "uv", "color"].includes(name)) g.deleteAttribute(name);
  if (!g.attributes.normal) g.computeVertexNormals();
  if (!g.attributes.uv) g.setAttribute("uv", new THREE.BufferAttribute(new Float32Array(g.attributes.position.count * 2), 2));
  const c = g.attributes.color;
  if (c && c.itemSize === 4) {
    const rgb = new Float32Array(c.count * 3);
    for (let i = 0; i < c.count; i++) rgb.set([c.getX(i), c.getY(i), c.getZ(i)], i * 3);
    g.setAttribute("color", new THREE.BufferAttribute(rgb, 3));
  }
  if (!g.attributes.color) g.setAttribute("color", new THREE.BufferAttribute(new Float32Array(g.attributes.position.count * 3).fill(1), 3));
  // Merging needs the same array types throughout.
  for (const name of ["position", "normal", "uv", "color"]) {
    const a = g.attributes[name];
    if (!(a.array instanceof Float32Array) || a.normalized) {
      const out = new Float32Array(a.count * a.itemSize);
      for (let i = 0; i < a.count; i++) for (let k = 0; k < a.itemSize; k++) out[i * a.itemSize + k] = a.getComponent(i, k);
      g.setAttribute(name, new THREE.BufferAttribute(out, a.itemSize));
    }
  }
  return g;
}

/** UVs with the colour map's transform (repeat / offset / rotation) applied. */
export function bakedUV(geometry, map) {
  const uv = geometry.attributes.uv;
  if (!uv || !map) return geometry;
  map.updateMatrix();
  const v = new THREE.Vector3();
  for (let i = 0; i < uv.count; i++) {
    v.set(uv.getX(i), uv.getY(i), 1).applyMatrix3(map.matrix);
    uv.setXY(i, v.x, v.y);
  }
  return geometry;
}

/** The same triangles facing the other way (the game culls back faces). */
export function backFaces(g) {
  const b = g.clone();
  const n = b.attributes.normal;
  for (let i = 0; i < n.count; i++) n.setXYZ(i, -n.getX(i), -n.getY(i), -n.getZ(i));
  for (const attr of Object.values(b.attributes)) {
    const a = attr.array, k = attr.itemSize;
    for (let t = 0; t < attr.count; t += 3) {
      for (let c = 0; c < k; c++) {
        const i1 = (t + 1) * k + c, i2 = (t + 2) * k + c;
        [a[i1], a[i2]] = [a[i2], a[i1]];
      }
    }
  }
  return b;
}

/** A skinned mesh frozen in its current pose, in world space. */
export function frozen(o) {
  const g = o.geometry.clone();
  const pos = g.attributes.position;
  const v = new THREE.Vector3();
  o.skeleton.update();
  for (let i = 0; i < pos.count; i++) {
    v.fromBufferAttribute(pos, i);
    o.applyBoneTransform(i, v);
    pos.setXYZ(i, v.x, v.y, v.z);
  }
  g.deleteAttribute("skinIndex");
  g.deleteAttribute("skinWeight");
  g.applyMatrix4(o.matrixWorld);
  g.computeVertexNormals();
  return g;
}

/** Whether a material glows by itself (the game names those `lampGlass`). */
export const glows = (src) => src.emissive && (src.emissiveIntensity ?? 0) > 0.05 && (src.emissive.r + src.emissive.g + src.emissive.b) > 0.05;
