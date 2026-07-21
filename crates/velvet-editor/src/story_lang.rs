//! Velvet Story integration for Studio (writer language `.vstory`).

use std::path::Path;

use velvet_story_lang::commands::CommandRegistry;
use velvet_story_lang::studio::{build_model, StudioModel};

/// Build a Studio model from `.vstory` source (resolves `include`s).
///
/// This is the editor-facing entry point into `velvet-story-lang`.
pub fn story_studio_model(source: &str, file: &str) -> StudioModel {
    let cmds = CommandRegistry::builtin();
    build_model(source, file, &cmds)
}

/// Load a `.vstory` path from disk and build a Studio model.
pub fn story_studio_model_path(path: &Path) -> Result<StudioModel, String> {
    let source = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let file = path.to_string_lossy().to_string();
    Ok(story_studio_model(&source, &file))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn editor_builds_studio_model_with_include() {
        let dir = tempdir().unwrap();
        let child = dir.path().join("chapter.vstory");
        let root = dir.path().join("main.vstory");
        std::fs::write(&child, "scene from_include\nnarrator:\n    hi\nend\n").unwrap();
        let root_src = "include \"chapter.vstory\"\n\nscene start\nnarrator:\n    root\nend\n";
        std::fs::write(&root, root_src).unwrap();

        let model = story_studio_model_path(&root).expect("model");
        assert!(
            model.scenes.iter().any(|s| s.name == "from_include"),
            "included scene missing: {:?}",
            model.scenes.iter().map(|s| &s.name).collect::<Vec<_>>()
        );
        assert!(model.scenes.iter().any(|s| s.name == "start"));
    }
}
