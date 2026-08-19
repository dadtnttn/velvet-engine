use velvet_script_vs3::{bool_val, float_val, list_val, map_val, string_val, Value};
#[derive(Debug, Clone, Default)]
pub struct InputFrame {
    pub held: Vec<String>,
    pub pressed: Vec<String>,
    pub released: Vec<String>,
    pub pointer: Option<[f64; 2]>,
    pub mouse_held: bool,
    pub mouse_pressed: bool,
    pub mouse_released: bool,
    pub wheel: f64,
}
impl InputFrame {
    pub fn into_value(self) -> Value {
        let pointer = match self.pointer {
            Some([x, y]) => velvet_script_vs3::vec2_val(x, y),
            None => Value::Null,
        };
        map_val([
            (
                "held".into(),
                list_val(self.held.into_iter().map(string_val).collect()),
            ),
            (
                "pressed".into(),
                list_val(self.pressed.into_iter().map(string_val).collect()),
            ),
            (
                "released".into(),
                list_val(self.released.into_iter().map(string_val).collect()),
            ),
            ("pointer".into(), pointer),
            ("mouse_held".into(), bool_val(self.mouse_held)),
            ("mouse_pressed".into(), bool_val(self.mouse_pressed)),
            ("mouse_released".into(), bool_val(self.mouse_released)),
            ("wheel".into(), float_val(self.wheel)),
        ])
    }
}
