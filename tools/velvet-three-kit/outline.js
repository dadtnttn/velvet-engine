// A thin ink line over the rendered frame, found in the depth buffer alone: silhouettes where the
// depth jumps, and creases where a plane meets another plane (1/depth is affine across a plane in
// screen space, so its second difference is zero on flat faces and spikes on their edges).
// Painted textures never draw lines: only geometry does.
import * as THREE from "three";
import { ShaderPass } from "three/addons/postprocessing/ShaderPass.js";

const OutlineShader = {
  uniforms: {
    tDiffuse: { value: null },
    tDepth: { value: null },
    resolution: { value: new THREE.Vector2(1, 1) },
    cameraNear: { value: 1 },
    cameraFar: { value: 200 },
    lineColor: { value: new THREE.Color("#3a1c0e") },
    strength: { value: 0.92 },
    thickness: { value: 1.6 },
    silhouette: { value: new THREE.Vector2(0.006, 0.02) },
    crease: { value: new THREE.Vector2(0.0022, 0.006) },
    fade: { value: new THREE.Vector2(1e5, 2e5) },
  },
  vertexShader: /* glsl */ `
    varying vec2 vUv;
    void main() {
      vUv = uv;
      gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0);
    }
  `,
  fragmentShader: /* glsl */ `
    #include <packing>
    uniform sampler2D tDiffuse;
    uniform sampler2D tDepth;
    uniform vec2 resolution;
    uniform float cameraNear;
    uniform float cameraFar;
    uniform vec3 lineColor;
    uniform float strength;
    uniform float thickness;
    uniform vec2 silhouette;
    uniform vec2 crease;
    uniform vec2 fade;
    varying vec2 vUv;

    float depthAt(vec2 uv) {
      float d = texture2D(tDepth, uv).x;
      return -perspectiveDepthToViewZ(d, cameraNear, cameraFar);
    }

    void main() {
      vec4 color = texture2D(tDiffuse, vUv);
      float raw = texture2D(tDepth, vUv).x;
      if (raw >= 0.99999) { gl_FragColor = color; return; }
      vec2 px = thickness / resolution;
      float c = depthAt(vUv);
      float l = depthAt(vUv - vec2(px.x, 0.0));
      float r = depthAt(vUv + vec2(px.x, 0.0));
      float d = depthAt(vUv - vec2(0.0, px.y));
      float u = depthAt(vUv + vec2(0.0, px.y));
      // Silhouettes: only on the near side of a jump, so the line stays one pixel wide.
      float jump = max(max(l - c, r - c), max(d - c, u - c)) / c;
      float sil = smoothstep(silhouette.x, silhouette.y, jump);
      // Creases: second difference of 1/depth, relative to it.
      float ic = 1.0 / c;
      float lap = abs(1.0 / l + 1.0 / r - 2.0 * ic) + abs(1.0 / u + 1.0 / d - 2.0 * ic);
      float cre = smoothstep(crease.x, crease.y, lap / ic) * step(jump, silhouette.y);
      float e = max(sil, cre * 0.8);
      // Lines thin out into the distance with the fog.
      e *= 1.0 - smoothstep(fade.x, fade.y, c);
      // Ink: the surface's own colour, much darker (warm, never pure black).
      vec3 ink = color.rgb * 0.2 + lineColor * 0.08;
      gl_FragColor = vec4(mix(color.rgb, ink, e * strength), color.a);
    }
  `,
};

/** The outline pass; call `sync()` before each composer.render(). */
export function createOutline(composer, camera) {
  // The render targets keep their depth so the pass can read what the RenderPass drew.
  for (const target of [composer.renderTarget1, composer.renderTarget2]) {
    target.depthTexture = new THREE.DepthTexture(target.width, target.height);
    target.depthTexture.type = THREE.UnsignedIntType;
  }
  const pass = new ShaderPass(OutlineShader);
  pass.uniforms.cameraNear.value = camera.near;
  pass.uniforms.cameraFar.value = camera.far;
  return {
    pass,
    sync(width, height) {
      pass.uniforms.tDepth.value = composer.readBuffer.depthTexture;
      if (width) pass.uniforms.resolution.value.set(width, height);
    },
  };
}
