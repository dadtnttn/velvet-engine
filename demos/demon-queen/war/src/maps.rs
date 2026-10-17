//! The battlefields, designed by hand (not noise): each one a place of the campaign with a layout
//! that matters — where the crossing is, who has the high ground, where the woods hide a flank,
//! where a column has to squeeze through. The player always comes from the south (+z), the enemy
//! holds the north. Metres; the field is 420 × 320 inside a frame of wooded hills.

#[derive(Clone, Copy, Debug)]
pub struct Rect {
    pub x0: f32,
    pub x1: f32,
    pub z0: f32,
    pub z1: f32,
}

#[derive(Clone, Debug)]
pub struct River {
    pub pts: Vec<[f32; 2]>,
    pub width: f32,
    /// Where along it (0..1) the ford is, and how long.
    pub ford: Option<(f32, f32)>,
    /// Where along it (0..1) the trolls' bridge stands.
    pub bridge: Option<f32>,
    /// A brook, waded anywhere.
    pub shallow: bool,
}

#[derive(Clone, Debug)]
pub struct Ridge {
    pub pts: Vec<[f32; 2]>,
    pub width: f32,
    pub height: f32,
    /// The side (±1 of the line's left) with the steep face, 0 for none.
    pub steep: i32,
    pub taper: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct Hill {
    pub x: f32,
    pub z: f32,
    pub r: f32,
    pub h: f32,
}

#[derive(Clone, Debug)]
pub struct Cliff {
    pub pts: Vec<[f32; 2]>,
    pub thick: f32,
    pub height: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct Circle {
    pub x: f32,
    pub z: f32,
    pub r: f32,
}

#[derive(Clone, Debug)]
pub struct Causeway {
    pub pts: Vec<[f32; 2]>,
    pub width: f32,
}

#[derive(Clone, Debug)]
pub enum Forest {
    Poly { pts: Vec<[f32; 2]>, density: f32 },
    Circle(Circle),
}

#[derive(Clone, Copy, Debug)]
pub struct Gap {
    pub at: f32,
    pub w: f32,
}

#[derive(Clone, Debug)]
pub struct WallSpec {
    pub pts: Vec<[f32; 2]>,
    pub thick: f32,
    pub height: f32,
    /// Field fences (no merlons, uneven height).
    pub low: bool,
    pub gaps: Vec<Gap>,
    /// The gap (by its `at`) with a gatehouse.
    pub gatehouse: Option<f32>,
}

#[derive(Clone, Debug)]
pub struct PropSpec {
    pub kind: &'static str,
    pub x: f32,
    pub z: f32,
    pub rot: f32,
    pub seed: u32,
}

#[derive(Clone, Debug)]
pub struct MapSpec {
    pub kind: &'static str,
    pub title: &'static str,
    pub blurb: &'static str,
    pub seed: i32,
    pub base: f32,
    pub w: f32,
    pub d: f32,
    pub play: Rect,
    pub river: Option<River>,
    pub ridges: Vec<Ridge>,
    pub hills: Vec<Hill>,
    pub cliffs: Vec<Cliff>,
    pub marsh: Vec<Circle>,
    pub causeways: Vec<Causeway>,
    pub roads: Vec<Vec<[f32; 2]>>,
    pub forests: Vec<Forest>,
    pub clearings: Vec<Circle>,
    pub props: Vec<PropSpec>,
    pub walls: Vec<WallSpec>,
    /// Farmland strips painted on the ground.
    pub fields: bool,
    pub deploy_player: Option<Rect>,
    pub deploy_enemy: Option<Rect>,
}

/// Points on an arc round (cx, cz), angles in degrees (0 = east, 90 = south).
fn arc(cx: f32, cz: f32, r: f32, a0: f32, a1: f32, step: f32) -> Vec<[f32; 2]> {
    let n = (((a1 - a0).abs() / step).round() as usize).max(2);
    (0..=n)
        .map(|k| {
            let a = (a0 + (a1 - a0) * k as f32 / n as f32).to_radians();
            [cx + a.cos() * r, cz + a.sin() * r]
        })
        .collect()
}

fn prop(kind: &'static str, x: f32, z: f32, rot: f32) -> PropSpec {
    PropSpec {
        kind,
        x,
        z,
        rot,
        seed: 0,
    }
}

fn poly(pts: &[[f32; 2]]) -> Forest {
    Forest::Poly {
        pts: pts.to_vec(),
        density: 1.0,
    }
}

fn wood(x: f32, z: f32, r: f32) -> Forest {
    Forest::Circle(Circle { x, z, r })
}

fn hill(x: f32, z: f32, r: f32, h: f32) -> Hill {
    Hill { x, z, r, h }
}

fn empty(
    kind: &'static str,
    title: &'static str,
    blurb: &'static str,
    seed: i32,
    base: f32,
) -> MapSpec {
    MapSpec {
        kind,
        title,
        blurb,
        seed,
        base,
        w: 560.0,
        d: 440.0,
        play: Rect {
            x0: -210.0,
            x1: 210.0,
            z0: -160.0,
            z1: 160.0,
        },
        river: None,
        ridges: vec![],
        hills: vec![],
        cliffs: vec![],
        marsh: vec![],
        causeways: vec![],
        roads: vec![],
        forests: vec![],
        clearings: vec![],
        props: vec![],
        walls: vec![],
        fields: false,
        deploy_player: None,
        deploy_enemy: None,
    }
}

pub const MAP_IDS: [&str; 7] = [
    "vado", "atalaya", "bosque", "penascal", "puerta", "cienaga", "trigales",
];

/// A battlefield by its id (the ford if unknown); `seed` overrides its own.
pub fn map_spec(kind: &str, seed: Option<i32>) -> MapSpec {
    let mut m = match kind {
        "atalaya" => atalaya(),
        "bosque" => bosque(),
        "penascal" => penascal(),
        "puerta" => puerta(),
        "cienaga" => cienaga(),
        "trigales" => trigales(),
        _ => vado(),
    };
    if let Some(s) = seed {
        m.seed = s;
    }
    m
}

fn vado() -> MapSpec {
    let mut m = empty(
        "vado",
        "El vado del Este",
        "El río solo se cruza por el vado (ancho y lento) o por el puente de los trolls (estrecho y lejos). Al norte, la loma con la atalaya en ruinas domina el vado.",
        7,
        1.4,
    );
    m.river = Some(River {
        pts: vec![
            [-300.0, 18.0],
            [-180.0, -6.0],
            [-90.0, 12.0],
            [-20.0, 2.0],
            [50.0, -18.0],
            [130.0, 4.0],
            [300.0, -12.0],
        ],
        width: 20.0,
        ford: Some((0.355, 38.0)),
        bridge: Some(0.69),
        shallow: false,
    });
    m.ridges = vec![Ridge {
        pts: vec![
            [-200.0, -84.0],
            [-120.0, -76.0],
            [-40.0, -80.0],
            [40.0, -92.0],
            [140.0, -104.0],
            [215.0, -96.0],
        ],
        width: 38.0,
        height: 9.0,
        steep: -1,
        taper: 40.0,
    }];
    m.hills = vec![hill(-128.0, 96.0, 44.0, 7.0), hill(84.0, 118.0, 36.0, 4.0)];
    m.forests = vec![
        poly(&[
            [-215.0, 30.0],
            [-168.0, 24.0],
            [-140.0, 58.0],
            [-150.0, 132.0],
            [-215.0, 150.0],
        ]),
        poly(&[
            [-215.0, -40.0],
            [-176.0, -30.0],
            [-158.0, -64.0],
            [-178.0, -130.0],
            [-215.0, -130.0],
        ]),
        wood(160.0, 70.0, 26.0),
        wood(48.0, -132.0, 22.0),
    ];
    m.roads = vec![
        vec![
            [-72.0, 230.0],
            [-76.0, 120.0],
            [-82.0, 44.0],
            [-84.0, 14.0],
            [-86.0, -14.0],
            [-74.0, -56.0],
            [-48.0, -110.0],
            [-30.0, -230.0],
        ],
        vec![
            [132.0, 230.0],
            [124.0, 96.0],
            [112.0, 22.0],
            [108.0, -26.0],
            [84.0, -72.0],
            [46.0, -122.0],
            [16.0, -230.0],
        ],
    ];
    m.props = vec![
        prop("watchtower", -96.0, -82.0, 0.3),
        prop("ruin_wall", -76.0, -86.0, 1.8),
        prop("ruin_wall", -112.0, -70.0, 0.6),
        prop("standing_stone", -58.0, 32.0, 0.2),
        prop("standing_stone", -106.0, 30.0, 1.4),
        prop("shrine", 142.0, 32.0, -0.6),
        prop("cart", -70.0, 76.0, 0.9),
        prop("totem", 96.0, 26.0, 0.0),
        prop("totem", 128.0, 26.0, 0.0),
    ];
    m
}

fn atalaya() -> MapSpec {
    let mut m = empty(
        "atalaya",
        "La loma de la atalaya",
        "Una loma larga corta el campo con la atalaya en ruinas en la cresta. Solo el collado del este es fácil de subir; el bosque del oeste esconde un flanco.",
        19,
        1.6,
    );
    m.ridges = vec![
        Ridge {
            pts: vec![
                [-215.0, -6.0],
                [-120.0, -24.0],
                [-20.0, -30.0],
                [70.0, -26.0],
                [104.0, -34.0],
            ],
            width: 46.0,
            height: 16.0,
            steep: -1,
            taper: 30.0,
        },
        Ridge {
            pts: vec![[160.0, -30.0], [215.0, -18.0]],
            width: 40.0,
            height: 11.0,
            steep: 0,
            taper: 20.0,
        },
    ];
    m.hills = vec![hill(60.0, 92.0, 40.0, 4.0), hill(-60.0, -120.0, 50.0, 6.0)];
    m.forests = vec![
        poly(&[
            [-215.0, -100.0],
            [-140.0, -96.0],
            [-118.0, -40.0],
            [-130.0, 40.0],
            [-215.0, 50.0],
        ]),
        wood(170.0, 96.0, 30.0),
        wood(120.0, -120.0, 26.0),
    ];
    m.roads = vec![
        vec![
            [126.0, 230.0],
            [132.0, 60.0],
            [132.0, -30.0],
            [116.0, -100.0],
            [100.0, -230.0],
        ],
        vec![
            [-215.0, 110.0],
            [-60.0, 104.0],
            [60.0, 110.0],
            [132.0, 60.0],
        ],
    ];
    m.walls = vec![WallSpec {
        pts: vec![[-90.0, 60.0], [-20.0, 64.0], [40.0, 58.0], [100.0, 66.0]],
        thick: 1.6,
        height: 1.3,
        low: true,
        gaps: vec![Gap { at: 40.0, w: 12.0 }, Gap { at: 120.0, w: 12.0 }],
        gatehouse: None,
    }];
    m.props = vec![
        prop("watchtower", -18.0, -30.0, 0.1),
        prop("ruin_wall", 4.0, -34.0, 1.4),
        prop("ruin_wall", -40.0, -24.0, 0.2),
        prop("shrine", 170.0, -40.0, 0.4),
        prop("standing_stone", 112.0, -34.0, 0.8),
        prop("standing_stone", 150.0, -40.0, 2.4),
        prop("cart", 128.0, 40.0, 1.6),
    ];
    m
}

fn bosque() -> MapSpec {
    let mut m = empty(
        "bosque",
        "El linde del Bosque Oscuro",
        "Medio campo es bosque cerrado: frena a todos y para las flechas. El arroyo se vadea en cualquier sitio. Al este, pradera abierta para la caballería.",
        23,
        1.4,
    );
    m.river = Some(River {
        pts: vec![
            [-300.0, -36.0],
            [-160.0, -30.0],
            [-60.0, -8.0],
            [30.0, 12.0],
            [120.0, -4.0],
            [300.0, 12.0],
        ],
        width: 9.0,
        ford: None,
        bridge: None,
        shallow: true,
    });
    m.forests = vec![
        poly(&[
            [-215.0, -170.0],
            [-46.0, -170.0],
            [-22.0, -110.0],
            [-58.0, -46.0],
            [-26.0, 16.0],
            [-54.0, 92.0],
            [-34.0, 170.0],
            [-215.0, 170.0],
        ]),
        wood(150.0, -110.0, 30.0),
    ];
    m.clearings = vec![
        Circle {
            x: -122.0,
            z: 6.0,
            r: 26.0,
        },
        Circle {
            x: -150.0,
            z: 110.0,
            r: 16.0,
        },
    ];
    m.hills = vec![hill(118.0, -78.0, 52.0, 8.0), hill(92.0, 102.0, 42.0, 5.0)];
    m.roads = vec![
        vec![
            [4.0, 230.0],
            [-4.0, 110.0],
            [10.0, 4.0],
            [-6.0, -100.0],
            [0.0, -230.0],
        ],
        vec![[-122.0, 6.0], [-60.0, 0.0], [10.0, 4.0]],
    ];
    m.props = vec![
        prop("lodge", -122.0, 6.0, 0.4),
        prop("standing_stone", 64.0, 36.0, 0.3),
        prop("standing_stone", 74.0, 46.0, 1.1),
        prop("standing_stone", 56.0, 50.0, 2.0),
        prop("shrine", 130.0, -84.0, 2.6),
        prop("totem", -30.0, 4.0, 0.0),
    ];
    m
}

fn penascal() -> MapSpec {
    let mut m = empty(
        "penascal",
        "El Peñascal",
        "La aldea trasga en su herradura de roca. Se entra por la boca del sur o por el paso estrecho del noreste; las harpías vuelan por encima de las peñas.",
        31,
        1.2,
    );
    m.cliffs = vec![
        Cliff {
            pts: arc(0.0, -52.0, 88.0, 118.0, 300.0, 10.0),
            thick: 12.0,
            height: 15.0,
        },
        Cliff {
            pts: arc(0.0, -52.0, 88.0, 314.0, 422.0, 10.0),
            thick: 12.0,
            height: 15.0,
        },
    ];
    m.hills = vec![hill(0.0, -52.0, 70.0, 2.0)];
    m.forests = vec![
        poly(&[
            [-215.0, 30.0],
            [-110.0, 24.0],
            [-70.0, 70.0],
            [-90.0, 160.0],
            [-215.0, 160.0],
        ]),
        poly(&[
            [215.0, 30.0],
            [118.0, 24.0],
            [76.0, 70.0],
            [96.0, 160.0],
            [215.0, 160.0],
        ]),
        poly(&[
            [-215.0, -160.0],
            [-120.0, -160.0],
            [-130.0, -60.0],
            [-215.0, -40.0],
        ]),
    ];
    m.roads = vec![
        vec![[0.0, 230.0], [6.0, 110.0], [0.0, 44.0], [0.0, -20.0]],
        vec![[62.0, -110.0], [100.0, -150.0], [150.0, -230.0]],
    ];
    let mut props = vec![prop("market_stone", 0.0, -58.0, 0.2)];
    for (k, [x, z]) in arc(0.0, -52.0, 74.0, 130.0, 290.0, 20.0)
        .into_iter()
        .enumerate()
    {
        props.push(PropSpec {
            kind: "house_rock",
            x,
            z,
            rot: (-x).atan2(-(z + 52.0)),
            seed: k as u32,
        });
    }
    for (k, [x, z]) in arc(0.0, -52.0, 74.0, 326.0, 410.0, 28.0)
        .into_iter()
        .enumerate()
    {
        props.push(PropSpec {
            kind: "house_rock",
            x,
            z,
            rot: (-x).atan2(-(z + 52.0)),
            seed: k as u32 + 20,
        });
    }
    props.extend([
        prop("totem", -30.0, 30.0, 0.0),
        prop("totem", 30.0, 30.0, 0.0),
        prop("well", 30.0, -36.0, 0.0),
        prop("tent", -34.0, -78.0, 0.6),
        prop("tent", 40.0, -84.0, -0.5),
        prop("cart", -22.0, -28.0, 2.2),
    ]);
    m.props = props;
    m.deploy_enemy = Some(Rect {
        x0: -55.0,
        x1: 55.0,
        z0: -122.0,
        z1: -34.0,
    });
    m
}

fn puerta() -> MapSpec {
    let mut m = empty(
        "puerta",
        "La Puerta del Este",
        "La vieja muralla cruza el campo: se pasa por la puerta o por la brecha del oeste. Detrás, el río y el puente de los trolls, el único paso hacia el este.",
        41,
        1.3,
    );
    m.river = Some(River {
        pts: vec![
            [-300.0, -112.0],
            [-160.0, -104.0],
            [-40.0, -118.0],
            [60.0, -106.0],
            [180.0, -116.0],
            [300.0, -108.0],
        ],
        width: 18.0,
        ford: None,
        bridge: Some(0.52),
        shallow: false,
    });
    m.walls = vec![WallSpec {
        pts: vec![
            [-230.0, -46.0],
            [-120.0, -40.0],
            [0.0, -44.0],
            [120.0, -40.0],
            [230.0, -46.0],
        ],
        thick: 3.2,
        height: 7.0,
        low: false,
        gaps: vec![Gap { at: 112.0, w: 20.0 }, Gap { at: 232.0, w: 16.0 }],
        gatehouse: Some(232.0),
    }];
    m.hills = vec![hill(110.0, 78.0, 48.0, 7.0), hill(-140.0, 110.0, 40.0, 5.0)];
    m.forests = vec![
        poly(&[
            [-215.0, 40.0],
            [-150.0, 36.0],
            [-120.0, 80.0],
            [-140.0, 160.0],
            [-215.0, 160.0],
        ]),
        wood(170.0, 140.0, 30.0),
        wood(-170.0, -80.0, 20.0),
    ];
    m.roads = vec![vec![
        [4.0, 230.0],
        [8.0, 80.0],
        [6.0, -20.0],
        [6.0, -60.0],
        [8.0, -112.0],
        [10.0, -230.0],
    ]];
    m.props = vec![
        prop("ruin_wall", -120.0, -30.0, 0.1),
        prop("ruin_wall", -128.0, -52.0, 1.2),
        prop("troll", -8.0, -92.0, 0.0),
        prop("troll", 22.0, -92.0, 0.0),
        prop("cart", 30.0, 20.0, 0.4),
        prop("standing_stone", -70.0, 60.0, 0.6),
        prop("tent", -40.0, -76.0, 0.2),
        prop("tent", 60.0, -80.0, -0.3),
    ];
    m.deploy_enemy = Some(Rect {
        x0: -150.0,
        x1: 150.0,
        z0: -98.0,
        z1: -54.0,
    });
    m
}

fn cienaga() -> MapSpec {
    let mut m = empty(
        "cienaga",
        "La Ciénaga de las lamias",
        "Agua hasta la rodilla y barro: todos van despacio menos las lamias. Las calzadas secas cruzan el pantano; las islas son el único suelo firme.",
        5,
        1.0,
    );
    m.marsh = vec![
        Circle {
            x: -64.0,
            z: 4.0,
            r: 62.0,
        },
        Circle {
            x: 46.0,
            z: -24.0,
            r: 56.0,
        },
        Circle {
            x: 0.0,
            z: 64.0,
            r: 36.0,
        },
        Circle {
            x: 128.0,
            z: 36.0,
            r: 40.0,
        },
        Circle {
            x: -136.0,
            z: -56.0,
            r: 36.0,
        },
    ];
    m.causeways = vec![
        Causeway {
            pts: vec![
                [-24.0, 230.0],
                [-12.0, 60.0],
                [8.0, 2.0],
                [0.0, -60.0],
                [22.0, -230.0],
            ],
            width: 10.0,
        },
        Causeway {
            pts: vec![
                [-230.0, 12.0],
                [-84.0, 22.0],
                [8.0, 2.0],
                [100.0, -12.0],
                [230.0, 0.0],
            ],
            width: 9.0,
        },
    ];
    m.hills = vec![
        hill(-66.0, 2.0, 20.0, 3.2),
        hill(48.0, -26.0, 16.0, 2.6),
        hill(-150.0, 100.0, 40.0, 5.0),
    ];
    m.forests = vec![
        wood(-176.0, 72.0, 34.0),
        wood(176.0, -76.0, 36.0),
        wood(160.0, 120.0, 26.0),
    ];
    m.props = vec![
        prop("hut", -66.0, -6.0, 0.4),
        prop("hut", -54.0, 12.0, 2.0),
        prop("hut", 50.0, -30.0, 1.0),
        prop("shrine", -72.0, 10.0, 0.2),
        prop("standing_stone", 120.0, 30.0, 1.0),
        prop("totem", 6.0, 40.0, 0.0),
    ];
    m
}

fn trigales() -> MapSpec {
    let mut m = empty(
        "trigales",
        "Los Trigales",
        "Campos de trigo cortados por cercas de piedra: frenan las cargas y protegen a los ballesteros. El molino corona la loma del este.",
        13,
        1.2,
    );
    m.fields = true;
    m.hills = vec![hill(112.0, -64.0, 54.0, 9.0), hill(-96.0, 104.0, 48.0, 5.0)];
    m.forests = vec![wood(-182.0, 24.0, 34.0), wood(186.0, 56.0, 30.0)];
    m.walls = vec![
        WallSpec {
            pts: vec![[-160.0, 30.0], [-60.0, 26.0], [40.0, 32.0], [150.0, 26.0]],
            thick: 1.5,
            height: 1.2,
            low: true,
            gaps: vec![
                Gap { at: 60.0, w: 14.0 },
                Gap { at: 170.0, w: 14.0 },
                Gap { at: 260.0, w: 12.0 },
            ],
            gatehouse: None,
        },
        WallSpec {
            pts: vec![[-120.0, -40.0], [-20.0, -44.0], [80.0, -40.0]],
            thick: 1.5,
            height: 1.2,
            low: true,
            gaps: vec![Gap { at: 90.0, w: 14.0 }, Gap { at: 170.0, w: 12.0 }],
            gatehouse: None,
        },
    ];
    m.roads = vec![
        vec![[-40.0, 230.0], [-16.0, 60.0], [-4.0, -20.0], [20.0, -230.0]],
        vec![[-230.0, -6.0], [-60.0, 0.0], [112.0, -64.0]],
    ];
    m.props = vec![
        prop("windmill", 112.0, -66.0, 0.6),
        prop("farmhouse", -96.0, 96.0, 0.3),
        prop("farmhouse", 60.0, -12.0, -0.4),
        prop("well", 72.0, -2.0, 0.0),
        prop("cart", -30.0, 50.0, 1.0),
    ];
    m
}
