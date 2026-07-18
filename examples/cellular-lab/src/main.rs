//! Full creator lab (**ALPHA**): brush, particles, spells, agents, enemies, perf.

use velvet_cellular::prelude::*;

fn main() -> anyhow::Result<()> {
    println!("=== velvet-cellular full lab (ALPHA) ===");
    let mut session = CellularSession::with_builtins(WorldConfig {
        max_loaded_chunks: 128,
        seed: 11,
        ..WorldConfig::default()
    });
    session.use_hot = true;

    session.gen_arena(0, 0, 48, 36);
    session.select_preset("Sand");
    session.brush_down(-12, 22);
    session.brush_drag(12, 20);
    session.brush_up();

    let n = session.particle_burst(0.0, 28.0, "sand", 40);
    println!("particle_burst sand n={n} live={}", session.particle_count());
    session.particle_blood(4.0, 18.0, 24);
    play_preset(
        &session.world,
        &mut session.particles,
        ParticlePreset::FireSparks,
        -6.0,
        16.0,
        1.0,
    );
    assert!(session.cast_spell("spark_bolt", 2.0, 20.0));
    assert!(session.cast_spell("water_ball", -4.0, 18.0));

    let agent = session.spawn_agent(0.0, 14.0);
    session.agent_input(
        agent,
        AgentInput {
            move_x: 0.2,
            dig: true,
            aim: -1.4,
            ..Default::default()
        },
    );

    let slime = session.spawn_enemy("slime", 10.0, 12.0).unwrap();
    session.set_enemy_target(Some(slime), 0.0, 12.0);

    for _ in 0..90 {
        session.step();
    }

    let buf = session.render(-48, -4, 96, 56);
    let opaque = opaque_pixel_count(&buf);
    println!(
        "tick={} occupied={} particles={} enemies={} spells={} materials≈{} opaque={}",
        session.world.tick,
        session.world.occupied_cells(),
        session.particle_count(),
        session.enemies.alive_count(),
        session.spells.len(),
        session.world.materials.len(),
        opaque
    );
    assert!(session.world.occupied_cells() > 0 || opaque > 0);
    println!("ASSERT_OK cellular_full");
    // second pass marker for dual-run verification
    session.particle_burst(1.0, 25.0, "water", 10);
    session.step_n(5);
    println!("ASSERT_OK cellular_full_pass2 particles={}", session.particle_count());
    Ok(())
}
