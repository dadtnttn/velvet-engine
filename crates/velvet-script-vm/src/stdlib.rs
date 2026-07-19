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
            Ok(NativeOutput {
                value: Value::Float(a.powf(b)),
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
            let hex = velvet_crypto::hash_sha256_hex(s.as_bytes())
                .map_err(|e| e.to_string())?;
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
        assert_eq!(NativeId::all().len(), 18);
        assert_eq!(lookup_native("sin"), Some(NativeId::Sin));
        assert_eq!(lookup_native("hash_sha256"), Some(NativeId::HashSha256));
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
}
