//! RPG plugin (registers empty resource hooks; host owns Party).

use velvet_app::{App, Plugin};
use velvet_core::plugin::PluginError;

use crate::item::ItemDb;
use crate::party::Party;
use crate::quest::QuestJournal;

/// Ensures default RPG resources exist.
#[derive(Default)]
pub struct RpgPlugin;

impl Plugin for RpgPlugin {
    fn name(&self) -> &'static str {
        "velvet.rpg"
    }

    fn build(&self, app: &mut App) -> Result<(), PluginError> {
        if app.resource::<Party>().is_none() {
            app.insert_resource(Party::default());
        }
        if app.resource::<QuestJournal>().is_none() {
            app.insert_resource(QuestJournal::default());
        }
        if app.resource::<ItemDb>().is_none() {
            app.insert_resource(ItemDb::default());
        }
        Ok(())
    }
}
