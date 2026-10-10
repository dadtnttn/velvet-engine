//! Shared ramps (dark → light, hue-shifted: cool shadows, warm lights); a port
//! of `palette.js`. Materials are compared by identity inside a figure, so a
//! palette is built once and shared.
use std::rc::Rc;

use super::pixel::{material, ramp, MatOpts, Material};

pub struct Palette {
    pub queen_skin: Rc<Material>,
    pub queen_hair: Rc<Material>,
    pub horn: Rc<Material>,
    pub gold: Rc<Material>,
    pub dress: Rc<Material>,
    pub crimson: Rc<Material>,
    pub ear_in: Rc<Material>,
    pub meat: Rc<Material>,
    pub bone: Rc<Material>,
    pub wine: Rc<Material>,
    pub gem_red: Rc<Material>,
    pub nyx_skin: Rc<Material>,
    pub nyx_hair: Rc<Material>,
    pub nyx_hair_tip: Rc<Material>,
    pub ivory: Rc<Material>,
    pub burgundy: Rc<Material>,
    pub black: Rc<Material>,
    pub glove: Rc<Material>,
    pub glass: Rc<Material>,
}

fn o() -> MatOpts {
    MatOpts::default()
}

impl Palette {
    pub fn new() -> Self {
        Self {
            queen_skin: material(
                ramp(&[
                    "#2f2660", "#463f8c", "#6566b2", "#8894d2", "#adbcec", "#d6e1fb",
                ]),
                o().dither(0.25).line([24, 14, 48]).inner([52, 42, 104]),
            ),
            queen_hair: material(
                ramp(&[
                    "#5b5873", "#8a889f", "#b3b5c9", "#d6d9e8", "#f0f2f9", "#ffffff",
                ]),
                o().dither(0.3).line([40, 36, 58]).inner([140, 140, 166]),
            ),
            horn: material(
                ramp(&["#0c0812", "#191221", "#2b2036", "#433451", "#5f4e74"]),
                o().dither(0.2).line([8, 4, 10]),
            ),
            gold: material(
                ramp(&["#4a2206", "#86480c", "#c4841c", "#eebd4a", "#fff0a0"]),
                o().dither(0.2).line([40, 16, 4]).inner([110, 58, 10]),
            ),
            dress: material(
                ramp(&["#09060e", "#141019", "#201a2c", "#30263f", "#463a58"]),
                o().dither(0.35).line([6, 3, 8]).inner([72, 58, 96]),
            ),
            crimson: material(
                ramp(&["#34050f", "#600c1e", "#921a30", "#c42c42", "#ea5a68"]),
                o().dither(0.3).line([26, 3, 8]).inner([60, 8, 18]),
            ),
            ear_in: material(
                ramp(&["#6b2a60", "#a44a82", "#d27aa8"]),
                o().dither(0.2).no_outline().inner([120, 50, 100]),
            ),
            meat: material(
                ramp(&[
                    "#351004", "#662407", "#9a4210", "#ca7022", "#eea246", "#ffd78a",
                ]),
                o().dither(0.3).line([34, 10, 2]).inner([80, 30, 8]),
            ),
            bone: material(
                ramp(&["#6a5c55", "#a4968a", "#d4c8ba", "#f6efe4"]),
                o().dither(0.2).line([50, 38, 34]),
            ),
            wine: material(
                ramp(&["#2a0410", "#4e0a1e", "#7a1430"]),
                o().dither(0.0).no_outline(),
            ),
            gem_red: material(
                ramp(&["#5a0414", "#b01030", "#ff4a5e", "#ffc0c8"]),
                o().dither(0.0).line([40, 4, 10]),
            ),
            nyx_skin: material(
                ramp(&[
                    "#24100b", "#421e12", "#63321d", "#864c34", "#a6664a", "#c48466",
                ]),
                o().dither(0.25).line([24, 9, 7]).inner([60, 27, 16]),
            ),
            nyx_hair: material(
                ramp(&[
                    "#050408", "#0b0911", "#12101b", "#1b1628", "#271e3a", "#3a2a58",
                ]),
                o().dither(0.3)
                    .contrast(0.9)
                    .line([3, 2, 6])
                    .inner([8, 6, 14]),
            ),
            nyx_hair_tip: material(
                ramp(&["#1c0f36", "#3a1e66", "#5e32a0", "#8a52d0", "#b27ef0"]),
                o().dither(0.3).line([10, 4, 22]).inner([34, 16, 60]),
            ),
            ivory: material(
                ramp(&[
                    "#4e4458", "#877c8e", "#bdb4bc", "#e2dcdc", "#f8f4ee", "#ffffff",
                ]),
                o().dither(0.3).line([36, 28, 44]).inner([104, 94, 112]),
            ),
            burgundy: material(
                ramp(&["#240410", "#44091e", "#6c1230", "#962042", "#c03a5a"]),
                o().dither(0.3).line([18, 2, 8]).inner([40, 6, 16]),
            ),
            black: material(
                ramp(&["#0c0a10", "#18151f", "#262130", "#383046", "#51465f"]),
                o().dither(0.3).line([4, 3, 6]).inner([66, 58, 84]),
            ),
            glove: material(
                ramp(&["#060509", "#110f16", "#201c28", "#34303e", "#5a5468"]),
                o().dither(0.2).line([3, 2, 5]).inner([70, 64, 88]),
            ),
            glass: material(
                ramp(&["#2a3448", "#4a5c78", "#7c92b0", "#b8cce0", "#eef6ff"]),
                o().dither(0.0).line([20, 26, 40]).inner([60, 76, 100]),
            ),
        }
    }
}

impl Default for Palette {
    fn default() -> Self {
        Self::new()
    }
}
