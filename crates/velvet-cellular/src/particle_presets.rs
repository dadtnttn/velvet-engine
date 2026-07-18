//! Named particle FX presets for authors (blood, sparks, dig, spells).

use crate::cell::MaterialId;
use crate::particles::{ParticleBurst, ParticleEnd, ParticleEmitter, ParticleWorld};
use crate::world::World;

/// Preset identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParticlePreset {
    /// Blood gush.
    BloodGush,
    /// Fine blood mist.
    BloodMist,
    /// Fire sparks up.
    FireSparks,
    /// Ember rain.
    EmberRain,
    /// Sand dump.
    SandDump,
    /// Water splash.
    WaterSplash,
    /// Acid spit.
    AcidSpit,
    /// Dig debris.
    DigDebris,
    /// Smoke puff.
    SmokePuff,
    /// Magic glitter.
    MagicGlitter,
    /// Oil drip.
    OilDrip,
    /// Lava spit.
    LavaSpit,
    /// Steam jet.
    SteamJet,
    /// Poison cloud.
    PoisonCloud,
    /// Snow flurry.
    SnowFlurry,
    /// Bone shards.
    BoneShards,
    /// Metal sparks.
    MetalSparks,
    /// Healing spores.
    HealSpores,
    /// Void dust.
    VoidDust,
    /// Impact dust.
    ImpactDust,
}

/// Play a preset at position.
pub fn play_preset(
    world: &World,
    particles: &mut ParticleWorld,
    preset: ParticlePreset,
    x: f32,
    y: f32,
    scale: f32,
) -> u32 {
    let scale = scale.clamp(0.25, 4.0);
    let count = |base: u32| ((base as f32) * scale).round() as u32;
    match preset {
        ParticlePreset::BloodGush => {
            let m = world.mat("blood");
            particles.burst_blood(x, y, m, count(28))
        }
        ParticlePreset::BloodMist => {
            let m = world.mat("blood");
            particles.burst(&ParticleBurst {
                x,
                y,
                material: m,
                count: count(40),
                speed_min: 1.0,
                speed_max: 6.0,
                lifetime: 1.8,
                full_circle: true,
                end: ParticleEnd::ConvertToCell,
                gravity_scale: 0.6,
                temp: 36.0,
                ..Default::default()
            })
        }
        ParticlePreset::FireSparks => {
            let m = world.mat("fire");
            particles.burst_sparks(x, y, m, count(22))
        }
        ParticlePreset::EmberRain => {
            let m = world.mat("fire");
            particles.burst(&ParticleBurst {
                x,
                y: y + 8.0,
                material: m,
                count: count(30),
                speed_min: 1.0,
                speed_max: 4.0,
                lifetime: 2.0,
                full_circle: false,
                angle: -1.57,
                cone: 0.8,
                end: ParticleEnd::HeatOnly,
                gravity_scale: 0.8,
                temp: 700.0,
                ..Default::default()
            })
        }
        ParticlePreset::SandDump => {
            let m = world.mat("sand");
            particles.burst(&ParticleBurst {
                x,
                y,
                material: m,
                count: count(36),
                speed_min: 0.5,
                speed_max: 3.0,
                lifetime: 2.5,
                full_circle: false,
                angle: -1.57,
                cone: 0.5,
                end: ParticleEnd::ConvertToCell,
                gravity_scale: 1.2,
                temp: 20.0,
                ..Default::default()
            })
        }
        ParticlePreset::WaterSplash => {
            let m = world.mat("water");
            particles.burst(&ParticleBurst {
                x,
                y,
                material: m,
                count: count(32),
                speed_min: 4.0,
                speed_max: 14.0,
                lifetime: 1.2,
                full_circle: true,
                end: ParticleEnd::ConvertToCell,
                gravity_scale: 1.0,
                temp: 15.0,
                ..Default::default()
            })
        }
        ParticlePreset::AcidSpit => {
            let m = world.mat("acid");
            particles.burst(&ParticleBurst {
                x,
                y,
                material: m,
                count: count(18),
                speed_min: 8.0,
                speed_max: 18.0,
                lifetime: 1.0,
                full_circle: false,
                angle: 0.4,
                cone: 0.4,
                end: ParticleEnd::ConvertToCell,
                gravity_scale: 0.9,
                temp: 20.0,
                ..Default::default()
            })
        }
        ParticlePreset::DigDebris => particles.burst_dig(x, y, count(16)),
        ParticlePreset::SmokePuff => {
            let m = world.mat("smoke");
            particles.burst(&ParticleBurst {
                x,
                y,
                material: m,
                count: count(20),
                speed_min: 1.0,
                speed_max: 5.0,
                lifetime: 2.0,
                full_circle: true,
                end: ParticleEnd::ConvertToCell,
                gravity_scale: -0.3,
                temp: 40.0,
                ..Default::default()
            })
        }
        ParticlePreset::MagicGlitter => {
            let m = world.mat("magic_dust");
            let m = if m.is_air() { world.mat("sand") } else { m };
            particles.burst(&ParticleBurst {
                x,
                y,
                material: m,
                count: count(24),
                speed_min: 2.0,
                speed_max: 9.0,
                lifetime: 1.5,
                full_circle: true,
                end: ParticleEnd::Vanish,
                gravity_scale: 0.1,
                temp: 20.0,
                ..Default::default()
            })
        }
        ParticlePreset::OilDrip => {
            let m = world.mat("oil");
            particles.burst(&ParticleBurst {
                x,
                y,
                material: m,
                count: count(12),
                speed_min: 0.2,
                speed_max: 2.0,
                lifetime: 2.0,
                full_circle: false,
                angle: -1.57,
                cone: 0.2,
                end: ParticleEnd::ConvertToCell,
                gravity_scale: 1.0,
                temp: 20.0,
                ..Default::default()
            })
        }
        ParticlePreset::LavaSpit => {
            let m = world.mat("lava");
            particles.burst(&ParticleBurst {
                x,
                y,
                material: m,
                count: count(10),
                speed_min: 5.0,
                speed_max: 12.0,
                lifetime: 1.6,
                full_circle: false,
                angle: 1.2,
                cone: 0.5,
                end: ParticleEnd::ConvertToCell,
                gravity_scale: 1.0,
                temp: 1100.0,
                ..Default::default()
            })
        }
        ParticlePreset::SteamJet => {
            let m = world.mat("steam");
            particles.burst(&ParticleBurst {
                x,
                y,
                material: m,
                count: count(26),
                speed_min: 6.0,
                speed_max: 16.0,
                lifetime: 1.0,
                full_circle: false,
                angle: 1.57,
                cone: 0.35,
                end: ParticleEnd::ConvertToCell,
                gravity_scale: -0.4,
                temp: 130.0,
                ..Default::default()
            })
        }
        ParticlePreset::PoisonCloud => {
            let m = world.mat("poison");
            particles.burst(&ParticleBurst {
                x,
                y,
                material: m,
                count: count(30),
                speed_min: 0.5,
                speed_max: 3.5,
                lifetime: 2.5,
                full_circle: true,
                end: ParticleEnd::ConvertToCell,
                gravity_scale: 0.15,
                temp: 20.0,
                ..Default::default()
            })
        }
        ParticlePreset::SnowFlurry => {
            let m = world.mat("snow");
            let m = if m.is_air() { world.mat("ice") } else { m };
            particles.burst(&ParticleBurst {
                x,
                y: y + 6.0,
                material: m,
                count: count(40),
                speed_min: 0.5,
                speed_max: 2.5,
                lifetime: 3.0,
                full_circle: false,
                angle: -1.4,
                cone: 1.0,
                end: ParticleEnd::ConvertToCell,
                gravity_scale: 0.4,
                temp: -5.0,
                ..Default::default()
            })
        }
        ParticlePreset::BoneShards => {
            let m = world.mat("bone");
            particles.burst(&ParticleBurst {
                x,
                y,
                material: m,
                count: count(14),
                speed_min: 6.0,
                speed_max: 16.0,
                lifetime: 1.0,
                full_circle: true,
                end: ParticleEnd::ConvertToCell,
                gravity_scale: 1.0,
                temp: 20.0,
                ..Default::default()
            })
        }
        ParticlePreset::MetalSparks => {
            let m = world.mat("fire");
            particles.burst(&ParticleBurst {
                x,
                y,
                material: m,
                count: count(18),
                speed_min: 10.0,
                speed_max: 26.0,
                lifetime: 0.4,
                full_circle: true,
                end: ParticleEnd::HeatOnly,
                gravity_scale: 0.5,
                temp: 1200.0,
                ..Default::default()
            })
        }
        ParticlePreset::HealSpores => {
            let m = world.mat("spore");
            let m = if m.is_air() { world.mat("grass") } else { m };
            particles.burst(&ParticleBurst {
                x,
                y,
                material: m,
                count: count(20),
                speed_min: 1.0,
                speed_max: 4.0,
                lifetime: 2.0,
                full_circle: true,
                end: ParticleEnd::ConvertToCell,
                gravity_scale: -0.1,
                temp: 20.0,
                ..Default::default()
            })
        }
        ParticlePreset::VoidDust => {
            let m = world.mat("void_dust");
            let m = if m.is_air() { world.mat("ash") } else { m };
            particles.burst(&ParticleBurst {
                x,
                y,
                material: m,
                count: count(16),
                speed_min: 2.0,
                speed_max: 8.0,
                lifetime: 1.2,
                full_circle: true,
                end: ParticleEnd::Vanish,
                gravity_scale: 0.0,
                temp: 0.0,
                ..Default::default()
            })
        }
        ParticlePreset::ImpactDust => {
            let m = world.mat("dust");
            let m = if m.is_air() { world.mat("sand") } else { m };
            particles.burst(&ParticleBurst {
                x,
                y,
                material: m,
                count: count(22),
                speed_min: 3.0,
                speed_max: 10.0,
                lifetime: 0.9,
                full_circle: true,
                end: ParticleEnd::ConvertToCell,
                gravity_scale: 0.7,
                temp: 20.0,
                ..Default::default()
            })
        }
    }
}

/// Attach a continuous emitter preset.
pub fn attach_emitter_preset(
    world: &World,
    particles: &mut ParticleWorld,
    preset: ParticlePreset,
    x: f32,
    y: f32,
    rate: f32,
) -> u32 {
    let (mat, end, grav, temp, angle) = match preset {
        ParticlePreset::FireSparks | ParticlePreset::EmberRain | ParticlePreset::MetalSparks => {
            (world.mat("fire"), ParticleEnd::HeatOnly, 0.3, 800.0, 1.57)
        }
        ParticlePreset::BloodGush | ParticlePreset::BloodMist => {
            (world.mat("blood"), ParticleEnd::ConvertToCell, 1.0, 36.0, -1.2)
        }
        ParticlePreset::WaterSplash => {
            (world.mat("water"), ParticleEnd::ConvertToCell, 1.0, 15.0, 1.0)
        }
        ParticlePreset::SteamJet => {
            (world.mat("steam"), ParticleEnd::ConvertToCell, -0.3, 120.0, 1.57)
        }
        ParticlePreset::SmokePuff => {
            (world.mat("smoke"), ParticleEnd::ConvertToCell, -0.2, 40.0, 1.4)
        }
        _ => (world.mat("sand"), ParticleEnd::ConvertToCell, 1.0, 20.0, -1.57),
    };
    let mut e = ParticleEmitter::new(0, x, y, mat, rate);
    e.end = end;
    e.gravity_scale = grav;
    e.temp = temp;
    e.angle = angle;
    e.cone = 0.7;
    particles.add_emitter(e)
}

/// All presets for iteration/UI.
pub fn all_presets() -> &'static [ParticlePreset] {
    &[
        ParticlePreset::BloodGush,
        ParticlePreset::BloodMist,
        ParticlePreset::FireSparks,
        ParticlePreset::EmberRain,
        ParticlePreset::SandDump,
        ParticlePreset::WaterSplash,
        ParticlePreset::AcidSpit,
        ParticlePreset::DigDebris,
        ParticlePreset::SmokePuff,
        ParticlePreset::MagicGlitter,
        ParticlePreset::OilDrip,
        ParticlePreset::LavaSpit,
        ParticlePreset::SteamJet,
        ParticlePreset::PoisonCloud,
        ParticlePreset::SnowFlurry,
        ParticlePreset::BoneShards,
        ParticlePreset::MetalSparks,
        ParticlePreset::HealSpores,
        ParticlePreset::VoidDust,
        ParticlePreset::ImpactDust,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builtin::builtin_registry;
    use crate::particles::ParticleWorld;
    use crate::world::{World, WorldConfig};

    #[test]
    fn all_presets_spawn_something() {
        let (reg, _) = builtin_registry();
        let world = World::new(reg, WorldConfig::default());
        let mut pw = ParticleWorld::default();
        for p in all_presets() {
            let before = pw.len() + pw.spawns as usize;
            play_preset(&world, &mut pw, *p, 0.0, 10.0, 1.0);
            assert!(
                pw.len() > 0 || pw.spawns as usize > before,
                "preset {p:?} should spawn"
            );
        }
    }
}
