//! # velvet-story
//!
//! Visual novel / narrative adventure runtime: characters, dialogue, choices,
//! variables, history, preferences, gallery, glossary, transitions, voice, and
//! versioned saves.

#![deny(missing_docs)]

mod auto_mode;
mod character;
#[cfg(test)]
mod classic_vel_tests;
mod gallery;
mod glossary;
mod history;
mod host;
mod ir;
mod live2d;
mod load;
mod localization_hook;
mod plugin;
mod prefs;
mod product;
mod product_paint;
mod product_presenter;
mod product_raster;
mod product_ui;
mod rollback;
mod runtime;
mod save;
mod skip;
mod transitions;
mod value;
mod variables;
mod voice;
mod vs3_bridge;
mod web_story;

pub mod prelude;

pub use auto_mode::{compute_auto_delay, estimate_reveal_secs, AutoModeConfig, AutoModeController};
pub use character::Character;
pub use gallery::{Gallery, GalleryEntry, GalleryError};
pub use glossary::{Glossary, GlossaryError, GlossaryTerm};
pub use history::{History, HistoryEntry};
pub use host::{
    command_host_continue, command_host_fn, CommandOutcome, SharedCommandHost, StoryCommandError,
    StoryCommandHost,
};
pub use ir::{
    StoryArithOp, StoryChoice, StoryCmpOp, StoryCond, StoryExpr, StoryOp, StoryOperand,
    StoryProgram, StoryScene,
};
pub use load::{load_program_from_source, lower_module, LoadError, StoryDiagnostic};
pub use localization_hook::{
    catalog_to_po_template, choice_key, dialogue_key, extract_loc_keys, extract_scene_loc_keys,
    load_tl_table, program_for_language, slugify_text, speakers_in_program, write_tl_scaffold,
    LocCatalog, LocEntry, LocKind, TranslationTable,
};
pub use plugin::StoryPlugin;
pub use prefs::{SkipMode, StoryPreferences, TextSpeed};
pub use product::{
    join_dialogue_lines, open_session_from_file, say_plain_and_cps, BgmController, BgmIntent,
    ChoiceScreen, ConfirmDialog, ConfirmKind, LayeredSprite, PresentationState, SayScreen,
    VnSession,
};
// VnSession::show_dialogue_line is the product path entry that applies say_plain_and_cps.
pub use live2d::{Live2dModel, Live2dStage};
pub use product_paint::{
    background_mood_color, paint_product_frame, paint_product_frame_at, paint_product_session,
    paint_to_render_descriptors, sprite_stand_color, ProductPaintCmd, ProductPaintList,
    RenderDrawDescriptor, PRODUCT_VIRTUAL_H, PRODUCT_VIRTUAL_W,
};
pub use product_presenter::{PresenterBackend, PresenterPhase, ProductPresenter};
pub use product_raster::{
    count_painted_pixels, draw_text_line, draw_text_wrapped, fill_rect, pack_rgb,
    rasterize_product_paint,
};
pub use product_ui::{
    build_product_ui_frame, detect_script_family, dialogue_box_fields, measure_say_body,
    FontAttachment, ProductUiFrame,
};
pub use rollback::{RollbackFrame, RollbackRecorder, RollbackStack};
pub use runtime::{
    ChoiceOption, SavedCallContinuation, SavedExecFrame, StoryEvent, StoryPlayer, StorySnapshot,
    StoryWait, VisibleCharacter,
};
pub use save::{SaveError, SaveGame, SaveMeta, SaveStore};
pub use skip::{count_skippable_read_lines, SkipConfig, SkipEngine, SkipResult};
pub use transitions::{Transition, TransitionKind, TransitionQueue, WipeDirection};
pub use value::StoryValue;
pub use variables::{AssignOp, StoryVariables};
pub use voice::{VoiceClip, VoiceError, VoicePlayState, VoiceQueue};
pub use vs3_bridge::{call_vs3_logic, Vs3BridgeError, VS3_HOST_CMD};
pub use web_story::program_to_web_json;
