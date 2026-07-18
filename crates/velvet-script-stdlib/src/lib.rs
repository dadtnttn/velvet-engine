//! Typed stdlib descriptors for Velvet Script 2 (prelude signatures).

#![deny(missing_docs)]

use velvet_script_hir::{HirTy, PrimTy};

/// Stdlib function signature.
#[derive(Debug, Clone)]
pub struct StdFn {
    /// Name.
    pub name: &'static str,
    /// Module path.
    pub module: &'static str,
    /// Params.
    pub params: &'static [&'static str],
    /// Return type name.
    pub ret: &'static str,
    /// Docs.
    pub doc: &'static str,
}

/// All stdlib functions.
pub static STDLIB: &[StdFn] = &[
    StdFn { name: "abs", module: "math", params: &["x"], ret: "()", doc: "math.abs" },
    StdFn { name: "abs_1", module: "math", params: &["x"], ret: "()", doc: "math.abs_1" },
    StdFn { name: "abs_2", module: "math", params: &["x"], ret: "()", doc: "math.abs_2" },
    StdFn { name: "abs_3", module: "math", params: &["x"], ret: "()", doc: "math.abs_3" },
    StdFn { name: "abs_4", module: "math", params: &["x"], ret: "()", doc: "math.abs_4" },
    StdFn { name: "min", module: "math", params: &["x"], ret: "()", doc: "math.min" },
    StdFn { name: "min_1", module: "math", params: &["x"], ret: "()", doc: "math.min_1" },
    StdFn { name: "min_2", module: "math", params: &["x"], ret: "()", doc: "math.min_2" },
    StdFn { name: "min_3", module: "math", params: &["x"], ret: "()", doc: "math.min_3" },
    StdFn { name: "min_4", module: "math", params: &["x"], ret: "()", doc: "math.min_4" },
    StdFn { name: "max", module: "math", params: &["x"], ret: "()", doc: "math.max" },
    StdFn { name: "max_1", module: "math", params: &["x"], ret: "()", doc: "math.max_1" },
    StdFn { name: "max_2", module: "math", params: &["x"], ret: "()", doc: "math.max_2" },
    StdFn { name: "max_3", module: "math", params: &["x"], ret: "()", doc: "math.max_3" },
    StdFn { name: "max_4", module: "math", params: &["x"], ret: "()", doc: "math.max_4" },
    StdFn { name: "clamp", module: "math", params: &["x"], ret: "()", doc: "math.clamp" },
    StdFn { name: "clamp_1", module: "math", params: &["x"], ret: "()", doc: "math.clamp_1" },
    StdFn { name: "clamp_2", module: "math", params: &["x"], ret: "()", doc: "math.clamp_2" },
    StdFn { name: "clamp_3", module: "math", params: &["x"], ret: "()", doc: "math.clamp_3" },
    StdFn { name: "clamp_4", module: "math", params: &["x"], ret: "()", doc: "math.clamp_4" },
    StdFn { name: "sin", module: "math", params: &["x"], ret: "()", doc: "math.sin" },
    StdFn { name: "sin_1", module: "math", params: &["x"], ret: "()", doc: "math.sin_1" },
    StdFn { name: "sin_2", module: "math", params: &["x"], ret: "()", doc: "math.sin_2" },
    StdFn { name: "sin_3", module: "math", params: &["x"], ret: "()", doc: "math.sin_3" },
    StdFn { name: "sin_4", module: "math", params: &["x"], ret: "()", doc: "math.sin_4" },
    StdFn { name: "cos", module: "math", params: &["x"], ret: "()", doc: "math.cos" },
    StdFn { name: "cos_1", module: "math", params: &["x"], ret: "()", doc: "math.cos_1" },
    StdFn { name: "cos_2", module: "math", params: &["x"], ret: "()", doc: "math.cos_2" },
    StdFn { name: "cos_3", module: "math", params: &["x"], ret: "()", doc: "math.cos_3" },
    StdFn { name: "cos_4", module: "math", params: &["x"], ret: "()", doc: "math.cos_4" },
    StdFn { name: "sqrt", module: "math", params: &["x"], ret: "()", doc: "math.sqrt" },
    StdFn { name: "sqrt_1", module: "math", params: &["x"], ret: "()", doc: "math.sqrt_1" },
    StdFn { name: "sqrt_2", module: "math", params: &["x"], ret: "()", doc: "math.sqrt_2" },
    StdFn { name: "sqrt_3", module: "math", params: &["x"], ret: "()", doc: "math.sqrt_3" },
    StdFn { name: "sqrt_4", module: "math", params: &["x"], ret: "()", doc: "math.sqrt_4" },
    StdFn { name: "floor", module: "math", params: &["x"], ret: "()", doc: "math.floor" },
    StdFn { name: "floor_1", module: "math", params: &["x"], ret: "()", doc: "math.floor_1" },
    StdFn { name: "floor_2", module: "math", params: &["x"], ret: "()", doc: "math.floor_2" },
    StdFn { name: "floor_3", module: "math", params: &["x"], ret: "()", doc: "math.floor_3" },
    StdFn { name: "floor_4", module: "math", params: &["x"], ret: "()", doc: "math.floor_4" },
    StdFn { name: "ceil", module: "math", params: &["x"], ret: "()", doc: "math.ceil" },
    StdFn { name: "ceil_1", module: "math", params: &["x"], ret: "()", doc: "math.ceil_1" },
    StdFn { name: "ceil_2", module: "math", params: &["x"], ret: "()", doc: "math.ceil_2" },
    StdFn { name: "ceil_3", module: "math", params: &["x"], ret: "()", doc: "math.ceil_3" },
    StdFn { name: "ceil_4", module: "math", params: &["x"], ret: "()", doc: "math.ceil_4" },
    StdFn { name: "pow", module: "math", params: &["x"], ret: "()", doc: "math.pow" },
    StdFn { name: "pow_1", module: "math", params: &["x"], ret: "()", doc: "math.pow_1" },
    StdFn { name: "pow_2", module: "math", params: &["x"], ret: "()", doc: "math.pow_2" },
    StdFn { name: "pow_3", module: "math", params: &["x"], ret: "()", doc: "math.pow_3" },
    StdFn { name: "pow_4", module: "math", params: &["x"], ret: "()", doc: "math.pow_4" },
    StdFn { name: "len", module: "string", params: &["x"], ret: "()", doc: "string.len" },
    StdFn { name: "len_1", module: "string", params: &["x"], ret: "()", doc: "string.len_1" },
    StdFn { name: "len_2", module: "string", params: &["x"], ret: "()", doc: "string.len_2" },
    StdFn { name: "len_3", module: "string", params: &["x"], ret: "()", doc: "string.len_3" },
    StdFn { name: "len_4", module: "string", params: &["x"], ret: "()", doc: "string.len_4" },
    StdFn { name: "contains", module: "string", params: &["x"], ret: "()", doc: "string.contains" },
    StdFn { name: "contains_1", module: "string", params: &["x"], ret: "()", doc: "string.contains_1" },
    StdFn { name: "contains_2", module: "string", params: &["x"], ret: "()", doc: "string.contains_2" },
    StdFn { name: "contains_3", module: "string", params: &["x"], ret: "()", doc: "string.contains_3" },
    StdFn { name: "contains_4", module: "string", params: &["x"], ret: "()", doc: "string.contains_4" },
    StdFn { name: "starts_with", module: "string", params: &["x"], ret: "()", doc: "string.starts_with" },
    StdFn { name: "starts_with_1", module: "string", params: &["x"], ret: "()", doc: "string.starts_with_1" },
    StdFn { name: "starts_with_2", module: "string", params: &["x"], ret: "()", doc: "string.starts_with_2" },
    StdFn { name: "starts_with_3", module: "string", params: &["x"], ret: "()", doc: "string.starts_with_3" },
    StdFn { name: "starts_with_4", module: "string", params: &["x"], ret: "()", doc: "string.starts_with_4" },
    StdFn { name: "ends_with", module: "string", params: &["x"], ret: "()", doc: "string.ends_with" },
    StdFn { name: "ends_with_1", module: "string", params: &["x"], ret: "()", doc: "string.ends_with_1" },
    StdFn { name: "ends_with_2", module: "string", params: &["x"], ret: "()", doc: "string.ends_with_2" },
    StdFn { name: "ends_with_3", module: "string", params: &["x"], ret: "()", doc: "string.ends_with_3" },
    StdFn { name: "ends_with_4", module: "string", params: &["x"], ret: "()", doc: "string.ends_with_4" },
    StdFn { name: "trim", module: "string", params: &["x"], ret: "()", doc: "string.trim" },
    StdFn { name: "trim_1", module: "string", params: &["x"], ret: "()", doc: "string.trim_1" },
    StdFn { name: "trim_2", module: "string", params: &["x"], ret: "()", doc: "string.trim_2" },
    StdFn { name: "trim_3", module: "string", params: &["x"], ret: "()", doc: "string.trim_3" },
    StdFn { name: "trim_4", module: "string", params: &["x"], ret: "()", doc: "string.trim_4" },
    StdFn { name: "to_upper", module: "string", params: &["x"], ret: "()", doc: "string.to_upper" },
    StdFn { name: "to_upper_1", module: "string", params: &["x"], ret: "()", doc: "string.to_upper_1" },
    StdFn { name: "to_upper_2", module: "string", params: &["x"], ret: "()", doc: "string.to_upper_2" },
    StdFn { name: "to_upper_3", module: "string", params: &["x"], ret: "()", doc: "string.to_upper_3" },
    StdFn { name: "to_upper_4", module: "string", params: &["x"], ret: "()", doc: "string.to_upper_4" },
    StdFn { name: "to_lower", module: "string", params: &["x"], ret: "()", doc: "string.to_lower" },
    StdFn { name: "to_lower_1", module: "string", params: &["x"], ret: "()", doc: "string.to_lower_1" },
    StdFn { name: "to_lower_2", module: "string", params: &["x"], ret: "()", doc: "string.to_lower_2" },
    StdFn { name: "to_lower_3", module: "string", params: &["x"], ret: "()", doc: "string.to_lower_3" },
    StdFn { name: "to_lower_4", module: "string", params: &["x"], ret: "()", doc: "string.to_lower_4" },
    StdFn { name: "replace", module: "string", params: &["x"], ret: "()", doc: "string.replace" },
    StdFn { name: "replace_1", module: "string", params: &["x"], ret: "()", doc: "string.replace_1" },
    StdFn { name: "replace_2", module: "string", params: &["x"], ret: "()", doc: "string.replace_2" },
    StdFn { name: "replace_3", module: "string", params: &["x"], ret: "()", doc: "string.replace_3" },
    StdFn { name: "replace_4", module: "string", params: &["x"], ret: "()", doc: "string.replace_4" },
    StdFn { name: "push_layer", module: "layer", params: &["x"], ret: "()", doc: "layer.push_layer" },
    StdFn { name: "push_layer_1", module: "layer", params: &["x"], ret: "()", doc: "layer.push_layer_1" },
    StdFn { name: "push_layer_2", module: "layer", params: &["x"], ret: "()", doc: "layer.push_layer_2" },
    StdFn { name: "push_layer_3", module: "layer", params: &["x"], ret: "()", doc: "layer.push_layer_3" },
    StdFn { name: "push_layer_4", module: "layer", params: &["x"], ret: "()", doc: "layer.push_layer_4" },
    StdFn { name: "pop_layer", module: "layer", params: &["x"], ret: "()", doc: "layer.pop_layer" },
    StdFn { name: "pop_layer_1", module: "layer", params: &["x"], ret: "()", doc: "layer.pop_layer_1" },
    StdFn { name: "pop_layer_2", module: "layer", params: &["x"], ret: "()", doc: "layer.pop_layer_2" },
    StdFn { name: "pop_layer_3", module: "layer", params: &["x"], ret: "()", doc: "layer.pop_layer_3" },
    StdFn { name: "pop_layer_4", module: "layer", params: &["x"], ret: "()", doc: "layer.pop_layer_4" },
    StdFn { name: "show_layer", module: "layer", params: &["x"], ret: "()", doc: "layer.show_layer" },
    StdFn { name: "show_layer_1", module: "layer", params: &["x"], ret: "()", doc: "layer.show_layer_1" },
    StdFn { name: "show_layer_2", module: "layer", params: &["x"], ret: "()", doc: "layer.show_layer_2" },
    StdFn { name: "show_layer_3", module: "layer", params: &["x"], ret: "()", doc: "layer.show_layer_3" },
    StdFn { name: "show_layer_4", module: "layer", params: &["x"], ret: "()", doc: "layer.show_layer_4" },
    StdFn { name: "hide_layer", module: "layer", params: &["x"], ret: "()", doc: "layer.hide_layer" },
    StdFn { name: "hide_layer_1", module: "layer", params: &["x"], ret: "()", doc: "layer.hide_layer_1" },
    StdFn { name: "hide_layer_2", module: "layer", params: &["x"], ret: "()", doc: "layer.hide_layer_2" },
    StdFn { name: "hide_layer_3", module: "layer", params: &["x"], ret: "()", doc: "layer.hide_layer_3" },
    StdFn { name: "hide_layer_4", module: "layer", params: &["x"], ret: "()", doc: "layer.hide_layer_4" },
    StdFn { name: "set_layer_z", module: "layer", params: &["x"], ret: "()", doc: "layer.set_layer_z" },
    StdFn { name: "set_layer_z_1", module: "layer", params: &["x"], ret: "()", doc: "layer.set_layer_z_1" },
    StdFn { name: "set_layer_z_2", module: "layer", params: &["x"], ret: "()", doc: "layer.set_layer_z_2" },
    StdFn { name: "set_layer_z_3", module: "layer", params: &["x"], ret: "()", doc: "layer.set_layer_z_3" },
    StdFn { name: "set_layer_z_4", module: "layer", params: &["x"], ret: "()", doc: "layer.set_layer_z_4" },
    StdFn { name: "layer_id", module: "layer", params: &["x"], ret: "()", doc: "layer.layer_id" },
    StdFn { name: "layer_id_1", module: "layer", params: &["x"], ret: "()", doc: "layer.layer_id_1" },
    StdFn { name: "layer_id_2", module: "layer", params: &["x"], ret: "()", doc: "layer.layer_id_2" },
    StdFn { name: "layer_id_3", module: "layer", params: &["x"], ret: "()", doc: "layer.layer_id_3" },
    StdFn { name: "layer_id_4", module: "layer", params: &["x"], ret: "()", doc: "layer.layer_id_4" },
    StdFn { name: "play_bgm", module: "audio", params: &["x"], ret: "()", doc: "audio.play_bgm" },
    StdFn { name: "play_bgm_1", module: "audio", params: &["x"], ret: "()", doc: "audio.play_bgm_1" },
    StdFn { name: "play_bgm_2", module: "audio", params: &["x"], ret: "()", doc: "audio.play_bgm_2" },
    StdFn { name: "play_bgm_3", module: "audio", params: &["x"], ret: "()", doc: "audio.play_bgm_3" },
    StdFn { name: "play_bgm_4", module: "audio", params: &["x"], ret: "()", doc: "audio.play_bgm_4" },
    StdFn { name: "stop_bgm", module: "audio", params: &["x"], ret: "()", doc: "audio.stop_bgm" },
    StdFn { name: "stop_bgm_1", module: "audio", params: &["x"], ret: "()", doc: "audio.stop_bgm_1" },
    StdFn { name: "stop_bgm_2", module: "audio", params: &["x"], ret: "()", doc: "audio.stop_bgm_2" },
    StdFn { name: "stop_bgm_3", module: "audio", params: &["x"], ret: "()", doc: "audio.stop_bgm_3" },
    StdFn { name: "stop_bgm_4", module: "audio", params: &["x"], ret: "()", doc: "audio.stop_bgm_4" },
    StdFn { name: "play_sfx", module: "audio", params: &["x"], ret: "()", doc: "audio.play_sfx" },
    StdFn { name: "play_sfx_1", module: "audio", params: &["x"], ret: "()", doc: "audio.play_sfx_1" },
    StdFn { name: "play_sfx_2", module: "audio", params: &["x"], ret: "()", doc: "audio.play_sfx_2" },
    StdFn { name: "play_sfx_3", module: "audio", params: &["x"], ret: "()", doc: "audio.play_sfx_3" },
    StdFn { name: "play_sfx_4", module: "audio", params: &["x"], ret: "()", doc: "audio.play_sfx_4" },
    StdFn { name: "play_voice", module: "audio", params: &["x"], ret: "()", doc: "audio.play_voice" },
    StdFn { name: "play_voice_1", module: "audio", params: &["x"], ret: "()", doc: "audio.play_voice_1" },
    StdFn { name: "play_voice_2", module: "audio", params: &["x"], ret: "()", doc: "audio.play_voice_2" },
    StdFn { name: "play_voice_3", module: "audio", params: &["x"], ret: "()", doc: "audio.play_voice_3" },
    StdFn { name: "play_voice_4", module: "audio", params: &["x"], ret: "()", doc: "audio.play_voice_4" },
    StdFn { name: "set_volume", module: "audio", params: &["x"], ret: "()", doc: "audio.set_volume" },
    StdFn { name: "set_volume_1", module: "audio", params: &["x"], ret: "()", doc: "audio.set_volume_1" },
    StdFn { name: "set_volume_2", module: "audio", params: &["x"], ret: "()", doc: "audio.set_volume_2" },
    StdFn { name: "set_volume_3", module: "audio", params: &["x"], ret: "()", doc: "audio.set_volume_3" },
    StdFn { name: "set_volume_4", module: "audio", params: &["x"], ret: "()", doc: "audio.set_volume_4" },
    StdFn { name: "say", module: "story", params: &["x"], ret: "()", doc: "story.say" },
    StdFn { name: "say_1", module: "story", params: &["x"], ret: "()", doc: "story.say_1" },
    StdFn { name: "say_2", module: "story", params: &["x"], ret: "()", doc: "story.say_2" },
    StdFn { name: "say_3", module: "story", params: &["x"], ret: "()", doc: "story.say_3" },
    StdFn { name: "say_4", module: "story", params: &["x"], ret: "()", doc: "story.say_4" },
    StdFn { name: "jump", module: "story", params: &["x"], ret: "()", doc: "story.jump" },
    StdFn { name: "jump_1", module: "story", params: &["x"], ret: "()", doc: "story.jump_1" },
    StdFn { name: "jump_2", module: "story", params: &["x"], ret: "()", doc: "story.jump_2" },
    StdFn { name: "jump_3", module: "story", params: &["x"], ret: "()", doc: "story.jump_3" },
    StdFn { name: "jump_4", module: "story", params: &["x"], ret: "()", doc: "story.jump_4" },
    StdFn { name: "call_scene", module: "story", params: &["x"], ret: "()", doc: "story.call_scene" },
    StdFn { name: "call_scene_1", module: "story", params: &["x"], ret: "()", doc: "story.call_scene_1" },
    StdFn { name: "call_scene_2", module: "story", params: &["x"], ret: "()", doc: "story.call_scene_2" },
    StdFn { name: "call_scene_3", module: "story", params: &["x"], ret: "()", doc: "story.call_scene_3" },
    StdFn { name: "call_scene_4", module: "story", params: &["x"], ret: "()", doc: "story.call_scene_4" },
    StdFn { name: "menu_select", module: "story", params: &["x"], ret: "()", doc: "story.menu_select" },
    StdFn { name: "menu_select_1", module: "story", params: &["x"], ret: "()", doc: "story.menu_select_1" },
    StdFn { name: "menu_select_2", module: "story", params: &["x"], ret: "()", doc: "story.menu_select_2" },
    StdFn { name: "menu_select_3", module: "story", params: &["x"], ret: "()", doc: "story.menu_select_3" },
    StdFn { name: "menu_select_4", module: "story", params: &["x"], ret: "()", doc: "story.menu_select_4" },
    StdFn { name: "show_char", module: "story", params: &["x"], ret: "()", doc: "story.show_char" },
    StdFn { name: "show_char_1", module: "story", params: &["x"], ret: "()", doc: "story.show_char_1" },
    StdFn { name: "show_char_2", module: "story", params: &["x"], ret: "()", doc: "story.show_char_2" },
    StdFn { name: "show_char_3", module: "story", params: &["x"], ret: "()", doc: "story.show_char_3" },
    StdFn { name: "show_char_4", module: "story", params: &["x"], ret: "()", doc: "story.show_char_4" },
    StdFn { name: "hide_char", module: "story", params: &["x"], ret: "()", doc: "story.hide_char" },
    StdFn { name: "hide_char_1", module: "story", params: &["x"], ret: "()", doc: "story.hide_char_1" },
    StdFn { name: "hide_char_2", module: "story", params: &["x"], ret: "()", doc: "story.hide_char_2" },
    StdFn { name: "hide_char_3", module: "story", params: &["x"], ret: "()", doc: "story.hide_char_3" },
    StdFn { name: "hide_char_4", module: "story", params: &["x"], ret: "()", doc: "story.hide_char_4" },
    StdFn { name: "t", module: "i18n", params: &["x"], ret: "()", doc: "i18n.t" },
    StdFn { name: "t_1", module: "i18n", params: &["x"], ret: "()", doc: "i18n.t_1" },
    StdFn { name: "t_2", module: "i18n", params: &["x"], ret: "()", doc: "i18n.t_2" },
    StdFn { name: "t_3", module: "i18n", params: &["x"], ret: "()", doc: "i18n.t_3" },
    StdFn { name: "t_4", module: "i18n", params: &["x"], ret: "()", doc: "i18n.t_4" },
    StdFn { name: "has_key", module: "i18n", params: &["x"], ret: "()", doc: "i18n.has_key" },
    StdFn { name: "has_key_1", module: "i18n", params: &["x"], ret: "()", doc: "i18n.has_key_1" },
    StdFn { name: "has_key_2", module: "i18n", params: &["x"], ret: "()", doc: "i18n.has_key_2" },
    StdFn { name: "has_key_3", module: "i18n", params: &["x"], ret: "()", doc: "i18n.has_key_3" },
    StdFn { name: "has_key_4", module: "i18n", params: &["x"], ret: "()", doc: "i18n.has_key_4" },
    StdFn { name: "locale", module: "i18n", params: &["x"], ret: "()", doc: "i18n.locale" },
    StdFn { name: "locale_1", module: "i18n", params: &["x"], ret: "()", doc: "i18n.locale_1" },
    StdFn { name: "locale_2", module: "i18n", params: &["x"], ret: "()", doc: "i18n.locale_2" },
    StdFn { name: "locale_3", module: "i18n", params: &["x"], ret: "()", doc: "i18n.locale_3" },
    StdFn { name: "locale_4", module: "i18n", params: &["x"], ret: "()", doc: "i18n.locale_4" },
    StdFn { name: "set_locale", module: "i18n", params: &["x"], ret: "()", doc: "i18n.set_locale" },
    StdFn { name: "set_locale_1", module: "i18n", params: &["x"], ret: "()", doc: "i18n.set_locale_1" },
    StdFn { name: "set_locale_2", module: "i18n", params: &["x"], ret: "()", doc: "i18n.set_locale_2" },
    StdFn { name: "set_locale_3", module: "i18n", params: &["x"], ret: "()", doc: "i18n.set_locale_3" },
    StdFn { name: "set_locale_4", module: "i18n", params: &["x"], ret: "()", doc: "i18n.set_locale_4" },
    StdFn { name: "pressed", module: "input", params: &["x"], ret: "()", doc: "input.pressed" },
    StdFn { name: "pressed_1", module: "input", params: &["x"], ret: "()", doc: "input.pressed_1" },
    StdFn { name: "pressed_2", module: "input", params: &["x"], ret: "()", doc: "input.pressed_2" },
    StdFn { name: "pressed_3", module: "input", params: &["x"], ret: "()", doc: "input.pressed_3" },
    StdFn { name: "pressed_4", module: "input", params: &["x"], ret: "()", doc: "input.pressed_4" },
    StdFn { name: "just_pressed", module: "input", params: &["x"], ret: "()", doc: "input.just_pressed" },
    StdFn { name: "just_pressed_1", module: "input", params: &["x"], ret: "()", doc: "input.just_pressed_1" },
    StdFn { name: "just_pressed_2", module: "input", params: &["x"], ret: "()", doc: "input.just_pressed_2" },
    StdFn { name: "just_pressed_3", module: "input", params: &["x"], ret: "()", doc: "input.just_pressed_3" },
    StdFn { name: "just_pressed_4", module: "input", params: &["x"], ret: "()", doc: "input.just_pressed_4" },
    StdFn { name: "axis", module: "input", params: &["x"], ret: "()", doc: "input.axis" },
    StdFn { name: "axis_1", module: "input", params: &["x"], ret: "()", doc: "input.axis_1" },
    StdFn { name: "axis_2", module: "input", params: &["x"], ret: "()", doc: "input.axis_2" },
    StdFn { name: "axis_3", module: "input", params: &["x"], ret: "()", doc: "input.axis_3" },
    StdFn { name: "axis_4", module: "input", params: &["x"], ret: "()", doc: "input.axis_4" },
    StdFn { name: "print", module: "util", params: &["x"], ret: "()", doc: "util.print" },
    StdFn { name: "print_1", module: "util", params: &["x"], ret: "()", doc: "util.print_1" },
    StdFn { name: "print_2", module: "util", params: &["x"], ret: "()", doc: "util.print_2" },
    StdFn { name: "print_3", module: "util", params: &["x"], ret: "()", doc: "util.print_3" },
    StdFn { name: "print_4", module: "util", params: &["x"], ret: "()", doc: "util.print_4" },
    StdFn { name: "assert", module: "util", params: &["x"], ret: "()", doc: "util.assert" },
    StdFn { name: "assert_1", module: "util", params: &["x"], ret: "()", doc: "util.assert_1" },
    StdFn { name: "assert_2", module: "util", params: &["x"], ret: "()", doc: "util.assert_2" },
    StdFn { name: "assert_3", module: "util", params: &["x"], ret: "()", doc: "util.assert_3" },
    StdFn { name: "assert_4", module: "util", params: &["x"], ret: "()", doc: "util.assert_4" },
    StdFn { name: "panic", module: "util", params: &["x"], ret: "()", doc: "util.panic" },
    StdFn { name: "panic_1", module: "util", params: &["x"], ret: "()", doc: "util.panic_1" },
    StdFn { name: "panic_2", module: "util", params: &["x"], ret: "()", doc: "util.panic_2" },
    StdFn { name: "panic_3", module: "util", params: &["x"], ret: "()", doc: "util.panic_3" },
    StdFn { name: "panic_4", module: "util", params: &["x"], ret: "()", doc: "util.panic_4" },
    StdFn { name: "ok", module: "util", params: &["x"], ret: "()", doc: "util.ok" },
    StdFn { name: "ok_1", module: "util", params: &["x"], ret: "()", doc: "util.ok_1" },
    StdFn { name: "ok_2", module: "util", params: &["x"], ret: "()", doc: "util.ok_2" },
    StdFn { name: "ok_3", module: "util", params: &["x"], ret: "()", doc: "util.ok_3" },
    StdFn { name: "ok_4", module: "util", params: &["x"], ret: "()", doc: "util.ok_4" },
    StdFn { name: "err", module: "util", params: &["x"], ret: "()", doc: "util.err" },
    StdFn { name: "err_1", module: "util", params: &["x"], ret: "()", doc: "util.err_1" },
    StdFn { name: "err_2", module: "util", params: &["x"], ret: "()", doc: "util.err_2" },
    StdFn { name: "err_3", module: "util", params: &["x"], ret: "()", doc: "util.err_3" },
    StdFn { name: "err_4", module: "util", params: &["x"], ret: "()", doc: "util.err_4" },
];

/// Lookup stdlib function.
pub fn find_std(name: &str) -> Option<&'static StdFn> {
    STDLIB.iter().find(|f| f.name == name)
}

/// Map ret name to HirTy roughly.
pub fn ret_ty(name: &str) -> HirTy {
    match name {
        "i32" => HirTy::Prim(PrimTy::I32),
        "bool" => HirTy::Prim(PrimTy::Bool),
        "str" => HirTy::Prim(PrimTy::Str),
        "LayerId" => HirTy::LayerId,
        "MsgId" => HirTy::MsgId,
        _ => HirTy::Prim(PrimTy::Unit),
    }
}

/// Crate version.
pub fn crate_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stdlib_nonempty() {
        assert!(STDLIB.len() > 50);
        assert!(find_std("push_layer").is_some() || STDLIB.iter().any(|f| f.module == "layer"));
    }
    #[test]
    fn std_math_abs() {
        assert!(STDLIB.iter().any(|f| f.name == "abs" && f.module == "math"));
    }
    #[test]
    fn std_math_abs_1() {
        assert!(STDLIB.iter().any(|f| f.name == "abs_1" && f.module == "math"));
    }
    #[test]
    fn std_math_abs_2() {
        assert!(STDLIB.iter().any(|f| f.name == "abs_2" && f.module == "math"));
    }
    #[test]
    fn std_math_abs_3() {
        assert!(STDLIB.iter().any(|f| f.name == "abs_3" && f.module == "math"));
    }
    #[test]
    fn std_math_abs_4() {
        assert!(STDLIB.iter().any(|f| f.name == "abs_4" && f.module == "math"));
    }
    #[test]
    fn std_math_min() {
        assert!(STDLIB.iter().any(|f| f.name == "min" && f.module == "math"));
    }
    #[test]
    fn std_math_min_1() {
        assert!(STDLIB.iter().any(|f| f.name == "min_1" && f.module == "math"));
    }
    #[test]
    fn std_math_min_2() {
        assert!(STDLIB.iter().any(|f| f.name == "min_2" && f.module == "math"));
    }
    #[test]
    fn std_math_min_3() {
        assert!(STDLIB.iter().any(|f| f.name == "min_3" && f.module == "math"));
    }
    #[test]
    fn std_math_min_4() {
        assert!(STDLIB.iter().any(|f| f.name == "min_4" && f.module == "math"));
    }
    #[test]
    fn std_math_max() {
        assert!(STDLIB.iter().any(|f| f.name == "max" && f.module == "math"));
    }
    #[test]
    fn std_math_max_1() {
        assert!(STDLIB.iter().any(|f| f.name == "max_1" && f.module == "math"));
    }
    #[test]
    fn std_math_max_2() {
        assert!(STDLIB.iter().any(|f| f.name == "max_2" && f.module == "math"));
    }
    #[test]
    fn std_math_max_3() {
        assert!(STDLIB.iter().any(|f| f.name == "max_3" && f.module == "math"));
    }
    #[test]
    fn std_math_max_4() {
        assert!(STDLIB.iter().any(|f| f.name == "max_4" && f.module == "math"));
    }
    #[test]
    fn std_math_clamp() {
        assert!(STDLIB.iter().any(|f| f.name == "clamp" && f.module == "math"));
    }
    #[test]
    fn std_math_clamp_1() {
        assert!(STDLIB.iter().any(|f| f.name == "clamp_1" && f.module == "math"));
    }
    #[test]
    fn std_math_clamp_2() {
        assert!(STDLIB.iter().any(|f| f.name == "clamp_2" && f.module == "math"));
    }
    #[test]
    fn std_math_clamp_3() {
        assert!(STDLIB.iter().any(|f| f.name == "clamp_3" && f.module == "math"));
    }
    #[test]
    fn std_math_clamp_4() {
        assert!(STDLIB.iter().any(|f| f.name == "clamp_4" && f.module == "math"));
    }
    #[test]
    fn std_math_sin() {
        assert!(STDLIB.iter().any(|f| f.name == "sin" && f.module == "math"));
    }
    #[test]
    fn std_math_sin_1() {
        assert!(STDLIB.iter().any(|f| f.name == "sin_1" && f.module == "math"));
    }
    #[test]
    fn std_math_sin_2() {
        assert!(STDLIB.iter().any(|f| f.name == "sin_2" && f.module == "math"));
    }
    #[test]
    fn std_math_sin_3() {
        assert!(STDLIB.iter().any(|f| f.name == "sin_3" && f.module == "math"));
    }
    #[test]
    fn std_math_sin_4() {
        assert!(STDLIB.iter().any(|f| f.name == "sin_4" && f.module == "math"));
    }
    #[test]
    fn std_math_cos() {
        assert!(STDLIB.iter().any(|f| f.name == "cos" && f.module == "math"));
    }
    #[test]
    fn std_math_cos_1() {
        assert!(STDLIB.iter().any(|f| f.name == "cos_1" && f.module == "math"));
    }
    #[test]
    fn std_math_cos_2() {
        assert!(STDLIB.iter().any(|f| f.name == "cos_2" && f.module == "math"));
    }
    #[test]
    fn std_math_cos_3() {
        assert!(STDLIB.iter().any(|f| f.name == "cos_3" && f.module == "math"));
    }
    #[test]
    fn std_math_cos_4() {
        assert!(STDLIB.iter().any(|f| f.name == "cos_4" && f.module == "math"));
    }
    #[test]
    fn std_math_sqrt() {
        assert!(STDLIB.iter().any(|f| f.name == "sqrt" && f.module == "math"));
    }
    #[test]
    fn std_math_sqrt_1() {
        assert!(STDLIB.iter().any(|f| f.name == "sqrt_1" && f.module == "math"));
    }
    #[test]
    fn std_math_sqrt_2() {
        assert!(STDLIB.iter().any(|f| f.name == "sqrt_2" && f.module == "math"));
    }
    #[test]
    fn std_math_sqrt_3() {
        assert!(STDLIB.iter().any(|f| f.name == "sqrt_3" && f.module == "math"));
    }
    #[test]
    fn std_math_sqrt_4() {
        assert!(STDLIB.iter().any(|f| f.name == "sqrt_4" && f.module == "math"));
    }
    #[test]
    fn std_math_floor() {
        assert!(STDLIB.iter().any(|f| f.name == "floor" && f.module == "math"));
    }
    #[test]
    fn std_math_floor_1() {
        assert!(STDLIB.iter().any(|f| f.name == "floor_1" && f.module == "math"));
    }
    #[test]
    fn std_math_floor_2() {
        assert!(STDLIB.iter().any(|f| f.name == "floor_2" && f.module == "math"));
    }
    #[test]
    fn std_math_floor_3() {
        assert!(STDLIB.iter().any(|f| f.name == "floor_3" && f.module == "math"));
    }
    #[test]
    fn std_math_floor_4() {
        assert!(STDLIB.iter().any(|f| f.name == "floor_4" && f.module == "math"));
    }
    #[test]
    fn std_math_ceil() {
        assert!(STDLIB.iter().any(|f| f.name == "ceil" && f.module == "math"));
    }
    #[test]
    fn std_math_ceil_1() {
        assert!(STDLIB.iter().any(|f| f.name == "ceil_1" && f.module == "math"));
    }
    #[test]
    fn std_math_ceil_2() {
        assert!(STDLIB.iter().any(|f| f.name == "ceil_2" && f.module == "math"));
    }
    #[test]
    fn std_math_ceil_3() {
        assert!(STDLIB.iter().any(|f| f.name == "ceil_3" && f.module == "math"));
    }
    #[test]
    fn std_math_ceil_4() {
        assert!(STDLIB.iter().any(|f| f.name == "ceil_4" && f.module == "math"));
    }
    #[test]
    fn std_math_pow() {
        assert!(STDLIB.iter().any(|f| f.name == "pow" && f.module == "math"));
    }
    #[test]
    fn std_math_pow_1() {
        assert!(STDLIB.iter().any(|f| f.name == "pow_1" && f.module == "math"));
    }
    #[test]
    fn std_math_pow_2() {
        assert!(STDLIB.iter().any(|f| f.name == "pow_2" && f.module == "math"));
    }
    #[test]
    fn std_math_pow_3() {
        assert!(STDLIB.iter().any(|f| f.name == "pow_3" && f.module == "math"));
    }
    #[test]
    fn std_math_pow_4() {
        assert!(STDLIB.iter().any(|f| f.name == "pow_4" && f.module == "math"));
    }
    #[test]
    fn std_string_len() {
        assert!(STDLIB.iter().any(|f| f.name == "len" && f.module == "string"));
    }
    #[test]
    fn std_string_len_1() {
        assert!(STDLIB.iter().any(|f| f.name == "len_1" && f.module == "string"));
    }
    #[test]
    fn std_string_len_2() {
        assert!(STDLIB.iter().any(|f| f.name == "len_2" && f.module == "string"));
    }
    #[test]
    fn std_string_len_3() {
        assert!(STDLIB.iter().any(|f| f.name == "len_3" && f.module == "string"));
    }
    #[test]
    fn std_string_len_4() {
        assert!(STDLIB.iter().any(|f| f.name == "len_4" && f.module == "string"));
    }
    #[test]
    fn std_string_contains() {
        assert!(STDLIB.iter().any(|f| f.name == "contains" && f.module == "string"));
    }
    #[test]
    fn std_string_contains_1() {
        assert!(STDLIB.iter().any(|f| f.name == "contains_1" && f.module == "string"));
    }
    #[test]
    fn std_string_contains_2() {
        assert!(STDLIB.iter().any(|f| f.name == "contains_2" && f.module == "string"));
    }
    #[test]
    fn std_string_contains_3() {
        assert!(STDLIB.iter().any(|f| f.name == "contains_3" && f.module == "string"));
    }
    #[test]
    fn std_string_contains_4() {
        assert!(STDLIB.iter().any(|f| f.name == "contains_4" && f.module == "string"));
    }
    #[test]
    fn std_string_starts_with() {
        assert!(STDLIB.iter().any(|f| f.name == "starts_with" && f.module == "string"));
    }
    #[test]
    fn std_string_starts_with_1() {
        assert!(STDLIB.iter().any(|f| f.name == "starts_with_1" && f.module == "string"));
    }
    #[test]
    fn std_string_starts_with_2() {
        assert!(STDLIB.iter().any(|f| f.name == "starts_with_2" && f.module == "string"));
    }
    #[test]
    fn std_string_starts_with_3() {
        assert!(STDLIB.iter().any(|f| f.name == "starts_with_3" && f.module == "string"));
    }
    #[test]
    fn std_string_starts_with_4() {
        assert!(STDLIB.iter().any(|f| f.name == "starts_with_4" && f.module == "string"));
    }
    #[test]
    fn std_string_ends_with() {
        assert!(STDLIB.iter().any(|f| f.name == "ends_with" && f.module == "string"));
    }
    #[test]
    fn std_string_ends_with_1() {
        assert!(STDLIB.iter().any(|f| f.name == "ends_with_1" && f.module == "string"));
    }
    #[test]
    fn std_string_ends_with_2() {
        assert!(STDLIB.iter().any(|f| f.name == "ends_with_2" && f.module == "string"));
    }
    #[test]
    fn std_string_ends_with_3() {
        assert!(STDLIB.iter().any(|f| f.name == "ends_with_3" && f.module == "string"));
    }
    #[test]
    fn std_string_ends_with_4() {
        assert!(STDLIB.iter().any(|f| f.name == "ends_with_4" && f.module == "string"));
    }
    #[test]
    fn std_string_trim() {
        assert!(STDLIB.iter().any(|f| f.name == "trim" && f.module == "string"));
    }
    #[test]
    fn std_string_trim_1() {
        assert!(STDLIB.iter().any(|f| f.name == "trim_1" && f.module == "string"));
    }
    #[test]
    fn std_string_trim_2() {
        assert!(STDLIB.iter().any(|f| f.name == "trim_2" && f.module == "string"));
    }
    #[test]
    fn std_string_trim_3() {
        assert!(STDLIB.iter().any(|f| f.name == "trim_3" && f.module == "string"));
    }
    #[test]
    fn std_string_trim_4() {
        assert!(STDLIB.iter().any(|f| f.name == "trim_4" && f.module == "string"));
    }
    #[test]
    fn std_string_to_upper() {
        assert!(STDLIB.iter().any(|f| f.name == "to_upper" && f.module == "string"));
    }
    #[test]
    fn std_string_to_upper_1() {
        assert!(STDLIB.iter().any(|f| f.name == "to_upper_1" && f.module == "string"));
    }
    #[test]
    fn std_string_to_upper_2() {
        assert!(STDLIB.iter().any(|f| f.name == "to_upper_2" && f.module == "string"));
    }
    #[test]
    fn std_string_to_upper_3() {
        assert!(STDLIB.iter().any(|f| f.name == "to_upper_3" && f.module == "string"));
    }
    #[test]
    fn std_string_to_upper_4() {
        assert!(STDLIB.iter().any(|f| f.name == "to_upper_4" && f.module == "string"));
    }
    #[test]
    fn std_string_to_lower() {
        assert!(STDLIB.iter().any(|f| f.name == "to_lower" && f.module == "string"));
    }
    #[test]
    fn std_string_to_lower_1() {
        assert!(STDLIB.iter().any(|f| f.name == "to_lower_1" && f.module == "string"));
    }
    #[test]
    fn std_string_to_lower_2() {
        assert!(STDLIB.iter().any(|f| f.name == "to_lower_2" && f.module == "string"));
    }
    #[test]
    fn std_string_to_lower_3() {
        assert!(STDLIB.iter().any(|f| f.name == "to_lower_3" && f.module == "string"));
    }
    #[test]
    fn std_string_to_lower_4() {
        assert!(STDLIB.iter().any(|f| f.name == "to_lower_4" && f.module == "string"));
    }
    #[test]
    fn std_string_replace() {
        assert!(STDLIB.iter().any(|f| f.name == "replace" && f.module == "string"));
    }
    #[test]
    fn std_string_replace_1() {
        assert!(STDLIB.iter().any(|f| f.name == "replace_1" && f.module == "string"));
    }
    #[test]
    fn std_string_replace_2() {
        assert!(STDLIB.iter().any(|f| f.name == "replace_2" && f.module == "string"));
    }
    #[test]
    fn std_string_replace_3() {
        assert!(STDLIB.iter().any(|f| f.name == "replace_3" && f.module == "string"));
    }
    #[test]
    fn std_string_replace_4() {
        assert!(STDLIB.iter().any(|f| f.name == "replace_4" && f.module == "string"));
    }
    #[test]
    fn std_layer_push_layer() {
        assert!(STDLIB.iter().any(|f| f.name == "push_layer" && f.module == "layer"));
    }
    #[test]
    fn std_layer_push_layer_1() {
        assert!(STDLIB.iter().any(|f| f.name == "push_layer_1" && f.module == "layer"));
    }
    #[test]
    fn std_layer_push_layer_2() {
        assert!(STDLIB.iter().any(|f| f.name == "push_layer_2" && f.module == "layer"));
    }
    #[test]
    fn std_layer_push_layer_3() {
        assert!(STDLIB.iter().any(|f| f.name == "push_layer_3" && f.module == "layer"));
    }
    #[test]
    fn std_layer_push_layer_4() {
        assert!(STDLIB.iter().any(|f| f.name == "push_layer_4" && f.module == "layer"));
    }
    #[test]
    fn std_layer_pop_layer() {
        assert!(STDLIB.iter().any(|f| f.name == "pop_layer" && f.module == "layer"));
    }
    #[test]
    fn std_layer_pop_layer_1() {
        assert!(STDLIB.iter().any(|f| f.name == "pop_layer_1" && f.module == "layer"));
    }
    #[test]
    fn std_layer_pop_layer_2() {
        assert!(STDLIB.iter().any(|f| f.name == "pop_layer_2" && f.module == "layer"));
    }
    #[test]
    fn std_layer_pop_layer_3() {
        assert!(STDLIB.iter().any(|f| f.name == "pop_layer_3" && f.module == "layer"));
    }
    #[test]
    fn std_layer_pop_layer_4() {
        assert!(STDLIB.iter().any(|f| f.name == "pop_layer_4" && f.module == "layer"));
    }
    #[test]
    fn std_layer_show_layer() {
        assert!(STDLIB.iter().any(|f| f.name == "show_layer" && f.module == "layer"));
    }
    #[test]
    fn std_layer_show_layer_1() {
        assert!(STDLIB.iter().any(|f| f.name == "show_layer_1" && f.module == "layer"));
    }
    #[test]
    fn std_layer_show_layer_2() {
        assert!(STDLIB.iter().any(|f| f.name == "show_layer_2" && f.module == "layer"));
    }
    #[test]
    fn std_layer_show_layer_3() {
        assert!(STDLIB.iter().any(|f| f.name == "show_layer_3" && f.module == "layer"));
    }
    #[test]
    fn std_layer_show_layer_4() {
        assert!(STDLIB.iter().any(|f| f.name == "show_layer_4" && f.module == "layer"));
    }
    #[test]
    fn std_layer_hide_layer() {
        assert!(STDLIB.iter().any(|f| f.name == "hide_layer" && f.module == "layer"));
    }
    #[test]
    fn std_layer_hide_layer_1() {
        assert!(STDLIB.iter().any(|f| f.name == "hide_layer_1" && f.module == "layer"));
    }
    #[test]
    fn std_layer_hide_layer_2() {
        assert!(STDLIB.iter().any(|f| f.name == "hide_layer_2" && f.module == "layer"));
    }
    #[test]
    fn std_layer_hide_layer_3() {
        assert!(STDLIB.iter().any(|f| f.name == "hide_layer_3" && f.module == "layer"));
    }
    #[test]
    fn std_layer_hide_layer_4() {
        assert!(STDLIB.iter().any(|f| f.name == "hide_layer_4" && f.module == "layer"));
    }
    #[test]
    fn std_layer_set_layer_z() {
        assert!(STDLIB.iter().any(|f| f.name == "set_layer_z" && f.module == "layer"));
    }
    #[test]
    fn std_layer_set_layer_z_1() {
        assert!(STDLIB.iter().any(|f| f.name == "set_layer_z_1" && f.module == "layer"));
    }
    #[test]
    fn std_layer_set_layer_z_2() {
        assert!(STDLIB.iter().any(|f| f.name == "set_layer_z_2" && f.module == "layer"));
    }
    #[test]
    fn std_layer_set_layer_z_3() {
        assert!(STDLIB.iter().any(|f| f.name == "set_layer_z_3" && f.module == "layer"));
    }
    #[test]
    fn std_layer_set_layer_z_4() {
        assert!(STDLIB.iter().any(|f| f.name == "set_layer_z_4" && f.module == "layer"));
    }
    #[test]
    fn std_layer_layer_id() {
        assert!(STDLIB.iter().any(|f| f.name == "layer_id" && f.module == "layer"));
    }
    #[test]
    fn std_layer_layer_id_1() {
        assert!(STDLIB.iter().any(|f| f.name == "layer_id_1" && f.module == "layer"));
    }
    #[test]
    fn std_layer_layer_id_2() {
        assert!(STDLIB.iter().any(|f| f.name == "layer_id_2" && f.module == "layer"));
    }
    #[test]
    fn std_layer_layer_id_3() {
        assert!(STDLIB.iter().any(|f| f.name == "layer_id_3" && f.module == "layer"));
    }
    #[test]
    fn std_layer_layer_id_4() {
        assert!(STDLIB.iter().any(|f| f.name == "layer_id_4" && f.module == "layer"));
    }
    #[test]
    fn std_audio_play_bgm() {
        assert!(STDLIB.iter().any(|f| f.name == "play_bgm" && f.module == "audio"));
    }
    #[test]
    fn std_audio_play_bgm_1() {
        assert!(STDLIB.iter().any(|f| f.name == "play_bgm_1" && f.module == "audio"));
    }
    #[test]
    fn std_audio_play_bgm_2() {
        assert!(STDLIB.iter().any(|f| f.name == "play_bgm_2" && f.module == "audio"));
    }
    #[test]
    fn std_audio_play_bgm_3() {
        assert!(STDLIB.iter().any(|f| f.name == "play_bgm_3" && f.module == "audio"));
    }
    #[test]
    fn std_audio_play_bgm_4() {
        assert!(STDLIB.iter().any(|f| f.name == "play_bgm_4" && f.module == "audio"));
    }
    #[test]
    fn std_audio_stop_bgm() {
        assert!(STDLIB.iter().any(|f| f.name == "stop_bgm" && f.module == "audio"));
    }
    #[test]
    fn std_audio_stop_bgm_1() {
        assert!(STDLIB.iter().any(|f| f.name == "stop_bgm_1" && f.module == "audio"));
    }
    #[test]
    fn std_audio_stop_bgm_2() {
        assert!(STDLIB.iter().any(|f| f.name == "stop_bgm_2" && f.module == "audio"));
    }
    #[test]
    fn std_audio_stop_bgm_3() {
        assert!(STDLIB.iter().any(|f| f.name == "stop_bgm_3" && f.module == "audio"));
    }
    #[test]
    fn std_audio_stop_bgm_4() {
        assert!(STDLIB.iter().any(|f| f.name == "stop_bgm_4" && f.module == "audio"));
    }
    #[test]
    fn std_audio_play_sfx() {
        assert!(STDLIB.iter().any(|f| f.name == "play_sfx" && f.module == "audio"));
    }
    #[test]
    fn std_audio_play_sfx_1() {
        assert!(STDLIB.iter().any(|f| f.name == "play_sfx_1" && f.module == "audio"));
    }
    #[test]
    fn std_audio_play_sfx_2() {
        assert!(STDLIB.iter().any(|f| f.name == "play_sfx_2" && f.module == "audio"));
    }
    #[test]
    fn std_audio_play_sfx_3() {
        assert!(STDLIB.iter().any(|f| f.name == "play_sfx_3" && f.module == "audio"));
    }
    #[test]
    fn std_audio_play_sfx_4() {
        assert!(STDLIB.iter().any(|f| f.name == "play_sfx_4" && f.module == "audio"));
    }
    #[test]
    fn std_audio_play_voice() {
        assert!(STDLIB.iter().any(|f| f.name == "play_voice" && f.module == "audio"));
    }
    #[test]
    fn std_audio_play_voice_1() {
        assert!(STDLIB.iter().any(|f| f.name == "play_voice_1" && f.module == "audio"));
    }
    #[test]
    fn std_audio_play_voice_2() {
        assert!(STDLIB.iter().any(|f| f.name == "play_voice_2" && f.module == "audio"));
    }
    #[test]
    fn std_audio_play_voice_3() {
        assert!(STDLIB.iter().any(|f| f.name == "play_voice_3" && f.module == "audio"));
    }
    #[test]
    fn std_audio_play_voice_4() {
        assert!(STDLIB.iter().any(|f| f.name == "play_voice_4" && f.module == "audio"));
    }
    #[test]
    fn std_audio_set_volume() {
        assert!(STDLIB.iter().any(|f| f.name == "set_volume" && f.module == "audio"));
    }
    #[test]
    fn std_audio_set_volume_1() {
        assert!(STDLIB.iter().any(|f| f.name == "set_volume_1" && f.module == "audio"));
    }
    #[test]
    fn std_audio_set_volume_2() {
        assert!(STDLIB.iter().any(|f| f.name == "set_volume_2" && f.module == "audio"));
    }
    #[test]
    fn std_audio_set_volume_3() {
        assert!(STDLIB.iter().any(|f| f.name == "set_volume_3" && f.module == "audio"));
    }
    #[test]
    fn std_audio_set_volume_4() {
        assert!(STDLIB.iter().any(|f| f.name == "set_volume_4" && f.module == "audio"));
    }
    #[test]
    fn std_story_say() {
        assert!(STDLIB.iter().any(|f| f.name == "say" && f.module == "story"));
    }
    #[test]
    fn std_story_say_1() {
        assert!(STDLIB.iter().any(|f| f.name == "say_1" && f.module == "story"));
    }
    #[test]
    fn std_story_say_2() {
        assert!(STDLIB.iter().any(|f| f.name == "say_2" && f.module == "story"));
    }
    #[test]
    fn std_story_say_3() {
        assert!(STDLIB.iter().any(|f| f.name == "say_3" && f.module == "story"));
    }
    #[test]
    fn std_story_say_4() {
        assert!(STDLIB.iter().any(|f| f.name == "say_4" && f.module == "story"));
    }
    #[test]
    fn std_story_jump() {
        assert!(STDLIB.iter().any(|f| f.name == "jump" && f.module == "story"));
    }
    #[test]
    fn std_story_jump_1() {
        assert!(STDLIB.iter().any(|f| f.name == "jump_1" && f.module == "story"));
    }
    #[test]
    fn std_story_jump_2() {
        assert!(STDLIB.iter().any(|f| f.name == "jump_2" && f.module == "story"));
    }
    #[test]
    fn std_story_jump_3() {
        assert!(STDLIB.iter().any(|f| f.name == "jump_3" && f.module == "story"));
    }
    #[test]
    fn std_story_jump_4() {
        assert!(STDLIB.iter().any(|f| f.name == "jump_4" && f.module == "story"));
    }
    #[test]
    fn std_story_call_scene() {
        assert!(STDLIB.iter().any(|f| f.name == "call_scene" && f.module == "story"));
    }
    #[test]
    fn std_story_call_scene_1() {
        assert!(STDLIB.iter().any(|f| f.name == "call_scene_1" && f.module == "story"));
    }
    #[test]
    fn std_story_call_scene_2() {
        assert!(STDLIB.iter().any(|f| f.name == "call_scene_2" && f.module == "story"));
    }
    #[test]
    fn std_story_call_scene_3() {
        assert!(STDLIB.iter().any(|f| f.name == "call_scene_3" && f.module == "story"));
    }
    #[test]
    fn std_story_call_scene_4() {
        assert!(STDLIB.iter().any(|f| f.name == "call_scene_4" && f.module == "story"));
    }
    #[test]
    fn std_story_menu_select() {
        assert!(STDLIB.iter().any(|f| f.name == "menu_select" && f.module == "story"));
    }
    #[test]
    fn std_story_menu_select_1() {
        assert!(STDLIB.iter().any(|f| f.name == "menu_select_1" && f.module == "story"));
    }
    #[test]
    fn std_story_menu_select_2() {
        assert!(STDLIB.iter().any(|f| f.name == "menu_select_2" && f.module == "story"));
    }
    #[test]
    fn std_story_menu_select_3() {
        assert!(STDLIB.iter().any(|f| f.name == "menu_select_3" && f.module == "story"));
    }
    #[test]
    fn std_story_menu_select_4() {
        assert!(STDLIB.iter().any(|f| f.name == "menu_select_4" && f.module == "story"));
    }
    #[test]
    fn std_story_show_char() {
        assert!(STDLIB.iter().any(|f| f.name == "show_char" && f.module == "story"));
    }
    #[test]
    fn std_story_show_char_1() {
        assert!(STDLIB.iter().any(|f| f.name == "show_char_1" && f.module == "story"));
    }
    #[test]
    fn std_story_show_char_2() {
        assert!(STDLIB.iter().any(|f| f.name == "show_char_2" && f.module == "story"));
    }
    #[test]
    fn std_story_show_char_3() {
        assert!(STDLIB.iter().any(|f| f.name == "show_char_3" && f.module == "story"));
    }
    #[test]
    fn std_story_show_char_4() {
        assert!(STDLIB.iter().any(|f| f.name == "show_char_4" && f.module == "story"));
    }
    #[test]
    fn std_story_hide_char() {
        assert!(STDLIB.iter().any(|f| f.name == "hide_char" && f.module == "story"));
    }
    #[test]
    fn std_story_hide_char_1() {
        assert!(STDLIB.iter().any(|f| f.name == "hide_char_1" && f.module == "story"));
    }
    #[test]
    fn std_story_hide_char_2() {
        assert!(STDLIB.iter().any(|f| f.name == "hide_char_2" && f.module == "story"));
    }
    #[test]
    fn std_story_hide_char_3() {
        assert!(STDLIB.iter().any(|f| f.name == "hide_char_3" && f.module == "story"));
    }
    #[test]
    fn std_story_hide_char_4() {
        assert!(STDLIB.iter().any(|f| f.name == "hide_char_4" && f.module == "story"));
    }
    #[test]
    fn std_i18n_t() {
        assert!(STDLIB.iter().any(|f| f.name == "t" && f.module == "i18n"));
    }
    #[test]
    fn std_i18n_t_1() {
        assert!(STDLIB.iter().any(|f| f.name == "t_1" && f.module == "i18n"));
    }
    #[test]
    fn std_i18n_t_2() {
        assert!(STDLIB.iter().any(|f| f.name == "t_2" && f.module == "i18n"));
    }
    #[test]
    fn std_i18n_t_3() {
        assert!(STDLIB.iter().any(|f| f.name == "t_3" && f.module == "i18n"));
    }
    #[test]
    fn std_i18n_t_4() {
        assert!(STDLIB.iter().any(|f| f.name == "t_4" && f.module == "i18n"));
    }
    #[test]
    fn std_i18n_has_key() {
        assert!(STDLIB.iter().any(|f| f.name == "has_key" && f.module == "i18n"));
    }
    #[test]
    fn std_i18n_has_key_1() {
        assert!(STDLIB.iter().any(|f| f.name == "has_key_1" && f.module == "i18n"));
    }
    #[test]
    fn std_i18n_has_key_2() {
        assert!(STDLIB.iter().any(|f| f.name == "has_key_2" && f.module == "i18n"));
    }
    #[test]
    fn std_i18n_has_key_3() {
        assert!(STDLIB.iter().any(|f| f.name == "has_key_3" && f.module == "i18n"));
    }
    #[test]
    fn std_i18n_has_key_4() {
        assert!(STDLIB.iter().any(|f| f.name == "has_key_4" && f.module == "i18n"));
    }
    #[test]
    fn std_i18n_locale() {
        assert!(STDLIB.iter().any(|f| f.name == "locale" && f.module == "i18n"));
    }
    #[test]
    fn std_i18n_locale_1() {
        assert!(STDLIB.iter().any(|f| f.name == "locale_1" && f.module == "i18n"));
    }
    #[test]
    fn std_i18n_locale_2() {
        assert!(STDLIB.iter().any(|f| f.name == "locale_2" && f.module == "i18n"));
    }
    #[test]
    fn std_i18n_locale_3() {
        assert!(STDLIB.iter().any(|f| f.name == "locale_3" && f.module == "i18n"));
    }
    #[test]
    fn std_i18n_locale_4() {
        assert!(STDLIB.iter().any(|f| f.name == "locale_4" && f.module == "i18n"));
    }
    #[test]
    fn std_i18n_set_locale() {
        assert!(STDLIB.iter().any(|f| f.name == "set_locale" && f.module == "i18n"));
    }
    #[test]
    fn std_i18n_set_locale_1() {
        assert!(STDLIB.iter().any(|f| f.name == "set_locale_1" && f.module == "i18n"));
    }
    #[test]
    fn std_i18n_set_locale_2() {
        assert!(STDLIB.iter().any(|f| f.name == "set_locale_2" && f.module == "i18n"));
    }
    #[test]
    fn std_i18n_set_locale_3() {
        assert!(STDLIB.iter().any(|f| f.name == "set_locale_3" && f.module == "i18n"));
    }
    #[test]
    fn std_i18n_set_locale_4() {
        assert!(STDLIB.iter().any(|f| f.name == "set_locale_4" && f.module == "i18n"));
    }
    #[test]
    fn std_input_pressed() {
        assert!(STDLIB.iter().any(|f| f.name == "pressed" && f.module == "input"));
    }
    #[test]
    fn std_input_pressed_1() {
        assert!(STDLIB.iter().any(|f| f.name == "pressed_1" && f.module == "input"));
    }
    #[test]
    fn std_input_pressed_2() {
        assert!(STDLIB.iter().any(|f| f.name == "pressed_2" && f.module == "input"));
    }
    #[test]
    fn std_input_pressed_3() {
        assert!(STDLIB.iter().any(|f| f.name == "pressed_3" && f.module == "input"));
    }
    #[test]
    fn std_input_pressed_4() {
        assert!(STDLIB.iter().any(|f| f.name == "pressed_4" && f.module == "input"));
    }
}
