//! Skeleton host for a cellular sandbox game.
//! Copy this template into your own Cargo package and depend on `velvet-cellular`.

use velvet_cellular::prelude::*;

fn main() {
    let mut session = CellularSession::with_builtins(WorldConfig::default());
    session.seed_demo_platform();
    session.paint(0, 28, 6, "sand");
    session.paint(12, 22, 4, "water");

    // Fixed 60 Hz sim for a few seconds (headless smoke).
    for _ in 0..180 {
        session.tick(1.0 / 60.0);
    }

    let buf = session.render(-48, -8, 96, 64);
    println!(
        "sandbox tick={} opaque={}",
        session.world.tick,
        opaque_pixel_count(&buf)
    );
}
