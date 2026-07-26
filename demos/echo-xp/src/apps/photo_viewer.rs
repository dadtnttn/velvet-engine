pub struct PhotoViewerApp {
    pub zoom_level: f32,
    pub inspected: bool,
}

impl PhotoViewerApp {
    pub fn new() -> Self {
        Self {
            zoom_level: 1.0,
            inspected: false,
        }
    }

    pub fn zoom_in(&mut self) {
        if self.zoom_level < 2.5 {
            self.zoom_level += 0.5;
        }
    }

    pub fn zoom_out(&mut self) {
        if self.zoom_level > 1.0 {
            self.zoom_level -= 0.5;
        }
    }
}
