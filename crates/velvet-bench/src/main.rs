//! Velvet microbenchmarks (no Criterion).
//!
//! Times:
//! - script compile (parse+compile loop)
//! - story pump (~10k ops)
//! - play world step (1k entities × frames)

use std::time::Instant;

use velvet_math::{Transform2D, Vec2};
use velvet_play::prelude::*;
use velvet_story::{load_program_from_source, StoryPlayer};

fn main() {
    println!("Velvet Bench");
    println!("============");
    println!(
        "host: {} / {}",
        std::env::consts::OS,
        std::env::consts::ARCH
    );
    println!();

    bench_script_compile();
    bench_story_pump();
    bench_play_world();
    bench_ecs_insert();
    bench_text_typewriter();

    println!();
    println!("done (wall-clock; results are noisy on busy machines)");
}

fn bench_script_compile() {
    let source = r#"
function fib(n) {
    if n < 2 {
        return n
    }
    return fib(n - 1) + fib(n - 2)
}

function main() {
    return fib(10)
}

character hero { name: "Hero" }
scene greet {
    hero "hello bench"
}
"#;
    // warmup
    for _ in 0..5 {
        let _ = velvet_script_compiler::compile_source(source, Some("bench.vel"));
    }

    let iterations = 200usize;
    let start = Instant::now();
    let mut ok = 0usize;
    for _ in 0..iterations {
        if velvet_script_compiler::compile_source(source, Some("bench.vel")).is_ok() {
            ok += 1;
        }
    }
    let elapsed = start.elapsed();
    let per = elapsed / iterations as u32;
    println!("[script compile]");
    println!("  iterations : {iterations} (ok={ok})");
    println!("  total      : {elapsed:?}");
    println!("  per compile: {per:?}");
    if elapsed.as_secs_f64() > 0.0 {
        println!(
            "  rate       : {:.1} compiles/s",
            iterations as f64 / elapsed.as_secs_f64()
        );
    }
    println!();
}

fn bench_story_pump() {
    let source = r#"
character a { name: "A" }
state { n: int = 0 }
scene main {
    a "line one"
    a "line two"
    a "line three"
    choice {
        "again" { n += 1 jump main }
        "done" { jump end }
    }
}
scene end {
    a "bye"
}
"#;
    let program = load_program_from_source(source, Some("bench.vel"), "bench")
        .expect("story program should load");

    // warmup
    {
        let mut p = StoryPlayer::start(program.clone());
        for _ in 0..100 {
            p.tick(1.0 / 60.0);
            if !p.choices().is_empty() {
                let _ = p.choose(0);
            } else if !p.is_ended() {
                p.advance();
            } else {
                break;
            }
        }
    }

    let ops_target = 10_000usize;
    let mut player = StoryPlayer::start(program);
    let start = Instant::now();
    let mut ops = 0usize;
    while ops < ops_target {
        player.tick(1.0 / 60.0);
        ops += 1;
        if !player.choices().is_empty() {
            // alternate choices to exercise both edges; prefer loop for volume
            let idx = if ops % 17 == 0 {
                1.min(player.choices().len() - 1)
            } else {
                0
            };
            let _ = player.choose(idx);
            ops += 1;
            if player.is_ended() {
                // restart to keep pumping
                // StoryPlayer may not expose restart; rebuild
                // We re-load by cloning start state via new player from same program reference
                // (program moved) — break if we cannot continue
                break;
            }
        } else if player.is_ended() {
            break;
        } else {
            player.advance();
            ops += 1;
        }
    }
    // If story ended early, keep ticking a finished player to still spend ops budget on tick path
    while ops < ops_target {
        player.tick(1.0 / 60.0);
        ops += 1;
    }
    let elapsed = start.elapsed();
    println!("[story pump]");
    println!("  ops        : {ops}");
    println!("  total      : {elapsed:?}");
    if elapsed.as_secs_f64() > 0.0 {
        println!(
            "  rate       : {:.0} ops/s",
            ops as f64 / elapsed.as_secs_f64()
        );
    }
    println!();
}

fn bench_play_world() {
    let map = TileMap::new(64, 64, 16.0).expect("map");
    let mut world = PlayWorld::new(map);
    world.spawn_player(Vec2::new(32.0, 32.0), 120.0);

    // 1k entities
    let n = 1000usize;
    for i in 0..n {
        let x = (i % 64) as f32 * 16.0 + 8.0;
        let y = (i / 64) as f32 * 16.0 + 8.0;
        world.spawn(PlayEntity {
            id: 0,
            transform: Transform2D::from_translation(Vec2::new(x, y)),
            velocity: Velocity::ZERO,
            collider: Some(Collider {
                layer: CollisionLayer::PLAYER,
                mask: CollisionMask::from_layers(CollisionLayer::WORLD),
                ..Collider::aabb(Vec2::splat(4.0))
            }),
            kinematic: Some(KinematicBody::default()),
            speed: Some(Speed(40.0)),
            facing: Facing::default(),
            player: false,
            trigger: None,
            interactable: None,
            alive: true,
        });
    }

    // warmup
    for _ in 0..10 {
        world.set_player_input(Vec2::new(1.0, 0.0));
        world.step(1.0 / 60.0);
    }

    let frames = 120usize;
    let start = Instant::now();
    for f in 0..frames {
        let t = f as f32 * 0.1;
        world.set_player_input(Vec2::new(t.cos(), t.sin()));
        world.step(1.0 / 60.0);
    }
    let elapsed = start.elapsed();
    let entity_steps = (n + 1) * frames; // rough
    println!("[play world step]");
    println!("  entities   : {}", world.entities.len());
    println!("  frames     : {frames}");
    println!("  total      : {elapsed:?}");
    println!("  per frame  : {:?}", elapsed / frames.max(1) as u32);
    if elapsed.as_secs_f64() > 0.0 {
        println!(
            "  frame rate : {:.1} Hz (sim)",
            frames as f64 / elapsed.as_secs_f64()
        );
        println!(
            "  entity-step: {:.0} /s (entities*frames / time)",
            entity_steps as f64 / elapsed.as_secs_f64()
        );
    }
    println!();
}

fn bench_ecs_insert() {
    // Component is blanket-implemented for all Send+Sync+'static types.
    use velvet_ecs::World;

    #[derive(Clone, Copy)]
    #[allow(dead_code)] // component payload for insert throughput
    struct Pos {
        x: f32,
        y: f32,
    }

    #[derive(Clone, Copy)]
    #[allow(dead_code)] // component payload for insert throughput
    struct Vel {
        x: f32,
        y: f32,
    }

    let n = 5_000usize;
    // warmup
    {
        let mut w = World::new();
        for i in 0..100 {
            let e = w.spawn();
            w.insert(
                e,
                Pos {
                    x: i as f32,
                    y: 0.0,
                },
            );
            w.insert(e, Vel { x: 1.0, y: 0.0 });
        }
    }

    let start = Instant::now();
    let mut world = World::new();
    for i in 0..n {
        let e = world.spawn();
        world.insert(
            e,
            Pos {
                x: (i % 256) as f32,
                y: (i / 256) as f32,
            },
        );
        world.insert(e, Vel { x: 1.0, y: -0.5 });
    }
    // mutate pass
    let ids: Vec<_> = world.iter_entities().map(|(e, _)| e).collect();
    for e in ids {
        if let Some(p) = world.get_mut::<Pos>(e) {
            p.x += 1.0;
        }
    }
    let elapsed = start.elapsed();
    println!("[ecs insert+mutate]");
    println!("  entities   : {n}");
    println!("  total      : {elapsed:?}");
    if elapsed.as_secs_f64() > 0.0 {
        println!(
            "  rate       : {:.0} entities/s",
            n as f64 / elapsed.as_secs_f64()
        );
    }
    println!();
}

fn bench_text_typewriter() {
    use velvet_text::Typewriter;

    let sample = "The quick brown fox jumps over the lazy dog. \
Velvet Engine typewriter microbench — progressive reveal of dialogue text \
with enough characters to exercise the reveal loop.";
    // warmup
    {
        let mut tw = Typewriter::new(sample, 120.0);
        for _ in 0..30 {
            tw.tick(1.0 / 60.0);
            if tw.is_finished() {
                break;
            }
        }
    }

    let iterations = 200usize;
    let start = Instant::now();
    let mut total_events = 0usize;
    for _ in 0..iterations {
        let mut tw = Typewriter::new(sample, 200.0);
        let mut guard = 0;
        while !tw.is_finished() && guard < 500 {
            let ev = tw.tick(1.0 / 60.0);
            total_events += ev.len();
            guard += 1;
        }
        // ensure skip path is hot too occasionally
        if !tw.is_finished() {
            tw.skip();
        }
    }
    let elapsed = start.elapsed();
    println!("[text typewriter]");
    println!("  iterations : {iterations}");
    println!("  events     : {total_events}");
    println!("  total      : {elapsed:?}");
    if elapsed.as_secs_f64() > 0.0 {
        println!(
            "  rate       : {:.1} runs/s",
            iterations as f64 / elapsed.as_secs_f64()
        );
    }
    println!();
}
