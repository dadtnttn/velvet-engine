//! Visual Novel demo — playable headless (auto choices) or interactive CLI.

use std::io::{self, Write};
use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use clap::Parser;
use velvet_app::prelude::*;
use velvet_input::InputPlugin;
use velvet_story::prelude::*;

#[derive(Parser, Debug)]
#[command(name = "visual-novel", about = "Velvet Engine visual novel demo")]
struct Args {
    /// Story script path (default: story/main.vel next to executable / CWD).
    #[arg(long)]
    story: Option<PathBuf>,
    /// Non-interactive: always pick choice index 0.
    #[arg(long)]
    auto: bool,
    /// Choice indices to feed in order (implies non-interactive).
    #[arg(long, value_delimiter = ',')]
    choices: Vec<usize>,
    /// Save slot after finishing (optional).
    #[arg(long)]
    save_dir: Option<PathBuf>,
    /// Load slot before play.
    #[arg(long)]
    load: Option<String>,
}

fn main() -> Result<()> {
    velvet_core::init_tracing_default("visual_novel=info,velvet=info,info");
    let args = Args::parse();

    let story_path = args.story.unwrap_or_else(|| {
        // Prefer crate-relative path when running via cargo
        let candidates = [
            PathBuf::from("examples/visual-novel/story/main.vel"),
            PathBuf::from("story/main.vel"),
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("story/main.vel"),
        ];
        candidates
            .into_iter()
            .find(|p| p.exists())
            .unwrap_or_else(|| PathBuf::from("story/main.vel"))
    });

    let source = std::fs::read_to_string(&story_path)
        .with_context(|| format!("read story {}", story_path.display()))?;
    let program = load_program_from_source(
        &source,
        Some(&story_path.to_string_lossy()),
        "Velvet Contract",
    )?;

    let mut app = App::new();
    app.add_plugin(StoryPlugin);
    app.add_plugin(InputPlugin);

    let mut player = StoryPlayer::start(program);

    if let Some(slot) = &args.load {
        let dir = args
            .save_dir
            .clone()
            .unwrap_or_else(|| PathBuf::from("saves/visual-novel"));
        let store = SaveStore::new(dir);
        let save = store.read(slot)?;
        player.load_save(save).map_err(|e| anyhow::anyhow!(e))?;
        println!("Loaded slot '{slot}' at scene {}", player.scene_name());
    }

    app.insert_resource(player);

    // Headless play loop (no window): step story until end.
    let auto = args.auto || !args.choices.is_empty();
    let mut choice_queue = args.choices;
    let mut steps = 0usize;

    loop {
        steps += 1;
        if steps > 10_000 {
            bail!("story exceeded step limit");
        }

        let player = app
            .resource_mut::<StoryPlayer>()
            .expect("StoryPlayer resource");

        for ev in player.drain_events() {
            match ev {
                StoryEvent::Background(p) => tracing::debug!(%p, "background"),
                StoryEvent::Music { path, .. } => tracing::debug!(%path, "music"),
                StoryEvent::Show(v) => tracing::debug!(id = %v.id, "show"),
                StoryEvent::Dialogue {
                    speaker_name, text, ..
                } => {
                    if speaker_name.trim().is_empty() {
                        println!("\n{text}");
                    } else {
                        println!("\n{speaker_name}: {text}");
                    }
                }
                StoryEvent::Choices(opts) => {
                    println!("\nChoices:");
                    for o in &opts {
                        let mark = if o.enabled { " " } else { "x" };
                        println!("  [{mark}] {}: {}", o.index + 1, o.text);
                    }
                }
                StoryEvent::Ended { ending } => {
                    println!("\n=== FIN ===");
                    if let Some(e) = ending {
                        println!("Ending id: {e}");
                    }
                }
                StoryEvent::Variable { name, value } => {
                    tracing::debug!(%name, %value, "var");
                }
                _ => {}
            }
        }

        match player.wait().clone() {
            StoryWait::Line => {
                if auto {
                    player.advance();
                } else {
                    print!("\n[Enter] continue · [s] skip · [q] quit > ");
                    let _ = io::stdout().flush();
                    let mut line = String::new();
                    io::stdin().read_line(&mut line)?;
                    match line.trim() {
                        "q" | "Q" => bail!("quit by user"),
                        "s" | "S" => {
                            player.preferences_mut().skip_mode = SkipMode::All;
                            let _ = player.try_skip();
                        }
                        _ => player.advance(),
                    }
                }
            }
            StoryWait::Choice => {
                let n = player.choices().len();
                let pick = if let Some(c) = choice_queue.first().copied() {
                    choice_queue.remove(0);
                    c
                } else if auto {
                    0
                } else {
                    print!("Select 1-{n} > ");
                    let _ = io::stdout().flush();
                    let mut line = String::new();
                    io::stdin().read_line(&mut line)?;
                    line.trim().parse::<usize>().unwrap_or(1).saturating_sub(1)
                };
                player
                    .choose(pick)
                    .map_err(|e| anyhow::anyhow!("choice: {e}"))?;
            }
            StoryWait::Ended => break,
            StoryWait::Ready | StoryWait::Pause { .. } => player.advance(),
            StoryWait::Host { token } => {
                let _ = player.resume_host(&token);
            }
        }
    }

    let player = app.resource::<StoryPlayer>().unwrap();
    println!("\nPlay time: {:.1}s", player.play_time_secs());
    println!(
        "aria_trust = {}",
        player.variables().get_int("aria_trust", 0)
    );
    println!("History lines: {}", player.history().len());

    if let Some(dir) = args.save_dir {
        let store = SaveStore::new(dir);
        let save = player.to_save("slot_1");
        store.write(&save)?;
        println!(
            "Saved to {}",
            store.root().join("slot_1.velsave.json").display()
        );
    }

    // One headless frame with plugins to prove App integration
    app.set_runner(HeadlessRunner {
        max_frames: Some(1),
        delta_secs: 1.0 / 60.0,
    });
    let _ = app.run();
    Ok(())
}
