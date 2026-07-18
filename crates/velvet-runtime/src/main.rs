//! Velvet Runtime — packaged game host (Phase 1: headless smoke).

use anyhow::Result;
use velvet_app::prelude::*;
use velvet_core::RunMode;

fn main() -> Result<()> {
    velvet_core::init_tracing_default("velvet=info,info");
    let config = EngineConfig {
        name: "Velvet Runtime".into(),
        mode: RunMode::Production,
        ..Default::default()
    };

    let mut app = App::with_config(config);
    app.set_runner(HeadlessRunner {
        max_frames: Some(1),
        delta_secs: 1.0 / 60.0,
    });
    let code = app.run();
    if code.0 != 0 {
        std::process::exit(code.0);
    }
    Ok(())
}
