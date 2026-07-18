//! Asset pack CLI.

use std::path::PathBuf;

use anyhow::Result;

/// Pack assets directory to JSON manifest with optional excludes.
pub fn cmd_pack(
    assets: PathBuf,
    out: PathBuf,
    exclude: Vec<String>,
    include: Vec<String>,
) -> Result<()> {
    let opts = velvet_build::PackOptions {
        exclude,
        include,
        skip_hidden: true,
        max_file_size: 0,
    };
    let pack =
        velvet_build::pack_directory_with(&assets, &opts).map_err(|e| anyhow::anyhow!("{e}"))?;
    std::fs::write(
        &out,
        pack.to_json_pretty().map_err(|e| anyhow::anyhow!("{e}"))?,
    )?;
    println!(
        "packed {} files ({} bytes) -> {}",
        pack.files.len(),
        pack.total_size,
        out.display()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn pack_with_exclude() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("a.png"), b"1").unwrap();
        fs::write(dir.path().join("b.tmp"), b"2").unwrap();
        let out = dir.path().join("pack.json");
        cmd_pack(
            dir.path().to_path_buf(),
            out.clone(),
            vec!["**/*.tmp".into()],
            vec![],
        )
        .unwrap();
        let text = fs::read_to_string(out).unwrap();
        assert!(text.contains("a.png"));
        assert!(!text.contains("b.tmp"));
    }

    #[test]
    fn pack_with_include_only_png() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("a.png"), b"1").unwrap();
        fs::write(dir.path().join("b.txt"), b"2").unwrap();
        let out = dir.path().join("pack.json");
        cmd_pack(
            dir.path().to_path_buf(),
            out.clone(),
            vec![],
            vec!["**/*.png".into()],
        )
        .unwrap();
        let text = fs::read_to_string(out).unwrap();
        assert!(text.contains("a.png"));
        assert!(!text.contains("b.txt"));
    }

    #[test]
    fn pack_nested_directories() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join("assets/sprites")).unwrap();
        fs::write(dir.path().join("assets/sprites/hero.png"), b"img").unwrap();
        fs::write(dir.path().join("readme.md"), b"doc").unwrap();
        let out = dir.path().join("out.json");
        cmd_pack(dir.path().to_path_buf(), out.clone(), vec![], vec![]).unwrap();
        let text = fs::read_to_string(&out).unwrap();
        assert!(text.contains("hero.png") || text.contains("sprites"));
        assert!(out.exists());
    }
}
