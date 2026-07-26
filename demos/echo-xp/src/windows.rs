use crate::model::{AppKind, Rect};

#[derive(Debug, Clone)]
pub struct DesktopWindow {
    pub id: u32,
    pub app: AppKind,
    pub title: String,
    pub rect: Rect,
    pub is_minimized: bool,
    pub is_focused: bool,
    pub z_index: u32,
    pub is_dragging: bool,
    pub drag_offset_x: i32,
    pub drag_offset_y: i32,
}

pub struct WindowManager {
    pub windows: Vec<DesktopWindow>,
    next_id: u32,
    next_z: u32,
}

impl WindowManager {
    pub fn new() -> Self {
        Self {
            windows: Vec::new(),
            next_id: 1,
            next_z: 1,
        }
    }

    pub fn open_window(&mut self, app: AppKind, title: impl Into<String>, rect: Rect) -> u32 {
        let existing_id = self.windows.iter().find(|w| w.app == app).map(|w| w.id);
        if let Some(id) = existing_id {
            if let Some(win) = self.windows.iter_mut().find(|w| w.id == id) {
                win.is_minimized = false;
            }
            self.focus_window(id);
            return id;
        }

        let id = self.next_id;
        self.next_id += 1;

        let z = self.next_z;
        self.next_z += 1;

        // Unfocus other windows
        for win in &mut self.windows {
            win.is_focused = false;
        }

        self.windows.push(DesktopWindow {
            id,
            app,
            title: title.into(),
            rect,
            is_minimized: false,
            is_focused: true,
            z_index: z,
            is_dragging: false,
            drag_offset_x: 0,
            drag_offset_y: 0,
        });

        id
    }

    pub fn close_window(&mut self, id: u32) {
        self.windows.retain(|w| w.id != id);
        if let Some(top) = self.top_window_id() {
            self.focus_window(top);
        }
    }

    pub fn close_app(&mut self, app: AppKind) {
        self.windows.retain(|w| w.app != app);
        if let Some(top) = self.top_window_id() {
            self.focus_window(top);
        }
    }

    pub fn toggle_minimize(&mut self, id: u32) {
        if let Some(win) = self.windows.iter_mut().find(|w| w.id == id) {
            win.is_minimized = !win.is_minimized;
            if !win.is_minimized {
                self.focus_window(id);
            }
        }
    }

    pub fn focus_window(&mut self, id: u32) {
        let z = self.next_z;
        self.next_z += 1;

        for win in &mut self.windows {
            if win.id == id {
                win.is_focused = true;
                win.z_index = z;
            } else {
                win.is_focused = false;
            }
        }
    }

    pub fn top_window_id(&self) -> Option<u32> {
        self.windows
            .iter()
            .filter(|w| !w.is_minimized)
            .max_by_key(|w| w.z_index)
            .map(|w| w.id)
    }

    pub fn get_window(&self, id: u32) -> Option<&DesktopWindow> {
        self.windows.iter().find(|w| w.id == id)
    }

    pub fn get_window_mut(&mut self, id: u32) -> Option<&mut DesktopWindow> {
        self.windows.iter_mut().find(|w| w.id == id)
    }

    pub fn is_open(&self, app: AppKind) -> bool {
        self.windows.iter().any(|w| w.app == app)
    }
}
