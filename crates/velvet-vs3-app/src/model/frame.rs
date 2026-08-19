use super::value::{optional_field, optional_list, optional_number};
use super::{RuntimeEvent, VisualCommandView};
use anyhow::{anyhow, Result};
use velvet_script_vs3::Value;
#[derive(Debug, Clone, Default)]
pub struct FrameView {
    pub time: f32,
    pub commands: Vec<VisualCommandView>,
    pub events: Vec<RuntimeEvent>,
}
impl FrameView {
    pub fn parse(value: &Value) -> Result<Self> {
        let commands_key = if optional_field(value, "commands")?.is_some() {
            "commands"
        } else {
            "visuals"
        };
        let commands = optional_list(value, commands_key, VisualCommandView::parse)?;
        if commands_key == "visuals" && commands.is_empty() {
            return Err(anyhow!("frame packet has no `commands` list"));
        }
        Ok(Self {
            time: optional_number(value, "time"),
            commands,
            events: optional_list(value, "events", RuntimeEvent::parse)?,
        })
    }
}
