from pathlib import Path
root=Path(r'C:\dev\velvet-engine')

# Runtime capture API
p=root/'crates/velvet-vs3-app/src/app.rs'
t=p.read_text(encoding='utf-8')
old='''pub fn capture_package(
    path: impl AsRef<Path>,
    output: impl AsRef<Path>,
    frames: u64,
) -> Result<()> {
    let mut app = Vs3App::new(path)?;
    for _ in 0..frames {
'''
new='''pub fn capture_package(
    path: impl AsRef<Path>,
    output: impl AsRef<Path>,
    frames: u64,
) -> Result<()> {
    capture_package_with_action(path, output, frames, None)
}

pub fn capture_package_with_action(
    path: impl AsRef<Path>,
    output: impl AsRef<Path>,
    frames: u64,
    action: Option<&str>,
) -> Result<()> {
    let mut app = Vs3App::new(path)?;
    if let Some(action) = action {
        let frame = app
            .runtime
            .action(action)
            .and_then(|value| FrameView::parse(&value))?;
        app.accept_checked(frame)?;
    }
    for _ in 0..frames {
'''
if old not in t:
    raise SystemExit('capture api block not found')
t=t.replace(old,new,1)
p.write_text(t,encoding='utf-8',newline='\n')

# Public export
p=root/'crates/velvet-vs3-app/src/lib.rs'
t=p.read_text(encoding='utf-8')
t=t.replace('pub use app::{capture_package, run_package, validate_package, AppOptions, Vs3App};',
            'pub use app::{capture_package, capture_package_with_action, run_package, validate_package, AppOptions, Vs3App};')
p.write_text(t,encoding='utf-8',newline='\n')

# CLI command adapter
p=root/'crates/velvet-cli/src/app_cmd.rs'
t=p.read_text(encoding='utf-8')
t=t.replace('use velvet_vs3_app::{capture_package, run_package, validate_package, AppOptions};',
            'use velvet_vs3_app::{capture_package_with_action, run_package, validate_package, AppOptions};')
t=t.replace('''pub fn cmd_app_capture(path: PathBuf, output: PathBuf, frames: u64) -> Result<()> {
    capture_package(path, &output, frames)?;
''','''pub fn cmd_app_capture(
    path: PathBuf,
    output: PathBuf,
    frames: u64,
    action: Option<String>,
) -> Result<()> {
    capture_package_with_action(path, &output, frames, action.as_deref())?;
''')
p.write_text(t,encoding='utf-8',newline='\n')

# Clap enum + dispatch
p=root/'crates/velvet-cli/src/main.rs'
t=p.read_text(encoding='utf-8')
old='''        /// Update this many frames before rendering.
        #[arg(long, default_value_t = 0)]
        frames: u64,
    },
'''
new='''        /// Update this many frames before rendering.
        #[arg(long, default_value_t = 0)]
        frames: u64,
        /// Invoke one declarative app action before rendering.
        #[arg(long)]
        action: Option<String>,
    },
'''
# replace only capture block by anchoring from Capture section
capture_pos=t.index('    Capture {')
block_end=t.index('    },', capture_pos)+7
block=t[capture_pos:block_end]
if 'action: Option<String>' not in block:
    if old not in block:
        raise SystemExit('capture enum block not found')
    block=block.replace(old,new,1)
    t=t[:capture_pos]+block+t[block_end:]
old_dispatch='''                AppCommands::Capture {
                    path,
                    output,
                    frames,
                },
        } => cmd_app_capture(path, output, frames),
'''
new_dispatch='''                AppCommands::Capture {
                    path,
                    output,
                    frames,
                    action,
                },
        } => cmd_app_capture(path, output, frames, action),
'''
if old_dispatch not in t:
    raise SystemExit('capture dispatch block not found')
t=t.replace(old_dispatch,new_dispatch,1)
p.write_text(t,encoding='utf-8',newline='\n')
print('CAPTURE_ACTION_ADDED')
