//! # velvet-style
//!
//! **`.vcss`** — one author language that blends **CSS** and **JS-lite** for UI
//! look + motion orchestration.
//!
//! | Side | What |
//! |------|------|
//! | **CSS** | selectors, cascade, colors, `@keyframes`, `animation:` |
//! | **JS-lite** | `@script { let, fn, for, if, play, animate, wait, on }` |
//!
//! Runtime playback still uses **velvet-anim** tools (`timeline_from_plan`).
//!
//! ## Example
//!
//! ```css
//! .button { background: #0a0c16; color: #d2af64; height: 52; }
//! .button:selected { background: #501e78; glow: #dc50dc; }
//!
//! @keyframes deal {
//!   from { opacity: 0; y: -80; scale: 0.65; yaw: 0.9; }
//!   to   { opacity: 1; y: 0;   scale: 1;    yaw: 0; }
//! }
//!
//! @script {
//!   let stagger = 0.08;
//!   fn dealHand(count) {
//!     for (let i = 0; i < count; i = i + 1) {
//!       play("deal", {
//!         target: "card" + i,
//!         delay: i * stagger,
//!         duration: 0.32,
//!         ease: "cubic_out"
//!       });
//!     }
//!   }
//!   on("menu.open", fn () {
//!     animate("#logo", { opacity: [0, 1], y: [-24, 0] }, 0.45);
//!   });
//! }
//! ```

#![deny(missing_docs)]

mod animation;
mod host;
mod parse;
mod resolve;
mod script;
mod value;

pub use animation::{
    animation_spec_from_computed, plan_animation, plan_from_spec, vanim_to_vcss, AnimationSpec,
    ChannelPlan, KeyframeStop, Keyframes, TimelinePlan,
};
pub use host::StyleStoryHost;
pub use parse::{parse_stylesheet, StyleParseError, StyleRule, Stylesheet};
pub use resolve::{resolve, ComputedStyle, StyleQuery, StyleRegistry};
pub use script::{
    actions_to_timelines, eval_script_fn, parse_script, run_event, run_function, EventHandler,
    Function, JsValue, ScriptError, ScriptModule, ScriptRun, StyleAction,
};
pub use value::{parse_color, parse_value, Color, StyleValue};

/// Run a named `@script` function from a parsed stylesheet.
pub fn call_style_fn(
    sheet: &Stylesheet,
    name: &str,
    args: &[JsValue],
) -> Result<ScriptRun, ScriptError> {
    run_function(&sheet.script, Some(sheet), name, args)
}

/// Dispatch `on("event", …)` handlers defined in the stylesheet script.
pub fn emit_style_event(
    sheet: &Stylesheet,
    event: &str,
    args: &[JsValue],
) -> Result<ScriptRun, ScriptError> {
    run_event(&sheet.script, Some(sheet), event, args)
}
