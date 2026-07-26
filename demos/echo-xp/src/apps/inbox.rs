use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct MailItem {
    pub id: String,
    pub from: String,
    pub subject: String,
    pub date: String,
    pub body: String,
    pub unread: bool,
}

pub struct InboxApp {
    pub mails: Vec<MailItem>,
    pub selected_idx: usize,
}

impl InboxApp {
    pub fn new() -> Self {
        let mails = vec![MailItem {
            id: "mail_001".to_string(),
            from: "Case Dispatch <archivist@echo.local>".to_string(),
            subject: "CASE 001 / MARA V.".to_string(),
            date: "14/07/2004 08:17".to_string(),
            body: "Operator,\n\nA continuity discrepancy has been detected in Residence C-17.\n\nReview the attached case record, photographic evidence and recovered audio.\nDo not contact the subject.\nDo not search for the second name outside this terminal.\n\nSubmit a classification when the record is complete.\n\n— ARCHIVE CONTROL"
                .to_string(),
            unread: true,
        }];

        Self {
            mails,
            selected_idx: 0,
        }
    }

    pub fn selected_mail(&self) -> Option<&MailItem> {
        self.mails.get(self.selected_idx)
    }
}
