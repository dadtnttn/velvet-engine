//! Every shader the renderer builds, translated offline to what a Vulkan driver receives: SPIR-V
//! as wgpu's Vulkan backend writes it (one module per entry point, its overrides resolved), read
//! back by naga's SPIR-V reader. A module naga writes wrong fails here instead of in a driver:
//! naga 24 once wrote the skinning shader with a variable used but never declared, which crashed
//! desktop Vulkan drivers and froze the Android emulator. `VELVET_SPV_DIR=<folder>` also writes
//! the SPIR-V there, for `spirv-val`.

use std::collections::HashMap;

use naga::back::spv;

/// The renderer's shaders as it builds them, by name.
fn sources() -> Vec<(String, String)> {
    use crate::depth_read::DepthRead;
    let mut out = vec![
        ("sprite".to_string(), crate::gpu::SPRITE_SHADER.to_string()),
        (
            "mesh".to_string(),
            crate::mesh3d::mesh_shader_source().to_string(),
        ),
        (
            "shadow".to_string(),
            crate::mesh3d::SHADOW_SHADER.to_string(),
        ),
        ("post".to_string(), crate::post3d::POST_SHADER.to_string()),
        ("skin".to_string(), crate::skin3d::SKIN_SHADER.to_string()),
        (
            "crowd".to_string(),
            crate::crowd3d::CROWD_SHADER.to_string(),
        ),
        ("upscale".to_string(), crate::upscale3d::SHADER.to_string()),
        (
            "water-sph".to_string(),
            crate::water3d::SPH_SHADER.to_string(),
        ),
        (
            "water-draw".to_string(),
            format!(
                "{}{}",
                crate::water3d::CAMERA_WGSL,
                crate::water3d::DRAW_SHADER
            ),
        ),
    ];
    for (read, tag) in [(DepthRead::DEPTH, "depth"), (DepthRead::FLOAT, "float")] {
        for (multisampled, samples) in [(false, "1"), (true, "4")] {
            out.push((
                format!("outline-{tag}-{samples}"),
                read.shader(crate::mesh3d::OUTLINE_SHADER, multisampled)
                    .replace("__SAMPLES__", samples),
            ));
            out.push((
                format!("gi-{tag}-{samples}"),
                read.shader(crate::gi3d::GI_SHADER, multisampled),
            ));
        }
        out.push((
            format!("shadow-restore-{tag}"),
            read.shader(crate::mesh3d::SHADOW_RESTORE_SHADER, false),
        ));
    }
    out
}

/// The overrides each shader's pipelines set (every combination they use).
fn constant_sets(name: &str) -> Vec<HashMap<String, f64>> {
    if name == "mesh" {
        [0.0, 1.0]
            .into_iter()
            .map(|normal_map| HashMap::from([("NORMAL_MAP".to_string(), normal_map)]))
            .collect()
    } else {
        vec![HashMap::new()]
    }
}

/// What wgpu 24's Vulkan backend hands naga (wgpu-hal `vulkan/adapter.rs`), with bounds checks
/// on and the workgroup memory zeroed by a polyfill (the strictest case).
fn vulkan_options() -> spv::Options<'static> {
    spv::Options {
        lang_version: (1, 0),
        flags: spv::WriterFlags::LABEL_VARYINGS | spv::WriterFlags::FORCE_POINT_SIZE,
        capabilities: None,
        bounds_check_policies: naga::proc::BoundsCheckPolicies {
            index: naga::proc::BoundsCheckPolicy::Restrict,
            buffer: naga::proc::BoundsCheckPolicy::Restrict,
            image_load: naga::proc::BoundsCheckPolicy::Restrict,
            binding_array: naga::proc::BoundsCheckPolicy::Unchecked,
        },
        zero_initialize_workgroup_memory: spv::ZeroInitializeWorkgroupMemoryMode::Polyfill,
        binding_map: Default::default(),
        debug_info: None,
    }
}

#[test]
fn every_renderer_shader_becomes_valid_spirv() {
    let dump = std::env::var_os("VELVET_SPV_DIR").map(std::path::PathBuf::from);
    if let Some(dir) = &dump {
        std::fs::create_dir_all(dir).unwrap();
    }
    let mut failures = Vec::new();
    let mut written = 0;
    for (name, source) in sources() {
        let module = match naga::front::wgsl::parse_str(&source) {
            Ok(module) => module,
            Err(error) => {
                failures.push(format!("{name}: {}", error.emit_to_string(&source)));
                continue;
            }
        };
        let info = match naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        )
        .validate(&module)
        {
            Ok(info) => info,
            Err(error) => {
                failures.push(format!("{name}: {error:?}"));
                continue;
            }
        };
        for (set, constants) in constant_sets(&name).into_iter().enumerate() {
            let (module, info) =
                naga::back::pipeline_constants::process_overrides(&module, &info, &constants)
                    .unwrap_or_else(|e| panic!("{name}: overrides: {e}"));
            for entry in &module.entry_points {
                let pipeline = spv::PipelineOptions {
                    shader_stage: entry.stage,
                    entry_point: entry.name.clone(),
                };
                match spv::write_vec(&module, &info, &vulkan_options(), Some(&pipeline)) {
                    Ok(words) => {
                        let bytes = bytemuck::cast_slice::<u32, u8>(&words);
                        if let Err(error) =
                            naga::front::spv::parse_u8_slice(bytes, &Default::default())
                        {
                            failures
                                .push(format!("{name}/{}: invalid SPIR-V: {error}", entry.name));
                        }
                        if let Some(dir) = &dump {
                            let file = dir.join(format!("{name}-{set}-{}.spv", entry.name));
                            std::fs::write(file, bytemuck::cast_slice::<u32, u8>(&words)).unwrap();
                            written += 1;
                        }
                    }
                    Err(error) => failures.push(format!("{name}/{}: spv: {error}", entry.name)),
                }
            }
        }
    }
    if dump.is_some() {
        eprintln!("{written} SPIR-V modules written");
    }
    assert!(failures.is_empty(), "{failures:#?}");
}
