//! The user's language preferences, for a game's first-run default (a saved choice should still win).

/// The system's preferred languages as BCP 47 tags (`es-ES`, `es-419`, `pt-BR`, `zh-Hant-TW`, `en`),
/// ordered from most preferred to least preferred:
///
/// - Android: the per-app `LocaleList` (`LocaleManager.getApplicationLocales` on Android 13+) when set,
///   else the activity's configuration `LocaleList` and `LocaleList.getDefault()`, with `AConfiguration`
///   as a last resort;
/// - Windows: the user's preferred UI language list (`GetUserPreferredUILanguages`), falling back to
///   `GetUserDefaultLocaleName`;
/// - elsewhere: `LC_ALL`, `LANGUAGE`, `LC_MESSAGES` and `LANG` (`es_ES.UTF-8` becomes `es-ES`; `C` and
///   `POSIX` say nothing).
pub fn system_locales() -> Vec<String> {
    #[cfg(target_os = "android")]
    {
        android_locales()
    }
    #[cfg(windows)]
    {
        windows_ui_languages()
    }
    #[cfg(not(any(target_os = "android", windows)))]
    {
        posix_locales_from_vars(
            std::env::var("LC_ALL").ok().as_deref(),
            std::env::var("LC_MESSAGES").ok().as_deref(),
            std::env::var("LANG").ok().as_deref(),
            std::env::var("LANGUAGE").ok().as_deref(),
        )
    }
}

/// The system's first preferred language as a BCP 47 tag (`es-ES`, `pt-BR`, `en`).
///
/// Games that support a fixed set of languages should prefer [`system_locales`] and pick the first
/// tag they speak, so a secondary language works when the primary one is unsupported.
pub fn system_locale() -> Option<String> {
    system_locales().into_iter().next()
}

/// `es_ES.UTF-8@euro` → `es-ES`; `C`, `POSIX` and empty values → `None`.
pub fn posix_to_bcp47(value: &str) -> Option<String> {
    let tag = value.split(['.', '@']).next()?.trim();
    if tag.is_empty() || tag.eq_ignore_ascii_case("c") || tag.eq_ignore_ascii_case("posix") {
        return None;
    }
    Some(tag.replace('_', "-"))
}

fn push_unique_tag(out: &mut Vec<String>, raw: &str) {
    if let Some(tag) = posix_to_bcp47(raw) {
        if !tag.eq_ignore_ascii_case("und")
            && !out
                .iter()
                .any(|existing| existing.eq_ignore_ascii_case(&tag))
        {
            out.push(tag);
        }
    }
}

/// Parses a comma-separated list of BCP 47 tags (such as Android's `LocaleList.toLanguageTags()`).
#[cfg_attr(not(any(target_os = "android", test)), allow(dead_code))]
pub(crate) fn parse_tag_list(list: &str) -> Vec<String> {
    let mut out = Vec::new();
    for part in list.split(',') {
        push_unique_tag(&mut out, part);
    }
    out
}

/// Resolves POSIX locale environment variables (`LC_ALL`, `LC_MESSAGES`, `LANG`, and GNU `LANGUAGE`).
#[cfg_attr(any(target_os = "android", windows), allow(dead_code))]
pub(crate) fn posix_locales_from_vars(
    lc_all: Option<&str>,
    lc_messages: Option<&str>,
    lang: Option<&str>,
    language: Option<&str>,
) -> Vec<String> {
    let effective = [lc_all, lc_messages, lang]
        .into_iter()
        .flatten()
        .map(str::trim)
        .find(|s| !s.is_empty());
    // When the effective POSIX category is explicitly C or POSIX, ignore LANGUAGE fallbacks.
    if let Some(eff) = effective {
        if posix_to_bcp47(eff).is_none() {
            return Vec::new();
        }
    }
    let mut out = Vec::new();
    if let Some(list) = language {
        for item in list.split(':') {
            push_unique_tag(&mut out, item);
        }
    }
    for var in [lc_all, lc_messages, lang].into_iter().flatten() {
        push_unique_tag(&mut out, var);
    }
    out
}

/// Parses a Windows double-NUL-terminated UTF-16 string list (`REG_MULTI_SZ` / `GetUserPreferredUILanguages`).
#[cfg_attr(not(any(windows, test)), allow(dead_code))]
pub(crate) fn parse_multi_sz(words: &[u16]) -> Vec<String> {
    let mut out = Vec::new();
    let mut start = 0;
    while start < words.len() {
        let len = words[start..]
            .iter()
            .position(|&c| c == 0)
            .unwrap_or(words.len() - start);
        if len == 0 {
            break;
        }
        let s = String::from_utf16_lossy(&words[start..start + len]);
        push_unique_tag(&mut out, &s);
        start += len + 1;
    }
    out
}

#[cfg(windows)]
fn windows_ui_languages() -> Vec<String> {
    #[link(name = "kernel32")]
    extern "system" {
        fn GetUserPreferredUILanguages(
            flags: u32,
            languages: *mut u32,
            buffer: *mut u16,
            buffer_len: *mut u32,
        ) -> i32;
        fn GetUserDefaultLocaleName(buffer: *mut u16, buffer_len: i32) -> i32;
    }
    const MUI_LANGUAGE_NAME: u32 = 0x8;
    let mut len = 0u32;
    let mut count = 0u32;
    // SAFETY: passing a null buffer queries the required length in UTF-16 code units.
    let _ = unsafe {
        GetUserPreferredUILanguages(
            MUI_LANGUAGE_NAME,
            &mut count,
            std::ptr::null_mut(),
            &mut len,
        )
    };
    let cap = (len as usize).max(512);
    let mut buffer = vec![0u16; cap];
    let mut buf_len = buffer.len() as u32;
    // SAFETY: `buffer` has `buf_len` elements and receives a double-NUL-terminated UTF-16 list.
    let ok = unsafe {
        GetUserPreferredUILanguages(
            MUI_LANGUAGE_NAME,
            &mut count,
            buffer.as_mut_ptr(),
            &mut buf_len,
        )
    };
    let mut out = if ok != 0 && count > 0 {
        parse_multi_sz(&buffer[..(buf_len as usize).min(buffer.len())])
    } else {
        Vec::new()
    };
    if out.is_empty() {
        let mut name_buf = [0u16; 85];
        // SAFETY: `name_buf` is a writable UTF-16 buffer of length 85 (`LOCALE_NAME_MAX_LENGTH`).
        let written =
            unsafe { GetUserDefaultLocaleName(name_buf.as_mut_ptr(), name_buf.len() as i32) };
        if written > 0 {
            let first: Vec<u16> = name_buf.iter().copied().take_while(|&c| c != 0).collect();
            let s = String::from_utf16_lossy(&first);
            push_unique_tag(&mut out, &s);
        }
    }
    out
}

#[cfg(target_os = "android")]
fn android_locales() -> Vec<String> {
    let Some(app) = crate::android::app() else {
        return Vec::new();
    };
    let mut out = android_jni_locales(&app);
    if out.is_empty() {
        let config = app.config();
        if let Some(language) = config.language() {
            let tag = match config.country() {
                Some(country) => format!("{language}-{country}"),
                None => language,
            };
            push_unique_tag(&mut out, &tag);
        }
    }
    out
}

#[cfg(target_os = "android")]
fn android_jni_locales(app: &crate::android::AndroidApp) -> Vec<String> {
    use jni::objects::{JObject, JString, JValue};
    use jni::{jni_sig, jni_str};

    let vm_ptr = app.vm_as_ptr() as *mut jni::sys::JavaVM;
    let activity_raw = app.activity_as_ptr() as jni::sys::jobject;
    if vm_ptr.is_null() || activity_raw.is_null() {
        return Vec::new();
    }
    // SAFETY: `vm_ptr` is android-activity's process-wide JavaVM, and `activity_raw` is a live global reference.
    let vm = unsafe { jni::vm::JavaVM::from_raw(vm_ptr) };
    let sdk = crate::android::sdk_version();

    let read_locale_list = |env: &mut jni::Env<'_>, list: &JObject<'_>| -> Option<Vec<String>> {
        if list.is_null() {
            return None;
        }
        let tags_obj = env
            .call_method(
                list,
                jni_str!("toLanguageTags"),
                jni_sig!("()Ljava/lang/String;"),
                &[],
            )
            .ok()?
            .l()
            .ok()?;
        if tags_obj.is_null() {
            return None;
        }
        let tags_jstr = unsafe { JString::from_raw(env, tags_obj.into_raw()) };
        let tags = tags_jstr.try_to_string(env).ok()?;
        let parsed = parse_tag_list(&tags);
        (!parsed.is_empty()).then_some(parsed)
    };

    let clear_ex = |env: &mut jni::Env<'_>| {
        if env.exception_check() {
            env.exception_clear();
        }
    };

    vm.attach_current_thread(|env| -> jni::errors::Result<Vec<String>> {
        // SAFETY: `activity_raw` is the activity's valid global reference.
        let activity = unsafe { env.as_cast_raw::<JObject>(&activity_raw)? };

        // 1. Android 13+ (API 33+): explicit per-app locales from LocaleManager.getApplicationLocales().
        // When non-empty, the user (or QA via `cmd locale set-app-locales`) chose a language list
        // specifically for this app, which takes precedence over the device's system language list.
        if sdk >= 33 {
            let service_name = env.new_string("locale")?;
            let manager = env
                .call_method(
                    &activity,
                    jni_str!("getSystemService"),
                    jni_sig!("(Ljava/lang/String;)Ljava/lang/Object;"),
                    &[JValue::Object(&service_name)],
                )
                .and_then(|v| v.l());
            clear_ex(env);
            if let Ok(manager) = manager {
                if !manager.is_null() {
                    let app_list = env
                        .call_method(
                            &manager,
                            jni_str!("getApplicationLocales"),
                            jni_sig!("()Landroid/os/LocaleList;"),
                            &[],
                        )
                        .and_then(|v| v.l());
                    clear_ex(env);
                    if let Ok(app_list) = app_list {
                        let got = read_locale_list(env, &app_list);
                        clear_ex(env);
                        if let Some(tags) = got {
                            return Ok(tags);
                        }
                    }
                }
            }
        }

        let mut out = Vec::new();

        // 2. Activity Resources.getConfiguration().getLocales() (API 24+).
        let resources = env
            .call_method(
                &activity,
                jni_str!("getResources"),
                jni_sig!("()Landroid/content/res/Resources;"),
                &[],
            )
            .and_then(|v| v.l());
        clear_ex(env);
        if let Ok(resources) = resources {
            if !resources.is_null() {
                let config = env
                    .call_method(
                        &resources,
                        jni_str!("getConfiguration"),
                        jni_sig!("()Landroid/content/res/Configuration;"),
                        &[],
                    )
                    .and_then(|v| v.l());
                clear_ex(env);
                if let Ok(config) = config {
                    if !config.is_null() {
                        let list = env
                            .call_method(
                                &config,
                                jni_str!("getLocales"),
                                jni_sig!("()Landroid/os/LocaleList;"),
                                &[],
                            )
                            .and_then(|v| v.l());
                        clear_ex(env);
                        if let Ok(list) = list {
                            if let Some(tags) = read_locale_list(env, &list) {
                                for tag in tags {
                                    push_unique_tag(&mut out, &tag);
                                }
                            }
                            clear_ex(env);
                        }
                    }
                }
            }
        }

        // 3. LocaleList.getDefault() (API 24+).
        let default_list = env
            .call_static_method(
                jni_str!("android/os/LocaleList"),
                jni_str!("getDefault"),
                jni_sig!("()Landroid/os/LocaleList;"),
                &[],
            )
            .and_then(|v| v.l());
        clear_ex(env);
        if let Ok(default_list) = default_list {
            if let Some(tags) = read_locale_list(env, &default_list) {
                for tag in tags {
                    push_unique_tag(&mut out, &tag);
                }
            }
            clear_ex(env);
        }

        // 4. Locale.getDefault().toLanguageTag() (API 21+).
        let default_locale = env
            .call_static_method(
                jni_str!("java/util/Locale"),
                jni_str!("getDefault"),
                jni_sig!("()Ljava/util/Locale;"),
                &[],
            )
            .and_then(|v| v.l());
        clear_ex(env);
        if let Ok(default_locale) = default_locale {
            if !default_locale.is_null() {
                let tag_obj = env
                    .call_method(
                        &default_locale,
                        jni_str!("toLanguageTag"),
                        jni_sig!("()Ljava/lang/String;"),
                        &[],
                    )
                    .and_then(|v| v.l());
                clear_ex(env);
                if let Ok(tag_obj) = tag_obj {
                    if !tag_obj.is_null() {
                        let tag_jstr = unsafe { JString::from_raw(env, tag_obj.into_raw()) };
                        if let Ok(tag) = tag_jstr.try_to_string(env) {
                            push_unique_tag(&mut out, &tag);
                        }
                    }
                }
            }
        }

        Ok(out)
    })
    .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn posix_locales_become_tags() {
        assert_eq!(posix_to_bcp47("es_ES.UTF-8").as_deref(), Some("es-ES"));
        assert_eq!(posix_to_bcp47("pt_BR@latin").as_deref(), Some("pt-BR"));
        assert_eq!(posix_to_bcp47("ja").as_deref(), Some("ja"));
        assert_eq!(posix_to_bcp47("C.UTF-8"), None);
        assert_eq!(posix_to_bcp47("c.utf8"), None);
        assert_eq!(posix_to_bcp47("POSIX"), None);
        assert_eq!(posix_to_bcp47("posix"), None);
        assert_eq!(posix_to_bcp47(""), None);
        // Whatever the machine says, it is a tag without POSIX decorations.
        if let Some(tag) = system_locale() {
            assert!(!tag.contains('_') && !tag.contains('.'), "{tag}");
        }
        for tag in system_locales() {
            assert!(!tag.contains('_') && !tag.contains('.'), "{tag}");
        }
    }

    #[test]
    fn multi_sz_and_tag_lists_preserve_preference_order() {
        let mut buf: Vec<u16> = Vec::new();
        for s in ["nl-NL", "es_419", "zh-Hans-HK", "en-US"] {
            buf.extend(s.encode_utf16());
            buf.push(0);
        }
        buf.push(0);
        assert_eq!(
            parse_multi_sz(&buf),
            vec!["nl-NL", "es-419", "zh-Hans-HK", "en-US"]
        );

        assert_eq!(
            parse_tag_list("nl-NL,es-419,pt-PT,und,nl-NL"),
            vec!["nl-NL", "es-419", "pt-PT"]
        );
    }

    #[test]
    fn posix_env_precedence_respects_c_locale_and_language_fallbacks() {
        assert_eq!(
            posix_locales_from_vars(None, None, Some("nl_NL.UTF-8"), Some("nl_NL:es_MX:en_US")),
            vec!["nl-NL", "es-MX", "en-US"]
        );
        // LC_ALL=C overrides LANGUAGE and LANG.
        assert!(posix_locales_from_vars(
            Some("C"),
            Some("es_ES.UTF-8"),
            Some("es_ES.UTF-8"),
            Some("es_ES:en_US"),
        )
        .is_empty());
        assert!(
            posix_locales_from_vars(Some("C.UTF-8"), None, Some("ja_JP.UTF-8"), None).is_empty()
        );
    }
}
