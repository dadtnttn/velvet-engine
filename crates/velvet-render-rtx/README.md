# velvet-render-rtx — Vulkan RTX backend (Nvidia, separado)

> **Separado a propósito de `velvet-render`.** `velvet-render` es el renderer wgpu forward multiplataforma (shadow maps + PBR) que compila en cualquier GPU. Este crate es el backend **hardware ray tracing** solo para Nvidia via Vulkan.

## Por qué separado
- `velvet-render` no toca RTX (`mesh3d.rs:384` lo descarta explícitamente). Low cost (SSAO/SSR/CSM/IBL) queda ahí.
- `velvet-render-rtx` vive detrás del feature `rtx-nvidia` y usa `ash` + `VK_KHR_ray_tracing_pipeline`. Sin el feature compila como stub y no rompe CI/laptops sin RTX.

## Uso
```toml
[dependencies]
velvet-render-rtx = { path = "crates/velvet-render-rtx", features = ["rtx-nvidia"] }
```

```rust
use velvet_render_rtx::{RenderBackend3D, RtxCamera, RtxDraw, RtxMesh, VulkanRtxBackend, FallbackBackend};

#[cfg(feature = "rtx-nvidia")]
let mut backend = match VulkanRtxBackend::probe() {
    Ok(b) if b.can_trace() => Box::new(b) as Box<dyn RenderBackend3D>,
    _ => Box::new(FallbackBackend::default()) as Box<dyn RenderBackend3D>,
};
#[cfg(not(feature = "rtx-nvidia"))]
let mut backend = FallbackBackend::default();

let mesh_idx = backend.upload_mesh(&RtxMesh { positions: vec![], normals: vec![], uvs: vec![], indices: vec![] })?;
backend.render(RtxCamera::default(), &[RtxDraw { mesh_index: mesh_idx, transform: Mat4::IDENTITY }])?;
```

## Roadmap Vulkan RT
1. `Instance` + `Device` con `VK_KHR_acceleration_structure` / `VK_KHR_ray_tracing_pipeline` / `VK_KHR_deferred_host_operations` — **hecho** (`vulkan_rt.rs:probe`)
2. BLAS por mesh, TLAS por frame (`vkBuildAccelerationStructuresKHR`)
3. Pipeline `raygen.rgen` / `miss.rmiss` / `closesthit.rchit` (GLSL -> SPIR-V via `shaderc`)
4. `vkCmdTraceRaysKHR` -> storage image -> blit a swapchain
5. DLSS opcional

Sin `rtx-nvidia` el crate es no-op y el engine usa `velvet-render::Wgpu3DRenderer`.

## Requisitos
- GPU Nvidia RTX, driver >= 470, Vulkan SDK
- `cargo build -p velvet-render-rtx --features rtx-nvidia`
