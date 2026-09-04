//! # velvet-story
//!
//! Visual novel / narrative adventure runtime: characters, dialogue, choices,
//! variables, history, preferences, gallery, glossary, transitions, voice, and
//! versioned saves.

#![deny(missing_docs)]

mod accessibility;
mod achievements;
mod assets_check;
mod atl;
mod auto_mode;
mod branch_map;
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
mod music_room;
mod plugin;
mod prefs;
mod product;
mod product_paint;
mod product_presenter;
mod product_raster;
mod product_ui;
mod rich_text;
mod rollback;
mod runtime;
mod save;
mod save_photo;
mod screens;
mod skip;
mod transitions;
mod value;
mod variables;
mod video_state;
mod voice;
mod voice_cast;
mod vs3_bridge;
mod web_story;

pub mod prelude;

pub use accessibility::AccessibilityPrefs;
pub use achievements::{AchievementDef, AchievementFile, AchievementState, AchievementTrigger};
pub use assets_check::{check_project_assets, AssetIssue, AssetReport};
pub use atl::{parse_motion_words, parse_show_with_motion, MotionKind, SpriteMotion};
pub use auto_mode::{compute_auto_delay, estimate_reveal_secs, AutoModeConfig, AutoModeController};
pub use branch_map::{BranchEdge, BranchMap};
pub use character::Character;
pub use gallery::{Gallery, GalleryEntry, GalleryError};
pub use glossary::{Glossary, GlossaryError, GlossaryTerm};
pub use history::{History, HistoryEntry};
pub use host::{
    command_host_continue, command_host_fn, CommandOutcome, SharedCommandHost, StoryCommandError,
    StoryCommandHost,
};
pub use ir::{
    ShowMotion, StoryArithOp, StoryChoice, StoryCmpOp, StoryCond, StoryExpr, StoryOp, StoryOperand,
    StoryProgram, StoryScene,
};
pub use load::{load_program_from_source, lower_module, LoadError, StoryDiagnostic};
pub use localization_hook::{
    catalog_to_po_template, choice_key, dialogue_key, extract_loc_keys, extract_scene_loc_keys,
    load_tl_table, program_for_language, screen_key, slugify_text, speakers_in_program,
    write_tl_scaffold, LocCatalog, LocEntry, LocKind, TranslationTable,
};
pub use plugin::StoryPlugin;
pub use prefs::{SkipMode, StoryPreferences, TextSpeed};
pub use product::{
    join_dialogue_lines, open_session_from_file, say_plain_and_cps, ActiveMotion, BgmController,
    BgmIntent, ChoiceScreen, ConfirmDialog, ConfirmKind, LayeredSprite, PresentationState,
    SayScreen, VnSession,
};
// VnSession::show_dialogue_line is the product path entry that applies say_plain_and_cps.
pub use atl::{
    lower_transform, next_variant, sample_transform, sample_transform_on, transform_variants,
    warper_ease, AtlEvent, TransformDef, ATL_CHANNELS,
};
pub use live2d::params as live2d_params;
pub use live2d::{
    ActiveLive2dMotion, Live2dBackend, Live2dHitArea, Live2dModel, Live2dModelDef, Live2dMotion,
    Live2dParam, Live2dStage, NullBackend, MOTION_PRIORITY_FORCE, MOTION_PRIORITY_IDLE,
    MOTION_PRIORITY_NORMAL,
};
pub use music_room::{MusicFile, MusicRoom, MusicRoomError, MusicTrack};
pub use product_paint::{
    background_mood_color, paint_product_frame, paint_product_frame_at, paint_product_session,
    paint_to_render_descriptors, sprite_stand_color, ProductPaintCmd, ProductPaintList,
    RenderDrawDescriptor, PRODUCT_VIRTUAL_H, PRODUCT_VIRTUAL_W,
};
pub use product_presenter::{PresenterBackend, PresenterPhase, ProductPresenter};
pub use product_raster::{
    count_painted_pixels, draw_text_line, draw_text_rich, draw_text_wrapped, fill_rect, pack_rgb,
    parse_text_color, rasterize_product_paint,
};
pub use product_ui::{
    build_product_ui_frame, detect_script_family, dialogue_box_fields, format_backlog_entry,
    game_screen_frames, measure_say_body, search_backlog, FontAttachment, GameScreenFrame,
    ProductUiFrame,
};
pub use rich_text::{
    char_width, estimate_voice_secs, parse_rich_line, RichLine, RichSpan, TextStyleFlags,
};
pub use rollback::{RollbackFrame, RollbackRecorder, RollbackStack};
pub use runtime::{
    ActiveGameScreen, ChoiceOption, SavedCallContinuation, SavedExecFrame, StoryEvent, StoryPlayer,
    StorySnapshot, StoryWait, VisibleCharacter,
};
pub use save::{SaveError, SaveGame, SaveMeta, SaveStore};
pub use save_photo::{AutoSavePolicy, SaveThumbnail};
pub use screens::{
    check_call_arity, expr_display, lower_screen, parse_screen_action, NvlPage, ScreenActionCall,
    ScreenChildRt, ScreenDef, ScreenLayout, ScreenMode, ScreenParamRt, ScreenPreset, ScreenPropRt,
    ScreenValueRt, SCREEN_WIDGETS,
};
pub use skip::{count_skippable_read_lines, SkipConfig, SkipEngine, SkipResult};
pub use transitions::{Transition, TransitionKind, TransitionQueue, WipeDirection};
pub use value::StoryValue;
pub use variables::{AssignOp, StoryVariables};
pub use video_state::VideoState;
pub use voice::{VoiceClip, VoiceError, VoicePlayState, VoiceQueue};
pub use voice_cast::{CharacterVoice, VoiceCast, VoiceCastFile};
pub use vs3_bridge::{call_vs3_logic, Vs3BridgeError, VS3_HOST_CMD};
pub use web_story::program_to_web_json;
