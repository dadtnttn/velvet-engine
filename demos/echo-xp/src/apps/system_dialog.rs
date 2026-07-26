#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogKind {
    Info,
    Warning,
    Error,
    Critical,
}

#[derive(Debug, Clone)]
pub struct SystemDialogApp {
    pub title: String,
    pub message: String,
    pub kind: DialogKind,
    pub visible: bool,
}

impl SystemDialogApp {
    pub fn new(title: impl Into<String>, message: impl Into<String>, kind: DialogKind) -> Self {
        Self {
            title: title.into(),
            message: message.into(),
            kind,
            visible: true,
        }
    }
}
