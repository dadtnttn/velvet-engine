//! The game's guiones (`data/guiones`) and portraits (`data/retratos.json`), read with velvet-guion:
//! who is who in them (the Queen is Gorou inside), where they are, and reading them once.
pub use velvet_guion::*;

use std::path::PathBuf;

/// Who is who: the names each character goes by in the guiones, and whose thoughts the bracketed
/// lines are (the Queen's).
pub fn cast() -> Cast {
    Cast::new()
        .alias("gorou", &["reina", "la_reina", "gorou", "queen"])
        .alias("hypnotist", &["mago", "hipnotizador", "viejo"])
        .alias("council_vex", &["vex"])
        .alias("council_zillah", &["zillah"])
        .thinker("gorou", "Reina")
}

/// Where the guiones are (`data/guiones`; the portraits' `retratos.json` is beside it).
pub fn folder() -> PathBuf {
    crate::exploration3d_state::data_dir().join("guiones")
}

/// The game's guiones (read once).
pub fn library() -> &'static Library {
    static LIB: std::sync::OnceLock<Library> = std::sync::OnceLock::new();
    LIB.get_or_init(|| {
        let lib = Library::load(&folder(), cast());
        for p in &lib.problems {
            eprintln!("[guion] {p}");
        }
        lib
    })
}

/// The game's portraits (read once).
pub fn portraits() -> &'static Portraits {
    static P: std::sync::OnceLock<Portraits> = std::sync::OnceLock::new();
    P.get_or_init(|| Portraits::load(&folder().with_file_name("retratos.json")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn the_queen_speaks_and_thinks_as_gorou() {
        let (g, problems) = parse(
            "**La Reina:** —Bien.\n(Ya.)\n**Mago:** —Hola.\n**Zillah:** —Majestad.\n",
            Path::new("t.md"),
            &cast(),
        );
        assert!(problems.is_empty(), "{problems:?}");
        let who: Vec<(&str, &str)> = g.scenes[0]
            .steps
            .iter()
            .map(|s| match s {
                Step::Line { who, name, .. } => (who.as_str(), name.as_str()),
                _ => ("", ""),
            })
            .collect();
        assert_eq!(
            who,
            [
                ("gorou", "La Reina"),
                ("gorou_thought", "Reina"),
                ("hypnotist", "Mago"),
                ("council_zillah", "Zillah")
            ]
        );
    }

    #[test]
    fn every_portrait_listed_is_there() {
        let portraits = Portraits::load(&folder().with_file_name("retratos.json"));
        assert!(
            !portraits.entries().is_empty(),
            "data/retratos.json lists portraits"
        );
        let assets = folder().with_file_name("assets");
        for (who, mood) in portraits.entries() {
            let p = portraits
                .find(&who, Some(&mood))
                .unwrap_or_else(|| panic!("{who} {mood}"));
            let file = assets.join(format!("{}.png", p.image));
            assert!(file.is_file(), "{who} {mood}: {}", file.display());
        }
        // Moods not listed fall back to the normal face; thoughts are the Queen's.
        assert_eq!(
            portraits.find("lira", Some("furiosa")),
            portraits.find("lira", None)
        );
        assert_eq!(
            portraits.find("gorou_thought", None),
            portraits.find("gorou", None)
        );
    }

    #[test]
    fn the_game_guiones_read_cleanly() {
        let lib = Library::load(&folder(), cast());
        assert!(!lib.guiones.is_empty());
        assert!(lib.problems.is_empty(), "{:#?}", lib.problems);
        let mut vars = Vars::default();
        for who in ["lira", "nemi"] {
            assert!(lib.entry(Some(who), None, &vars).is_some(), "{who}");
        }
        for place in [
            "trono",
            "tocador",
            "balcon",
            "mesa-guerra",
            "estudio",
            "sello",
            "hogar",
        ] {
            let at = lib
                .entry(None, Some(place), &vars)
                .unwrap_or_else(|| panic!("{place}"));
            // Every conversation ends (or hands over) within a few hundred steps, whatever is picked.
            let mut c = Conversation::start(&lib, at, &mut vars);
            for _ in 0..300 {
                match c.shown(&lib, &vars) {
                    Shown::Line { .. } => c.advance(&lib, &mut vars),
                    Shown::Choice { .. } => {
                        c.choose(&lib, &mut vars, 0);
                    }
                    _ => break,
                }
            }
            assert!(
                matches!(c.shown(&lib, &vars), Shown::End | Shown::Vn(_)),
                "{place}"
            );
        }
    }
}
