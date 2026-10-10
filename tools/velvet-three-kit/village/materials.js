// One shared library of materials for the village. Those flagged `world` get their texture
// coordinates rebuilt from world positions when a group is baked, so stone, plaster and grain keep
// one size wherever a box happens to be. Roofs carry their own UVs (metres along the ridge and
// down the slope), so their textures repeat every 2 m by `repeat`. Glowing glass is flagged
// `glow` ("window", "lamp", "heart") so the time of day can turn it up and down.
import * as THREE from "three";
import * as T from "../textures.js";
import * as V from "./village-textures.js";

let lib = null;

/** A lit material; a map painted with relief brings its normal map along. */
const std = (color, rough = 0.85, metal = 0, extra = {}) => {
  const m = new THREE.MeshStandardMaterial({ color, roughness: rough, metalness: metal, ...extra });
  const relief = extra.map?.userData?.normal;
  if (relief) {
    m.normalMap = relief;
    const k = extra.map.userData.normalScale ?? 1;
    m.normalScale.set(k, k);
  }
  return m;
};

/** Marks a material as world-mapped: `scale` metres per texture tile; `macro` a slow tone drift. */
export function world(material, scale = 2, macro = 0) {
  material.userData.worldUV = true;
  material.userData.uvScale = scale;
  material.userData.macro = macro;
  return material;
}

/** A material whose UVs are in metres: its texture repeats every `tile` metres. */
function metric(material, tile = 2) {
  for (const t of [material.map, material.normalMap]) if (t) t.repeat.set(1 / tile, 1 / tile);
  return material;
}

const glow = (material, kind) => {
  material.userData.glow = kind;
  return material;
};

export function tinted(material, color, extra = {}) {
  const m = material.clone();
  m.color = new THREE.Color(color);
  Object.assign(m, extra);
  return m;
}

const roof = (kind, seed) => metric(std("#ffffff", 0.82, 0, { map: V.roofingTexture(kind, seed) }), 2);

export function mats() {
  if (lib) return lib;
  const postTex = T.woodTexture({ base: [74, 44, 26], boards: 3, seed: 9 });
  const oakTex = T.woodTexture({ base: [160, 110, 66], boards: 4, seed: 15 });
  const greyWood = T.woodTexture({ base: [128, 110, 92], boards: 4, seed: 23 });
  const deck = T.plankTexture({ base: [168, 122, 78], seed: 19 });
  const stone = V.stoneWallTexture();
  const stoneWarm = V.stoneWallTexture({ base: [190, 168, 136], seed: 27 });
  const lime = V.limePlasterTexture();
  const rock = V.rockTexture();
  const leaded = V.leadedGlassTexture();
  const shutter = V.shutterTexture();

  lib = {
    // Ground.
    grass: world(std("#ffffff", 1, 0, { map: V.grassTexture() }), 4, 0.22),
    grassDark: world(std("#c6d8b4", 1, 0, { map: V.grassTexture({ seed: 9 }) }), 4, 0.22),
    // The streets, the lanes and the plaza are one paving in world space: the same stones, one
    // size, rows running east-west everywhere (2 m a tile; see terrain.js).
    paving: world(std("#ffffff", 0.92, 0, { map: V.cobbleTexture() }), 2, 0.1),
    kerb: world(std("#e8e2d4", 0.95, 0, { map: stoneWarm }), 1.2, 0.1),
    soil: world(std("#6b4a2e", 1, 0, { map: V.cobbleTexture({ base: [110, 78, 48], mortar: [70, 48, 30], seed: 51, rows: 22 }) }), 1.5),
    // Walls: lime plaster between the timbers, each house its own wash.
    plaster: world(std("#f6e6c8", 0.95, 0, { map: lime }), 3, 0.1),
    plasterWhite: world(std("#fdf6ea", 0.95, 0, { map: lime }), 3, 0.1),
    plasterRose: world(std("#f4c6c4", 0.95, 0, { map: lime }), 3, 0.1),
    plasterSky: world(std("#d8e6f0", 0.95, 0, { map: lime }), 3, 0.1),
    plasterSage: world(std("#dee8c6", 0.95, 0, { map: lime }), 3, 0.1),
    plasterOchre: world(std("#f4daa4", 0.95, 0, { map: lime }), 3, 0.1),
    stone: world(std("#ffffff", 0.95, 0, { map: stone }), 2, 0.14),
    stoneWarm: world(std("#ffffff", 0.95, 0, { map: stoneWarm }), 2, 0.14),
    // Dressed stones (plinths, chimneys, steps, arches), three of them so neighbours differ, and
    // the dark mortar between them.
    rock: world(std("#e8dfcc", 0.95, 0, { map: rock }), 1.1, 0.06),
    rock2: world(std("#cdc2ac", 0.95, 0, { map: rock }), 1.1, 0.06),
    rock3: world(std("#b2afa2", 0.95, 0, { map: rock }), 1.1, 0.06),
    mortar: std("#5c5046", 1),
    moss: world(std("#ffffff", 1, 0, { map: V.foliageTexture({ dark: "#3a5a26", mid: "#5f8f36", light: "#a4c866", seed: 47 }) }), 0.9),
    // Timber.
    timber: world(std("#ffffff", 0.82, 0, { map: postTex }), 2, 0.12),
    timberH: world(std("#ffffff", 0.82, 0, { map: T.turned(postTex) }), 2, 0.12),
    oak: world(std("#ffffff", 0.7, 0, { map: oakTex }), 2, 0.12),
    oakH: world(std("#ffffff", 0.7, 0, { map: T.turned(oakTex) }), 2, 0.12),
    weathered: world(std("#ffffff", 0.9, 0, { map: greyWood }), 2, 0.12),
    weatheredH: world(std("#ffffff", 0.9, 0, { map: T.turned(greyWood) }), 2, 0.12),
    deck: world(std("#ffffff", 0.8, 0, { map: deck }), 4, 0.15),
    door: std("#ffffff", 0.78, 0, { map: T.doorTexture() }),
    doorPlank: std("#ffffff", 0.8, 0, { map: V.plankDoorTexture() }),
    // Roofs.
    roofShake: roof("shake", 31),
    roofShakeDark: roof("shakeDark", 33),
    roofMoss: roof("moss", 35),
    roofTeal: roof("teal", 37),
    roofRed: roof("red", 39),
    roofSlate: roof("slate", 41),
    ridge: std("#5a3a24", 0.8),
    ridgeMoss: std("#4c5c2a", 0.85),
    ridgeTeal: std("#2c6a62", 0.8),
    ridgeRed: std("#76261f", 0.8),
    // Shutters: painted planks.
    shutterGreen: std("#4a8a5e", 0.7, 0, { map: shutter }),
    shutterBlue: std("#4a7aa8", 0.7, 0, { map: shutter }),
    shutterRed: std("#aa3c40", 0.7, 0, { map: shutter }),
    shutterOchre: std("#d09a44", 0.7, 0, { map: shutter }),
    // Metal, glass, light.
    iron: std("#3d3b42", 0.5, 0.6),
    brass: std("#d8aa4c", 0.32, 0.85),
    // Window glass in small leaded panes: the panes glow at night, the lead does not.
    windowGlow: glow(std("#ffcf8a", 0.4, 0, { map: leaded, emissive: new THREE.Color("#ff9c40"), emissiveMap: leaded, emissiveIntensity: 0.9 }), "window"),
    windowRose: glow(std("#ffb0a8", 0.4, 0, { map: leaded, emissive: new THREE.Color("#ff5a6a"), emissiveMap: leaded, emissiveIntensity: 0.9 }), "window"),
    lampGlass: glow(std("#ffe0a0", 0.35, 0, { emissive: new THREE.Color("#ffa24a"), emissiveIntensity: 1.6 }), "lamp"),
    heartGlass: glow(std("#ff8a9a", 0.35, 0, { emissive: new THREE.Color("#ff2a4a"), emissiveIntensity: 1.6 }), "lamp"),
    paperLamp: glow(std("#fff0d0", 0.6, 0, { emissive: new THREE.Color("#ffb060"), emissiveIntensity: 1.2 }), "lamp"),
    flame: new THREE.MeshBasicMaterial({ color: "#ffb04a" }),
    // Cloth, foliage, food.
    awningRed: std("#ffffff", 0.95, 0, { map: V.awningTexture(), alphaTest: 0.5, side: THREE.DoubleSide }),
    awningGreen: std("#ffffff", 0.95, 0, { map: V.awningTexture("#2f7a4a", "#f4e6c8"), alphaTest: 0.5, side: THREE.DoubleSide }),
    noren: std("#ffffff", 0.95, 0, { map: V.norenTexture(), alphaTest: 0.5, side: THREE.DoubleSide }),
    banner: std("#ffffff", 0.9, 0, { map: V.councilBannerTexture(), alphaTest: 0.5, side: THREE.DoubleSide }),
    clock: std("#ffffff", 0.6, 0, { map: V.clockTexture(), alphaTest: 0.5 }),
    gateSign: std("#ffffff", 0.8, 0, { map: V.gateSignTexture() }),
    curtainRed: std("#8a1c2c", 0.95),
    linen: std("#f2e8d2", 0.95),
    cloth: ["#c9584a", "#e8d6a8", "#5a7ab0", "#7a9a5a", "#d8a0b8"].map((c) => std(c, 0.95, 0, { side: THREE.DoubleSide })),
    hedge: world(std("#ffffff", 0.9, 0, { map: V.foliageTexture() }), 1.2),
    vine: std("#ffffff", 0.9, 0, { map: V.foliageTexture({ dark: "#2a5a2a", mid: "#3f8638", light: "#7cb850", seed: 43 }) }),
    ivy: std("#ffffff", 0.85, 0, { map: T.leafTexture({ dark: "#24582c", mid: "#3c8a3c", light: "#86c060", seed: 9 }), alphaTest: 0.5, side: THREE.DoubleSide }),
    hay: world(std("#e6c46a", 1, 0, { map: T.woodTexture({ base: [214, 178, 92], boards: 12, seed: 61 }) }), 1),
    log: std("#7a5232", 0.95),
    logEnd: std("#d8b07a", 0.9),
    apple: std("#c8302a", 0.5),
    pear: std("#b8c040", 0.6),
    pumpkin: std("#e07a24", 0.7),
    cabbage: std("#6aa848", 0.8),
    bread: std("#c88a44", 0.8),
    flowers: ["#f4f0e4", "#f2c84a", "#e8566a", "#b77ad8", "#ff9a3a", "#f08cc0", "#6a8af0"].map((c) => std(c, 0.7)),
    leafGreen: std("#4a8c3a", 0.8),
    potClay: std("#b8683c", 0.85),
    paper: std("#efe2bc", 0.95),
    water: new THREE.MeshStandardMaterial({ color: "#3a7a8a", roughness: 0.15, metalness: 0.1 }),
    rope: std("#b89a6a", 0.95),
    gold: std("#e2b85a", 0.35, 0.8),
  };
  // What lies on the ground never hides her: the see-through window leaves it alone.
  for (const k of ["grass", "grassDark", "paving", "kerb", "soil", "water"]) lib[k].userData.occluder = false;
  return lib;
}
