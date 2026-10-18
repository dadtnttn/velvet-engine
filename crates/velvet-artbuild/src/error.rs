//! Errors of the art build: documents that are refused as a whole, and files that cannot be read
//! or written. A single output that fails is not an error of this kind: it is listed in the report.

use std::fmt;
use std::path::PathBuf;

use serde::Serialize;

/// The rule that a document broke. The names are the codes of `docs/formats/ART_MANIFEST.md` and
/// `docs/formats/ART_BUILD.md`, which every reader uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ErrorCode {
    /// The text is not JSON.
    Json,
    /// The format tag is not the one that the document must carry.
    Format,
    /// The version is not 1.
    Version,
    /// A field is missing, or has the wrong JSON type.
    Structure,
    /// A value breaks its grammar or its range.
    Value,
    /// Two entries, variants or file names use the same name.
    Duplicate,
    /// The build config has a name that the format does not define.
    UnknownField,
    /// The build config overrides an id that the manifest does not declare.
    UnknownId,
}

impl ErrorCode {
    /// The code as the format documents name it, such as `unknown-field`.
    pub fn as_str(self) -> &'static str {
        match self {
            ErrorCode::Json => "json",
            ErrorCode::Format => "format",
            ErrorCode::Version => "version",
            ErrorCode::Structure => "structure",
            ErrorCode::Value => "value",
            ErrorCode::Duplicate => "duplicate",
            ErrorCode::UnknownField => "unknown-field",
            ErrorCode::UnknownId => "unknown-id",
        }
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One problem found in a document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Issue {
    /// The rule that broke.
    pub code: ErrorCode,
    /// Where the problem is, such as `entries[2].size`. None when it concerns the whole document.
    pub path: Option<String>,
    /// A sentence for logs and developers. Its wording is not part of the format.
    pub message: String,
}

/// Builds one issue at a path.
pub(crate) fn issue(code: ErrorCode, path: &str, message: impl Into<String>) -> Issue {
    Issue {
        code,
        path: Some(path.to_string()),
        message: message.into(),
    }
}

/// A document that a reader refuses as a whole. `code` is the code of the first problem, and
/// `issues` lists every problem that was found, in the order of the document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refusal {
    /// The code of the first problem.
    pub code: ErrorCode,
    /// Every problem found. Never empty.
    pub issues: Vec<Issue>,
}

impl Refusal {
    /// A refusal with one problem, at `path` when there is one.
    pub(crate) fn single(code: ErrorCode, message: impl Into<String>, path: Option<&str>) -> Self {
        let issue = Issue {
            code,
            path: path.map(str::to_string),
            message: message.into(),
        };
        Refusal {
            code,
            issues: vec![issue],
        }
    }

    /// A refusal for a list of problems, or None when the list is empty.
    pub(crate) fn from_issues(issues: Vec<Issue>) -> Option<Self> {
        let code = issues.first()?.code;
        Some(Refusal { code, issues })
    }
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let first = self
            .issues
            .first()
            .map_or("unknown problem", |issue| issue.message.as_str());
        match self.issues.len() {
            0 | 1 => write!(f, "{}: {first}", self.code),
            count => write!(
                f,
                "{}: {first} (and {} more problems)",
                self.code,
                count - 1
            ),
        }
    }
}

impl std::error::Error for Refusal {}

/// Everything that stops a build or a check. A failed output is not one of these: the build goes
/// on, and the failure is listed in the report.
#[derive(Debug)]
pub enum ArtBuildError {
    /// A document was refused as a whole, so nothing was built.
    Refused {
        /// The file that was refused.
        file: PathBuf,
        /// Why it was refused, with every problem found.
        refusal: Refusal,
    },
    /// A file or a folder could not be read, or the outputs could not be listed.
    Io {
        /// The path that could not be used.
        path: PathBuf,
        /// The reason, from the operating system.
        source: std::io::Error,
    },
}

impl fmt::Display for ArtBuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ArtBuildError::Refused { file, refusal } => {
                write!(f, "{} is refused: {refusal}", file.display())
            }
            ArtBuildError::Io { path, source } => {
                write!(f, "cannot use {}: {source}", path.display())
            }
        }
    }
}

impl std::error::Error for ArtBuildError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ArtBuildError::Refused { refusal, .. } => Some(refusal),
            ArtBuildError::Io { source, .. } => Some(source),
        }
    }
}
