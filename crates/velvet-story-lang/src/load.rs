//! Multi-file load: resolve `include` into a single StoryFile for check/build/run.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::ast::{StoryFile, TopItem};
use crate::diag::StoryDiag;
use crate::parser::parse;
use crate::span::Span;

/// Load a root `.vstory` and recursively merge included files.
pub fn load_story_path(path: &Path) -> Result<(StoryFile, Vec<StoryDiag>), String> {
    let mut diags = Vec::new();
    let mut visited = HashSet::new();
    let file = load_recursive(path, &mut visited, &mut diags, 0)?;
    Ok((file, diags))
}

/// Load from source string with an optional base directory for includes.
pub fn load_story_source(
    source: &str,
    file: &str,
    base_dir: Option<&Path>,
) -> Result<(StoryFile, Vec<StoryDiag>), String> {
    let mut diags = Vec::new();
    let mut visited = HashSet::new();
    // virtual path for cycle detection
    let root = PathBuf::from(file);
    visited.insert(std::fs::canonicalize(&root).unwrap_or(root.clone()));
    let parsed = parse(source, file);
    diags.extend(parsed.diags);
    let base = base_dir
        .map(|p| p.to_path_buf())
        .or_else(|| root.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."));
    let merged = merge_includes(parsed.file, &base, &mut visited, &mut diags, 0)?;
    Ok((merged, diags))
}

fn load_recursive(
    path: &Path,
    visited: &mut HashSet<PathBuf>,
    diags: &mut Vec<StoryDiag>,
    depth: usize,
) -> Result<StoryFile, String> {
    if depth > 32 {
        return Err("include demasiado profundo (máx 32)".into());
    }
    let canon = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    if !visited.insert(canon.clone()) {
        diags.push(StoryDiag::error(
            "VST040",
            format!("include circular: {}", path.display()),
            path.to_string_lossy(),
            Span::unknown(),
        ));
        return Ok(StoryFile {
            file: path.to_string_lossy().into(),
            items: vec![],
        });
    }
    let source = std::fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    let file_s = path.to_string_lossy().to_string();
    let parsed = parse(&source, &file_s);
    diags.extend(parsed.diags);
    let base = path.parent().unwrap_or_else(|| Path::new("."));
    merge_includes(parsed.file, base, visited, diags, depth)
}

fn merge_includes(
    mut file: StoryFile,
    base: &Path,
    visited: &mut HashSet<PathBuf>,
    diags: &mut Vec<StoryDiag>,
    depth: usize,
) -> Result<StoryFile, String> {
    let mut out_items = Vec::new();
    let items = std::mem::take(&mut file.items);
    for item in items {
        match item {
            TopItem::Include { path, span } => {
                let resolved = resolve_include(base, &path);
                if !resolved.exists() {
                    diags.push(
                        StoryDiag::error(
                            "VST041",
                            format!(
                                "No se encuentra el archivo incluido `{path}` (buscado en {}).",
                                resolved.display()
                            ),
                            &file.file,
                            span,
                        )
                        .with_suggestion("Revisa la ruta relativa al archivo actual.")
                        .with_node("include"),
                    );
                    continue;
                }
                match load_recursive(&resolved, visited, diags, depth + 1) {
                    Ok(inc) => {
                        for it in inc.items {
                            // keep scenes/characters from included file; drop nested includes already expanded
                            match it {
                                TopItem::Include { .. } => {}
                                other => out_items.push(other),
                            }
                        }
                    }
                    Err(e) => {
                        diags.push(StoryDiag::error(
                            "VST042",
                            format!("Error al cargar include `{path}`: {e}"),
                            &file.file,
                            span,
                        ));
                    }
                }
            }
            other => out_items.push(other),
        }
    }
    file.items = out_items;
    Ok(file)
}

fn resolve_include(base: &Path, path: &str) -> PathBuf {
    let p = Path::new(path);
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        base.join(p)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn include_merges_scene() {
        let dir = tempdir().unwrap();
        let inc = dir.path().join("part.vstory");
        let root = dir.path().join("main.vstory");
        std::fs::write(
            &inc,
            "scene hallway\nnarrator:\n    Pasillo.\nend\n",
        )
        .unwrap();
        std::fs::write(
            &root,
            "include \"part.vstory\"\n\nscene start\ngoto hallway\n",
        )
        .unwrap();
        let (file, diags) = load_story_path(&root).unwrap();
        assert!(
            !diags.iter().any(|d| d.is_error()),
            "{:?}",
            diags
        );
        let names: Vec<_> = file
            .items
            .iter()
            .filter_map(|i| match i {
                TopItem::Scene(s) => Some(s.name.as_str()),
                _ => None,
            })
            .collect();
        assert!(names.contains(&"start"));
        assert!(names.contains(&"hallway"));
    }
}
