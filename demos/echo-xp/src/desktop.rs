use crate::model::{AppKind, Rect};

#[derive(Debug, Clone)]
pub struct DesktopIcon {
    pub id: &'static str,
    pub label: &'static str,
    pub app: AppKind,
    pub icon_idx: u32,
    pub rect: Rect,
    pub selected: bool,
}

pub struct DesktopShell {
    pub icons: Vec<DesktopIcon>,
    pub start_menu_open: bool,
    pub selected_icon: Option<&'static str>,
}

impl DesktopShell {
    pub fn new() -> Self {
        let icons = vec![
            DesktopIcon {
                id: "inbox",
                label: "Inbox",
                app: AppKind::Inbox,
                icon_idx: 1, // Email icon
                rect: Rect::new(20, 20, 64, 64),
                selected: false,
            },
            DesktopIcon {
                id: "case_files",
                label: "Case Files",
                app: AppKind::CaseFiles,
                icon_idx: 2, // Folder/dossier icon
                rect: Rect::new(20, 100, 64, 64),
                selected: false,
            },
            DesktopIcon {
                id: "photo_viewer",
                label: "Evidence Photo",
                app: AppKind::PhotoViewer,
                icon_idx: 3, // Picture icon
                rect: Rect::new(20, 180, 64, 64),
                selected: false,
            },
            DesktopIcon {
                id: "tape_player",
                label: "Tape Player",
                app: AppKind::TapePlayer,
                icon_idx: 4, // Media/audio icon
                rect: Rect::new(20, 260, 64, 64),
                selected: false,
            },
            DesktopIcon {
                id: "classifier",
                label: "Classifier",
                app: AppKind::Classifier,
                icon_idx: 5, // Shield/classifier icon
                rect: Rect::new(20, 340, 64, 64),
                selected: false,
            },
            DesktopIcon {
                id: "recycle_bin",
                label: "Recycle Bin",
                app: AppKind::RecycleBin,
                icon_idx: 6, // Recycle bin icon
                rect: Rect::new(20, 420, 64, 64),
                selected: false,
            },
        ];

        Self {
            icons,
            start_menu_open: false,
            selected_icon: None,
        }
    }

    pub fn select_icon(&mut self, id: Option<&'static str>) {
        self.selected_icon = id;
        for icon in &mut self.icons {
            icon.selected = Some(icon.id) == id;
        }
    }
}
