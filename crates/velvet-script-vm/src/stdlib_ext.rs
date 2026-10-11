//! Conversion, string, map and JSON natives added after the original stdlib.
//!
//! All functions here are pure: they only read their arguments. Natives that
//! call back into script code (`call`, `list_map`, ...) live in the VM because
//! they need to execute bytecode.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::rc::Rc;

use velvet_script_bytecode::NativeId;

use crate::stdlib::NativeOutput;
use crate::value::Value;

/// Maximum nesting accepted by `json_decode` and `json_encode`.
const JSON_MAX_DEPTH: usize = 128;

fn out(value: Value) -> NativeOutput {
    NativeOutput {
        value,
        printed: None,
    }
}

fn text(s: String) -> Value {
    Value::String(Rc::from(s))
}

fn as_str<'a>(v: &'a Value, name: &str) -> Result<&'a str, String> {
    v.as_str()
        .ok_or_else(|| format!("{name} expects a string, got `{}`", v.type_name()))
}

fn as_int(v: &Value, name: &str) -> Result<i64, String> {
    match v {
        Value::Int(i) => Ok(*i),
        _ => Err(format!("{name} expects an int, got `{}`", v.type_name())),
    }
}

/// Converts a value to an integer, or explains why it cannot.
pub(crate) fn to_int(v: &Value) -> Result<i64, String> {
    match v {
        Value::Int(i) => Ok(*i),
        Value::Float(f) => {
            if !f.is_finite() {
                return Err(format!("int() cannot convert non-finite float {f}"));
            }
            let t = f.trunc();
            if t < i64::MIN as f64 || t > i64::MAX as f64 {
                return Err(format!("int() float {f} is out of range"));
            }
            Ok(t as i64)
        }
        Value::Bool(b) => Ok(i64::from(*b)),
        Value::String(s) => {
            let t = s.trim();
            if let Ok(i) = t.parse::<i64>() {
                return Ok(i);
            }
            match t.parse::<f64>() {
                Ok(f) if f.is_finite() && t.contains('.') => to_int(&Value::Float(f)),
                _ => Err(format!("int() cannot convert \"{t}\" to an integer")),
            }
        }
        other => Err(format!("int() cannot convert `{}`", other.type_name())),
    }
}

/// Converts a value to a float, or explains why it cannot.
pub(crate) fn to_float(v: &Value) -> Result<f64, String> {
    match v {
        Value::Int(i) => Ok(*i as f64),
        Value::Float(f) => Ok(*f),
        Value::Bool(b) => Ok(if *b { 1.0 } else { 0.0 }),
        Value::String(s) => {
            let t = s.trim();
            match t.parse::<f64>() {
                Ok(f) if f.is_finite() => Ok(f),
                _ => Err(format!("float() cannot convert \"{t}\" to a number")),
            }
        }
        other => Err(format!("float() cannot convert `{}`", other.type_name())),
    }
}

fn pad(
    textv: &Value,
    width: &Value,
    fill: Option<&Value>,
    left: bool,
    name: &str,
) -> Result<Value, String> {
    let s = as_str(textv, name)?;
    let width = as_int(width, name)?.clamp(0, 1_000_000) as usize;
    let fill = match fill {
        Some(f) => as_str(f, name)?.to_string(),
        None => " ".to_string(),
    };
    if fill.is_empty() {
        return Err(format!("{name} fill must not be empty"));
    }
    let len = s.chars().count();
    if len >= width {
        return Ok(text(s.to_string()));
    }
    let padding: String = fill.chars().cycle().take(width - len).collect();
    Ok(text(if left {
        format!("{padding}{s}")
    } else {
        format!("{s}{padding}")
    }))
}

/// Run one of the pure natives defined in this module; `None` when the id
/// belongs elsewhere.
pub(crate) fn call_ext(native: NativeId, args: &[Value]) -> Option<Result<NativeOutput, String>> {
    use NativeId as N;
    let spec = native.spec();
    if !matches!(
        native,
        N::Int
            | N::Float
            | N::ParseInt
            | N::ParseFloat
            | N::MapGet
            | N::StrIndexOf
            | N::StrRepeat
            | N::StrPadLeft
            | N::StrPadRight
            | N::StrChars
            | N::StrSub
            | N::StrMeasure
            | N::StrWrap
            | N::JsonEncode
            | N::JsonDecode
    ) {
        return None;
    }
    if args.len() < spec.min_args as usize || args.len() > spec.max_args as usize {
        return Some(Err(format!(
            "{} expects {}..={} arguments, got {}",
            spec.name,
            spec.min_args,
            spec.max_args,
            args.len()
        )));
    }
    let r = match native {
        N::Int => to_int(&args[0]).map(|i| out(Value::Int(i))),
        N::Float => to_float(&args[0]).map(|f| out(Value::Float(f))),
        N::ParseInt => Ok(out(match &args[0] {
            Value::String(_) | Value::Int(_) | Value::Float(_) => match to_int(&args[0]) {
                Ok(i) => Value::Int(i),
                Err(_) => args[1].clone(),
            },
            _ => args[1].clone(),
        })),
        N::ParseFloat => Ok(out(match &args[0] {
            Value::String(_) | Value::Int(_) | Value::Float(_) => match to_float(&args[0]) {
                Ok(f) => Value::Float(f),
                Err(_) => args[1].clone(),
            },
            _ => args[1].clone(),
        })),
        N::MapGet => (|| {
            let key = as_str(&args[1], "map_get")?;
            let found = args[0]
                .map_get(key)
                .map_err(|_| format!("map_get expects a map, got `{}`", args[0].type_name()))?;
            Ok(out(found.unwrap_or_else(|| {
                args.get(2).cloned().unwrap_or(Value::Null)
            })))
        })(),
        N::StrIndexOf => (|| {
            let hay = as_str(&args[0], "str_index_of")?;
            let needle = as_str(&args[1], "str_index_of")?;
            Ok(out(Value::Int(match hay.find(needle) {
                Some(byte) => hay[..byte].chars().count() as i64,
                None => -1,
            })))
        })(),
        N::StrRepeat => (|| {
            let s = as_str(&args[0], "str_repeat")?;
            let n = as_int(&args[1], "str_repeat")?;
            if n < 0 {
                return Err("str_repeat count must not be negative".to_string());
            }
            let total = s.len().saturating_mul(n as usize);
            if total > 16 * 1024 * 1024 {
                return Err("str_repeat result is too large".to_string());
            }
            Ok(out(text(s.repeat(n as usize))))
        })(),
        N::StrPadLeft => pad(&args[0], &args[1], args.get(2), true, "str_pad_left").map(out),
        N::StrPadRight => pad(&args[0], &args[1], args.get(2), false, "str_pad_right").map(out),
        N::StrChars => as_str(&args[0], "str_chars").map(|s| {
            out(Value::list(
                s.chars().map(|c| text(c.to_string())).collect(),
            ))
        }),
        N::StrSub => (|| {
            let s = as_str(&args[0], "str_sub")?;
            let chars: Vec<char> = s.chars().collect();
            let len = chars.len() as i64;
            let norm = |i: i64| (if i < 0 { len + i } else { i }).clamp(0, len) as usize;
            let start = norm(as_int(&args[1], "str_sub")?);
            let end = match args.get(2) {
                Some(v) => norm(as_int(v, "str_sub")?),
                None => len as usize,
            };
            let sub: String = if start < end {
                chars[start..end].iter().collect()
            } else {
                String::new()
            };
            Ok(out(text(sub)))
        })(),
        N::StrMeasure => (|| {
            let s = as_str(&args[0], "str_measure")?;
            let metrics = Advances::new(&args[1], &args[2], "str_measure")?;
            Ok(out(Value::Float(metrics.width(s))))
        })(),
        N::StrWrap => (|| {
            let s = as_str(&args[0], "str_wrap")?;
            let metrics = Advances::new(&args[1], &args[2], "str_wrap")?;
            let max_width = args[3]
                .as_f64()
                .ok_or_else(|| "str_wrap expects a numeric max_width".to_string())?;
            let lines = wrap_lines(s, max_width, |piece| metrics.width(piece));
            Ok(out(Value::list(lines.into_iter().map(text).collect())))
        })(),
        N::JsonEncode => {
            let pretty = matches!(args.get(1), Some(Value::Bool(true)));
            json_encode(&args[0], pretty).map(|s| out(text(s)))
        }
        N::JsonDecode => as_str(&args[0], "json_decode")
            .and_then(json_decode)
            .map(out),
        _ => return None,
    };
    Some(r)
}

/// Extra instruction cost for size-dependent natives in this module.
pub(crate) fn ext_cost(native: NativeId, args: &[Value]) -> u64 {
    use NativeId as N;
    let slen = |i: usize| args.get(i).and_then(Value::as_str).map_or(0, str::len) as u64;
    match native {
        N::StrIndexOf | N::StrChars | N::StrSub | N::StrMeasure | N::JsonDecode => slen(0) / 4,
        N::StrWrap => slen(0),
        N::StrRepeat => {
            slen(0).saturating_mul(args.get(1).and_then(Value::as_i64).unwrap_or(0).max(0) as u64)
                / 8
        }
        N::JsonEncode => args.first().map_or(0, |v| v.memory_units() as u64),
        _ => 0,
    }
}

// ---------------------------------------------------------------- JSON

fn json_escape(s: &str, outs: &mut String) {
    outs.push('"');
    for c in s.chars() {
        match c {
            '"' => outs.push_str("\\\""),
            '\\' => outs.push_str("\\\\"),
            '\n' => outs.push_str("\\n"),
            '\r' => outs.push_str("\\r"),
            '\t' => outs.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let _ = write!(outs, "\\u{:04x}", c as u32);
            }
            c => outs.push(c),
        }
    }
    outs.push('"');
}

fn json_indent(outs: &mut String, pretty: bool, depth: usize) {
    if pretty {
        outs.push('\n');
        for _ in 0..depth {
            outs.push_str("  ");
        }
    }
}

fn json_floats(outs: &mut String, items: &[f64]) -> Result<(), String> {
    outs.push('[');
    for (i, f) in items.iter().enumerate() {
        if i > 0 {
            outs.push(',');
        }
        json_float(*f, outs)?;
    }
    outs.push(']');
    Ok(())
}

fn json_float(f: f64, outs: &mut String) -> Result<(), String> {
    if !f.is_finite() {
        return Err(format!("json_encode cannot encode non-finite number {f}"));
    }
    if f.fract() == 0.0 && f.abs() < 1e15 {
        let _ = write!(outs, "{f:.1}");
    } else {
        let _ = write!(outs, "{f}");
    }
    Ok(())
}

fn json_write(v: &Value, outs: &mut String, pretty: bool, depth: usize) -> Result<(), String> {
    if depth > JSON_MAX_DEPTH {
        return Err("json_encode: value is nested too deeply (or cyclic)".to_string());
    }
    match v {
        Value::Null => outs.push_str("null"),
        Value::Bool(b) => outs.push_str(if *b { "true" } else { "false" }),
        Value::Int(i) => {
            let _ = write!(outs, "{i}");
        }
        Value::Float(f) => json_float(*f, outs)?,
        Value::String(s) => json_escape(s, outs),
        Value::List(items) => {
            let items = items.borrow();
            outs.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    outs.push(',');
                }
                json_indent(outs, pretty, depth + 1);
                json_write(item, outs, pretty, depth + 1)?;
            }
            if !items.is_empty() {
                json_indent(outs, pretty, depth);
            }
            outs.push(']');
        }
        Value::Map(entries) => {
            let entries = entries.borrow();
            outs.push('{');
            for (i, (k, item)) in entries.iter().enumerate() {
                if i > 0 {
                    outs.push(',');
                }
                json_indent(outs, pretty, depth + 1);
                json_escape(k, outs);
                outs.push(':');
                if pretty {
                    outs.push(' ');
                }
                json_write(item, outs, pretty, depth + 1)?;
            }
            if !entries.is_empty() {
                json_indent(outs, pretty, depth);
            }
            outs.push('}');
        }
        Value::Vec2(c) => json_floats(outs, c)?,
        Value::Vec3(c) => json_floats(outs, c)?,
        Value::Vec4(c) | Value::Quat(c) => json_floats(outs, c)?,
        Value::Mat3(c) => json_floats(outs, c)?,
        Value::Mat4(c) => json_floats(outs, c)?,
        other => return Err(format!("json_encode cannot encode `{}`", other.type_name())),
    }
    Ok(())
}

/// Per-character advance table (`{"a": 55.2, ..., "default": 55}`, widths
/// at size 100) scaled to a font size.
struct Advances {
    table: BTreeMap<String, Value>,
    default: f64,
    scale: f64,
}

impl Advances {
    fn new(table: &Value, size: &Value, name: &str) -> Result<Self, String> {
        let scale = size
            .as_f64()
            .ok_or_else(|| format!("{name} expects a numeric size"))?
            / 100.0;
        let table = table
            .map_entries()
            .map_err(|_| format!("{name} expects an advance map"))?;
        let default = table.get("default").and_then(Value::as_f64).unwrap_or(55.0);
        Ok(Self {
            table,
            default,
            scale,
        })
    }

    fn width(&self, s: &str) -> f64 {
        let mut key = String::new();
        let mut width = 0.0;
        for c in s.chars() {
            key.clear();
            key.push(c);
            width += self
                .table
                .get(&key)
                .and_then(Value::as_f64)
                .unwrap_or(self.default);
        }
        width * self.scale
    }
}

/// Greedy word wrap shared by `str_wrap` and hosts: paragraphs split on
/// `\n`, words that alone exceed `max_width` are broken by character.
pub fn wrap_lines(text: &str, max_width: f64, measure: impl Fn(&str) -> f64) -> Vec<String> {
    let mut lines = Vec::new();
    for paragraph in text.split('\n') {
        let mut line = String::new();
        for word in paragraph.split_whitespace() {
            let candidate = if line.is_empty() {
                word.to_owned()
            } else {
                format!("{line} {word}")
            };
            if measure(&candidate) <= max_width || (line.is_empty() && max_width <= 0.0) {
                line = candidate;
                continue;
            }
            if !line.is_empty() {
                lines.push(std::mem::take(&mut line));
            }
            if measure(word) <= max_width {
                line = word.to_owned();
                continue;
            }
            for c in word.chars() {
                let mut next = line.clone();
                next.push(c);
                if !line.is_empty() && measure(&next) > max_width {
                    lines.push(std::mem::take(&mut line));
                    line.push(c);
                } else {
                    line = next;
                }
            }
        }
        lines.push(line);
    }
    lines
}

/// Encode a value as JSON (vectors, matrices and quaternions become float
/// arrays; functions and RNG streams are rejected).
pub fn json_encode(value: &Value, pretty: bool) -> Result<String, String> {
    let mut s = String::new();
    json_write(value, &mut s, pretty, 0)?;
    Ok(s)
}

/// Decode JSON text into VS3 values (`null`, bool, int, float, string, list,
/// map).
pub fn json_decode(text: &str) -> Result<Value, String> {
    let mut parser = JsonParser {
        text,
        src: text.as_bytes(),
        pos: 0,
    };
    let value = parser.value(0)?;
    parser.skip_ws();
    if parser.pos != parser.src.len() {
        return Err(format!(
            "json_decode: unexpected data at byte {}",
            parser.pos
        ));
    }
    Ok(value)
}

struct JsonParser<'a> {
    text: &'a str,
    src: &'a [u8],
    pos: usize,
}

impl JsonParser<'_> {
    fn err(&self, what: &str) -> String {
        format!("json_decode: {what} at byte {}", self.pos)
    }

    fn skip_ws(&mut self) {
        while self.pos < self.src.len()
            && matches!(self.src[self.pos], b' ' | b'\n' | b'\r' | b'\t')
        {
            self.pos += 1;
        }
    }

    fn eat(&mut self, lit: &str) -> bool {
        if self.src[self.pos..].starts_with(lit.as_bytes()) {
            self.pos += lit.len();
            true
        } else {
            false
        }
    }

    fn value(&mut self, depth: usize) -> Result<Value, String> {
        if depth > JSON_MAX_DEPTH {
            return Err(self.err("nested too deeply"));
        }
        self.skip_ws();
        let Some(&c) = self.src.get(self.pos) else {
            return Err(self.err("unexpected end"));
        };
        match c {
            b'n' if self.eat("null") => Ok(Value::Null),
            b't' if self.eat("true") => Ok(Value::Bool(true)),
            b'f' if self.eat("false") => Ok(Value::Bool(false)),
            b'"' => self.string().map(text),
            b'[' => {
                self.pos += 1;
                let mut items = Vec::new();
                self.skip_ws();
                if self.eat("]") {
                    return Ok(Value::list(items));
                }
                loop {
                    items.push(self.value(depth + 1)?);
                    self.skip_ws();
                    if self.eat(",") {
                        continue;
                    }
                    if self.eat("]") {
                        return Ok(Value::list(items));
                    }
                    return Err(self.err("expected ',' or ']'"));
                }
            }
            b'{' => {
                self.pos += 1;
                let mut map = BTreeMap::new();
                self.skip_ws();
                if self.eat("}") {
                    return Ok(Value::map(map));
                }
                loop {
                    self.skip_ws();
                    if self.src.get(self.pos) != Some(&b'"') {
                        return Err(self.err("expected a string key"));
                    }
                    let k = self.string()?;
                    self.skip_ws();
                    if !self.eat(":") {
                        return Err(self.err("expected ':'"));
                    }
                    let v = self.value(depth + 1)?;
                    map.insert(k, v);
                    self.skip_ws();
                    if self.eat(",") {
                        continue;
                    }
                    if self.eat("}") {
                        return Ok(Value::map(map));
                    }
                    return Err(self.err("expected ',' or '}'"));
                }
            }
            b'-' | b'0'..=b'9' => self.number(),
            _ => Err(self.err("unexpected character")),
        }
    }

    fn number(&mut self) -> Result<Value, String> {
        let start = self.pos;
        while self.pos < self.src.len()
            && matches!(
                self.src[self.pos],
                b'-' | b'+' | b'.' | b'e' | b'E' | b'0'..=b'9'
            )
        {
            self.pos += 1;
        }
        let t =
            std::str::from_utf8(&self.src[start..self.pos]).map_err(|_| self.err("bad number"))?;
        if !t.contains(['.', 'e', 'E']) {
            if let Ok(i) = t.parse::<i64>() {
                return Ok(Value::Int(i));
            }
        }
        t.parse::<f64>()
            .ok()
            .filter(|f| f.is_finite())
            .map(Value::Float)
            .ok_or_else(|| self.err("bad number"))
    }

    fn string(&mut self) -> Result<String, String> {
        self.pos += 1;
        let mut s = String::new();
        loop {
            let Some(&c) = self.src.get(self.pos) else {
                return Err(self.err("unterminated string"));
            };
            match c {
                b'"' => {
                    self.pos += 1;
                    return Ok(s);
                }
                b'\\' => {
                    let Some(&e) = self.src.get(self.pos + 1) else {
                        return Err(self.err("bad escape"));
                    };
                    self.pos += 2;
                    match e {
                        b'"' => s.push('"'),
                        b'\\' => s.push('\\'),
                        b'/' => s.push('/'),
                        b'n' => s.push('\n'),
                        b'r' => s.push('\r'),
                        b't' => s.push('\t'),
                        b'b' => s.push('\u{8}'),
                        b'f' => s.push('\u{c}'),
                        b'u' => {
                            let hex = self
                                .src
                                .get(self.pos..self.pos + 4)
                                .and_then(|h| std::str::from_utf8(h).ok())
                                .and_then(|h| u32::from_str_radix(h, 16).ok())
                                .ok_or_else(|| self.err("bad unicode escape"))?;
                            self.pos += 4;
                            s.push(char::from_u32(hex).unwrap_or('\u{fffd}'));
                        }
                        _ => return Err(self.err("bad escape")),
                    }
                }
                _ => {
                    // Copy one UTF-8 sequence (positions always sit on char boundaries).
                    let ch = self.text[self.pos..].chars().next().unwrap_or('\u{fffd}');
                    s.push(ch);
                    self.pos += ch.len_utf8();
                }
            }
        }
    }
}
