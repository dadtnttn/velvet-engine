//! Story prelude.

pub use crate::accessibility::AccessibilityPrefs;
pub use crate::achievements::{AchievementDef, AchievementState};
pub use crate::assets_check::{check_project_assets, AssetReport};
pub use crate::atl::{
    sample_transform, sample_transform_on, AtlEvent, MotionKind, SpriteMotion, TransformDef,
};
pub use crate::auto_mode::{AutoModeConfig, AutoModeController};
pub use crate::branch_map::BranchMap;
pub use crate::character::Character;
pub use crate::gallery::{Gallery, GalleryEntry};
pub use crate::glossary::{Glossary, GlossaryTerm};
pub use crate::history::{History, HistoryEntry};
pub use crate::ir::{ShowMotion, StoryOp, StoryProgram, StoryScene};
pub use crate::live2d::{Live2dModel, Live2dModelDef, Live2dMotion, Live2dStage};
pub use crate::load::{load_program_from_source, LoadError};
pub use crate::localization_hook::{
    extract_loc_keys, load_tl_table, program_for_language, screen_key, write_tl_scaffold,
    LocCatalog, LocEntry, LocKind, TranslationTable,
};
pub use crate::music_room::{MusicFile, MusicRoom, MusicTrack};
pub use crate::plugin::StoryPlugin;
pub use crate::prefs::{SkipMode, StoryPreferences, TextSpeed};
pub use crate::product::{
    open_session_from_file, BgmController, BgmIntent, ChoiceScreen, ConfirmDialog, ConfirmKind,
    LayeredSprite, PresentationState, SayScreen, VnSession,
};
pub use crate::product_paint::{
    paint_product_frame, paint_product_session, ProductPaintList, PRODUCT_VIRTUAL_H,
    PRODUCT_VIRTUAL_W,
};
pub use crate::product_presenter::{PresenterBackend, PresenterPhase, ProductPresenter};
pub use crate::product_raster::rasterize_product_paint;
pub use crate::product_ui::{
    build_product_ui_frame, format_backlog_entry, game_screen_frames, search_backlog,
    GameScreenFrame,
};
pub use crate::rich_text::{parse_rich_line, RichLine};
pub use crate::rollback::{RollbackRecorder, RollbackStack};
pub use crate::runtime::{
    ActiveGameScreen, ChoiceOption, StoryEvent, StoryPlayer, StoryWait, VisibleCharacter,
};
pub use crate::save::{SaveError, SaveGame, SaveMeta, SaveStore};
pub use crate::save_photo::{AutoSavePolicy, SaveThumbnail};
pub use crate::screens::{ScreenDef, ScreenLayout, ScreenPreset, ScreenValueRt};
pub use crate::skip::{SkipConfig, SkipEngine, SkipResult};
pub use crate::transitions::{Transition, TransitionKind, TransitionQueue, WipeDirection};
pub use crate::value::StoryValue;
pub use crate::variables::StoryVariables;
pub use crate::video_state::VideoState;
pub use crate::voice::{VoiceClip, VoiceQueue};
pub use crate::voice_cast::{CharacterVoice, VoiceCast, VoiceCastFile};
