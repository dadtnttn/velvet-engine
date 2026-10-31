//! Reading a depth texture texel by texel (`textureLoad`) in a shader, on every backend.
//!
//! WGSL's depth textures (`texture_depth_2d`, `texture_depth_multisampled_2d`) cannot be read
//! with `textureLoad` once naga translates them to OpenGL's GLSL ("textureLoad from depth textures
//! is not supported in GLSL"): GLSL has no `texelFetch` for shadow samplers. WebGPU also lets a
//! depth texture be bound as an unfilterable float texture, which GLSL reads with `texelFetch`, so
//! on OpenGL (ES) the passes that read the depth (the ink outline, the bounce light, the cached
//! shadow map's restore) bind it that way. Every other backend keeps the depth binding.
//!
//! The shaders write `__DEPTH__` for the binding's type and put `__DEPTH_X__` after each
//! `textureLoad` of it; [`DepthRead::shader`] fills them in. With the depth binding the source is
//! the same text as it was before these tokens.

/// How the depth-reading passes bind and read the depth.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DepthRead {
    as_float: bool,
}

impl DepthRead {
    /// The depth binding, or the float one on OpenGL (ES). QA: `VELVET_DEPTH_AS_FLOAT=1` takes
    /// the float binding on any backend (it reads the same values).
    pub(crate) fn for_device(device: &wgpu::Device) -> Self {
        let forced = std::env::var("VELVET_DEPTH_AS_FLOAT").is_ok_and(|v| v == "1");
        Self {
            as_float: forced || is_gl(device),
        }
    }

    /// The depth binding (what every backend but OpenGL uses).
    #[cfg(test)]
    pub(crate) const DEPTH: Self = Self { as_float: false };
    /// The float binding (OpenGL).
    #[cfg(test)]
    pub(crate) const FLOAT: Self = Self { as_float: true };

    /// The binding's sample type in a bind group layout.
    pub(crate) fn sample_type(self) -> wgpu::TextureSampleType {
        if self.as_float {
            wgpu::TextureSampleType::Float { filterable: false }
        } else {
            wgpu::TextureSampleType::Depth
        }
    }

    /// The WGSL type of the binding.
    pub(crate) fn wgsl_type(self, multisampled: bool) -> &'static str {
        match (self.as_float, multisampled) {
            (false, false) => "texture_depth_2d",
            (false, true) => "texture_depth_multisampled_2d",
            (true, false) => "texture_2d<f32>",
            (true, true) => "texture_multisampled_2d<f32>",
        }
    }

    /// A shader's source with its depth tokens filled in (`multisampled`: what `__DEPTH__` is).
    pub(crate) fn shader(self, source: &str, multisampled: bool) -> String {
        source
            .replace("__DEPTH_X__", if self.as_float { ".x" } else { "" })
            .replace("__DEPTH__", self.wgsl_type(multisampled))
    }
}

/// Whether the device runs on OpenGL (ES). wgpu builds its GL backend only on Unix systems other
/// than Apple's (Linux, Android); elsewhere there is none to ask about.
fn is_gl(device: &wgpu::Device) -> bool {
    #[cfg(all(unix, not(target_vendor = "apple"), not(target_arch = "wasm32")))]
    {
        // SAFETY: the callback only looks whether there is a GL device; it keeps nothing.
        unsafe { device.as_hal::<wgpu::hal::api::Gles, _, _>(|gl| gl.is_some()) }
    }
    #[cfg(not(all(unix, not(target_vendor = "apple"), not(target_arch = "wasm32"))))]
    {
        let _ = device;
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_depth_binding_leaves_the_source_as_it_was() {
        let source = "var d: __DEPTH__;\nlet x = textureLoad(d, p, 0)__DEPTH_X__;";
        assert_eq!(
            DepthRead::DEPTH.shader(source, false),
            "var d: texture_depth_2d;\nlet x = textureLoad(d, p, 0);"
        );
        assert_eq!(
            DepthRead::FLOAT.shader(source, true),
            "var d: texture_multisampled_2d<f32>;\nlet x = textureLoad(d, p, 0).x;"
        );
        assert_eq!(
            DepthRead::FLOAT.sample_type(),
            wgpu::TextureSampleType::Float { filterable: false }
        );
        assert_eq!(
            DepthRead::DEPTH.sample_type(),
            wgpu::TextureSampleType::Depth
        );
    }

    /// A shader as OpenGL ES 3.1 GLSL (what wgpu's GL backend makes of it).
    fn glsl(source: &str, stage: naga::ShaderStage, entry: &str) -> Result<String, String> {
        use naga::back::glsl;
        let module = naga::front::wgsl::parse_str(source).map_err(|e| e.emit_to_string(source))?;
        let info = naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        )
        .validate(&module)
        .map_err(|e| format!("{e:?}"))?;
        let options = glsl::Options {
            version: glsl::Version::Embedded {
                version: 310,
                is_webgl: false,
            },
            ..Default::default()
        };
        let pipeline = glsl::PipelineOptions {
            shader_stage: stage,
            entry_point: entry.into(),
            multiview: None,
        };
        let mut out = String::new();
        glsl::Writer::new(
            &mut out,
            &module,
            &info,
            &options,
            &pipeline,
            naga::proc::BoundsCheckPolicies::default(),
        )
        .and_then(|mut writer| writer.write().map(|_| ()))
        .map_err(|e| e.to_string())?;
        Ok(out)
    }

    #[test]
    fn the_depth_reading_passes_translate_to_opengl_es_with_the_float_binding() {
        use naga::ShaderStage::{Compute, Fragment};
        let outline = |read: DepthRead, multisampled: bool| {
            read.shader(crate::mesh3d::OUTLINE_SHADER, multisampled)
                .replace("__SAMPLES__", if multisampled { "4" } else { "1" })
        };
        for multisampled in [false, true] {
            glsl(
                &outline(DepthRead::FLOAT, multisampled),
                Fragment,
                "fs_outline",
            )
            .unwrap();
            let gi = DepthRead::FLOAT.shader(crate::gi3d::GI_SHADER, multisampled);
            glsl(&gi, Compute, "gather").unwrap();
        }
        let restore = DepthRead::FLOAT.shader(crate::mesh3d::SHADOW_RESTORE_SHADER, false);
        glsl(&restore, Fragment, "fs_main").unwrap();
        // The depth binding is what GLSL cannot read texel by texel.
        assert!(glsl(&outline(DepthRead::DEPTH, false), Fragment, "fs_outline").is_err());
    }
}
