//! Project validation: schema-ish checks, paths, modules.

use std::path::{Path, PathBuf};

use crate::modules::{ModuleRegistry, ModuleResolveError};
use crate::VelvetProject;

/// Severity of a validation finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationSeverity {
    /// Hard error — project should not ship / run in strict mode.
    Error,
    /// Soft issue.
    Warning,
    /// Informational note.
    Info,
}

/// One validation finding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationIssue {
    /// Severity.
    pub severity: ValidationSeverity,
    /// Machine-oriented code.
    pub code: String,
    /// Human message.
    pub message: String,
    /// Optional path related to the issue.
    pub path: Option<PathBuf>,
}

impl ValidationIssue {
    /// Create an error issue.
    pub fn error(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            severity: ValidationSeverity::Error,
            code: code.into(),
            message: message.into(),
            path: None,
        }
    }

    /// Create a warning issue.
    pub fn warning(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            severity: ValidationSeverity::Warning,
            code: code.into(),
            message: message.into(),
            path: None,
        }
    }

    /// Create an informational issue.
    pub fn info(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            severity: ValidationSeverity::Info,
            code: code.into(),
            message: message.into(),
            path: None,
        }
    }

    /// Attach a filesystem path to this issue.
    pub fn with_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.path = Some(path.into());
        self
    }
}

/// Full validation report.
#[derive(Debug, Clone, Default)]
pub struct ValidationReport {
    /// Findings.
    pub issues: Vec<ValidationIssue>,
    /// Resolved module load order when successful.
    pub resolved_modules: Vec<String>,
}

impl ValidationReport {
    /// Number of error-severity findings.
    pub fn error_count(&self) -> usize {
        self.issues
            .iter()
            .filter(|i| i.severity == ValidationSeverity::Error)
            .count()
    }

    /// Number of warning-severity findings.
    pub fn warning_count(&self) -> usize {
        self.issues
            .iter()
            .filter(|i| i.severity == ValidationSeverity::Warning)
            .count()
    }

    /// Whether the report has no errors.
    pub fn is_ok(&self) -> bool {
        self.error_count() == 0
    }

    /// Append an issue.
    pub fn push(&mut self, issue: ValidationIssue) {
        self.issues.push(issue);
    }
}

/// Options controlling filesystem checks.
#[derive(Debug, Clone)]
pub struct ValidateOptions {
    /// Project root directory (for path existence checks).
    pub root: Option<PathBuf>,
    /// Fail on unknown module names.
    pub strict_modules: bool,
    /// Auto-note missing dependency modules as errors (vs warnings).
    pub require_deps_listed: bool,
    /// Check that entry_scene / assets_dir exist when root is set.
    pub check_paths: bool,
}

impl Default for ValidateOptions {
    fn default() -> Self {
        Self {
            root: None,
            strict_modules: false,
            require_deps_listed: false,
            check_paths: true,
        }
    }
}

/// Validate a project document (optionally against a filesystem root).
pub fn validate_project(project: &VelvetProject, opts: &ValidateOptions) -> ValidationReport {
    let mut report = ValidationReport::default();
    validate_identity(project, &mut report);
    validate_modules(project, opts, &mut report);
    if opts.check_paths {
        if let Some(root) = &opts.root {
            validate_paths(project, root, &mut report);
        }
    }
    report
}

fn validate_identity(project: &VelvetProject, report: &mut ValidationReport) {
    if project.name.trim().is_empty() {
        report.push(ValidationIssue::error(
            "empty_name",
            "project name must not be empty",
        ));
    }
    if project.identifier.trim().is_empty() {
        report.push(ValidationIssue::error(
            "empty_identifier",
            "project identifier must not be empty",
        ));
    } else if !is_reasonable_identifier(&project.identifier) {
        report.push(ValidationIssue::warning(
            "identifier_format",
            format!(
                "identifier `{}` should look like reverse-DNS (e.g. com.studio.game)",
                project.identifier
            ),
        ));
    }
    if project.version.trim().is_empty() {
        report.push(ValidationIssue::error(
            "empty_version",
            "project version must not be empty",
        ));
    } else if !looks_like_semver(&project.version) {
        report.push(ValidationIssue::warning(
            "version_format",
            format!(
                "version `{}` does not look like semver (major.minor.patch)",
                project.version
            ),
        ));
    }
    if project.entry_scene.trim().is_empty() {
        report.push(ValidationIssue::error(
            "empty_entry",
            "entry_scene must not be empty",
        ));
    }
    if project.modules.is_empty() {
        report.push(ValidationIssue::warning(
            "no_modules",
            "no modules enabled — enable at least `story` or `play`",
        ));
    }
    if project.window.width == 0 || project.window.height == 0 {
        report.push(ValidationIssue::error(
            "bad_window",
            "window width/height must be non-zero",
        ));
    }
}

fn validate_modules(
    project: &VelvetProject,
    opts: &ValidateOptions,
    report: &mut ValidationReport,
) {
    let reg = ModuleRegistry::builtin();

    // Duplicates
    let mut seen = BTreeSet::new();
    for m in &project.modules {
        if !seen.insert(m.as_str()) {
            report.push(ValidationIssue::warning(
                "duplicate_module",
                format!("module `{m}` listed more than once"),
            ));
        }
        if !reg.is_known(m) {
            if opts.strict_modules {
                report.push(ValidationIssue::error(
                    "unknown_module",
                    format!("unknown module `{m}`"),
                ));
            } else {
                report.push(ValidationIssue::info(
                    "unknown_module",
                    format!("unknown module `{m}` (treated as project-specific flag)"),
                ));
            }
        }
    }

    match reg.resolve_dependencies(&project.modules) {
        Ok(order) => {
            report.resolved_modules = order;
            match reg.missing_dependencies(&project.modules) {
                Ok(missing) => {
                    for m in missing {
                        if opts.require_deps_listed {
                            report.push(ValidationIssue::error(
                                "missing_dep",
                                format!(
                                    "module dependency `{m}` is required but not listed in modules"
                                ),
                            ));
                        } else {
                            report.push(ValidationIssue::warning(
                                "missing_dep",
                                format!(
                                    "module dependency `{m}` will be pulled in implicitly; consider listing it"
                                ),
                            ));
                        }
                    }
                }
                Err(e) => report.push(dep_error(e)),
            }
            for w in reg.recommendation_warnings(&project.modules) {
                report.push(ValidationIssue::info("recommend", w));
            }
        }
        Err(e) => report.push(dep_error(e)),
    }
}

fn dep_error(e: ModuleResolveError) -> ValidationIssue {
    ValidationIssue::error("module_resolve", e.to_string())
}

fn validate_paths(project: &VelvetProject, root: &Path, report: &mut ValidationReport) {
    if !root.exists() {
        report.push(
            ValidationIssue::error(
                "root_missing",
                format!("project root does not exist: {}", root.display()),
            )
            .with_path(root),
        );
        return;
    }

    let assets = root.join(&project.assets_dir);
    if !assets.exists() {
        report.push(
            ValidationIssue::warning(
                "assets_missing",
                format!("assets directory missing: {}", assets.display()),
            )
            .with_path(assets),
        );
    }

    let entry = root.join(&project.entry_scene);
    if !entry.exists() {
        // entry may be virtual; also check scripts/main.vel fallback
        let alt = root.join("scripts/main.vel");
        if !alt.exists() {
            report.push(
                ValidationIssue::warning(
                    "entry_missing",
                    format!(
                        "entry_scene `{}` not found (also no scripts/main.vel)",
                        project.entry_scene
                    ),
                )
                .with_path(entry),
            );
        } else {
            report.push(ValidationIssue::info(
                "entry_fallback",
                format!(
                    "entry_scene `{}` missing; scripts/main.vel present",
                    project.entry_scene
                ),
            ));
        }
    }

    let project_file = root.join("velvet.project");
    if !project_file.exists() {
        report.push(
            ValidationIssue::warning(
                "project_file_missing",
                "velvet.project not found under root (validating in-memory document only)",
            )
            .with_path(project_file),
        );
    }
}

fn is_reasonable_identifier(id: &str) -> bool {
    let parts: Vec<&str> = id.split('.').collect();
    if parts.len() < 2 {
        return false;
    }
    parts.iter().all(|p| {
        !p.is_empty()
            && p.chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    })
}

fn looks_like_semver(v: &str) -> bool {
    let core = v.split('-').next().unwrap_or(v);
    let mut parts = core.split('.');
    let major = parts.next().and_then(|s| s.parse::<u64>().ok());
    let minor = parts.next().and_then(|s| s.parse::<u64>().ok());
    let patch = parts.next().and_then(|s| s.parse::<u64>().ok());
    major.is_some() && minor.is_some() && patch.is_some() && parts.next().is_none()
}

use std::collections::BTreeSet;

/// Load project from root and validate against filesystem.
pub fn validate_root(
    root: impl AsRef<Path>,
) -> Result<(VelvetProject, ValidationReport), crate::ProjectError> {
    let root = root.as_ref();
    let path = root.join("velvet.project");
    let project = VelvetProject::load(&path)?;
    let report = validate_project(
        &project,
        &ValidateOptions {
            root: Some(root.to_path_buf()),
            ..ValidateOptions::default()
        },
    );
    Ok((project, report))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::default_project_ron;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn default_project_validates() {
        let p = VelvetProject::default();
        let report = validate_project(&p, &ValidateOptions::default());
        assert!(report.is_ok(), "{:?}", report.issues);
        assert!(!report.resolved_modules.is_empty());
    }

    #[test]
    fn empty_name_errors() {
        let mut p = VelvetProject::default();
        p.name.clear();
        let report = validate_project(&p, &ValidateOptions::default());
        assert!(!report.is_ok());
        assert!(report.issues.iter().any(|i| i.code == "empty_name"));
    }

    #[test]
    fn filesystem_warnings() {
        let dir = tempdir().unwrap();
        let text = default_project_ron("Demo");
        fs::write(dir.path().join("velvet.project"), &text).unwrap();
        let (p, report) = validate_root(dir.path()).unwrap();
        assert_eq!(p.name, "Demo");
        // assets and entry missing => warnings
        assert!(report.warning_count() >= 1);
    }

    #[test]
    fn rpg_resolves() {
        let p = VelvetProject {
            modules: vec!["rpg".into()],
            ..Default::default()
        };
        let report = validate_project(&p, &ValidateOptions::default());
        assert!(report.is_ok());
        assert!(report.resolved_modules.contains(&"play".into()));
    }
}
