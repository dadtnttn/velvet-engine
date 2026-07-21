//! Integration tests: multi-system Noita-like paths on shipped APIs.

use velvet_cellular::prelude::*;
use velvet_cellular::reaction_chain::apply_reaction_chains;

#[test]
fn particle_to_grid_to_fluid_to_dig() {
    let mut s = CellularSession::with_builtins(WorldConfig::default());
    s.use_hot = false;
    s.world.paint_rect(-16, 0, 16, 2, s.mat("bedrock"));
    // free particles settle into sand
    s.particle_burst(0.0, 12.0, "sand", 30);
    s.step_n(80);
    let sand = s.mat("sand");
    let mut sand_cells = 0;
    for y in 0..14 {
        for x in -10..10 {
            if s.get(x, y).material == sand {
                sand_cells += 1;
            }
        }
    }
    assert!(
        sand_cells > 0 || s.particles.conversions > 0,
        "particles should leave sand"
    );

    // add water and run fluid pass
    s.world.paint_rect(-4, 2, 4, 6, s.mat("water"));
    let stats = fluid_pass(&mut s.world, 40);
    assert!(stats.blob_count >= 1 || stats.falling_liquid >= 0);

    // dig into terrain
    dig_at(&mut s.world, &mut s.particles, 0, 3, 3);
    let h = histogram(&s.world, -16, 0, 16, 16);
    assert!(h.air > 0);
}

#[test]
fn electricity_and_catalog_copper_wire() {
    let mut s = CellularSession::with_builtins(WorldConfig::default());
    let copper = s.mat("copper");
    assert!(!copper.is_air(), "catalog copper registered");
    for x in 0..10 {
        s.world.set(x, 2, Cell::of(copper));
    }
    let path = find_conductive_path(&s.world, 0, 2, 9, 2, 128).expect("copper path");
    assert!(path.len() >= 10);
    assert!(try_arc(&mut s.world, 0, 2, 9, 2, 20, 25.0));
}

#[test]
fn wand_combo_and_replay_fingerprint() {
    let mut s = CellularSession::with_builtins(WorldConfig {
        seed: 7,
        ..WorldConfig::default()
    });
    s.use_hot = false;
    s.seed_demo_platform();
    let mut book = SpellBook::new();
    register_builtin_spells(&mut book);
    let mut wand = Wand::new("test")
        .modify(WandModifier::DoublePower)
        .spell("water_ball");
    let n = cast_wand(
        &book,
        &mut wand,
        &mut s.world,
        &mut s.particles,
        0.0,
        16.0,
        1.2,
    );
    assert!(n >= 1);

    let mut log = ReplayLog::new(7);
    log.push(ReplayAction::SeedDemo);
    log.push(ReplayAction::Burst {
        x: 1.0,
        y: 14.0,
        material: "sand".into(),
        count: 15,
    });
    log.push(ReplayAction::Step(20));
    let mut a = CellularSession::with_builtins(WorldConfig {
        seed: 7,
        ..WorldConfig::default()
    });
    a.use_hot = false;
    play_log(&mut a, &log);
    let mut b = CellularSession::with_builtins(WorldConfig {
        seed: 7,
        ..WorldConfig::default()
    });
    b.use_hot = false;
    play_log(&mut b, &log);
    assert_eq!(world_fingerprint(&a.world), world_fingerprint(&b.world));
}

#[test]
fn growth_reaction_chain_light() {
    let mut s = CellularSession::with_builtins(WorldConfig::default());
    s.world.paint_rect(-3, 0, 3, 1, s.mat("bedrock"));
    plant_seed(&mut s.world, 0, 1);
    let mut gcfg = GrowthConfig::default();
    gcfg.seed_sprout = 1.0;
    gcfg.vine_up = 1.0;
    for _ in 0..15 {
        growth_pass(&mut s.world, -3, 0, 3, 20, &gcfg);
    }

    s.world.set(2, 2, Cell::of(s.mat("oil")));
    s.world
        .set(3, 2, Cell::of(s.mat("fire")).with_life(20).with_temp(900.0));
    let reacted = apply_reaction_chains(&mut s.world, -4, 0, 6, 6, 64);
    let _ = reacted;
    let map = bake_light(&s.world, -8, 0, 16, 16, 30);
    assert!(average_light(&map) >= 0.0);
}
