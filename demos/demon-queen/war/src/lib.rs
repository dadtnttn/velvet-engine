//! La guerra de la chispa: el «Total War» de Demon Queen.
//!
//! A turn-based campaign over the west of the Dark Continent (provinces, armies, gold, food and the
//! Queen's spark) and real-time battles of hundreds of soldiers per side, ported from the Three.js
//! prototype (`demos/demon-queen-war-threejs`). This crate holds what has no window: the data, the
//! battlefields, the simulation, the enemy general, the campaign's rules; and the meshes the game
//! draws them with. The game (`src/war/`) owns the camera, the input, the HUD and the sound.
pub mod ai;
pub mod battle;
pub mod campaign_map;
pub mod data;
pub mod field;
pub mod maps;
pub mod models;
pub mod noise;
pub mod prims;
pub mod rules;
pub mod scene;
pub mod sim;
