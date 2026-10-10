// Toon shading for the whole house (the same art direction as velvet-render's `ToonStyle3D`):
// N·L is cut into four broad levels with softened edges, the dark side takes a cool tint, a small
// shaped highlight replaces glossy specular, and a soft rim lifts silhouettes. Painted textures,
// vertex AO and emissive stay as they were, so wood grain and stone joints keep their detail.
import * as THREE from "three";

/**
 * The see-through window round the Queen: whatever stands between the camera and her (a roof, a
 * tree) is dithered away inside a soft circle on screen centred on her, as the game does for
 * walls. `focus`: her position in normalised device coordinates and window depth (z); `radius`:
 * the circle's radius in NDC height; `aspect`: viewport width / height.
 */
export const occlusion = {
  focus: { value: new THREE.Vector3(0, 0, 1) },
  radius: { value: 0.0 },
  aspect: { value: 16 / 9 },
  viewport: { value: new THREE.Vector2(1, 1) },
};

/** Shared controls: every toon material reads these, so they can be tuned live. */
export const toonStyle = {
  softness: { value: 0.035 }, // width of the band edges, in cosine space
  shadowTint: { value: new THREE.Color(0.8, 0.87, 1.0) }, // multiplies the dark side of the ramp
  rim: { value: 0.32 }, // soft rim, scaled by how lit the surface already is
  rimColor: { value: new THREE.Color("#ffc890") },
  aoKeep: 0.55, // how much of a baked AO map is kept (softer AO)
};

const PARS = /* glsl */ `
uniform vec3 uOccFocus;
uniform float uOccRadius;
uniform float uOccAspect;
uniform vec2 uOccViewport;
uniform float uOccluder;
float occBayer( vec2 p ) {
  vec2 q = mod( floor( p ), 4.0 );
  float b = q.x + q.y * 4.0;
  // 4x4 Bayer matrix, row-major.
  if ( b < 0.5 ) return 0.0 / 16.0; if ( b < 1.5 ) return 8.0 / 16.0; if ( b < 2.5 ) return 2.0 / 16.0; if ( b < 3.5 ) return 10.0 / 16.0;
  if ( b < 4.5 ) return 12.0 / 16.0; if ( b < 5.5 ) return 4.0 / 16.0; if ( b < 6.5 ) return 14.0 / 16.0; if ( b < 7.5 ) return 6.0 / 16.0;
  if ( b < 8.5 ) return 3.0 / 16.0; if ( b < 9.5 ) return 11.0 / 16.0; if ( b < 10.5 ) return 1.0 / 16.0; if ( b < 11.5 ) return 9.0 / 16.0;
  if ( b < 12.5 ) return 15.0 / 16.0; if ( b < 13.5 ) return 7.0 / 16.0; if ( b < 14.5 ) return 13.0 / 16.0; return 5.0 / 16.0;
}
void occlusionCut() {
  if ( uOccluder < 0.5 || uOccRadius <= 0.0 ) return;
  if ( gl_FragCoord.z >= uOccFocus.z ) return; // only what stands in front of her
  vec2 ndc = gl_FragCoord.xy / uOccViewport * 2.0 - 1.0;
  vec2 d = ndc - uOccFocus.xy;
  d.x *= uOccAspect;
  float r = length( d ) / uOccRadius;
  if ( r >= 1.0 ) return;
  float keep = smoothstep( 0.55, 1.0, r );
  if ( occBayer( gl_FragCoord.xy ) >= keep * 0.94 + 0.04 ) discard;
}
uniform float uToonSoft;
uniform vec3 uShadowTint;
uniform float uSpec;
uniform float uRim;
uniform vec3 uRimColor;
uniform float uSheen;
uniform float uMacro;
varying vec3 vToonWorld;
float toonHash( vec2 p ) { return fract( sin( dot( p, vec2( 127.1, 311.7 ) ) ) * 43758.5453 ); }
float toonNoise( vec2 p ) {
  vec2 i = floor( p ), f = fract( p );
  vec2 u = f * f * ( 3.0 - 2.0 * f );
  return mix( mix( toonHash( i ), toonHash( i + vec2( 1.0, 0.0 ) ), u.x ), mix( toonHash( i + vec2( 0.0, 1.0 ) ), toonHash( i + vec2( 1.0, 1.0 ) ), u.x ), u.y );
}
float toonEdge( float threshold, float value ) {
  float w = max( uToonSoft, 0.001 );
  float t = clamp( ( value - threshold + w ) / ( 2.0 * w ), 0.0, 1.0 );
  return t * t * ( 3.0 - 2.0 * t );
}
// Four broad levels; kept in sync with ToonStyle3D::diffuse in velvet-render.
float toonRamp( float cosine ) {
  float x = clamp( cosine, 0.0, 1.0 );
  return 0.08 + 0.28 * toonEdge( 0.22, x ) + 0.31 * toonEdge( 0.55, x ) + 0.33 * toonEdge( 0.85, x );
}
vec3 getGradientIrradiance( vec3 normal, vec3 lightDirection ) {
  float level = toonRamp( dot( normal, lightDirection ) );
  return level * mix( uShadowTint, vec3( 1.0 ), level );
}
`;

const DIRECT = "reflectedLight.directDiffuse += irradiance * BRDF_Lambert( material.diffuseColor );";
const DIRECT_TOON = /* glsl */ `
  reflectedLight.directDiffuse += irradiance * BRDF_Lambert( material.diffuseColor );
  // A small shaped highlight instead of glossy specular.
  vec3 halfDir = normalize( directLight.direction + geometryViewDir );
  float lit = step( 0.0, dot( geometryNormal, directLight.direction ) );
  reflectedLight.directDiffuse += directLight.color * toonEdge( 0.94, saturate( dot( geometryNormal, halfDir ) ) ) * lit * uSpec * 0.25;
`;

const RIM = /* glsl */ `
  {
    float facing = saturate( dot( normal, normalize( vViewPosition ) ) );
    float rimF = toonEdge( 0.62, 1.0 - facing ) * 0.6 + pow( 1.0 - facing, 3.0 ) * 0.4;
    vec3 lit = reflectedLight.directDiffuse + reflectedLight.indirectDiffuse;
    outgoingLight += rimF * uRim * ( lit + uRimColor * 0.04 );
    // Cloth: velvet pile catches the light at grazing angles and looks deeper face-on.
    if ( uSheen > 0.0 ) {
      float graze = pow( 1.0 - facing, 1.6 );
      float luma = dot( lit, vec3( 0.3, 0.59, 0.11 ) );
      outgoingLight *= 1.0 - 0.18 * uSheen * facing * facing;
      outgoingLight += toonEdge( 0.45, graze ) * uSheen * ( diffuseColor.rgb * 1.4 + 0.06 ) * ( 0.25 + luma * 1.6 );
    }
  }
  #include <opaque_fragment>
`;

// Large-scale tone drift over the world (two octaves of value noise), so tiles never read as tiles.
const MACRO = /* glsl */ `
  #include <map_fragment>
  if ( uMacro > 0.0 ) {
    vec2 q = vToonWorld.xz + vec2( vToonWorld.y * 0.7, vToonWorld.y * 0.35 );
    float n = toonNoise( q * 0.42 ) * 0.65 + toonNoise( q * 1.35 + 17.0 ) * 0.35;
    diffuseColor.rgb *= 1.0 + uMacro * ( n - 0.5 ) * 2.0;
  }
`;

function patch(material, spec, sheen = 0, macro = 0, occluder = 1) {
  material.onBeforeCompile = (shader) => {
    shader.uniforms.uOccFocus = occlusion.focus;
    shader.uniforms.uOccRadius = occlusion.radius;
    shader.uniforms.uOccAspect = occlusion.aspect;
    shader.uniforms.uOccViewport = occlusion.viewport;
    shader.uniforms.uOccluder = { value: occluder };
    shader.uniforms.uToonSoft = toonStyle.softness;
    shader.uniforms.uShadowTint = toonStyle.shadowTint;
    shader.uniforms.uRim = toonStyle.rim;
    shader.uniforms.uRimColor = toonStyle.rimColor;
    shader.uniforms.uSpec = { value: spec };
    shader.uniforms.uSheen = { value: sheen };
    shader.uniforms.uMacro = { value: macro };
    shader.vertexShader = shader.vertexShader
      .replace("#include <common>", "#include <common>\nvarying vec3 vToonWorld;")
      .replace("#include <project_vertex>", "#include <project_vertex>\n  vToonWorld = ( modelMatrix * vec4( transformed, 1.0 ) ).xyz;");
    const lights = THREE.ShaderChunk.lights_toon_pars_fragment.replace(DIRECT, DIRECT_TOON);
    shader.fragmentShader = shader.fragmentShader
      .replace("#include <gradientmap_pars_fragment>", PARS)
      .replace("#include <lights_toon_pars_fragment>", lights)
      .replace("#include <opaque_fragment>", RIM)
      .replace("#include <map_fragment>", MACRO)
      .replace("void main() {", "void main() {\n\tocclusionCut();");
  };
  material.customProgramCacheKey = () => `toon-${spec}-${sheen}-${macro}-${occluder}`;
  return material;
}

/** How much shaped highlight a material gets: metal and glass shine, cloth and plaster do not. */
function specOf(src) {
  if ((src.metalness ?? 0) > 0.5) return 0.9;
  if ((src.roughness ?? 1) < 0.35) return 0.6;
  if ((src.roughness ?? 1) < 0.7) return 0.25;
  return 0.0;
}

const cache = new Map();
const sources = new WeakMap();

/** The lit material a toon material was made from (for exporting), or the material itself. */
export function sourceOf(material) {
  return sources.get(material) ?? material;
}

/** The toon version of a lit material (MeshStandard / MeshPhysical / MeshLambert / MeshPhong). */
export function toonOf(src) {
  if (!src || src.isMeshToonMaterial || src.isShaderMaterial || src.isMeshBasicMaterial || src.isPointsMaterial) return src;
  if (!(src.isMeshStandardMaterial || src.isMeshLambertMaterial || src.isMeshPhongMaterial)) return src;
  if (cache.has(src)) return cache.get(src);
  const m = new THREE.MeshToonMaterial({
    name: src.name,
    color: src.color,
    map: src.map,
    emissive: src.emissive,
    emissiveMap: src.emissiveMap,
    emissiveIntensity: src.emissiveIntensity,
    aoMap: src.aoMap,
    normalMap: src.normalMap,
    normalScale: src.normalScale,
    aoMapIntensity: (src.aoMapIntensity ?? 1) * toonStyle.aoKeep,
    alphaMap: src.alphaMap,
    alphaTest: src.alphaTest,
    transparent: src.transparent,
    opacity: src.opacity,
    side: src.side,
    vertexColors: src.vertexColors,
    depthWrite: src.depthWrite,
    polygonOffset: src.polygonOffset,
    polygonOffsetFactor: src.polygonOffsetFactor,
    polygonOffsetUnits: src.polygonOffsetUnits,
  });
  m.userData = { ...src.userData, toonFrom: src.uuid };
  patch(m, specOf(src), src.userData.sheen ?? 0, src.userData.macro ?? 0, src.userData.occluder === false ? 0 : 1);
  cache.set(src, m);
  sources.set(m, src);
  return m;
}

/** Swaps every lit material under `root` for its toon version (call after baking). */
export function toonify(root) {
  root.traverse((o) => {
    if (!o.isMesh && !o.isSkinnedMesh) return;
    o.material = Array.isArray(o.material) ? o.material.map(toonOf) : toonOf(o.material);
  });
}
