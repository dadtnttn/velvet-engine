// Turns a pile of small meshes into a few big ones: everything static under a
// root is merged per material. On the way, world-mapped materials get their UVs
// rebuilt from world positions (so wood grain and plaster keep one scale) and
// every vertex gets a little ambient occlusion: vertical faces darken towards
// the floor, undersides go dark, upward faces stay lit.
import * as THREE from "three";
import { mergeGeometries } from "three/addons/utils/BufferGeometryUtils.js";

const AO_LOW = 0.74; // soft: the toon ramp already darkens the sides
const AO_HEIGHT = 1.15;

const smooth = (a, b, v) => {
  const t = Math.min(1, Math.max(0, (v - a) / (b - a)));
  return t * t * (3 - 2 * t);
};

function remapUV(geo, scale) {
  const pos = geo.attributes.position, nor = geo.attributes.normal;
  const uv = new Float32Array(pos.count * 2);
  for (let i = 0; i < pos.count; i++) {
    const ax = Math.abs(nor.getX(i)), ay = Math.abs(nor.getY(i)), az = Math.abs(nor.getZ(i));
    let u, v;
    if (ay >= ax && ay >= az) {
      u = pos.getX(i);
      v = pos.getZ(i);
    } else if (ax >= az) {
      u = pos.getZ(i);
      v = pos.getY(i);
    } else {
      u = pos.getX(i);
      v = pos.getY(i);
    }
    uv[i * 2] = u / scale;
    uv[i * 2 + 1] = v / scale;
  }
  geo.setAttribute("uv", new THREE.BufferAttribute(uv, 2));
}

function occlusion(geo) {
  const pos = geo.attributes.position, nor = geo.attributes.normal;
  const colors = new Float32Array(pos.count * 3);
  for (let i = 0; i < pos.count; i++) {
    const ny = nor.getY(i), y = pos.getY(i);
    let k;
    if (ny > 0.55) k = 1;
    else if (ny < -0.55) k = 0.66;
    else k = AO_LOW + (1 - AO_LOW) * smooth(0.0, AO_HEIGHT, y);
    colors[i * 3] = colors[i * 3 + 1] = colors[i * 3 + 2] = k;
  }
  geo.setAttribute("color", new THREE.BufferAttribute(colors, 3));
}

function skipped(o, root) {
  for (let p = o; p && p !== root; p = p.parent) if (p.userData.dynamic || p.userData.noBake) return true;
  return false;
}

function owner(o, root) {
  for (let p = o.parent; p && p !== root; p = p.parent) if (p.userData.bakeRoot) return p;
  return null;
}

function normalise(geo, withUV) {
  for (const name of Object.keys(geo.attributes)) if (!["position", "normal", "uv", "color"].includes(name)) geo.deleteAttribute(name);
  if (!geo.attributes.normal) geo.computeVertexNormals();
  if (!geo.attributes.uv) geo.setAttribute("uv", new THREE.BufferAttribute(new Float32Array(geo.attributes.position.count * 2), 2));
  void withUV;
  return geo;
}

function bakeInto(target, { ao }, stats) {
  const inv = new THREE.Matrix4().copy(target.matrixWorld).invert();
  const groups = new Map();
  const used = new Map();
  const bakedMaterial = (mat) => {
    if (!used.has(mat)) {
      const m = mat.clone();
      m.vertexColors = ao;
      m.userData.bakedFrom = mat; // the library material it stands for (exports name it by that)
      used.set(mat, m);
    }
    return used.get(mat);
  };
  const meshes = [];
  target.traverse((o) => {
    if (!o.isMesh || o.isSkinnedMesh || o.isInstancedMesh) return;
    if (Array.isArray(o.material) || o.material.transparent || o.userData.noBake) return;
    if (skipped(o, target)) return;
    // Meshes under a nested bake root (a floor inside the house, a wall inside a floor) are its own.
    if (owner(o, target)) return;
    meshes.push(o);
  });
  for (const o of meshes) {
    const geo = o.geometry.clone();
    geo.applyMatrix4(o.matrixWorld);
    normalise(geo);
    if (o.material.userData.worldUV) remapUV(geo, o.material.userData.uvScale ?? 2);
    if (ao) occlusion(geo);
    geo.applyMatrix4(inv);
    const key = `${o.material.uuid}|${o.castShadow ? 1 : 0}${o.receiveShadow ? 1 : 0}`;
    if (!groups.has(key)) groups.set(key, { material: o.material, cast: o.castShadow, receive: o.receiveShadow, geos: [] });
    groups.get(key).geos.push(geo);
    stats.meshes++;
  }
  for (const o of meshes) {
    o.parent?.remove(o);
    o.geometry.dispose();
  }
  for (const { material, cast, receive, geos } of groups.values()) {
    const indexed = geos.every((g) => g.index);
    const list = indexed ? geos : geos.map((g) => (g.index ? g.toNonIndexed() : g));
    if (!ao) for (const g of list) g.deleteAttribute("color");
    const merged = mergeGeometries(list, false);
    if (!merged) continue;
    const mesh = new THREE.Mesh(merged, bakedMaterial(material));
    mesh.castShadow = cast;
    mesh.receiveShadow = receive;
    mesh.frustumCulled = false;
    mesh.name = `baked:${material.name || material.uuid.slice(0, 6)}`;
    target.add(mesh);
    stats.merged++;
    for (const g of geos) g.dispose();
  }
}

/**
 * Bakes `root`: static meshes merged per material. Subtrees marked `dynamic`
 * are left alone; subtrees marked `bakeRoot` (walls that fade) are baked into
 * themselves so they can still be shown and hidden.
 */
export function bake(root, { ao = true } = {}) {
  root.updateMatrixWorld(true);
  const stats = { meshes: 0, merged: 0 };
  const roots = [];
  root.traverse((o) => {
    if (o !== root && o.userData.bakeRoot) roots.push(o);
  });
  for (const r of roots) bakeInto(r, { ao }, stats);
  bakeInto(root, { ao }, stats);
  return stats;
}

/** Merges the direct mesh children of `group` per material (for characters). Keeps UVs. */
export function mergeChildren(group) {
  const groups = new Map();
  const remove = [];
  for (const child of group.children) {
    if (!child.isMesh || child.isSkinnedMesh || Array.isArray(child.material) || child.userData.keep) continue;
    child.updateMatrix();
    const geo = child.geometry.clone().applyMatrix4(child.matrix);
    normalise(geo);
    const key = `${child.material.uuid}|${child.castShadow ? 1 : 0}`;
    if (!groups.has(key)) groups.set(key, { material: child.material, cast: child.castShadow, geos: [] });
    groups.get(key).geos.push(geo);
    remove.push(child);
  }
  for (const c of remove) {
    group.remove(c);
    c.geometry.dispose();
  }
  for (const { material, cast, geos } of groups.values()) {
    const indexed = geos.every((g) => g.index);
    const merged = mergeGeometries(indexed ? geos : geos.map((g) => (g.index ? g.toNonIndexed() : g)), false);
    if (!merged) continue;
    const mesh = new THREE.Mesh(merged, material);
    mesh.castShadow = cast;
    mesh.receiveShadow = true;
    mesh.frustumCulled = false;
    group.add(mesh);
  }
  for (const child of group.children) if (child.isGroup) mergeChildren(child);
}
