//! # velvet-qa
//!
//! The parts of an automatic QA run that no game owns: a walker that plays a zone the way a
//! player would (with the game's real controls and its real tick) and notes what goes wrong, and
//! the report it writes.
//!
//! - [`Survey`]: what a zone's navmesh says about it, seen from where the zone starts: the
//!   walkable and the reachable ground, and the islands nobody can get to.
//! - [`spread_targets`]: places to walk to, spread over the reachable ground.
//! - [`PathFollower`]: walking a way corner by corner, telling when the walker is no longer
//!   getting anywhere (a snag a player would hit).
//! - [`eight_way_keys`]: the keys that walk closest to a direction in a view that cannot turn.
//! - [`FrameTimes`]: how long frames take (mean, 95th percentile, worst) and where the slow
//!   ones were.
//! - [`ZoneReport`], [`Incident`] and [`write_report`]: what a run found, as JSON and Markdown;
//!   [`zone_map`] pictures a zone's reachable ground and where the walker got stuck.
//! - [`private_mb`]: the process's private memory, to hunt leaks.
//!
//! The game supplies the rest: its zones, how to enter each, where one can stand, how to hold
//! its keys, and its screenshots.
#![deny(missing_docs)]

mod frames;
mod memory;
mod report;
mod walk;

pub use frames::FrameTimes;
pub use memory::private_mb;
pub use report::{write_report, zone_map, Incident, ZoneReport};
pub use walk::{eight_way_keys, spread_targets, PathFollower, Progress, Survey};
