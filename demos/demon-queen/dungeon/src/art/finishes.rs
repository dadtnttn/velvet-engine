//! Room finishes (web prototype `finishes.js`): every themed chamber gets its
//! own materials — wall bond, floor, ceiling, pilasters, arches, ribs,
//! doorways and columns, its wear and the particles in its air — drawn per
//! room from its theme. Signature halls keep their bespoke finishes.
use super::super::themes::{self, Theme};
use super::kit::*;
use velvet_dungeon::{Biome, Blueprint, Room, Shape};

/// A wall, floor or ceiling bond as `panel()` parameters.
#[derive(Clone, Copy, Debug, Default)]
pub struct BondStyle {
    pub courses: Option<[f32; 2]>,
    pub bevel: Option<[f32; 2]>,
    pub face_depth: Option<[f32; 2]>,
    pub tone: Option<[f32; 2]>,
    pub crack: Option<f32>,
    pub chip: Option<[f32; 2]>,
    pub moss_coverage: Option<f32>,
}

#[derive(Clone, Copy, Debug)]
pub struct Bond {
    pub masonry: bool,
    pub cell: [f32; 2],
    pub relief: f32,
    pub style: BondStyle,
    pub mortar: bool,
    pub plaster: f32,
    pub wainscot: f32,
}

const fn st(
    courses: Option<[f32; 2]>,
    bevel: Option<[f32; 2]>,
    face_depth: Option<[f32; 2]>,
    tone: Option<[f32; 2]>,
    crack: Option<f32>,
    chip: Option<[f32; 2]>,
    moss_coverage: Option<f32>,
) -> BondStyle {
    BondStyle {
        courses,
        bevel,
        face_depth,
        tone,
        crack,
        chip,
        moss_coverage,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Wall {
    Ashlar,
    AshlarLarge,
    Brick,
    Rubble,
    Rock,
    Cyclopean,
    Plaster,
    Panelled,
}

pub fn wall_bond(w: Wall) -> Bond {
    let ashlar = Bond {
        masonry: true,
        cell: [0.95, 0.46],
        relief: 0.24,
        style: st(
            Some([0.8, 1.2]),
            Some([0.86, 0.92]),
            Some([0.35, 0.55]),
            Some([0.82, 1.12]),
            Some(0.2),
            None,
            None,
        ),
        mortar: false,
        plaster: 0.0,
        wainscot: 0.0,
    };
    let brick = Bond {
        masonry: true,
        cell: [0.5, 0.21],
        relief: 0.13,
        style: st(
            None,
            Some([0.9, 0.95]),
            Some([0.3, 0.5]),
            Some([0.68, 1.16]),
            Some(0.06),
            Some([0.004, 0.012]),
            None,
        ),
        mortar: true,
        plaster: 0.0,
        wainscot: 0.0,
    };
    match w {
        Wall::Ashlar => ashlar,
        Wall::AshlarLarge => Bond {
            masonry: true,
            cell: [1.5, 0.74],
            relief: 0.3,
            style: st(
                Some([0.75, 1.3]),
                Some([0.84, 0.9]),
                Some([0.3, 0.52]),
                Some([0.78, 1.12]),
                Some(0.28),
                None,
                None,
            ),
            mortar: false,
            plaster: 0.0,
            wainscot: 0.0,
        },
        Wall::Brick => brick,
        Wall::Rubble => Bond {
            masonry: false,
            cell: [0.6, 0.44],
            relief: 0.26,
            style: st(
                None,
                Some([0.78, 0.88]),
                Some([0.35, 0.6]),
                Some([0.68, 1.16]),
                Some(0.14),
                Some([0.02, 0.06]),
                None,
            ),
            mortar: false,
            plaster: 0.0,
            wainscot: 0.0,
        },
        Wall::Rock => Bond {
            masonry: false,
            cell: [1.8, 1.1],
            relief: 0.45,
            style: st(
                None,
                Some([0.84, 0.93]),
                None,
                Some([0.74, 1.14]),
                Some(0.38),
                None,
                None,
            ),
            mortar: false,
            plaster: 0.0,
            wainscot: 0.0,
        },
        Wall::Cyclopean => Bond {
            masonry: false,
            cell: [1.45, 1.2],
            relief: 0.3,
            style: st(
                None,
                Some([0.9, 0.95]),
                Some([0.3, 0.45]),
                Some([0.8, 1.1]),
                Some(0.3),
                Some([0.01, 0.03]),
                None,
            ),
            mortar: false,
            plaster: 0.0,
            wainscot: 0.0,
        },
        Wall::Plaster => Bond {
            plaster: 0.3,
            ..brick
        },
        Wall::Panelled => Bond {
            wainscot: 1.15,
            ..ashlar
        },
    }
}

pub fn wall_budget(w: Wall) -> f32 {
    match w {
        Wall::Brick | Wall::Plaster => 900.0,
        Wall::Ashlar | Wall::Panelled => 600.0,
        Wall::AshlarLarge => 420.0,
        Wall::Rubble => 520.0,
        Wall::Rock => 230.0,
        Wall::Cyclopean => 300.0,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FloorKind {
    Flagstone,
    Slabs,
    Cobbles,
    Dirt,
    Planks,
    Checker,
    Bricks,
    AshlarPave,
}

#[derive(Clone, Copy, Debug)]
pub struct Floor {
    pub masonry: bool,
    pub cell: [f32; 2],
    pub relief: f32,
    pub style: BondStyle,
    pub pebbles: bool,
    pub axis: bool,
    pub checker: bool,
    pub ashlar: bool,
}

pub fn floor_bond(k: FloorKind) -> Floor {
    let f = |masonry, cell, relief, style, pebbles, axis, checker| Floor {
        masonry,
        cell,
        relief,
        style,
        pebbles,
        axis,
        checker,
        ashlar: false,
    };
    match k {
        FloorKind::Flagstone => f(
            false,
            [1.35, 1.12],
            0.055,
            st(None, None, None, None, None, None, Some(0.22)),
            false,
            false,
            false,
        ),
        FloorKind::Slabs => f(
            false,
            [2.1, 1.7],
            0.06,
            st(
                None,
                Some([0.95, 0.975]),
                None,
                Some([0.78, 1.1]),
                Some(0.5),
                None,
                None,
            ),
            false,
            false,
            false,
        ),
        FloorKind::Cobbles => f(
            false,
            [0.5, 0.42],
            0.07,
            st(
                None,
                Some([0.8, 0.9]),
                None,
                Some([0.66, 1.18]),
                Some(0.05),
                Some([0.03, 0.07]),
                None,
            ),
            false,
            false,
            false,
        ),
        FloorKind::Dirt => f(
            false,
            [1.6, 1.3],
            0.028,
            st(
                None,
                Some([0.97, 0.99]),
                Some([0.1, 0.3]),
                Some([0.8, 1.12]),
                Some(0.2),
                Some([0.002, 0.008]),
                None,
            ),
            true,
            false,
            false,
        ),
        FloorKind::Planks => f(
            true,
            [2.4, 0.26],
            0.035,
            st(
                None,
                Some([0.975, 0.99]),
                Some([0.1, 0.25]),
                Some([0.78, 1.18]),
                Some(0.12),
                Some([0.002, 0.006]),
                Some(0.0),
            ),
            false,
            true,
            false,
        ),
        FloorKind::Checker => f(
            true,
            [0.95, 0.95],
            0.04,
            st(
                None,
                Some([0.95, 0.97]),
                Some([0.15, 0.3]),
                Some([0.92, 1.06]),
                Some(0.18),
                None,
                Some(0.02),
            ),
            false,
            true,
            true,
        ),
        FloorKind::Bricks => f(
            true,
            [0.6, 0.28],
            0.045,
            st(
                None,
                Some([0.93, 0.96]),
                Some([0.2, 0.4]),
                Some([0.7, 1.15]),
                Some(0.08),
                None,
                None,
            ),
            false,
            true,
            false,
        ),
        FloorKind::AshlarPave => Floor {
            masonry: false,
            cell: [1.0, 1.0],
            relief: 0.0,
            style: BondStyle::default(),
            pebbles: false,
            axis: false,
            checker: false,
            ashlar: true,
        },
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CeilingKind {
    Rock,
    Vault,
    Timber,
    Coffered,
}

#[derive(Clone, Copy, Debug)]
pub struct Ceiling {
    pub masonry: bool,
    pub cell: [f32; 2],
    pub relief: f32,
    pub style: Option<BondStyle>,
    pub wood: bool,
    pub beams: bool,
}

pub fn ceiling_bond(k: CeilingKind) -> Ceiling {
    match k {
        CeilingKind::Rock => Ceiling {
            masonry: false,
            cell: [2.25, 1.8],
            relief: 0.68,
            style: None,
            wood: false,
            beams: false,
        },
        CeilingKind::Vault => Ceiling {
            masonry: true,
            cell: [1.4, 0.7],
            relief: 0.3,
            style: Some(st(
                None,
                Some([0.88, 0.93]),
                Some([0.3, 0.5]),
                Some([0.8, 1.08]),
                Some(0.12),
                None,
                None,
            )),
            wood: false,
            beams: false,
        },
        CeilingKind::Timber => Ceiling {
            masonry: true,
            cell: [2.8, 0.3],
            relief: 0.06,
            style: Some(st(
                None,
                Some([0.97, 0.99]),
                Some([0.1, 0.3]),
                Some([0.75, 1.15]),
                Some(0.1),
                None,
                Some(0.0),
            )),
            wood: true,
            beams: true,
        },
        CeilingKind::Coffered => Ceiling {
            masonry: true,
            cell: [1.3, 1.3],
            relief: 0.34,
            style: Some(st(
                None,
                Some([0.7, 0.78]),
                Some([0.5, 0.7]),
                Some([0.9, 1.06]),
                Some(0.05),
                None,
                Some(0.0),
            )),
            wood: false,
            beams: false,
        },
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pilaster {
    Block,
    Rough,
    Cluster,
    Ornate,
    Brick,
    Timber,
    None,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BayKind {
    Round,
    Lintel,
    Pointed,
    None,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rib {
    Round,
    Beam,
    Pointed,
    None,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Door {
    Arch,
    Frame,
    Rough,
    Timber,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PillarKind {
    Octagon,
    Timber,
    Cluster,
    Ornate,
    Rough,
    Brick,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Bands {
    Plinth,
    Full,
}

/// Wear keys in the web version's order.
#[derive(Clone, Copy, Debug, Default)]
pub struct Wear {
    pub rubble: f32,
    pub moss: f32,
    pub damp: f32,
    pub streaks: f32,
    pub soot: f32,
    pub salt: f32,
    pub cracks: f32,
    pub webs: f32,
    pub heat: f32,
}

struct Recipe {
    stone: C3,
    floor: C3,
    floor2: Option<C3>,
    walls: &'static [Wall],
    floors: &'static [FloorKind],
    ceilings: &'static [CeilingKind],
    pilaster: &'static [Pilaster],
    bay: &'static [BayKind],
    rib: &'static [Rib],
    door: &'static [Door],
    pillar: &'static [PillarKind],
    bands: Option<Bands>,
    ridge: bool,
    /// Overrides of the base wear: (rubble, moss, damp, streaks, soot, salt, cracks, webs, heat).
    wear: [Option<f32>; 9],
    plaster: Option<C3>,
}

const N: Option<f32> = None;

fn recipe(t: Theme) -> Recipe {
    use BayKind as B;
    use CeilingKind as C;
    use FloorKind as F;
    use PillarKind as Pl;
    use Wall as W;
    let r = |stone,
             floor,
             walls,
             floors,
             ceilings,
             pilaster,
             bay,
             rib,
             door,
             pillar,
             bands,
             ridge,
             wear| Recipe {
        stone,
        floor,
        floor2: None,
        walls,
        floors,
        ceilings,
        pilaster,
        bay,
        rib,
        door,
        pillar,
        bands,
        ridge,
        wear,
        plaster: None,
    };
    match t {
        Theme::Threshold => r(
            [0.46, 0.38, 0.29],
            [0.36, 0.3, 0.24],
            &[W::AshlarLarge, W::Ashlar, W::Rubble],
            &[F::Flagstone, F::Slabs, F::Cobbles],
            &[C::Rock, C::Vault, C::Timber],
            &[Pilaster::Block, Pilaster::Rough],
            &[B::Round, B::Lintel],
            &[Rib::Round, Rib::Beam],
            &[Door::Arch, Door::Frame],
            &[Pl::Octagon, Pl::Timber],
            Some(Bands::Plinth),
            false,
            [
                Some(0.6),
                Some(0.4),
                Some(0.25),
                Some(0.2),
                Some(0.15),
                N,
                N,
                N,
                N,
            ],
        ),
        Theme::Return => r(
            [0.3, 0.36, 0.38],
            [0.24, 0.28, 0.3],
            &[W::Cyclopean, W::Ashlar],
            &[F::Slabs, F::AshlarPave, F::Checker],
            &[C::Vault, C::Coffered],
            &[Pilaster::Cluster, Pilaster::Ornate],
            &[B::Pointed, B::Round],
            &[Rib::Pointed],
            &[Door::Arch],
            &[Pl::Cluster],
            Some(Bands::Full),
            true,
            [
                Some(0.3),
                Some(0.3),
                Some(0.3),
                Some(0.35),
                N,
                Some(0.2),
                N,
                N,
                N,
            ],
        ),
        Theme::Seal => Recipe {
            floor2: Some([0.16, 0.14, 0.2]),
            ..r(
                [0.46, 0.43, 0.5],
                [0.5, 0.48, 0.52],
                &[W::Ashlar, W::Cyclopean],
                &[F::Checker, F::AshlarPave],
                &[C::Coffered, C::Vault],
                &[Pilaster::Ornate, Pilaster::Cluster],
                &[B::Round, B::Pointed],
                &[Rib::Round],
                &[Door::Arch],
                &[Pl::Ornate],
                Some(Bands::Full),
                true,
                [
                    Some(0.2),
                    Some(0.1),
                    Some(0.15),
                    Some(0.2),
                    N,
                    Some(0.3),
                    N,
                    N,
                    N,
                ],
            )
        },
        Theme::Memory => r(
            [0.42, 0.46, 0.5],
            [0.3, 0.33, 0.36],
            &[W::Cyclopean, W::Rock],
            &[F::Slabs, F::Flagstone],
            &[C::Rock, C::Vault],
            &[Pilaster::Rough, Pilaster::None],
            &[B::None],
            &[Rib::None],
            &[Door::Rough],
            &[Pl::Rough],
            None,
            false,
            [Some(0.5), Some(0.8), Some(0.4), Some(0.5), N, N, N, N, N],
        ),
        Theme::Lair => r(
            [0.24, 0.2, 0.17],
            [0.2, 0.16, 0.12],
            &[W::Rock, W::Rubble, W::Cyclopean, W::Ashlar],
            &[F::Dirt, F::Cobbles, F::Flagstone],
            &[C::Rock, C::Vault],
            &[Pilaster::Rough, Pilaster::None, Pilaster::Block],
            &[B::None, B::Round],
            &[Rib::None, Rib::Round],
            &[Door::Rough, Door::Frame],
            &[Pl::Rough, Pl::Octagon],
            None,
            false,
            [
                Some(1.0),
                Some(0.5),
                Some(0.4),
                Some(0.4),
                N,
                N,
                Some(0.5),
                Some(0.5),
                N,
            ],
        ),
        Theme::Guardpost => r(
            [0.4, 0.4, 0.42],
            [0.3, 0.3, 0.32],
            &[W::AshlarLarge, W::Brick, W::Plaster],
            &[F::AshlarPave, F::Flagstone, F::Planks],
            &[C::Timber, C::Vault],
            &[Pilaster::Block, Pilaster::Brick],
            &[B::Lintel, B::Round],
            &[Rib::Beam],
            &[Door::Frame],
            &[Pl::Timber, Pl::Octagon],
            Some(Bands::Plinth),
            false,
            [Some(0.4), Some(0.15), Some(0.2), N, Some(0.5), N, N, N, N],
        ),
        Theme::Custodian => r(
            [0.16, 0.15, 0.16],
            [0.14, 0.12, 0.12],
            &[W::Cyclopean, W::Rock],
            &[F::Slabs, F::Cobbles],
            &[C::Rock],
            &[Pilaster::Rough],
            &[B::None],
            &[Rib::Round],
            &[Door::Rough],
            &[Pl::Rough],
            None,
            false,
            [
                Some(0.8),
                Some(0.0),
                N,
                N,
                Some(0.5),
                N,
                Some(0.6),
                N,
                Some(0.5),
            ],
        ),
        Theme::Heart => r(
            [0.3, 0.12, 0.12],
            [0.2, 0.08, 0.08],
            &[W::Rock],
            &[F::Slabs, F::Cobbles],
            &[C::Rock],
            &[Pilaster::None],
            &[B::None],
            &[Rib::None],
            &[Door::Rough],
            &[Pl::Rough],
            None,
            false,
            [
                Some(0.6),
                Some(0.0),
                Some(0.5),
                Some(0.6),
                N,
                N,
                Some(0.5),
                N,
                Some(0.8),
            ],
        ),
        Theme::Crypt => r(
            [0.33, 0.31, 0.36],
            [0.26, 0.25, 0.28],
            &[W::Rubble, W::Ashlar],
            &[F::Flagstone, F::Slabs],
            &[C::Vault],
            &[Pilaster::Cluster, Pilaster::Block],
            &[B::Pointed],
            &[Rib::Pointed],
            &[Door::Arch],
            &[Pl::Cluster],
            Some(Bands::Plinth),
            false,
            [
                Some(0.5),
                Some(0.3),
                Some(0.5),
                Some(0.5),
                N,
                Some(0.5),
                Some(0.5),
                Some(0.9),
                N,
            ],
        ),
        Theme::Mine => r(
            [0.38, 0.28, 0.19],
            [0.28, 0.21, 0.15],
            &[W::Rock],
            &[F::Dirt],
            &[C::Rock],
            &[Pilaster::None],
            &[B::None],
            &[Rib::None],
            &[Door::Timber],
            &[Pl::Timber],
            None,
            false,
            [
                Some(1.0),
                Some(0.1),
                Some(0.3),
                Some(0.4),
                Some(0.2),
                N,
                N,
                N,
                N,
            ],
        ),
        Theme::Fungal => r(
            [0.24, 0.3, 0.3],
            [0.18, 0.23, 0.22],
            &[W::Rubble, W::Rock],
            &[F::Flagstone, F::Cobbles, F::Dirt],
            &[C::Rock],
            &[Pilaster::Rough],
            &[B::None, B::Round],
            &[Rib::None],
            &[Door::Rough, Door::Arch],
            &[Pl::Rough],
            None,
            false,
            [Some(0.5), Some(1.0), Some(0.8), Some(0.8), N, N, N, N, N],
        ),
        Theme::Forge => Recipe {
            plaster: Some([0.62, 0.55, 0.46]),
            ..r(
                [0.42, 0.2, 0.14],
                [0.26, 0.19, 0.16],
                &[W::Brick, W::Plaster],
                &[F::Bricks, F::Slabs],
                &[C::Timber, C::Vault],
                &[Pilaster::Brick],
                &[B::Round],
                &[Rib::Round, Rib::Beam],
                &[Door::Frame],
                &[Pl::Brick],
                None,
                false,
                [
                    Some(0.4),
                    Some(0.0),
                    N,
                    N,
                    Some(1.0),
                    N,
                    Some(0.3),
                    N,
                    Some(0.4),
                ],
            )
        },
        Theme::Library => Recipe {
            floor2: Some([0.2, 0.16, 0.12]),
            plaster: Some([0.7, 0.62, 0.5]),
            ..r(
                [0.5, 0.4, 0.28],
                [0.36, 0.22, 0.12],
                &[W::Panelled, W::Plaster],
                &[F::Planks, F::Checker],
                &[C::Coffered, C::Timber],
                &[Pilaster::Ornate],
                &[B::Round],
                &[Rib::Round],
                &[Door::Arch],
                &[Pl::Ornate],
                Some(Bands::Full),
                true,
                [
                    Some(0.2),
                    Some(0.2),
                    Some(0.45),
                    Some(0.3),
                    N,
                    N,
                    N,
                    Some(0.5),
                    N,
                ],
            )
        },
        Theme::Overgrowth => r(
            [0.3, 0.33, 0.27],
            [0.24, 0.26, 0.2],
            &[W::Ashlar, W::Rubble],
            &[F::Flagstone, F::Dirt],
            &[C::Rock],
            &[Pilaster::Block, Pilaster::Rough],
            &[B::Round, B::None],
            &[Rib::Round],
            &[Door::Arch, Door::Rough],
            &[Pl::Octagon, Pl::Rough],
            None,
            false,
            [Some(0.6), Some(1.0), Some(0.7), Some(0.6), N, N, N, N, N],
        ),
    }
}

/// How a room has aged (0 flooded, 1 scorched, 2 ancient, 3 overgrown, 4 pristine).
fn condition_wear(k: usize) -> [f32; 9] {
    match k {
        0 => [0.0, 0.25, 0.5, 0.5, -0.4, 0.0, 0.0, 0.0, 0.0],
        1 => [0.0, -0.5, -0.2, 0.0, 0.7, 0.0, 0.0, 0.0, 0.15],
        2 => [0.3, 0.2, 0.0, 0.0, 0.0, 0.5, 0.4, 0.0, 0.0],
        3 => [0.0, 0.6, 0.3, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        _ => [-0.4, 0.0, 0.0, -0.3, -0.3, 0.0, -0.3, -0.4, 0.0],
    }
}

/// A room's finish (web `finishOf`).
#[derive(Clone, Copy, Debug)]
pub struct Finish {
    pub theme: Theme,
    pub condition: Option<usize>,
    pub seed: Seed,
    pub stone: C3,
    pub floor: C3,
    pub floor2: C3,
    pub wood: C3,
    pub plaster: C3,
    pub mortar: C3,
    pub wall: Wall,
    pub floor_kind: FloorKind,
    pub ceiling: CeilingKind,
    pub pilaster: Pilaster,
    pub bay: BayKind,
    pub rib: Rib,
    pub door: Door,
    pub pillar: PillarKind,
    pub bands: Option<Bands>,
    pub ridge: bool,
    pub accent: C3,
    pub wear: Wear,
}

const WOOD: C3 = [0.34, 0.2, 0.11];

/// The finish of a themed room, or `None` for signature halls and plain rooms.
pub fn finish_of(map: &Blueprint, room: &Room, biome: &Biome) -> Option<Finish> {
    if room.shape == Shape::Bridge {
        return None;
    }
    let theme = themes::of(map, room)?;
    let rec = recipe(theme);
    let mut rng = ArtRng::stream(map.seed, "room_finish_v1", room.id as u64);
    let style = super::super::surfaces::style(biome);
    let (biome_stone, biome_floor) = (style.stone, style.floor);
    let drift = |rng: &mut ArtRng| {
        [
            rng.range(0.9, 1.1),
            rng.range(0.92, 1.08),
            rng.range(0.9, 1.1),
        ]
    };
    let jitter = |rng: &mut ArtRng, c: C3| {
        let d = drift(rng);
        [c[0] * d[0], c[1] * d[1], c[2] * d[2]]
    };
    let shade = rng.range(0.88, 1.12);
    let condition = if rng.chance(0.65) {
        Some(rng.index(5))
    } else {
        None
    };
    let age = |c: C3| -> C3 {
        let mut out = c;
        match condition {
            Some(2) => out = mix_color(out, [0.55, 0.55, 0.56], 0.22),
            Some(3) => out = mix_color(out, [0.3, 0.42, 0.25], 0.14),
            _ => {}
        }
        match condition {
            Some(0) => out = [out[0] * 0.86, out[1] * 0.9, out[2] * 0.96],
            Some(1) => out = [out[0] * 0.76, out[1] * 0.72, out[2] * 0.7],
            _ => {}
        }
        out
    };
    let stone = age(tint(
        jitter(&mut rng, mix_color(rec.stone, biome_stone, 0.3)),
        shade,
    ));
    let floor = age(tint(
        jitter(&mut rng, mix_color(rec.floor, biome_floor, 0.3)),
        shade,
    ));
    let wall = rec.walls[rng.index(rec.walls.len())];
    let floor_kind = rec.floors[rng.index(rec.floors.len())];
    let ceiling = rec.ceilings[rng.index(rec.ceilings.len())];
    let accent = match room.biome {
        0 => [0.72, 0.5, 0.24],
        1 => [0.2, 0.58, 0.6],
        _ => [0.78, 0.3, 0.14],
    };
    let base = [0.5, 0.3, 0.3, 0.3, 0.0, 0.0, 0.2, 0.0, 0.0];
    let extra = condition.map(condition_wear).unwrap_or([0.0; 9]);
    let mut w = [0.0f32; 9];
    for k in 0..9 {
        let value = rec.wear[k].unwrap_or(base[k]);
        w[k] = (value * rng.range(0.75, 1.25) + extra[k]).clamp(0.0, 1.0);
    }
    let wear = Wear {
        rubble: w[0],
        moss: w[1],
        damp: w[2],
        streaks: w[3],
        soot: w[4],
        salt: w[5],
        cracks: w[6],
        webs: w[7],
        heat: w[8],
    };
    let seed = rng.next_seed();
    let floor2 = match rec.floor2 {
        Some(c) => jitter(&mut rng, c),
        None => tint(floor, 0.55),
    };
    let wood_shade = rng.range(0.85, 1.15);
    let wood = jitter(&mut rng, tint(WOOD, wood_shade));
    let plaster = jitter(
        &mut rng,
        rec.plaster
            .unwrap_or_else(|| mix_color(stone, [0.72, 0.66, 0.56], 0.55)),
    );
    let pilaster = rec.pilaster[rng.index(rec.pilaster.len())];
    let bay = rec.bay[rng.index(rec.bay.len())];
    let rib = rec.rib[rng.index(rec.rib.len())];
    let door = rec.door[rng.index(rec.door.len())];
    let pillar = rec.pillar[rng.index(rec.pillar.len())];
    Some(Finish {
        theme,
        condition,
        seed,
        stone,
        floor,
        floor2,
        wood,
        plaster,
        mortar: tint(stone, 0.45),
        wall,
        floor_kind,
        ceiling,
        pilaster,
        bay,
        rib,
        door,
        pillar,
        bands: rec.bands,
        ridge: rec.ridge,
        accent,
        wear,
    })
}

// ------------------------------------------------------------ weathering

fn hash(x: i32, y: i32, z: i32) -> f64 {
    let mut h = x.wrapping_mul(374761393) ^ y.wrapping_mul(668265263) ^ z.wrapping_mul(2147483647);
    h = (h ^ ((h as u32) >> 13) as i32).wrapping_mul(1274126177);
    ((h ^ ((h as u32) >> 16) as i32) as u32) as f64 / 4294967296.0
}

/// Smooth 3D value noise in [0, 1].
pub fn noise3(x: f64, y: f64, z: f64) -> f64 {
    let (xi, yi, zi) = (x.floor(), y.floor(), z.floor());
    let (xf, yf, zf) = (x - xi, y - yi, z - zi);
    let s = |t: f64| t * t * (3.0 - 2.0 * t);
    let (u, v, w) = (s(xf), s(yf), s(zf));
    let l = |a: f64, b: f64, t: f64| a + (b - a) * t;
    let (xi, yi, zi) = (xi as i32, yi as i32, zi as i32);
    let c = |dx: i32, dy: i32, dz: i32| hash(xi + dx, yi + dy, zi + dz);
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

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Surface {
    Wall,
    Floor,
    Ceiling,
}

/// Bake the room's wear into vertex colours [from, to).
pub fn weather(
    m: &mut velvet_render::Mesh3D,
    from: usize,
    to: usize,
    finish: Option<&Finish>,
    room: &Room,
    kind: Surface,
    sources: &[velvet_math::Vec3],
) {
    let Some(finish) = finish else { return };
    let w = finish.wear;
    let (w_damp, w_moss, w_streaks, w_soot, w_salt, w_heat) = (
        w.damp as f64,
        w.moss as f64,
        w.streaks as f64,
        w.soot as f64,
        w.salt as f64,
        w.heat as f64,
    );
    let base = room.center.y as f64;
    let h = room.height as f64;
    let damp = [0.62, 0.66, 0.6];
    let moss = [0.72, 0.95, 0.55];
    for i in from..to {
        let p = m.vertices[i].position;
        let (x, y, z) = (p[0] as f64, p[1] as f64 - base, p[2] as f64);
        let (mut r, mut g, mut b) = (1.0f64, 1.0f64, 1.0f64);
        let mut mul = |c: [f64; 3], t: f64| {
            r *= 1.0 + (c[0] - 1.0) * t;
            g *= 1.0 + (c[1] - 1.0) * t;
            b *= 1.0 + (c[2] - 1.0) * t;
        };
        let n1 = noise3(x * 0.9, y * 0.9, z * 0.9);
        match kind {
            Surface::Wall => {
                let foot = (1.0 - y / (0.5 + w_damp * 0.9)).max(0.0);
                mul(damp, foot * w_damp * (0.55 + 0.45 * n1));
                mul(moss, foot * w_moss * 0.7 * (n1 - 0.35).max(0.0) * 1.6);
                if w_streaks > 0.0 {
                    let band = noise3(x * 3.1 + z * 3.1, 0.5, (x - z) * 0.7);
                    let streak = (band - 0.62).max(0.0) / 0.38;
                    let fall = ((y - h * 0.15) / (h * 0.7)).clamp(0.0, 1.0);
                    mul([0.55, 0.58, 0.56], streak * fall * w_streaks);
                }
                if w_soot > 0.0 {
                    let high = ((y - h * 0.35) / (h * 0.55)).clamp(0.0, 1.0);
                    let mut near = 0.0f64;
                    for s in sources {
                        let d = (x - s.x as f64).hypot(z - s.z as f64);
                        near =
                            near.max((1.0 - d / 1.1).max(0.0) * ((y - 1.2) / 2.0).clamp(0.0, 1.0));
                    }
                    mul(
                        [0.35, 0.33, 0.32],
                        (high * 0.8 + near).min(1.0) * w_soot * (0.6 + 0.4 * n1),
                    );
                }
                if w_salt > 0.0 {
                    let bloom = (noise3(x * 1.7, y * 1.7, z * 1.7) - 0.66).max(0.0) / 0.34;
                    mul([1.35, 1.33, 1.28], bloom * w_salt);
                }
                if w_heat > 0.0 {
                    mul(
                        [1.45, 0.8, 0.62],
                        (1.0 - y / 1.4).max(0.0) * w_heat * 0.6 * n1,
                    );
                }
            }
            Surface::Floor => {
                let patch = (noise3(x * 0.45, 7.3, z * 0.45) - 0.45).max(0.0) / 0.55;
                mul([0.72, 0.7, 0.68], patch * 0.8);
                mul(
                    damp,
                    (noise3(x * 1.3, 3.1, z * 1.3) - 0.55).max(0.0) * 2.0 * w_damp,
                );
                if w_moss > 0.4 {
                    mul(
                        moss,
                        (noise3(x * 0.8, 11.7, z * 0.8) - 0.55).max(0.0) * 2.0 * (w_moss - 0.3),
                    );
                }
                if w_soot > 0.0 {
                    mul(
                        [0.45, 0.42, 0.4],
                        (noise3(x * 0.7, 5.5, z * 0.7) - 0.5).max(0.0) * 2.0 * w_soot,
                    );
                }
                if w_heat > 0.0 {
                    mul(
                        [1.3, 0.8, 0.66],
                        (noise3(x * 0.6, 9.1, z * 0.6) - 0.6).max(0.0) * 2.5 * w_heat,
                    );
                }
            }
            Surface::Ceiling => {
                mul(
                    [0.6, 0.58, 0.56],
                    (0.4 + 0.6 * n1) * (w_soot * 0.8 + w_damp * 0.3),
                );
                if w_moss > 0.5 {
                    mul(moss, (n1 - 0.6).max(0.0) * 2.0 * w_moss);
                }
            }
        }
        let c = &mut m.vertices[i].color;
        c[0] = (c[0] as f64 * r) as f32;
        c[1] = (c[1] as f64 * g) as f32;
        c[2] = (c[2] as f64 * b) as f32;
    }
}
