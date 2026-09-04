//! Style values (colors, lengths, keywords).

use serde::{Deserialize, Serialize};

/// Parsed color (sRGB 0–255 + alpha 0–1).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Color {
    /// Red.
    pub r: u8,
    /// Green.
    pub g: u8,
    /// Blue.
    pub b: u8,
    /// Alpha 0..=1.
    pub a: f32,
}

impl Color {
    /// RGB opaque.
    pub fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 1.0 }
    }

    /// RGBA.
    pub fn rgba(r: u8, g: u8, b: u8, a: f32) -> Self {
        Self {
            r,
            g,
            b,
            a: a.clamp(0.0, 1.0),
        }
    }

    /// Pack to 0x00RRGGBB (softbuffer-friendly).
    pub fn pack_rgb(&self) -> u32 {
        ((self.r as u32) << 16) | ((self.g as u32) << 8) | (self.b as u32)
    }

    /// Tuple for demo paint helpers.
    pub fn rgb_tuple(&self) -> (u8, u8, u8) {
        (self.r, self.g, self.b)
    }
}

/// One CSS-like value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum StyleValue {
    /// Color.
    Color(Color),
    /// Length in logical px (unitless number treated as px).
    Length(f32),
    /// Number (opacity, scale, …).
    Number(f32),
    /// Percentage 0..=100 (needs a layout base to resolve to px).
    Percent(f32),
    /// Gradient (`linear-gradient()` / `radial-gradient()`).
    Gradient(Gradient),
    /// Deferred `color-mix()` with `var()` arguments (folds to [`StyleValue::Color`] in resolve).
    ColorMix {
        /// First color (`Color` or `Var`/`VarFallback`).
        a: Box<StyleValue>,
        /// Weight of `a` in percent.
        wa: f32,
        /// Second color.
        b: Box<StyleValue>,
        /// Weight of `b` in percent.
        wb: f32,
    },
    /// Deferred `alpha()` with a `var()` color (folds in resolve).
    ///
    /// Kept separate from [`StyleValue::ColorMix`] on purpose: `alpha()`
    /// overrides the channel (rgb untouched), which `color-mix()` against
    /// transparent does NOT reproduce for translucent bases.
    Alpha {
        /// Base color (`Color` or `Var`/`VarFallback`).
        color: Box<StyleValue>,
        /// Override alpha 0..=1.
        amount: f32,
    },
    /// Deferred `auto-contrast()` with `var()` arguments (folds in resolve).
    AutoContrast {
        /// Base color to contrast against.
        base: Box<StyleValue>,
        /// Picked when the base is bright (default black).
        dark: Box<StyleValue>,
        /// Picked when the base is dark (default white).
        light: Box<StyleValue>,
    },
    /// Structured `box-shadow` layers (colors may hold `var()`).
    Shadow(Vec<BoxShadow>),
    /// Unevaluated math (`calc()`/`min()`/`max()`/`clamp()`).
    Calc(CalcExpr),
    /// Time in seconds (`s` and `ms` inputs normalize to this variant).
    Time(f32),
    /// Keyword / string token.
    Keyword(String),
    /// String (quoted).
    String(String),
    /// Custom property reference: `var(--name)`.
    Var(String),
    /// Custom property reference with a fallback: `var(--name, fallback)`.
    ///
    /// The existing [`StyleValue::Var`] variant remains the representation for
    /// references without a fallback.
    VarFallback {
        /// Canonical custom property name, including the `--` prefix.
        name: String,
        /// Value used when the custom property is missing or cyclic.
        fallback: Box<StyleValue>,
    },
    /// Asset / resource URL: `url("path")` or `svg(name)`.
    Url(String),
    /// Named inline SVG from `@svg name { … }`.
    SvgRef(String),
}

impl StyleValue {
    /// As color if possible.
    pub fn as_color(&self) -> Option<Color> {
        match self {
            Self::Color(c) => Some(*c),
            Self::Keyword(k) => parse_named_color(k),
            _ => None,
        }
    }

    /// As f32 (length, number or time).
    ///
    /// `Percent` and `Calc` need a layout base and never resolve here
    /// (same as keywords before them).
    pub fn as_f32(&self) -> Option<f32> {
        match self {
            Self::Length(v) | Self::Number(v) | Self::Time(v) => Some(*v),
            _ => None,
        }
    }

    /// As string keyword.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::Keyword(s) | Self::String(s) => Some(s),
            Self::Var(s) | Self::VarFallback { name: s, .. } => Some(s.as_str()),
            _ => None,
        }
    }

    /// Custom property name if this is a `var(--…)`.
    pub fn as_var(&self) -> Option<&str> {
        match self {
            Self::Var(s) | Self::VarFallback { name: s, .. } => Some(s.as_str()),
            _ => None,
        }
    }
}

/// One gradient stop.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GradientStop {
    /// Stop color (`Color`, or `Var`/`VarFallback` until resolve folds it).
    pub color: StyleValue,
    /// Position 0..=1 (None = evenly distributed at sample time).
    pub position: Option<f32>,
}

/// Gradient value from `linear-gradient()` / `radial-gradient()`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Gradient {
    /// True for radial (circle), false for linear.
    pub radial: bool,
    /// Linear angle in degrees (`to bottom` = 180); radial ignores it.
    pub angle_deg: f32,
    /// Radial center `[x, y]` 0..=1 (default 50/50); linear ignores it.
    pub center: [f32; 2],
    /// Color stops in author order (≥1).
    pub stops: Vec<GradientStop>,
}

impl Gradient {
    /// Concrete `(color, position)` stops, or `None` when any stop still
    /// holds an unresolved `var()` (resolve first).
    fn concrete_stops(&self) -> Option<Vec<(Color, Option<f32>)>> {
        self.stops
            .iter()
            .map(|s| match &s.color {
                StyleValue::Color(c) => Some((*c, s.position)),
                _ => None,
            })
            .collect()
    }

    /// Sample the gradient at `t` 0..=1 (missing positions spread evenly).
    ///
    /// Returns `None` when a stop still holds `var()` — resolve first.
    pub fn sample(&self, t: f32) -> Option<Color> {
        let stops = self.concrete_stops()?;
        if stops.is_empty() {
            return None;
        }
        if stops.len() == 1 {
            return Some(stops[0].0);
        }
        let n = stops.len();
        let pos = |i: usize| stops[i].1.unwrap_or(i as f32 / (n - 1).max(1) as f32);
        let t = t.clamp(0.0, 1.0);
        if t <= pos(0) {
            return Some(stops[0].0);
        }
        for i in 0..n - 1 {
            let (a, b) = (pos(i), pos(i + 1));
            if t <= b {
                let k = if b > a { (t - a) / (b - a) } else { 1.0 };
                return Some(mix_color(stops[i].0, stops[i + 1].0, k));
            }
        }
        Some(stops[n - 1].0)
    }

    /// First stop color (solid fallback for hosts without gradients).
    ///
    /// Returns `None` when the first stop still holds `var()`.
    pub fn fallback_color(&self) -> Option<Color> {
        match self.stops.first().map(|s| &s.color) {
            Some(StyleValue::Color(c)) => Some(*c),
            _ => None,
        }
    }
}

fn mix_color(a: Color, b: Color, k: f32) -> Color {
    let k = k.clamp(0.0, 1.0);
    let ch = |x: u8, y: u8| ((x as f32) + ((y as f32) - (x as f32)) * k).round() as u8;
    Color::rgba(
        ch(a.r, b.r),
        ch(a.g, b.g),
        ch(a.b, b.b),
        (a.a + (b.a - a.a) * k).clamp(0.0, 1.0),
    )
}

/// Split on top-level commas (nested parens never split).
fn split_top_commas(s: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut depth = 0u32;
    let mut start = 0usize;
    for (i, c) in s.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                out.push(&s[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    out.push(&s[start..]);
    out
}

/// Parse `linear-gradient(...)` / `radial-gradient(...)`.
pub fn parse_gradient(raw: &str) -> Option<Gradient> {
    let s = raw.trim();
    let lower = s.to_ascii_lowercase();
    let (radial, inner) = gradient_body(&lower, s)?;
    let parts = split_top_commas(inner);
    if parts.is_empty() {
        return None;
    }
    let mut angle_deg = 180.0;
    let mut center = [0.5, 0.5];
    let mut first = 0;
    if !radial {
        if let Some(angle) = parse_linear_direction(parts[0]) {
            angle_deg = angle;
            first = 1;
        }
    } else if let Some(at) = parse_radial_at(parts[0]) {
        center = at;
        first = 1;
    }
    let mut stops = Vec::new();
    for part in &parts[first..] {
        stops.push(parse_gradient_stop(part.trim())?);
    }
    if stops.is_empty() {
        return None;
    }
    Some(Gradient {
        radial,
        angle_deg,
        center,
        stops,
    })
}

/// Split `linear-gradient(` / `radial-gradient(` head from the body.
fn gradient_body<'a>(lower: &str, original: &'a str) -> Option<(bool, &'a str)> {
    for (prefix, radial) in [("linear-gradient(", false), ("radial-gradient(", true)] {
        if lower.starts_with(prefix) {
            let body = original.strip_suffix(')')?;
            return Some((radial, &body[prefix.len()..]));
        }
    }
    None
}

/// `to <side> [<side>]` / `Ndeg` / `Nturn` → degrees (`to bottom` = 180).
fn parse_linear_direction(part: &str) -> Option<f32> {
    let words: Vec<&str> = part.split_whitespace().collect();
    if words.first() == Some(&"to") && (2..=3).contains(&words.len()) {
        let mut x = 0.0f32;
        let mut y = 0.0f32;
        for side in &words[1..] {
            match *side {
                "top" => y -= 1.0,
                "bottom" => y += 1.0,
                "left" => x -= 1.0,
                "right" => x += 1.0,
                _ => return None,
            }
        }
        if x == 0.0 && y == 0.0 {
            return None;
        }
        // CSS angle: 0deg points up, clockwise positive.
        let angle = x.atan2(-y).to_degrees();
        return Some(angle.rem_euclid(360.0));
    }
    let lower = part.trim().to_ascii_lowercase();
    if let Some(deg) = lower.strip_suffix("deg") {
        return deg.trim().parse::<f32>().ok().map(|d| d.rem_euclid(360.0));
    }
    if let Some(turn) = lower.strip_suffix("turn") {
        return turn
            .trim()
            .parse::<f32>()
            .ok()
            .map(|t| (t * 360.0).rem_euclid(360.0));
    }
    None
}

/// `circle at X Y` / `at X Y` → center 0..=1 (`50%` units or 0..=1 numbers).
fn parse_radial_at(part: &str) -> Option<[f32; 2]> {
    let words: Vec<&str> = part.split_whitespace().collect();
    let at = words.iter().position(|w| *w == "at")?;
    let coords: Vec<&str> = words[at + 1..].to_vec();
    if coords.len() != 2 {
        return None;
    }
    // Only `circle` (or nothing) may precede `at`.
    if !words[..at].iter().all(|w| *w == "circle") {
        return None;
    }
    let mut out = [0.5, 0.5];
    for (i, coord) in coords.iter().enumerate() {
        out[i] = if let Some(pct) = coord.strip_suffix('%') {
            pct.parse::<f32>().ok()? / 100.0
        } else {
            coord.parse::<f32>().ok()?
        };
    }
    Some(out)
}

/// `<color> [<pos>]` — position as `%` or 0..=1 number.
///
/// The color may be a top-level `var()` reference (kept deferred until
/// resolve folds it into a concrete `Color`).
fn parse_gradient_stop(part: &str) -> Option<GradientStop> {
    // Position is a trailing bare token; the color may contain spaces.
    let mut bits: Vec<&str> = part.rsplitn(2, char::is_whitespace).collect();
    let (color_text, position) = if bits.len() == 2 {
        bits.reverse();
        let pos = if let Some(pct) = bits[1].strip_suffix('%') {
            pct.parse::<f32>().ok()? / 100.0
        } else if let Ok(n) = bits[1].parse::<f32>() {
            // Bare numbers below 1 are positions; larger ones belong to
            // space-separated color syntax (handled by falling through).
            if n > 1.0 {
                return parse_color_or_var(part).map(|color| GradientStop {
                    color,
                    position: None,
                });
            }
            n
        } else {
            return parse_color_or_var(part).map(|color| GradientStop {
                color,
                position: None,
            });
        };
        (bits[0], Some(pos))
    } else {
        (part, None)
    };
    parse_color_or_var(color_text).map(|color| GradientStop { color, position })
}

/// Parse a concrete color, or a top-level `var()` / `var(--x, <color>)`
/// reference kept deferred as [`StyleValue::Var`]/[`StyleValue::VarFallback`].
pub fn parse_color_or_var(text: &str) -> Option<StyleValue> {
    let s = text.trim();
    if let Some(c) = parse_color(s) {
        return Some(StyleValue::Color(c));
    }
    match parse_value(s) {
        v @ (StyleValue::Var(_) | StyleValue::VarFallback { .. }) => Some(v),
        _ => None,
    }
}

/// One parsed `box-shadow` layer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BoxShadow {
    /// Horizontal offset in px.
    pub offset_x: f32,
    /// Vertical offset in px.
    pub offset_y: f32,
    /// Blur radius in px.
    pub blur: f32,
    /// Spread radius in px.
    pub spread: f32,
    /// Shadow color (`Color`, or `Var`/`VarFallback` until resolve folds it).
    pub color: StyleValue,
    /// Whether this is an `inset` shadow.
    pub inset: bool,
}

/// Parse `box-shadow: [inset] x y [blur] [spread] [color]` (comma list).
///
/// Lengths accept `px` or bare numbers (bare = px, like `Length`).
/// Returns None when any layer is malformed — hosts keep the raw
/// keyword then, never a half-parsed shadow.
pub fn parse_box_shadow(raw: &str) -> Option<Vec<BoxShadow>> {
    let mut out = Vec::new();
    for layer in split_top_commas(raw) {
        out.push(parse_shadow_layer(layer.trim())?);
    }
    if out.is_empty() {
        return None;
    }
    Some(out)
}

fn parse_shadow_layer(layer: &str) -> Option<BoxShadow> {
    if layer.is_empty() || layer.eq_ignore_ascii_case("none") {
        return None;
    }
    // Keep function colors (rgb(…), color-mix(…)) in one piece.
    let tokens = split_shadow_tokens(layer);
    let mut tokens = tokens.as_slice();
    let mut inset = false;
    if tokens
        .first()
        .is_some_and(|t| t.eq_ignore_ascii_case("inset"))
    {
        inset = true;
        tokens = &tokens[1..];
    }
    if tokens.len() < 2 {
        return None;
    }
    // Color trails (`… #000`) or leads (`red 0 2px …`); lengths are 2-4.
    // Colors may be `var()` references (kept deferred until resolve).
    let (color, lengths) = match parse_color_or_var(tokens[0]) {
        Some(color) => {
            let mut lengths = Vec::new();
            for token in &tokens[1..] {
                match parse_shadow_length(token) {
                    Some(v) if lengths.len() < 4 => lengths.push(v),
                    _ => return None,
                }
            }
            (color, lengths)
        }
        None => {
            let mut lengths = Vec::new();
            let mut rest = tokens.len();
            for (i, token) in tokens.iter().enumerate() {
                match parse_shadow_length(token) {
                    Some(v) if lengths.len() < 4 => lengths.push(v),
                    _ => {
                        rest = i;
                        break;
                    }
                }
            }
            let color = if rest < tokens.len() {
                parse_color_or_var(&tokens[rest..].join(" "))?
            } else {
                StyleValue::Color(Color::rgb(0, 0, 0))
            };
            (color, lengths)
        }
    };
    if !(2..=4).contains(&lengths.len()) {
        return None;
    }
    let blur = lengths.get(2).copied().unwrap_or(0.0);
    let spread = lengths.get(3).copied().unwrap_or(0.0);
    if blur < 0.0 || spread < 0.0 {
        return None;
    }
    Some(BoxShadow {
        offset_x: lengths[0],
        offset_y: lengths[1],
        blur,
        spread,
        color,
        inset,
    })
}

/// Split on whitespace, keeping `(…)` groups whole.
fn split_shadow_tokens(layer: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut depth = 0u32;
    let mut start: Option<usize> = None;
    for (i, c) in layer.char_indices() {
        match c {
            '(' => {
                depth += 1;
                if start.is_none() {
                    start = Some(i);
                }
            }
            ')' => {
                depth = depth.saturating_sub(1);
                if start.is_none() {
                    start = Some(i);
                }
            }
            _ if c.is_whitespace() && depth == 0 => {
                if let Some(s) = start.take() {
                    out.push(&layer[s..i]);
                }
            }
            _ => {
                if start.is_none() {
                    start = Some(i);
                }
            }
        }
    }
    if let Some(s) = start {
        out.push(&layer[s..]);
    }
    out
}

fn parse_shadow_length(token: &str) -> Option<f32> {
    let num = token.strip_suffix("px").unwrap_or(token).trim();
    // Bare numbers count as px; anything else (colors, keywords) stops.
    if num.chars().any(|c| c.is_alphabetic() || c == '%') {
        return None;
    }
    num.parse::<f32>().ok().filter(|v| v.is_finite())
}

/// Expand `border:` / `background:` / `font:` shorthands into longhand
/// declarations (in author order). Returns None when the value is not an
/// expandable shorthand — the caller keeps the single declaration (which
/// then typically warns as an unknown property, honestly unsupported).
///
/// One-way: `fmt` prints the longhands back, never re-collapses.
pub fn expand_shorthand(prop: &str, raw: &str) -> Option<Vec<(String, StyleValue)>> {
    let value = raw.trim().trim_end_matches(';').trim();
    // CSS-wide keywords fan out so `inherit`/`initial`/`unset` keep working.
    if let Some(word) = ["inherit", "initial", "unset"]
        .into_iter()
        .find(|w| value.eq_ignore_ascii_case(w))
    {
        let longhands: &[&str] = match prop {
            "border" => &["border-width", "border-style", "border-color"],
            "background" => &["background", "background-image"],
            "font" => &["font-size", "font-weight", "font-family", "line-height"],
            _ => return None,
        };
        return Some(
            longhands
                .iter()
                .map(|k| (k.to_string(), StyleValue::Keyword(word.to_string())))
                .collect(),
        );
    }
    match prop {
        "border" => expand_border(value),
        "background" => expand_background(value),
        "font" => expand_font(value),
        _ => None,
    }
}

/// `border: <width> <style> <color>` in any order (1-3 parts, each class ≤ once).
fn expand_border(value: &str) -> Option<Vec<(String, StyleValue)>> {
    let tokens = split_shadow_tokens(value);
    if tokens.is_empty() || tokens.len() > 3 {
        return None;
    }
    let mut width = None;
    let mut style = None;
    let mut color = None;
    for token in tokens {
        if width.is_none() && is_border_width(token) {
            width = Some(border_width_value(token)?);
        } else if style.is_none() && is_border_style(token) {
            style = Some(StyleValue::Keyword(token.to_ascii_lowercase()));
        } else if color.is_none() && parse_color_or_var(token).is_some() {
            color = Some(parse_color_or_var(token)?);
        } else {
            return None;
        }
    }
    let mut out = Vec::new();
    if let Some(w) = width {
        out.push(("border-width".to_string(), w));
    }
    if let Some(s) = style {
        out.push(("border-style".to_string(), s));
    }
    if let Some(c) = color {
        out.push(("border-color".to_string(), c));
    }
    if out.is_empty() {
        return None;
    }
    Some(out)
}

fn is_border_width(token: &str) -> bool {
    match token.to_ascii_lowercase().as_str() {
        "thin" | "medium" | "thick" => true,
        _ => matches!(
            parse_value(token),
            StyleValue::Length(_) | StyleValue::Number(_)
        ),
    }
}

fn border_width_value(token: &str) -> Option<StyleValue> {
    match token.to_ascii_lowercase().as_str() {
        "thin" => Some(StyleValue::Length(1.0)),
        "medium" => Some(StyleValue::Length(3.0)),
        "thick" => Some(StyleValue::Length(5.0)),
        _ => match parse_value(token) {
            v @ (StyleValue::Length(_) | StyleValue::Number(_)) => Some(v),
            _ => None,
        },
    }
}

fn is_border_style(token: &str) -> bool {
    matches!(
        token.to_ascii_lowercase().as_str(),
        "none"
            | "hidden"
            | "dotted"
            | "dashed"
            | "solid"
            | "double"
            | "groove"
            | "ridge"
            | "inset"
            | "outset"
    )
}

/// `background: <color> [<image>]` / single `url()`/`svg()`.
///
/// Single colors/gradients/`var()` stay on `background` (no expansion);
/// multi-layers (`a, b`) and gradient-over-color stay single keywords
/// (the bridge paints one layer).
fn expand_background(value: &str) -> Option<Vec<(String, StyleValue)>> {
    // `none` clears the image layer.
    if value.eq_ignore_ascii_case("none") {
        return Some(vec![(
            "background-image".to_string(),
            StyleValue::Keyword("none".to_string()),
        )]);
    }
    let single = parse_value(value);
    match &single {
        StyleValue::Url(_) | StyleValue::SvgRef(_) => {
            return Some(vec![("background-image".to_string(), single)]);
        }
        _ => {}
    }
    let tokens = split_shadow_tokens(value);
    if tokens.len() != 2 {
        return None;
    }
    let mut color = None;
    let mut image = None;
    for token in tokens {
        match parse_value(token) {
            StyleValue::Url(_) | StyleValue::SvgRef(_) if image.is_none() => {
                image = Some(parse_value(token));
            }
            // Unparseable text: keep the single keyword. A gradient plus a
            // solid color also stays single (both would share `background`,
            // which holds one layer); gradient plus *image* expands, since
            // those live on different keys.
            StyleValue::Keyword(_) => return None,
            StyleValue::Url(_) | StyleValue::SvgRef(_) => return None,
            v if color.is_none() => {
                color = Some(v);
            }
            _ => return None,
        }
    }
    Some(vec![
        ("background".to_string(), color?),
        ("background-image".to_string(), image?),
    ])
}

/// `font: [weight] size[/line-height] family…` (no `font-style`: rejected).
fn expand_font(value: &str) -> Option<Vec<(String, StyleValue)>> {
    let tokens = split_font_tokens(value);
    if tokens.len() < 2 {
        return None;
    }
    // Find the size token (first Length/Number, maybe with /line-height).
    let mut size_at = None;
    for (i, token) in tokens.iter().enumerate() {
        let head = token.split_once('/').map(|(h, _)| h).unwrap_or(token);
        if matches!(
            parse_value(head),
            StyleValue::Length(_) | StyleValue::Number(_)
        ) {
            size_at = Some(i);
            break;
        }
    }
    let size_at = size_at?;
    // ≤1 token before size: the weight (a style word rejects).
    if size_at > 1 {
        return None;
    }
    let mut out = Vec::new();
    if size_at == 1 {
        out.push(("font-weight".to_string(), parse_font_weight(&tokens[0])?));
    }
    let (size_text, line_height) = match tokens[size_at].split_once('/') {
        Some((size, lh)) => (size, Some(lh)),
        None => (tokens[size_at].as_str(), None),
    };
    let size = match parse_value(size_text.trim()) {
        v @ (StyleValue::Length(_) | StyleValue::Number(_)) => v,
        _ => return None,
    };
    out.push(("font-size".to_string(), size));
    if let Some(lh) = line_height {
        match parse_value(lh.trim()) {
            v @ (StyleValue::Length(_) | StyleValue::Number(_)) => {
                out.push(("line-height".to_string(), v));
            }
            _ => return None,
        }
    }
    let family = tokens[size_at + 1..].join(" ").trim().to_string();
    if family.is_empty() {
        return None;
    }
    out.push(("font-family".to_string(), font_family_value(&family)));
    Some(out)
}

fn parse_font_weight(token: &str) -> Option<StyleValue> {
    if matches!(
        token.to_ascii_lowercase().as_str(),
        "normal" | "bold" | "bolder" | "lighter"
    ) {
        return Some(StyleValue::Keyword(token.to_ascii_lowercase()));
    }
    match parse_value(token) {
        StyleValue::Number(n) if (100.0..=900.0).contains(&n) => Some(StyleValue::Number(n)),
        _ => None,
    }
}

fn font_family_value(family: &str) -> StyleValue {
    let family = family.trim();
    if family.len() >= 2
        && ((family.starts_with('"') && family.ends_with('"'))
            || (family.starts_with('\'') && family.ends_with('\'')))
    {
        StyleValue::String(family[1..family.len() - 1].to_string())
    } else {
        StyleValue::Keyword(family.to_string())
    }
}

/// Split on whitespace, keeping `(…)`, quotes and `url("a b")` whole.
fn split_font_tokens(value: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut depth = 0u32;
    let mut quote = None;
    let mut current = String::new();
    let mut chars = value.chars().peekable();
    while let Some(c) = chars.next() {
        if let Some(q) = quote {
            current.push(c);
            if c == '\\' {
                if let Some(e) = chars.next() {
                    current.push(e);
                }
            } else if c == q {
                quote = None;
            }
            continue;
        }
        match c {
            '"' | '\'' => {
                quote = Some(c);
                current.push(c);
            }
            '(' => {
                depth += 1;
                current.push(c);
            }
            ')' => {
                depth = depth.saturating_sub(1);
                current.push(c);
            }
            _ if c.is_whitespace() && depth == 0 => {
                if !current.is_empty() {
                    out.push(std::mem::take(&mut current));
                }
            }
            _ => current.push(c),
        }
    }
    if !current.is_empty() {
        out.push(current);
    }
    out
}

/// Parse `#rgb`, `#rrggbb`, `#rrggbbaa`, `rgb()`/`rgba()` (comma or
/// space + `/ alpha` forms), `hsl()`/`hsla()`, `color-mix()`,
/// `alpha()`, or named colors.
pub fn parse_color(raw: &str) -> Option<Color> {
    let s = raw.trim();
    if let Some(c) = parse_named_color(s) {
        return Some(c);
    }
    let lower = s.to_ascii_lowercase();
    if let Some(c) = parse_hsl_function(&lower) {
        return Some(c);
    }
    if let Some(c) = parse_oklch_function(&lower) {
        return Some(c);
    }
    if let Some(c) = parse_color_mix(&lower) {
        return Some(c);
    }
    if let Some(c) = parse_alpha_function(&lower) {
        return Some(c);
    }
    if let Some(hex) = s.strip_prefix('#') {
        return parse_hex(hex);
    }
    if let Some(inner) = lower
        .strip_prefix("rgba(")
        .or_else(|| lower.strip_prefix("rgb("))
        .and_then(|t| t.strip_suffix(')'))
    {
        // `rgb(255, 0, 0[, 0.5])` or `rgb(255 0 0 [/ 0.5])`.
        let (channels, alpha) = match inner.split_once('/') {
            Some((ch, a)) => (ch, a.trim().parse::<f32>().ok()?),
            None => (inner, 1.0),
        };
        let parts: Vec<&str> = if channels.contains(',') {
            channels.split(',').map(|p| p.trim()).collect()
        } else {
            channels.split_whitespace().collect()
        };
        if parts.len() >= 3 {
            let channel = |p: &str| -> Option<u8> {
                if let Some(pct) = p.strip_suffix('%') {
                    let v: f32 = pct.trim().parse().ok()?;
                    Some((v.clamp(0.0, 100.0) / 100.0 * 255.0).round() as u8)
                } else {
                    p.parse().ok()
                }
            };
            let (r, g, b) = (channel(parts[0])?, channel(parts[1])?, channel(parts[2])?);
            let a = if parts.len() >= 4 && !channels.contains('/') {
                // Comma fourth slot is legacy alpha.
                parts[3].parse().unwrap_or(1.0)
            } else {
                alpha
            };
            return Some(Color::rgba(r, g, b, a.clamp(0.0, 1.0)));
        }
    }
    None
}

/// `color-mix(in srgb, <color> [<pct>], <color> [<pct>])` → sRGB mix.
///
/// Weights default to 50/50 and normalize when they do not sum to 100.
/// Only `in srgb` is supported (other spaces return None).
pub fn parse_color_mix(lower_trimmed: &str) -> Option<Color> {
    let inner = lower_trimmed
        .strip_prefix("color-mix(")
        .and_then(|t| t.strip_suffix(')'))?;
    let (space_part, rest) = inner.split_once(',')?;
    if space_part.trim() != "in srgb" {
        return None;
    }
    let (first, second) = split_top_level_comma(rest);
    let second = second?;
    let (a, wa) = split_color_weight(first)?;
    let (b, wb) = split_color_weight(second)?;
    let ca = parse_color(a)?;
    let cb = parse_color(b)?;
    let total = (wa + wb).max(1e-6);
    let (wa, wb) = (wa / total, wb / total);
    let mix = |x: u8, y: u8| ((x as f32) * wa + (y as f32) * wb).round() as u8;
    Some(Color::rgba(
        mix(ca.r, cb.r),
        mix(ca.g, cb.g),
        mix(ca.b, cb.b),
        (ca.a * wa + cb.a * wb).clamp(0.0, 1.0),
    ))
}

/// Split `"<color> [<pct>]"` into color text + weight (default 50).
fn split_color_weight(part: &str) -> Option<(&str, f32)> {
    let part = part.trim();
    // Weight is a trailing bare percentage separated by whitespace.
    let mut bits: Vec<&str> = part.rsplitn(2, char::is_whitespace).collect();
    if bits.len() == 2 {
        bits.reverse();
        if let Some(pct) = bits[1].strip_suffix('%') {
            if let Ok(w) = pct.trim().parse::<f32>() {
                return Some((bits[0], w.max(0.0)));
            }
        }
    }
    Some((part, 50.0))
}

/// `oklch(L C H [/ A])` → sRGB (Oklch → linear sRGB → gamma, clamped).
///
/// `L`: bare 0..=1 or `%`; `C`: bare (≈0..0.37) or `%` (100% = 0.4 per
/// CSS Color 4); `H`: bare/`deg`/`turn`. Out-of-gamut clamps per channel
/// (no gamut mapping — document the clip).
pub fn parse_oklch_function(lower_trimmed: &str) -> Option<Color> {
    let inner = lower_trimmed
        .strip_prefix("oklch(")
        .and_then(|t| t.strip_suffix(')'))?;
    let (channels, alpha) = match inner.split_once('/') {
        Some((ch, a)) => (ch, parse_oklch_alpha(a)?),
        None => (inner, 1.0),
    };
    let parts: Vec<&str> = if channels.contains(',') {
        channels.split(',').map(|p| p.trim()).collect()
    } else {
        channels.split_whitespace().collect()
    };
    if parts.len() != 3 {
        return None;
    }
    let l = if let Some(pct) = parts[0].strip_suffix('%') {
        pct.trim().parse::<f32>().ok()? / 100.0
    } else {
        parts[0].parse::<f32>().ok()?
    };
    let c = if let Some(pct) = parts[1].strip_suffix('%') {
        pct.trim().parse::<f32>().ok()? * 0.004
    } else {
        parts[1].parse::<f32>().ok()?
    };
    let h = {
        let h = parts[2].trim();
        if let Some(deg) = h.strip_suffix("deg") {
            deg.trim().parse::<f32>().ok()?
        } else if let Some(turn) = h.strip_suffix("turn") {
            turn.trim().parse::<f32>().ok()? * 360.0
        } else {
            h.parse::<f32>().ok()?
        }
    };
    if !l.is_finite() || !c.is_finite() || !h.is_finite() {
        return None;
    }
    let (r, g, b) = oklch_to_srgb(l.clamp(0.0, 1.0), c.max(0.0), h);
    Some(Color::rgba(r, g, b, alpha))
}

fn parse_oklch_alpha(raw: &str) -> Option<f32> {
    let a = raw.trim();
    let v = if let Some(pct) = a.strip_suffix('%') {
        pct.trim().parse::<f32>().ok()? / 100.0
    } else {
        a.parse::<f32>().ok()?
    };
    if !v.is_finite() {
        return None;
    }
    Some(v.clamp(0.0, 1.0))
}

/// Oklch → sRGB bytes (matrices by Björn Ottosson; clipped, not mapped).
fn oklch_to_srgb(l: f32, c: f32, h_deg: f32) -> (u8, u8, u8) {
    let h = h_deg.to_radians();
    let (a, b) = (c * h.cos(), c * h.sin());
    // f32-rounded matrices (more digits are not representable in f32).
    let l_ = l + 0.39633778 * a + 0.21580376 * b;
    let m_ = l - 0.10556135 * a - 0.06385417 * b;
    let s_ = l - 0.08948418 * a - 1.2914855 * b;
    let (l, m, s) = (l_ * l_ * l_, m_ * m_ * m_, s_ * s_ * s_);
    let r = 4.0767417 * l - 3.3077116 * m + 0.23096993 * s;
    let g = -1.268438 * l + 2.6097574 * m - 0.3413194 * s;
    let b = -0.0041960863 * l - 0.7034186 * m + 1.7076147 * s;
    (gamma_u8(r), gamma_u8(g), gamma_u8(b))
}

fn gamma_u8(v: f32) -> u8 {
    if !v.is_finite() {
        return 0;
    }
    let srgb = if v <= 0.0031308 {
        12.92 * v
    } else {
        1.055 * v.powf(1.0 / 2.4) - 0.055
    };
    (srgb.clamp(0.0, 1.0) * 255.0).round() as u8
}

/// WCAG relative luminance of an sRGB color (0 = black, 1 = white).
pub fn relative_luminance(color: Color) -> f32 {
    fn linear(c: u8) -> f32 {
        let v = f32::from(c) / 255.0;
        if v <= 0.04045 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    }
    0.2126 * linear(color.r) + 0.7152 * linear(color.g) + 0.0722 * linear(color.b)
}

/// Pick `dark` for bright bases, `light` for dark ones (WCAG 0.179 cut).
pub fn contrast_pick(base: Color, dark: Color, light: Color) -> Color {
    if relative_luminance(base) > 0.179 {
        dark
    } else {
        light
    }
}

/// `auto-contrast(<color> [, <dark>] [, <light>])` — black/white (or the
/// given pair) picked by WCAG luminance. `var()` arguments stay deferred
/// ([`StyleValue::AutoContrast`]) until resolve folds them.
pub fn parse_auto_contrast_fn(raw: &str) -> Option<StyleValue> {
    let s = raw.trim();
    if !s.to_ascii_lowercase().starts_with("auto-contrast(") || !s.ends_with(')') {
        return None;
    }
    // Slice the original (byte lengths match: ASCII prefix).
    let body = &s["auto-contrast(".len()..s.len() - 1];
    let parts = split_top_commas(body);
    if !(1..=3).contains(&parts.len()) {
        return None;
    }
    let mut it = parts.iter();
    let base = parse_color_or_var(it.next()?.trim())?;
    let dark = match it.next() {
        None => StyleValue::Color(Color::rgb(0, 0, 0)),
        Some(p) => parse_color_or_var(p.trim())?,
    };
    let light = match it.next() {
        None => StyleValue::Color(Color::rgb(255, 255, 255)),
        Some(p) => parse_color_or_var(p.trim())?,
    };
    debug_assert!(it.next().is_none());
    match (&base, &dark, &light) {
        (StyleValue::Color(b), StyleValue::Color(d), StyleValue::Color(l)) => {
            Some(StyleValue::Color(contrast_pick(*b, *d, *l)))
        }
        _ => Some(StyleValue::AutoContrast {
            base: Box::new(base),
            dark: Box::new(dark),
            light: Box::new(light),
        }),
    }
}

/// `alpha(<color>, <amount>)` — override alpha (0..=1 or %).
pub fn parse_alpha_function(lower_trimmed: &str) -> Option<Color> {
    let inner = lower_trimmed
        .strip_prefix("alpha(")
        .and_then(|t| t.strip_suffix(')'))?;
    let (color_part, amount_part) = split_top_level_comma(inner);
    let amount_part = amount_part?;
    let mut base = parse_color(color_part.trim())?;
    base.a = parse_alpha_amount(amount_part)?;
    Some(base)
}

/// Parse an `alpha()` amount (`0..=1` or `%`) into 0..=1.
fn parse_alpha_amount(amount_part: &str) -> Option<f32> {
    let amount = amount_part.trim();
    let a = if let Some(pct) = amount.strip_suffix('%') {
        pct.trim().parse::<f32>().ok()? / 100.0
    } else {
        amount.parse::<f32>().ok()?
    };
    Some(a.clamp(0.0, 1.0))
}

/// sRGB mix of two concrete colors with percent weights (normalized).
pub fn mix_concrete(ca: Color, wa: f32, cb: Color, wb: f32) -> Color {
    let total = (wa + wb).max(1e-6);
    let (wa, wb) = (wa / total, wb / total);
    let mix = |x: u8, y: u8| ((x as f32) * wa + (y as f32) * wb).round() as u8;
    Color::rgba(
        mix(ca.r, cb.r),
        mix(ca.g, cb.g),
        mix(ca.b, cb.b),
        (ca.a * wa + cb.a * wb).clamp(0.0, 1.0),
    )
}

/// Parse `color-mix()` / `alpha()` whose color arguments hold `var()`.
///
/// Returns `Color` when both sides are concrete (same result as
/// [`parse_color_mix`]/[`parse_alpha_function`]); otherwise a deferred
/// [`StyleValue::ColorMix`] that resolve folds once vars are known.
/// Case-preserving (unlike the lowercased helpers) so `--Names` survive.
/// Returns None for non-supported spaces or malformed input.
pub fn parse_color_fn_deferred(raw: &str) -> Option<StyleValue> {
    let s = raw.trim();
    if !s.ends_with(')') {
        return None;
    }
    // Case-insensitive head match, but slice the ORIGINAL text (byte
    // lengths match for ASCII prefixes) so `--Names` keep their case.
    let lower = s.to_ascii_lowercase();
    if lower.starts_with("color-mix(") {
        let body = &s["color-mix(".len()..s.len() - 1];
        let (space_part, rest) = body.split_once(',')?;
        if !space_part.trim().eq_ignore_ascii_case("in srgb") {
            return None;
        }
        let (first, second) = split_top_level_comma(rest);
        let second = second?;
        let (a_text, wa) = split_color_weight(first)?;
        let (b_text, wb) = split_color_weight(second)?;
        let a = parse_color_or_var(a_text)?;
        let b = parse_color_or_var(b_text)?;
        if let (StyleValue::Color(ca), StyleValue::Color(cb)) = (&a, &b) {
            return Some(StyleValue::Color(mix_concrete(*ca, wa, *cb, wb)));
        }
        return Some(StyleValue::ColorMix {
            a: Box::new(a),
            wa,
            b: Box::new(b),
            wb,
        });
    }
    if lower.starts_with("alpha(") {
        let body = &s["alpha(".len()..s.len() - 1];
        let (color_part, amount_part) = split_top_level_comma(body);
        let amount_part = amount_part?;
        let amount = parse_alpha_amount(amount_part)?;
        let base = parse_color_or_var(color_part.trim())?;
        return match base {
            StyleValue::Color(mut c) => {
                c.a = amount;
                Some(StyleValue::Color(c))
            }
            var => Some(StyleValue::Alpha {
                color: Box::new(var),
                amount,
            }),
        };
    }
    None
}

fn parse_hex(hex: &str) -> Option<Color> {
    let hex = hex.trim();
    match hex.len() {
        3 => {
            let r = u8::from_str_radix(&hex[0..1].repeat(2), 16).ok()?;
            let g = u8::from_str_radix(&hex[1..2].repeat(2), 16).ok()?;
            let b = u8::from_str_radix(&hex[2..3].repeat(2), 16).ok()?;
            Some(Color::rgb(r, g, b))
        }
        6 => {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            Some(Color::rgb(r, g, b))
        }
        8 => {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            let a = u8::from_str_radix(&hex[6..8], 16).ok()? as f32 / 255.0;
            Some(Color::rgba(r, g, b, a))
        }
        _ => None,
    }
}

fn parse_named_color(name: &str) -> Option<Color> {
    Some(match name.to_ascii_lowercase().as_str() {
        "transparent" => Color::rgba(0, 0, 0, 0.0),
        "black" => Color::rgb(0, 0, 0),
        "white" => Color::rgb(255, 255, 255),
        // game-flavored aliases (kept stable for existing sheets)
        "gold" => Color::rgb(235, 200, 120),
        "purple" => Color::rgb(120, 60, 180),
        "magenta" => Color::rgb(220, 80, 220),
        "navy" => Color::rgb(10, 12, 22),
        "neon" => Color::rgb(170, 100, 255),
        // CSS basics
        "red" => Color::rgb(255, 0, 0),
        "green" => Color::rgb(0, 128, 0),
        "lime" => Color::rgb(0, 255, 0),
        "blue" => Color::rgb(0, 0, 255),
        "yellow" => Color::rgb(255, 255, 0),
        "orange" => Color::rgb(255, 165, 0),
        "pink" => Color::rgb(255, 192, 203),
        "cyan" | "aqua" => Color::rgb(0, 255, 255),
        "teal" => Color::rgb(0, 128, 128),
        "gray" | "grey" => Color::rgb(128, 128, 128),
        "silver" => Color::rgb(192, 192, 192),
        "brown" => Color::rgb(165, 42, 42),
        "maroon" => Color::rgb(128, 0, 0),
        "olive" => Color::rgb(128, 128, 0),
        "indigo" => Color::rgb(75, 0, 130),
        "violet" => Color::rgb(238, 130, 238),
        "fuchsia" => Color::rgb(255, 0, 255),
        _ => return None,
    })
}

/// `hsl(210, 60%, 50%)` / `hsla(210, 60%, 50%, 0.5)` → sRGB.
pub fn hsl_to_rgb(h_deg: f32, s_pct: f32, l_pct: f32) -> (u8, u8, u8) {
    let h = h_deg.rem_euclid(360.0) / 360.0;
    let s = (s_pct / 100.0).clamp(0.0, 1.0);
    let l = (l_pct / 100.0).clamp(0.0, 1.0);
    if s <= 0.0 {
        let v = (l * 255.0).round() as u8;
        return (v, v, v);
    }
    let q = if l < 0.5 {
        l * (1.0 + s)
    } else {
        l + s - l * s
    };
    let p = 2.0 * l - q;
    let channel = |t: f32| {
        let t = t.rem_euclid(1.0);
        let v = if t < 1.0 / 6.0 {
            p + (q - p) * 6.0 * t
        } else if t < 0.5 {
            q
        } else if t < 2.0 / 3.0 {
            p + (q - p) * (2.0 / 3.0 - t) * 6.0
        } else {
            p
        };
        (v * 255.0).round().clamp(0.0, 255.0) as u8
    };
    (channel(h + 1.0 / 3.0), channel(h), channel(h - 1.0 / 3.0))
}

fn parse_percent(part: &str) -> Option<f32> {
    part.trim()
        .strip_suffix('%')
        .and_then(|n| n.trim().parse::<f32>().ok())
}

fn parse_hsl_function(s: &str) -> Option<Color> {
    let (name, inner) =
        if let Some(inner) = s.strip_prefix("hsla(").and_then(|t| t.strip_suffix(')')) {
            ("hsla", inner)
        } else {
            let inner = s.strip_prefix("hsl(").and_then(|t| t.strip_suffix(')'))?;
            ("hsl", inner)
        };
    let parts: Vec<&str> = inner.split(',').map(|p| p.trim()).collect();
    if parts.len() < 3 {
        return None;
    }
    let h: f32 = parts[0].trim_end_matches("deg").trim().parse().ok()?;
    let s = parse_percent(parts[1])?;
    let l = parse_percent(parts[2])?;
    let a = if name == "hsla" && parts.len() >= 4 {
        parts[3].parse().unwrap_or(1.0)
    } else {
        1.0
    };
    let (r, g, b) = hsl_to_rgb(h, s, l);
    Some(Color::rgba(r, g, b, a))
}

/// Math expression from `calc()`/`min()`/`max()`/`clamp()`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CalcExpr {
    /// Literal value (length, number, time or percent).
    Lit(Box<StyleValue>),
    /// `a + b`.
    Add(Box<CalcExpr>, Box<CalcExpr>),
    /// `a - b`.
    Sub(Box<CalcExpr>, Box<CalcExpr>),
    /// `a * b` (one side must be a plain number).
    Mul(Box<CalcExpr>, Box<CalcExpr>),
    /// `a / b` (`b` must be a plain non-zero number).
    Div(Box<CalcExpr>, Box<CalcExpr>),
    /// `min(a, …)`.
    Min(Vec<CalcExpr>),
    /// `max(a, …)`.
    Max(Vec<CalcExpr>),
    /// `clamp(lo, val, hi)`.
    Clamp(Box<CalcExpr>, Box<CalcExpr>, Box<CalcExpr>),
    /// `var(--x)` / `var(--x, <fallback>)` inside calc (substituted in resolve).
    Var {
        /// Variable name with the `--` prefix (same as [`StyleValue::Var`]).
        name: String,
        /// Fallback expression when the variable is undefined.
        fallback: Option<Box<CalcExpr>>,
    },
}

/// Unit kind for calc type checking.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CalcUnit {
    Length,
    Number,
    Time,
    Percent,
}

fn calc_unit(value: &StyleValue) -> Option<CalcUnit> {
    match value {
        StyleValue::Length(_) => Some(CalcUnit::Length),
        StyleValue::Number(_) => Some(CalcUnit::Number),
        StyleValue::Time(_) => Some(CalcUnit::Time),
        StyleValue::Percent(_) => Some(CalcUnit::Percent),
        _ => None,
    }
}

fn calc_from_unit(unit: CalcUnit, v: f32) -> StyleValue {
    match unit {
        CalcUnit::Length => StyleValue::Length(v),
        CalcUnit::Number => StyleValue::Number(v),
        CalcUnit::Time => StyleValue::Time(v),
        CalcUnit::Percent => StyleValue::Percent(v),
    }
}

/// Evaluate a calc expression. `percent_base` resolves `%` against a
/// layout base (e.g. parent width); without it, percent math stays
/// unevaluated (`None`) so hosts can resolve it later.
pub fn eval_calc(expr: &CalcExpr, percent_base: Option<f32>) -> Option<StyleValue> {
    // Returns (unit, value) with percents resolved when possible.
    fn eval(expr: &CalcExpr, base: Option<f32>) -> Option<(CalcUnit, f32)> {
        match expr {
            CalcExpr::Lit(v) => {
                let unit = calc_unit(v)?;
                if unit == CalcUnit::Percent {
                    // Percent has no `as_f32`; resolve against the base here.
                    let pct = match v.as_ref() {
                        StyleValue::Percent(p) => *p,
                        _ => return None,
                    };
                    return Some((CalcUnit::Length, base? / 100.0 * pct));
                }
                Some((unit, v.as_f32()?))
            }
            CalcExpr::Add(a, b) => {
                let (ua, va) = eval(a, base)?;
                let (ub, vb) = eval(b, base)?;
                if ua != ub {
                    return None;
                }
                Some((ua, va + vb))
            }
            CalcExpr::Sub(a, b) => {
                let (ua, va) = eval(a, base)?;
                let (ub, vb) = eval(b, base)?;
                if ua != ub {
                    return None;
                }
                Some((ua, va - vb))
            }
            CalcExpr::Mul(a, b) => {
                let (ua, va) = eval(a, base)?;
                let (ub, vb) = eval(b, base)?;
                match (ua, ub) {
                    (CalcUnit::Number, u) => Some((u, va * vb)),
                    (u, CalcUnit::Number) => Some((u, va * vb)),
                    _ => None,
                }
            }
            CalcExpr::Div(a, b) => {
                let (ua, va) = eval(a, base)?;
                let (ub, vb) = eval(b, base)?;
                if ub != CalcUnit::Number || vb == 0.0 {
                    return None;
                }
                Some((ua, va / vb))
            }
            CalcExpr::Min(args) | CalcExpr::Max(args) => {
                let is_max = matches!(expr, CalcExpr::Max(_));
                let mut out: Option<(CalcUnit, f32)> = None;
                for arg in args {
                    let (u, v) = eval(arg, base)?;
                    out = Some(match out {
                        None => (u, v),
                        Some((ou, ov)) => {
                            if ou != u {
                                return None;
                            }
                            if is_max {
                                (u, ov.max(v))
                            } else {
                                (u, ov.min(v))
                            }
                        }
                    });
                }
                out
            }
            CalcExpr::Clamp(lo, val, hi) => {
                let (ul, vl) = eval(lo, base)?;
                let (uv, vv) = eval(val, base)?;
                let (uh, vh) = eval(hi, base)?;
                if ul != uv || uv != uh {
                    return None;
                }
                Some((uv, vl.max(vv).min(vh.max(vl))))
            }
            // Unsubstituted variables cannot evaluate; resolve replaces
            // them via `subst_calc_vars` before calling `eval_calc`.
            CalcExpr::Var { .. } => None,
        }
    }
    let (unit, v) = eval(expr, percent_base)?;
    Some(calc_from_unit(unit, v))
}

/// Substitute `var(--x)` leaves in a calc expression with values from
/// `vars` (only numeric leaves — length/number/time/percent — convert;
/// anything else keeps the `Var`, leaving the expression deferred).
/// Fallbacks are substituted recursively when the variable is undefined.
pub fn subst_calc_vars(
    expr: &CalcExpr,
    vars: &std::collections::HashMap<String, StyleValue>,
) -> CalcExpr {
    fn lit(value: &StyleValue) -> Option<CalcExpr> {
        match value {
            StyleValue::Length(_)
            | StyleValue::Number(_)
            | StyleValue::Time(_)
            | StyleValue::Percent(_) => Some(CalcExpr::Lit(Box::new(value.clone()))),
            _ => None,
        }
    }
    match expr {
        CalcExpr::Lit(_) => expr.clone(),
        CalcExpr::Add(a, b) => CalcExpr::Add(
            Box::new(subst_calc_vars(a, vars)),
            Box::new(subst_calc_vars(b, vars)),
        ),
        CalcExpr::Sub(a, b) => CalcExpr::Sub(
            Box::new(subst_calc_vars(a, vars)),
            Box::new(subst_calc_vars(b, vars)),
        ),
        CalcExpr::Mul(a, b) => CalcExpr::Mul(
            Box::new(subst_calc_vars(a, vars)),
            Box::new(subst_calc_vars(b, vars)),
        ),
        CalcExpr::Div(a, b) => CalcExpr::Div(
            Box::new(subst_calc_vars(a, vars)),
            Box::new(subst_calc_vars(b, vars)),
        ),
        CalcExpr::Min(args) => {
            CalcExpr::Min(args.iter().map(|a| subst_calc_vars(a, vars)).collect())
        }
        CalcExpr::Max(args) => {
            CalcExpr::Max(args.iter().map(|a| subst_calc_vars(a, vars)).collect())
        }
        CalcExpr::Clamp(lo, val, hi) => CalcExpr::Clamp(
            Box::new(subst_calc_vars(lo, vars)),
            Box::new(subst_calc_vars(val, vars)),
            Box::new(subst_calc_vars(hi, vars)),
        ),
        CalcExpr::Var { name, fallback } => match vars.get(name).and_then(lit) {
            Some(replacement) => replacement,
            None => match fallback {
                Some(fb) => subst_calc_vars(fb, vars),
                None => expr.clone(),
            },
        },
    }
}

/// Evaluate `calc()` values in place where fully concrete (no `%`
/// without a base). Returns true when the value changed.
pub fn eval_calc_in_place(value: &mut StyleValue) -> bool {
    // Percent alone without a base cannot resolve to px here.
    if let StyleValue::Calc(expr) = value.clone() {
        if let Some(concrete) = eval_calc(&expr, None) {
            // Keep percent math unevaluated (hosts resolve against layout).
            if !matches!(concrete, StyleValue::Percent(_)) {
                *value = concrete;
                return true;
            }
        }
    }
    false
}

struct CalcParser<'a> {
    chars: &'a [u8],
    pos: usize,
}

impl<'a> CalcParser<'a> {
    fn new(s: &'a str) -> Self {
        Self {
            chars: s.as_bytes(),
            pos: 0,
        }
    }

    fn skip_ws(&mut self) {
        while self.pos < self.chars.len() && self.chars[self.pos].is_ascii_whitespace() {
            self.pos += 1;
        }
    }

    fn peek(&self) -> Option<u8> {
        self.chars.get(self.pos).copied()
    }

    fn eat(&mut self, c: u8) -> bool {
        if self.peek() == Some(c) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn parse_expr(&mut self) -> Option<CalcExpr> {
        let mut node = self.parse_term()?;
        loop {
            self.skip_ws();
            if self.eat(b'+') {
                let rhs = self.parse_term()?;
                node = CalcExpr::Add(Box::new(node), Box::new(rhs));
            } else if self.eat(b'-') {
                let rhs = self.parse_term()?;
                node = CalcExpr::Sub(Box::new(node), Box::new(rhs));
            } else {
                break;
            }
        }
        Some(node)
    }

    fn parse_term(&mut self) -> Option<CalcExpr> {
        let mut node = self.parse_factor()?;
        loop {
            self.skip_ws();
            if self.eat(b'*') {
                let rhs = self.parse_factor()?;
                node = CalcExpr::Mul(Box::new(node), Box::new(rhs));
            } else if self.eat(b'/') {
                let rhs = self.parse_factor()?;
                node = CalcExpr::Div(Box::new(node), Box::new(rhs));
            } else {
                break;
            }
        }
        Some(node)
    }

    /// Try to parse `var(--name)` / `var(--name, <fallback expr>)`.
    fn parse_var_factor(&mut self) -> Option<CalcExpr> {
        let rest = &self.chars[self.pos..];
        if rest.len() < 6 || !rest[..4].eq_ignore_ascii_case(b"var(") {
            return None;
        }
        self.pos += 4;
        self.skip_ws();
        // Keep the `--` prefix in the name (same convention as StyleValue::Var).
        let name_start = self.pos;
        if self.pos + 2 > self.chars.len()
            || self.chars[self.pos] != b'-'
            || self.chars[self.pos + 1] != b'-'
        {
            return None;
        }
        self.pos += 2;
        let body_start = self.pos;
        while let Some(c) = self.peek() {
            if c.is_ascii_alphanumeric() || c == b'-' || c == b'_' {
                self.pos += 1;
            } else {
                break;
            }
        }
        if body_start == self.pos {
            return None;
        }
        let name = std::str::from_utf8(&self.chars[name_start..self.pos])
            .ok()?
            .to_owned();
        self.skip_ws();
        let fallback = if self.eat(b',') {
            let fb = self.parse_expr()?;
            self.skip_ws();
            Some(Box::new(fb))
        } else {
            None
        };
        if !self.eat(b')') {
            return None;
        }
        Some(CalcExpr::Var { name, fallback })
    }

    fn parse_factor(&mut self) -> Option<CalcExpr> {
        self.skip_ws();
        if let Some(v) = self.parse_var_factor() {
            return Some(v);
        }
        if self.eat(b'(') {
            let node = self.parse_expr()?;
            self.skip_ws();
            if !self.eat(b')') {
                return None;
            }
            return Some(node);
        }
        // Signed number with optional unit.
        let start = self.pos;
        if self.peek() == Some(b'+') || self.peek() == Some(b'-') {
            self.pos += 1;
        }
        let num_start = self.pos;
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() || c == b'.' {
                self.pos += 1;
            } else {
                break;
            }
        }
        if num_start == self.pos {
            return None;
        }
        let num: f32 = std::str::from_utf8(&self.chars[num_start..self.pos])
            .ok()?
            .parse()
            .ok()?;
        let sign = if self.chars[start] == b'-' { -1.0 } else { 1.0 };
        // Unit letters / %.
        let unit_start = self.pos;
        while let Some(c) = self.peek() {
            if c.is_ascii_alphabetic() || c == b'%' {
                self.pos += 1;
            } else {
                break;
            }
        }
        let unit = std::str::from_utf8(&self.chars[unit_start..self.pos])
            .ok()?
            .to_ascii_lowercase();
        let value = match unit.as_str() {
            "" => StyleValue::Number(sign * num),
            "px" => StyleValue::Length(sign * num),
            "%" => StyleValue::Percent(sign * num),
            "s" => StyleValue::Time(sign * num),
            "ms" => StyleValue::Time(sign * num / 1000.0),
            _ => return None,
        };
        Some(CalcExpr::Lit(Box::new(value)))
    }

    fn parse_arg_list(&mut self) -> Option<Vec<CalcExpr>> {
        let mut args = Vec::new();
        loop {
            self.skip_ws();
            if self.peek() == Some(b')') || self.peek().is_none() {
                break;
            }
            args.push(self.parse_expr()?);
            self.skip_ws();
            if self.eat(b',') {
                continue;
            }
            break;
        }
        Some(args)
    }

    fn trailing_end(&mut self) -> bool {
        self.skip_ws();
        self.pos == self.chars.len()
    }
}

/// Parse `calc(…)` / `min(…)` / `max(…)` / `clamp(…)` into an AST.
/// Returns None when the text is not one of these calls or is malformed.
pub fn parse_calc_func(raw: &str) -> Option<CalcExpr> {
    let s = raw.trim();
    let lower = s.to_ascii_lowercase();
    let (name, inner) = ["calc(", "min(", "max(", "clamp("]
        .into_iter()
        .find_map(|prefix| {
            if lower.starts_with(prefix) && s.ends_with(')') && s.len() > prefix.len() {
                Some((&prefix[..prefix.len() - 1], &s[prefix.len()..s.len() - 1]))
            } else {
                None
            }
        })?;
    let mut p = CalcParser::new(inner);
    let expr = match name {
        "calc" => {
            let node = p.parse_expr()?;
            if !p.trailing_end() {
                return None;
            }
            node
        }
        "min" | "max" => {
            let args = p.parse_arg_list()?;
            if !p.trailing_end() || args.is_empty() {
                return None;
            }
            if name == "min" {
                CalcExpr::Min(args)
            } else {
                CalcExpr::Max(args)
            }
        }
        "clamp" => {
            let args = p.parse_arg_list()?;
            if !p.trailing_end() || args.len() != 3 {
                return None;
            }
            let mut it = args.into_iter();
            CalcExpr::Clamp(
                Box::new(it.next()?),
                Box::new(it.next()?),
                Box::new(it.next()?),
            )
        }
        _ => return None,
    };
    Some(expr)
}

/// Parse a single property value token string.
pub fn parse_value(raw: &str) -> StyleValue {
    let s = raw.trim().trim_end_matches(';').trim();
    // var(--token) or var(--token, fallback)
    if let Some(inner) = s
        .strip_prefix("var(")
        .and_then(|t| t.strip_suffix(')'))
        .map(|t| t.trim())
    {
        let (name, fallback) = split_top_level_comma(inner);
        let name = name.trim();
        let name = name.trim_matches(|c| c == '"' || c == '\'');
        if !name.is_empty() {
            let canonical = if name.starts_with("--") {
                name.to_string()
            } else {
                format!("--{name}")
            };
            if let Some(fallback) = fallback.filter(|value| !value.trim().is_empty()) {
                return StyleValue::VarFallback {
                    name: canonical,
                    fallback: Box::new(parse_value(fallback)),
                };
            }
            return StyleValue::Var(canonical);
        }
    }
    // url("path") / url('path')
    if let Some(inner) = s
        .strip_prefix("url(")
        .and_then(|t| t.strip_suffix(')'))
        .map(|t| t.trim())
    {
        let path = inner.trim_matches(|c| c == '"' || c == '\'');
        if !path.is_empty() {
            return StyleValue::Url(path.to_string());
        }
    }
    // svg(name)
    if let Some(inner) = s
        .strip_prefix("svg(")
        .and_then(|t| t.strip_suffix(')'))
        .map(|t| t.trim())
    {
        let name = inner.trim_matches(|c| c == '"' || c == '\'');
        if !name.is_empty() {
            return StyleValue::SvgRef(name.to_string());
        }
    }
    // calc() / min() / max() / clamp(): keep the AST, evaluate the
    // fully-concrete cases now so hosts read plain values.
    if let Some(expr) = parse_calc_func(s) {
        let mut value = StyleValue::Calc(expr);
        eval_calc_in_place(&mut value);
        return value;
    }
    // Bare percentage (layout base resolves it later).
    if let Some(num) = s.strip_suffix('%') {
        if let Ok(v) = num.trim().parse::<f32>() {
            return StyleValue::Percent(v);
        }
    }
    if let Some(g) = parse_gradient(s) {
        return StyleValue::Gradient(g);
    }
    if let Some(c) = parse_color(s) {
        return StyleValue::Color(c);
    }
    // `color-mix()` / `alpha()` with `var()` colors (concrete ones folded
    // above by `parse_color`).
    if let Some(v) = parse_color_fn_deferred(s) {
        return v;
    }
    // `auto-contrast()` (folds when concrete, defers over `var()`).
    if let Some(v) = parse_auto_contrast_fn(s) {
        return v;
    }
    // `box-shadow` layers with `var()` colors parse structured so resolve
    // can substitute them. Gated on `var(`: bare multi-length values like
    // `margin: 0 0` must keep the Keyword path.
    if s.to_ascii_lowercase().contains("var(") {
        if let Some(layers) = parse_box_shadow(s) {
            return StyleValue::Shadow(layers);
        }
    }
    if has_time_suffix(s) {
        if let Some(seconds) = parse_time_seconds_token(s) {
            return StyleValue::Time(seconds);
        }
    }
    if let Some(num) = s.strip_suffix("px").or(Some(s)) {
        if let Ok(v) = num.trim().parse::<f32>() {
            if s.ends_with("px") || !s.contains(char::is_alphabetic) {
                return if s.ends_with("px") {
                    StyleValue::Length(v)
                } else {
                    StyleValue::Number(v)
                };
            }
        }
    }
    if (s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')) {
        return StyleValue::String(s[1..s.len() - 1].to_string());
    }
    StyleValue::Keyword(s.to_string())
}

fn has_time_suffix(raw: &str) -> bool {
    raw.strip_suffix("ms").is_some() || raw.strip_suffix('s').is_some()
}

/// Parse a time token into seconds.
///
/// Unitless tokens retain the historical shorthand behavior and are interpreted
/// as seconds. Full declaration values only become [`StyleValue::Time`] when a
/// `s` or `ms` suffix is present.
pub(crate) fn parse_time_seconds_token(raw: &str) -> Option<f32> {
    let raw = raw.trim();
    if let Some(ms) = raw.strip_suffix("ms") {
        return ms.trim().parse::<f32>().ok().map(|value| value / 1000.0);
    }
    if let Some(seconds) = raw.strip_suffix('s') {
        return seconds.trim().parse::<f32>().ok();
    }
    raw.parse::<f32>().ok()
}

fn split_top_level_comma(input: &str) -> (&str, Option<&str>) {
    let mut quote = None;
    let mut escaped = false;
    let mut depth = 0u32;
    for (index, ch) in input.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if let Some(end_quote) = quote {
            if ch == '\\' {
                escaped = true;
            } else if ch == end_quote {
                quote = None;
            }
            continue;
        }
        match ch {
            '"' | '\'' => quote = Some(ch),
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                return (&input[..index], Some(&input[index + ch.len_utf8()..]));
            }
            _ => {}
        }
    }
    (input, None)
}

/// Known game-UI property names (documentation / tooling).
pub const KNOWN_PROPERTIES: &[&str] = &[
    // colors / chrome
    "background",
    "background-image",
    "background-size",
    "nine-slice",
    "nine-slice-top",
    "nine-slice-right",
    "nine-slice-bottom",
    "nine-slice-left",
    "nine-slice-width",
    "nine-slice-width-top",
    "nine-slice-width-right",
    "nine-slice-width-bottom",
    "nine-slice-width-left",
    "color",
    "border-color",
    "border-width",
    "border-radius",
    "corner-cut",
    "border-style",
    "glow",
    "glow-strength",
    "box-shadow",
    "opacity",
    // box
    "width",
    "height",
    "min-width",
    "min-height",
    "max-width",
    "max-height",
    "margin",
    "margin-x",
    "margin-y",
    "margin-top",
    "margin-right",
    "margin-bottom",
    "margin-left",
    "padding",
    "padding-x",
    "padding-y",
    "padding-top",
    "padding-right",
    "padding-bottom",
    "padding-left",
    "gap",
    // type
    "font-size",
    "font-family",
    "font-weight",
    "letter-spacing",
    "text-align",
    "line-height",
    // transform / motion static
    "x",
    "y",
    "scale",
    "rotate",
    "yaw",
    "pitch",
    "roll",
    "translate-x",
    "translate-y",
    // animation
    "animation",
    "animation-name",
    "animation-duration",
    "animation-delay",
    "animation-timing-function",
    "animation-ease",
    "animation-iteration-count",
    "animation-fill-mode",
    "animation-target",
    "transition",
    "transition-duration",
    "transition-property",
    "transition-timing-function",
    // game chrome
    "icon",
    "icon-size",
    "gold",
    "neon",
    "foil",
    "depth",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_calc_math() {
        assert_eq!(parse_value("calc(8px + 4px)"), StyleValue::Length(12.0));
        assert_eq!(parse_value("calc(2 * 3px + 1px)"), StyleValue::Length(7.0));
        assert_eq!(
            parse_value("calc((10px - 4px) / 2)"),
            StyleValue::Length(3.0)
        );
        assert_eq!(parse_value("min(3px, 8px)"), StyleValue::Length(3.0));
        assert_eq!(parse_value("max(3, 8)"), StyleValue::Number(8.0));
        assert_eq!(
            parse_value("clamp(0px, 20px, 10px)"),
            StyleValue::Length(10.0)
        );
        assert_eq!(parse_value("calc(0.5s + 250ms)"), StyleValue::Time(0.75));
        // Unit mismatch stays unevaluated (hosts decide per property).
        assert!(matches!(parse_value("calc(8px + 4)"), StyleValue::Calc(_)));
        assert!(matches!(parse_value("calc(1px / 0)"), StyleValue::Calc(_)));
        // Percent math needs a layout base.
        assert_eq!(parse_value("50%"), StyleValue::Percent(50.0));
        assert!(matches!(
            parse_value("calc(100% - 8px)"),
            StyleValue::Calc(_)
        ));
        assert_eq!(
            eval_calc(&parse_calc_func("calc(100% - 8px)").unwrap(), Some(200.0)),
            Some(StyleValue::Length(192.0))
        );
        // Malformed calc degrades to keyword (lenient, like unknown values).
        assert_eq!(
            parse_value("calc(1px +)"),
            StyleValue::Keyword("calc(1px +)".into())
        );
    }

    #[test]
    fn parses_color_mix_alpha_and_slash() {
        // Even mix of red and blue.
        assert_eq!(
            parse_color("color-mix(in srgb, red, blue)"),
            Some(Color::rgb(128, 0, 128))
        );
        // 75/25 weights.
        assert_eq!(
            parse_color("color-mix(in srgb, red 75%, blue 25%)"),
            Some(Color::rgb(191, 0, 64))
        );
        // Omitted second weight defaults to 50, then normalizes.
        assert_eq!(
            parse_color("color-mix(in srgb, white 100%, black)"),
            Some(Color::rgb(170, 170, 170))
        );
        assert!(parse_color("color-mix(in xyz, red, blue)").is_none());
        assert!(parse_color("color-mix(in srgb, nope, blue)").is_none());
        // Alpha override.
        assert_eq!(
            parse_color("alpha(red, 0.5)"),
            Some(Color::rgba(255, 0, 0, 0.5))
        );
        assert_eq!(
            parse_color("alpha(#00ff00, 25%)"),
            Some(Color::rgba(0, 255, 0, 0.25))
        );
        // Space + slash rgb forms (alpha clamps).
        assert_eq!(
            parse_color("rgb(255 0 0 / 0.5)"),
            Some(Color::rgba(255, 0, 0, 0.5))
        );
        assert_eq!(parse_color("rgb(100% 0% 0%)"), Some(Color::rgb(255, 0, 0)));
        // Legacy comma forms keep working.
        assert_eq!(parse_color("rgb(1, 2, 3)"), Some(Color::rgb(1, 2, 3)));
        assert_eq!(
            parse_value("color-mix(in srgb, red, blue)"),
            StyleValue::Color(Color::rgb(128, 0, 128))
        );
    }

    #[test]
    fn parses_gradients() {
        let g = parse_gradient("linear-gradient(to bottom, red, blue)").unwrap();
        assert!(!g.radial);
        assert!((g.angle_deg - 180.0).abs() < 1e-4);
        assert_eq!(g.stops.len(), 2);
        assert_eq!(g.sample(0.0), Some(Color::rgb(255, 0, 0)));
        assert_eq!(g.sample(1.0), Some(Color::rgb(0, 0, 255)));
        assert_eq!(g.sample(0.5), Some(Color::rgb(128, 0, 128)));
        assert_eq!(g.fallback_color(), Some(Color::rgb(255, 0, 0)));

        let g = parse_gradient("linear-gradient(45deg, #000 0%, #fff 100%)").unwrap();
        assert!((g.angle_deg - 45.0).abs() < 1e-4);
        assert_eq!(g.stops[0].position, Some(0.0));
        assert_eq!(g.stops[1].position, Some(1.0));

        let g = parse_gradient("linear-gradient(to top right, red, blue)").unwrap();
        assert!((g.angle_deg - 45.0).abs() < 1e-4);

        let g = parse_gradient("radial-gradient(circle at 25% 75%, red, blue)").unwrap();
        assert!(g.radial);
        assert_eq!(g.center, [0.25, 0.75]);

        let g = parse_gradient("radial-gradient(red, blue)").unwrap();
        assert_eq!(g.center, [0.5, 0.5]);

        assert!(parse_gradient("linear-gradient(red)").is_some());
        assert!(parse_gradient("linear-gradient()").is_none());
        assert!(parse_gradient("linear-gradient(to nowhere, red)").is_none());
        assert!(parse_gradient("conic-gradient(red, blue)").is_none());
        assert!(matches!(
            parse_value("linear-gradient(to bottom, red, blue)"),
            StyleValue::Gradient(_)
        ));
    }

    #[test]
    fn parses_box_shadows() {
        let shadows = parse_box_shadow("0 2px 8px #000").unwrap();
        assert_eq!(shadows.len(), 1);
        assert_eq!(shadows[0].offset_x, 0.0);
        assert_eq!(shadows[0].offset_y, 2.0);
        assert_eq!(shadows[0].blur, 8.0);
        assert_eq!(shadows[0].spread, 0.0);
        assert_eq!(shadows[0].color, StyleValue::Color(Color::rgb(0, 0, 0)));
        assert!(!shadows[0].inset);

        let shadows =
            parse_box_shadow("inset red 1px 2px 3px 4px, 0 0 4px rgb(1 2 3 / 0.5)").unwrap();
        assert_eq!(shadows.len(), 2);
        assert!(shadows[0].inset);
        assert_eq!(shadows[0].color, StyleValue::Color(Color::rgb(255, 0, 0)));
        assert_eq!(shadows[0].spread, 4.0);
        assert_eq!(
            shadows[1].color,
            StyleValue::Color(Color::rgba(1, 2, 3, 0.5))
        );

        assert!(parse_box_shadow("none").is_none());
        assert!(parse_box_shadow("0 2px -1px #000").is_none());
        assert!(parse_box_shadow("0 #000").is_none());
        assert!(parse_box_shadow("0 2px nope").is_none());
    }

    #[test]
    fn color_mix_with_var_stays_deferred() {
        assert_eq!(
            parse_value("color-mix(in srgb, var(--gold) 70%, #501e78)"),
            StyleValue::ColorMix {
                a: Box::new(StyleValue::Var("--gold".into())),
                wa: 70.0,
                b: Box::new(StyleValue::Color(Color::rgb(0x50, 0x1e, 0x78))),
                wb: 50.0,
            }
        );
        // Concrete mixes still fold to Color (existing fast path).
        assert_eq!(
            parse_value("color-mix(in srgb, #ffffff 70%, #000000)"),
            StyleValue::Color(parse_color_mix("color-mix(in srgb, #ffffff 70%, #000000)").unwrap())
        );
        // Both sides deferred: weights default to 50/50.
        assert_eq!(
            crate::display_value(&parse_value("color-mix(in srgb, var(--gold), var(--plum))")),
            "color-mix(in srgb, var(--gold) 50%, var(--plum) 50%)"
        );
    }

    #[test]
    fn alpha_with_var_defers_to_color_mix() {
        // Deferred alpha keeps its own node: unlike color-mix against
        // transparent it leaves rgb untouched (matters for translucent bases).
        assert_eq!(
            parse_value("alpha(var(--brand), 0.5)"),
            StyleValue::Alpha {
                color: Box::new(StyleValue::Var("--brand".into())),
                amount: 0.5,
            }
        );
        // Concrete alpha still folds (existing path).
        assert_eq!(
            parse_value("alpha(#ff0000, 0.5)"),
            StyleValue::Color(Color::rgba(255, 0, 0, 0.5))
        );
    }

    #[test]
    fn oklch_converts_anchors() {
        // Black / white exact.
        assert_eq!(parse_color("oklch(0 0 0)"), Some(Color::rgb(0, 0, 0)));
        assert_eq!(parse_color("oklch(1 0 0)"), Some(Color::rgb(255, 255, 255)));
        assert_eq!(
            parse_color("oklch(100% 0 0)"),
            Some(Color::rgb(255, 255, 255))
        );
        // sRGB red ≈ oklch(0.628 0.258 29.23).
        let red = parse_color("oklch(0.628 0.258 29.23)").expect("red");
        assert!((red.r as i16 - 255).abs() <= 4, "{red:?}");
        assert!(red.g <= 4, "{red:?}");
        assert!(red.b <= 4, "{red:?}");
        // sRGB blue ≈ oklch(0.452 0.313 264.05).
        let blue = parse_color("oklch(0.452 0.313 264.05)").expect("blue");
        assert!(blue.r <= 5, "{blue:?}");
        assert!(blue.g <= 5, "{blue:?}");
        assert!((blue.b as i16 - 255).abs() <= 5, "{blue:?}");
        // deg/turn/alpha forms agree.
        assert_eq!(
            parse_color("oklch(0 0 29.23deg)"),
            parse_color("oklch(0 0 29.23)")
        );
        assert_eq!(
            parse_color("oklch(1 0 0.25turn)"),
            Some(Color::rgb(255, 255, 255))
        );
        assert_eq!(
            parse_color("oklch(0 0 0 / 50%)"),
            Some(Color::rgba(0, 0, 0, 0.5))
        );
        // Malformed never parses (stays keyword downstream).
        assert_eq!(parse_oklch_function("oklch(0 0)"), None);
        assert_eq!(parse_oklch_function("oklch(none 0 0)"), None);
        assert_eq!(parse_oklch_function("oklch(0 0 0 / nope)"), None);
    }

    #[test]
    fn auto_contrast_picks_and_defers() {
        // Concrete folds immediately.
        assert_eq!(
            parse_value("auto-contrast(#ffffff)"),
            StyleValue::Color(Color::rgb(0, 0, 0))
        );
        assert_eq!(
            parse_value("auto-contrast(#000000)"),
            StyleValue::Color(Color::rgb(255, 255, 255))
        );
        // Bright gold gets dark text.
        assert_eq!(
            parse_value("auto-contrast(#ebc878)"),
            StyleValue::Color(Color::rgb(0, 0, 0))
        );
        // Custom pair.
        assert_eq!(
            parse_value("auto-contrast(#ffffff, #123456, #abcdef)"),
            StyleValue::Color(Color::rgb(0x12, 0x34, 0x56))
        );
        // var() stays deferred with explicit defaults filled.
        assert_eq!(
            parse_value("auto-contrast(var(--bg))"),
            StyleValue::AutoContrast {
                base: Box::new(StyleValue::Var("--bg".into())),
                dark: Box::new(StyleValue::Color(Color::rgb(0, 0, 0))),
                light: Box::new(StyleValue::Color(Color::rgb(255, 255, 255))),
            }
        );
        // Too many args never parses.
        assert_eq!(
            parse_auto_contrast_fn("auto-contrast(red, black, white, blue)"),
            None
        );
        assert_eq!(parse_auto_contrast_fn("auto-contrast(nonsense)"), None);
    }

    #[test]
    fn calc_with_var_parses_and_roundtrips() {
        let v = parse_value("calc(var(--pad) + 4px)");
        assert!(matches!(v, StyleValue::Calc(_)), "got {v:?}");
        assert_eq!(crate::display_value(&v), "calc(var(--pad) + 4px)");
        // Fallback survives the roundtrip too.
        let v = parse_value("min(var(--w, 10px), 20px)");
        assert_eq!(crate::display_value(&v), "min(var(--w, 10px), 20px)");
        // Undefined var without fallback cannot evaluate yet.
        let mut v = parse_value("calc(var(--pad) + 4px)");
        assert!(!eval_calc_in_place(&mut v));
        assert!(matches!(v, StyleValue::Calc(_)));
    }

    #[test]
    fn gradient_stop_with_var_stays_deferred() {
        let v = parse_value("linear-gradient(180deg, var(--gold), #501e78)");
        let StyleValue::Gradient(g) = v else {
            panic!("expected gradient");
        };
        assert_eq!(g.stops.len(), 2);
        assert_eq!(g.stops[0].color, StyleValue::Var("--gold".into()));
        assert_eq!(
            g.stops[1].color,
            StyleValue::Color(Color::rgb(0x50, 0x1e, 0x78))
        );
        // Sampling waits for resolve.
        assert_eq!(g.sample(0.0), None);
        assert_eq!(g.fallback_color(), None);
    }

    #[test]
    fn shadow_with_var_structures_but_bare_lengths_do_not() {
        // Gated on var(): var-bearing shadows parse structured…
        let v = parse_value("0 2px 8px var(--shadow)");
        assert_eq!(crate::display_value(&v), "0 2 8 0 var(--shadow)");
        let StyleValue::Shadow(layers) = v else {
            panic!("expected shadow");
        };
        assert_eq!(layers.len(), 1);
        assert_eq!(layers[0].color, StyleValue::Var("--shadow".into()));
        // …while bare multi-length values (margin & co.) keep Keyword.
        assert_eq!(parse_value("0 0"), StyleValue::Keyword("0 0".into()));
        assert_eq!(
            parse_value("10px 20px"),
            StyleValue::Keyword("10px 20px".into())
        );
    }

    #[test]
    fn parses_seconds_and_milliseconds() {
        assert_eq!(parse_value("0.25s"), StyleValue::Time(0.25));
        assert_eq!(parse_value("180ms"), StyleValue::Time(0.18));
        assert_eq!(parse_value("12"), StyleValue::Number(12.0));
    }

    #[test]
    fn parses_css_named_colors_and_hsl() {
        assert_eq!(parse_color("red"), Some(Color::rgb(255, 0, 0)));
        assert_eq!(parse_color("teal"), Some(Color::rgb(0, 128, 128)));
        // game aliases keep their values
        assert_eq!(parse_color("gold"), Some(Color::rgb(235, 200, 120)));
        // hsl primaries
        assert_eq!(
            parse_color("hsl(0, 100%, 50%)"),
            Some(Color::rgb(255, 0, 0))
        );
        assert_eq!(
            parse_color("hsl(120, 100%, 50%)"),
            Some(Color::rgb(0, 255, 0))
        );
        assert_eq!(
            parse_color("hsl(240, 100%, 50%)"),
            Some(Color::rgb(0, 0, 255))
        );
        assert_eq!(parse_color("hsl(0, 0%, 0%)"), Some(Color::rgb(0, 0, 0)));
        assert_eq!(
            parse_color("hsla(0, 100%, 50%, 0.5)"),
            Some(Color::rgba(255, 0, 0, 0.5))
        );
        assert!(parse_color("hsl(0, 50%)").is_none());
        assert!(parse_color("nope").is_none());
        // reachable through declaration values
        assert!(matches!(
            parse_value("hsl(210, 60%, 50%)"),
            StyleValue::Color(_)
        ));
    }

    #[test]
    fn parses_var_fallback_without_changing_plain_var() {
        assert_eq!(parse_value("var(--gold)"), StyleValue::Var("--gold".into()));
        assert_eq!(
            parse_value("var(--gold, var(--accent, #ffffff))"),
            StyleValue::VarFallback {
                name: "--gold".into(),
                fallback: Box::new(StyleValue::VarFallback {
                    name: "--accent".into(),
                    fallback: Box::new(StyleValue::Color(Color::rgb(255, 255, 255))),
                }),
            }
        );
    }
}
