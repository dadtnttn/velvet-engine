//! Opening links in the player's own programs: the `shell.openExternal(url)` call.
//!
//! [`validate_url`] is the safety rule. Only `https:`, `http:` and `mailto:` addresses are allowed,
//! made of plain ASCII with no spaces or quotes, so that nothing can break out of the command that
//! the system runs, and no user information (`user@host`) that could disguise the real host.
//! [`open_external`] checks the address first. On Windows it then opens the address with
//! `ShellExecuteW`. Other systems answer that opening links is not supported yet.

use thiserror::Error;

/// The longest address that may be opened, in bytes.
pub const MAX_URL_BYTES: usize = 2048;

/// Why a link was not opened.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ShellError {
    /// The address is not one that may be opened. The text says which rule it broke.
    #[error("the link is refused: {0}")]
    Refused(&'static str),
    /// This platform cannot open links yet.
    #[error("opening links is not supported on this platform yet")]
    Unsupported,
    /// The system could not open the link. The number is the `ShellExecute` error code.
    #[error("the system could not open the link (ShellExecute code {0})")]
    OpenFailed(isize),
}

impl ShellError {
    /// A short machine-readable code, which the game receives as `error.code`.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Refused(_) => "invalid_url",
            Self::Unsupported => "unsupported",
            Self::OpenFailed(_) => "open_failed",
        }
    }
}

/// Checks that `url` may be opened. Nothing is opened here.
pub fn validate_url(url: &str) -> Result<(), ShellError> {
    if url.is_empty() {
        return Err(ShellError::Refused("the address is empty"));
    }
    if url.len() > MAX_URL_BYTES {
        return Err(ShellError::Refused("the address is longer than 2048 bytes"));
    }
    if !url.bytes().all(is_plain_url_byte) {
        return Err(ShellError::Refused(
            "the address has a space, a non-ASCII or control character, or one of the characters \
             \" < > \\ ^ ` { | }; percent-encode it instead",
        ));
    }
    // Only ASCII is left, so lower-casing cannot change the length or the boundaries.
    let lower = url.to_ascii_lowercase();
    if let Some(rest) = lower
        .strip_prefix("https://")
        .or_else(|| lower.strip_prefix("http://"))
    {
        let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
        let host = authority.split(':').next().unwrap_or("");
        if host.is_empty() {
            return Err(ShellError::Refused("the address has no host name"));
        }
        if authority.contains('@') {
            return Err(ShellError::Refused(
                "the address has user information before the host name",
            ));
        }
        return Ok(());
    }
    if let Some(rest) = lower.strip_prefix("mailto:") {
        if rest.is_empty() {
            return Err(ShellError::Refused("the mail link has no address"));
        }
        return Ok(());
    }
    Err(ShellError::Refused(
        "the address must start with https://, http:// or mailto:",
    ))
}

/// Opens `url` in the player's browser or mail program, after [`validate_url`] accepts it.
///
/// Refused addresses are reported before anything is opened. On systems other than Windows, and in
/// builds without the `window` feature, a valid address gets [`ShellError::Unsupported`].
pub fn open_external(url: &str) -> Result<(), ShellError> {
    validate_url(url)?;
    open_validated(url)
}

/// A byte that may appear in an address to be opened: printable ASCII, except the characters that
/// are unsafe in a URL and in a command line.
fn is_plain_url_byte(byte: u8) -> bool {
    byte.is_ascii_graphic()
        && !matches!(
            byte,
            b'"' | b'<' | b'>' | b'\\' | b'^' | b'`' | b'{' | b'|' | b'}'
        )
}

#[cfg(all(windows, feature = "window"))]
fn open_validated(url: &str) -> Result<(), ShellError> {
    use windows_sys::Win32::UI::Shell::ShellExecuteW;
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    // Both strings are wide and end with a zero, as the Windows call expects.
    let verb: Vec<u16> = "open".encode_utf16().chain(std::iter::once(0)).collect();
    let target: Vec<u16> = url.encode_utf16().chain(std::iter::once(0)).collect();
    // SAFETY: both strings end with a zero and stay alive until the call returns. A null window
    // handle means no owner window, and null parameters and directory use the defaults.
    let result = unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            verb.as_ptr(),
            target.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            SW_SHOWNORMAL,
        )
    };
    // ShellExecute returns a value above 32 when it worked, and a small error code otherwise.
    let code = result as isize;
    if code > 32 {
        Ok(())
    } else {
        Err(ShellError::OpenFailed(code))
    }
}

#[cfg(not(all(windows, feature = "window")))]
fn open_validated(_url: &str) -> Result<(), ShellError> {
    Err(ShellError::Unsupported)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn refused(url: &str) -> bool {
        matches!(validate_url(url), Err(ShellError::Refused(_)))
    }

    #[test]
    fn web_and_mail_addresses_are_accepted() {
        for url in [
            "https://example.com/a?b=1#c",
            "http://localhost:5173/",
            "HTTPS://Example.com",
            "https://[::1]:8080/x",
            "mailto:someone@example.com?subject=Hi",
        ] {
            assert!(validate_url(url).is_ok(), "`{url}` should be accepted");
        }
    }

    #[test]
    fn other_schemes_and_broken_addresses_are_refused() {
        for url in [
            "",
            "javascript:alert(1)",
            "file:///C:/Windows/win.ini",
            "ftp://example.com",
            "data:text/html,hi",
            "http:example.com",
            "https:/example.com",
            "https://",
            "https:///path",
            "https://:443/",
            "mailto:",
            "example.com",
        ] {
            assert!(refused(url), "`{url}` should be refused");
        }
    }

    #[test]
    fn characters_that_can_break_out_of_the_command_are_refused() {
        for url in [
            "https://a b.example",
            "https://a\"b.example",
            "https://a<b.example",
            "https://example.com/\\x",
            "https://example.com/`x`",
            "https://example.com/|x",
            "https://example.com/\n",
            "https://example.com/\u{7f}",
            "https://ñ.example",
            " https://example.com",
        ] {
            assert!(refused(url), "`{url}` should be refused");
        }
    }

    #[test]
    fn user_information_before_the_host_is_refused() {
        assert!(refused("https://example.com@evil.example/"));
        assert!(refused("https://user:pass@example.com/"));
    }

    #[test]
    fn the_length_limit_is_2048_bytes() {
        let longest = format!("https://example.com/{}", "a".repeat(MAX_URL_BYTES - 20));
        assert_eq!(longest.len(), MAX_URL_BYTES);
        assert!(
            validate_url(&longest).is_ok(),
            "exactly 2048 bytes is allowed"
        );
        let too_long = format!("{longest}a");
        assert!(refused(&too_long), "2049 bytes is refused");
    }

    #[test]
    fn refused_links_never_reach_the_system() {
        // Refusals come before any call to the system, so these are safe to run anywhere.
        assert!(matches!(
            open_external("javascript:alert(1)"),
            Err(ShellError::Refused(_))
        ));
        assert!(matches!(
            open_external("https://a b.example"),
            Err(ShellError::Refused(_))
        ));
    }

    #[test]
    fn the_codes_are_stable() {
        assert_eq!(ShellError::Refused("x").code(), "invalid_url");
        assert_eq!(ShellError::Unsupported.code(), "unsupported");
        assert_eq!(ShellError::OpenFailed(2).code(), "open_failed");
    }

    /// Only for platforms where the call cannot open anything: a valid link is reported as
    /// unsupported, so the test never starts a browser.
    #[cfg(not(all(windows, feature = "window")))]
    #[test]
    fn a_valid_link_is_unsupported_without_the_windows_window() {
        assert_eq!(
            open_external("https://example.com"),
            Err(ShellError::Unsupported)
        );
    }
}
