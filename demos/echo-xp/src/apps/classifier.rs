#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnomalyCategory {
    NoAnomaly,
    IdentityDrift,
    MandelaClassIntrusion,
    DoppelgangerEvent,
    InsufficientEvidence,
}

impl AnomalyCategory {
    pub const ALL: [AnomalyCategory; 5] = [
        AnomalyCategory::NoAnomaly,
        AnomalyCategory::IdentityDrift,
        AnomalyCategory::MandelaClassIntrusion,
        AnomalyCategory::DoppelgangerEvent,
        AnomalyCategory::InsufficientEvidence,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            AnomalyCategory::NoAnomaly => "NO ANOMALY",
            AnomalyCategory::IdentityDrift => "IDENTITY DRIFT",
            AnomalyCategory::MandelaClassIntrusion => "MANDELA-CLASS INTRUSION",
            AnomalyCategory::DoppelgangerEvent => "DOPPELGÄNGER EVENT",
            AnomalyCategory::InsufficientEvidence => "INSUFFICIENT EVIDENCE",
        }
    }

    pub fn vs3_string(&self) -> &'static str {
        match self {
            AnomalyCategory::NoAnomaly => "NO_ANOMALY",
            AnomalyCategory::IdentityDrift => "IDENTITY_DRIFT",
            AnomalyCategory::MandelaClassIntrusion => "MANDELA-CLASS INTRUSION",
            AnomalyCategory::DoppelgangerEvent => "DOPPELGANGER_EVENT",
            AnomalyCategory::InsufficientEvidence => "INSUFFICIENT_EVIDENCE",
        }
    }
}

pub struct ClassifierApp {
    pub selected_category: Option<AnomalyCategory>,
    pub feedback_message: String,
}

impl ClassifierApp {
    pub fn new() -> Self {
        Self {
            selected_category: Some(AnomalyCategory::MandelaClassIntrusion),
            feedback_message: String::new(),
        }
    }
}
