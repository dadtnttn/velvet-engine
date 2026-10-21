//! El Pozo del Minero: a shaft by the mine's mouth, dug block by block as deep as she likes. Every
//! block is a metre of rock; the deeper, the harder the rock and the rarer what it holds (coal and
//! copper near the top; silver and gold further down; rubies, sapphires and emeralds in the basalt;
//! mithril, adamantite and the Heart's own stone where the rock turns red). Now and then the pick
//! opens a buried chest, a geode, a fossil. The Tendero at the mouth sells better picks (each hits
//! harder and faster, and the hardest rock needs a good one) and bigger bags, and buys what she
//! brings up — a little cheaper than the shop in the village.
//!
//! The shaft is `WIDTH` blocks square, its mouth at y 0 in a cave room with a ledge round it. A block
//! `[x, k, z]` (x, z in the shaft, k its depth, 0 at the top) fills x..x+1, z..z+1 and -k-1..-k. Its
//! rock and what it holds come from the seed alone; only what she dug out is kept.
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Arc;
use velvet_dungeon::{rng, Blueprint, FloorGrid, FloorSpan};
use velvet_math::Vec3;

/// The shaft's width in blocks, both ways.
pub const WIDTH: i32 = 10;
/// The ledge round the shaft's mouth, in blocks.
pub const LEDGE: i32 = 4;
/// The room's height over the mouth.
pub const ROOM_HEIGHT: f32 = 7.0;
/// How far her pick reaches.
pub const REACH: f32 = 3.4;
/// Where she comes down (the cage under the winch) and which way she looks (at the shaft).
pub const ARRIVAL: [f32; 3] = [5.0, 0.0, -2.4];
/// How long the winch takes to haul her up.
pub const HAUL_SECONDS: f32 = 1.4;

/// The rock at each depth.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rock {
    Tierra,
    Piedra,
    Pizarra,
    Granito,
    Basalto,
    Obsidiana,
    Abisal,
    Corazon,
}

impl Rock {
    pub const ALL: [Rock; 8] = [
        Rock::Tierra,
        Rock::Piedra,
        Rock::Pizarra,
        Rock::Granito,
        Rock::Basalto,
        Rock::Obsidiana,
        Rock::Abisal,
        Rock::Corazon,
    ];

    /// Where each rock begins (in metres down).
    fn from(self) -> i32 {
        match self {
            Rock::Tierra => 0,
            Rock::Piedra => 8,
            Rock::Pizarra => 20,
            Rock::Granito => 35,
            Rock::Basalto => 55,
            Rock::Obsidiana => 80,
            Rock::Abisal => 120,
            Rock::Corazon => 170,
        }
    }

    /// The rock of depth `k` (its band's edge wavers a block or two from column to column).
    pub fn at(k: i32, waver: i32) -> Rock {
        let k = k + waver;
        Rock::ALL
            .iter()
            .rev()
            .copied()
            .find(|r| k >= r.from())
            .unwrap_or(Rock::Tierra)
    }

    pub fn name(self) -> &'static str {
        match self {
            Rock::Tierra => "Tierra",
            Rock::Piedra => "Piedra",
            Rock::Pizarra => "Pizarra",
            Rock::Granito => "Granito",
            Rock::Basalto => "Basalto",
            Rock::Obsidiana => "Obsidiana",
            Rock::Abisal => "Roca abisal",
            Rock::Corazon => "Roca del Corazón",
        }
    }

    /// Blows of the worn pick it takes (deeper in the Heart's rock it only gets harder).
    pub fn hardness(self, k: i32) -> f32 {
        let base = match self {
            Rock::Tierra => 2.0,
            Rock::Piedra => 4.0,
            Rock::Pizarra => 7.0,
            Rock::Granito => 12.0,
            Rock::Basalto => 20.0,
            Rock::Obsidiana => 36.0,
            Rock::Abisal => 60.0,
            Rock::Corazon => 100.0,
        };
        base * (1.0 + ((k - 170).max(0) as f32 / 50.0) * 0.12)
    }

    /// The pick it asks for at least (a worse one only scratches it).
    pub fn tier(self) -> u8 {
        match self {
            Rock::Obsidiana => 3,
            Rock::Abisal => 4,
            Rock::Corazon => 5,
            _ => 0,
        }
    }

    /// Its colour over the texture.
    pub fn color(self) -> [f32; 3] {
        match self {
            Rock::Tierra => [1.0, 0.92, 0.82],
            Rock::Piedra => [0.95, 0.93, 0.9],
            Rock::Pizarra => [0.62, 0.66, 0.74],
            Rock::Granito => [1.0, 0.82, 0.76],
            Rock::Basalto => [0.72, 0.72, 0.74],
            Rock::Obsidiana => [0.62, 0.52, 0.8],
            Rock::Abisal => [0.6, 0.7, 0.86],
            Rock::Corazon => [1.0, 0.42, 0.4],
        }
    }

    /// Which of the shaft's textures it wears: 0 earth, 1 stone, 2 dark rock.
    pub fn texture(self) -> usize {
        match self {
            Rock::Tierra => 0,
            Rock::Piedra | Rock::Pizarra | Rock::Granito => 1,
            _ => 2,
        }
    }
}

/// An ore (or gem) of the shaft: what it is worth at the village's shop, its look, and where it is
/// found (from a depth, most often round its peak; `rarity` is its chance per block there).
pub struct Ore {
    pub id: &'static str,
    pub name: &'static str,
    pub value: u32,
    pub color: [f32; 3],
    pub glow: f32,
    pub from: i32,
    pub peak: i32,
    pub rarity: f32,
}

pub const ORES: [Ore; 12] = [
    Ore {
        id: "mineral_carbon",
        name: "Carbón",
        value: 3,
        color: [0.12, 0.11, 0.11],
        glow: 0.0,
        from: 0,
        peak: 6,
        rarity: 0.09,
    },
    Ore {
        id: "mineral_cobre",
        name: "Cobre",
        value: 5,
        color: [0.85, 0.45, 0.22],
        glow: 0.0,
        from: 2,
        peak: 12,
        rarity: 0.07,
    },
    Ore {
        id: "mineral_hierro",
        name: "Hierro",
        value: 9,
        color: [0.72, 0.6, 0.52],
        glow: 0.0,
        from: 10,
        peak: 26,
        rarity: 0.065,
    },
    Ore {
        id: "mineral_plata",
        name: "Plata",
        value: 16,
        color: [0.86, 0.88, 0.92],
        glow: 0.15,
        from: 20,
        peak: 42,
        rarity: 0.045,
    },
    Ore {
        id: "mineral_oro",
        name: "Oro",
        value: 28,
        color: [1.0, 0.78, 0.2],
        glow: 0.35,
        from: 28,
        peak: 60,
        rarity: 0.035,
    },
    Ore {
        id: "gema_amatista",
        name: "Amatista",
        value: 40,
        color: [0.66, 0.36, 0.95],
        glow: 0.9,
        from: 36,
        peak: 62,
        rarity: 0.02,
    },
    Ore {
        id: "gema_rubi",
        name: "Rubí",
        value: 65,
        color: [0.95, 0.12, 0.2],
        glow: 1.0,
        from: 55,
        peak: 85,
        rarity: 0.014,
    },
    Ore {
        id: "gema_zafiro",
        name: "Zafiro",
        value: 80,
        color: [0.2, 0.42, 1.0],
        glow: 1.0,
        from: 58,
        peak: 95,
        rarity: 0.012,
    },
    Ore {
        id: "gema_esmeralda",
        name: "Esmeralda",
        value: 95,
        color: [0.2, 0.92, 0.45],
        glow: 1.0,
        from: 62,
        peak: 105,
        rarity: 0.011,
    },
    Ore {
        id: "mineral_mithril",
        name: "Mithril",
        value: 150,
        color: [0.62, 0.92, 1.0],
        glow: 1.3,
        from: 85,
        peak: 140,
        rarity: 0.012,
    },
    Ore {
        id: "mineral_adamantita",
        name: "Adamantita",
        value: 240,
        color: [0.45, 1.0, 0.75],
        glow: 1.5,
        from: 125,
        peak: 190,
        rarity: 0.008,
    },
    Ore {
        id: "corazonita",
        name: "Corazonita",
        value: 400,
        color: [1.0, 0.18, 0.3],
        glow: 2.0,
        from: 170,
        peak: 260,
        rarity: 0.005,
    },
];

/// A fossil, the odd find of the middle rock.
pub const FOSSIL: (&str, &str, u32) = ("fosil", "Fósil", 45);

/// The ore of an id.
pub fn ore(id: &str) -> Option<&'static Ore> {
    ORES.iter().find(|o| o.id == id)
}

/// What the village's shop pays for a thing the shaft gives, if it is one.
pub fn value(id: &str) -> Option<u32> {
    ore(id)
        .map(|o| o.value)
        .or((id == FOSSIL.0).then_some(FOSSIL.2))
}

/// The Tendero pays this much of the village's price.
pub const MOUTH_RATE: f32 = 0.8;

/// A pick the Tendero sells: how hard and how often it hits, the rock it can bite.
pub struct Pick {
    pub id: &'static str,
    pub name: &'static str,
    pub damage: f32,
    pub rate: f32,
    pub tier: u8,
    pub price: u32,
    pub color: [f32; 3],
}

pub const PICKS: [Pick; 7] = [
    Pick {
        id: "pico_gastado",
        name: "Pico gastado",
        damage: 1.0,
        rate: 1.7,
        tier: 0,
        price: 0,
        color: [0.45, 0.42, 0.4],
    },
    Pick {
        id: "pico_hierro",
        name: "Pico de hierro",
        damage: 2.0,
        rate: 1.9,
        tier: 1,
        price: 120,
        color: [0.62, 0.62, 0.64],
    },
    Pick {
        id: "pico_acero",
        name: "Pico de acero",
        damage: 4.0,
        rate: 2.1,
        tier: 2,
        price: 450,
        color: [0.78, 0.82, 0.88],
    },
    Pick {
        id: "pico_templado",
        name: "Pico templado",
        damage: 7.0,
        rate: 2.3,
        tier: 3,
        price: 1400,
        color: [0.36, 0.48, 0.7],
    },
    Pick {
        id: "pico_mithril",
        name: "Pico de mithril",
        damage: 12.0,
        rate: 2.6,
        tier: 4,
        price: 4200,
        color: [0.62, 0.92, 1.0],
    },
    Pick {
        id: "pico_runico",
        name: "Pico rúnico",
        damage: 20.0,
        rate: 2.9,
        tier: 5,
        price: 12000,
        color: [0.7, 0.45, 1.0],
    },
    Pick {
        id: "pico_corazon",
        name: "Pico del Corazón",
        damage: 35.0,
        rate: 3.3,
        tier: 6,
        price: 35000,
        color: [1.0, 0.2, 0.28],
    },
];

/// A bag the Tendero sells: how many pieces of ore it carries.
pub struct Bag {
    pub id: &'static str,
    pub name: &'static str,
    pub capacity: u32,
    pub price: u32,
}

pub const BAGS: [Bag; 5] = [
    Bag {
        id: "saco_minero",
        name: "Saco de arpillera",
        capacity: 30,
        price: 0,
    },
    Bag {
        id: "mochila_cuero",
        name: "Mochila de cuero",
        capacity: 60,
        price: 150,
    },
    Bag {
        id: "mochila_reforzada",
        name: "Mochila reforzada",
        capacity: 120,
        price: 600,
    },
    Bag {
        id: "mochila_minero",
        name: "Mochila de minero",
        capacity: 250,
        price: 2000,
    },
    Bag {
        id: "carretilla",
        name: "Carretilla de veta",
        capacity: 600,
        price: 7000,
    },
];

/// The pick and the bag she has (the best of each she owns).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Gear {
    pub pick: usize,
    pub bag: usize,
}

impl Gear {
    /// The best she owns of what `owned` says she has.
    pub fn from_owned(owned: impl Fn(&str) -> bool) -> Gear {
        Gear {
            pick: (0..PICKS.len())
                .rev()
                .find(|&i| i == 0 || owned(PICKS[i].id))
                .unwrap_or(0),
            bag: (0..BAGS.len())
                .rev()
                .find(|&i| i == 0 || owned(BAGS[i].id))
                .unwrap_or(0),
        }
    }
    pub fn pick(&self) -> &'static Pick {
        &PICKS[self.pick.min(PICKS.len() - 1)]
    }
    pub fn bag(&self) -> &'static Bag {
        &BAGS[self.bag.min(BAGS.len() - 1)]
    }
}

/// How many pieces of the shaft's goods she carries.
pub fn carried(materials: &BTreeMap<String, u32>) -> u32 {
    materials
        .iter()
        .filter(|(id, _)| value(id).is_some())
        .map(|(_, n)| *n)
        .sum()
}

/// What a block holds besides its rock.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Find {
    Ore(usize),
    Chest,
    Geode,
    Fossil,
    Bones,
}

/// A block: its rock and what it holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Block {
    pub rock: Rock,
    pub find: Option<Find>,
}

fn unit(seed: u64, x: i32, k: i32, z: i32, salt: u64) -> f32 {
    let mut bytes = [0u8; 40];
    bytes[..8].copy_from_slice(&seed.to_le_bytes());
    bytes[8..16].copy_from_slice(&(x as i64).to_le_bytes());
    bytes[16..24].copy_from_slice(&(k as i64).to_le_bytes());
    bytes[24..32].copy_from_slice(&(z as i64).to_le_bytes());
    bytes[32..].copy_from_slice(&salt.to_le_bytes());
    // FNV, then an avalanche so neighbouring blocks share nothing
    let mut h = rng::hash(&bytes);
    h ^= h >> 33;
    h = h.wrapping_mul(0xff51_afd7_ed55_8ccd);
    h ^= h >> 33;
    h = h.wrapping_mul(0xc4ce_b9fe_1a85_ec53);
    h ^= h >> 33;
    (h >> 11) as f32 / (1u64 << 53) as f32
}

/// The shaft's blocks as the seed makes them.
pub fn block(seed: u64, x: i32, k: i32, z: i32) -> Block {
    let waver = (unit(seed, x / 3, k / 6, z / 3, 1) * 3.0) as i32 - 1;
    let rock = Rock::at(k, waver);
    let roll = unit(seed, x, k, z, 2);
    let special = unit(seed, x, k, z, 3);
    let find = if k >= 4 && special < 0.0035 {
        Some(Find::Chest)
    } else if k >= 15 && special < 0.007 {
        Some(Find::Geode)
    } else if (10..90).contains(&k) && special < 0.012 {
        Some(Find::Fossil)
    } else if k < 30 && special < 0.02 {
        Some(Find::Bones)
    } else {
        let mut acc = 0.0;
        ORES.iter().enumerate().find_map(|(i, o)| {
            if k < o.from {
                return None;
            }
            // most often round its peak, a little less above and below it
            let off = (k - o.peak) as f32 / (o.peak - o.from).max(6) as f32;
            acc += o.rarity * (1.0 - 0.6 * off.abs().min(1.5) / 1.5).max(0.25);
            (roll < acc).then_some(Find::Ore(i))
        })
    };
    Block { rock, find }
}

/// Air the seed leaves in the rock: caves the pick breaks into (never near the top).
pub fn pocket(seed: u64, x: i32, k: i32, z: i32) -> bool {
    if k < 14 {
        return false;
    }
    // a soft field on a 4-block lattice, opened where it peaks
    let f = |dx: i32, dk: i32, dz: i32| {
        unit(
            seed,
            x.div_euclid(4) + dx,
            k.div_euclid(4) + dk,
            z.div_euclid(4) + dz,
            9,
        )
    };
    let (tx, tk, tz) = (
        x.rem_euclid(4) as f32 / 4.0,
        k.rem_euclid(4) as f32 / 4.0,
        z.rem_euclid(4) as f32 / 4.0,
    );
    let lerp = |a: f32, b: f32, t: f32| a + (b - a) * (t * t * (3.0 - 2.0 * t));
    let v = lerp(
        lerp(
            lerp(f(0, 0, 0), f(1, 0, 0), tx),
            lerp(f(0, 1, 0), f(1, 1, 0), tx),
            tk,
        ),
        lerp(
            lerp(f(0, 0, 1), f(1, 0, 1), tx),
            lerp(f(0, 1, 1), f(1, 1, 1), tx),
            tk,
        ),
        tz,
    );
    v > 0.8
}

/// Whether a column of the plan is in the shaft.
pub fn in_shaft(x: i32, z: i32) -> bool {
    (0..WIDTH).contains(&x) && (0..WIDTH).contains(&z)
}

/// Something that happened down there, for the HUD and the sounds.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    Hit { at: Vec3, rock: Rock },
    Broke { at: Vec3, rock: Rock },
    Got { label: String, color: [f32; 3] },
    TooHard { needs: &'static str },
    Full,
}

/// The shaft and her in it.
pub struct Pozo {
    pub seed: u64,
    /// Her floors here: the mine's map with the shaft's floors (rebuilt as she digs).
    pub map: Blueprint,
    mined: HashSet<[i32; 3]>,
    /// Damage done to blocks she has started on.
    pub damage: HashMap<[i32; 3], f32>,
    /// The block in front of her pick.
    pub target: Option<[i32; 3]>,
    /// The pick's blow: time left of its swing, time to the next.
    pub swing: f32,
    pub cooldown: f32,
    /// The mouse held down: she keeps digging.
    pub holding: bool,
    /// The deepest block she has dug.
    pub deepest: i32,
    /// Bumped when blocks change: the scene rebuilds what it shows.
    pub version: u64,
    /// The winch hauling her up: time left.
    pub hauling: Option<f32>,
    /// Where she stood in the mine before coming down.
    pub came_from: (Vec3, f32),
    /// The last "too hard" or "full" word, so it is not repeated every blow.
    pub last_word: f32,
    /// The block she last hit or broke (the scene rebuilds round it).
    pub touched: Option<[i32; 3]>,
}

impl Pozo {
    pub fn new(seed: u64, mine: &Blueprint, came_from: (Vec3, f32)) -> Self {
        let mut map = mine.clone();
        map.obstacles.clear();
        let mut pozo = Self {
            seed,
            map,
            mined: HashSet::new(),
            damage: HashMap::new(),
            target: None,
            swing: 0.0,
            cooldown: 0.0,
            holding: false,
            deepest: -1,
            version: 0,
            hauling: None,
            came_from,
            last_word: -10.0,
            touched: None,
        };
        pozo.rebuild_floors();
        pozo
    }

    pub fn arrival() -> (Vec3, f32) {
        (
            Vec3::new(ARRIVAL[0], ARRIVAL[1], ARRIVAL[2]),
            std::f32::consts::PI,
        )
    }

    /// Whether a block is rock (in the shaft and not dug out, nor a cave the seed left).
    pub fn solid(&self, x: i32, k: i32, z: i32) -> bool {
        if k < 0 {
            return false;
        }
        if !in_shaft(x, z) {
            return true;
        }
        !self.mined.contains(&[x, k, z]) && !pocket(self.seed, x, k, z)
    }

    /// Whether a block has been opened (dug or a cave) — what the scene shows the inside of.
    pub fn open(&self, x: i32, k: i32, z: i32) -> bool {
        !self.solid(x, k, z)
    }

    /// The deepest block the scene and the floors need to know about.
    pub fn depth_shown(&self) -> i32 {
        self.deepest.max(0) + 12
    }

    pub fn block(&self, x: i32, k: i32, z: i32) -> Block {
        block(self.seed, x, k, z)
    }

    /// Her depth in metres (0 at the mouth).
    pub fn depth_of(position: Vec3) -> f32 {
        (-position.y).max(0.0)
    }

    /// The shaft's floors, column by column: the ledge round the mouth, and down each column of
    /// the shaft every top of rock with air over it (and how much air).
    fn rebuild_floors(&mut self) {
        let x0 = -LEDGE;
        let z0 = -LEDGE;
        let w = (WIDTH + 2 * LEDGE) as u32;
        let bottom = self.depth_shown();
        let mut columns = Vec::with_capacity((w * w) as usize);
        for row in 0..w as i32 {
            for col in 0..w as i32 {
                let (x, z) = (x0 + col, z0 + row);
                let mut spans = Vec::new();
                if !in_shaft(x, z) {
                    spans.push(FloorSpan {
                        floor: 0.0,
                        ceiling: ROOM_HEIGHT,
                        slope: [0.0, 0.0],
                        room: None,
                    });
                } else {
                    // down the column: a floor wherever rock has air on it; its ceiling the bottom
                    // of the rock over that air (or the room's)
                    let mut ceiling = ROOM_HEIGHT;
                    let mut air_above = true;
                    for k in 0..=bottom {
                        if self.solid(x, k, z) {
                            if air_above {
                                spans.push(FloorSpan {
                                    floor: -k as f32,
                                    ceiling,
                                    slope: [0.0, 0.0],
                                    room: None,
                                });
                            }
                            ceiling = -(k as f32) - 1.0;
                            air_above = false;
                        } else {
                            air_above = true;
                        }
                    }
                    spans.reverse();
                    spans.sort_by(|a, b| b.floor.total_cmp(&a.floor));
                }
                columns.push(spans);
            }
        }
        let mut grid = FloorGrid::new(x0 as f32, z0 as f32, 1.0, w, w, columns);
        // the blocks stand where they are: no margin baked in, the body keeps its own width
        grid.margin = 0.0;
        self.map.floors = Some(Arc::new(grid));
    }

    /// The first rock her eye meets along `dir` within reach: the block, and the face it came
    /// in by (the open side).
    pub fn aim(&self, eye: Vec3, dir: Vec3) -> Option<[i32; 3]> {
        let dir = dir.normalize_or_zero();
        if dir.length_squared() < 0.5 {
            return None;
        }
        // through the grid of blocks (y up: a block's k is -floor(y) - 1)
        let mut t = 0.0;
        let step = 0.04;
        while t < REACH {
            let p = eye + dir * t;
            let (x, z) = (p.x.floor() as i32, p.z.floor() as i32);
            let k = -(p.y.floor() as i32) - 1;
            if k >= 0 && self.solid(x, k, z) {
                return Some([x, k, z]);
            }
            t += step;
        }
        None
    }

    /// Blocks of the shaft she can dig (not the ledge's rock, not under the room).
    pub fn diggable(&self, b: [i32; 3]) -> bool {
        in_shaft(b[0], b[2]) && b[1] >= 0
    }

    /// The middle of a block.
    pub fn center(b: [i32; 3]) -> Vec3 {
        Vec3::new(b[0] as f32 + 0.5, -(b[1] as f32) - 0.5, b[2] as f32 + 0.5)
    }

    /// One step of digging: the swing plays out, and while the mouse is held a blow lands on the
    /// block in front of her as often as the pick allows. What a broken block held goes into
    /// `carried` (ore and gems, while the bag has room), `gold` (a buried chest).
    pub fn dig(
        &mut self,
        eye: Vec3,
        forward: Vec3,
        gear: Gear,
        carried_now: &mut BTreeMap<String, u32>,
        gold: &mut u32,
        elapsed: f32,
        dt: f32,
    ) -> Vec<Event> {
        let mut events = Vec::new();
        self.swing = (self.swing - dt).max(0.0);
        self.cooldown = (self.cooldown - dt).max(0.0);
        self.target = self.aim(eye, forward);
        if !self.holding || self.cooldown > 0.0 || self.hauling.is_some() {
            return events;
        }
        let pick = gear.pick();
        self.cooldown = 1.0 / pick.rate;
        self.swing = 0.3;
        let Some(b) = self.target else {
            return events;
        };
        let at = Self::center(b);
        if !self.diggable(b) {
            events.push(Event::Hit {
                at,
                rock: Rock::Abisal,
            });
            return events;
        }
        let block = self.block(b[0], b[1], b[2]);
        if pick.tier < block.rock.tier() {
            events.push(Event::Hit {
                at,
                rock: block.rock,
            });
            if elapsed - self.last_word > 2.0 {
                self.last_word = elapsed;
                let needs = PICKS
                    .iter()
                    .find(|p| p.tier >= block.rock.tier())
                    .map_or("un pico mejor", |p| p.name);
                events.push(Event::TooHard { needs });
            }
            return events;
        }
        let holds_goods = matches!(block.find, Some(Find::Ore(_) | Find::Geode | Find::Fossil));
        if holds_goods && carried(carried_now) >= gear.bag().capacity {
            events.push(Event::Hit {
                at,
                rock: block.rock,
            });
            if elapsed - self.last_word > 2.0 {
                self.last_word = elapsed;
                events.push(Event::Full);
            }
            return events;
        }
        let hp = block.rock.hardness(b[1])
            * if matches!(block.find, Some(Find::Ore(_))) {
                1.5
            } else {
                1.0
            };
        self.touched = Some(b);
        let done = self.damage.entry(b).or_insert(0.0);
        *done += pick.damage;
        if *done < hp {
            events.push(Event::Hit {
                at,
                rock: block.rock,
            });
            self.version += 1;
            return events;
        }
        // it breaks
        self.damage.remove(&b);
        self.mined.insert(b);
        self.deepest = self.deepest.max(b[1]);
        self.version += 1;
        self.rebuild_floors();
        events.push(Event::Broke {
            at,
            rock: block.rock,
        });
        let mut take =
            |id: &str, n: u32, label: String, color: [f32; 3], events: &mut Vec<Event>| {
                let room = gear.bag().capacity.saturating_sub(carried(carried_now));
                let n = n.min(room);
                if n > 0 {
                    *carried_now.entry(id.to_string()).or_default() += n;
                    events.push(Event::Got {
                        label: format!("+{n} {label}"),
                        color,
                    });
                }
            };
        match block.find {
            Some(Find::Ore(i)) => {
                let o = &ORES[i];
                take(o.id, 1, o.name.to_string(), o.color, &mut events);
            }
            Some(Find::Geode) => {
                // a geode: a few of the gems of its depth
                let gem = ORES
                    .iter()
                    .filter(|o| o.glow > 0.5 && b[1] >= o.from)
                    .last()
                    .unwrap_or(&ORES[5]);
                let n = 2 + (unit(self.seed, b[0], b[1], b[2], 5) * 3.0) as u32;
                take(
                    gem.id,
                    n,
                    format!("{} (¡una geoda!)", gem.name),
                    gem.color,
                    &mut events,
                );
            }
            Some(Find::Fossil) => take(
                FOSSIL.0,
                1,
                FOSSIL.1.to_string(),
                [0.86, 0.8, 0.66],
                &mut events,
            ),
            Some(Find::Bones) => {
                *carried_now.entry("hueso_viejo".into()).or_default() += 2;
                events.push(Event::Got {
                    label: "+2 Hueso viejo".into(),
                    color: [0.85, 0.79, 0.64],
                });
            }
            Some(Find::Chest) => {
                let k = b[1].max(0) as u32;
                let coins = 15
                    + k * 3
                    + (unit(self.seed, b[0], b[1], b[2], 6) * (k * 2 + 20) as f32) as u32;
                *gold += coins;
                events.push(Event::Got {
                    label: format!("+{coins} oro (un cofre enterrado)"),
                    color: [1.0, 0.82, 0.3],
                });
            }
            None => {}
        }
        events
    }

    /// The winch: a few seconds and she is back on the ledge.
    pub fn haul(&mut self) {
        if self.hauling.is_none() {
            self.hauling = Some(HAUL_SECONDS);
        }
    }

    /// Whether she stands on the ledge by the cage (where F takes her back out to the mine).
    pub fn at_cage(position: Vec3) -> bool {
        let (a, _) = Self::arrival();
        (position - a).length() < 2.2 && position.y > -0.5
    }

    /// How much of a block has been broken (0 whole, 1 about to give).
    pub fn wear(&self, b: [i32; 3]) -> f32 {
        let Some(done) = self.damage.get(&b) else {
            return 0.0;
        };
        let block = self.block(b[0], b[1], b[2]);
        let hp = block.rock.hardness(b[1])
            * if matches!(block.find, Some(Find::Ore(_))) {
                1.5
            } else {
                1.0
            };
        (done / hp).clamp(0.0, 1.0)
    }

    /// How many blocks she has dug.
    pub fn dug(&self) -> usize {
        self.mined.len()
    }

    /// Blocks dug out at once, with nothing taken from them (captures, tests).
    pub fn dig_out(&mut self, blocks: impl IntoIterator<Item = [i32; 3]>) {
        for b in blocks {
            if self.diggable(b) {
                self.mined.insert(b);
                self.deepest = self.deepest.max(b[1]);
                self.touched = Some(b);
            }
        }
        self.version += 1;
        self.rebuild_floors();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mine_map() -> Blueprint {
        crate::Dungeon::generate(7).map
    }

    #[test]
    fn the_deeper_the_rarer() {
        let seed = 42;
        let value_at = |from: i32, to: i32| {
            let mut total = 0u64;
            let mut blocks = 0u64;
            for k in from..to {
                for x in 0..WIDTH {
                    for z in 0..WIDTH {
                        blocks += 1;
                        if let Some(Find::Ore(i)) = block(seed, x, k, z).find {
                            total += u64::from(ORES[i].value);
                        }
                    }
                }
            }
            total as f32 / blocks as f32
        };
        let top = value_at(0, 10);
        let middle = value_at(40, 50);
        let deep = value_at(120, 130);
        assert!(top < middle && middle < deep, "{top} {middle} {deep}");
        // every ore turns up somewhere down the first three hundred metres
        for (i, o) in ORES.iter().enumerate() {
            let found = (0..300).any(|k| {
                (0..WIDTH)
                    .any(|x| (0..WIDTH).any(|z| block(seed, x, k, z).find == Some(Find::Ore(i))))
            });
            assert!(found, "{}", o.name);
        }
        assert_eq!(Rock::at(0, 0), Rock::Tierra);
        assert_eq!(Rock::at(100, 0), Rock::Obsidiana);
        assert_eq!(Rock::at(400, 0), Rock::Corazon);
    }

    #[test]
    fn digging_breaks_blocks_and_she_falls_into_the_hole() {
        let map = mine_map();
        let mut pozo = Pozo::new(3, &map, (Vec3::ZERO, 0.0));
        let gear = Gear::default();
        let mut carried_now = BTreeMap::new();
        let mut gold = 0;
        // standing on the shaft's top, looking straight down at the block under her
        let feet = Vec3::new(4.5, 0.0, 4.5);
        let eye = feet + Vec3::Y * 1.62;
        pozo.holding = true;
        let mut broke = 0;
        for frame in 0..(60 * 20) {
            let events = pozo.dig(
                eye,
                Vec3::new(0.0, -1.0, 0.0),
                gear,
                &mut carried_now,
                &mut gold,
                frame as f32 / 60.0,
                1.0 / 60.0,
            );
            broke += events
                .iter()
                .filter(|e| matches!(e, Event::Broke { .. }))
                .count();
            if broke > 0 {
                break;
            }
        }
        assert_eq!(broke, 1, "the earth under her gives");
        // her floor there is now a metre lower
        let floor = pozo
            .map
            .surface(feet.x, feet.z, 0.0, velvet_dungeon::PLAYER_HEIGHT);
        assert_eq!(floor, Some(-1.0));
        // the ledge does not give
        assert!(!pozo.diggable([-1, 0, 3]));
        assert!(pozo.solid(-1, 5, 3));
    }

    #[test]
    fn a_full_bag_or_a_weak_pick_stops_the_digging() {
        let map = mine_map();
        let mut pozo = Pozo::new(11, &map, (Vec3::ZERO, 0.0));
        // a block of obsidian before a worn pick: only scratches
        let deep = (0..WIDTH)
            .flat_map(|x| (0..WIDTH).map(move |z| (x, z)))
            .find(|&(x, z)| block(11, x, 100, z).rock == Rock::Obsidiana && !pocket(11, x, 100, z))
            .unwrap();
        for k in 0..100 {
            pozo.mined.insert([deep.0, k, deep.1]);
        }
        pozo.rebuild_floors();
        let eye = Vec3::new(deep.0 as f32 + 0.5, -100.0 + 1.62, deep.1 as f32 + 0.5);
        pozo.holding = true;
        let mut carried_now = BTreeMap::new();
        let mut gold = 0;
        let mut too_hard = false;
        for frame in 0..600 {
            for e in pozo.dig(
                eye,
                Vec3::new(0.0, -1.0, 0.0),
                Gear::default(),
                &mut carried_now,
                &mut gold,
                frame as f32 / 60.0,
                1.0 / 60.0,
            ) {
                too_hard |= matches!(e, Event::TooHard { .. });
                assert!(
                    !matches!(e, Event::Broke { .. }),
                    "a worn pick does not break obsidian"
                );
            }
        }
        assert!(too_hard);
        // the tempered pick does
        let gear = Gear { pick: 3, bag: 0 };
        let mut broke = false;
        for frame in 0..(60 * 40) {
            broke |= pozo
                .dig(
                    eye,
                    Vec3::new(0.0, -1.0, 0.0),
                    gear,
                    &mut carried_now,
                    &mut gold,
                    frame as f32 / 60.0,
                    1.0 / 60.0,
                )
                .iter()
                .any(|e| matches!(e, Event::Broke { .. }));
            if broke {
                break;
            }
        }
        assert!(broke);
        assert_eq!(
            carried(&BTreeMap::from([
                ("mineral_oro".to_string(), 3),
                ("hueso_viejo".to_string(), 9)
            ])),
            3
        );
        assert_eq!(value("gema_rubi"), Some(65));
        assert_eq!(value("hueso_viejo"), None);
    }
}
