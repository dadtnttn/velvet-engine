use super::value::{optional_field, string};
use anyhow::Result;
use velvet_script_vs3::Value;
#[derive(Debug, Clone)]
pub struct RuntimeEvent {
    pub kind: String,
    pub payload: Option<Value>,
}
impl RuntimeEvent {
    pub fn parse(value: &Value) -> Result<Self> {
        Ok(Self {
            kind: string(value, "kind")?,
            payload: optional_field(value, "payload")?,
        })
    }
}
