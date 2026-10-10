// What the exporters of world scenes written piece by piece share (the wood, the villa): materials
// as plain lit copies named after their library key, texture transforms baked into the UVs,
// instances written out one by one, back faces as real triangles.
import * as THREE from "three";
import { mergeGeometries } from "three/addons/utils/BufferGeometryUtils.js";
import { sourceOf } from "../toon.js";
import { plainMap } from "./gltf.js";

/** A plain copy of a lit material for export (no toon patch, no custom shader), one-sided. */
export function exportMaterial(material, cache, names) {
  const src = sourceOf(material);
  if (cache.has(src)) return cache.get(src);
  let out;
  if (src.isMeshBasicMaterial) {
    out = new THREE.MeshStandardMaterial({ color: src.color, map: src.map, emissive: src.color, emissiveIntensity: 1 });
  } else {
    out = new THREE.MeshStandardMaterial({
      color: src.color, map: src.map, normalMap: src.normalMap, roughness: src.roughness ?? 0.8, metalness: src.metalness ?? 0,
      alphaTest: src.alphaTest ?? 0, vertexColors: !!src.vertexColors,
      emissive: src.emissive ?? new THREE.Color(0), emissiveIntensity: src.emissiveIntensity ?? 1,
    });
  }
  const base = names.get(src) ?? names.get(src.userData?.bakedFrom) ?? names.get(material) ?? src.name ?? "";
  out.name = base || `mat_${cache.size}`;
  cache.set(src, out);
  return out;
}

/** UVs with the colour map's transform (rotation / repeat / offset) applied. */
export function bakedUV(geometry, map) {
  const uv = geometry.attributes.uv;
  if (!uv || !map) return geometry;
  map.updateMatrix();
  const e = map.matrix.elements;
  if (e[0] === 1 && e[1] === 0 && e[3] === 0 && e[4] === 1 && e[6] === 0 && e[7] === 0) return geometry;
  const g = geometry.clone();
  const out = new Float32Array(uv.count * 2);
  const v = new THREE.Vector3();
  for (let i = 0; i < uv.count; i++) {
    v.set(uv.getX(i), uv.getY(i), 1).applyMatrix3(map.matrix);
    out[i * 2] = v.x;
    out[i * 2 + 1] = v.y;
  }
  g.setAttribute("uv", new THREE.BufferAttribute(out, 2));
  return g;
}

/** Whether a material cuts its shape out of its texture's alpha. */
export const cutsOut = (src) => (src.alphaTest ?? 0) > 0 || !!src.transparent || !!src.alphaMap;

/** Only the attributes the game reads, all present (a white COLOR_0 if `color` is asked). */
export function tidy(geometry, color) {
  const g = geometry.clone();
  for (const name of Object.keys(g.attributes)) if (!["position", "normal", "uv", "color"].includes(name)) g.deleteAttribute(name);
  if (!g.attributes.normal) g.computeVertexNormals();
  if (!g.attributes.uv) g.setAttribute("uv", new THREE.BufferAttribute(new Float32Array(g.attributes.position.count * 2), 2));
  if (g.attributes.color && g.attributes.color.itemSize === 4) {
    const c = g.attributes.color, rgb = new Float32Array(c.count * 3);
    for (let i = 0; i < c.count; i++) rgb.set([c.getX(i), c.getY(i), c.getZ(i)], i * 3);
    g.setAttribute("color", new THREE.BufferAttribute(rgb, 3));
  }
  if (color && !g.attributes.color) g.setAttribute("color", new THREE.BufferAttribute(new Float32Array(g.attributes.position.count * 3).fill(1), 3));
  return g;
}

/** The same triangles facing the other way (the game culls back faces). */
export function backFaces(geometry) {
  const g = geometry.clone();
  const n = g.attributes.normal;
  for (let i = 0; i < n.count; i++) n.setXYZ(i, -n.getX(i), -n.getY(i), -n.getZ(i));
  if (g.index) {
    const idx = g.index.array.slice();
    for (let t = 0; t < idx.length; t += 3) [idx[t + 1], idx[t + 2]] = [idx[t + 2], idx[t + 1]];
    g.setIndex(new THREE.BufferAttribute(idx, 1));
    return g;
  }
  for (const attr of Object.values(g.attributes)) {
    const a = attr.array, k = attr.itemSize;
    for (let t = 0; t < attr.count; t += 3) {
      for (let c = 0; c < k; c++) {
        const i1 = (t + 1) * k + c, i2 = (t + 2) * k + c;
        [a[i1], a[i2]] = [a[i2], a[i1]];
      }
    }
  }
  return g;
}

/**
 * A scene of world-space meshes for every visible, non-skinned mesh under `roots` for which
 * `keep(mesh)` holds; instanced meshes become one mesh with every instance (and its colour)
 * written out, double-sided ones get their back faces.
 */
function worldScene(roots, keep, libraryNames) {
  const scene = new THREE.Scene();
  const names = libraryNames();
  const matCache = new Map(), mapCache = new Map();
  const m = new THREE.Matrix4(), col = new THREE.Color();
  for (const root of [roots].flat()) {
    if (!root) continue;
    root.updateMatrixWorld(true);
    root.traverse((o) => {
      if (!o.isMesh || o.isSkinnedMesh || !keep(o)) return;
      for (let p = o; p; p = p.parent) if (p.visible === false && p !== root) return;
      const src = sourceOf(Array.isArray(o.material) ? o.material[0] : o.material);
      if (src.isShaderMaterial || src.isPointsMaterial || src.blending === THREE.AdditiveBlending || (src.transparent && src.opacity < 0.99)) return;
      let geometry;
      if (o.isInstancedMesh) {
        const parts = [];
        for (let i = 0; i < o.count; i++) {
          o.getMatrixAt(i, m);
          const g = tidy(o.geometry, !!o.instanceColor).applyMatrix4(m.premultiply(o.matrixWorld));
          if (o.instanceColor) {
            o.getColorAt(i, col);
            const c = g.attributes.color;
            for (let k = 0; k < c.count; k++) c.setXYZ(k, c.getX(k) * col.r, c.getY(k) * col.g, c.getZ(k) * col.b);
          }
          parts.push(g);
        }
        if (!parts.length) return;
        geometry = mergeGeometries(parts, false);
      } else {
        geometry = tidy(o.geometry, false).applyMatrix4(o.matrixWorld);
      }
      geometry = bakedUV(geometry, src.map);
      if (src.side === THREE.DoubleSide) geometry = mergeGeometries([geometry, backFaces(geometry)], false);
      const base = exportMaterial(src, matCache, names);
      const material = base.clone();
      material.name = base.name;
      material.map = plainMap(src.map, mapCache, cutsOut(src));
      material.normalMap = plainMap(src.normalMap, mapCache);
      if (src.normalMap) material.normalScale = src.normalScale.clone();
      material.vertexColors = !!geometry.attributes.color;
      geometry.name = material.name;
      const mesh = new THREE.Mesh(geometry, material);
      mesh.name = material.name;
      scene.add(mesh);
    });
  }
  return scene;
}

/** A scene of ready meshes (already in world space, export materials). */
export function plainScene(meshes) {
  const scene = new THREE.Scene();
  for (const mesh of meshes) {
    mesh.geometry = tidy(mesh.geometry.clone().applyMatrix4(mesh.matrix), false);
    mesh.position.set(0, 0, 0);
    mesh.rotation.set(0, 0, 0);
    mesh.scale.set(1, 1, 1);
    mesh.updateMatrix();
    mesh.geometry.name = mesh.material.name;
    mesh.name = mesh.material.name; // the glTF mesh name the game keys on
    scene.add(mesh);
  }
  return scene;
}


/**
 * The world-scene writer of a prototype whose materials are named by `libraryNames()` (a Map from a
 * material to its library key): `worldScene(roots, keep)` writes every visible, non-skinned mesh
 * under `roots` that `keep` lets through, in world coordinates, each named after its material.
 */
export function worldExporter(libraryNames) {
  return { worldScene: (roots, keep = () => true) => worldScene(roots, keep, libraryNames) };
}
