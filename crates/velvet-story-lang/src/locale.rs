//! Configurable writer-facing diagnostic locale (es / en / ja / de / zh).
//!
//! Stable `VSTxxx` codes stay fixed; only human text is localized.
//! Default is Spanish for continuity with existing writer tooling.
//!
//! # Isolation (Studio / multi-doc)
//!
//! Prefer [`with_diag_locale`] or [`DiagLocaleGuard`] so concurrent checks with
//! different languages do not race on a process-global lock alone. Catalog
//! lookup is pure: [`diag_message_for`] / [`diag_suggestion_for`].
//!
//! CLI still may call [`set_diag_locale`] / env `VELVET_STORY_LANG` as the
//! process default when no thread-local scope is active.

use std::cell::RefCell;
use std::sync::RwLock;

use serde::{Deserialize, Serialize};

/// Supported diagnostic UI languages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum DiagLocale {
    /// Español (default).
    #[default]
    Es,
    /// English.
    En,
    /// 日本語.
    Ja,
    /// Deutsch.
    De,
    /// 中文（简体）.
    Zh,
}

impl DiagLocale {
    /// BCP-47-ish short code (`es`, `en`, `ja`, `de`, `zh`).
    pub fn code(self) -> &'static str {
        match self {
            Self::Es => "es",
            Self::En => "en",
            Self::Ja => "ja",
            Self::De => "de",
            Self::Zh => "zh",
        }
    }

    /// Parse `es`/`en`/`ja`/`de`/`zh` (case-insensitive; accepts aliases).
    pub fn parse(s: &str) -> Result<Self, String> {
        let t = s.trim().to_ascii_lowercase();
        match t.as_str() {
            "es" | "spa" | "spanish" | "español" | "espanol" => Ok(Self::Es),
            "en" | "eng" | "english" => Ok(Self::En),
            "ja" | "jp" | "jpn" | "japanese" => Ok(Self::Ja),
            "de" | "ger" | "deu" | "german" | "deutsch" => Ok(Self::De),
            "zh" | "cn" | "zho" | "chinese" | "zh-cn" | "zh_hans" => Ok(Self::Zh),
            _ => Err(format!(
                "unknown story diag locale `{s}` (use es|en|ja|de|zh)"
            )),
        }
    }

    /// All supported locales in stable order.
    pub fn all() -> &'static [DiagLocale] {
        &[
            DiagLocale::Es,
            DiagLocale::En,
            DiagLocale::Ja,
            DiagLocale::De,
            DiagLocale::Zh,
        ]
    }
}

/// Process-wide default (CLI / single-consumer tools). Prefer thread-local
/// scopes for multi-document concurrent validation.
static DEFAULT_LOCALE: RwLock<DiagLocale> = RwLock::new(DiagLocale::Es);

thread_local! {
    /// Stack of scoped locales for the current thread (Studio multi-doc safe).
    static LOCALE_STACK: RefCell<Vec<DiagLocale>> = RefCell::new(Vec::new());
}

/// Set the process-wide **default** diagnostic locale (no active scope).
///
/// Does not override an active [`with_diag_locale`] / [`DiagLocaleGuard`] on
/// this thread. Prefer scopes when multiple consumers share one process.
pub fn set_diag_locale(locale: DiagLocale) {
    if let Ok(mut g) = DEFAULT_LOCALE.write() {
        *g = locale;
    }
}

/// Process-wide default locale (ignores thread-local stack).
pub fn default_diag_locale() -> DiagLocale {
    DEFAULT_LOCALE.read().map(|g| *g).unwrap_or_default()
}

/// Effective locale: top of thread-local stack, else process default.
pub fn diag_locale() -> DiagLocale {
    LOCALE_STACK.with(|s| s.borrow().last().copied()).unwrap_or_else(default_diag_locale)
}

/// RAII push of a thread-local diagnostic locale (pop on drop).
#[derive(Debug)]
pub struct DiagLocaleGuard {
    _private: (),
}

impl Drop for DiagLocaleGuard {
    fn drop(&mut self) {
        LOCALE_STACK.with(|s| {
            s.borrow_mut().pop();
        });
    }
}

/// Push `locale` for the current thread until the guard is dropped.
pub fn push_diag_locale(locale: DiagLocale) -> DiagLocaleGuard {
    LOCALE_STACK.with(|s| s.borrow_mut().push(locale));
    DiagLocaleGuard { _private: () }
}

/// Run `f` with `locale` as the effective diagnostic language on this thread.
///
/// Nested scopes restore the previous locale on exit. Safe for concurrent
/// threads each calling with a different locale (no cross-thread bleed).
pub fn with_diag_locale<R>(locale: DiagLocale, f: impl FnOnce() -> R) -> R {
    let _guard = push_diag_locale(locale);
    f()
}

/// Apply `VELVET_STORY_LANG` if set and valid (ignores invalid values).
pub fn apply_locale_from_env() {
    if let Ok(v) = std::env::var("VELVET_STORY_LANG") {
        if let Ok(loc) = DiagLocale::parse(&v) {
            set_diag_locale(loc);
        }
    }
}

/// Localized label before suggestion text for a given locale.
pub fn suggestion_label_for(locale: DiagLocale) -> &'static str {
    match locale {
        DiagLocale::Es => "Sugerencia:",
        DiagLocale::En => "Suggestion:",
        DiagLocale::Ja => "提案:",
        DiagLocale::De => "Vorschlag:",
        DiagLocale::Zh => "建议:",
    }
}

/// Localized label using the effective locale.
pub fn suggestion_label() -> &'static str {
    suggestion_label_for(diag_locale())
}

/// Pure catalog: primary message for `code` in `locale`.
///
/// Placeholders: `{name}`, `{target}`, `{req}`, `{path}`, `{resolved}`, `{line}`,
/// `{expected}`, `{indent}`, `{ch}`, `{speaker}`, `{hint}`, `{detail}`, `{err}`.
pub fn diag_message_for(locale: DiagLocale, code: &str, args: &[(&str, &str)]) -> String {
    fill(template_message(locale, code), args)
}

/// Localized primary message using the effective locale.
pub fn diag_message(code: &str, args: &[(&str, &str)]) -> String {
    diag_message_for(diag_locale(), code, args)
}

/// Pure catalog: suggestion body for `code` in `locale`, if any.
pub fn diag_suggestion_for(locale: DiagLocale, code: &str, args: &[(&str, &str)]) -> Option<String> {
    template_suggestion(locale, code).map(|t| fill(t, args))
}

/// Localized suggestion using the effective locale.
pub fn diag_suggestion(code: &str, args: &[(&str, &str)]) -> Option<String> {
    diag_suggestion_for(diag_locale(), code, args)
}

/// Hint fragment for invalid `if` conditions (used as `{hint}` in VST030).
pub fn if_cond_hint_str_for(locale: DiagLocale, s: &str) -> String {
    fill(template_hint(locale, "str"), &[("s", s)])
}

/// Hint fragment for invalid `if` conditions (effective locale).
pub fn if_cond_hint_str(s: &str) -> String {
    if_cond_hint_str_for(diag_locale(), s)
}

/// Hint for bare integer in `if`.
pub fn if_cond_hint_int_for(locale: DiagLocale, n: i64) -> String {
    let ns = n.to_string();
    fill(template_hint(locale, "int"), &[("n", &ns)])
}

/// Hint for bare integer in `if` (effective locale).
pub fn if_cond_hint_int(n: i64) -> String {
    if_cond_hint_int_for(diag_locale(), n)
}

/// Hint for bare float text in `if`.
pub fn if_cond_hint_float_for(locale: DiagLocale, s: &str) -> String {
    fill(template_hint(locale, "float"), &[("s", s)])
}

/// Hint for bare float text in `if` (effective locale).
pub fn if_cond_hint_float(s: &str) -> String {
    if_cond_hint_float_for(diag_locale(), s)
}

/// Generic invalid-condition hint.
pub fn if_cond_hint_other_for(locale: DiagLocale) -> String {
    template_hint(locale, "other").to_string()
}

/// Generic invalid-condition hint (effective locale).
pub fn if_cond_hint_other() -> String {
    if_cond_hint_other_for(diag_locale())
}

fn fill(template: &str, args: &[(&str, &str)]) -> String {
    let mut out = template.to_string();
    for (k, v) in args {
        out = out.replace(&format!("{{{k}}}"), v);
    }
    out
}

fn template_message(loc: DiagLocale, code: &str) -> &'static str {
    match code {
        "VST001" => match loc {
            DiagLocale::Es => "No mezcles tabulaciones y espacios en la indentación.",
            DiagLocale::En => "Do not mix tabs and spaces in indentation.",
            DiagLocale::Ja => "インデントでタブとスペースを混ぜないでください。",
            DiagLocale::De => "Mischen Sie keine Tabs und Leerzeichen in der Einrückung.",
            DiagLocale::Zh => "不要在缩进中混用制表符和空格。",
        },
        "VST002" => match loc {
            DiagLocale::Es => {
                "Indentación inconsistente (esperaba {expected} espacios, hay {indent})."
            }
            DiagLocale::En => {
                "Inconsistent indentation (expected {expected} spaces, found {indent})."
            }
            DiagLocale::Ja => {
                "インデントが一致しません（期待 {expected} スペース、実際 {indent}）。"
            }
            DiagLocale::De => {
                "Inkonsistente Einrückung (erwartet {expected} Leerzeichen, gefunden {indent})."
            }
            DiagLocale::Zh => "缩进不一致（期望 {expected} 个空格，实际 {indent}）。",
        },
        "VST003" => match loc {
            DiagLocale::Es => "Carácter no reconocido: {ch}",
            DiagLocale::En => "Unrecognized character: {ch}",
            DiagLocale::Ja => "認識できない文字: {ch}",
            DiagLocale::De => "Unbekanntes Zeichen: {ch}",
            DiagLocale::Zh => "无法识别的字符: {ch}",
        },
        "VST010" => match loc {
            DiagLocale::Es => "No se pudo interpretar esta línea.",
            DiagLocale::En => "Could not parse this line.",
            DiagLocale::Ja => "この行を解釈できませんでした。",
            DiagLocale::De => "Diese Zeile konnte nicht gelesen werden.",
            DiagLocale::Zh => "无法解析这一行。",
        },
        "VST011" => match loc {
            DiagLocale::Es => "Tras include hace falta una ruta.",
            DiagLocale::En => "include requires a path.",
            DiagLocale::Ja => "include の後にパスが必要です。",
            DiagLocale::De => "Nach include ist ein Pfad erforderlich.",
            DiagLocale::Zh => "include 后面需要路径。",
        },
        "VST012" => match loc {
            DiagLocale::Es => "En call, usa `parametro: valor`.",
            DiagLocale::En => "In call, use `parameter: value`.",
            DiagLocale::Ja => "call では `parameter: value` を使ってください。",
            DiagLocale::De => "Bei call bitte `parameter: wert` verwenden.",
            DiagLocale::Zh => "在 call 中请使用 `参数: 值`。",
        },
        "VST013" => match loc {
            DiagLocale::Es => "Tras set usa `set nombre = valor`.",
            DiagLocale::En => "After set use `set name = value`.",
            DiagLocale::Ja => "set の後は `set name = value` を使います。",
            DiagLocale::De => "Nach set bitte `set name = wert` verwenden.",
            DiagLocale::Zh => "set 之后请使用 `set 名称 = 值`。",
        },
        "VST014" => match loc {
            DiagLocale::Es => "Cada opción de choice debe ser un texto entre comillas.",
            DiagLocale::En => "Each choice option must be a quoted string.",
            DiagLocale::Ja => "choice の各選択肢は引用符付き文字列である必要があります。",
            DiagLocale::De => "Jede choice-Option muss ein zitierter Text sein.",
            DiagLocale::Zh => "每个 choice 选项必须是带引号的文本。",
        },
        "VST015" => match loc {
            DiagLocale::Es => {
                "Línea desconocida `{speaker}`. ¿Quisiste `speaker:` para diálogo?"
            }
            DiagLocale::En => {
                "Unknown line `{speaker}`. Did you mean `speaker:` for dialogue?"
            }
            DiagLocale::Ja => {
                "不明な行 `{speaker}`。台詞なら `speaker:` ですか？"
            }
            DiagLocale::De => {
                "Unbekannte Zeile `{speaker}`. Meinten Sie `speaker:` für Dialog?"
            }
            DiagLocale::Zh => {
                "未知行 `{speaker}`。是否想写 `speaker:` 对话？"
            }
        },
        "VST016" => match loc {
            DiagLocale::Es => {
                "Se esperaba un valor (número, texto, true/false o variable)."
            }
            DiagLocale::En => {
                "Expected a value (number, text, true/false, or variable)."
            }
            DiagLocale::Ja => {
                "値（数値、文字列、true/false、変数）が必要です。"
            }
            DiagLocale::De => {
                "Wert erwartet (Zahl, Text, true/false oder Variable)."
            }
            DiagLocale::Zh => "需要一个值（数字、文本、true/false 或变量）。",
        },
        "VST017" => match loc {
            DiagLocale::Es => "Se esperaba un nombre.",
            DiagLocale::En => "Expected a name.",
            DiagLocale::Ja => "名前が必要です。",
            DiagLocale::De => "Name erwartet.",
            DiagLocale::Zh => "需要一个名称。",
        },
        "VST018" => match loc {
            DiagLocale::Es => "Se esperaba un identificador o texto.",
            DiagLocale::En => "Expected an identifier or text.",
            DiagLocale::Ja => "識別子または文字列が必要です。",
            DiagLocale::De => "Bezeichner oder Text erwartet.",
            DiagLocale::Zh => "需要标识符或文本。",
        },
        "VST020" => match loc {
            DiagLocale::Es => "La escena `{name}` ya existe.",
            DiagLocale::En => "Scene `{name}` already exists.",
            DiagLocale::Ja => "シーン `{name}` は既に存在します。",
            DiagLocale::De => "Szene `{name}` existiert bereits.",
            DiagLocale::Zh => "场景 `{name}` 已存在。",
        },
        "VST021" => match loc {
            DiagLocale::Es => {
                "La variable `{name}` se modifica sin un `set` previo; se asumirá 0."
            }
            DiagLocale::En => {
                "Variable `{name}` is modified without a prior `set`; it will be treated as 0."
            }
            DiagLocale::Ja => {
                "変数 `{name}` は先に `set` されていません。0 として扱います。"
            }
            DiagLocale::De => {
                "Variable `{name}` wird ohne vorheriges `set` geändert; gilt als 0."
            }
            DiagLocale::Zh => {
                "变量 `{name}` 在未先 `set` 的情况下被修改；将视为 0。"
            }
        },
        "VST022" => match loc {
            DiagLocale::Es => {
                "Al comando `{name}` le falta el parámetro obligatorio `{req}`."
            }
            DiagLocale::En => {
                "Command `{name}` is missing required parameter `{req}`."
            }
            DiagLocale::Ja => {
                "コマンド `{name}` に必須パラメータ `{req}` がありません。"
            }
            DiagLocale::De => {
                "Befehl `{name}` fehlt der Pflichtparameter `{req}`."
            }
            DiagLocale::Zh => "命令 `{name}` 缺少必需参数 `{req}`。",
        },
        "VST023" => match loc {
            DiagLocale::Es => {
                "El parámetro `{name}` no está documentado para `{cmd}`."
            }
            DiagLocale::En => {
                "Parameter `{name}` is not documented for `{cmd}`."
            }
            DiagLocale::Ja => {
                "パラメータ `{name}` は `{cmd}` に文書化されていません。"
            }
            DiagLocale::De => {
                "Parameter `{name}` ist für `{cmd}` nicht dokumentiert."
            }
            DiagLocale::Zh => "参数 `{name}` 未在 `{cmd}` 中文档化。",
        },
        "VST024" => match loc {
            DiagLocale::Es => {
                "No hay un comando registrado llamado `{name}`. Un programador debe exponerlo desde Velvet Script 2."
            }
            DiagLocale::En => {
                "No registered command named `{name}`. A programmer must expose it from Velvet Script 2."
            }
            DiagLocale::Ja => {
                "登録されたコマンド `{name}` がありません。Velvet Script 2 から公開する必要があります。"
            }
            DiagLocale::De => {
                "Kein registrierter Befehl `{name}`. Ein Programmierer muss ihn aus Velvet Script 2 freigeben."
            }
            DiagLocale::Zh => {
                "没有名为 `{name}` 的已注册命令。需要由程序员从 Velvet Script 2 暴露。"
            }
        },
        "VST025" => match loc {
            DiagLocale::Es => "Este diálogo no tiene texto.",
            DiagLocale::En => "This dialogue has no text.",
            DiagLocale::Ja => "この台詞にテキストがありません。",
            DiagLocale::De => "Dieser Dialog hat keinen Text.",
            DiagLocale::Zh => "这段对话没有文本。",
        },
        "VST026" => match loc {
            DiagLocale::Es => "Un `choice` necesita al menos una opción.",
            DiagLocale::En => "A `choice` needs at least one option.",
            DiagLocale::Ja => "`choice` には少なくとも 1 つの選択肢が必要です。",
            DiagLocale::De => "Ein `choice` braucht mindestens eine Option.",
            DiagLocale::Zh => "`choice` 至少需要一个选项。",
        },
        "VST027" => match loc {
            DiagLocale::Es => "No existe la escena o etiqueta `{target}`.",
            DiagLocale::En => "Scene or label `{target}` does not exist.",
            DiagLocale::Ja => "シーンまたはラベル `{target}` は存在しません。",
            DiagLocale::De => "Szene oder Label `{target}` existiert nicht.",
            DiagLocale::Zh => "场景或标签 `{target}` 不存在。",
        },
        "VST030" => match loc {
            DiagLocale::Es => {
                "La condición de \"if\" debe producir verdadero o falso. {hint}"
            }
            DiagLocale::En => {
                "The \"if\" condition must evaluate to true or false. {hint}"
            }
            DiagLocale::Ja => {
                "\"if\" の条件は真または偽である必要があります。{hint}"
            }
            DiagLocale::De => {
                "Die \"if\"-Bedingung muss wahr oder falsch ergeben. {hint}"
            }
            DiagLocale::Zh => "\"if\" 条件必须为真或假。{hint}",
        },
        "VST040" => match loc {
            DiagLocale::Es => "include circular: {path}",
            DiagLocale::En => "circular include: {path}",
            DiagLocale::Ja => "循環 include: {path}",
            DiagLocale::De => "zirkuläres include: {path}",
            DiagLocale::Zh => "循环 include: {path}",
        },
        "VST041" => match loc {
            DiagLocale::Es => {
                "No se encuentra el archivo incluido `{path}` (buscado en {resolved})."
            }
            DiagLocale::En => {
                "Included file `{path}` not found (looked in {resolved})."
            }
            DiagLocale::Ja => {
                "include ファイル `{path}` が見つかりません（検索: {resolved}）。"
            }
            DiagLocale::De => {
                "Include-Datei `{path}` nicht gefunden (gesucht in {resolved})."
            }
            DiagLocale::Zh => {
                "找不到 include 文件 `{path}`（查找于 {resolved}）。"
            }
        },
        "VST042" => match loc {
            DiagLocale::Es => "Error al cargar include `{path}`: {err}",
            DiagLocale::En => "Failed to load include `{path}`: {err}",
            DiagLocale::Ja => "include `{path}` の読み込みに失敗: {err}",
            DiagLocale::De => "Include `{path}` konnte nicht geladen werden: {err}",
            DiagLocale::Zh => "加载 include `{path}` 失败: {err}",
        },
        "VST043" => match loc {
            DiagLocale::Es => "{detail}",
            DiagLocale::En => "{detail}",
            DiagLocale::Ja => "{detail}",
            DiagLocale::De => "{detail}",
            DiagLocale::Zh => "{detail}",
        },
        "VST050" => match loc {
            // Matches writer rules: bare number/string alone is invalid; need var / not / and / or / compare.
            DiagLocale::Es => {
                "La condición de \"if\" debe ser una variable, not/and/or, o una comparación (no un número o texto sueltos)."
            }
            DiagLocale::En => {
                "The \"if\" condition must be a variable, not/and/or, or a comparison (not a bare number or string)."
            }
            DiagLocale::Ja => {
                "\"if\" の条件は変数、not/and/or、または比較である必要があります（裸の数値や文字列は不可）。"
            }
            DiagLocale::De => {
                "Die \"if\"-Bedingung muss eine Variable, not/and/or oder ein Vergleich sein (keine bloße Zahl/Zeichenkette)."
            }
            DiagLocale::Zh => {
                "\"if\" 条件必须是变量、not/and/or 或比较（不能是单独的数字或字符串）。"
            }
        },
        "VST051" => match loc {
            DiagLocale::Es => {
                "No se pudo resolver un nombre interno. Detalle técnico: {detail}"
            }
            DiagLocale::En => {
                "Could not resolve an internal name. Technical detail: {detail}"
            }
            DiagLocale::Ja => {
                "内部名を解決できませんでした。技術詳細: {detail}"
            }
            DiagLocale::De => {
                "Interner Name konnte nicht aufgelöst werden. Technik: {detail}"
            }
            DiagLocale::Zh => "无法解析内部名称。技术细节: {detail}",
        },
        "VST060" => match loc {
            DiagLocale::Es => "{detail}",
            DiagLocale::En => "{detail}",
            DiagLocale::Ja => "{detail}",
            DiagLocale::De => "{detail}",
            DiagLocale::Zh => "{detail}",
        },
        "VST099" => match loc {
            DiagLocale::Es => "Error al preparar la historia. Detalle: {detail}",
            DiagLocale::En => "Error preparing the story. Detail: {detail}",
            DiagLocale::Ja => "ストーリー準備エラー。詳細: {detail}",
            DiagLocale::De => "Fehler beim Vorbereiten der Geschichte. Detail: {detail}",
            DiagLocale::Zh => "准备故事时出错。详情: {detail}",
        },
        other => {
            // Unknown codes: keep code visible; message is English-neutral fallback.
            let _ = (loc, other);
            "Velvet Story diagnostic."
        }
    }
}

fn template_suggestion(loc: DiagLocale, code: &str) -> Option<&'static str> {
    match code {
        "VST001" => Some(match loc {
            DiagLocale::Es => "Usa solo espacios (4 por nivel).",
            DiagLocale::En => "Use only spaces (4 per level).",
            DiagLocale::Ja => "スペースのみを使ってください（レベルごとに 4）。",
            DiagLocale::De => "Nur Leerzeichen verwenden (4 pro Ebene).",
            DiagLocale::Zh => "只使用空格（每级 4 个）。",
        }),
        "VST020" => Some(match loc {
            DiagLocale::Es => {
                "Renombra una de las dos escenas. La primera estaba cerca de la línea {line}."
            }
            DiagLocale::En => {
                "Rename one of the two scenes. The first was near line {line}."
            }
            DiagLocale::Ja => {
                "どちらかのシーン名を変えてください。最初は {line} 行付近にあります。"
            }
            DiagLocale::De => {
                "Benenne eine der beiden Szenen um. Die erste lag bei Zeile {line}."
            }
            DiagLocale::Zh => {
                "请重命名其中一个场景。第一个约在第 {line} 行。"
            }
        }),
        "VST021" => Some("set {name} = 0"),
        "VST022" => Some("{req}: …"),
        "VST027" => Some(match loc {
            DiagLocale::Es => "Revisa el nombre o crea la escena con `scene …`.",
            DiagLocale::En => "Check the name or create the scene with `scene …`.",
            DiagLocale::Ja => "名前を確認するか `scene …` でシーンを作成してください。",
            DiagLocale::De => "Namen prüfen oder Szene mit `scene …` anlegen.",
            DiagLocale::Zh => "检查名称，或用 `scene …` 创建场景。",
        }),
        "VST030" | "VST050" => Some(match loc {
            DiagLocale::Es => {
                "if affection >= 3:\n# o una variable booleana: if has_key:"
            }
            DiagLocale::En => {
                "if affection >= 3:\n# or a boolean variable: if has_key:"
            }
            DiagLocale::Ja => {
                "if affection >= 3:\n# または真偽変数: if has_key:"
            }
            DiagLocale::De => {
                "if affection >= 3:\n# oder boolesche Variable: if has_key:"
            }
            DiagLocale::Zh => {
                "if affection >= 3:\n# 或布尔变量: if has_key:"
            }
        }),
        "VST041" => Some(match loc {
            DiagLocale::Es => "Revisa la ruta relativa al archivo actual.",
            DiagLocale::En => "Check the path relative to the current file.",
            DiagLocale::Ja => "現在のファイルからの相対パスを確認してください。",
            DiagLocale::De => "Relativen Pfad zur aktuellen Datei prüfen.",
            DiagLocale::Zh => "检查相对于当前文件的路径。",
        }),
        _ => None,
    }
}

fn template_hint(loc: DiagLocale, kind: &str) -> &'static str {
    match (loc, kind) {
        (DiagLocale::Es, "str") => "Actualmente estás usando el texto \"{s}\".",
        (DiagLocale::En, "str") => "You are currently using the text \"{s}\".",
        (DiagLocale::Ja, "str") => "現在テキスト \"{s}\" を使っています。",
        (DiagLocale::De, "str") => "Sie verwenden derzeit den Text \"{s}\".",
        (DiagLocale::Zh, "str") => "当前使用的是文本 \"{s}\"。",

        (DiagLocale::Es, "int") => "Actualmente estás usando el número {n} sin comparar.",
        (DiagLocale::En, "int") => "You are currently using the number {n} without comparing.",
        (DiagLocale::Ja, "int") => "現在 数値 {n} を比較なしで使っています。",
        (DiagLocale::De, "int") => "Sie verwenden die Zahl {n} ohne Vergleich.",
        (DiagLocale::Zh, "int") => "当前使用数字 {n} 但未进行比较。",

        (DiagLocale::Es, "float") => "Actualmente estás usando el número {s} sin comparar.",
        (DiagLocale::En, "float") => "You are currently using the number {s} without comparing.",
        (DiagLocale::Ja, "float") => "現在 数値 {s} を比較なしで使っています。",
        (DiagLocale::De, "float") => "Sie verwenden die Zahl {s} ohne Vergleich.",
        (DiagLocale::Zh, "float") => "当前使用数字 {s} 但未进行比较。",

        (DiagLocale::Es, _) => "La condición no se puede interpretar como verdadero o falso.",
        (DiagLocale::En, _) => "The condition cannot be interpreted as true or false.",
        (DiagLocale::Ja, _) => "条件を真または偽として解釈できません。",
        (DiagLocale::De, _) => "Die Bedingung lässt sich nicht als wahr/falsch deuten.",
        (DiagLocale::Zh, _) => "该条件无法解释为真或假。",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_aliases() {
        assert_eq!(DiagLocale::parse("ES").unwrap(), DiagLocale::Es);
        assert_eq!(DiagLocale::parse("english").unwrap(), DiagLocale::En);
        assert_eq!(DiagLocale::parse("ja").unwrap(), DiagLocale::Ja);
        assert_eq!(DiagLocale::parse("de").unwrap(), DiagLocale::De);
        assert_eq!(DiagLocale::parse("zh-CN").unwrap(), DiagLocale::Zh);
        assert!(DiagLocale::parse("xx").is_err());
    }

    #[test]
    fn messages_differ_by_locale() {
        let es = diag_message_for(DiagLocale::Es, "VST027", &[("target", "x")]);
        let en = diag_message_for(DiagLocale::En, "VST027", &[("target", "x")]);
        let ja = diag_message_for(DiagLocale::Ja, "VST027", &[("target", "x")]);
        assert_ne!(es, en);
        assert_ne!(en, ja);
        assert!(es.contains("escena") || es.contains("etiqueta"));
        assert!(en.to_ascii_lowercase().contains("scene") || en.contains("label"));
    }

    #[test]
    fn scoped_locale_overrides_default_without_mutating_default() {
        set_diag_locale(DiagLocale::Es);
        let es_default = diag_message("VST027", &[("target", "x")]);
        let en_scoped = with_diag_locale(DiagLocale::En, || {
            diag_message("VST027", &[("target", "x")])
        });
        // Default restored after scope
        let es_after = diag_message("VST027", &[("target", "x")]);
        assert_ne!(es_default, en_scoped);
        assert_eq!(es_default, es_after);
        assert_eq!(default_diag_locale(), DiagLocale::Es);
        assert!(en_scoped.to_ascii_lowercase().contains("scene") || en_scoped.contains("label"));
    }

    #[test]
    fn concurrent_threads_isolated_locales() {
        use std::sync::mpsc;
        use std::thread;

        let (tx, rx) = mpsc::channel();
        let pairs = [
            (DiagLocale::En, "scene"),
            (DiagLocale::Es, "escena"),
            (DiagLocale::Ja, "シーン"),
            (DiagLocale::De, "Szene"),
            (DiagLocale::Zh, "场景"),
        ];
        let mut handles = Vec::new();
        for (loc, cue) in pairs {
            let tx = tx.clone();
            handles.push(thread::spawn(move || {
                // Intentional process-default noise — must not bleed into scoped checks.
                set_diag_locale(DiagLocale::Es);
                let msg = with_diag_locale(loc, || {
                    // re-set default while scoped; scope must win
                    set_diag_locale(DiagLocale::Zh);
                    diag_message_for(diag_locale(), "VST027", &[("target", "missing")])
                });
                tx.send((loc.code(), msg, cue)).unwrap();
            }));
        }
        drop(tx);
        for h in handles {
            h.join().unwrap();
        }
        set_diag_locale(DiagLocale::Es);
        let mut got = Vec::new();
        while let Ok((code, msg, cue)) = rx.recv() {
            assert!(
                msg.contains(cue) || msg.to_ascii_lowercase().contains(cue),
                "locale {code}: expected cue `{cue}` in `{msg}`"
            );
            // Spanish-only bleed into English scope
            if code == "en" {
                assert!(
                    !msg.contains("escena") && !msg.contains("etiqueta"),
                    "English context bled Spanish: {msg}"
                );
            }
            if code == "es" {
                assert!(
                    !msg.to_ascii_lowercase().contains("does not exist"),
                    "Spanish context bled English: {msg}"
                );
            }
            got.push(code.to_string());
        }
        assert_eq!(got.len(), 5);
    }
}
