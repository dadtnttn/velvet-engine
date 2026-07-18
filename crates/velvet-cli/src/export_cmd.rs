//! Desktop + web export CLI.

use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use velvet_story::prelude::*;

/// Export desktop package (dry-run by default), or web player when platform=web.
pub fn cmd_export(
    out: PathBuf,
    binary: String,
    assets: PathBuf,
    release: bool,
    build: bool,
    platform: Option<String>,
    multi: bool,
    exclude: Vec<String>,
) -> Result<()> {
    // Web product player (Node-runnable + interactive browser).
    if platform.as_deref() == Some("web") {
        return cmd_export_web(out, assets, binary);
    }

    // Android APK packaging path (dry-run by default unless --build).
    if platform.as_deref() == Some("android") {
        let report = velvet_build::export_android(&velvet_build::AndroidExportOptions {
            out_dir: out.clone(),
            application_id: format!("com.velvet.{}", binary.replace('-', "_")),
            app_name: binary.clone(),
            version_name: "0.1.0".into(),
            version_code: 1,
            dry_run: !build,
            abis: vec!["arm64-v8a".into(), "x86_64".into()],
        })
        .map_err(|e| anyhow::anyhow!("{e}"))?;
        for line in &report.log {
            println!("{line}");
        }
        println!("ANDROID_APK_BUILT={}", report.apk_built);
        println!("ANDROID_NOTE={}", report.toolchain_note);
        println!("export android ready at {}", report.out_dir.display());
        return Ok(());
    }

    // Explicit linux cross path with honest toolchain failure logs.
    if matches!(platform.as_deref(), Some("linux") | Some("linux-x64")) && build {
        let report = velvet_build::try_cross_compile_linux(&binary, &out, false)
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        for line in &report.log {
            if !line.trim().is_empty() {
                println!("{line}");
            }
        }
        println!("LINUX_CROSS_OK={}", report.success);
        println!("LINUX_CROSS_NOTE={}", report.note);
        if report.success {
            if let Some(bin) = &report.binary_path {
                println!("linux binary: {}", bin.display());
            }
            return Ok(());
        }
        // Toolchain missing or link failed: keep entry point real, ship dry-run metadata.
        println!("linux cross failed; writing dry-run packaging metadata (honest failure)");
        let dry = velvet_build::export_desktop(&velvet_build::ExportOptions {
            out_dir: out,
            binary_name: binary,
            assets_dir: assets,
            release: true,
            target: Some("x86_64-unknown-linux-gnu".into()),
            project_name: "velvet-export".into(),
            dry_run: true,
            platform: "linux-x64".into(),
            exclude: vec![],
            include: vec![],
        })
        .map_err(|e| anyhow::anyhow!("{e}"))?;
        for line in dry.log {
            if !line.trim().is_empty() {
                println!("{line}");
            }
        }
        println!("export dry-run ready at {}", dry.out_dir.display());
        return Ok(());
    }

    if multi {
        let platforms: Vec<velvet_build::ExportPlatform> = if let Some(p) = platform {
            // comma-separated
            p.split(',')
                .filter_map(|s| velvet_build::ExportPlatform::parse(s.trim()))
                .collect()
        } else {
            vec![]
        };
        let summary = velvet_build::export_multi_platform_dry_run(
            "velvet-export",
            &assets,
            &out,
            &binary,
            &platforms,
        )
        .map_err(|e| anyhow::anyhow!("{e}"))?;
        for note in &summary.notes {
            println!("{note}");
        }
        println!(
            "multi-platform dry-run: {} platform(s) -> {}",
            summary.platforms.len(),
            out.display()
        );
        return Ok(());
    }

    let mut platform_key = "host".to_string();
    let mut target = None;
    if let Some(p) = platform {
        if let Some(ep) = velvet_build::ExportPlatform::parse(&p) {
            platform_key = ep.as_str().into();
            target = ep.target_triple().map(str::to_string);
        } else if p.contains('-') {
            // raw triple
            platform_key = p.clone();
            target = Some(p);
        } else {
            bail!("unknown platform: {p}");
        }
    }

    let report = velvet_build::export_desktop(&velvet_build::ExportOptions {
        out_dir: out,
        binary_name: binary,
        assets_dir: assets,
        release,
        target,
        project_name: "velvet-export".into(),
        dry_run: !build,
        platform: platform_key,
        exclude,
        include: vec![],
    })
    .map_err(|e| anyhow::anyhow!("{e}"))?;
    for line in report.log {
        if !line.trim().is_empty() {
            println!("{line}");
        }
    }
    if let Some(archive) = &report.archive_path {
        println!("archive: {}", archive.display());
        if let Ok(entries) = velvet_build::list_zip_entries(archive) {
            println!("archive entries: {}", entries.len());
            for e in entries.iter().take(32) {
                println!("  {e}");
            }
        }
    }
    println!("export ready at {}", report.out_dir.display());
    Ok(())
}

/// Export static web player from a project `assets` parent or story next to out.
fn cmd_export_web(out: PathBuf, assets: PathBuf, title: String) -> Result<()> {
    // Resolve story: assets/../scripts/main.vel or assets if it is a .vel
    let project = assets
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));
    let story_path = if assets.extension().and_then(|e| e.to_str()) == Some("vel") {
        assets.clone()
    } else {
        let candidates = [
            project.join("scripts/main.vel"),
            project.join("story/main.vel"),
            PathBuf::from("scripts/main.vel"),
        ];
        candidates
            .into_iter()
            .find(|p| p.exists())
            .ok_or_else(|| anyhow::anyhow!("no scripts/main.vel near {}", assets.display()))?
    };
    let source = std::fs::read_to_string(&story_path)
        .with_context(|| format!("read {}", story_path.display()))?;
    let program =
        load_program_from_source(&source, Some(&story_path.to_string_lossy()), title.clone())
            .map_err(|e| anyhow::anyhow!("{e}"))?;
    let json = velvet_story::program_to_web_json(&program);
    let json_s = serde_json::to_string_pretty(&json)?;
    let report = velvet_build::export_web_player(&out, &title, &json_s)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    for line in &report.log {
        println!("{line}");
    }
    println!("web export ready at {}", report.out_dir.display());
    // Real run log via Node
    match velvet_build::run_web_player_node(&report.play_js, 0) {
        Ok((stdout, code)) => {
            print!("{stdout}");
            println!("WEB_RUN_EXIT={code}");
            if code != 0 {
                bail!("web player node exit {code}");
            }
        }
        Err(e) => {
            println!("web node run failed: {e}");
            bail!("{e}");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn dry_run_multi() {
        let dir = tempdir().unwrap();
        let assets = dir.path().join("assets");
        fs::create_dir_all(&assets).unwrap();
        fs::write(assets.join("x.txt"), b"1").unwrap();
        let out = dir.path().join("dist");
        cmd_export(
            out.clone(),
            "bin".into(),
            assets,
            true,
            false,
            Some("host,linux".into()),
            true,
            vec![],
        )
        .unwrap();
        assert!(out.join("multi-platform-export.json").exists());
    }
}
