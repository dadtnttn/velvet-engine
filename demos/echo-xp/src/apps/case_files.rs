use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct CaseFile {
    pub id: String,
    pub title: String,
    pub subject_name: String,
    pub subject_id: String,
    pub registered_age: u32,
    pub status: String,
    pub last_location: String,
    pub incident_date: String,
    pub initial_classification: String,
    pub official_record: String,
    pub discrepancies: String,
    pub evidence_references: Vec<String>,
}

pub struct CaseFilesApp {
    pub files: Vec<CaseFile>,
    pub selected_idx: usize,
}

impl CaseFilesApp {
    pub fn new() -> Self {
        let files = vec![
            CaseFile {
                id: "CASE_001_MARA_V".to_string(),
                title: "CASE 001 — MARA V.".to_string(),
                subject_name: "Mara Valen".to_string(),
                subject_id: "MV-1407".to_string(),
                registered_age: 24,
                status: "Missing".to_string(),
                last_location: "Residence C-17".to_string(),
                incident_date: "14/07/2004".to_string(),
                initial_classification: "Pending".to_string(),
                official_record: "Official records state Mara Valen was an only child raised at Residence C-17.".to_string(),
                discrepancies: "Contradictory records recovered from site C-17 mention a sister named Elisa. Physical census logs list two occupants in 2003, but single occupancy in 2004 without death/relocation certificates.".to_string(),
                evidence_references: vec!["PHOTO_EVID_01.png".to_string(), "C17_RECOVERED_02.wav".to_string()],
            },
            CaseFile {
                id: "ARCHIVE_PROTOCOL".to_string(),
                title: "ARCHIVE PROTOCOL".to_string(),
                subject_name: "Protocol Standard".to_string(),
                subject_id: "AP-000".to_string(),
                registered_age: 0,
                status: "Active".to_string(),
                last_location: "Terminal".to_string(),
                incident_date: "01/01/2004".to_string(),
                initial_classification: "Standard".to_string(),
                official_record: "Inspect all evidence carefully before submitting classification. Discrepancies in temporal continuity indicate MANDELA-CLASS INTRUSION events.".to_string(),
                discrepancies: "Do not attempt to restore deleted local files without authorization.".to_string(),
                evidence_references: vec![],
            },
            CaseFile {
                id: "OPERATOR_GUIDE".to_string(),
                title: "OPERATOR GUIDE".to_string(),
                subject_name: "Guide".to_string(),
                subject_id: "OG-001".to_string(),
                registered_age: 0,
                status: "Active".to_string(),
                last_location: "Terminal".to_string(),
                incident_date: "01/01/2004".to_string(),
                initial_classification: "Standard".to_string(),
                official_record: "Double-click desktop icons to launch applications. Use Tape Player to inspect recovered recordings.".to_string(),
                discrepancies: "If terminal output degrades or alters user metadata, report immediately to Archive Control.".to_string(),
                evidence_references: vec![],
            },
        ];

        Self {
            files,
            selected_idx: 0,
        }
    }

    pub fn selected_file(&self) -> Option<&CaseFile> {
        self.files.get(self.selected_idx)
    }
}
