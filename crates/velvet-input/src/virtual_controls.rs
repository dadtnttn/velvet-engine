//! On-screen virtual stick and button state (touch / mobile).

use velvet_math::Vec2;

/// Virtual button (on-screen).
#[derive(Debug, Clone, PartialEq)]
pub struct VirtualButton {
    /// Debug name / action mapping key.
    pub name: String,
    /// Center in screen space.
    pub center: Vec2,
    /// Hit radius.
    pub radius: f32,
    /// Currently pressed.
    pub pressed: bool,
    /// Pressed this frame.
    pub just_pressed: bool,
    /// Released this frame.
    pub just_released: bool,
}

impl VirtualButton {
    /// Create button.
    pub fn new(name: impl Into<String>, center: Vec2, radius: f32) -> Self {
        Self {
            name: name.into(),
            center,
            radius: radius.max(1.0),
            pressed: false,
            just_pressed: false,
            just_released: false,
        }
    }

    /// Whether a screen point is inside the button.
    pub fn contains(&self, point: Vec2) -> bool {
        self.center.distance(point) <= self.radius
    }
}

/// Virtual analog stick.
#[derive(Debug, Clone, PartialEq)]
pub struct VirtualStick {
    /// Name.
    pub name: String,
    /// Base center (rest position).
    pub base: Vec2,
    /// Max visual / input radius.
    pub radius: f32,
    /// Deadzone as fraction of radius `0..=1`.
    pub deadzone: f32,
    /// Whether a pointer is grabbing the stick.
    pub active: bool,
    /// Current pointer id if any.
    pub pointer_id: Option<u64>,
    /// Knob offset from base (clamped to radius).
    pub offset: Vec2,
}

impl VirtualStick {
    /// Create stick.
    pub fn new(name: impl Into<String>, base: Vec2, radius: f32) -> Self {
        Self {
            name: name.into(),
            base,
            radius: radius.max(1.0),
            deadzone: 0.15,
            active: false,
            pointer_id: None,
            offset: Vec2::ZERO,
        }
    }

    /// Normalized axis `-1..=1` after deadzone.
    pub fn axis(&self) -> Vec2 {
        let len = self.offset.length();
        if len < self.radius * self.deadzone {
            return Vec2::ZERO;
        }
        let n = self.offset / self.radius;
        Vec2::new(n.x.clamp(-1.0, 1.0), n.y.clamp(-1.0, 1.0))
    }

    /// Begin grab if point near base.
    pub fn begin(&mut self, pointer_id: u64, point: Vec2) -> bool {
        if point.distance(self.base) <= self.radius * 1.25 {
            self.active = true;
            self.pointer_id = Some(pointer_id);
            self.move_to(point);
            true
        } else {
            false
        }
    }

    /// Move knob toward point while active.
    pub fn move_to(&mut self, point: Vec2) {
        if !self.active {
            return;
        }
        let delta = point - self.base;
        self.offset = delta.clamp_length_max(self.radius);
    }

    /// End grab for pointer.
    pub fn end(&mut self, pointer_id: u64) {
        if self.pointer_id == Some(pointer_id) {
            self.active = false;
            self.pointer_id = None;
            self.offset = Vec2::ZERO;
        }
    }
}

/// Collection of virtual on-screen controls.
#[derive(Debug, Default, Clone)]
pub struct VirtualControls {
    /// Sticks.
    pub sticks: Vec<VirtualStick>,
    /// Buttons.
    pub buttons: Vec<VirtualButton>,
}

impl VirtualControls {
    /// Empty set.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a left-stick style control.
    pub fn add_stick(&mut self, stick: VirtualStick) {
        self.sticks.push(stick);
    }

    /// Add a button.
    pub fn add_button(&mut self, button: VirtualButton) {
        self.buttons.push(button);
    }

    /// Clear per-frame button edges (call at begin_frame).
    pub fn begin_frame(&mut self) {
        for b in &mut self.buttons {
            b.just_pressed = false;
            b.just_released = false;
        }
    }

    /// Pointer down at screen point.
    pub fn pointer_down(&mut self, pointer_id: u64, point: Vec2) {
        for stick in &mut self.sticks {
            if stick.begin(pointer_id, point) {
                return;
            }
        }
        for btn in &mut self.buttons {
            if btn.contains(point) {
                if !btn.pressed {
                    btn.just_pressed = true;
                }
                btn.pressed = true;
            }
        }
    }

    /// Pointer move.
    pub fn pointer_move(&mut self, pointer_id: u64, point: Vec2) {
        for stick in &mut self.sticks {
            if stick.pointer_id == Some(pointer_id) {
                stick.move_to(point);
            }
        }
    }

    /// Pointer up.
    pub fn pointer_up(&mut self, pointer_id: u64, point: Vec2) {
        for stick in &mut self.sticks {
            stick.end(pointer_id);
        }
        for btn in &mut self.buttons {
            if btn.pressed && btn.contains(point) {
                btn.just_released = true;
            }
            // Also release if this pointer was pressing (simplified: release all matching).
            if btn.pressed {
                // Clear press whether released inside or outside the button.
                btn.pressed = false;
            }
        }
    }

    /// Stick axis by name.
    pub fn stick_axis(&self, name: &str) -> Option<Vec2> {
        self.sticks
            .iter()
            .find(|s| s.name == name)
            .map(|s| s.axis())
    }

    /// Button pressed by name.
    pub fn button_pressed(&self, name: &str) -> bool {
        self.buttons
            .iter()
            .find(|b| b.name == name)
            .map(|b| b.pressed)
            .unwrap_or(false)
    }

    /// Button just pressed.
    pub fn button_just_pressed(&self, name: &str) -> bool {
        self.buttons
            .iter()
            .find(|b| b.name == name)
            .map(|b| b.just_pressed)
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stick_axis_and_deadzone() {
        let mut stick = VirtualStick::new("move", Vec2::new(100.0, 100.0), 50.0);
        stick.begin(1, Vec2::new(100.0, 100.0));
        stick.move_to(Vec2::new(102.0, 100.0)); // inside deadzone
        let a = stick.axis();
        assert!(a.length() < 1e-4);
        stick.move_to(Vec2::new(150.0, 100.0));
        let a = stick.axis();
        assert!(a.x > 0.9);
    }

    #[test]
    fn button_press_release() {
        let mut vc = VirtualControls::new();
        vc.add_button(VirtualButton::new("attack", Vec2::new(200.0, 200.0), 30.0));
        vc.begin_frame();
        vc.pointer_down(1, Vec2::new(200.0, 200.0));
        assert!(vc.button_just_pressed("attack"));
        assert!(vc.button_pressed("attack"));
        vc.begin_frame();
        vc.pointer_up(1, Vec2::new(200.0, 200.0));
        assert!(!vc.button_pressed("attack"));
    }
}
