//! La cámara isométrica del juego: su ajuste (`ISO`) y el encuadre de cada lugar. El giro por pasos,
//! la elevación, el zoom, el suavizado y el recorte de lo que tapa a la Reina son del motor
//! (`velvet_render::IsoCamera3D` e `IsoSettings3D`).
use super::*;

/// The game's isometric view: true isometric, 15° turns, a zoom of 3.8 to 18 m, aimed at her
/// chest.
pub(super) const ISO: IsoSettings3D = IsoSettings3D {
    step: std::f32::consts::PI / 12.0,
    elevation: 0.615_479_7,
    top_down_elevation: 1.42,
    min_distance: 3.8,
    max_distance: 18.0,
    orbit: 18.0,
    focus_height: 0.78,
    ease: 16.0,
};

impl App {
    pub(super) fn camera(&self) -> Camera3D {
        if let Some(camera) = self.camera_override {
            return camera;
        }
        if let Some(camera) = self.drink_camera() {
            return camera;
        }
        if let Some(camera) = self.alcove_camera() {
            return camera;
        }
        if self.dungeon_mode {
            return self.dungeon.camera();
        }
        if let Some(fight) = &self.boss {
            return boss_camera(fight);
        }
        // The Queen is the camera anchor in both scenes.  The old nature
        // composition applied a large X/Z offset here; it looked attractive
        // in a wide screenshot but meant Q zoomed toward empty terrain and
        // pushed the Queen out of frame.  Keep only a small vertical offset
        // (ISO's focus height) so the target sits around her chest instead of at her feet.
        let mut camera = self.iso.camera(self.camera_focus);
        // Outdoors the camera stands 18 m from her, lower than the tallest crowns: an
        // orthographic view keeps what stands behind its plane (nothing grows there but the
        // wood), so trees between her and the camera are drawn whole instead of sliced open.
        camera.near = if self.nature_mode { -30.0 } else { 0.1 };
        // Keep the extended meadow and its horizon props in the depth range;
        // fog still softens the far layer so it reads as an endless landscape.
        camera.far = 86.0;
        camera
    }
}
