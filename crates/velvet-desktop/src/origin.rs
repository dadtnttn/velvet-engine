//! Origins: the `scheme://host[:port]` part of an address, and whether an address is on one.
//!
//! The bridge is only for the game's own origin (or the dev server's), so every navigation and the
//! `--dev` address are checked here. Plain string logic, with no webview.

/// The origin (`scheme://host[:port]`) of an address, or `None` if it has no scheme or host.
pub fn origin_of(url: &str) -> Option<String> {
    let (scheme, rest) = url.split_once("://")?;
    let scheme_ok = !scheme.is_empty()
        && scheme
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'-' | b'.'));
    let authority = rest.split(['/', '?', '#']).next()?;
    if !scheme_ok || authority.is_empty() {
        return None;
    }
    Some(format!("{scheme}://{authority}"))
}

/// Whether `url` is on `origin` (the same scheme, host and port), and not on a longer host
/// that merely starts with the same text.
pub fn is_same_origin(origin: &str, url: &str) -> bool {
    url.strip_prefix(origin)
        .is_some_and(|rest| rest.is_empty() || rest.starts_with(['/', '?', '#']))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn origins_are_read_from_addresses() {
        assert_eq!(
            origin_of("http://localhost:5173/"),
            Some("http://localhost:5173".into())
        );
        assert_eq!(
            origin_of("https://example.com/a?b#c"),
            Some("https://example.com".into())
        );
        assert_eq!(
            origin_of("http://[::1]:8080/x"),
            Some("http://[::1]:8080".into())
        );
        assert_eq!(origin_of("http://"), None);
        assert_eq!(origin_of("http:///path"), None);
        assert_eq!(origin_of("localhost:5173"), None);
        assert_eq!(origin_of("://host"), None);
        assert_eq!(origin_of("bad scheme://host"), None);
    }

    #[test]
    fn same_origin_checks_the_whole_host() {
        assert!(is_same_origin(
            "http://velvet.game",
            "http://velvet.game/index.html"
        ));
        assert!(is_same_origin("http://velvet.game", "http://velvet.game"));
        assert!(is_same_origin(
            "http://velvet.game",
            "http://velvet.game?x=1"
        ));
        assert!(is_same_origin("velvet://game", "velvet://game/main.js"));
        assert!(!is_same_origin(
            "http://velvet.game",
            "http://velvet.game.evil.example/"
        ));
        assert!(!is_same_origin(
            "http://velvet.game",
            "http://velvet.gamex/"
        ));
        assert!(!is_same_origin(
            "http://localhost:5173",
            "http://localhost:51730/"
        ));
        assert!(!is_same_origin(
            "http://velvet.game",
            "https://velvet.game/"
        ));
        assert!(!is_same_origin("velvet://game", "https://example.com/"));
    }
}
