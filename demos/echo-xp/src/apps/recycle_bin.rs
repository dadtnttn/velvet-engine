pub struct RecycleBinItem {
    pub name: &'static str,
    pub original_location: &'static str,
    pub date_deleted: &'static str,
    pub size_kb: u32,
    pub restored: bool,
}

pub struct RecycleBinApp {
    pub items: Vec<RecycleBinItem>,
    pub selected_idx: Option<usize>,
}

impl RecycleBinApp {
    pub fn new() -> Self {
        let items = vec![RecycleBinItem {
            name: "ELISA_V_2004.tmp",
            original_location: "C:\\Archives\\Case001\\Drafts",
            date_deleted: "13/07/2004 23:59",
            size_kb: 14,
            restored: false,
        }];

        Self {
            items,
            selected_idx: Some(0),
        }
    }
}
