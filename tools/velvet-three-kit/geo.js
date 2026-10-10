// Mesh helpers. Everything stands on y = 0 (boxes, cylinders) unless a helper
// says it is centred (sphere). Props declare their lamps, halos and floor
// shadows on `userData`; `collectDecor` turns those into world positions once
// the whole house is in place.
import * as THREE from "three";
import { RoundedBoxGeometry } from "three/addons/geometries/RoundedBoxGeometry.js";

function flagged(mesh, cast = true) {
  mesh.castShadow = cast;
  mesh.receiveShadow = true;
  return mesh;
}

export const group = (name) => {
  const g = new THREE.Group();
  if (name) g.name = name;
  return g;
};

/** A box with softly rounded edges, standing on `y`. */
export function rbox(w, h, d, mat, x = 0, y = 0, z = 0, r = 0.02) {
  const rr = Math.min(r, w / 2 - 1e-3, h / 2 - 1e-3, d / 2 - 1e-3);
  const geometry = rr > 0.004 ? new RoundedBoxGeometry(w, h, d, 2, rr) : new THREE.BoxGeometry(w, h, d);
  const mesh = new THREE.Mesh(geometry, mat);
  mesh.position.set(x, y + h / 2, z);
  return flagged(mesh);
}

export function box(w, h, d, mat, x = 0, y = 0, z = 0) {
  const mesh = new THREE.Mesh(new THREE.BoxGeometry(w, h, d), mat);
  mesh.position.set(x, y + h / 2, z);
  return flagged(mesh);
}

export function cyl(rTop, rBottom, h, mat, x = 0, y = 0, z = 0, seg = 16) {
  const mesh = new THREE.Mesh(new THREE.CylinderGeometry(rTop, rBottom, h, seg), mat);
  mesh.position.set(x, y + h / 2, z);
  return flagged(mesh);
}

/** A cylinder lying along an axis (x or z), centred on (x, y, z): rails, rods, logs. */
export function hcyl(len, r, mat, x = 0, y = 0, z = 0, axis = "x", seg = 10) {
  const mesh = new THREE.Mesh(new THREE.CylinderGeometry(r, r, len, seg), mat);
  mesh.position.set(x, y, z);
  if (axis === "x") mesh.rotation.z = Math.PI / 2;
  else mesh.rotation.x = Math.PI / 2;
  return flagged(mesh);
}

export function cone(r, h, mat, x = 0, y = 0, z = 0, seg = 16) {
  const mesh = new THREE.Mesh(new THREE.ConeGeometry(r, h, seg), mat);
  mesh.position.set(x, y + h / 2, z);
  return flagged(mesh);
}

/** A sphere centred on y, optionally squashed on its height. */
export function sphere(r, mat, x = 0, y = 0, z = 0, sy = 1, seg = 16) {
  const mesh = new THREE.Mesh(new THREE.SphereGeometry(r, seg, Math.max(6, seg - 4)), mat);
  mesh.position.set(x, y, z);
  mesh.scale.y = sy;
  return flagged(mesh);
}

/** A ring lying flat (a hoop, a rim); `tilt` turns it up on its side. */
export function torus(R, r, mat, x = 0, y = 0, z = 0, tilt = Math.PI / 2, seg = 20) {
  const mesh = new THREE.Mesh(new THREE.TorusGeometry(R, r, 8, seg), mat);
  mesh.position.set(x, y, z);
  mesh.rotation.x = tilt;
  return flagged(mesh);
}

/** A turned profile (vases, candlesticks, pots): [[radius, height], ...] from the base up. */
export function lathe(profile, mat, x = 0, y = 0, z = 0, seg = 20) {
  const points = profile.map(([r, h]) => new THREE.Vector2(r, h));
  const mesh = new THREE.Mesh(new THREE.LatheGeometry(points, seg), mat);
  mesh.position.set(x, y, z);
  return flagged(mesh);
}

/** A flat panel facing +z, centred on y. */
export function plane(w, h, mat, x = 0, y = 0, z = 0, yaw = 0, cast = false) {
  const mesh = new THREE.Mesh(new THREE.PlaneGeometry(w, h), mat);
  mesh.position.set(x, y, z);
  mesh.rotation.y = yaw;
  return flagged(mesh, cast);
}

/** A flat panel lying on the floor (rugs, decals), width along x, length along z. */
export function flat(w, l, mat, x = 0, y = 0.03, z = 0, yaw = 0) {
  const geometry = new THREE.PlaneGeometry(w, l);
  geometry.rotateX(-Math.PI / 2);
  const mesh = new THREE.Mesh(geometry, mat);
  mesh.position.set(x, y, z);
  mesh.rotation.y = yaw;
  mesh.castShadow = false;
  mesh.receiveShadow = true;
  return mesh;
}

export function heartShape(s = 1) {
  const shape = new THREE.Shape();
  shape.moveTo(0, -0.75 * s);
  shape.bezierCurveTo(-1.25 * s, 0.05 * s, -0.6 * s, 0.95 * s, 0, 0.38 * s);
  shape.bezierCurveTo(0.6 * s, 0.95 * s, 1.25 * s, 0.05 * s, 0, -0.75 * s);
  return shape;
}

/** A door-like outline: w wide, h tall, its top rounded into an arch `rise` high. */
export function archShape(w, h, rise = 0.15) {
  const shape = new THREE.Shape();
  shape.moveTo(-w / 2, 0);
  shape.lineTo(w / 2, 0);
  shape.lineTo(w / 2, h - rise);
  shape.quadraticCurveTo(w / 2, h, 0, h);
  shape.quadraticCurveTo(-w / 2, h, -w / 2, h - rise);
  shape.closePath();
  return shape;
}

/** An extruded outline, centred on its own middle. */
export function extrude(shape, depth, mat, x = 0, y = 0, z = 0, bevel = 0.012) {
  const geometry = new THREE.ExtrudeGeometry(shape, { depth, bevelEnabled: bevel > 0, bevelThickness: bevel, bevelSize: bevel, bevelSegments: 2, curveSegments: 14 });
  geometry.translate(0, 0, -depth / 2);
  const mesh = new THREE.Mesh(geometry, mat);
  mesh.position.set(x, y, z);
  return flagged(mesh);
}

/** A raised slab over an outline given as [x, z] points (steps, stoops, dais). */
export function slab(points, height, mat, y = 0) {
  const shape = new THREE.Shape();
  points.forEach(([x, z], i) => (i ? shape.lineTo(x, -z) : shape.moveTo(x, -z)));
  shape.closePath();
  const geometry = new THREE.ExtrudeGeometry(shape, { depth: height, bevelEnabled: true, bevelThickness: 0.02, bevelSize: 0.02, bevelSegments: 1 });
  geometry.rotateX(-Math.PI / 2);
  const mesh = new THREE.Mesh(geometry, mat);
  mesh.position.y = y;
  return flagged(mesh);
}

/** A tube along a curve whose radius follows `radius(t)`. */
export function tube(curve, radius, mat, tubular = 18, radial = 8) {
  const frames = curve.computeFrenetFrames(tubular, false);
  const positions = [], uvs = [], index = [];
  for (let i = 0; i <= tubular; i++) {
    const t = i / tubular;
    const p = curve.getPointAt(t);
    const n = frames.normals[i], b = frames.binormals[i];
    const r = radius(t);
    for (let j = 0; j <= radial; j++) {
      const a = (j / radial) * Math.PI * 2;
      const c = Math.cos(a), s = Math.sin(a);
      positions.push(p.x + r * (c * n.x + s * b.x), p.y + r * (c * n.y + s * b.y), p.z + r * (c * n.z + s * b.z));
      uvs.push(j / radial, t);
    }
  }
  for (let i = 0; i < tubular; i++) {
    for (let j = 0; j < radial; j++) {
      const a = i * (radial + 1) + j, b = a + radial + 1;
      index.push(a, b, a + 1, b, b + 1, a + 1);
    }
  }
  const geometry = new THREE.BufferGeometry();
  geometry.setAttribute("position", new THREE.Float32BufferAttribute(positions, 3));
  geometry.setAttribute("uv", new THREE.Float32BufferAttribute(uvs, 2));
  geometry.setIndex(index);
  geometry.computeVertexNormals();
  return flagged(new THREE.Mesh(geometry, mat));
}

/** Puts `obj` on the floor at (x, z), turned by yaw, raised by y. */
export function put(parent, obj, x, z, yaw = 0, y = 0) {
  obj.position.set(x, y, z);
  obj.rotation.y = yaw;
  parent.add(obj);
  return obj;
}

/** Puts `obj` anywhere. */
export function at(parent, obj, x, y, z, yaw = 0, pitch = 0, roll = 0) {
  obj.position.set(x, y, z);
  obj.rotation.set(pitch, yaw, roll);
  parent.add(obj);
  return obj;
}

// ------------------------------------------------------------ declarations
/** A lamp that lights the room: one of the house's fixed point lights sits on it (see main.js). */
export function lampAt(obj, x, y, z, { power = 1, color = "#ffa552", fire = false, both = false } = {}) {
  (obj.userData.lamps ??= []).push({ x, y, z, power, color, fire, both });
}

/** A glow halo drawn as a soft point. */
export function glowAt(obj, x, y, z, size = 1, color = "#ffb45a", intensity = 0.6) {
  (obj.userData.glows ??= []).push({ x, y, z, size, color, intensity });
}

/** A soft footprint on the floor under the object. */
export function shadowOf(obj, strength = 0.4, grow = 0.08) {
  obj.userData.shadow = { strength, grow };
  return obj;
}

/** A light pool on the floor or a wash on a wall. */
export function poolAt(obj, x, y, z, radius = 2, color = "#ff9a3a", intensity = 0.5, normal = "up") {
  (obj.userData.pools ??= []).push({ x, y, z, radius, color, intensity, normal });
}

/** World-space lists of everything the props declared. Call before baking. */
export function collectDecor(root) {
  root.updateMatrixWorld(true);
  const out = { lamps: [], glows: [], shadows: [], pools: [] };
  const v = new THREE.Vector3();
  root.traverse((o) => {
    const d = o.userData;
    if (d.lamps) for (const l of d.lamps) out.lamps.push({ ...l, ...toWorld(o, l, v) });
    if (d.glows) for (const g of d.glows) out.glows.push({ ...g, ...toWorld(o, g, v) });
    if (d.pools) {
      for (const p of d.pools) {
        const local = p.normal === "+z" ? [0, 0, 1] : p.normal === "-z" ? [0, 0, -1] : p.normal === "+x" ? [1, 0, 0] : p.normal === "-x" ? [-1, 0, 0] : [0, 1, 0];
        const n = new THREE.Vector3(...local).transformDirection(o.matrixWorld);
        out.pools.push({ ...p, ...toWorld(o, p, v), nx: n.x, ny: n.y, nz: n.z });
      }
    }
    if (d.shadow) {
      const b = new THREE.Box3().setFromObject(o);
      if (isFinite(b.min.x)) out.shadows.push({ x: (b.min.x + b.max.x) / 2, y: b.min.y, z: (b.min.z + b.max.z) / 2, w: b.max.x - b.min.x + d.shadow.grow * 2, d: b.max.z - b.min.z + d.shadow.grow * 2, strength: d.shadow.strength });
    }
  });
  return out;
}

function toWorld(o, p, v) {
  v.set(p.x, p.y, p.z).applyMatrix4(o.matrixWorld);
  return { x: v.x, y: v.y, z: v.z };
}
