//! Runtime values.

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;

use velvet_script_bytecode::Constant;

/// Runtime value.
#[derive(Debug, Clone, PartialEq)]
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
    Map(Rc<RefCell<HashMap<String, Value>>>),
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
            Self::Function(_) | Self::Native(_) => true,
        }
    }

    /// Approximate heap units for memory limit (rough).
    pub fn memory_units(&self) -> usize {
        match self {
            Self::Null
            | Self::Bool(_)
            | Self::Int(_)
            | Self::Float(_)
            | Self::Function(_)
            | Self::Native(_) => 1,
            Self::String(s) => 1 + s.len() / 8,
            Self::List(l) => 1 + l.borrow().iter().map(Value::memory_units).sum::<usize>(),
            Self::Map(m) => {
                1 + m
                    .borrow()
                    .iter()
                    .map(|(k, v)| 1 + k.len() / 8 + v.memory_units())
                    .sum::<usize>()
            }
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
            _ => Err("value is not a mutable index target".into()),
        }
    }

    /// Build a list value.
    pub fn list(items: Vec<Value>) -> Self {
        Self::List(Rc::new(RefCell::new(items)))
    }

    /// Build a map value.
    pub fn map(entries: HashMap<String, Value>) -> Self {
        Self::Map(Rc::new(RefCell::new(entries)))
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
            Self::Function(_) => "function",
            Self::Native(_) => "native",
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Null => write!(f, "null"),
            Self::Bool(b) => write!(f, "{b}"),
            Self::Int(i) => write!(f, "{i}"),
            Self::Float(x) => write!(f, "{x}"),
            Self::String(s) => write!(f, "{s}"),
            Self::List(l) => {
                write!(f, "[")?;
                for (i, v) in l.borrow().iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{v}")?;
                }
                write!(f, "]")
            }
            Self::Map(m) => {
                write!(f, "{{")?;
                for (i, (k, v)) in m.borrow().iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{k}: {v}")?;
                }
                write!(f, "}}")
            }
            Self::Function(i) => write!(f, "<fn #{i}>"),
            Self::Native(i) => write!(f, "<native #{i}>"),
        }
    }
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
        let mut m = HashMap::new();
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
}
