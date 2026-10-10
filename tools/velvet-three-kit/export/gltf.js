// What the prototypes' exporters share (page side): a binary glTF of a scene, plain copies of the
// textures for it, numbers rounded for the plans.
import * as THREE from "three";
import { GLTFExporter } from "three/addons/exporters/GLTFExporter.js";

/** A number rounded to 1/k (the plans keep millimetres). */
export const round = (v, k = 1000) => Math.round(v * k) / k;

/** A scene as a binary glTF, hidden pieces too, textures up to 2048 px. */
export async function glb(object, options = {}) {
  const exporter = new GLTFExporter();
  const result = await exporter.parseAsync(object, { binary: true, onlyVisible: false, maxTextureSize: 2048, ...options });
  return new Uint8Array(result);
}

/** Plain maps for export: the same image, no transform (it is in the UVs now). Written as JPEG
 *  (a fraction of a PNG's size) unless the material cuts its shape out of the texture. */
export function plainMap(map, cache, cutout = false) {
  if (!map) return null;
  if (cache.has(map)) return cache.get(map);
  const t = new THREE.Texture(map.image);
  t.wrapS = t.wrapT = THREE.RepeatWrapping;
  t.colorSpace = map.colorSpace;
  t.flipY = map.flipY;
  t.userData.mimeType = cutout ? "image/png" : "image/jpeg";
  t.needsUpdate = true;
  cache.set(map, t);
  return t;
}
