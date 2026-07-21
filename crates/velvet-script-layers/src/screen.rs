//! Typed declarative screen blueprints compiled from Velvet Script AST.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
use thiserror::Error;
use velvet_script_ast::{Expr, Item, ScreenProperty, Severity, SourceLoc};
use velvet_script_parser::parse_file;

/// Runtime-neutral description of one declarative Velvet Script screen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScreenBlueprint {
    /// Stable screen name from `screen name { ... }`.
    pub name: String,
    /// VCSS class applied to the screen root.
    pub class: String,
    /// Primary screen heading.
    pub title: String,
    /// Secondary heading or short supporting copy.
    pub subtitle: String,
    /// Small label displayed above the title.
    pub eyebrow: String,
    /// Supporting copy displayed after the action list.
    pub footer: String,
    /// Buttons in authored order.
    pub buttons: Vec<ScreenButtonSpec>,
}

impl Default for ScreenBlueprint {
    fn default() -> Self {
        Self {
            name: String::new(),
            class: "screen".into(),
            title: String::new(),
            subtitle: String::new(),
            eyebrow: String::new(),
            footer: String::new(),
            buttons: Vec::new(),
        }
    }
}

/// Runtime-neutral description of one screen button.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScreenButtonSpec {
    /// Stable widget id used by actions and VCSS `#id` selectors.
    pub id: String,
    /// Visible action label. Required.
    pub label: String,
    /// Optional supporting text.
    pub description: String,
    /// Host action id emitted when activated. Required.
    pub action: String,
    /// Optional icon id.
    pub icon: String,
    /// VCSS class applied to the button.
    pub class: String,
    /// Optional keyboard/gamepad shortcut label.
    pub hotkey: String,
    /// Optional badge text.
    pub badge: String,
    /// Whether the action starts enabled.
    pub enabled: bool,
}

impl Default for ScreenButtonSpec {
    fn default() -> Self {
        Self {
            id: String::new(),
            label: String::new(),
            description: String::new(),
            action: String::new(),
            icon: String::new(),
            class: "button".into(),
            hotkey: String::new(),
            badge: String::new(),
            enabled: true,
        }
    }
}

/// Declarative screen compilation failure.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ScreenCompileError {
    /// Lexer or unrecoverable parser failure.
    #[error("screen parse error: {message}")]
    Parse {
        /// Parser message.
        message: String,
    },
    /// Recoverable parser diagnostic promoted to an error.
    #[error("{location}: {message}")]
    Syntax {
        /// Source location.
        location: String,
        /// Diagnostic message.
        message: String,
    },
    /// Two screens in one source used the same name.
    #[error("{location}: duplicate screen `{name}`")]
    DuplicateScreen {
        /// Repeated screen name.
        name: String,
        /// Source location of the duplicate.
        location: String,
    },
    /// Two buttons in one screen used the same id.
    #[error("{location}: duplicate button id `{id}` in screen `{screen}`")]
    DuplicateButton {
        /// Screen name.
        screen: String,
        /// Repeated button id.
        id: String,
        /// Source location of the duplicate.
        location: String,
    },
    /// A property was authored more than once on the same object.
    #[error("{location}: duplicate property `{property}` on {owner}")]
    DuplicateProperty {
        /// Human-readable screen or button owner.
        owner: String,
        /// Repeated property name.
        property: String,
        /// Source location of the duplicate.
        location: String,
    },
    /// A property is not part of the typed blueprint surface.
    #[error("{location}: unknown property `{property}` on {owner}")]
    UnknownProperty {
        /// Human-readable screen or button owner.
        owner: String,
        /// Unknown property name.
        property: String,
        /// Source location.
        location: String,
    },
    /// A required button property is missing or empty.
    #[error("{location}: {owner} requires non-empty `{property}`")]
    MissingProperty {
        /// Human-readable button owner.
        owner: String,
        /// Required property name.
        property: String,
        /// Source location of the button.
        location: String,
    },
    /// A property expression was not a supported literal of the expected type.
    #[error("{location}: `{property}` on {owner} must be {expected}")]
    InvalidLiteral {
        /// Human-readable screen or button owner.
        owner: String,
        /// Property name.
        property: String,
        /// Expected literal kind.
        expected: String,
        /// Source location of the value.
        location: String,
    },
}

/// Parse and validate every declarative `screen` item in Velvet Script source.
///
/// Non-screen items are ignored. Screen names must be unique within the source,
/// button ids must be unique within their screen, and `label` plus `action` are
/// required for every button.
pub fn parse_screen_source(
    source: &str,
    file: Option<&str>,
) -> Result<Vec<ScreenBlueprint>, ScreenCompileError> {
    let parsed = parse_file(source, file).map_err(|error| ScreenCompileError::Parse {
        message: error.to_string(),
    })?;

    if let Some(diagnostic) = parsed
        .module
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.severity == Severity::Error)
    {
        return Err(ScreenCompileError::Syntax {
            location: diagnostic.loc.display(),
            message: diagnostic.message.clone(),
        });
    }

    let mut names = HashSet::new();
    let mut screens = Vec::new();
    for item in &parsed.module.items {
        let Item::Screen {
            name,
            properties,
            buttons,
            loc,
        } = item
        else {
            continue;
        };

        if !names.insert(name.as_str()) {
            return Err(ScreenCompileError::DuplicateScreen {
                name: name.clone(),
                location: loc.display(),
            });
        }

        let owner = format!("screen `{name}`");
        let props = property_map(
            properties,
            &owner,
            &["class", "title", "subtitle", "eyebrow", "footer"],
        )?;
        let mut blueprint = ScreenBlueprint {
            name: name.clone(),
            class: optional_string(&props, "class", "screen", &owner)?,
            title: optional_string(&props, "title", "", &owner)?,
            subtitle: optional_string(&props, "subtitle", "", &owner)?,
            eyebrow: optional_string(&props, "eyebrow", "", &owner)?,
            footer: optional_string(&props, "footer", "", &owner)?,
            buttons: Vec::with_capacity(buttons.len()),
        };

        let mut button_ids = HashSet::new();
        for button in buttons {
            if !button_ids.insert(button.id.as_str()) {
                return Err(ScreenCompileError::DuplicateButton {
                    screen: name.clone(),
                    id: button.id.clone(),
                    location: button.loc.display(),
                });
            }

            let button_owner = format!("button `{}`", button.id);
            let button_props = property_map(
                &button.properties,
                &button_owner,
                &[
                    "label",
                    "description",
                    "action",
                    "icon",
                    "class",
                    "hotkey",
                    "badge",
                    "enabled",
                ],
            )?;
            let label = required_string(&button_props, "label", &button_owner, &button.loc)?;
            let action = required_string(&button_props, "action", &button_owner, &button.loc)?;
            blueprint.buttons.push(ScreenButtonSpec {
                id: button.id.clone(),
                label,
                description: optional_string(&button_props, "description", "", &button_owner)?,
                action,
                icon: optional_string(&button_props, "icon", "", &button_owner)?,
                class: optional_string(&button_props, "class", "button", &button_owner)?,
                hotkey: optional_string_or_int(&button_props, "hotkey", &button_owner)?,
                badge: optional_string_or_int(&button_props, "badge", &button_owner)?,
                enabled: optional_bool(&button_props, "enabled", true, &button_owner)?,
            });
        }

        screens.push(blueprint);
    }
    Ok(screens)
}

fn property_map<'a>(
    properties: &'a [ScreenProperty],
    owner: &str,
    known: &[&str],
) -> Result<HashMap<&'a str, &'a ScreenProperty>, ScreenCompileError> {
    let mut map = HashMap::with_capacity(properties.len());
    for property in properties {
        if !known.contains(&property.name.as_str()) {
            return Err(ScreenCompileError::UnknownProperty {
                owner: owner.into(),
                property: property.name.clone(),
                location: property.loc.display(),
            });
        }
        if map.insert(property.name.as_str(), property).is_some() {
            return Err(ScreenCompileError::DuplicateProperty {
                owner: owner.into(),
                property: property.name.clone(),
                location: property.loc.display(),
            });
        }
    }
    Ok(map)
}

fn required_string(
    properties: &HashMap<&str, &ScreenProperty>,
    name: &str,
    owner: &str,
    owner_loc: &SourceLoc,
) -> Result<String, ScreenCompileError> {
    let Some(property) = properties.get(name) else {
        return Err(ScreenCompileError::MissingProperty {
            owner: owner.into(),
            property: name.into(),
            location: owner_loc.display(),
        });
    };
    let value = string_literal(&property.value, name, owner)?;
    if value.trim().is_empty() {
        return Err(ScreenCompileError::MissingProperty {
            owner: owner.into(),
            property: name.into(),
            location: property.value.loc().display(),
        });
    }
    Ok(value)
}

fn optional_string(
    properties: &HashMap<&str, &ScreenProperty>,
    name: &str,
    default: &str,
    owner: &str,
) -> Result<String, ScreenCompileError> {
    properties
        .get(name)
        .map(|property| string_literal(&property.value, name, owner))
        .transpose()
        .map(|value| value.unwrap_or_else(|| default.into()))
}

fn optional_string_or_int(
    properties: &HashMap<&str, &ScreenProperty>,
    name: &str,
    owner: &str,
) -> Result<String, ScreenCompileError> {
    let Some(property) = properties.get(name) else {
        return Ok(String::new());
    };
    match &property.value {
        Expr::String { value, .. } => Ok(value.clone()),
        Expr::Int { value, .. } => Ok(value.to_string()),
        other => Err(invalid_literal(
            other,
            name,
            owner,
            "a string or integer literal",
        )),
    }
}

fn optional_bool(
    properties: &HashMap<&str, &ScreenProperty>,
    name: &str,
    default: bool,
    owner: &str,
) -> Result<bool, ScreenCompileError> {
    let Some(property) = properties.get(name) else {
        return Ok(default);
    };
    match &property.value {
        Expr::Bool { value, .. } => Ok(*value),
        other => Err(invalid_literal(other, name, owner, "a boolean literal")),
    }
}

fn string_literal(value: &Expr, property: &str, owner: &str) -> Result<String, ScreenCompileError> {
    match value {
        Expr::String { value, .. } => Ok(value.clone()),
        other => Err(invalid_literal(other, property, owner, "a string literal")),
    }
}

fn invalid_literal(
    value: &Expr,
    property: &str,
    owner: &str,
    expected: &str,
) -> ScreenCompileError {
    ScreenCompileError::InvalidLiteral {
        owner: owner.into(),
        property: property.into(),
        expected: expected.into(),
        location: value.loc().display(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compiles_all_public_fields_in_author_order() {
        let source = r#"
screen main_menu {
    class: "casino-menu"
    eyebrow: "THE HOUSE AWAITS"
    title: "VELVET ARCANA"
    subtitle: "NIGHTFALL CASINO"
    footer: "FORTUNE FAVORS THE BOLD"

    button start {
        label: "START RUN"
        description: "Deal the first hand"
        action: "start_run"
        icon: "play"
        class: "menu-action primary"
        hotkey: 1
        badge: "NEW"
        enabled: true
    }
    button quit {
        label: "QUIT"
        action: "quit"
        enabled: false
    }
}
"#;
        let screens = parse_screen_source(source, Some("menu.vel")).unwrap();
        assert_eq!(screens.len(), 1);
        let screen = &screens[0];
        assert_eq!(screen.name, "main_menu");
        assert_eq!(screen.class, "casino-menu");
        assert_eq!(screen.eyebrow, "THE HOUSE AWAITS");
        assert_eq!(screen.title, "VELVET ARCANA");
        assert_eq!(screen.subtitle, "NIGHTFALL CASINO");
        assert_eq!(screen.footer, "FORTUNE FAVORS THE BOLD");
        assert_eq!(screen.buttons.len(), 2);
        assert_eq!(screen.buttons[0].id, "start");
        assert_eq!(screen.buttons[0].hotkey, "1");
        assert_eq!(screen.buttons[0].badge, "NEW");
        assert!(!screen.buttons[1].enabled);
    }

    #[test]
    fn fills_safe_defaults_and_ignores_non_screen_items() {
        let source = r#"
function helper() { return 1 }
screen pause {
    button resume { label: "RESUME"; action: "resume"; }
}
scene start { "ready" }
"#;
        let screens = parse_screen_source(source, None).unwrap();
        let screen = &screens[0];
        assert_eq!(screen.class, "screen");
        assert!(screen.title.is_empty());
        assert!(screen.subtitle.is_empty());
        assert!(screen.eyebrow.is_empty());
        assert!(screen.footer.is_empty());
        let button = &screen.buttons[0];
        assert_eq!(button.class, "button");
        assert!(button.description.is_empty());
        assert!(button.icon.is_empty());
        assert!(button.hotkey.is_empty());
        assert!(button.badge.is_empty());
        assert!(button.enabled);
    }

    #[test]
    fn rejects_duplicate_screen_names() {
        let error =
            parse_screen_source("screen menu {}\nscreen menu {}", Some("dup.vel")).unwrap_err();
        assert!(matches!(
            error,
            ScreenCompileError::DuplicateScreen { ref name, .. } if name == "menu"
        ));
        assert!(error.to_string().contains("dup.vel:2:1"));
    }

    #[test]
    fn rejects_duplicate_button_ids() {
        let source = r#"
screen menu {
    button play { label: "PLAY"; action: "play"; }
    button play { label: "AGAIN"; action: "again"; }
}
"#;
        let error = parse_screen_source(source, None).unwrap_err();
        assert!(matches!(
            error,
            ScreenCompileError::DuplicateButton { ref id, .. } if id == "play"
        ));
    }

    #[test]
    fn requires_non_empty_label_and_action() {
        let missing_label =
            parse_screen_source("screen menu { button play { action: \"play\" } }", None)
                .unwrap_err();
        assert!(matches!(
            missing_label,
            ScreenCompileError::MissingProperty { ref property, .. } if property == "label"
        ));

        let empty_action = parse_screen_source(
            "screen menu { button play { label: \"PLAY\" action: \"\" } }",
            None,
        )
        .unwrap_err();
        assert!(matches!(
            empty_action,
            ScreenCompileError::MissingProperty { ref property, .. } if property == "action"
        ));
    }

    #[test]
    fn rejects_wrong_literal_kinds_and_expressions() {
        let wrong_bool = parse_screen_source(
            "screen menu { button play { label: \"PLAY\" action: \"play\" enabled: 1 } }",
            None,
        )
        .unwrap_err();
        assert!(matches!(
            wrong_bool,
            ScreenCompileError::InvalidLiteral { ref property, .. } if property == "enabled"
        ));

        let expression = parse_screen_source(
            "screen menu { title: make_title() button play { label: \"PLAY\" action: \"play\" } }",
            None,
        )
        .unwrap_err();
        assert!(matches!(
            expression,
            ScreenCompileError::InvalidLiteral { ref property, .. } if property == "title"
        ));
    }

    #[test]
    fn rejects_unknown_and_duplicate_properties() {
        let unknown = parse_screen_source("screen menu { layout: \"grid\" }", None).unwrap_err();
        assert!(matches!(
            unknown,
            ScreenCompileError::UnknownProperty { ref property, .. } if property == "layout"
        ));

        let duplicate =
            parse_screen_source("screen menu { title: \"A\" title: \"B\" }", None).unwrap_err();
        assert!(matches!(
            duplicate,
            ScreenCompileError::DuplicateProperty { ref property, .. } if property == "title"
        ));
    }
}
