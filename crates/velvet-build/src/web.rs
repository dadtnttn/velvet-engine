//! Web product player export (static HTML interactive + Node runner).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use thiserror::Error;

use crate::pack::ensure_dir;

/// Web export errors.
#[derive(Debug, Error)]
pub enum WebExportError {
    /// IO.
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    /// Message.
    #[error("{0}")]
    Message(String),
}

/// Report after web export.
#[derive(Debug, Clone)]
pub struct WebExportReport {
    /// Output directory.
    pub out_dir: PathBuf,
    /// Path to `index.html`.
    pub index_html: PathBuf,
    /// Path to `play.mjs` Node runner.
    pub play_js: PathBuf,
    /// Path to `story.json`.
    pub story_json: PathBuf,
    /// Path to browser player module.
    pub browser_js: PathBuf,
    /// Log lines.
    pub log: Vec<String>,
}

/// Export a static web player that Node can run headlessly to an ending,
/// and that browsers can advance interactively (choices + dialogue).
pub fn export_web_player(
    out_dir: impl Into<PathBuf>,
    title: &str,
    story_json: &str,
) -> Result<WebExportReport, WebExportError> {
    let out_dir = out_dir.into();
    let out = ensure_dir(&out_dir).map_err(|e| WebExportError::Message(e.to_string()))?;
    let mut log = Vec::new();

    let story_path = out.join("story.json");
    fs::write(&story_path, story_json)?;
    log.push(format!("wrote {}", story_path.display()));

    let play_js = out.join("play.mjs");
    fs::write(&play_js, PLAY_MJS)?;
    log.push(format!("wrote {}", play_js.display()));

    let browser_js = out.join("player.mjs");
    fs::write(&browser_js, BROWSER_PLAYER_MJS)?;
    log.push(format!("wrote {}", browser_js.display()));

    let index = out.join("index.html");
    fs::write(
        &index,
        format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8"/>
  <meta name="viewport" content="width=device-width, initial-scale=1"/>
  <title>{title}</title>
  <style>
    body {{ font-family: system-ui, sans-serif; background:#1a1a22; color:#f0f0f0; max-width:720px; margin:2rem auto; padding:0 1rem; }}
    #namebox {{ color:#9fef9f; font-weight:600; min-height:1.2em; }}
    #body {{ background:#222; padding:1rem; border-radius:8px; min-height:4em; white-space:pre-wrap; cursor:pointer; }}
    #choices button {{ display:block; width:100%; margin:0.4rem 0; padding:0.6rem; cursor:pointer; }}
    #status {{ color:#888; font-size:0.85rem; margin-top:1rem; }}
    .ended {{ border:1px solid #4a4; }}
  </style>
</head>
<body>
  <h1>{title}</h1>
  <div id="namebox"></div>
  <div id="body">Loading…</div>
  <div id="choices"></div>
  <div id="status"></div>
  <script type="module">
    import {{ createPlayer }} from './player.mjs';
    const res = await fetch('./story.json');
    const story = await res.json();
    const player = createPlayer(story, {{
      namebox: document.getElementById('namebox'),
      body: document.getElementById('body'),
      choices: document.getElementById('choices'),
      status: document.getElementById('status'),
    }});
    player.start();
    window.velvetPlayer = player;
  </script>
</body>
</html>
"#
        ),
    )?;
    log.push(format!("wrote {}", index.display()));

    let readme = out.join("README.md");
    fs::write(
        &readme,
        format!(
            "# {title} (Velvet Web export)\n\n\
             ## Browser (interactive)\nOpen `index.html` via a static server (or file URL).\n\
             Click the body to advance dialogue; click choices to branch.\n\n\
             ## Headless Node run (product path)\n```bash\nnode play.mjs --choice 0\n```\n\
             Exit 0 with ending marker when complete.\n"
        ),
    )?;

    Ok(WebExportReport {
        out_dir: out,
        index_html: index,
        play_js,
        story_json: story_path,
        browser_js,
        log,
    })
}

/// Run the exported Node player; returns stdout + exit code.
pub fn run_web_player_node(play_js: &Path, choice: usize) -> Result<(String, i32), WebExportError> {
    let output = Command::new("node")
        .arg(play_js)
        .arg("--choice")
        .arg(choice.to_string())
        .output()
        .map_err(|e| WebExportError::Message(format!("node: {e}")))?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let code = output.status.code().unwrap_or(-1);
    if code != 0 && stdout.is_empty() {
        return Err(WebExportError::Message(format!(
            "node exit {code}: {stderr}"
        )));
    }
    Ok((format!("{stdout}{stderr}"), code))
}

const BROWSER_PLAYER_MJS: &str = r#"
// Interactive browser product player — advances dialogue & choices to an ending.
export function createPlayer(story, dom) {
  const scenes = story.scenes || {};
  let scene = story.entry || Object.keys(scenes)[0];
  let ip = 0;
  let ended = false;
  let steps = 0;

  function setStatus(t) {
    if (dom.status) dom.status.textContent = t;
  }

  function renderDialogue(speaker, text) {
    if (dom.namebox) dom.namebox.textContent = speaker || '';
    if (dom.body) {
      dom.body.textContent = text || '';
      dom.body.classList.toggle('ended', String(text).includes('Ending:'));
    }
    if (dom.choices) dom.choices.innerHTML = '';
  }

  function renderChoices(opts) {
    if (!dom.choices) return;
    dom.choices.innerHTML = '';
    opts.forEach((o, i) => {
      const b = document.createElement('button');
      b.textContent = o.text || ('Choice ' + i);
      b.onclick = () => pickChoice(i);
      dom.choices.appendChild(b);
    });
  }

  function pickChoice(idx) {
    if (ended) return;
    const sc = scenes[scene];
    if (!sc) return;
    const op = sc.ops[ip];
    const kind = op.kind || op.type;
    if (kind !== 'choice' && kind !== 'Choice') return;
    const opts = op.options || [];
    const arm = opts[Math.min(idx, Math.max(0, opts.length - 1))];
    if (arm && arm.body) {
      for (const b of arm.body) {
        const bk = b.kind || b.type;
        if (bk === 'jump' || bk === 'Jump') {
          scene = b.target || b.label;
          ip = 0;
          tick();
          return;
        }
      }
    }
    ip++;
    tick();
  }

  function advanceLine() {
    if (ended) return;
    const sc = scenes[scene];
    if (!sc) return;
    const op = sc.ops[ip];
    if (!op) return;
    const kind = op.kind || op.type;
    if (kind === 'dialogue' || kind === 'Dialogue') {
      ip++;
      tick();
    }
  }

  function tick() {
    steps++;
    if (steps > 500) {
      setStatus('stopped after 500 steps');
      ended = true;
      return;
    }
    const sc = scenes[scene];
    if (!sc) {
      setStatus('missing scene ' + scene);
      ended = true;
      return;
    }
    if (ip >= sc.ops.length) {
      setStatus('ending reached (scene end)');
      ended = true;
      if (dom.body) dom.body.classList.add('ended');
      return;
    }
    const op = sc.ops[ip];
    const kind = op.kind || op.type;
    if (kind === 'dialogue' || kind === 'Dialogue') {
      const sp = op.speaker_name || op.speaker || '';
      const text = op.text || '';
      renderDialogue(sp, text);
      setStatus('line · click body to advance');
      if (String(text).includes('Ending:')) {
        setStatus('ending reached after ' + steps + ' step(s)');
        ended = true;
      }
      return;
    }
    if (kind === 'choice' || kind === 'Choice') {
      renderDialogue('', 'Choose:');
      renderChoices(op.options || []);
      setStatus('choice');
      return;
    }
    if (kind === 'jump' || kind === 'Jump') {
      scene = op.target || op.label;
      ip = 0;
      tick();
      return;
    }
    if (kind === 'end' || kind === 'End') {
      const t = 'Ending: ' + (op.ending || op.id || 'end');
      renderDialogue('', t);
      setStatus('ending reached after ' + steps + ' step(s)');
      ended = true;
      return;
    }
    ip++;
    tick();
  }

  return {
    start() {
      if (dom.body) dom.body.onclick = () => advanceLine();
      tick();
    },
    isEnded() { return ended; },
    steps() { return steps; },
    // Programmatic advance for tests / automation
    advance() { advanceLine(); },
    choose(i) { pickChoice(i); },
  };
}
"#;

const PLAY_MJS: &str = r#"
// Velvet web product runner (Node) — reaches a named ending.
import { readFileSync } from 'fs';
import { fileURLToPath } from 'url';
import { dirname, join } from 'path';

const __dirname = dirname(fileURLToPath(import.meta.url));
const story = JSON.parse(readFileSync(join(__dirname, 'story.json'), 'utf8'));
const args = process.argv.slice(2);
let choice = 0;
for (let i = 0; i < args.length; i++) {
  if (args[i] === '--choice' && args[i + 1]) choice = parseInt(args[i + 1], 10) || 0;
}

const scenes = story.scenes || {};
let scene = story.entry || Object.keys(scenes)[0];
let ip = 0;
let steps = 0;
const prefer = choice;

function interp(t) { return t; }

while (steps < 500) {
  steps++;
  const sc = scenes[scene];
  if (!sc) {
    console.log('error: missing scene', scene);
    process.exit(2);
  }
  if (ip >= sc.ops.length) {
    console.log('ending reached (scene end)');
    process.exit(0);
  }
  const op = sc.ops[ip];
  const kind = op.kind || op.type;
  if (kind === 'dialogue' || kind === 'Dialogue') {
    const sp = op.speaker_name || op.speaker || '';
    const text = interp(op.text || '');
    if (sp) console.log(`[say] ${sp}: ${text}`);
    else console.log(`[say] ${text}`);
    if (String(text).includes('Ending:')) {
      console.log(text);
      console.log('ending reached after', steps, 'step(s)');
      process.exit(0);
    }
    ip++;
    continue;
  }
  if (kind === 'choice' || kind === 'Choice') {
    const opts = op.options || [];
    console.log('[choice]', opts.map(o => o.text).join(' | '));
    const idx = Math.min(prefer, Math.max(0, opts.length - 1));
    const arm = opts[idx];
    if (arm && arm.body) {
      for (const b of arm.body) {
        const bk = b.kind || b.type;
        if (bk === 'jump' || bk === 'Jump') {
          scene = b.target || b.label;
          ip = 0;
          break;
        }
        if (bk === 'dialogue' || bk === 'Dialogue') {
          console.log(`[say] ${b.text || ''}`);
        }
      }
      if (arm.body.some(b => (b.kind || b.type) === 'jump' || (b.kind || b.type) === 'Jump')) {
        continue;
      }
    }
    ip++;
    continue;
  }
  if (kind === 'jump' || kind === 'Jump') {
    scene = op.target || op.label;
    ip = 0;
    continue;
  }
  if (kind === 'end' || kind === 'End') {
    console.log('Ending:', op.ending || op.id || 'end');
    console.log('ending reached after', steps, 'step(s)');
    process.exit(0);
  }
  ip++;
}
console.log('stopped after', steps, 'steps');
process.exit(1);
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn export_and_node_reaches_ending() {
        let dir = tempfile::tempdir().unwrap();
        let story = r#"{
  "title": "WebSample",
  "entry": "main",
  "scenes": {
    "main": {
      "ops": [
        {"kind": "dialogue", "speaker": "Hero", "text": "Hi"},
        {"kind": "choice", "options": [
          {"text": "Go", "body": [{"kind": "jump", "target": "end"}]}
        ]}
      ]
    },
    "end": {
      "ops": [
        {"kind": "dialogue", "text": "Ending: Web Lights"}
      ]
    }
  }
}"#;
        let report = export_web_player(dir.path().join("web"), "WebSample", story).unwrap();
        assert!(report.index_html.exists());
        assert!(report.play_js.exists());
        assert!(report.browser_js.exists());
        let html = fs::read_to_string(&report.index_html).unwrap();
        assert!(html.contains("player.mjs"));
        let browser = fs::read_to_string(&report.browser_js).unwrap();
        assert!(browser.contains("createPlayer"));
        assert!(
            browser.contains("pickChoice"),
            "browser runtime lacks choice bridge"
        );
        let (out, code) = run_web_player_node(&report.play_js, 0).unwrap();
        assert_eq!(code, 0, "stdout={out}");
        assert!(out.contains("Ending: Web Lights"), "{out}");
    }
}
