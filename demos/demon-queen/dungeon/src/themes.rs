//! Native cave art direction. This layer reads the validated blueprint but
//! never changes it: art streams cannot move doors, spawns or navigation.
use velvet_dungeon::{rng::Rng, Biome, Blueprint, Role, Room, Shape};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Theme {
    Threshold,
    Return,
    Seal,
    Memory,
    Lair,
    Guardpost,
    Custodian,
    Heart,
    Crypt,
    Mine,
    Fungal,
    Forge,
    Library,
    Overgrowth,
}

impl Theme {
    pub fn title(self) -> &'static str {
        match self {
            Self::Threshold => "Campamento del umbral",
            Self::Return => "Puerta del retorno",
            Self::Seal => "Cámara del sello",
            Self::Memory => "Memorial de piedra",
            Self::Lair => "Guarida",
            Self::Guardpost => "Cuerpo de guardia",
            Self::Custodian => "Arena del Custodio",
            Self::Heart => "Corazón de la caverna",
            Self::Crypt => "Cripta",
            Self::Mine => "Mina abandonada",
            Self::Fungal => "Gruta de hongos",
            Self::Forge => "Forja en ruinas",
            Self::Library => "Biblioteca hundida",
            Self::Overgrowth => "Raíces anegadas",
        }
    }
}

const GALLERIES: [[Theme; 6]; 3] = [
    [
        Theme::Crypt,
        Theme::Library,
        Theme::Mine,
        Theme::Overgrowth,
        Theme::Fungal,
        Theme::Forge,
    ],
    [
        Theme::Fungal,
        Theme::Overgrowth,
        Theme::Crypt,
        Theme::Library,
        Theme::Mine,
        Theme::Forge,
    ],
    [
        Theme::Forge,
        Theme::Mine,
        Theme::Crypt,
        Theme::Fungal,
        Theme::Library,
        Theme::Overgrowth,
    ],
];

/// Same semantic mapping and per-biome gallery cycle as the Three.js caves.
/// Counts only preceding ordinary chambers, so adding decoration or gameplay
/// cannot reshuffle later rooms.
pub fn of(map: &Blueprint, room: &Room) -> Option<Theme> {
    if room.shape == Shape::Bridge
        || matches!(room.role, Role::Treasure | Role::Secret | Role::Rest)
    {
        return None;
    }
    let fixed = match room.role {
        Role::Entrance => Some(Theme::Threshold),
        Role::Exit => Some(Theme::Return),
        Role::Puzzle => Some(Theme::Seal),
        Role::Event => Some(Theme::Memory),
        Role::Encounter => Some(Theme::Lair),
        Role::Elite => Some(Theme::Guardpost),
        Role::Miniboss => Some(Theme::Custodian),
        Role::Boss => Some(Theme::Heart),
        _ => None,
    };
    if fixed.is_some() {
        return fixed;
    }
    if room.role != Role::Normal {
        return None;
    }
    let biome = room.biome % GALLERIES.len();
    let ordinal = map
        .rooms
        .iter()
        .take(room.id)
        .filter(|r| {
            r.role == Role::Normal && r.shape != Shape::Bridge && r.biome % GALLERIES.len() == biome
        })
        .count();
    let offset = (map.seed as u32 % 6) as usize;
    Some(GALLERIES[biome][(ordinal + offset) % 6])
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Wear {
    pub moss: f32,
    pub damp: f32,
    pub streaks: f32,
    pub soot: f32,
    pub salt: f32,
    pub heat: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Finish {
    pub theme: Theme,
    pub stone: [f32; 3],
    pub floor: [f32; 3],
    pub wood: [f32; 3],
    pub wall_cell: [f32; 2],
    pub wall_relief: f32,
    pub masonry: bool,
    pub floor_cell: [f32; 2],
    pub floor_relief: f32,
    pub floor_masonry: bool,
    pub ceiling_cell: [f32; 2],
    pub ceiling_relief: f32,
    pub timber_ceiling: bool,
    pub wear: Wear,
}

fn mix(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    std::array::from_fn(|i| a[i] * (1.0 - t) + b[i] * t)
}
fn mul(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    std::array::from_fn(|i| a[i] * b[i])
}

#[derive(Clone, Copy)]
struct Recipe {
    stone: [f32; 3],
    floor: [f32; 3],
    wall: &'static [u8],
    paving: &'static [u8],
    ceiling: &'static [u8],
    wear: Wear,
}
const fn w(moss: f32, damp: f32, streaks: f32, soot: f32, salt: f32, heat: f32) -> Wear {
    Wear {
        moss,
        damp,
        streaks,
        soot,
        salt,
        heat,
    }
}
fn recipe(t: Theme) -> Recipe {
    use Theme::*;
    match t {
        Threshold => Recipe {
            stone: [0.46, 0.38, 0.29],
            floor: [0.36, 0.30, 0.24],
            wall: &[1, 0, 3],
            paving: &[0, 1, 2],
            ceiling: &[0, 1, 2],
            wear: w(0.4, 0.25, 0.2, 0.15, 0., 0.),
        },
        Return => Recipe {
            stone: [0.30, 0.36, 0.38],
            floor: [0.24, 0.28, 0.30],
            wall: &[5, 0],
            paving: &[1, 7, 5],
            ceiling: &[1, 3],
            wear: w(0.3, 0.3, 0.35, 0., 0.2, 0.),
        },
        Seal => Recipe {
            stone: [0.46, 0.43, 0.50],
            floor: [0.50, 0.48, 0.52],
            wall: &[0, 5],
            paving: &[5, 7],
            ceiling: &[3, 1],
            wear: w(0.1, 0.15, 0.2, 0., 0.3, 0.),
        },
        Memory => Recipe {
            stone: [0.42, 0.46, 0.50],
            floor: [0.30, 0.33, 0.36],
            wall: &[5, 4],
            paving: &[1, 0],
            ceiling: &[0, 1],
            wear: w(0.8, 0.4, 0.5, 0., 0., 0.),
        },
        Lair => Recipe {
            stone: [0.24, 0.20, 0.17],
            floor: [0.20, 0.16, 0.12],
            wall: &[4, 3, 5, 0],
            paving: &[3, 2, 0],
            ceiling: &[0, 1],
            wear: w(0.5, 0.4, 0.4, 0., 0., 0.),
        },
        Guardpost => Recipe {
            stone: [0.40, 0.40, 0.42],
            floor: [0.30, 0.30, 0.32],
            wall: &[1, 2, 6],
            paving: &[7, 0, 4],
            ceiling: &[2, 1],
            wear: w(0.15, 0.2, 0., 0.5, 0., 0.),
        },
        Custodian => Recipe {
            stone: [0.16, 0.15, 0.16],
            floor: [0.14, 0.12, 0.12],
            wall: &[5, 4],
            paving: &[1, 2],
            ceiling: &[0],
            wear: w(0., 0., 0., 0.5, 0., 0.5),
        },
        Heart => Recipe {
            stone: [0.30, 0.12, 0.12],
            floor: [0.20, 0.08, 0.08],
            wall: &[4],
            paving: &[1, 2],
            ceiling: &[0],
            wear: w(0., 0.5, 0.6, 0., 0., 0.8),
        },
        Crypt => Recipe {
            stone: [0.33, 0.31, 0.36],
            floor: [0.26, 0.25, 0.28],
            wall: &[3, 0],
            paving: &[0, 1],
            ceiling: &[1],
            wear: w(0.3, 0.5, 0.5, 0., 0.5, 0.),
        },
        Mine => Recipe {
            stone: [0.38, 0.28, 0.19],
            floor: [0.28, 0.21, 0.15],
            wall: &[4],
            paving: &[3],
            ceiling: &[0],
            wear: w(0.1, 0.3, 0.4, 0.2, 0., 0.),
        },
        Fungal => Recipe {
            stone: [0.24, 0.30, 0.30],
            floor: [0.18, 0.23, 0.22],
            wall: &[3, 4],
            paving: &[0, 2, 3],
            ceiling: &[0],
            wear: w(1., 0.8, 0.8, 0., 0., 0.),
        },
        Forge => Recipe {
            stone: [0.42, 0.20, 0.14],
            floor: [0.26, 0.19, 0.16],
            wall: &[2, 6],
            paving: &[6, 1],
            ceiling: &[2, 1],
            wear: w(0., 0., 0., 1., 0., 0.4),
        },
        Library => Recipe {
            stone: [0.50, 0.40, 0.28],
            floor: [0.36, 0.22, 0.12],
            wall: &[7, 6],
            paving: &[4, 5],
            ceiling: &[3, 2],
            wear: w(0.2, 0.45, 0.3, 0., 0., 0.),
        },
        Overgrowth => Recipe {
            stone: [0.30, 0.33, 0.27],
            floor: [0.24, 0.26, 0.20],
            wall: &[0, 3],
            paving: &[0, 3],
            ceiling: &[0],
            wear: w(1., 0.7, 0.6, 0., 0., 0.),
        },
    }
}

/// Bond IDs parallel `art/finishes.js`: ashlar, large ashlar, brick,
/// rubble, rock, cyclopean, plaster, panelled.
const WALL_BONDS: [([f32; 2], f32, bool); 8] = [
    ([0.95, 0.46], 0.24, true),
    ([1.5, 0.74], 0.30, true),
    ([0.5, 0.21], 0.13, true),
    ([0.6, 0.44], 0.26, false),
    ([1.8, 1.1], 0.45, false),
    ([1.45, 1.2], 0.30, false),
    ([0.5, 0.21], 0.13, true),
    ([0.95, 0.46], 0.24, true),
];
const FLOOR_BONDS: [([f32; 2], f32, bool); 8] = [
    ([1.35, 1.12], 0.055, false),
    ([2.1, 1.7], 0.06, false),
    ([0.5, 0.42], 0.07, false),
    ([1.6, 1.3], 0.028, false),
    ([2.4, 0.26], 0.035, true),
    ([0.95, 0.95], 0.04, true),
    ([0.6, 0.28], 0.045, true),
    ([1., 0.62], 0.05, true),
];
const CEILING_BONDS: [([f32; 2], f32, bool); 4] = [
    ([2.25, 1.8], 0.68, false),
    ([1.4, 0.7], 0.30, false),
    ([2.8, 0.3], 0.06, true),
    ([1.3, 1.3], 0.34, false),
];

/// The ageing a room's finish draws first (web `finishOf(...).condition`):
/// 0 flooded, 1 scorched, 2 ancient, 3 overgrown, 4 pristine.
pub fn condition(map: &Blueprint, room: &Room) -> Option<usize> {
    of(map, room)?;
    // Web `finishOf`: its art stream draws the shade, then the condition.
    let mut rng = super::art::kit::ArtRng::stream(map.seed, "room_finish_v1", room.id as u64);
    rng.range(0.88, 1.12);
    if rng.chance(0.65) {
        Some(rng.index(5))
    } else {
        None
    }
}

pub fn finish(map: &Blueprint, room: &Room, biome: &Biome) -> Option<Finish> {
    let theme = of(map, room)?;
    let rec = recipe(theme);
    let mut rng = Rng::stream(map.seed, "room_finish_v1", room.id as u64);
    // The ageing is the web prototype's (the incidents follow it too).
    if rng.chance(0.65) {
        rng.index(5);
    }
    let condition = self::condition(map, room);
    let shade = rng.range(0.88, 1.12);
    let drift = |rng: &mut Rng| {
        [
            rng.range(0.9, 1.1),
            rng.range(0.92, 1.08),
            rng.range(0.9, 1.1),
        ]
    };
    let mut stone = mul(mix(rec.stone, biome.wall, 0.3), drift(&mut rng));
    let mut floor = mul(mix(rec.floor, biome.floor, 0.3), drift(&mut rng));
    for c in [&mut stone, &mut floor] {
        for v in c {
            *v *= shade;
        }
    }
    if let Some(i) = condition {
        let tint = match i {
            0 => [0.86, 0.9, 0.96],
            1 => [0.76, 0.72, 0.7],
            _ => [1., 1., 1.],
        };
        stone = mul(stone, tint);
        floor = mul(floor, tint);
        if i == 2 {
            stone = mix(stone, [0.55, 0.55, 0.56], 0.22);
            floor = mix(floor, [0.55, 0.55, 0.56], 0.22);
        }
        if i == 3 {
            stone = mix(stone, [0.3, 0.42, 0.25], 0.14);
            floor = mix(floor, [0.3, 0.42, 0.25], 0.14);
        }
    }
    let wi = rec.wall[rng.index(rec.wall.len())] as usize;
    let fi = rec.paving[rng.index(rec.paving.len())] as usize;
    let ci = rec.ceiling[rng.index(rec.ceiling.len())] as usize;
    let wall = WALL_BONDS[wi];
    let paving = FLOOR_BONDS[fi];
    let ceiling = CEILING_BONDS[ci];
    let mut wear = rec.wear;
    for v in [
        &mut wear.moss,
        &mut wear.damp,
        &mut wear.streaks,
        &mut wear.soot,
        &mut wear.salt,
        &mut wear.heat,
    ] {
        *v = (*v * rng.range(0.75, 1.25)).clamp(0., 1.);
    }
    if let Some(i) = condition {
        let extra = match i {
            0 => w(0.25, 0.5, 0.5, -0.4, 0., 0.),
            1 => w(-0.5, -0.2, 0., 0.7, 0., 0.15),
            2 => w(0.2, 0., 0., 0., 0.5, 0.),
            3 => w(0.6, 0.3, 0., 0., 0., 0.),
            _ => w(0., 0., -0.3, -0.3, 0., 0.),
        };
        wear.moss = (wear.moss + extra.moss).clamp(0., 1.);
        wear.damp = (wear.damp + extra.damp).clamp(0., 1.);
        wear.streaks = (wear.streaks + extra.streaks).clamp(0., 1.);
        wear.soot = (wear.soot + extra.soot).clamp(0., 1.);
        wear.salt = (wear.salt + extra.salt).clamp(0., 1.);
        wear.heat = (wear.heat + extra.heat).clamp(0., 1.);
    }
    Some(Finish {
        theme,
        stone,
        floor,
        wood: mul([0.34, 0.20, 0.11], drift(&mut rng)),
        wall_cell: wall.0,
        wall_relief: wall.1,
        masonry: wall.2,
        floor_cell: paving.0,
        floor_relief: paving.1,
        floor_masonry: paving.2,
        ceiling_cell: ceiling.0,
        ceiling_relief: ceiling.1,
        timber_ceiling: ceiling.2,
        wear,
    })
}

pub fn mood(theme: Theme, mut mood: super::surfaces::Mood) -> super::surfaces::Mood {
    use Theme::*;
    let (flame, glow, ambient, fog) = match theme {
        Heart => (
            Some([1., 0.22, 0.14]),
            Some([1., 0.18, 0.14]),
            Some([0.16, 0.035, 0.045]),
            Some([0.07, 0.012, 0.018]),
        ),
        Custodian => (
            Some([1., 0.46, 0.14]),
            Some([1., 0.42, 0.14]),
            Some([0.16, 0.08, 0.05]),
            Some([0.06, 0.03, 0.02]),
        ),
        Guardpost => (
            Some([1., 0.56, 0.24]),
            None,
            Some([0.1, 0.1, 0.12]),
            Some([0.03, 0.035, 0.045]),
        ),
        Lair => (
            None,
            Some([0.5, 0.9, 0.3]),
            Some([0.07, 0.1, 0.06]),
            Some([0.03, 0.045, 0.025]),
        ),
        Seal => (
            None,
            Some([0.66, 0.4, 1.]),
            Some([0.1, 0.08, 0.16]),
            Some([0.04, 0.03, 0.07]),
        ),
        Memory => (
            None,
            Some([0.46, 0.7, 1.]),
            Some([0.08, 0.11, 0.16]),
            Some([0.03, 0.045, 0.07]),
        ),
        Threshold => (Some([1., 0.6, 0.26]), None, Some([0.14, 0.1, 0.07]), None),
        Return => (
            None,
            Some([0.31, 0.89, 0.82]),
            Some([0.06, 0.13, 0.13]),
            Some([0.02, 0.05, 0.05]),
        ),
        Crypt => (
            Some([0.7, 0.6, 1.]),
            Some([0.6, 0.5, 1.]),
            Some([0.08, 0.07, 0.12]),
            Some([0.03, 0.025, 0.05]),
        ),
        Mine => (Some([1., 0.66, 0.3]), None, Some([0.12, 0.09, 0.06]), None),
        Fungal => (
            None,
            Some([0.3, 1., 0.85]),
            Some([0.05, 0.12, 0.12]),
            Some([0.02, 0.06, 0.06]),
        ),
        Forge => (
            Some([1., 0.45, 0.12]),
            None,
            Some([0.16, 0.08, 0.04]),
            Some([0.06, 0.03, 0.015]),
        ),
        Library => (Some([1., 0.7, 0.4]), None, Some([0.13, 0.1, 0.08]), None),
        Overgrowth => (
            None,
            Some([0.5, 1., 0.5]),
            Some([0.06, 0.11, 0.07]),
            Some([0.025, 0.05, 0.03]),
        ),
    };
    if let Some(c) = flame {
        mood.flame = c;
    }
    if let Some(c) = glow {
        mood.glow = c;
    }
    if let Some(c) = ambient {
        mood.ambient = c;
    }
    if let Some(c) = fog {
        mood.fog = c;
    }
    mood
}

fn hash(x: i32, y: i32, z: i32) -> f32 {
    let mut h =
        x.wrapping_mul(374_761_393) ^ y.wrapping_mul(668_265_263) ^ z.wrapping_mul(2_147_483_647);
    h = (h ^ ((h as u32 >> 13) as i32)).wrapping_mul(1_274_126_177);
    ((h ^ ((h as u32 >> 16) as i32)) as u32) as f32 / 4_294_967_296.0
}
/// Smooth three-dimensional value noise used by the native weather pass.
pub fn noise3(x: f32, y: f32, z: f32) -> f32 {
    let (xi, yi, zi) = (x.floor() as i32, y.floor() as i32, z.floor() as i32);
    let s = |t: f32| t * t * (3. - 2. * t);
    let (u, v, w) = (s(x - x.floor()), s(y - y.floor()), s(z - z.floor()));
    let l = |a: f32, b: f32, t: f32| a + (b - a) * t;
    let c = |dx, dy, dz| hash(xi + dx, yi + dy, zi + dz);
    l(
        l(
            l(c(0, 0, 0), c(1, 0, 0), u),
            l(c(0, 1, 0), c(1, 1, 0), u),
            v,
        ),
        l(
            l(c(0, 0, 1), c(1, 0, 1), u),
            l(c(0, 1, 1), c(1, 1, 1), u),
            v,
        ),
        w,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use velvet_dungeon::{gameplay::Difficulty, generate, Catalog, Config};
    #[test]
    fn themes_cover_roles_and_cycle_galleries_without_affecting_layout() {
        let catalog = Catalog::bundled();
        for seed in 0..48 {
            let map = generate(seed, &Config::default(), &catalog, &Difficulty::default()).unwrap();
            let before = serde_json::to_vec(&map).unwrap();
            let mut seen = [false; 6];
            for room in &map.rooms {
                if let Some(t) = of(&map, room) {
                    assert_eq!(Some(t), of(&map, room));
                    let f = finish(&map, room, &catalog.biomes[room.biome]).unwrap();
                    assert_eq!(f, finish(&map, room, &catalog.biomes[room.biome]).unwrap());
                    assert!(f.stone.iter().all(|v| v.is_finite() && *v > 0.));
                    if room.role == Role::Normal {
                        let index = GALLERIES[0].iter().position(|g| *g == t).unwrap();
                        seen[index] = true;
                    }
                }
            }
            assert_eq!(before, serde_json::to_vec(&map).unwrap());
            if seed == 0 {
                assert!(seen.iter().any(|v| *v));
            }
        }
    }
    #[test]
    fn value_noise_is_bounded_and_repeatable() {
        for i in -20..20 {
            let a = noise3(i as f32 * 0.319, 1.7, -i as f32 * 0.48);
            assert!((0.0..=1.0).contains(&a));
            assert_eq!(a, noise3(i as f32 * 0.319, 1.7, -i as f32 * 0.48));
        }
    }
    #[test]
    fn themed_meshes_remain_finite_and_bounded_across_seeds() {
        let catalog = Catalog::bundled();
        let mut galleries = 0;
        for seed in 0..6 {
            let map = generate(seed, &Config::default(), &catalog, &Difficulty::default()).unwrap();
            for room in map.rooms.iter().filter(|r| r.role == Role::Normal) {
                let mesh = super::super::visual::room(&map, &catalog, room.id);
                assert!(!mesh.indices.is_empty());
                assert!(
                    mesh.vertices.len() < 250_000,
                    "oversized room mesh seed={seed} room={}",
                    room.id
                );
                assert!(mesh.vertices.iter().all(|v| v
                    .position
                    .iter()
                    .chain(v.normal.iter())
                    .chain(v.color.iter())
                    .all(|x| x.is_finite())));
                galleries += 1;
            }
        }
        assert!(galleries >= 12);
    }
}
