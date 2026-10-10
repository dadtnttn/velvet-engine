//! The vault-shaped plan a themed room fills (web prototype `vaults.js`
//! plans): deferred art pieces, authored props, lights, flames, sparkles and
//! solids, plus the placer that keeps doorways, spawns and pillars clear.
use super::kit::*;
use std::sync::Arc;
use velvet_dungeon::{Blueprint, Room};
use velvet_math::Vec3;
use velvet_render::Mesh3D;

/// What a piece builder receives: the room's stone, its glow tint, the
/// shared art stream (consumed in piece order) and the water list.
pub struct Ctx {
    pub stone: C3,
    pub glow_color: C3,
    pub rng: ArtRng,
    pub wet: Vec<Pond>,
}

/// A deferred art piece: writes into the lit mesh and the emissive mesh.
pub type Build = Arc<dyn Fn(&mut Mesh3D, &mut Mesh3D, &mut Ctx) + Send + Sync>;

/// One water surface for the water shader (web prototype `ponds.js`).
#[derive(Clone, Debug)]
pub struct Pond {
    pub center: Vec3,
    pub outline: Vec<Vec3>,
    pub color: C3,
    pub reflect: C3,
    pub depth: f32,
    pub halo: f32,
    /// Height a leak's drops fall from.
    pub drip: Option<f32>,
}

#[derive(Clone, Copy, Debug)]
pub struct Prop {
    pub asset: &'static str,
    pub position: Vec3,
    pub yaw: f32,
    pub height: f32,
    pub radius: f32,
}
#[derive(Clone, Copy, Debug)]
pub struct Mount {
    pub asset: &'static str,
    pub anchor: Vec3,
    pub yaw: f32,
    pub height: f32,
}
#[derive(Clone, Copy, Debug)]
pub struct Light {
    pub position: Vec3,
    pub color: C3,
    pub intensity: f32,
    pub radius: f32,
    pub beat: bool,
}
#[derive(Clone, Copy, Debug)]
pub struct Fixture {
    pub p: Vec3,
    pub color: C3,
    pub size: f32,
}
#[derive(Clone, Copy, Debug)]
pub struct Weapon {
    pub name: &'static str,
    pub position: Vec3,
    pub height: f32,
    pub rotation: Vec3,
}
#[derive(Clone, Copy, Debug)]
pub struct RotatedProp {
    pub asset: &'static str,
    pub position: Vec3,
    pub yaw: f32,
    pub height: f32,
    pub roll: f32,
}

#[derive(Clone)]
pub struct Plan {
    pub focus: Vec3,
    pub inward: Vec3,
    pub props: Vec<Prop>,
    pub mounts: Vec<Mount>,
    pub pieces: Vec<Build>,
    /// Pieces that beat with the Heart (drawn with a pulsing glow).
    pub pulse_pieces: Vec<Build>,
    pub lights: Vec<Light>,
    pub fixtures: Vec<Fixture>,
    pub sparkles: Vec<Vec3>,
    pub sparkle_tint: Option<C3>,
    pub solids: Vec<(Vec3, f32)>,
    pub wall_solids: Vec<(Vec3, f32)>,
    pub weapons: Vec<Weapon>,
    pub rotated_props: Vec<RotatedProp>,
    pub used_slots: Vec<Vec3>,
    /// Anchors of wall-standing pieces (web: pieces with `inward` and a centre).
    pub wall_pieces: Vec<Vec3>,
    pub replace_pillars: bool,
    pub heart: Option<(Vec3, f32)>,
    pub orrery: Option<Vec3>,
    /// Signature chambers (web `vaults.js`): which one this plan dresses.
    pub vault: Option<VaultKind>,
    /// Gold mounds: (centre, radius, height).
    pub hoards: Vec<(Vec3, f32, f32)>,
    /// Relics on altars inside the grand treasury's niches: (base, relic, yaw).
    pub niche_relics: Vec<(Vec3, &'static str, f32)>,
    pub obelisks: Vec<Vec3>,
    pub sconces: Vec<Sconce>,
    pub open_chests: Vec<(Vec3, f32)>,
    /// Lights in the room's glow colour: (position, intensity, radius).
    pub glow_lights: Vec<(Vec3, f32, f32)>,
    pub medallion_radius: f32,
    pub medallion_y: f32,
    pub font_core: Option<Vec3>,
    pub shaft: Option<Shaft>,
    pub host_pillar: Option<Vec3>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VaultKind {
    Treasury,
    Arcane,
    Sanctuary,
}

/// A wall torch: its backplate, facing and flame.
#[derive(Clone, Copy, Debug)]
pub struct Sconce {
    pub backplate: Vec3,
    pub yaw: f32,
    pub flame: Vec3,
}

/// A shaft of light falling on the relic or the rest font.
#[derive(Clone, Copy, Debug)]
pub struct Shaft {
    pub top: Vec3,
    pub bottom: Vec3,
    pub radius_top: f32,
    pub radius_bottom: f32,
}

impl std::fmt::Debug for Plan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Plan")
            .field("pieces", &self.pieces.len())
            .field("props", &self.props.len())
            .finish()
    }
}

impl Plan {
    pub fn new(focus: Vec3, inward: Vec3) -> Self {
        Self {
            focus,
            inward,
            props: vec![],
            mounts: vec![],
            pieces: vec![],
            pulse_pieces: vec![],
            lights: vec![],
            fixtures: vec![],
            sparkles: vec![],
            sparkle_tint: None,
            solids: vec![],
            wall_solids: vec![],
            weapons: vec![],
            rotated_props: vec![],
            used_slots: vec![],
            wall_pieces: vec![],
            vault: None,
            hoards: vec![],
            niche_relics: vec![],
            obelisks: vec![],
            sconces: vec![],
            open_chests: vec![],
            glow_lights: vec![],
            medallion_radius: 1.3,
            medallion_y: 0.0,
            font_core: None,
            shaft: None,
            host_pillar: None,
            replace_pillars: false,
            heart: None,
            orrery: None,
        }
    }
    pub fn piece(
        &mut self,
        build: impl Fn(&mut Mesh3D, &mut Mesh3D, &mut Ctx) + Send + Sync + 'static,
    ) {
        self.pieces.push(Arc::new(build));
    }
    pub fn pulse(
        &mut self,
        build: impl Fn(&mut Mesh3D, &mut Mesh3D, &mut Ctx) + Send + Sync + 'static,
    ) {
        self.pulse_pieces.push(Arc::new(build));
    }
    pub fn light(&mut self, position: Vec3, color: C3, intensity: f32, radius: f32) {
        self.lights.push(Light {
            position,
            color,
            intensity,
            radius,
            beat: false,
        });
    }
    /// Every solid footprint (web `vaultColliders`).
    pub fn colliders(&self) -> Vec<(Vec3, f32)> {
        let mut out: Vec<(Vec3, f32)> = self
            .props
            .iter()
            .filter(|p| p.radius > 0.0)
            .map(|p| (p.position, p.radius))
            .collect();
        // Web `vaultColliders`: hoards, obelisks and the planned solids.
        out.extend(self.hoards.iter().map(|&(p, r, _)| (p, r * 0.9)));
        out.extend(self.obelisks.iter().map(|&p| (p, 0.34)));
        out.extend(self.solids.iter().copied());
        out
    }
}

pub fn segment_distance(p: Vec3, a: Vec3, b: Vec3) -> f32 {
    let d = hz(b - a);
    let t = if d.length_squared() > 1e-4 {
        (hz(p - a).dot(d) / d.length_squared()).clamp(0.0, 1.0)
    } else {
        0.0
    };
    hz(p - (a + d * t)).length()
}

/// Keeps doorways, their approach to the focus, spawns and pillars clear.
pub struct Placer<'a> {
    pub map: &'a Blueprint,
    pub room: &'a Room,
    pub focus: Vec3,
    pub taken: Vec<(Vec3, f32)>,
}

impl<'a> Placer<'a> {
    pub fn new(map: &'a Blueprint, room: &'a Room, focus: Vec3) -> Self {
        Self {
            map,
            room,
            focus,
            taken: vec![],
        }
    }
    /// `obstacle_gap`: clearance to cover pillars (smaller for wall dressing).
    pub fn try_place(
        &mut self,
        candidates: &[Vec3],
        radius: f32,
        focus_gap: f32,
        obstacle_gap: f32,
    ) -> Option<Vec3> {
        for &c in candidates {
            if !self.room.contains(c.x, c.z, radius + 0.12) {
                continue;
            }
            let p = with_y(c, self.room.floor_y(c.x, c.z));
            if self.clear(p, radius, focus_gap, obstacle_gap) {
                self.taken.push((p, radius));
                return Some(p);
            }
        }
        None
    }
    pub fn clear(&self, p: Vec3, radius: f32, focus_gap: f32, obstacle_gap: f32) -> bool {
        let (map, room, focus) = (self.map, self.room, self.focus);
        room.sockets.iter().all(|s| {
            hdist(s.position, p) > s.width * 0.5 + radius + 1.2
                && segment_distance(p, s.position, focus) > radius + 1.0
        }) && hdist(p, focus) > radius + focus_gap
            && map
                .spawns
                .iter()
                .filter(|s| s.room == room.id && hdist(s.position, focus) > 0.01)
                .all(|s| hdist(s.position, p) > radius + 1.4)
            && map
                .obstacles
                .iter()
                .filter(|o| o.room == room.id)
                .all(|o| hdist(o.center, p) > radius + obstacle_gap)
            && self
                .taken
                .iter()
                .all(|&(q, r)| hdist(q, p) > radius + r + 0.3)
    }
    /// Small non-solid dressing: inside, out of doorways and off every solid.
    pub fn floor_spot(&self, p: Vec3, radius: f32) -> Option<Vec3> {
        let (map, room) = (self.map, self.room);
        if !room.contains(p.x, p.z, radius + 0.15) {
            return None;
        }
        let q = with_y(p, room.floor_y(p.x, p.z));
        let ok = room
            .sockets
            .iter()
            .all(|s| hdist(s.position, q) > s.width * 0.5 + radius + 0.5)
            && self
                .taken
                .iter()
                .all(|&(t, r)| hdist(t, q) > radius + r + 0.08)
            && map
                .obstacles
                .iter()
                .filter(|o| o.room == room.id)
                .all(|o| hdist(o.center, q) > radius + 0.7);
        ok.then_some(q)
    }
}

/// The room frame from its first doorway: inward, right, depth and width.
pub fn frame(room: &Room) -> (Vec3, Vec3, f32, f32) {
    let mut inward = room
        .sockets
        .first()
        .map(|s| norm(v(-s.outward.x, 0.0, -s.outward.z)))
        .unwrap_or(VZ);
    if inward.length_squared() < 0.5 {
        inward = VZ;
    }
    let right = v(inward.z, 0.0, -inward.x);
    let depth = inward.x.abs() * room.half[0] + inward.z.abs() * room.half[1];
    let width = right.x.abs() * room.half[0] + right.z.abs() * room.half[1];
    (inward, right, depth, width)
}
