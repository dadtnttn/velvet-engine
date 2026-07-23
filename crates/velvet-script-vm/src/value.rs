//! Runtime values.

use std::cell::RefCell;
use std::collections::{BTreeMap, HashSet};
use std::fmt;
use std::rc::Rc;

use velvet_script_bytecode::Constant;

/// Runtime value.
#[derive(Clone)]
pub enum Value {
    /// Null.
    Null,
    /// Bool.
    Bool(bool),
    /// Integer.
    Int(i64),
    /// Float.
    Float(f64),
    /// String.
    String(Rc<str>),
    /// List (shared, interior-mutable).
    List(Rc<RefCell<Vec<Value>>>),
    /// String-keyed map (shared, interior-mutable).
    Map(Rc<RefCell<BTreeMap<String, Value>>>),
    /// Immutable two-component `f64` vector.
    Vec2([f64; 2]),
    /// Immutable three-component `f64` vector.
    Vec3([f64; 3]),
    /// Immutable four-component `f64` vector.
    Vec4([f64; 4]),
    /// Immutable column-major 3x3 `f64` matrix.
    Mat3([f64; 9]),
    /// Immutable column-major 4x4 `f64` matrix.
    Mat4([f64; 16]),
    /// Immutable quaternion `(x, y, z, w)`.
    Quat([f64; 4]),
    /// Shared deterministic PCG random stream `[state, increment]`.
    Rng(Rc<RefCell<[u64; 2]>>),
    /// Function (module function index).
    Function(u16),
    /// Native host function id.
    Native(u16),
}

impl Value {
    /// From constant.
    pub fn from_constant(c: &Constant) -> Self {
        match c {
            Constant::Null => Self::Null,
            Constant::Bool(b) => Self::Bool(*b),
            Constant::Int(i) => Self::Int(*i),
            Constant::Float(f) => Self::Float(*f),
            Constant::String(s) => Self::String(Rc::from(s.as_str())),
            Constant::Function(i) => Self::Function(*i),
            Constant::Native(i) => Self::Native(*i),
        }
    }

    /// Truthiness.
    pub fn is_truthy(&self) -> bool {
        match self {
            Self::Null => false,
            Self::Bool(b) => *b,
            Self::Int(i) => *i != 0,
            Self::Float(f) => *f != 0.0,
            Self::String(s) => !s.is_empty(),
            Self::List(l) => !l.borrow().is_empty(),
            Self::Map(m) => !m.borrow().is_empty(),
            Self::Vec2(_)
            | Self::Vec3(_)
            | Self::Vec4(_)
            | Self::Mat3(_)
            | Self::Mat4(_)
            | Self::Quat(_)
            | Self::Rng(_)
            | Self::Function(_)
            | Self::Native(_) => true,
        }
    }

    /// Approximate heap units for memory limit (rough).
    pub fn memory_units(&self) -> usize {
        self.memory_units_inner(&mut HashSet::new())
    }

    fn memory_units_inner(&self, seen: &mut HashSet<(u8, usize)>) -> usize {
        match self {
            Self::Null
            | Self::Bool(_)
            | Self::Int(_)
            | Self::Float(_)
            | Self::Function(_)
            | Self::Native(_) => 1,
            Self::Vec2(_) => 2,
            Self::Vec3(_) => 3,
            Self::Vec4(_) | Self::Quat(_) => 4,
            Self::Mat3(_) => 9,
            Self::Mat4(_) => 16,
            Self::String(s) => 1 + s.len() / 8,
            Self::List(l) => {
                let key = (0, Rc::as_ptr(l) as usize);
                if !seen.insert(key) {
                    return 0;
                }
                1 + l
                    .borrow()
                    .iter()
                    .map(|value| value.memory_units_inner(seen))
                    .sum::<usize>()
            }
            Self::Map(m) => {
                let key = (1, Rc::as_ptr(m) as usize);
                if !seen.insert(key) {
                    return 0;
                }
                1 + m
                    .borrow()
                    .iter()
                    .map(|(k, v)| 1 + k.len() / 8 + v.memory_units_inner(seen))
                    .sum::<usize>()
            }
            Self::Rng(_) => 2,
        }
    }

    /// As i64 if numeric.
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Self::Int(i) => Some(*i),
            Self::Float(f) => Some(*f as i64),
            Self::Bool(b) => Some(if *b { 1 } else { 0 }),
            _ => None,
        }
    }

    /// As f64 if numeric.
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Float(f) => Some(*f),
            Self::Int(i) => Some(*i as f64),
            Self::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
            _ => None,
        }
    }

    /// As string slice if string.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(s) => Some(s.as_ref()),
            _ => None,
        }
    }

    /// Length for strings, lists, and maps.
    pub fn len(&self) -> Option<usize> {
        match self {
            Self::String(s) => Some(s.chars().count()),
            Self::List(l) => Some(l.borrow().len()),
            Self::Map(m) => Some(m.borrow().len()),
            Self::Vec2(_) => Some(2),
            Self::Vec3(_) => Some(3),
            Self::Vec4(_) | Self::Quat(_) => Some(4),
            Self::Mat3(_) => Some(9),
            Self::Mat4(_) => Some(16),
            _ => None,
        }
    }

    /// Whether list/string/map is empty (or null).
    pub fn is_empty(&self) -> bool {
        match self {
            Self::Null => true,
            Self::String(s) => s.is_empty(),
            Self::List(l) => l.borrow().is_empty(),
            Self::Map(m) => m.borrow().is_empty(),
            _ => false,
        }
    }

    /// Index into list (integer), map (string key), or string (char).
    pub fn get_index(&self, index: &Value) -> Result<Value, String> {
        match self {
            Self::List(items) => {
                let i = index
                    .as_i64()
                    .ok_or_else(|| "list index must be a number".to_string())?;
                let items = items.borrow();
                if i < 0 || i as usize >= items.len() {
                    return Err(format!(
                        "list index {i} out of bounds (len {})",
                        items.len()
                    ));
                }
                Ok(items[i as usize].clone())
            }
            Self::Map(map) => {
                let key = match index {
                    Self::String(s) => s.as_ref().to_string(),
                    other => other.to_string(),
                };
                Ok(map.borrow().get(&key).cloned().unwrap_or(Self::Null))
            }
            Self::String(s) => {
                let i = index
                    .as_i64()
                    .ok_or_else(|| "string index must be a number".to_string())?;
                let chars: Vec<char> = s.chars().collect();
                if i < 0 || i as usize >= chars.len() {
                    return Err(format!(
                        "string index {i} out of bounds (len {})",
                        chars.len()
                    ));
                }
                Ok(Self::String(Rc::from(chars[i as usize].to_string())))
            }
            Self::Vec2(values) => vector_index(values, index),
            Self::Vec3(values) => vector_index(values, index),
            Self::Vec4(values) | Self::Quat(values) => vector_index(values, index),
            Self::Mat3(values) => numeric_index(values, index),
            Self::Mat4(values) => numeric_index(values, index),
            _ => Err("value is not indexable".into()),
        }
    }

    /// Set index on list or map in place; returns the stored value.
    pub fn set_index(&self, index: &Value, value: Value) -> Result<Value, String> {
        match self {
            Self::List(items) => {
                let i = index
                    .as_i64()
                    .ok_or_else(|| "list index must be a number".to_string())?;
                let mut items = items.borrow_mut();
                if i < 0 || i as usize >= items.len() {
                    return Err(format!(
                        "list index {i} out of bounds (len {})",
                        items.len()
                    ));
                }
                items[i as usize] = value.clone();
                Ok(value)
            }
            Self::Map(map) => {
                let key = match index {
                    Self::String(s) => s.as_ref().to_string(),
                    other => other.to_string(),
                };
                map.borrow_mut().insert(key, value.clone());
                Ok(value)
            }
            Self::Vec2(_)
            | Self::Vec3(_)
            | Self::Vec4(_)
            | Self::Mat3(_)
            | Self::Mat4(_)
            | Self::Quat(_) => Err("mathematical values are immutable".into()),
            _ => Err("value is not a mutable index target".into()),
        }
    }

    /// Build a list value.
    pub fn list(items: Vec<Value>) -> Self {
        Self::List(Rc::new(RefCell::new(items)))
    }

    /// Build a map value.
    pub fn map(entries: impl IntoIterator<Item = (String, Value)>) -> Self {
        Self::Map(Rc::new(RefCell::new(entries.into_iter().collect())))
    }

    /// Push onto a list (clone-friendly helper).
    pub fn list_push(&self, value: Value) -> Result<(), String> {
        match self {
            Self::List(items) => {
                items.borrow_mut().push(value);
                Ok(())
            }
            _ => Err("list_push on non-list".into()),
        }
    }

    /// Pop the last list item, returning null when the list is empty.
    pub fn list_pop(&self) -> Result<Value, String> {
        match self {
            Self::List(items) => Ok(items.borrow_mut().pop().unwrap_or(Self::Null)),
            _ => Err("list_pop on non-list".into()),
        }
    }

    /// Whether a map contains a string key.
    pub fn map_has(&self, key: &str) -> Result<bool, String> {
        match self {
            Self::Map(entries) => Ok(entries.borrow().contains_key(key)),
            _ => Err("map_has on non-map".into()),
        }
    }

    /// Map keys in deterministic sorted order.
    pub fn map_keys(&self) -> Result<Vec<String>, String> {
        match self {
            Self::Map(entries) => Ok(entries.borrow().keys().cloned().collect()),
            _ => Err("map_keys on non-map".into()),
        }
    }

    /// Clone the items of a list for a host boundary.
    ///
    /// Returning owned values prevents a host from holding a `RefCell` borrow
    /// while it invokes more script code.
    pub fn list_items(&self) -> Result<Vec<Value>, String> {
        match self {
            Self::List(items) => Ok(items.borrow().clone()),
            _ => Err("list_items on non-list".into()),
        }
    }

    /// Clone all entries of a map for a host boundary.
    pub fn map_entries(&self) -> Result<BTreeMap<String, Value>, String> {
        match self {
            Self::Map(entries) => Ok(entries.borrow().clone()),
            _ => Err("map_entries on non-map".into()),
        }
    }

    /// Clone one named map field for a host boundary.
    pub fn map_get(&self, key: &str) -> Result<Option<Value>, String> {
        match self {
            Self::Map(entries) => Ok(entries.borrow().get(key).cloned()),
            _ => Err("map_get on non-map".into()),
        }
    }

    /// Concatenate two values as strings.
    pub fn concat_str(a: &Value, b: &Value) -> Value {
        Value::String(Rc::from(format!("{a}{b}")))
    }

    /// Type name for diagnostics / hover.
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::Null => "null",
            Self::Bool(_) => "bool",
            Self::Int(_) => "int",
            Self::Float(_) => "float",
            Self::String(_) => "string",
            Self::List(_) => "list",
            Self::Map(_) => "map",
            Self::Vec2(_) => "vec2",
            Self::Vec3(_) => "vec3",
            Self::Vec4(_) => "vec4",
            Self::Mat3(_) => "mat3",
            Self::Mat4(_) => "mat4",
            Self::Quat(_) => "quat",
            Self::Rng(_) => "rng",
            Self::Function(_) => "function",
            Self::Native(_) => "native",
        }
    }
}

fn numeric_index<const N: usize>(values: &[f64; N], index: &Value) -> Result<Value, String> {
    let index = index
        .as_i64()
        .ok_or_else(|| "numeric value index must be an integer".to_string())?;
    if index < 0 || index as usize >= N {
        return Err(format!("index {index} out of bounds (len {N})"));
    }
    Ok(Value::Float(values[index as usize]))
}

fn vector_index<const N: usize>(values: &[f64; N], index: &Value) -> Result<Value, String> {
    let component = match index.as_str() {
        Some("x") => Some(0),
        Some("y") => Some(1),
        Some("z") => Some(2),
        Some("w") => Some(3),
        Some(name) => return Err(format!("unknown vector component `{name}`")),
        None => None,
    };
    if let Some(component) = component {
        return values
            .get(component)
            .copied()
            .map(Value::Float)
            .ok_or_else(|| format!("component is not available on vec{N}"));
    }
    numeric_index(values, index)
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        value_eq(self, other, &mut HashSet::new())
    }
}

fn value_eq(a: &Value, b: &Value, seen: &mut HashSet<(u8, usize, usize)>) -> bool {
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(a), Value::Bool(b)) => a == b,
        (Value::Int(a), Value::Int(b)) => a == b,
        (Value::Float(a), Value::Float(b)) => a == b,
        (Value::String(a), Value::String(b)) => a == b,
        (Value::Vec2(a), Value::Vec2(b)) => a == b,
        (Value::Vec3(a), Value::Vec3(b)) => a == b,
        (Value::Vec4(a), Value::Vec4(b)) => a == b,
        (Value::Mat3(a), Value::Mat3(b)) => a == b,
        (Value::Mat4(a), Value::Mat4(b)) => a == b,
        (Value::Quat(a), Value::Quat(b)) => a == b,
        (Value::Rng(a), Value::Rng(b)) => Rc::ptr_eq(a, b) || *a.borrow() == *b.borrow(),
        (Value::Function(a), Value::Function(b)) | (Value::Native(a), Value::Native(b)) => a == b,
        (Value::List(a), Value::List(b)) => {
            if Rc::ptr_eq(a, b) {
                return true;
            }
            let key = (0, Rc::as_ptr(a) as usize, Rc::as_ptr(b) as usize);
            if !seen.insert(key) {
                return true;
            }
            let a = a.borrow();
            let b = b.borrow();
            a.len() == b.len() && a.iter().zip(b.iter()).all(|(a, b)| value_eq(a, b, seen))
        }
        (Value::Map(a), Value::Map(b)) => {
            if Rc::ptr_eq(a, b) {
                return true;
            }
            let key = (1, Rc::as_ptr(a) as usize, Rc::as_ptr(b) as usize);
            if !seen.insert(key) {
                return true;
            }
            let a = a.borrow();
            let b = b.borrow();
            a.len() == b.len()
                && a.iter().all(|(key, value)| {
                    b.get(key).is_some_and(|other| value_eq(value, other, seen))
                })
        }
        _ => false,
    }
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_value(self, f, &mut HashSet::new())
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_value(self, f, &mut HashSet::new())
    }
}

fn fmt_value(
    value: &Value,
    f: &mut fmt::Formatter<'_>,
    active: &mut HashSet<(u8, usize)>,
) -> fmt::Result {
    match value {
        Value::Null => write!(f, "null"),
        Value::Bool(value) => write!(f, "{value}"),
        Value::Int(value) => write!(f, "{value}"),
        Value::Float(value) => write!(f, "{value}"),
        Value::String(value) => write!(f, "{value}"),
        Value::Vec2(value) => write!(f, "vec2({}, {})", value[0], value[1]),
        Value::Vec3(value) => write!(f, "vec3({}, {}, {})", value[0], value[1], value[2]),
        Value::Vec4(value) => write!(
            f,
            "vec4({}, {}, {}, {})",
            value[0], value[1], value[2], value[3]
        ),
        Value::Mat3(value) => fmt_numeric_value("mat3", value, f),
        Value::Mat4(value) => fmt_numeric_value("mat4", value, f),
        Value::Quat(value) => write!(
            f,
            "quat({}, {}, {}, {})",
            value[0], value[1], value[2], value[3]
        ),
        Value::Rng(_) => write!(f, "<rng>"),
        Value::List(values) => {
            let key = (0, Rc::as_ptr(values) as usize);
            if !active.insert(key) {
                return write!(f, "<cycle>");
            }
            write!(f, "[")?;
            for (index, value) in values.borrow().iter().enumerate() {
                if index > 0 {
                    write!(f, ", ")?;
                }
                fmt_value(value, f, active)?;
            }
            active.remove(&key);
            write!(f, "]")
        }
        Value::Map(values) => {
            let key = (1, Rc::as_ptr(values) as usize);
            if !active.insert(key) {
                return write!(f, "<cycle>");
            }
            write!(f, "{{")?;
            for (index, (key, value)) in values.borrow().iter().enumerate() {
                if index > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{key}: ")?;
                fmt_value(value, f, active)?;
            }
            active.remove(&key);
            write!(f, "}}")
        }
        Value::Function(index) => write!(f, "<fn #{index}>"),
        Value::Native(index) => write!(f, "<native #{index}>"),
    }
}

fn fmt_numeric_value(name: &str, values: &[f64], f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{name}(")?;
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            write!(f, ", ")?;
        }
        write!(f, "{value}")?;
    }
    write!(f, ")")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_index_roundtrip() {
        let list = Value::list(vec![Value::Int(1), Value::Int(2), Value::Int(3)]);
        assert_eq!(list.get_index(&Value::Int(1)).unwrap(), Value::Int(2));
        let stored = list.set_index(&Value::Int(1), Value::Int(99)).unwrap();
        assert_eq!(stored, Value::Int(99));
        assert_eq!(list.get_index(&Value::Int(1)).unwrap(), Value::Int(99));
    }

    #[test]
    fn map_get_set() {
        let mut m = BTreeMap::new();
        m.insert("a".into(), Value::Int(1));
        let map = Value::map(m);
        assert_eq!(
            map.get_index(&Value::String(Rc::from("a"))).unwrap(),
            Value::Int(1)
        );
        map.set_index(&Value::String(Rc::from("b")), Value::Int(2))
            .unwrap();
        assert_eq!(map.len(), Some(2));
    }

    #[test]
    fn string_len_and_index() {
        let s = Value::String(Rc::from("hi"));
        assert_eq!(s.len(), Some(2));
        assert_eq!(
            s.get_index(&Value::Int(1)).unwrap(),
            Value::String(Rc::from("i"))
        );
    }

    #[test]
    fn concat_str() {
        let a = Value::Int(1);
        let b = Value::String(Rc::from("x"));
        assert_eq!(Value::concat_str(&a, &b).as_str(), Some("1x"));
    }

    #[test]
    fn cyclic_values_are_safe_to_measure_format_and_compare() {
        let list = Value::list(vec![]);
        list.list_push(list.clone()).unwrap();

        assert_eq!(list.memory_units(), 1);
        assert_eq!(list.to_string(), "[<cycle>]");
        assert_eq!(format!("{list:?}"), "[<cycle>]");
        assert_eq!(list, list.clone());
    }

    #[test]
    fn maps_have_deterministic_key_order() {
        let map = Value::map([("z".into(), Value::Int(1)), ("a".into(), Value::Int(2))]);
        assert_eq!(map.to_string(), "{a: 2, z: 1}");
    }

    #[test]
    fn host_collection_views_are_owned_and_non_borrowing() {
        let list = Value::list(vec![Value::Int(1)]);
        let mut items = list.list_items().unwrap();
        items.push(Value::Int(2));
        assert_eq!(list.len(), Some(1));

        let map = Value::map([("score".into(), Value::Int(17))]);
        assert_eq!(map.map_get("score").unwrap(), Some(Value::Int(17)));
        let entries = map.map_entries().unwrap();
        assert_eq!(entries.get("score"), Some(&Value::Int(17)));
    }
}
