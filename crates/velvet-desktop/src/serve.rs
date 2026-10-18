//! Serving the game's files to the webview.
//!
//! The webview asks for paths such as `/main.js`. Each path is percent-decoded and checked, then
//! resolved under the game folder. A path that is not a plain relative path (`..`, a drive letter,
//! a backslash, a control character) is refused. So is a path that resolves outside the folder,
//! for example through a symbolic link. Refused and missing files get a 404. Files are read whole
//! and sent with a MIME type from a fixed table.

use std::borrow::Cow;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use wry::http::{header, HeaderName, HeaderValue, Method, Response, StatusCode};

/// Why a request path was not served from the game folder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// The path is not a plain relative path inside the game folder.
    Unsafe,
    /// No file is there, or the file resolves outside the game folder.
    NotFound,
}

/// Why a game folder cannot be served.
#[derive(Debug, thiserror::Error)]
pub enum SiteError {
    /// The folder cannot be opened.
    #[error("cannot open the game folder {}: {source}", path.display())]
    Root {
        /// The folder that failed.
        path: PathBuf,
        /// The underlying error.
        source: io::Error,
    },
    /// The entry page is not a file inside the folder.
    #[error("the entry page `{0}` is missing from the game folder")]
    MissingEntry(String),
}

/// A game folder that the webview can read from.
#[derive(Debug, Clone)]
pub struct Site {
    root: PathBuf,
    entry: String,
}

impl Site {
    /// Opens `root` and checks that the `entry` page exists inside it.
    pub fn new(root: &Path, entry: &str) -> Result<Self, SiteError> {
        let root_canonical = fs::canonicalize(root).map_err(|source| SiteError::Root {
            path: root.to_path_buf(),
            source,
        })?;
        let site = Self {
            root: root_canonical,
            entry: entry.to_string(),
        };
        site.resolve("/")
            .map_err(|_| SiteError::MissingEntry(entry.to_string()))?;
        Ok(site)
    }

    /// Maps a request path, as the webview sends it, to a file inside the game folder.
    ///
    /// `/` maps to the entry page, and a path ending in `/` maps to that folder's `index.html`.
    pub fn resolve(&self, url_path: &str) -> Result<PathBuf, Refusal> {
        let decoded = percent_decode(url_path).ok_or(Refusal::Unsafe)?;
        let (segments, wants_dir) = split_path(&decoded).ok_or(Refusal::Unsafe)?;
        let mut target = if segments.is_empty() && !wants_dir {
            self.root.join(&self.entry)
        } else {
            segments
                .iter()
                .fold(self.root.clone(), |path, segment| path.join(*segment))
        };
        if wants_dir {
            target.push("index.html");
        }
        let resolved = fs::canonicalize(&target).map_err(|_| Refusal::NotFound)?;
        if resolved.starts_with(&self.root) && resolved.is_file() {
            Ok(resolved)
        } else {
            Err(Refusal::NotFound)
        }
    }

    /// Builds the answer to one request.
    ///
    /// 200 with the file when it exists, 404 when it is missing or refused, and 405 for any
    /// method other than GET.
    pub fn respond(&self, method: &Method, url_path: &str) -> Response<Cow<'static, [u8]>> {
        if *method != Method::GET {
            let mut response = finish(
                StatusCode::METHOD_NOT_ALLOWED,
                "text/plain; charset=utf-8",
                text("Method not allowed"),
            );
            response
                .headers_mut()
                .insert(header::ALLOW, HeaderValue::from_static("GET"));
            return response;
        }
        match self.resolve(url_path) {
            Ok(path) => match fs::read(&path) {
                Ok(bytes) => finish(StatusCode::OK, mime_for(&path), Cow::Owned(bytes)),
                Err(_) => not_found(),
            },
            Err(_) => not_found(),
        }
    }
}

/// The MIME type of a file, from its extension (case-insensitive).
///
/// Unknown extensions, and files without one, are `application/octet-stream`.
pub fn mime_for(path: &Path) -> &'static str {
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match extension.as_str() {
        "html" => "text/html; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" => "application/json",
        "txt" => "text/plain; charset=utf-8",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "avif" => "image/avif",
        "svg" => "image/svg+xml",
        "wasm" => "application/wasm",
        "woff2" => "font/woff2",
        "ttf" => "font/ttf",
        "ogg" => "audio/ogg",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "glb" => "model/gltf-binary",
        "gltf" => "model/gltf+json",
        "bin" => "application/octet-stream",
        _ => "application/octet-stream",
    }
}

/// Decodes `%XX` escapes. Returns `None` for a bad escape or for bytes that are not UTF-8.
fn percent_decode(input: &str) -> Option<String> {
    let bytes = input.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while let Some(&byte) = bytes.get(index) {
        if byte == b'%' {
            let high = hex_digit(*bytes.get(index + 1)?)?;
            let low = hex_digit(*bytes.get(index + 2)?)?;
            decoded.push((high << 4) | low);
            index += 3;
        } else {
            decoded.push(byte);
            index += 1;
        }
    }
    String::from_utf8(decoded).ok()
}

fn hex_digit(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

/// Splits an absolute, decoded path into its segments.
///
/// Returns the segments and whether the path ended with `/`. Returns `None` for anything that is
/// not a plain relative path: a missing leading `/`, a backslash, a colon, a control character,
/// an empty segment, `.` or `..`.
fn split_path(decoded: &str) -> Option<(Vec<&str>, bool)> {
    let body = decoded.strip_prefix('/')?;
    if decoded
        .chars()
        .any(|character| character == '\\' || character == ':' || character.is_control())
    {
        return None;
    }
    if body.is_empty() {
        return Some((Vec::new(), false));
    }
    let (core, wants_dir) = match body.strip_suffix('/') {
        Some(core) => (core, true),
        None => (body, false),
    };
    let mut segments = Vec::new();
    for segment in core.split('/') {
        if segment.is_empty() || segment == "." || segment == ".." {
            return None;
        }
        segments.push(segment);
    }
    Some((segments, wants_dir))
}

fn finish(
    status: StatusCode,
    content_type: &'static str,
    body: Cow<'static, [u8]>,
) -> Response<Cow<'static, [u8]>> {
    let mut response = Response::new(body);
    *response.status_mut() = status;
    let headers = response.headers_mut();
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static(content_type));
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    headers.insert(
        HeaderName::from_static("x-content-type-options"),
        HeaderValue::from_static("nosniff"),
    );
    response
}

fn not_found() -> Response<Cow<'static, [u8]>> {
    finish(
        StatusCode::NOT_FOUND,
        "text/plain; charset=utf-8",
        text("Not found"),
    )
}

fn text(body: &'static str) -> Cow<'static, [u8]> {
    Cow::Borrowed(body.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A folder with a game in `game/` and a secret file just outside it.
    fn fixture() -> (tempfile::TempDir, Site) {
        let dir = tempfile::tempdir().expect("temp dir");
        let game = dir.path().join("game");
        fs::create_dir_all(game.join("assets")).expect("assets folder");
        fs::create_dir_all(game.join("pages")).expect("pages folder");
        fs::write(game.join("index.html"), "<p>home</p>").expect("index");
        fs::write(game.join("main.js"), "console.log(1);").expect("script");
        fs::write(game.join("assets").join("a b.txt"), "spaced").expect("spaced name");
        fs::write(game.join("pages").join("index.html"), "<p>pages</p>").expect("pages index");
        fs::write(game.join("data.xyz"), "unknown type").expect("unknown");
        fs::write(dir.path().join("secret.txt"), "outside").expect("secret");
        let site = Site::new(&game, "index.html").expect("site opens");
        (dir, site)
    }

    fn canonical(path: &Path) -> PathBuf {
        fs::canonicalize(path).expect("canonical path")
    }

    #[test]
    fn the_root_maps_to_the_entry_page() {
        let (dir, site) = fixture();
        let resolved = site.resolve("/").expect("root resolves");
        assert_eq!(
            resolved,
            canonical(&dir.path().join("game").join("index.html"))
        );
    }

    #[test]
    fn a_custom_entry_page_is_used_for_the_root() {
        let dir = tempfile::tempdir().expect("temp dir");
        fs::create_dir_all(dir.path().join("pages")).expect("pages folder");
        fs::write(dir.path().join("pages").join("start.html"), "start").expect("start page");
        let site = Site::new(dir.path(), "pages/start.html").expect("site opens");
        let resolved = site.resolve("/").expect("root resolves");
        assert_eq!(
            resolved,
            canonical(&dir.path().join("pages").join("start.html"))
        );
    }

    #[test]
    fn a_missing_entry_page_is_reported_when_the_site_opens() {
        let dir = tempfile::tempdir().expect("temp dir");
        assert!(matches!(
            Site::new(dir.path(), "missing.html"),
            Err(SiteError::MissingEntry(name)) if name == "missing.html"
        ));
    }

    #[test]
    fn a_folder_that_does_not_exist_is_reported() {
        let dir = tempfile::tempdir().expect("temp dir");
        let missing = dir.path().join("nope");
        assert!(matches!(
            Site::new(&missing, "index.html"),
            Err(SiteError::Root { .. })
        ));
    }

    #[test]
    fn plain_files_resolve_inside_the_folder() {
        let (dir, site) = fixture();
        let resolved = site.resolve("/main.js").expect("script resolves");
        assert_eq!(
            resolved,
            canonical(&dir.path().join("game").join("main.js"))
        );
    }

    #[test]
    fn percent_encoded_names_resolve() {
        let (dir, site) = fixture();
        let resolved = site
            .resolve("/assets/a%20b.txt")
            .expect("spaced name resolves");
        assert_eq!(
            resolved,
            canonical(&dir.path().join("game").join("assets").join("a b.txt"))
        );
    }

    #[test]
    fn traversal_and_unsafe_paths_are_refused() {
        let (_dir, site) = fixture();
        for path in [
            "/../secret.txt",
            "/%2e%2e/secret.txt",
            "/%2E%2E%2Fsecret.txt",
            "/assets/../../secret.txt",
            "/..\\secret.txt",
            "/%5c..%5csecret.txt",
            "/C:/Windows/win.ini",
            "/main.js:stream",
            "//etc/passwd",
            "/assets//a b.txt",
            "/%00main.js",
            "/%ZZ",
            "/%E9",
            "main.js",
            "",
        ] {
            assert_eq!(
                site.resolve(path),
                Err(Refusal::Unsafe),
                "`{path}` should be refused as unsafe"
            );
        }
    }

    #[test]
    fn missing_files_are_not_found() {
        let (_dir, site) = fixture();
        assert_eq!(site.resolve("/nope.js"), Err(Refusal::NotFound));
        assert_eq!(site.resolve("/assets"), Err(Refusal::NotFound));
    }

    #[test]
    fn a_trailing_slash_serves_that_folders_index() {
        let (dir, site) = fixture();
        let resolved = site.resolve("/pages/").expect("folder index resolves");
        assert_eq!(
            resolved,
            canonical(&dir.path().join("game").join("pages").join("index.html"))
        );
        assert_eq!(site.resolve("/main.js/"), Err(Refusal::NotFound));
    }

    #[cfg(unix)]
    #[test]
    fn a_symbolic_link_out_of_the_folder_is_not_followed() {
        let (dir, site) = fixture();
        let link = dir.path().join("game").join("link.txt");
        std::os::unix::fs::symlink(dir.path().join("secret.txt"), &link).expect("symlink");
        assert_eq!(site.resolve("/link.txt"), Err(Refusal::NotFound));
    }

    #[test]
    fn mime_types_come_from_the_table() {
        let cases = [
            ("index.html", "text/html; charset=utf-8"),
            ("main.mjs", "text/javascript; charset=utf-8"),
            ("main.js", "text/javascript; charset=utf-8"),
            ("style.css", "text/css; charset=utf-8"),
            ("data.json", "application/json"),
            ("LOGO.PNG", "image/png"),
            ("photo.jpeg", "image/jpeg"),
            ("photo.jpg", "image/jpeg"),
            ("a.webp", "image/webp"),
            ("a.avif", "image/avif"),
            ("a.svg", "image/svg+xml"),
            ("engine.wasm", "application/wasm"),
            ("font.woff2", "font/woff2"),
            ("font.ttf", "font/ttf"),
            ("theme.ogg", "audio/ogg"),
            ("theme.mp3", "audio/mpeg"),
            ("sfx.wav", "audio/wav"),
            ("hero.glb", "model/gltf-binary"),
            ("hero.gltf", "model/gltf+json"),
            ("pack.bin", "application/octet-stream"),
            ("notes.txt", "text/plain; charset=utf-8"),
            ("data.xyz", "application/octet-stream"),
            ("README", "application/octet-stream"),
        ];
        for (name, expected) in cases {
            assert_eq!(mime_for(Path::new(name)), expected, "MIME of {name}");
        }
    }

    #[test]
    fn respond_sends_the_file_with_its_type() {
        let (_dir, site) = fixture();
        let response = site.respond(&Method::GET, "/main.js");
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response
                .headers()
                .get(header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok()),
            Some("text/javascript; charset=utf-8")
        );
        let body: &[u8] = response.body();
        assert_eq!(body, b"console.log(1);");
    }

    #[test]
    fn respond_answers_refused_and_missing_paths_with_404() {
        let (_dir, site) = fixture();
        for path in ["/../secret.txt", "/nope.js"] {
            let response = site.respond(&Method::GET, path);
            assert_eq!(response.status(), StatusCode::NOT_FOUND, "{path}");
            let body: &[u8] = response.body();
            assert_eq!(body, b"Not found");
        }
    }

    #[test]
    fn respond_refuses_other_methods_with_405() {
        let (_dir, site) = fixture();
        let response = site.respond(&Method::POST, "/main.js");
        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
        assert_eq!(
            response
                .headers()
                .get(header::ALLOW)
                .and_then(|v| v.to_str().ok()),
            Some("GET")
        );
    }

    #[test]
    fn percent_decoding_handles_hex_and_rejects_bad_escapes() {
        assert_eq!(percent_decode("/a%20b"), Some("/a b".to_string()));
        assert_eq!(percent_decode("/%2F%2e"), Some("//.".to_string()));
        assert_eq!(percent_decode("/100%"), None);
        assert_eq!(percent_decode("/%G1"), None);
        assert_eq!(
            percent_decode("/plain+plus"),
            Some("/plain+plus".to_string())
        );
    }
}
