//! The armies of the Dark Continent: the factions and every regiment type, the quick battles and
//! the campaign's provinces, as plain data (the same numbers as the prototype's `data/*.js`).
//!
//! Stats are per soldier. `armor` (0..100) is the share of plain damage it stops; `ap` is the part
//! of a blow that goes through armour anyway. `shield` (0..100) is the chance to stop a missile from
//! the front, and adds to melee defence from the front. Speeds in m/s. Morale 999 = unbreakable.

/// The three powers of the war, and the free peoples who answer to none of them (they hold
/// their land with a militia and take no turn).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Faction {
    Reina,
    Alianza,
    Rebeldes,
    Libres,
}

impl Faction {
    pub const ALL: [Faction; 3] = [Faction::Reina, Faction::Alianza, Faction::Rebeldes];

    pub fn info(self) -> &'static FactionInfo {
        match self {
            Faction::Reina => &FACTIONS[0],
            Faction::Alianza => &FACTIONS[1],
            Faction::Rebeldes => &FACTIONS[2],
            Faction::Libres => &FACTIONS[3],
        }
    }

    pub fn id(self) -> &'static str {
        self.info().id
    }

    pub fn index(self) -> usize {
        self as usize
    }
}

pub struct FactionInfo {
    pub id: &'static str,
    pub name: &'static str,
    pub short: &'static str,
    pub kicker: &'static str,
    pub color: u32,
    pub bright: u32,
    pub accent: u32,
    pub dark: u32,
    pub cloth: u32,
    pub trim: u32,
}

pub static FACTIONS: [FactionInfo; 4] = [
    FactionInfo {
        id: "reina",
        name: "La Corte de la Reina",
        short: "La Reina",
        kicker: "PUEBLOS DE LA CHISPA",
        color: 0x9a5cff,
        bright: 0xc9a2ff,
        accent: 0xf2b54a,
        dark: 0x22113c,
        cloth: 0x6a32c0,
        trim: 0xe8b04a,
    },
    FactionInfo {
        id: "alianza",
        name: "La Alianza del Este",
        short: "La Alianza",
        kicker: "REINOS DEL ESTE",
        color: 0x4f8ff0,
        bright: 0xa8ccff,
        accent: 0xf4ecd2,
        dark: 0x10203e,
        cloth: 0xe9e4d4,
        trim: 0x3d6fd0,
    },
    FactionInfo {
        id: "rebeldes",
        name: "Los clanes del norte",
        short: "Los rebeldes",
        kicker: "REBELDES Y APAGADOS",
        color: 0xe0623a,
        bright: 0xffb08a,
        accent: 0xe8c890,
        dark: 0x34140a,
        cloth: 0xa8442a,
        trim: 0x3a2614,
    },
    FactionInfo {
        id: "libres",
        name: "Los pueblos libres",
        short: "Los libres",
        kicker: "SIN SEÑOR",
        color: 0x8fa08a,
        bright: 0xc8d6c0,
        accent: 0xd8c8a0,
        dark: 0x1e2620,
        cloth: 0x6f7f68,
        trim: 0x3a3226,
    },
];

/// A colour 0xRRGGBB as linear-ish floats 0..1 (as the prototype's THREE.Color read them).
pub fn rgb(hex: u32) -> [f32; 3] {
    let c = |shift: u32| {
        let s = ((hex >> shift) & 0xff) as f32 / 255.0;
        // three.js converts sRGB hex to linear
        if s <= 0.04045 {
            s / 12.92
        } else {
            ((s + 0.055) / 1.055).powf(2.4)
        }
    };
    [c(16), c(8), c(0)]
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Infantry,
    Spear,
    Skirmisher,
    Archer,
    Flyer,
    Cavalry,
    Support,
    Hero,
    Beast,
}

impl Role {
    pub fn name(self) -> &'static str {
        match self {
            Role::Infantry => "Infantería",
            Role::Spear => "Lanceros",
            Role::Skirmisher => "Hostigadores",
            Role::Archer => "Proyectiles",
            Role::Flyer => "Voladores",
            Role::Cavalry => "Caballería",
            Role::Support => "Apoyo",
            Role::Hero => "Héroe",
            Role::Beast => "Bestias",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Arc {
    Low,
    High,
    Beam,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Projectile {
    Javelin,
    Arrow,
    Bolt,
    Light,
}

#[derive(Clone, Copy, Debug)]
pub struct Ranged {
    pub range: f32,
    pub dmg: f32,
    pub ap: f32,
    pub reload: f32,
    pub ammo: u32,
    pub arc: Arc,
    pub speed: f32,
    pub spread: f32,
    pub projectile: Projectile,
    /// Spark the hit costs a spark regiment (the priests' light).
    pub cohesion: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuraKind {
    Queen,
    Dogma,
    Leader,
}

#[derive(Clone, Copy, Debug)]
pub struct Aura {
    pub kind: AuraKind,
    pub radius: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct Cost {
    pub gold: i32,
    pub spark: i32,
    pub food: i32,
}

/// One kind of regiment.
#[derive(Debug)]
pub struct UnitType {
    pub id: &'static str,
    /// How a message names it («La banda de orcos»).
    pub call: &'static str,
    /// One of them (a hero, «la banda»): the message's verb in the singular.
    pub sing: bool,
    pub faction: Faction,
    pub name: &'static str,
    pub short: &'static str,
    pub role: Role,
    /// Which crowd model draws it.
    pub model: &'static str,
    pub blurb: &'static str,
    pub men: u32,
    pub hp: f32,
    pub armor: f32,
    pub shield: f32,
    pub atk: f32,
    pub def: f32,
    pub dmg: f32,
    pub ap: f32,
    pub reach: f32,
    pub cd: f32,
    pub charge: f32,
    pub mass: f32,
    pub walk: f32,
    pub run: f32,
    pub morale: f32,
    pub anti_large: f32,
    pub large: bool,
    pub flyer: bool,
    /// Runs on the Queen's spark (dims away from her).
    pub spark: bool,
    /// Fearless while its leader stands (the dimmed round Varrak).
    pub follows_leader: bool,
    pub ranged: Option<Ranged>,
    pub aura: Option<Aura>,
    pub cleave: u32,
    pub scale: f32,
    pub radius: f32,
    pub spacing: f32,
    pub row_gap: f32,
    pub files: u32,
    pub cost: Cost,
}

const BASE: UnitType = UnitType {
    id: "",
    call: "",
    sing: false,
    faction: Faction::Reina,
    name: "",
    short: "",
    role: Role::Infantry,
    model: "",
    blurb: "",
    men: 60,
    hp: 10.0,
    armor: 0.0,
    shield: 0.0,
    atk: 8.0,
    def: 8.0,
    dmg: 4.0,
    ap: 0.0,
    reach: 1.0,
    cd: 1.4,
    charge: 4.0,
    mass: 1.0,
    walk: 3.0,
    run: 5.0,
    morale: 50.0,
    anti_large: 0.0,
    large: false,
    flyer: false,
    spark: false,
    follows_leader: false,
    ranged: None,
    aura: None,
    cleave: 1,
    scale: 1.0,
    radius: 0.45,
    spacing: 1.25,
    row_gap: 1.45,
    files: 16,
    cost: Cost {
        gold: 75,
        spark: 0,
        food: 1,
    },
};

const JAVELINS: Ranged = Ranged {
    range: 60.0,
    dmg: 6.0,
    ap: 2.0,
    reload: 5.0,
    ammo: 8,
    arc: Arc::Low,
    speed: 24.0,
    spread: 1.6,
    projectile: Projectile::Javelin,
    cohesion: 0.0,
};

pub static UNITS: [UnitType; 16] = [
    // ------------------------------------------------------------------ the Queen
    UnitType {
        id: "orcos",
        call: "La banda de orcos",
        sing: true,
        faction: Faction::Reina,
        name: "Banda de orcos",
        short: "Orcos",
        role: Role::Infantry,
        model: "orc",
        blurb: "Infantería de choque. Orcos con chispa: recuerdan, ríen y pegan muy fuerte.",
        men: 60,
        hp: 22.0,
        armor: 30.0,
        atk: 14.0,
        def: 9.0,
        dmg: 7.0,
        ap: 2.0,
        reach: 1.35,
        cd: 1.4,
        charge: 14.0,
        mass: 1.6,
        walk: 3.0,
        run: 5.6,
        morale: 62.0,
        spark: true,
        radius: 0.55,
        spacing: 1.45,
        row_gap: 1.6,
        files: 15,
        cost: Cost {
            gold: 105,
            spark: 2,
            food: 2,
        },
        ..BASE
    },
    UnitType {
        id: "trasgos",
        call: "Los hostigadores trasgos",
        faction: Faction::Reina,
        name: "Hostigadores trasgos",
        short: "Trasgos",
        role: Role::Skirmisher,
        model: "goblin",
        blurb: "Rápidos y ligeros. Lanzan jabalinas y no aguantan un cuerpo a cuerpo.",
        men: 72,
        hp: 9.0,
        armor: 5.0,
        atk: 7.0,
        def: 7.0,
        dmg: 3.0,
        reach: 0.9,
        cd: 1.1,
        charge: 4.0,
        mass: 0.6,
        walk: 3.6,
        run: 6.8,
        morale: 40.0,
        spark: true,
        radius: 0.36,
        spacing: 1.5,
        row_gap: 1.6,
        files: 18,
        ranged: Some(JAVELINS),
        cost: Cost {
            gold: 65,
            spark: 1,
            food: 1,
        },
        ..BASE
    },
    UnitType {
        id: "arpias",
        call: "Las harpías",
        faction: Faction::Reina,
        name: "Harpías del Nido",
        short: "Harpías",
        role: Role::Flyer,
        model: "harpy",
        blurb: "Vuelan por encima de las líneas y del río. Ideales para cazar arqueros y atacar por la espalda.",
        men: 36,
        hp: 12.0,
        armor: 5.0,
        atk: 12.0,
        def: 6.0,
        dmg: 5.0,
        ap: 1.0,
        reach: 1.25,
        cd: 1.0,
        charge: 16.0,
        mass: 0.9,
        walk: 6.5,
        run: 12.0,
        morale: 46.0,
        spark: true,
        flyer: true,
        radius: 0.5,
        spacing: 2.4,
        row_gap: 2.4,
        files: 9,
        cost: Cost {
            gold: 120,
            spark: 2,
            food: 1,
        },
        ..BASE
    },
    UnitType {
        id: "lamias",
        call: "Las lanzas lamias",
        faction: Faction::Reina,
        name: "Lanzas lamias",
        short: "Lamias",
        role: Role::Spear,
        model: "lamia",
        blurb: "Lanzas largas sobre cola de serpiente. Aguantan cargas: los caballos se clavan en ellas.",
        men: 56,
        hp: 18.0,
        armor: 22.0,
        atk: 10.0,
        def: 13.0,
        dmg: 5.0,
        ap: 1.0,
        reach: 2.3,
        cd: 1.3,
        charge: 6.0,
        mass: 1.4,
        walk: 2.9,
        run: 5.0,
        morale: 56.0,
        spark: true,
        anti_large: 14.0,
        radius: 0.55,
        spacing: 1.45,
        row_gap: 2.2,
        files: 14,
        cost: Cost {
            gold: 95,
            spark: 2,
            food: 1,
        },
        ..BASE
    },
    UnitType {
        id: "esqueletos",
        call: "Los arqueros del canciller",
        faction: Faction::Reina,
        name: "Arqueros del canciller",
        short: "Esqueletos",
        role: Role::Archer,
        model: "skeleton",
        blurb: "Los muertos de Osario. No tienen chispa que perder ni miedo: nunca huyen, se deshacen.",
        men: 72,
        hp: 10.0,
        armor: 10.0,
        atk: 6.0,
        def: 5.0,
        dmg: 3.0,
        reach: 0.9,
        cd: 1.4,
        charge: 2.0,
        mass: 0.8,
        walk: 2.6,
        run: 4.4,
        morale: 999.0,
        radius: 0.4,
        spacing: 1.3,
        row_gap: 1.5,
        files: 18,
        ranged: Some(Ranged {
            range: 155.0,
            dmg: 5.0,
            ap: 1.0,
            reload: 6.5,
            ammo: 18,
            arc: Arc::High,
            speed: 30.0,
            spread: 2.6,
            projectile: Projectile::Arrow,
            cohesion: 0.0,
        }),
        cost: Cost {
            gold: 75,
            spark: 0,
            food: 0,
        },
        ..BASE
    },
    UnitType {
        id: "reina",
        call: "La Reina",
        sing: true,
        faction: Faction::Reina,
        name: "La Reina",
        short: "La Reina",
        role: Role::Hero,
        model: "queen",
        blurb: "La Reina Demonio. Su chispa da mente a sus pueblos. Habilidad: «Chispa» (F).",
        men: 1,
        hp: 650.0,
        armor: 45.0,
        atk: 28.0,
        def: 24.0,
        dmg: 20.0,
        ap: 10.0,
        reach: 2.4,
        cd: 1.1,
        charge: 26.0,
        mass: 7.0,
        walk: 4.0,
        run: 7.4,
        morale: 999.0,
        large: true,
        cleave: 4,
        radius: 0.9,
        spacing: 2.0,
        row_gap: 2.0,
        files: 1,
        aura: Some(Aura {
            kind: AuraKind::Queen,
            radius: 70.0,
        }),
        cost: Cost {
            gold: 0,
            spark: 0,
            food: 0,
        },
        ..BASE
    },
    // --------------------------------------------------------------- the Alliance
    UnitType {
        id: "lanceros",
        call: "Los lanceros de la Marca",
        faction: Faction::Alianza,
        name: "Lanceros de la Marca",
        short: "Lanceros",
        role: Role::Spear,
        model: "spearman",
        blurb: "Escudo grande y lanza larga. Lentos, tozudos, y la pesadilla de cualquier carga.",
        men: 72,
        hp: 16.0,
        armor: 34.0,
        shield: 35.0,
        atk: 9.0,
        def: 14.0,
        dmg: 5.0,
        ap: 1.0,
        reach: 2.1,
        cd: 1.4,
        charge: 6.0,
        mass: 1.1,
        walk: 2.8,
        run: 5.0,
        morale: 55.0,
        anti_large: 14.0,
        radius: 0.45,
        spacing: 1.25,
        row_gap: 1.45,
        files: 18,
        cost: Cost {
            gold: 85,
            spark: 0,
            food: 1,
        },
        ..BASE
    },
    UnitType {
        id: "caballeros",
        call: "Los caballeros del Alba",
        faction: Faction::Alianza,
        name: "Caballeros del Alba",
        short: "Caballeros",
        role: Role::Cavalry,
        model: "knight",
        blurb: "Caballería pesada. Su carga lo arrolla todo; en un cuerpo a cuerpo largo se atascan.",
        men: 30,
        hp: 42.0,
        armor: 55.0,
        shield: 20.0,
        atk: 15.0,
        def: 10.0,
        dmg: 8.0,
        ap: 4.0,
        reach: 1.7,
        cd: 1.3,
        charge: 40.0,
        mass: 5.0,
        walk: 4.6,
        run: 12.5,
        morale: 66.0,
        large: true,
        radius: 0.9,
        spacing: 2.3,
        row_gap: 3.6,
        files: 10,
        cost: Cost {
            gold: 155,
            spark: 0,
            food: 2,
        },
        ..BASE
    },
    UnitType {
        id: "ballesteros",
        call: "Los ballesteros",
        faction: Faction::Alianza,
        name: "Ballesteros",
        short: "Ballesteros",
        role: Role::Archer,
        model: "crossbow",
        blurb: "Virotes pesados que atraviesan armaduras. Recargan despacio.",
        men: 56,
        hp: 12.0,
        armor: 24.0,
        atk: 6.0,
        def: 6.0,
        dmg: 3.0,
        reach: 1.0,
        cd: 1.4,
        charge: 2.0,
        mass: 1.0,
        walk: 2.8,
        run: 5.0,
        morale: 45.0,
        radius: 0.42,
        spacing: 1.3,
        row_gap: 1.5,
        files: 16,
        ranged: Some(Ranged {
            range: 135.0,
            dmg: 10.0,
            ap: 6.0,
            reload: 9.0,
            ammo: 14,
            arc: Arc::Low,
            speed: 55.0,
            spread: 2.0,
            projectile: Projectile::Bolt,
            cohesion: 0.0,
        }),
        cost: Cost {
            gold: 90,
            spark: 0,
            food: 1,
        },
        ..BASE
    },
    UnitType {
        id: "sacerdotes",
        call: "Los sacerdotes del Dogma",
        faction: Faction::Alianza,
        name: "Sacerdotes del Dogma",
        short: "Sacerdotes",
        role: Role::Support,
        model: "priest",
        blurb: "Su fe sostiene a la Alianza. «Las bestias no tienen alma»: cerca de ellos, la chispa se apaga antes.",
        men: 20,
        hp: 14.0,
        armor: 10.0,
        atk: 5.0,
        def: 6.0,
        dmg: 3.0,
        reach: 1.0,
        cd: 1.6,
        charge: 2.0,
        mass: 1.0,
        walk: 2.8,
        run: 4.8,
        morale: 62.0,
        radius: 0.45,
        spacing: 1.6,
        row_gap: 1.8,
        files: 10,
        ranged: Some(Ranged {
            range: 75.0,
            dmg: 6.0,
            ap: 4.0,
            reload: 7.0,
            ammo: 30,
            arc: Arc::Beam,
            speed: 60.0,
            spread: 1.4,
            projectile: Projectile::Light,
            cohesion: 6.0,
        }),
        aura: Some(Aura {
            kind: AuraKind::Dogma,
            radius: 38.0,
        }),
        cost: Cost {
            gold: 110,
            spark: 0,
            food: 1,
        },
        ..BASE
    },
    // -------------------------------------------------------- the northern clans
    UnitType {
        id: "clan_orcos",
        call: "Los orcos del clan",
        faction: Faction::Rebeldes,
        name: "Orcos del clan",
        short: "Orcos del clan",
        role: Role::Infantry,
        model: "orc",
        blurb: "Orcos con chispa que ya no obedecen. Creen que la Reina está acabada.",
        men: 60,
        hp: 22.0,
        armor: 26.0,
        atk: 14.0,
        def: 8.0,
        dmg: 7.0,
        ap: 2.0,
        reach: 1.35,
        cd: 1.4,
        charge: 14.0,
        mass: 1.6,
        walk: 3.0,
        run: 5.6,
        morale: 52.0,
        radius: 0.55,
        spacing: 1.45,
        row_gap: 1.6,
        files: 15,
        cost: Cost {
            gold: 100,
            spark: 0,
            food: 2,
        },
        ..BASE
    },
    UnitType {
        id: "clan_trasgos",
        call: "Los trasgos del clan",
        faction: Faction::Rebeldes,
        name: "Trasgos del clan",
        short: "Trasgos del clan",
        role: Role::Skirmisher,
        model: "goblin",
        blurb: "Hostigadores del norte, con hondas y jabalinas.",
        men: 72,
        hp: 9.0,
        armor: 5.0,
        atk: 7.0,
        def: 7.0,
        dmg: 3.0,
        reach: 0.9,
        cd: 1.1,
        charge: 4.0,
        mass: 0.6,
        walk: 3.6,
        run: 6.8,
        morale: 36.0,
        radius: 0.36,
        spacing: 1.5,
        row_gap: 1.6,
        files: 18,
        ranged: Some(JAVELINS),
        cost: Cost {
            gold: 60,
            spark: 0,
            food: 1,
        },
        ..BASE
    },
    UnitType {
        id: "lobos",
        call: "Los lobos apagados",
        faction: Faction::Rebeldes,
        name: "Lobos apagados",
        short: "Lobos",
        role: Role::Beast,
        model: "wolf",
        blurb: "Manada sin chispa. Rápidos, rodean y muerden por los flancos.",
        men: 32,
        hp: 18.0,
        armor: 5.0,
        atk: 13.0,
        def: 6.0,
        dmg: 5.0,
        reach: 1.15,
        cd: 0.9,
        charge: 18.0,
        mass: 1.7,
        walk: 5.0,
        run: 11.5,
        morale: 40.0,
        radius: 0.6,
        spacing: 1.9,
        row_gap: 2.6,
        files: 8,
        cost: Cost {
            gold: 75,
            spark: 0,
            food: 1,
        },
        ..BASE
    },
    UnitType {
        id: "apagados",
        call: "Los apagados",
        faction: Faction::Rebeldes,
        name: "Apagados",
        short: "Apagados",
        role: Role::Infantry,
        model: "dimmed",
        blurb: "Orcos y trasgos que perdieron la chispa. No sienten miedo mientras su jefe siga en pie.",
        men: 80,
        hp: 14.0,
        armor: 0.0,
        atk: 10.0,
        def: 4.0,
        dmg: 5.0,
        reach: 1.15,
        cd: 1.2,
        charge: 10.0,
        mass: 1.1,
        walk: 3.2,
        run: 6.0,
        morale: 999.0,
        follows_leader: true,
        radius: 0.5,
        spacing: 1.35,
        row_gap: 1.5,
        files: 16,
        cost: Cost {
            gold: 40,
            spark: 0,
            food: 1,
        },
        ..BASE
    },
    UnitType {
        id: "varrak",
        call: "Varrak",
        sing: true,
        faction: Faction::Rebeldes,
        name: "Varrak, el Colmillo",
        short: "Varrak",
        role: Role::Hero,
        model: "varrak",
        blurb: "El general de la Reina, apagado. Los apagados lo siguen. Despiértalo con la chispa cuando esté débil.",
        men: 1,
        hp: 760.0,
        armor: 40.0,
        atk: 26.0,
        def: 20.0,
        dmg: 22.0,
        ap: 8.0,
        reach: 2.6,
        cd: 1.3,
        charge: 30.0,
        mass: 8.0,
        walk: 3.6,
        run: 7.0,
        morale: 999.0,
        large: true,
        cleave: 4,
        radius: 1.0,
        spacing: 2.0,
        row_gap: 2.0,
        files: 1,
        aura: Some(Aura {
            kind: AuraKind::Leader,
            radius: 90.0,
        }),
        cost: Cost {
            gold: 0,
            spark: 0,
            food: 0,
        },
        ..BASE
    },
    // Varrak, awake again: the Queen's general (the campaign gives him back after the battle).
    UnitType {
        id: "varrak_reina",
        call: "Varrak",
        sing: true,
        faction: Faction::Reina,
        name: "Varrak, el Colmillo",
        short: "Varrak",
        role: Role::Hero,
        model: "varrak",
        blurb: "Despierto, débil y fiel. Juega fatal al ajedrez. Los suyos pelean mejor cerca de él.",
        men: 1,
        hp: 700.0,
        armor: 40.0,
        atk: 26.0,
        def: 20.0,
        dmg: 22.0,
        ap: 8.0,
        reach: 2.6,
        cd: 1.3,
        charge: 30.0,
        mass: 8.0,
        walk: 3.6,
        run: 7.0,
        morale: 999.0,
        large: true,
        cleave: 4,
        radius: 1.0,
        spacing: 2.0,
        row_gap: 2.0,
        files: 1,
        aura: Some(Aura {
            kind: AuraKind::Leader,
            radius: 70.0,
        }),
        cost: Cost {
            gold: 0,
            spark: 0,
            food: 0,
        },
        ..BASE
    },
];

/// A regiment type by its id (panics on an unknown id: the data names only these).
pub fn unit(id: &str) -> &'static UnitType {
    UNITS
        .iter()
        .find(|u| u.id == id)
        .unwrap_or_else(|| panic!("unknown regiment type {id}"))
}

pub fn unit_index(id: &str) -> usize {
    UNITS.iter().position(|u| u.id == id).unwrap_or(0)
}

/// A rough worth of a regiment, for the balance bar and the campaign's auto-resolve.
pub fn unit_power(t: &UnitType, men: u32) -> f32 {
    let per = (t.hp * (1.0 + t.armor / 80.0) * (t.atk + t.def + t.dmg * 2.0 + t.charge * 0.3)
        + t.ranged.map_or(0.0, |r| r.dmg * r.range * 0.6))
        / 100.0;
    per * men as f32 * if t.role == Role::Hero { 0.55 } else { 1.0 }
}

// ------------------------------------------------------------------ quick battles

pub struct Scenario {
    pub id: &'static str,
    pub title: &'static str,
    pub kicker: &'static str,
    pub subtitle: &'static str,
    pub blurb: &'static str,
    pub map: &'static str,
    pub player_attacks: bool,
    pub player: (Faction, &'static [&'static str]),
    pub enemy: (Faction, &'static [&'static str]),
}

pub static SCENARIOS: [Scenario; 5] = [
    Scenario {
        id: "vado",
        title: "El vado del Este",
        kicker: "BATALLA RÁPIDA · LA PUERTA DEL ESTE",
        subtitle: "La Alianza cruza el río. Sujeta el vado y el puente de los trolls.",
        blurb: "La Reina y sus pueblos contra lanceros, caballeros, ballesteros y sacerdotes del este.",
        map: "vado",
        player_attacks: false,
        player: (
            Faction::Reina,
            &["reina", "orcos", "orcos", "orcos", "lamias", "lamias", "trasgos", "trasgos", "esqueletos", "esqueletos", "arpias"],
        ),
        enemy: (
            Faction::Alianza,
            &["lanceros", "lanceros", "lanceros", "lanceros", "caballeros", "caballeros", "ballesteros", "ballesteros", "ballesteros", "sacerdotes"],
        ),
    },
    Scenario {
        id: "riscos",
        title: "El Peñascal",
        kicker: "BATALLA RÁPIDA · LA ALDEA DE VARRAK",
        subtitle: "Varrak espera en su herradura de roca con los clanes y la manada. Despiértalo con la chispa cuando flaquee.",
        blurb: "Asalta la herradura de roca: Varrak, los clanes del norte y sus lobos apagados.",
        map: "penascal",
        player_attacks: true,
        player: (
            Faction::Reina,
            &["reina", "orcos", "orcos", "lamias", "lamias", "trasgos", "esqueletos", "esqueletos", "arpias", "arpias"],
        ),
        enemy: (
            Faction::Rebeldes,
            &["varrak", "apagados", "apagados", "apagados", "clan_orcos", "clan_orcos", "clan_trasgos", "clan_trasgos", "lobos", "lobos"],
        ),
    },
    Scenario {
        id: "cienaga",
        title: "La Ciénaga de las lamias",
        kicker: "BATALLA RÁPIDA · LA CIÉNAGA",
        subtitle: "Una columna de la Alianza se mete en el pantano. El barro es de las lamias.",
        blurb: "Pantano, juncos y agua hasta la rodilla: mala tierra para los caballos.",
        map: "cienaga",
        player_attacks: true,
        player: (
            Faction::Reina,
            &["reina", "lamias", "lamias", "lamias", "orcos", "trasgos", "trasgos", "esqueletos", "arpias"],
        ),
        enemy: (
            Faction::Alianza,
            &["lanceros", "lanceros", "lanceros", "caballeros", "ballesteros", "ballesteros", "sacerdotes", "sacerdotes"],
        ),
    },
    Scenario {
        id: "atalaya",
        title: "La loma de la atalaya",
        kicker: "BATALLA RÁPIDA · EL NIDO DE LAS HARPÍAS",
        subtitle: "Los clanes del Nido tienen la loma y la atalaya. Sube por el collado o rodea por el bosque.",
        blurb: "Una loma larga con la atalaya en ruinas: subirla contra los clanes del norte.",
        map: "atalaya",
        player_attacks: true,
        player: (
            Faction::Reina,
            &["reina", "orcos", "orcos", "lamias", "trasgos", "trasgos", "esqueletos", "esqueletos", "arpias"],
        ),
        enemy: (
            Faction::Rebeldes,
            &["clan_orcos", "clan_orcos", "clan_orcos", "clan_trasgos", "clan_trasgos", "clan_trasgos", "lobos", "apagados"],
        ),
    },
    Scenario {
        id: "puerta",
        title: "La Puerta del Este",
        kicker: "BATALLA RÁPIDA · LA PUERTA DEL ESTE",
        subtitle: "La Alianza ha cruzado el puente y defiende la vieja muralla. Entra por la puerta o por la brecha.",
        blurb: "La vieja muralla, su puerta y su brecha; detrás, el río y el puente de los trolls.",
        map: "puerta",
        player_attacks: true,
        player: (
            Faction::Reina,
            &["reina", "orcos", "orcos", "orcos", "lamias", "trasgos", "esqueletos", "esqueletos", "arpias", "arpias"],
        ),
        enemy: (
            Faction::Alianza,
            &["lanceros", "lanceros", "lanceros", "ballesteros", "ballesteros", "caballeros", "sacerdotes"],
        ),
    },
];

pub fn scenario(id: &str) -> &'static Scenario {
    SCENARIOS
        .iter()
        .find(|s| s.id == id)
        .unwrap_or(&SCENARIOS[0])
}

// ------------------------------------------------------------------- the campaign

/// The campaign map's size (map units, x east, z south) and where the river runs.
pub const MAP_W: f32 = 240.0;
pub const MAP_D: f32 = 150.0;
pub const RIVER_X: f32 = 30.0;
pub const MAX_UNITS: usize = 10;

pub struct Province {
    pub id: &'static str,
    pub name: &'static str,
    pub kicker: &'static str,
    pub x: f32,
    pub z: f32,
    /// Where the settlement stands, if not at the centre.
    pub settle: Option<(f32, f32)>,
    pub owner: Faction,
    pub settlement: &'static str,
    /// The battlefield fought here.
    pub battle: &'static str,
    pub gold: f32,
    pub food: f32,
    pub spark: f32,
    pub capital: bool,
    pub victory: bool,
    pub garrison: &'static [&'static str],
    /// What each faction raises here once it holds the place.
    pub recruit: &'static [(Faction, &'static [&'static str])],
    pub lore: &'static str,
    /// The province (of the ten) the region belongs to.
    pub group: &'static str,
}

impl Province {
    pub fn site(&self) -> (f32, f32) {
        self.settle.unwrap_or((self.x, self.z))
    }

    pub fn recruits(&self, f: Faction) -> &'static [&'static str] {
        self.recruit
            .iter()
            .find(|(g, _)| *g == f)
            .map_or(&[], |(_, list)| *list)
    }
}

pub(crate) const P: Province = Province {
    id: "",
    name: "",
    kicker: "",
    x: 0.0,
    z: 0.0,
    settle: None,
    owner: Faction::Reina,
    settlement: "village",
    battle: "vado",
    gold: 0.0,
    food: 0.0,
    spark: 0.0,
    capital: false,
    victory: false,
    garrison: &[],
    recruit: &[],
    lore: "",
    group: "",
};

pub use crate::regions::{adjacent, crossing, Crossing, CROSSINGS, GROUPS, PROVINCES};

pub fn province(id: &str) -> &'static Province {
    PROVINCES
        .iter()
        .find(|p| p.id == id)
        .unwrap_or_else(|| panic!("unknown province {id}"))
}

pub fn province_index(id: &str) -> usize {
    PROVINCES.iter().position(|p| p.id == id).unwrap_or(0)
}

pub struct StartArmy {
    pub id: &'static str,
    pub faction: Faction,
    pub name: &'static str,
    pub at: &'static str,
    pub units: &'static [&'static str],
}

pub static START_ARMIES: [StartArmy; 7] = [
    StartArmy {
        id: "a-reina",
        faction: Faction::Reina,
        name: "Ejército de la Reina",
        at: "castillo",
        units: &[
            "reina",
            "orcos",
            "orcos",
            "lamias",
            "trasgos",
            "esqueletos",
            "esqueletos",
        ],
    },
    StartArmy {
        id: "a-guardia",
        faction: Faction::Reina,
        name: "Guardia del bosque",
        at: "bosque",
        units: &["orcos", "trasgos", "esqueletos"],
    },
    StartArmy {
        id: "r-manada",
        faction: Faction::Rebeldes,
        name: "La manada de Varrak",
        at: "penascal",
        units: &["varrak", "apagados", "apagados", "lobos", "clan_trasgos"],
    },
    StartArmy {
        id: "r-nido",
        faction: Faction::Rebeldes,
        name: "Los clanes del Nido",
        at: "nido",
        units: &["clan_orcos", "clan_orcos", "clan_trasgos"],
    },
    StartArmy {
        id: "l-marca",
        faction: Faction::Alianza,
        name: "Hueste de la Marca",
        at: "marca",
        units: &[
            "lanceros",
            "lanceros",
            "lanceros",
            "ballesteros",
            "ballesteros",
            "caballeros",
            "sacerdotes",
        ],
    },
    StartArmy {
        id: "l-capital",
        faction: Faction::Alianza,
        name: "Guarnición de la capital",
        at: "capital",
        units: &[
            "lanceros",
            "lanceros",
            "ballesteros",
            "caballeros",
            "caballeros",
        ],
    },
    StartArmy {
        // The one the Alliance sends over first: across the bridge, at the Queen's gate.
        id: "l-puente",
        faction: Faction::Alianza,
        name: "Guardia del Puente",
        at: "cabeza",
        units: &["lanceros", "lanceros", "ballesteros"],
    },
];

/// Gold, food and spark each faction starts with.
pub fn start_purse(f: Faction) -> (i32, i32, i32) {
    match f {
        Faction::Reina => (350, 18, 9),
        Faction::Rebeldes => (150, 10, 0),
        Faction::Alianza => (300, 20, 0),
        Faction::Libres => (0, 0, 0),
    }
}

pub fn army_names(f: Faction) -> &'static [&'static str] {
    match f {
        Faction::Reina => &[
            "Hueste del Bosque",
            "Lanzas de la Ciénaga",
            "La Camada de Grizna",
            "Vanguardia del sello",
            "Los Colmillos",
        ],
        Faction::Rebeldes => &["Clan del Risco", "La Jauría", "Los Sin Tributo"],
        Faction::Alianza => &[
            "Hueste de Valdesanto",
            "Columna del Alba",
            "Lanzas de Aldren",
            "Los Juramentados",
        ],
        Faction::Libres => &["Milicia libre"],
    }
}

pub const SEASONS: [&str; 4] = ["Primavera", "Verano", "Otoño", "Invierno"];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_name_in_the_data_is_known() {
        for s in &SCENARIOS {
            for id in s.player.1.iter().chain(s.enemy.1) {
                unit(id);
            }
        }
        for p in &PROVINCES {
            for id in p.garrison {
                unit(id);
            }
            for (_, list) in p.recruit {
                for id in *list {
                    unit(id);
                }
            }
            for q in adjacent(p.id) {
                assert!(adjacent(q).contains(&p.id), "{} - {q} goes one way", p.id);
            }
        }
        for a in &START_ARMIES {
            province(a.at);
            for id in a.units {
                unit(id);
            }
        }
    }

    #[test]
    fn power_ranks_regiments_sensibly() {
        assert!(unit_power(unit("caballeros"), 30) > unit_power(unit("trasgos"), 72) * 0.8);
        assert!(unit_power(unit("orcos"), 60) > unit_power(unit("apagados"), 80));
    }
}
