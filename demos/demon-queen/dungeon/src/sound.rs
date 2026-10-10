//! What the dungeon sounds like, as events: gameplay records a sound and
//! where it happened; the game's audio plays it (spatialised, with the
//! cave's echo). Keeping this data-only lets tests and the lab run silent.
use velvet_math::Vec3;

/// Every sound the dungeon makes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Sfx {
    // The Queen.
    Footstep,
    Jump,
    Land,
    Swing,
    PlayerHurt,
    Drink,
    Respawn,
    // Hits and deaths.
    HitBone,
    BoneCrack,
    BoneRattle,
    SkeletonDeath,
    HitSlime,
    SlimeHop,
    SlimeDeath,
    // Foes' attacks.
    EnemySwing,
    CrossbowShot,
    BoltImpact,
    SpellCharge,
    SpellBurst,
    Shockwave,
    Roar,
    Summon,
    Rubble,
    // The Heart.
    HeartBeat,
    // Treasure and puzzles.
    ChestCreak,
    Coins,
    Pickup,
    Relic,
    RuneChime,
    SealOpen,
    // Ambience (the game adds drips on its own).
    Drip,
    // Steps above ground, by the ground.
    StepGrass,
    StepCobble,
    StepHall,
    StepWood,
    /// Wading: a leg pushing through warm water.
    StepWater,
}

impl Sfx {
    pub const ALL: [Sfx; 36] = [
        Sfx::Footstep,
        Sfx::Jump,
        Sfx::Land,
        Sfx::Swing,
        Sfx::PlayerHurt,
        Sfx::Drink,
        Sfx::Respawn,
        Sfx::HitBone,
        Sfx::BoneCrack,
        Sfx::BoneRattle,
        Sfx::SkeletonDeath,
        Sfx::HitSlime,
        Sfx::SlimeHop,
        Sfx::SlimeDeath,
        Sfx::EnemySwing,
        Sfx::CrossbowShot,
        Sfx::BoltImpact,
        Sfx::SpellCharge,
        Sfx::SpellBurst,
        Sfx::Shockwave,
        Sfx::Roar,
        Sfx::Summon,
        Sfx::Rubble,
        Sfx::HeartBeat,
        Sfx::ChestCreak,
        Sfx::Coins,
        Sfx::Pickup,
        Sfx::Relic,
        Sfx::RuneChime,
        Sfx::SealOpen,
        Sfx::Drip,
        Sfx::StepGrass,
        Sfx::StepCobble,
        Sfx::StepHall,
        Sfx::StepWood,
        Sfx::StepWater,
    ];

    /// File name a recorded sound may replace it with (`data/audio/<name>.wav`
    /// or `.ogg`).
    pub fn name(self) -> &'static str {
        match self {
            Sfx::Footstep => "footstep",
            Sfx::Jump => "jump",
            Sfx::Land => "land",
            Sfx::Swing => "sword_swing",
            Sfx::PlayerHurt => "player_hurt",
            Sfx::Drink => "drink",
            Sfx::Respawn => "respawn",
            Sfx::HitBone => "hit_bone",
            Sfx::BoneCrack => "bone_crack",
            Sfx::BoneRattle => "bone_rattle",
            Sfx::SkeletonDeath => "skeleton_death",
            Sfx::HitSlime => "hit_slime",
            Sfx::SlimeHop => "slime_hop",
            Sfx::SlimeDeath => "slime_death",
            Sfx::EnemySwing => "enemy_swing",
            Sfx::CrossbowShot => "crossbow_shot",
            Sfx::BoltImpact => "bolt_impact",
            Sfx::SpellCharge => "spell_charge",
            Sfx::SpellBurst => "spell_burst",
            Sfx::Shockwave => "shockwave",
            Sfx::Roar => "roar",
            Sfx::Summon => "summon",
            Sfx::Rubble => "rubble",
            Sfx::HeartBeat => "heart_beat",
            Sfx::ChestCreak => "chest_creak",
            Sfx::Coins => "coins",
            Sfx::Pickup => "pickup",
            Sfx::Relic => "relic",
            Sfx::RuneChime => "rune_chime",
            Sfx::SealOpen => "seal_open",
            Sfx::Drip => "drip",
            Sfx::StepGrass => "step_grass",
            Sfx::StepCobble => "step_cobble",
            Sfx::StepHall => "step_hall",
            Sfx::StepWood => "step_wood",
            Sfx::StepWater => "step_water",
        }
    }
}

/// One sound: what, where (`None` is at the listener), how loud (0..1).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sound {
    pub sfx: Sfx,
    pub at: Option<Vec3>,
    pub volume: f32,
}
