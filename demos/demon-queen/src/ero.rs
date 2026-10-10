//! The Queen's ero record: how many times she has done what. Kept in the story's variables (so
//! it is saved with the game and reaches the visual novel) and shown on the pause menu's status
//! page. Each scene counts once, when its card closes.
use std::collections::BTreeMap;

/// Alone in her bed (the castle's bed scene).
pub(crate) const MASTURBACIONES: &str = "ero_masturbaciones";
/// Times she went all the way, alone or not.
pub(crate) const ORGASMOS: &str = "ero_orgasmos";
/// Scenes with a girl of the brothel.
pub(crate) const ENCUENTROS: &str = "ero_encuentros";
/// Those under Rosalía's elixir.
pub(crate) const FUTA: &str = "ero_futa";
/// Flasks of the elixir drunk.
pub(crate) const ELIXIR: &str = "ero_elixir";
/// Times the hypnotist's ring took her.
pub(crate) const HIPNOTIZADA: &str = "ero_hipnotizada";
/// Set (1) the day she is no longer a virgin; nothing sets it yet.
#[allow(dead_code)] // read by the host's status page
pub(crate) const VIRGINIDAD_PERDIDA: &str = "ero_virginidad_perdida";

/// The tally of scenes with `girl`.
pub(crate) fn with(girl: &str) -> String {
    format!("ero_con_{girl}")
}

/// One more of `key`.
pub(crate) fn tally(values: &mut BTreeMap<String, i32>, key: &str) {
    *values.entry(key.to_owned()).or_insert(0) += 1;
}

/// A scene with `girl` closed; `climax` when it went all the way, `futa` under the elixir.
pub(crate) fn encounter(values: &mut BTreeMap<String, i32>, girl: &str, futa: bool, climax: bool) {
    tally(values, ENCUENTROS);
    tally(values, &with(girl));
    if futa {
        tally(values, FUTA);
    }
    if climax {
        tally(values, ORGASMOS);
    }
}

/// She touched herself; `climax` when she went all the way.
pub(crate) fn alone(values: &mut BTreeMap<String, i32>, climax: bool) {
    tally(values, MASTURBACIONES);
    if climax {
        tally(values, ORGASMOS);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encounters_and_solo_scenes_count_where_the_status_page_reads_them() {
        let mut v = BTreeMap::new();
        encounter(&mut v, "nina", false, true);
        encounter(&mut v, "nina", true, false);
        encounter(&mut v, "alba", false, true);
        alone(&mut v, false);
        alone(&mut v, true);
        assert_eq!(v[ENCUENTROS], 3);
        assert_eq!(v[&with("nina")], 2);
        assert_eq!(v[&with("alba")], 1);
        assert_eq!(v[FUTA], 1);
        assert_eq!(v[ORGASMOS], 3);
        assert_eq!(v[MASTURBACIONES], 2);
        assert!(!v.contains_key(VIRGINIDAD_PERDIDA), "nothing takes it yet");
    }
}
