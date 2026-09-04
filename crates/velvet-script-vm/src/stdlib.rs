//! Host / native standard library functions.

use std::rc::Rc;

use velvet_script_bytecode::NativeId;

use crate::value::Value;

/// Result of invoking a native (value + optional print side-effect).
#[derive(Debug, Clone)]
pub struct NativeOutput {
    /// Return value.
    pub value: Value,
    /// Optional printed line (for `print`).
    pub printed: Option<String>,
}

/// Invoke a native by id with already-evaluated arguments.
pub fn call_native(id: u16, args: &[Value]) -> Result<NativeOutput, String> {
    let native = NativeId::from_u16(id).ok_or_else(|| format!("unknown native id {id}"))?;
    match native {
        NativeId::Print => {
            let line = args
                .iter()
                .map(|a| a.to_string())
                .collect::<Vec<_>>()
                .join(" ");
            Ok(NativeOutput {
                value: Value::Null,
                printed: Some(line),
            })
        }
        NativeId::Abs => {
            expect_argc("abs", args, 1)?;
            Ok(NativeOutput {
                value: abs_val(&args[0])?,
                printed: None,
            })
        }
        NativeId::Min => {
            expect_argc("min", args, 2)?;
            Ok(NativeOutput {
                value: min_max(&args[0], &args[1], true)?,
                printed: None,
            })
        }
        NativeId::Max => {
            expect_argc("max", args, 2)?;
            Ok(NativeOutput {
                value: min_max(&args[0], &args[1], false)?,
                printed: None,
            })
        }
        NativeId::Floor => {
            expect_argc("floor", args, 1)?;
            let f = args[0]
                .as_f64()
                .ok_or_else(|| "floor expects a number".to_string())?;
            Ok(NativeOutput {
                value: Value::Float(f.floor()),
                printed: None,
            })
        }
        NativeId::Ceil => {
            expect_argc("ceil", args, 1)?;
            let f = args[0]
                .as_f64()
                .ok_or_else(|| "ceil expects a number".to_string())?;
            Ok(NativeOutput {
                value: Value::Float(f.ceil()),
                printed: None,
            })
        }
        NativeId::Clamp => {
            expect_argc("clamp", args, 3)?;
            let x = args[0]
                .as_f64()
                .ok_or_else(|| "clamp expects numbers".to_string())?;
            let lo = args[1]
                .as_f64()
                .ok_or_else(|| "clamp expects numbers".to_string())?;
            let hi = args[2]
                .as_f64()
                .ok_or_else(|| "clamp expects numbers".to_string())?;
            let (lo, hi) = if lo <= hi { (lo, hi) } else { (hi, lo) };
            let v = x.clamp(lo, hi);
            // Prefer int when all inputs were ints.
            let value = if args.iter().all(|a| matches!(a, Value::Int(_))) {
                Value::Int(v as i64)
            } else {
                Value::Float(v)
            };
            Ok(NativeOutput {
                value,
                printed: None,
            })
        }
        NativeId::Len => {
            expect_argc("len", args, 1)?;
            let n = args[0]
                .len()
                .ok_or_else(|| "len expects string, list, or map".to_string())?;
            Ok(NativeOutput {
                value: Value::Int(n as i64),
                printed: None,
            })
        }
        NativeId::Concat => {
            if args.is_empty() {
                return Ok(NativeOutput {
                    value: Value::String(Rc::from("")),
                    printed: None,
                });
            }
            let mut s = String::new();
            for a in args {
                s.push_str(&a.to_string());
            }
            Ok(NativeOutput {
                value: Value::String(Rc::from(s)),
                printed: None,
            })
        }
        NativeId::Str => {
            expect_argc("str", args, 1)?;
            Ok(NativeOutput {
                value: Value::String(Rc::from(args[0].to_string())),
                printed: None,
            })
        }
        NativeId::Sin => {
            expect_argc("sin", args, 1)?;
            let f = args[0]
                .as_f64()
                .ok_or_else(|| "sin expects a number".to_string())?;
            Ok(NativeOutput {
                value: Value::Float(f.sin()),
                printed: None,
            })
        }
        NativeId::Cos => {
            expect_argc("cos", args, 1)?;
            let f = args[0]
                .as_f64()
                .ok_or_else(|| "cos expects a number".to_string())?;
            Ok(NativeOutput {
                value: Value::Float(f.cos()),
                printed: None,
            })
        }
        NativeId::Sqrt => {
            expect_argc("sqrt", args, 1)?;
            let f = args[0]
                .as_f64()
                .ok_or_else(|| "sqrt expects a number".to_string())?;
            if !f.is_finite() || f < 0.0 {
                return Err(format!(
                    "sqrt domain error: expected finite x >= 0, received {f}"
                ));
            }
            Ok(NativeOutput {
                value: Value::Float(f.sqrt()),
                printed: None,
            })
        }
        NativeId::Pow => {
            expect_argc("pow", args, 2)?;
            let a = args[0]
                .as_f64()
                .ok_or_else(|| "pow expects numbers".to_string())?;
            let b = args[1]
                .as_f64()
                .ok_or_else(|| "pow expects numbers".to_string())?;
            let result = a.powf(b);
            if !result.is_finite() {
                return Err(format!(
                    "pow domain error: ({a}, {b}) produced a non-finite result"
                ));
            }
            Ok(NativeOutput {
                value: Value::Float(result),
                printed: None,
            })
        }
        NativeId::Lerp => {
            expect_argc("lerp", args, 3)?;
            let a = args[0]
                .as_f64()
                .ok_or_else(|| "lerp expects numbers".to_string())?;
            let b = args[1]
                .as_f64()
                .ok_or_else(|| "lerp expects numbers".to_string())?;
            let t = args[2]
                .as_f64()
                .ok_or_else(|| "lerp expects numbers".to_string())?;
            Ok(NativeOutput {
                value: Value::Float(a + (b - a) * t),
                printed: None,
            })
        }
        NativeId::HashSha256 => {
            expect_argc("hash_sha256", args, 1)?;
            let s = args[0].to_string();
            let hex = velvet_crypto::hash_sha256_hex(s.as_bytes()).map_err(|e| e.to_string())?;
            Ok(NativeOutput {
                value: Value::String(Rc::from(hex)),
                printed: None,
            })
        }
        NativeId::HexEncode => {
            expect_argc("hex_encode", args, 1)?;
            let s = args[0].to_string();
            Ok(NativeOutput {
                value: Value::String(Rc::from(velvet_crypto::hex_encode(s.as_bytes()))),
                printed: None,
            })
        }
        NativeId::Base64Encode => {
            expect_argc("base64_encode", args, 1)?;
            let s = args[0].to_string();
            let b64 = velvet_crypto::base64_encode(s.as_bytes()).map_err(|e| e.to_string())?;
            Ok(NativeOutput {
                value: Value::String(Rc::from(b64)),
                printed: None,
            })
        }
        NativeId::TypeOf => {
            expect_argc("type_of", args, 1)?;
            Ok(NativeOutput {
                value: Value::String(Rc::from(args[0].type_name())),
                printed: None,
            })
        }
        NativeId::ListPush => {
            expect_argc("list_push", args, 2)?;
            args[0].list_push(args[1].clone())?;
            Ok(NativeOutput {
                value: args[0].clone(),
                printed: None,
            })
        }
        NativeId::ListPop => {
            expect_argc("list_pop", args, 1)?;
            Ok(NativeOutput {
                value: args[0].list_pop()?,
                printed: None,
            })
        }
        NativeId::MapHas => {
            expect_argc("map_has", args, 2)?;
            let key = args[1]
                .as_str()
                .ok_or_else(|| "map_has expects a string key".to_string())?;
            Ok(NativeOutput {
                value: Value::Bool(args[0].map_has(key)?),
                printed: None,
            })
        }
        NativeId::MapKeys => {
            expect_argc("map_keys", args, 1)?;
            let keys = args[0]
                .map_keys()?
                .into_iter()
                .map(|key| Value::String(Rc::from(key)))
                .collect();
            Ok(NativeOutput {
                value: Value::list(keys),
                printed: None,
            })
        }
        NativeId::StrUpper => {
            expect_argc("str_upper", args, 1)?;
            let s = as_string(&args[0], "str_upper")?;
            Ok(str_output(s.to_uppercase()))
        }
        NativeId::StrLower => {
            expect_argc("str_lower", args, 1)?;
            let s = as_string(&args[0], "str_lower")?;
            Ok(str_output(s.to_lowercase()))
        }
        NativeId::StrTrim => {
            expect_argc("str_trim", args, 1)?;
            let s = as_string(&args[0], "str_trim")?;
            Ok(str_output(s.trim().to_string()))
        }
        NativeId::StrSplit => {
            expect_argc("str_split", args, 2)?;
            let s = as_string(&args[0], "str_split")?;
            let sep = as_string(&args[1], "str_split")?;
            let parts: Vec<Value> = if sep.is_empty() {
                s.chars().map(|c| string_val(c.to_string())).collect()
            } else {
                s.split(sep.as_str())
                    .map(|p| string_val(p.to_string()))
                    .collect()
            };
            Ok(list_output(parts))
        }
        NativeId::StrJoin => {
            expect_argc("str_join", args, 2)?;
            let items = as_list(&args[0], "str_join")?;
            let sep = as_string(&args[1], "str_join")?;
            let joined = items
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(sep.as_str());
            Ok(str_output(joined))
        }
        NativeId::StrContains => {
            expect_argc("str_contains", args, 2)?;
            let s = as_string(&args[0], "str_contains")?;
            let sub = as_string(&args[1], "str_contains")?;
            Ok(bool_output(s.contains(sub.as_str())))
        }
        NativeId::StrReplace => {
            expect_argc("str_replace", args, 3)?;
            let s = as_string(&args[0], "str_replace")?;
            let from = as_string(&args[1], "str_replace")?;
            let to = as_string(&args[2], "str_replace")?;
            Ok(str_output(s.replace(from.as_str(), to.as_str())))
        }
        NativeId::StrStartsWith => {
            expect_argc("str_starts_with", args, 2)?;
            let s = as_string(&args[0], "str_starts_with")?;
            let prefix = as_string(&args[1], "str_starts_with")?;
            Ok(bool_output(s.starts_with(prefix.as_str())))
        }
        NativeId::StrEndsWith => {
            expect_argc("str_ends_with", args, 2)?;
            let s = as_string(&args[0], "str_ends_with")?;
            let suffix = as_string(&args[1], "str_ends_with")?;
            Ok(bool_output(s.ends_with(suffix.as_str())))
        }
        NativeId::ListContains => {
            expect_argc("list_contains", args, 2)?;
            let items = as_list(&args[0], "list_contains")?;
            Ok(bool_output(items.iter().any(|item| item == &args[1])))
        }
        NativeId::ListIndexOf => {
            expect_argc("list_index_of", args, 2)?;
            let items = as_list(&args[0], "list_index_of")?;
            let index = items
                .iter()
                .position(|item| item == &args[1])
                .map(|i| i as i64)
                .unwrap_or(-1);
            Ok(NativeOutput {
                value: Value::Int(index),
                printed: None,
            })
        }
        NativeId::ListReverse => {
            expect_argc("list_reverse", args, 1)?;
            match &args[0] {
                Value::List(items) => {
                    items.borrow_mut().reverse();
                    Ok(NativeOutput {
                        value: args[0].clone(),
                        printed: None,
                    })
                }
                _ => Err("list_reverse expects a list".into()),
            }
        }
        NativeId::ListSort => {
            expect_argc("list_sort", args, 1)?;
            match &args[0] {
                Value::List(items) => {
                    items.borrow_mut().sort_by(compare_values);
                    Ok(NativeOutput {
                        value: args[0].clone(),
                        printed: None,
                    })
                }
                _ => Err("list_sort expects a list".into()),
            }
        }
        NativeId::ListSlice => {
            if args.len() < 2 || args.len() > 3 {
                return Err(format!(
                    "list_slice expected 2..=3 argument(s), got {}",
                    args.len()
                ));
            }
            let items = as_list(&args[0], "list_slice")?;
            let len = items.len() as i64;
            let start = normalize_index(arg_i64(&args[1], "list_slice")?, len);
            let end = match args.get(2) {
                Some(v) => normalize_index(arg_i64(v, "list_slice")?, len),
                None => len,
            };
            let (start, end) = (start.min(end) as usize, end.max(start) as usize);
            Ok(list_output(
                items[start.min(items.len())..end.min(items.len())].to_vec(),
            ))
        }
        NativeId::MapValues => {
            expect_argc("map_values", args, 1)?;
            match &args[0] {
                Value::Map(entries) => {
                    Ok(list_output(entries.borrow().values().cloned().collect()))
                }
                _ => Err("map_values expects a map".into()),
            }
        }
        NativeId::MapRemove => {
            expect_argc("map_remove", args, 2)?;
            let key = as_string(&args[1], "map_remove")?;
            match &args[0] {
                Value::Map(entries) => Ok(bool_output(
                    entries.borrow_mut().remove(key.as_str()).is_some(),
                )),
                _ => Err("map_remove expects a map".into()),
            }
        }
        NativeId::MapMerge => {
            expect_argc("map_merge", args, 2)?;
            match (&args[0], &args[1]) {
                (Value::Map(a), Value::Map(b)) => {
                    let mut merged = a.borrow().clone();
                    for (k, v) in b.borrow().iter() {
                        merged.insert(k.clone(), v.clone());
                    }
                    Ok(NativeOutput {
                        value: Value::Map(Rc::new(std::cell::RefCell::new(merged))),
                        printed: None,
                    })
                }
                _ => Err("map_merge expects two maps".into()),
            }
        }
        NativeId::Assert => {
            if args.is_empty() || args.len() > 2 {
                return Err(format!(
                    "assert expected 1 or 2 arguments, got {}",
                    args.len()
                ));
            }
            if !args[0].is_truthy() {
                return Err(args
                    .get(1)
                    .map(ToString::to_string)
                    .unwrap_or_else(|| "assertion failed".into()));
            }
            Ok(NativeOutput {
                value: Value::Null,
                printed: None,
            })
        }
        NativeId::Fail => {
            expect_argc("fail", args, 1)?;
            Err(args[0].to_string())
        }
        // Presentation host natives — mutate PresentHostState (no drawing).
        NativeId::PresentShow => {
            if args.is_empty() || args.len() > 3 {
                return Err(format!(
                    "present_show expected 1..=3 arguments (id [, expression [, at]]), got {}",
                    args.len()
                ));
            }
            let id = args[0].to_string();
            let expression = args.get(1).map(|v| v.to_string()).filter(|s| !s.is_empty());
            let at = args.get(2).map(|v| v.to_string()).filter(|s| !s.is_empty());
            crate::present_host::present_host_mut(|h| {
                h.show(id, expression, at);
            });
            Ok(NativeOutput {
                value: Value::Null,
                printed: None,
            })
        }
        NativeId::PresentHide => {
            expect_argc("present_hide", args, 1)?;
            let id = args[0].to_string();
            crate::present_host::present_host_mut(|h| {
                h.hide(&id);
            });
            Ok(NativeOutput {
                value: Value::Null,
                printed: None,
            })
        }
        NativeId::PresentSetBg => {
            expect_argc("set_bg", args, 1)?;
            let path = args[0].to_string();
            crate::present_host::present_host_mut(|h| {
                h.set_bg(path);
            });
            Ok(NativeOutput {
                value: Value::Null,
                printed: None,
            })
        }
        NativeId::PresentUiFlag => {
            expect_argc("ui_flag", args, 2)?;
            let name = args[0].to_string();
            let on = value_as_bool(&args[1])?;
            crate::present_host::present_host_mut(|h| {
                h.set_ui_flag(name, on);
            });
            Ok(NativeOutput {
                value: Value::Null,
                printed: None,
            })
        }
        NativeId::PresentUiFlagGet => {
            expect_argc("ui_flag_get", args, 1)?;
            let name = args[0].to_string();
            let on = crate::present_host::present_host_mut(|h| h.ui_flag(&name));
            Ok(NativeOutput {
                value: Value::Bool(on),
                printed: None,
            })
        }
        native => crate::math::call_math_native(native, args),
    }
}

fn value_as_bool(v: &Value) -> Result<bool, String> {
    match v {
        Value::Bool(b) => Ok(*b),
        Value::Int(i) => Ok(*i != 0),
        Value::Float(f) => Ok(*f != 0.0),
        Value::String(s) => {
            let t = s.as_ref();
            Ok(t == "true" || t == "1" || t.eq_ignore_ascii_case("yes"))
        }
        Value::Null => Ok(false),
        _ => Err("ui_flag expects a bool-ish second argument".into()),
    }
}

fn expect_argc(name: &str, args: &[Value], n: usize) -> Result<(), String> {
    if args.len() != n {
        return Err(format!(
            "{name} expected {n} argument(s), got {}",
            args.len()
        ));
    }
    Ok(())
}

fn str_output(s: String) -> NativeOutput {
    NativeOutput {
        value: Value::String(Rc::from(s)),
        printed: None,
    }
}

fn string_val(s: String) -> Value {
    Value::String(Rc::from(s))
}

fn list_output(items: Vec<Value>) -> NativeOutput {
    NativeOutput {
        value: Value::list(items),
        printed: None,
    }
}

fn bool_output(b: bool) -> NativeOutput {
    NativeOutput {
        value: Value::Bool(b),
        printed: None,
    }
}

fn as_string(v: &Value, name: &str) -> Result<String, String> {
    v.as_str()
        .map(str::to_string)
        .ok_or_else(|| format!("{name} expects a string, got `{}`", v.type_name()))
}

fn as_list(v: &Value, name: &str) -> Result<Vec<Value>, String> {
    v.list_items()
        .map_err(|_| format!("{name} expects a list, got `{}`", v.type_name()))
}

fn arg_i64(v: &Value, name: &str) -> Result<i64, String> {
    v.as_i64()
        .ok_or_else(|| format!("{name} expects an integer index, got `{}`", v.type_name()))
}

/// Normalize possibly-negative index into `0..=len`.
fn normalize_index(i: i64, len: i64) -> i64 {
    let i = if i < 0 { len + i } else { i };
    i.clamp(0, len)
}

/// Deterministic total order for `list_sort`: numbers, then strings,
/// then everything else by type name + display.
fn compare_values(a: &Value, b: &Value) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    let rank = |v: &Value| match v {
        Value::Null => 0,
        Value::Bool(_) => 1,
        Value::Int(_) | Value::Float(_) => 2,
        Value::String(_) => 3,
        _ => 4,
    };
    match rank(a).cmp(&rank(b)) {
        Ordering::Equal => {}
        other => return other,
    }
    match (a, b) {
        (Value::Null, Value::Null) => Ordering::Equal,
        (Value::Bool(x), Value::Bool(y)) => x.cmp(y),
        (x, y)
            if matches!(x, Value::Int(_) | Value::Float(_))
                && matches!(y, Value::Int(_) | Value::Float(_)) =>
        {
            let xf = x.as_f64().unwrap_or(f64::NAN);
            let yf = y.as_f64().unwrap_or(f64::NAN);
            xf.partial_cmp(&yf).unwrap_or(Ordering::Equal)
        }
        (Value::String(x), Value::String(y)) => x.cmp(y),
        _ => a
            .type_name()
            .cmp(b.type_name())
            .then_with(|| a.to_string().cmp(&b.to_string())),
    }
}

fn abs_val(v: &Value) -> Result<Value, String> {
    match v {
        Value::Int(i) => Ok(Value::Int(i.saturating_abs())),
        Value::Float(f) => Ok(Value::Float(f.abs())),
        _ => Err("abs expects a number".into()),
    }
}

fn min_max(a: &Value, b: &Value, want_min: bool) -> Result<Value, String> {
    if let (Value::Int(ai), Value::Int(bi)) = (a, b) {
        let v = if want_min {
            (*ai).min(*bi)
        } else {
            (*ai).max(*bi)
        };
        return Ok(Value::Int(v));
    }
    let af = a
        .as_f64()
        .ok_or_else(|| "min/max expects numbers".to_string())?;
    let bf = b
        .as_f64()
        .ok_or_else(|| "min/max expects numbers".to_string())?;
    let v = if want_min { af.min(bf) } else { af.max(bf) };
    Ok(Value::Float(v))
}

#[cfg(test)]
mod tests {
    use super::*;
    use velvet_script_bytecode::lookup_native;

    #[test]
    fn abs_min_max_clamp() {
        assert_eq!(
            call_native(NativeId::Abs.as_u16(), &[Value::Int(-3)])
                .unwrap()
                .value,
            Value::Int(3)
        );
        assert_eq!(
            call_native(NativeId::Min.as_u16(), &[Value::Int(3), Value::Int(1)])
                .unwrap()
                .value,
            Value::Int(1)
        );
        assert_eq!(
            call_native(NativeId::Max.as_u16(), &[Value::Int(3), Value::Int(1)])
                .unwrap()
                .value,
            Value::Int(3)
        );
        assert_eq!(
            call_native(
                NativeId::Clamp.as_u16(),
                &[Value::Int(10), Value::Int(0), Value::Int(5)]
            )
            .unwrap()
            .value,
            Value::Int(5)
        );
    }

    #[test]
    fn floor_ceil_len_concat() {
        assert_eq!(
            call_native(NativeId::Floor.as_u16(), &[Value::Float(3.7)])
                .unwrap()
                .value,
            Value::Float(3.0)
        );
        assert_eq!(
            call_native(NativeId::Ceil.as_u16(), &[Value::Float(3.2)])
                .unwrap()
                .value,
            Value::Float(4.0)
        );
        let list = Value::list(vec![Value::Int(1), Value::Int(2)]);
        assert_eq!(
            call_native(NativeId::Len.as_u16(), &[list]).unwrap().value,
            Value::Int(2)
        );
        assert_eq!(
            call_native(
                NativeId::Concat.as_u16(),
                &[
                    Value::String(Rc::from("a")),
                    Value::Int(1),
                    Value::String(Rc::from("b"))
                ]
            )
            .unwrap()
            .value
            .as_str(),
            Some("a1b")
        );
    }

    #[test]
    fn lookup_names() {
        assert_eq!(lookup_native("abs"), Some(NativeId::Abs));
        assert!(lookup_native("nope").is_none());
        assert_eq!(NativeId::all().len(), 162);
        // New collection natives resolve by name and keep stable ids.
        assert_eq!(lookup_native("str_split"), Some(NativeId::StrSplit));
        assert_eq!(lookup_native("list_sort"), Some(NativeId::ListSort));
        assert_eq!(lookup_native("map_merge"), Some(NativeId::MapMerge));
        assert_eq!(NativeId::StrUpper.as_u16(), 145);
        assert_eq!(NativeId::MapMerge.as_u16(), 161);
        // Old ids never move (compat).
        assert_eq!(NativeId::Fail.as_u16(), 29);
        assert_eq!(NativeId::RayPlaneIntersection.as_u16(), 144);
        assert_eq!(NativeId::Fail.as_u16(), 29);
        assert_eq!(NativeId::Tan.as_u16(), 30);
        assert!(lookup_native("present_show").is_some());
        assert!(lookup_native("set_bg").is_some());
        assert!(lookup_native("ui_flag").is_some());
        assert!(lookup_native("ui_flag_get").is_some());
        assert!(lookup_native("present_hide").is_some());
        assert_eq!(lookup_native("sin"), Some(NativeId::Sin));
        assert_eq!(lookup_native("hash_sha256"), Some(NativeId::HashSha256));
        assert_eq!(lookup_native("type_of"), Some(NativeId::TypeOf));
        assert_eq!(lookup_native("fail"), Some(NativeId::Fail));
    }

    #[test]
    fn hash_sha256_native() {
        let out = call_native(
            NativeId::HashSha256.as_u16(),
            &[Value::String(Rc::from(""))],
        )
        .unwrap();
        assert_eq!(
            out.value.as_str(),
            Some("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855")
        );
    }

    #[test]
    fn print_captures() {
        let out = call_native(
            NativeId::Print.as_u16(),
            &[Value::String(Rc::from("hi")), Value::Int(1)],
        )
        .unwrap();
        assert_eq!(out.printed.as_deref(), Some("hi 1"));
        assert_eq!(out.value, Value::Null);
    }

    #[test]
    fn str_and_concat_mixed() {
        let s = call_native(NativeId::Str.as_u16(), &[Value::Int(42)])
            .unwrap()
            .value;
        assert_eq!(s.as_str(), Some("42"));
        let s2 = call_native(NativeId::Str.as_u16(), &[Value::Bool(true)])
            .unwrap()
            .value;
        assert_eq!(s2.as_str(), Some("true"));
        let cat = call_native(
            NativeId::Concat.as_u16(),
            &[
                Value::Int(1),
                Value::String(Rc::from("-")),
                Value::Float(2.5),
            ],
        )
        .unwrap()
        .value;
        let text = cat.as_str().unwrap();
        assert!(text.contains('1') && text.contains('2'), "text={text}");
    }

    #[test]
    fn abs_float_and_int() {
        assert_eq!(
            call_native(NativeId::Abs.as_u16(), &[Value::Float(-2.5)])
                .unwrap()
                .value,
            Value::Float(2.5)
        );
        assert_eq!(
            call_native(NativeId::Abs.as_u16(), &[Value::Int(0)])
                .unwrap()
                .value,
            Value::Int(0)
        );
    }

    #[test]
    fn clamp_float_edges() {
        let v = call_native(
            NativeId::Clamp.as_u16(),
            &[Value::Float(-1.0), Value::Float(0.0), Value::Float(1.0)],
        )
        .unwrap()
        .value;
        assert_eq!(v, Value::Float(0.0));
        let v2 = call_native(
            NativeId::Clamp.as_u16(),
            &[Value::Float(5.0), Value::Float(0.0), Value::Float(1.0)],
        )
        .unwrap()
        .value;
        assert_eq!(v2, Value::Float(1.0));
    }

    #[test]
    fn len_string_and_empty_list() {
        let s = Value::String(Rc::from("abc"));
        assert_eq!(
            call_native(NativeId::Len.as_u16(), &[s]).unwrap().value,
            Value::Int(3)
        );
        let empty = Value::list(vec![]);
        assert_eq!(
            call_native(NativeId::Len.as_u16(), &[empty]).unwrap().value,
            Value::Int(0)
        );
    }

    #[test]
    fn min_max_float() {
        assert_eq!(
            call_native(
                NativeId::Min.as_u16(),
                &[Value::Float(1.5), Value::Float(1.2)]
            )
            .unwrap()
            .value,
            Value::Float(1.2)
        );
        assert_eq!(
            call_native(
                NativeId::Max.as_u16(),
                &[Value::Float(1.5), Value::Float(1.2)]
            )
            .unwrap()
            .value,
            Value::Float(1.5)
        );
    }

    #[test]
    fn arity_errors() {
        assert!(call_native(NativeId::Abs.as_u16(), &[]).is_err());
        assert!(call_native(NativeId::Min.as_u16(), &[Value::Int(1)]).is_err());
        assert!(call_native(NativeId::Clamp.as_u16(), &[Value::Int(1), Value::Int(2)]).is_err());
    }

    #[test]
    fn all_native_ids_dispatch() {
        for id in NativeId::all() {
            // Smoke: wrong arity should not panic.
            let _ = call_native(id.as_u16(), &[]);
        }
        assert!(lookup_native("str").is_some());
        assert!(lookup_native("floor").is_some());
        assert!(lookup_native("ceil").is_some());
    }

    #[test]
    fn string_natives() {
        let s = |t: &str| Value::String(Rc::from(t));
        let call = |id: NativeId, args: &[Value]| call_native(id.as_u16(), args).unwrap().value;
        assert_eq!(
            call(NativeId::StrUpper, &[s("hola")]).as_str(),
            Some("HOLA")
        );
        assert_eq!(
            call(NativeId::StrLower, &[s("HoLa")]).as_str(),
            Some("hola")
        );
        assert_eq!(call(NativeId::StrTrim, &[s("  x \t")]).as_str(), Some("x"));
        let parts = call(NativeId::StrSplit, &[s("a,b,c"), s(",")]);
        assert_eq!(parts.to_string(), "[a, b, c]");
        let chars = call(NativeId::StrSplit, &[s("ab"), s("")]);
        assert_eq!(chars.to_string(), "[a, b]");
        let list = Value::list(vec![s("a"), s("b")]);
        assert_eq!(
            call(NativeId::StrJoin, &[list, s("-")]).as_str(),
            Some("a-b")
        );
        assert_eq!(
            call(NativeId::StrContains, &[s("hello"), s("ell")]),
            Value::Bool(true)
        );
        assert_eq!(
            call(NativeId::StrReplace, &[s("aaa"), s("a"), s("b")]).as_str(),
            Some("bbb")
        );
        assert_eq!(
            call(NativeId::StrStartsWith, &[s("hello"), s("he")]),
            Value::Bool(true)
        );
        assert_eq!(
            call(NativeId::StrEndsWith, &[s("hello"), s("lo")]),
            Value::Bool(true)
        );
        assert_eq!(
            call(NativeId::StrEndsWith, &[s("hello"), s("he")]),
            Value::Bool(false)
        );
        // type errors, not panics
        assert!(call_native(NativeId::StrUpper.as_u16(), &[Value::Int(1)]).is_err());
        assert!(call_native(NativeId::StrSplit.as_u16(), &[s("a")]).is_err());
    }

    #[test]
    fn list_and_map_natives() {
        let call = |id: NativeId, args: &[Value]| call_native(id.as_u16(), args).unwrap().value;
        let list = Value::list(vec![Value::Int(1), Value::Int(2), Value::Int(3)]);
        assert_eq!(
            call(NativeId::ListContains, &[list.clone(), Value::Int(2)]),
            Value::Bool(true)
        );
        assert_eq!(
            call(NativeId::ListContains, &[list.clone(), Value::Int(9)]),
            Value::Bool(false)
        );
        assert_eq!(
            call(NativeId::ListIndexOf, &[list.clone(), Value::Int(3)]),
            Value::Int(2)
        );
        assert_eq!(
            call(NativeId::ListIndexOf, &[list.clone(), Value::Int(9)]),
            Value::Int(-1)
        );
        // reverse + sort mutate like list_push and return the same list
        let rev = Value::list(vec![Value::Int(1), Value::Int(2)]);
        call(NativeId::ListReverse, std::slice::from_ref(&rev));
        assert_eq!(rev.to_string(), "[2, 1]");
        let unsorted = Value::list(vec![
            Value::String(Rc::from("b")),
            Value::Int(3),
            Value::String(Rc::from("a")),
            Value::Int(1),
        ]);
        call(NativeId::ListSort, std::slice::from_ref(&unsorted));
        assert_eq!(unsorted.to_string(), "[1, 3, a, b]");
        // slice is a pure copy with negative support
        let sliced = call(
            NativeId::ListSlice,
            &[list.clone(), Value::Int(1), Value::Int(-1)],
        );
        assert_eq!(sliced.to_string(), "[2]");
        let tail = call(NativeId::ListSlice, &[list.clone(), Value::Int(-2)]);
        assert_eq!(tail.to_string(), "[2, 3]");
        // maps
        let map = Value::map([("b".into(), Value::Int(2)), ("a".into(), Value::Int(1))]);
        let values = call(NativeId::MapValues, std::slice::from_ref(&map));
        assert_eq!(values.to_string(), "[1, 2]");
        assert_eq!(
            call(
                NativeId::MapRemove,
                &[map.clone(), Value::String(Rc::from("a"))]
            ),
            Value::Bool(true)
        );
        assert_eq!(
            call(
                NativeId::MapRemove,
                &[map.clone(), Value::String(Rc::from("zzz"))]
            ),
            Value::Bool(false)
        );
        let other = Value::map([("a".into(), Value::Int(99))]);
        let merged = call(NativeId::MapMerge, &[map.clone(), other]);
        assert_eq!(merged.to_string(), "{a: 99, b: 2}");
        // originals untouched by merge
        assert_eq!(map.to_string(), "{b: 2}");
        assert!(call_native(NativeId::ListSort.as_u16(), &[Value::Int(1)]).is_err());
    }

    #[test]
    fn collection_and_error_natives() {
        let list = Value::list(vec![Value::Int(1)]);
        call_native(NativeId::ListPush.as_u16(), &[list.clone(), Value::Int(2)]).unwrap();
        assert_eq!(list.len(), Some(2));
        assert_eq!(
            call_native(NativeId::ListPop.as_u16(), &[list])
                .unwrap()
                .value,
            Value::Int(2)
        );

        let map = Value::map([("z".into(), Value::Int(1)), ("a".into(), Value::Int(2))]);
        assert_eq!(
            call_native(
                NativeId::MapHas.as_u16(),
                &[map.clone(), Value::String(Rc::from("a"))]
            )
            .unwrap()
            .value,
            Value::Bool(true)
        );
        let keys = call_native(NativeId::MapKeys.as_u16(), &[map])
            .unwrap()
            .value;
        assert_eq!(keys.to_string(), "[a, z]");
        assert!(call_native(
            NativeId::Assert.as_u16(),
            &[Value::Bool(false), Value::String(Rc::from("bad state"))]
        )
        .is_err());
    }
}
