//! Advanced deterministic-friendly mathematics for VS3 natives.

use std::cell::RefCell;
use std::rc::Rc;

use velvet_script_bytecode::{NativeId, Op};

use crate::stdlib::NativeOutput;
use crate::Value;

const DEFAULT_EPSILON: f64 = 1.0e-9;
const ZERO_EPSILON: f64 = 1.0e-12;

/// Invoke an advanced mathematics native.
pub(crate) fn call_math_native(native: NativeId, args: &[Value]) -> Result<NativeOutput, String> {
    validate_arity(native, args)?;
    use NativeId as N;

    let value = match native {
        N::Tan => float_result("tan", number(&args[0], "tan")?.tan())?,
        N::Asin => {
            let value = number(&args[0], "asin")?;
            require_range("asin", value, -1.0, 1.0)?;
            Value::Float(value.asin())
        }
        N::Acos => {
            let value = number(&args[0], "acos")?;
            require_range("acos", value, -1.0, 1.0)?;
            Value::Float(value.acos())
        }
        N::Atan => Value::Float(number(&args[0], "atan")?.atan()),
        N::Atan2 => Value::Float(number(&args[0], "atan2")?.atan2(number(&args[1], "atan2")?)),
        N::Exp => float_result("exp", number(&args[0], "exp")?.exp())?,
        N::Ln | N::Log2 | N::Log10 => {
            let value = number(&args[0], native.name())?;
            if value <= 0.0 {
                return Err(format!(
                    "{} domain error: expected x > 0, received {value}",
                    native.name()
                ));
            }
            let result = match native {
                N::Ln => value.ln(),
                N::Log2 => value.log2(),
                N::Log10 => value.log10(),
                _ => unreachable!(),
            };
            Value::Float(result)
        }
        N::Cbrt => Value::Float(number(&args[0], "cbrt")?.cbrt()),
        N::Round => Value::Float(number(&args[0], "round")?.round()),
        N::Trunc => Value::Float(number(&args[0], "trunc")?.trunc()),
        N::Fract => Value::Float(number(&args[0], "fract")?.fract()),
        N::Sign => Value::Float(number(&args[0], "sign")?.signum()),
        N::Hypot => Value::Float(number(&args[0], "hypot")?.hypot(number(&args[1], "hypot")?)),
        N::Degrees => Value::Float(number(&args[0], "degrees")?.to_degrees()),
        N::Radians => Value::Float(number(&args[0], "radians")?.to_radians()),
        N::InverseLerp => {
            let a = number(&args[0], "inverse_lerp")?;
            let b = number(&args[1], "inverse_lerp")?;
            let value = number(&args[2], "inverse_lerp")?;
            let span = b - a;
            if span.abs() <= ZERO_EPSILON {
                return Err("inverse_lerp domain error: endpoints must differ".into());
            }
            Value::Float((value - a) / span)
        }
        N::Remap => {
            let value = number(&args[0], "remap")?;
            let in_min = number(&args[1], "remap")?;
            let in_max = number(&args[2], "remap")?;
            let out_min = number(&args[3], "remap")?;
            let out_max = number(&args[4], "remap")?;
            let span = in_max - in_min;
            if span.abs() <= ZERO_EPSILON {
                return Err("remap domain error: input endpoints must differ".into());
            }
            Value::Float(out_min + (out_max - out_min) * ((value - in_min) / span))
        }
        N::Smoothstep => {
            let edge0 = number(&args[0], "smoothstep")?;
            let edge1 = number(&args[1], "smoothstep")?;
            let value = number(&args[2], "smoothstep")?;
            let span = edge1 - edge0;
            if span.abs() <= ZERO_EPSILON {
                return Err("smoothstep domain error: edges must differ".into());
            }
            let t = ((value - edge0) / span).clamp(0.0, 1.0);
            Value::Float(t * t * (3.0 - 2.0 * t))
        }
        N::IsFinite => Value::Bool(number(&args[0], "is_finite")?.is_finite()),
        N::IsNan => Value::Bool(number(&args[0], "is_nan")?.is_nan()),
        N::IsInfinite => Value::Bool(number(&args[0], "is_infinite")?.is_infinite()),
        N::ApproxEq => {
            let epsilon = args
                .get(2)
                .map(|value| number(value, "approx_eq"))
                .transpose()?
                .unwrap_or(DEFAULT_EPSILON);
            if !epsilon.is_finite() || epsilon < 0.0 {
                return Err("approx_eq expects a finite epsilon >= 0".into());
            }
            Value::Bool(approx_value(&args[0], &args[1], epsilon)?)
        }
        N::Gcd => Value::Int(gcd(integer(&args[0], "gcd")?, integer(&args[1], "gcd")?)),
        N::Lcm => {
            let a = integer(&args[0], "lcm")?;
            let b = integer(&args[1], "lcm")?;
            let divisor = gcd(a, b);
            let result = if divisor == 0 {
                0
            } else {
                (a / divisor)
                    .checked_mul(b)
                    .and_then(i64::checked_abs)
                    .ok_or_else(|| "lcm integer overflow".to_string())?
            };
            Value::Int(result)
        }
        N::Vec2 => construct_vector(args, 2)?,
        N::Vec3 => construct_vector(args, 3)?,
        N::Vec4 => construct_vector(args, 4)?,
        N::Dot => Value::Float(dot_same(&args[0], &args[1], "dot")?),
        N::Cross => cross(&args[0], &args[1])?,
        N::Length => Value::Float(vector_length(&args[0])?),
        N::Normalize => normalize(&args[0])?,
        N::Distance => {
            let difference = vector_zip(&args[0], &args[1], "distance", |a, b| a - b)?;
            Value::Float(dot_slice(&difference, &difference).sqrt())
        }
        N::AngleBetween => {
            let a = vector(&args[0], "angle_between")?;
            let b = vector(&args[1], "angle_between")?;
            require_same_dimension(a, b, "angle_between")?;
            let denominator = dot_slice(a, a).sqrt() * dot_slice(b, b).sqrt();
            if denominator <= ZERO_EPSILON {
                return Err("angle_between domain error: zero-length vector".into());
            }
            Value::Float((dot_slice(a, b) / denominator).clamp(-1.0, 1.0).acos())
        }
        N::Reflect => {
            let v = vector(&args[0], "reflect")?;
            let normal = normalized_slice(vector(&args[1], "reflect")?, "reflect")?;
            require_same_dimension(v, &normal, "reflect")?;
            let scale = 2.0 * dot_slice(v, &normal);
            vector_like(&args[0], v.iter().zip(normal).map(|(a, n)| a - scale * n))?
        }
        N::Project => {
            let v = vector(&args[0], "project")?;
            let onto = vector(&args[1], "project")?;
            require_same_dimension(v, onto, "project")?;
            let denominator = dot_slice(onto, onto);
            if denominator <= ZERO_EPSILON {
                return Err("project domain error: target vector has zero length".into());
            }
            let scale = dot_slice(v, onto) / denominator;
            vector_like(&args[0], onto.iter().map(|value| value * scale))?
        }
        N::Refract => {
            let incident = vector(&args[0], "refract")?;
            let normal = normalized_slice(vector(&args[1], "refract")?, "refract")?;
            require_same_dimension(incident, &normal, "refract")?;
            let eta = number(&args[2], "refract")?;
            if !eta.is_finite() || eta <= 0.0 {
                return Err("refract expects finite eta > 0".into());
            }
            let cosine = dot_slice(incident, &normal);
            let k = 1.0 - eta * eta * (1.0 - cosine * cosine);
            if k < 0.0 {
                return Err("refract domain error: total internal reflection".into());
            }
            vector_like(
                &args[0],
                incident
                    .iter()
                    .zip(normal)
                    .map(|(incident, normal)| eta * incident - (eta * cosine + k.sqrt()) * normal),
            )?
        }
        N::VecLerp => {
            let t = number(&args[2], "vec_lerp")?;
            let values = vector_zip(&args[0], &args[1], "vec_lerp", |a, b| a + (b - a) * t)?;
            vector_like(&args[0], values)?
        }
        N::Mat3Identity => Value::Mat3(identity_matrix::<9>(3)),
        N::Mat4Identity => Value::Mat4(identity_matrix::<16>(4)),
        N::Mat3 => Value::Mat3(array_from_args::<9>(args, "mat3")?),
        N::Mat4 => Value::Mat4(array_from_args::<16>(args, "mat4")?),
        N::Mat3Translation => {
            let value = vec2(&args[0], "mat3_translation")?;
            Value::Mat3([1.0, 0.0, 0.0, 0.0, 1.0, 0.0, value[0], value[1], 1.0])
        }
        N::Mat3Scale => {
            let value = vec2(&args[0], "mat3_scale")?;
            Value::Mat3([value[0], 0.0, 0.0, 0.0, value[1], 0.0, 0.0, 0.0, 1.0])
        }
        N::Mat3Rotation => Value::Mat3(rotation_z3(number(&args[0], "mat3_rotation")?)),
        N::Mat4Translation => {
            let value = vec3(&args[0], "mat4_translation")?;
            let mut matrix = identity_matrix::<16>(4);
            matrix[12] = value[0];
            matrix[13] = value[1];
            matrix[14] = value[2];
            Value::Mat4(matrix)
        }
        N::Mat4Scale => {
            let value = vec3(&args[0], "mat4_scale")?;
            Value::Mat4([
                value[0], 0.0, 0.0, 0.0, 0.0, value[1], 0.0, 0.0, 0.0, 0.0, value[2], 0.0, 0.0,
                0.0, 0.0, 1.0,
            ])
        }
        N::Mat4RotationX => Value::Mat4(rotation_x4(number(&args[0], "mat4_rotation_x")?)),
        N::Mat4RotationY => Value::Mat4(rotation_y4(number(&args[0], "mat4_rotation_y")?)),
        N::Mat4RotationZ => Value::Mat4(rotation_z4(number(&args[0], "mat4_rotation_z")?)),
        N::Mat4Orthographic => Value::Mat4(orthographic(args)?),
        N::Mat4Perspective => Value::Mat4(perspective(args)?),
        N::Mat4LookAt => Value::Mat4(look_at(args)?),
        N::MatMul => matrix_multiply(&args[0], &args[1])?,
        N::MatTranspose => matrix_transpose(&args[0])?,
        N::MatDeterminant => Value::Float(matrix_determinant(&args[0])?),
        N::MatInverse => matrix_inverse(&args[0])?,
        N::TransformPoint => transform(&args[0], &args[1], true)?,
        N::TransformVector => transform(&args[0], &args[1], false)?,
        N::Quat => Value::Quat(array_from_args::<4>(args, "quat")?),
        N::QuatIdentity => Value::Quat([0.0, 0.0, 0.0, 1.0]),
        N::QuatAxisAngle => {
            let axis = match normalize(&args[0])? {
                Value::Vec3(axis) => axis,
                _ => return Err("quat_axis_angle expects vec3 axis".into()),
            };
            let half = number(&args[1], "quat_axis_angle")? * 0.5;
            let sine = half.sin();
            Value::Quat([axis[0] * sine, axis[1] * sine, axis[2] * sine, half.cos()])
        }
        N::QuatMul => Value::Quat(quat_mul(
            quat(&args[0], "quat_mul")?,
            quat(&args[1], "quat_mul")?,
        )),
        N::QuatRotate => {
            let q = normalized_quat(quat(&args[0], "quat_rotate")?, "quat_rotate")?;
            let v = match &args[1] {
                Value::Vec3(value) => *value,
                other => {
                    return Err(format!(
                        "quat_rotate expects vec3, got {}",
                        other.type_name()
                    ))
                }
            };
            Value::Vec3(quat_rotate(q, v))
        }
        N::QuatNormalize => Value::Quat(normalized_quat(
            quat(&args[0], "quat_normalize")?,
            "quat_normalize",
        )?),
        N::QuatInverse => {
            let q = quat(&args[0], "quat_inverse")?;
            let length_squared = dot_slice(q, q);
            if length_squared <= ZERO_EPSILON {
                return Err("quat_inverse domain error: zero-length quaternion".into());
            }
            Value::Quat([
                -q[0] / length_squared,
                -q[1] / length_squared,
                -q[2] / length_squared,
                q[3] / length_squared,
            ])
        }
        N::QuatSlerp => Value::Quat(quat_slerp(
            quat(&args[0], "quat_slerp")?,
            quat(&args[1], "quat_slerp")?,
            number(&args[2], "quat_slerp")?,
        )?),
        N::RngNew => rng_new(integer(&args[0], "rng_new")? as u64),
        N::RngNextFloat => Value::Float(rng_next_f64(rng(&args[0], "rng_next_float")?)),
        N::RngRangeInt => rng_range_int(args)?,
        N::RngRangeFloat => {
            let stream = rng(&args[0], "rng_range_float")?;
            let min = number(&args[1], "rng_range_float")?;
            let max = number(&args[2], "rng_range_float")?;
            if !min.is_finite() || !max.is_finite() || min >= max {
                return Err("rng_range_float expects finite min < max".into());
            }
            Value::Float(min + (max - min) * rng_next_f64(stream))
        }
        N::RngBool => {
            let probability = number(&args[1], "rng_bool")?;
            require_range("rng_bool probability", probability, 0.0, 1.0)?;
            Value::Bool(rng_next_f64(rng(&args[0], "rng_bool")?) < probability)
        }
        N::RngGaussian => {
            let stream = rng(&args[0], "rng_gaussian")?;
            let mean = number(&args[1], "rng_gaussian")?;
            let stddev = number(&args[2], "rng_gaussian")?;
            if !mean.is_finite() || !stddev.is_finite() || stddev < 0.0 {
                return Err("rng_gaussian expects finite mean and stddev >= 0".into());
            }
            let u1 = (1.0 - rng_next_f64(stream)).max(f64::MIN_POSITIVE);
            let u2 = rng_next_f64(stream);
            let standard = (-2.0 * u1.ln()).sqrt() * (std::f64::consts::TAU * u2).cos();
            Value::Float(mean + standard * stddev)
        }
        N::RngExponential => {
            let stream = rng(&args[0], "rng_exponential")?;
            let rate = number(&args[1], "rng_exponential")?;
            if !rate.is_finite() || rate <= 0.0 {
                return Err("rng_exponential expects finite rate > 0".into());
            }
            Value::Float(-(1.0 - rng_next_f64(stream)).ln() / rate)
        }
        N::Shuffle => shuffle(args)?,
        N::Choose => choose(args)?,
        N::WeightedChoose => weighted_choose(args)?,
        N::Noise1 => Value::Float(noise1(
            number(&args[0], "noise1")?,
            integer(&args[1], "noise1")? as u64,
        )),
        N::Noise2 => Value::Float(noise2(
            number(&args[0], "noise2")?,
            number(&args[1], "noise2")?,
            integer(&args[2], "noise2")? as u64,
        )),
        N::Perlin2 => Value::Float(perlin2(
            number(&args[0], "perlin2")?,
            number(&args[1], "perlin2")?,
            integer(&args[2], "perlin2")? as u64,
        )),
        N::Fbm2 => Value::Float(fbm2(args)?),
        N::Turbulence2 => Value::Float(turbulence2(args)?),
        N::DomainWarp2 => Value::Float(domain_warp2(args)?),
        N::Sum => Value::Float(numbers(&args[0], "sum")?.iter().sum()),
        N::Product => Value::Float(numbers(&args[0], "product")?.iter().product()),
        N::Mean => Value::Float(mean(&numbers_non_empty(&args[0], "mean")?)),
        N::Median => Value::Float(median(numbers_non_empty(&args[0], "median")?)),
        N::Mode => Value::Float(mode(numbers_non_empty(&args[0], "mode")?)),
        N::Variance => Value::Float(variance(
            &numbers_non_empty(&args[0], "variance")?,
            optional_bool(args.get(1), "variance")?,
        )?),
        N::Stddev => Value::Float(
            variance(
                &numbers_non_empty(&args[0], "stddev")?,
                optional_bool(args.get(1), "stddev")?,
            )?
            .sqrt(),
        ),
        N::Quantile => Value::Float(quantile(
            numbers_non_empty(&args[0], "quantile")?,
            number(&args[1], "quantile")?,
        )?),
        N::Covariance => Value::Float(covariance(
            &numbers_non_empty(&args[0], "covariance")?,
            &numbers_non_empty(&args[1], "covariance")?,
            optional_bool(args.get(2), "covariance")?,
        )?),
        N::Correlation => Value::Float(correlation(
            &numbers_non_empty(&args[0], "correlation")?,
            &numbers_non_empty(&args[1], "correlation")?,
        )?),
        N::MovingAverage => moving_average(args)?,
        N::Histogram => histogram(args)?,
        N::Ema => ema(args)?,
        N::IsEven => Value::Bool(integer(&args[0], "is_even")? % 2 == 0),
        N::IsOdd => Value::Bool(integer(&args[0], "is_odd")? % 2 != 0),
        N::PowInt => Value::Int(pow_int(
            integer(&args[0], "pow_int")?,
            integer(&args[1], "pow_int")?,
        )?),
        N::VecMin => {
            let values = vector_zip(&args[0], &args[1], "vec_min", f64::min)?;
            vector_like(&args[0], values)?
        }
        N::VecMax => {
            let values = vector_zip(&args[0], &args[1], "vec_max", f64::max)?;
            vector_like(&args[0], values)?
        }
        N::ClampLength => clamp_length(args)?,
        N::PolyEval => Value::Float(poly_eval(
            &numbers_non_empty(&args[0], "poly_eval")?,
            number(&args[1], "poly_eval")?,
        )),
        N::IntegrateTrapezoid => Value::Float(integrate_trapezoid(args)?),
        N::IntegrateSimpson => Value::Float(integrate_simpson(args)?),
        N::PolyRootBisection => Value::Float(poly_root_bisection(args)?),
        N::QuadraticBezier => curve(args, CurveKind::Quadratic)?,
        N::CubicBezier => curve(args, CurveKind::Cubic)?,
        N::CatmullRom => curve(args, CurveKind::CatmullRom)?,
        N::Hermite => curve(args, CurveKind::Hermite)?,
        N::QuatEuler => Value::Quat(quat_euler(args)?),
        N::ClosestPointSegment => closest_point_segment(args)?,
        N::SegmentIntersection2 => segment_intersection2(args)?,
        N::RayPlaneIntersection => ray_plane_intersection(args)?,
        _ => {
            return Err(format!(
                "native `{}` is not an advanced math native",
                native.name()
            ))
        }
    };

    Ok(NativeOutput {
        value,
        printed: None,
    })
}

/// Variable instruction charge for collection- and iteration-sized math work.
pub(crate) fn dynamic_cost(native: NativeId, args: &[Value]) -> u64 {
    use NativeId as N;
    let list_len = |index: usize| args.get(index).and_then(Value::len).unwrap_or_default() as u64;
    match native {
        N::Shuffle
        | N::WeightedChoose
        | N::Sum
        | N::Product
        | N::Mean
        | N::Median
        | N::Mode
        | N::Variance
        | N::Stddev
        | N::Quantile
        | N::MovingAverage
        | N::Histogram
        | N::Ema
        | N::PolyEval
        | N::IntegrateTrapezoid
        | N::IntegrateSimpson
        | N::PolyRootBisection => list_len(if matches!(native, N::Shuffle | N::WeightedChoose) {
            1
        } else {
            0
        }),
        N::Covariance | N::Correlation => list_len(0).saturating_add(list_len(1)),
        N::Fbm2 | N::Turbulence2 | N::DomainWarp2 => {
            args.get(3)
                .and_then(Value::as_i64)
                .unwrap_or_default()
                .clamp(0, 16) as u64
                * 4
        }
        _ => 0,
    }
}

fn validate_arity(native: NativeId, args: &[Value]) -> Result<(), String> {
    let spec = native.spec();
    if args.len() < spec.min_args as usize || args.len() > spec.max_args as usize {
        let expected = if spec.min_args == spec.max_args {
            spec.min_args.to_string()
        } else {
            format!("{}..={}", spec.min_args, spec.max_args)
        };
        return Err(format!(
            "{} expected {expected} argument(s), got {}",
            spec.name,
            args.len()
        ));
    }
    Ok(())
}

fn number(value: &Value, name: &str) -> Result<f64, String> {
    value
        .as_f64()
        .ok_or_else(|| format!("{name} expects a number, got {}", value.type_name()))
}

fn integer(value: &Value, name: &str) -> Result<i64, String> {
    match value {
        Value::Int(value) => Ok(*value),
        _ => Err(format!("{name} expects an int, got {}", value.type_name())),
    }
}

fn float_result(name: &str, value: f64) -> Result<Value, String> {
    if value.is_finite() {
        Ok(Value::Float(value))
    } else {
        Err(format!("{name} produced a non-finite result"))
    }
}

fn require_range(name: &str, value: f64, min: f64, max: f64) -> Result<(), String> {
    if !value.is_finite() || value < min || value > max {
        Err(format!(
            "{name} domain error: expected {min} <= x <= {max}, received {value}"
        ))
    } else {
        Ok(())
    }
}

fn gcd(a: i64, b: i64) -> i64 {
    let mut a = a.unsigned_abs();
    let mut b = b.unsigned_abs();
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a.min(i64::MAX as u64) as i64
}

fn construct_vector(args: &[Value], dimension: usize) -> Result<Value, String> {
    let mut values = vec![0.0; dimension];
    if args.len() == 1 {
        values.fill(number(&args[0], "vector constructor")?);
    } else {
        for (slot, argument) in values.iter_mut().zip(args) {
            *slot = number(argument, "vector constructor")?;
        }
    }
    match values.as_slice() {
        [x, y] => Ok(Value::Vec2([*x, *y])),
        [x, y, z] => Ok(Value::Vec3([*x, *y, *z])),
        [x, y, z, w] => Ok(Value::Vec4([*x, *y, *z, *w])),
        _ => unreachable!(),
    }
}

fn vector<'a>(value: &'a Value, name: &str) -> Result<&'a [f64], String> {
    match value {
        Value::Vec2(values) => Ok(values),
        Value::Vec3(values) => Ok(values),
        Value::Vec4(values) => Ok(values),
        _ => Err(format!(
            "{name} expects a vector, got {}",
            value.type_name()
        )),
    }
}

fn vec2<'a>(value: &'a Value, name: &str) -> Result<&'a [f64; 2], String> {
    match value {
        Value::Vec2(value) => Ok(value),
        _ => Err(format!("{name} expects vec2, got {}", value.type_name())),
    }
}

fn vec3<'a>(value: &'a Value, name: &str) -> Result<&'a [f64; 3], String> {
    match value {
        Value::Vec3(value) => Ok(value),
        _ => Err(format!("{name} expects vec3, got {}", value.type_name())),
    }
}

fn vector_like(template: &Value, values: impl IntoIterator<Item = f64>) -> Result<Value, String> {
    let values = values.into_iter().collect::<Vec<_>>();
    match (template, values.as_slice()) {
        (Value::Vec2(_), [x, y]) => Ok(Value::Vec2([*x, *y])),
        (Value::Vec3(_), [x, y, z]) => Ok(Value::Vec3([*x, *y, *z])),
        (Value::Vec4(_), [x, y, z, w]) => Ok(Value::Vec4([*x, *y, *z, *w])),
        _ => Err("vector dimension mismatch".into()),
    }
}

fn require_same_dimension(a: &[f64], b: &[f64], name: &str) -> Result<(), String> {
    if a.len() == b.len() {
        Ok(())
    } else {
        Err(format!(
            "{name} vector dimension mismatch: vec{} and vec{}",
            a.len(),
            b.len()
        ))
    }
}

fn dot_slice(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b).map(|(a, b)| a * b).sum()
}

fn dot_same(a: &Value, b: &Value, name: &str) -> Result<f64, String> {
    let a = vector(a, name)?;
    let b = vector(b, name)?;
    require_same_dimension(a, b, name)?;
    Ok(dot_slice(a, b))
}

fn vector_zip(
    a: &Value,
    b: &Value,
    name: &str,
    operation: impl Fn(f64, f64) -> f64,
) -> Result<Vec<f64>, String> {
    let a = vector(a, name)?;
    let b = vector(b, name)?;
    require_same_dimension(a, b, name)?;
    Ok(a.iter().zip(b).map(|(a, b)| operation(*a, *b)).collect())
}

fn vector_length(value: &Value) -> Result<f64, String> {
    let values = vector(value, "length")?;
    Ok(dot_slice(values, values).sqrt())
}

fn normalized_slice(values: &[f64], name: &str) -> Result<Vec<f64>, String> {
    let length = dot_slice(values, values).sqrt();
    if length <= ZERO_EPSILON {
        return Err(format!("{name} domain error: zero-length value"));
    }
    Ok(values.iter().map(|value| value / length).collect())
}

fn normalize(value: &Value) -> Result<Value, String> {
    match value {
        Value::Quat(values) => Ok(Value::Quat(normalized_quat(values, "normalize")?)),
        _ => vector_like(
            value,
            normalized_slice(vector(value, "normalize")?, "normalize")?,
        ),
    }
}

fn cross(a: &Value, b: &Value) -> Result<Value, String> {
    match (a, b) {
        (Value::Vec2(a), Value::Vec2(b)) => Ok(Value::Float(a[0] * b[1] - a[1] * b[0])),
        (Value::Vec3(a), Value::Vec3(b)) => Ok(Value::Vec3([
            a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0],
        ])),
        _ => Err(format!(
            "cross expects two vec2 or two vec3 values, got {} and {}",
            a.type_name(),
            b.type_name()
        )),
    }
}

fn array_from_args<const N: usize>(args: &[Value], name: &str) -> Result<[f64; N], String> {
    let mut values = [0.0; N];
    for (slot, argument) in values.iter_mut().zip(args) {
        *slot = number(argument, name)?;
    }
    Ok(values)
}

fn identity_matrix<const N: usize>(dimension: usize) -> [f64; N] {
    let mut values = [0.0; N];
    for index in 0..dimension {
        values[index * dimension + index] = 1.0;
    }
    values
}

fn rotation_z3(angle: f64) -> [f64; 9] {
    let (sine, cosine) = angle.sin_cos();
    [cosine, sine, 0.0, -sine, cosine, 0.0, 0.0, 0.0, 1.0]
}

fn rotation_x4(angle: f64) -> [f64; 16] {
    let (sine, cosine) = angle.sin_cos();
    [
        1.0, 0.0, 0.0, 0.0, 0.0, cosine, sine, 0.0, 0.0, -sine, cosine, 0.0, 0.0, 0.0, 0.0, 1.0,
    ]
}

fn rotation_y4(angle: f64) -> [f64; 16] {
    let (sine, cosine) = angle.sin_cos();
    [
        cosine, 0.0, -sine, 0.0, 0.0, 1.0, 0.0, 0.0, sine, 0.0, cosine, 0.0, 0.0, 0.0, 0.0, 1.0,
    ]
}

fn rotation_z4(angle: f64) -> [f64; 16] {
    let (sine, cosine) = angle.sin_cos();
    [
        cosine, sine, 0.0, 0.0, -sine, cosine, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
    ]
}

fn orthographic(args: &[Value]) -> Result<[f64; 16], String> {
    let left = number(&args[0], "mat4_orthographic")?;
    let right = number(&args[1], "mat4_orthographic")?;
    let bottom = number(&args[2], "mat4_orthographic")?;
    let top = number(&args[3], "mat4_orthographic")?;
    let near = number(&args[4], "mat4_orthographic")?;
    let far = number(&args[5], "mat4_orthographic")?;
    if (right - left).abs() <= ZERO_EPSILON
        || (top - bottom).abs() <= ZERO_EPSILON
        || (far - near).abs() <= ZERO_EPSILON
    {
        return Err("mat4_orthographic expects distinct bounds".into());
    }
    Ok([
        2.0 / (right - left),
        0.0,
        0.0,
        0.0,
        0.0,
        2.0 / (top - bottom),
        0.0,
        0.0,
        0.0,
        0.0,
        -2.0 / (far - near),
        0.0,
        -(right + left) / (right - left),
        -(top + bottom) / (top - bottom),
        -(far + near) / (far - near),
        1.0,
    ])
}

fn perspective(args: &[Value]) -> Result<[f64; 16], String> {
    let fov = number(&args[0], "mat4_perspective")?;
    let aspect = number(&args[1], "mat4_perspective")?;
    let near = number(&args[2], "mat4_perspective")?;
    let far = number(&args[3], "mat4_perspective")?;
    if !fov.is_finite()
        || fov <= 0.0
        || fov >= std::f64::consts::PI
        || !aspect.is_finite()
        || aspect <= 0.0
        || !near.is_finite()
        || !far.is_finite()
        || near <= 0.0
        || near >= far
    {
        return Err("mat4_perspective expects 0 < fov < PI, aspect > 0, and 0 < near < far".into());
    }
    let scale = 1.0 / (fov * 0.5).tan();
    Ok([
        scale / aspect,
        0.0,
        0.0,
        0.0,
        0.0,
        scale,
        0.0,
        0.0,
        0.0,
        0.0,
        (far + near) / (near - far),
        -1.0,
        0.0,
        0.0,
        (2.0 * far * near) / (near - far),
        0.0,
    ])
}

fn look_at(args: &[Value]) -> Result<[f64; 16], String> {
    let eye = vec3(&args[0], "mat4_look_at")?;
    let target = vec3(&args[1], "mat4_look_at")?;
    let up = vec3(&args[2], "mat4_look_at")?;
    let forward = normalized_slice(
        &[eye[0] - target[0], eye[1] - target[1], eye[2] - target[2]],
        "mat4_look_at",
    )?;
    let right = normalized_slice(
        &[
            up[1] * forward[2] - up[2] * forward[1],
            up[2] * forward[0] - up[0] * forward[2],
            up[0] * forward[1] - up[1] * forward[0],
        ],
        "mat4_look_at",
    )?;
    let corrected_up = [
        forward[1] * right[2] - forward[2] * right[1],
        forward[2] * right[0] - forward[0] * right[2],
        forward[0] * right[1] - forward[1] * right[0],
    ];
    Ok([
        right[0],
        corrected_up[0],
        forward[0],
        0.0,
        right[1],
        corrected_up[1],
        forward[1],
        0.0,
        right[2],
        corrected_up[2],
        forward[2],
        0.0,
        -dot_slice(&right, eye),
        -dot_slice(&corrected_up, eye),
        -dot_slice(&forward, eye),
        1.0,
    ])
}

fn matrix(value: &Value) -> Result<(&[f64], usize), String> {
    match value {
        Value::Mat3(values) => Ok((values, 3)),
        Value::Mat4(values) => Ok((values, 4)),
        _ => Err(format!("expected matrix, got {}", value.type_name())),
    }
}

fn matrix_value(dimension: usize, values: Vec<f64>) -> Result<Value, String> {
    match (dimension, values.as_slice()) {
        (3, values) => Ok(Value::Mat3(values.try_into().map_err(|_| "invalid mat3")?)),
        (4, values) => Ok(Value::Mat4(values.try_into().map_err(|_| "invalid mat4")?)),
        _ => Err("unsupported matrix dimension".into()),
    }
}

fn matrix_multiply(a: &Value, b: &Value) -> Result<Value, String> {
    let (a, dimension) = matrix(a)?;
    let (b, other_dimension) = matrix(b)?;
    if dimension != other_dimension {
        return Err(format!(
            "mat_mul dimension mismatch: mat{dimension} and mat{other_dimension}"
        ));
    }
    let mut out = vec![0.0; dimension * dimension];
    for column in 0..dimension {
        for row in 0..dimension {
            out[column * dimension + row] = (0..dimension)
                .map(|k| a[k * dimension + row] * b[column * dimension + k])
                .sum();
        }
    }
    matrix_value(dimension, out)
}

fn matrix_transpose(value: &Value) -> Result<Value, String> {
    let (matrix, dimension) = matrix(value)?;
    let mut out = vec![0.0; matrix.len()];
    for column in 0..dimension {
        for row in 0..dimension {
            out[column * dimension + row] = matrix[row * dimension + column];
        }
    }
    matrix_value(dimension, out)
}

fn matrix_determinant(value: &Value) -> Result<f64, String> {
    let (matrix, dimension) = matrix(value)?;
    let mut rows = to_rows(matrix, dimension);
    let mut determinant = 1.0;
    for pivot in 0..dimension {
        let best = (pivot..dimension)
            .max_by(|a, b| rows[*a][pivot].abs().total_cmp(&rows[*b][pivot].abs()))
            .unwrap();
        if rows[best][pivot].abs() <= ZERO_EPSILON {
            return Ok(0.0);
        }
        if best != pivot {
            rows.swap(best, pivot);
            determinant = -determinant;
        }
        let pivot_value = rows[pivot][pivot];
        determinant *= pivot_value;
        let pivot_row = rows[pivot].clone();
        for row in rows.iter_mut().skip(pivot + 1) {
            let factor = row[pivot] / pivot_value;
            for (value, pivot_value) in row[(pivot + 1)..].iter_mut().zip(&pivot_row[(pivot + 1)..])
            {
                *value -= factor * pivot_value;
            }
        }
    }
    Ok(determinant)
}

fn matrix_inverse(value: &Value) -> Result<Value, String> {
    let (matrix, dimension) = matrix(value)?;
    let mut rows = to_rows(matrix, dimension);
    for (row, values) in rows.iter_mut().enumerate() {
        values.extend((0..dimension).map(|column| f64::from(row == column)));
    }
    for pivot in 0..dimension {
        let best = (pivot..dimension)
            .max_by(|a, b| rows[*a][pivot].abs().total_cmp(&rows[*b][pivot].abs()))
            .unwrap();
        if rows[best][pivot].abs() <= ZERO_EPSILON {
            return Err("mat_inverse domain error: matrix is singular".into());
        }
        rows.swap(best, pivot);
        let pivot_value = rows[pivot][pivot];
        for value in &mut rows[pivot] {
            *value /= pivot_value;
        }
        let pivot_row = rows[pivot].clone();
        for (row_index, row) in rows.iter_mut().enumerate().take(dimension) {
            if row_index == pivot {
                continue;
            }
            let factor = row[pivot];
            for (value, pivot_value) in row.iter_mut().zip(&pivot_row) {
                *value -= factor * pivot_value;
            }
        }
    }
    let mut out = vec![0.0; dimension * dimension];
    for row in 0..dimension {
        for column in 0..dimension {
            out[column * dimension + row] = rows[row][dimension + column];
        }
    }
    matrix_value(dimension, out)
}

fn to_rows(matrix: &[f64], dimension: usize) -> Vec<Vec<f64>> {
    (0..dimension)
        .map(|row| {
            (0..dimension)
                .map(|column| matrix[column * dimension + row])
                .collect()
        })
        .collect()
}

fn transform(matrix_value: &Value, vector_value: &Value, point: bool) -> Result<Value, String> {
    match (matrix_value, vector_value) {
        (Value::Mat3(matrix), Value::Vec2(vector)) => {
            let w = f64::from(point);
            let x = matrix[0] * vector[0] + matrix[3] * vector[1] + matrix[6] * w;
            let y = matrix[1] * vector[0] + matrix[4] * vector[1] + matrix[7] * w;
            let output_w = matrix[2] * vector[0] + matrix[5] * vector[1] + matrix[8] * w;
            if point && output_w.abs() > ZERO_EPSILON && (output_w - 1.0).abs() > ZERO_EPSILON {
                Ok(Value::Vec2([x / output_w, y / output_w]))
            } else {
                Ok(Value::Vec2([x, y]))
            }
        }
        (Value::Mat4(matrix), Value::Vec3(vector)) => {
            let w = f64::from(point);
            let x = matrix[0] * vector[0]
                + matrix[4] * vector[1]
                + matrix[8] * vector[2]
                + matrix[12] * w;
            let y = matrix[1] * vector[0]
                + matrix[5] * vector[1]
                + matrix[9] * vector[2]
                + matrix[13] * w;
            let z = matrix[2] * vector[0]
                + matrix[6] * vector[1]
                + matrix[10] * vector[2]
                + matrix[14] * w;
            let output_w = matrix[3] * vector[0]
                + matrix[7] * vector[1]
                + matrix[11] * vector[2]
                + matrix[15] * w;
            if point && output_w.abs() > ZERO_EPSILON && (output_w - 1.0).abs() > ZERO_EPSILON {
                Ok(Value::Vec3([x / output_w, y / output_w, z / output_w]))
            } else {
                Ok(Value::Vec3([x, y, z]))
            }
        }
        _ => Err(format!(
            "{} expects mat3+vec2 or mat4+vec3, got {}+{}",
            if point {
                "transform_point"
            } else {
                "transform_vector"
            },
            matrix_value.type_name(),
            vector_value.type_name()
        )),
    }
}

fn quat<'a>(value: &'a Value, name: &str) -> Result<&'a [f64; 4], String> {
    match value {
        Value::Quat(value) => Ok(value),
        _ => Err(format!("{name} expects quat, got {}", value.type_name())),
    }
}

fn normalized_quat(value: &[f64; 4], name: &str) -> Result<[f64; 4], String> {
    let normalized = normalized_slice(value, name)?;
    Ok([normalized[0], normalized[1], normalized[2], normalized[3]])
}

fn quat_mul(a: &[f64; 4], b: &[f64; 4]) -> [f64; 4] {
    [
        a[3] * b[0] + a[0] * b[3] + a[1] * b[2] - a[2] * b[1],
        a[3] * b[1] - a[0] * b[2] + a[1] * b[3] + a[2] * b[0],
        a[3] * b[2] + a[0] * b[1] - a[1] * b[0] + a[2] * b[3],
        a[3] * b[3] - a[0] * b[0] - a[1] * b[1] - a[2] * b[2],
    ]
}

fn quat_rotate(q: [f64; 4], v: [f64; 3]) -> [f64; 3] {
    let qv = [q[0], q[1], q[2]];
    let uv = [
        qv[1] * v[2] - qv[2] * v[1],
        qv[2] * v[0] - qv[0] * v[2],
        qv[0] * v[1] - qv[1] * v[0],
    ];
    let uuv = [
        qv[1] * uv[2] - qv[2] * uv[1],
        qv[2] * uv[0] - qv[0] * uv[2],
        qv[0] * uv[1] - qv[1] * uv[0],
    ];
    [
        v[0] + 2.0 * (q[3] * uv[0] + uuv[0]),
        v[1] + 2.0 * (q[3] * uv[1] + uuv[1]),
        v[2] + 2.0 * (q[3] * uv[2] + uuv[2]),
    ]
}

fn quat_slerp(a: &[f64; 4], b: &[f64; 4], t: f64) -> Result<[f64; 4], String> {
    let a = normalized_quat(a, "quat_slerp")?;
    let mut b = normalized_quat(b, "quat_slerp")?;
    let mut cosine = dot_slice(&a, &b);
    if cosine < 0.0 {
        for value in &mut b {
            *value = -*value;
        }
        cosine = -cosine;
    }
    if cosine > 0.9995 {
        let mut result = [0.0; 4];
        for index in 0..4 {
            result[index] = a[index] + (b[index] - a[index]) * t;
        }
        return normalized_quat(&result, "quat_slerp");
    }
    let angle = cosine.clamp(-1.0, 1.0).acos();
    let sine = angle.sin();
    let a_weight = ((1.0 - t) * angle).sin() / sine;
    let b_weight = (t * angle).sin() / sine;
    Ok([
        a[0] * a_weight + b[0] * b_weight,
        a[1] * a_weight + b[1] * b_weight,
        a[2] * a_weight + b[2] * b_weight,
        a[3] * a_weight + b[3] * b_weight,
    ])
}

fn quat_euler(args: &[Value]) -> Result<[f64; 4], String> {
    let half_x = number(&args[0], "quat_euler")? * 0.5;
    let half_y = number(&args[1], "quat_euler")? * 0.5;
    let half_z = number(&args[2], "quat_euler")? * 0.5;
    let (sx, cx) = half_x.sin_cos();
    let (sy, cy) = half_y.sin_cos();
    let (sz, cz) = half_z.sin_cos();
    Ok([
        sx * cy * cz - cx * sy * sz,
        cx * sy * cz + sx * cy * sz,
        cx * cy * sz - sx * sy * cz,
        cx * cy * cz + sx * sy * sz,
    ])
}

fn rng_new(seed: u64) -> Value {
    let stream = Rc::new(RefCell::new([0, 1_442_695_040_888_963_407]));
    let _ = rng_next_u32(&stream);
    let state = stream.borrow()[0];
    stream.borrow_mut()[0] = state.wrapping_add(seed);
    let _ = rng_next_u32(&stream);
    Value::Rng(stream)
}

/// Handle an arithmetic operation involving a first-class math value.
pub(crate) fn binary_math(op: Op, a: &Value, b: &Value) -> Option<Result<Value, String>> {
    let componentwise = |a: &[f64], b: &[f64], template: &Value| {
        if a.len() != b.len() {
            return Err("vector dimension mismatch".into());
        }
        vector_like(
            template,
            a.iter()
                .zip(b)
                .map(|(a, b)| match op {
                    Op::Add => Ok(a + b),
                    Op::Sub => Ok(a - b),
                    Op::Mul => Ok(a * b),
                    Op::Div if *b != 0.0 => Ok(a / b),
                    Op::Div => Err("division by zero".to_string()),
                    _ => Err("unsupported vector operator".to_string()),
                })
                .collect::<Result<Vec<_>, _>>()?,
        )
    };
    match (a, b) {
        (Value::Vec2(av), Value::Vec2(bv)) => Some(componentwise(av, bv, a)),
        (Value::Vec3(av), Value::Vec3(bv)) => Some(componentwise(av, bv, a)),
        (Value::Vec4(av), Value::Vec4(bv)) => Some(componentwise(av, bv, a)),
        (Value::Vec2(_) | Value::Vec3(_) | Value::Vec4(_), Value::Int(_) | Value::Float(_))
            if matches!(op, Op::Mul | Op::Div) =>
        {
            let scalar = b.as_f64().unwrap();
            if op == Op::Div && scalar == 0.0 {
                return Some(Err("division by zero".into()));
            }
            let values = vector(a, "vector arithmetic").unwrap();
            Some(vector_like(
                a,
                values.iter().map(|value| {
                    if op == Op::Mul {
                        value * scalar
                    } else {
                        value / scalar
                    }
                }),
            ))
        }
        (Value::Int(_) | Value::Float(_), Value::Vec2(_) | Value::Vec3(_) | Value::Vec4(_))
            if op == Op::Mul =>
        {
            let scalar = a.as_f64().unwrap();
            let values = vector(b, "vector arithmetic").unwrap();
            Some(vector_like(b, values.iter().map(|value| scalar * value)))
        }
        (Value::Mat3(_) | Value::Mat4(_), Value::Mat3(_) | Value::Mat4(_)) if op == Op::Mul => {
            Some(matrix_multiply(a, b))
        }
        (Value::Quat(a), Value::Quat(b)) if op == Op::Mul => Some(Ok(Value::Quat(quat_mul(a, b)))),
        _ => None,
    }
}

/// Negate a first-class vector or quaternion when supported.
pub(crate) fn negate_math(value: &Value) -> Option<Value> {
    match value {
        Value::Vec2(v) => Some(Value::Vec2([-v[0], -v[1]])),
        Value::Vec3(v) => Some(Value::Vec3([-v[0], -v[1], -v[2]])),
        Value::Vec4(v) => Some(Value::Vec4([-v[0], -v[1], -v[2], -v[3]])),
        Value::Quat(v) => Some(Value::Quat([-v[0], -v[1], -v[2], -v[3]])),
        _ => None,
    }
}

fn rng<'a>(value: &'a Value, name: &str) -> Result<&'a Rc<RefCell<[u64; 2]>>, String> {
    match value {
        Value::Rng(stream) => Ok(stream),
        _ => Err(format!("{name} expects rng, got {}", value.type_name())),
    }
}

fn rng_next_u32(stream: &Rc<RefCell<[u64; 2]>>) -> u32 {
    let mut state = stream.borrow_mut();
    let old = state[0];
    state[0] = old
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(state[1] | 1);
    let xorshifted = (((old >> 18) ^ old) >> 27) as u32;
    let rotation = (old >> 59) as u32;
    xorshifted.rotate_right(rotation)
}

fn rng_next_u64(stream: &Rc<RefCell<[u64; 2]>>) -> u64 {
    ((rng_next_u32(stream) as u64) << 32) | rng_next_u32(stream) as u64
}

fn rng_next_f64(stream: &Rc<RefCell<[u64; 2]>>) -> f64 {
    let bits = rng_next_u64(stream) >> 11;
    bits as f64 * (1.0 / ((1_u64 << 53) as f64))
}

fn rng_range_int(args: &[Value]) -> Result<Value, String> {
    let stream = rng(&args[0], "rng_range_int")?;
    let min = integer(&args[1], "rng_range_int")?;
    let max = integer(&args[2], "rng_range_int")?;
    if min >= max {
        return Err("rng_range_int expects min < max".into());
    }
    let span = (max as i128 - min as i128) as u64;
    let threshold = span.wrapping_neg() % span;
    let random = loop {
        let candidate = rng_next_u64(stream);
        if candidate >= threshold {
            break candidate % span;
        }
    };
    Ok(Value::Int((min as i128 + random as i128) as i64))
}

fn shuffle(args: &[Value]) -> Result<Value, String> {
    let stream = rng(&args[0], "shuffle")?;
    let Value::List(items) = &args[1] else {
        return Err(format!("shuffle expects list, got {}", args[1].type_name()));
    };
    let mut items = items.borrow_mut();
    for index in (1..items.len()).rev() {
        let target = (rng_next_u64(stream) % (index as u64 + 1)) as usize;
        items.swap(index, target);
    }
    drop(items);
    Ok(args[1].clone())
}

fn choose(args: &[Value]) -> Result<Value, String> {
    let stream = rng(&args[0], "choose")?;
    let Value::List(items) = &args[1] else {
        return Err(format!("choose expects list, got {}", args[1].type_name()));
    };
    let items = items.borrow();
    if items.is_empty() {
        Ok(Value::Null)
    } else {
        Ok(items[(rng_next_u64(stream) % items.len() as u64) as usize].clone())
    }
}

fn weighted_choose(args: &[Value]) -> Result<Value, String> {
    let stream = rng(&args[0], "weighted_choose")?;
    let weights = numbers_non_empty(&args[1], "weighted_choose")?;
    if weights
        .iter()
        .any(|weight| !weight.is_finite() || *weight < 0.0)
    {
        return Err("weighted_choose expects finite non-negative weights".into());
    }
    let total: f64 = weights.iter().sum();
    if total <= 0.0 {
        return Err("weighted_choose expects at least one positive weight".into());
    }
    let mut target = rng_next_f64(stream) * total;
    for (index, weight) in weights.iter().enumerate() {
        target -= weight;
        if target < 0.0 {
            return Ok(Value::Int(index as i64));
        }
    }
    Ok(Value::Int((weights.len() - 1) as i64))
}

fn mix64(mut value: u64) -> u64 {
    value ^= value >> 30;
    value = value.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

fn lattice(seed: u64, x: i64, y: i64) -> f64 {
    let hash = mix64(
        seed ^ (x as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15)
            ^ (y as u64).wrapping_mul(0xd1b5_4a32_d192_ed03),
    );
    ((hash >> 11) as f64 * (1.0 / ((1_u64 << 53) as f64))) * 2.0 - 1.0
}

fn fade(value: f64) -> f64 {
    value * value * value * (value * (value * 6.0 - 15.0) + 10.0)
}

fn noise1(x: f64, seed: u64) -> f64 {
    let x0 = x.floor() as i64;
    let t = fade(x - x.floor());
    lattice(seed, x0, 0) + (lattice(seed, x0 + 1, 0) - lattice(seed, x0, 0)) * t
}

fn noise2(x: f64, y: f64, seed: u64) -> f64 {
    let x0 = x.floor() as i64;
    let y0 = y.floor() as i64;
    let tx = fade(x - x.floor());
    let ty = fade(y - y.floor());
    let a = lattice(seed, x0, y0);
    let b = lattice(seed, x0 + 1, y0);
    let c = lattice(seed, x0, y0 + 1);
    let d = lattice(seed, x0 + 1, y0 + 1);
    let lower = a + (b - a) * tx;
    let upper = c + (d - c) * tx;
    lower + (upper - lower) * ty
}

fn gradient(seed: u64, x: i64, y: i64) -> [f64; 2] {
    const GRADIENTS: [[f64; 2]; 8] = [
        [1.0, 0.0],
        [-1.0, 0.0],
        [0.0, 1.0],
        [0.0, -1.0],
        [
            std::f64::consts::FRAC_1_SQRT_2,
            std::f64::consts::FRAC_1_SQRT_2,
        ],
        [
            -std::f64::consts::FRAC_1_SQRT_2,
            std::f64::consts::FRAC_1_SQRT_2,
        ],
        [
            std::f64::consts::FRAC_1_SQRT_2,
            -std::f64::consts::FRAC_1_SQRT_2,
        ],
        [
            -std::f64::consts::FRAC_1_SQRT_2,
            -std::f64::consts::FRAC_1_SQRT_2,
        ],
    ];
    let hash = mix64(
        seed ^ (x as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15)
            ^ (y as u64).wrapping_mul(0xd1b5_4a32_d192_ed03),
    );
    GRADIENTS[(hash as usize) & 7]
}

fn perlin2(x: f64, y: f64, seed: u64) -> f64 {
    let x0 = x.floor() as i64;
    let y0 = y.floor() as i64;
    let local_x = x - x.floor();
    let local_y = y - y.floor();
    let contribution = |ix: i64, iy: i64, dx: f64, dy: f64| {
        let gradient = gradient(seed, ix, iy);
        gradient[0] * dx + gradient[1] * dy
    };
    let a = contribution(x0, y0, local_x, local_y);
    let b = contribution(x0 + 1, y0, local_x - 1.0, local_y);
    let c = contribution(x0, y0 + 1, local_x, local_y - 1.0);
    let d = contribution(x0 + 1, y0 + 1, local_x - 1.0, local_y - 1.0);
    let tx = fade(local_x);
    let ty = fade(local_y);
    let lower = a + (b - a) * tx;
    let upper = c + (d - c) * tx;
    (lower + (upper - lower) * ty) * std::f64::consts::SQRT_2
}

fn fbm2(args: &[Value]) -> Result<f64, String> {
    let mut x = number(&args[0], "fbm2")?;
    let mut y = number(&args[1], "fbm2")?;
    let seed = integer(&args[2], "fbm2")? as u64;
    let octaves = integer(&args[3], "fbm2")?;
    if !(1..=16).contains(&octaves) {
        return Err("fbm2 expects octaves in 1..=16".into());
    }
    let lacunarity = args
        .get(4)
        .map(|value| number(value, "fbm2"))
        .transpose()?
        .unwrap_or(2.0);
    let gain = args
        .get(5)
        .map(|value| number(value, "fbm2"))
        .transpose()?
        .unwrap_or(0.5);
    if !lacunarity.is_finite() || lacunarity <= 0.0 || !gain.is_finite() || gain < 0.0 {
        return Err("fbm2 expects lacunarity > 0 and gain >= 0".into());
    }
    let mut amplitude = 1.0;
    let mut total = 0.0;
    let mut amplitude_sum = 0.0;
    for octave in 0..octaves {
        total += noise2(x, y, seed.wrapping_add(octave as u64)) * amplitude;
        amplitude_sum += amplitude;
        amplitude *= gain;
        x *= lacunarity;
        y *= lacunarity;
    }
    Ok(if amplitude_sum > 0.0 {
        total / amplitude_sum
    } else {
        0.0
    })
}

fn turbulence2(args: &[Value]) -> Result<f64, String> {
    let mut x = number(&args[0], "turbulence2")?;
    let mut y = number(&args[1], "turbulence2")?;
    let seed = integer(&args[2], "turbulence2")? as u64;
    let octaves = integer(&args[3], "turbulence2")?;
    if !(1..=16).contains(&octaves) {
        return Err("turbulence2 expects octaves in 1..=16".into());
    }
    let lacunarity = args
        .get(4)
        .map(|value| number(value, "turbulence2"))
        .transpose()?
        .unwrap_or(2.0);
    let gain = args
        .get(5)
        .map(|value| number(value, "turbulence2"))
        .transpose()?
        .unwrap_or(0.5);
    if lacunarity <= 0.0 || gain < 0.0 || !lacunarity.is_finite() || !gain.is_finite() {
        return Err("turbulence2 expects lacunarity > 0 and gain >= 0".into());
    }
    let mut amplitude = 1.0;
    let mut total = 0.0;
    let mut amplitude_sum = 0.0;
    for octave in 0..octaves {
        total += perlin2(x, y, seed.wrapping_add(octave as u64)).abs() * amplitude;
        amplitude_sum += amplitude;
        amplitude *= gain;
        x *= lacunarity;
        y *= lacunarity;
    }
    Ok(total / amplitude_sum.max(f64::MIN_POSITIVE))
}

fn domain_warp2(args: &[Value]) -> Result<f64, String> {
    let x = number(&args[0], "domain_warp2")?;
    let y = number(&args[1], "domain_warp2")?;
    let seed = integer(&args[2], "domain_warp2")? as u64;
    let octaves = integer(&args[3], "domain_warp2")?;
    let strength = number(&args[4], "domain_warp2")?;
    if !(1..=16).contains(&octaves) || !strength.is_finite() {
        return Err("domain_warp2 expects octaves in 1..=16 and finite strength".into());
    }
    let mut amplitude = 1.0;
    let mut frequency = 1.0;
    let mut warp_x = 0.0;
    let mut warp_y = 0.0;
    for octave in 0..octaves {
        warp_x += perlin2(
            x * frequency + 17.0,
            y * frequency - 31.0,
            seed.wrapping_add(octave as u64),
        ) * amplitude;
        warp_y += perlin2(
            x * frequency - 47.0,
            y * frequency + 11.0,
            seed.wrapping_add(0x9e37).wrapping_add(octave as u64),
        ) * amplitude;
        amplitude *= 0.5;
        frequency *= 2.0;
    }
    Ok(perlin2(
        x + warp_x * strength,
        y + warp_y * strength,
        seed ^ 0xa5a5_a5a5_a5a5_a5a5,
    ))
}

fn numbers(value: &Value, name: &str) -> Result<Vec<f64>, String> {
    let Value::List(values) = value else {
        return Err(format!("{name} expects list, got {}", value.type_name()));
    };
    values
        .borrow()
        .iter()
        .enumerate()
        .map(|(index, value)| {
            value
                .as_f64()
                .filter(|value| value.is_finite())
                .ok_or_else(|| format!("{name} expects finite number at index {index}"))
        })
        .collect()
}

fn numbers_non_empty(value: &Value, name: &str) -> Result<Vec<f64>, String> {
    let values = numbers(value, name)?;
    if values.is_empty() {
        Err(format!("{name} expects a non-empty list"))
    } else {
        Ok(values)
    }
}

fn mean(values: &[f64]) -> f64 {
    values.iter().sum::<f64>() / values.len() as f64
}

fn median(mut values: Vec<f64>) -> f64 {
    values.sort_by(f64::total_cmp);
    let middle = values.len() / 2;
    if values.len() % 2 == 0 {
        (values[middle - 1] + values[middle]) * 0.5
    } else {
        values[middle]
    }
}

fn mode(mut values: Vec<f64>) -> f64 {
    values.sort_by(f64::total_cmp);
    let mut best_value = values[0];
    let mut best_count = 1usize;
    let mut current_value = values[0];
    let mut current_count = 1usize;
    for value in values.into_iter().skip(1) {
        if value == current_value {
            current_count += 1;
        } else {
            if current_count > best_count {
                best_value = current_value;
                best_count = current_count;
            }
            current_value = value;
            current_count = 1;
        }
    }
    if current_count > best_count {
        current_value
    } else {
        best_value
    }
}

fn optional_bool(value: Option<&Value>, name: &str) -> Result<bool, String> {
    match value {
        None => Ok(false),
        Some(Value::Bool(value)) => Ok(*value),
        Some(value) => Err(format!(
            "{name} expects bool option, got {}",
            value.type_name()
        )),
    }
}

fn variance(values: &[f64], sample: bool) -> Result<f64, String> {
    if sample && values.len() < 2 {
        return Err("sample variance requires at least two values".into());
    }
    let average = mean(values);
    let sum = values
        .iter()
        .map(|value| {
            let delta = value - average;
            delta * delta
        })
        .sum::<f64>();
    Ok(sum / (values.len() - usize::from(sample)) as f64)
}

fn quantile(mut values: Vec<f64>, q: f64) -> Result<f64, String> {
    require_range("quantile q", q, 0.0, 1.0)?;
    values.sort_by(f64::total_cmp);
    let position = q * (values.len() - 1) as f64;
    let lower = position.floor() as usize;
    let upper = position.ceil() as usize;
    let t = position - lower as f64;
    Ok(values[lower] + (values[upper] - values[lower]) * t)
}

fn covariance(a: &[f64], b: &[f64], sample: bool) -> Result<f64, String> {
    if a.len() != b.len() {
        return Err(format!(
            "covariance length mismatch: {} and {}",
            a.len(),
            b.len()
        ));
    }
    if sample && a.len() < 2 {
        return Err("sample covariance requires at least two pairs".into());
    }
    let mean_a = mean(a);
    let mean_b = mean(b);
    Ok(a.iter()
        .zip(b)
        .map(|(a, b)| (a - mean_a) * (b - mean_b))
        .sum::<f64>()
        / (a.len() - usize::from(sample)) as f64)
}

fn correlation(a: &[f64], b: &[f64]) -> Result<f64, String> {
    let covariance = covariance(a, b, false)?;
    let denominator = variance(a, false)?.sqrt() * variance(b, false)?.sqrt();
    if denominator <= ZERO_EPSILON {
        return Err("correlation domain error: zero variance".into());
    }
    Ok(covariance / denominator)
}

fn moving_average(args: &[Value]) -> Result<Value, String> {
    let values = numbers(&args[0], "moving_average")?;
    let window = integer(&args[1], "moving_average")?;
    if window <= 0 || window as usize > values.len() {
        return Err(format!(
            "moving_average expects window in 1..={}, received {window}",
            values.len()
        ));
    }
    let window = window as usize;
    let mut sum: f64 = values[..window].iter().sum();
    let mut output = vec![Value::Float(sum / window as f64)];
    for index in window..values.len() {
        sum += values[index] - values[index - window];
        output.push(Value::Float(sum / window as f64));
    }
    Ok(Value::list(output))
}

fn histogram(args: &[Value]) -> Result<Value, String> {
    let values = numbers_non_empty(&args[0], "histogram")?;
    let bins = integer(&args[1], "histogram")?;
    if !(1..=4096).contains(&bins) {
        return Err("histogram expects bins in 1..=4096".into());
    }
    let bins = bins as usize;
    let min = values.iter().copied().fold(f64::INFINITY, f64::min);
    let max = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let width = if max > min {
        (max - min) / bins as f64
    } else {
        1.0
    };
    let mut counts = vec![0_i64; bins];
    for value in values {
        let index = if max > min {
            (((value - min) / width).floor() as usize).min(bins - 1)
        } else {
            0
        };
        counts[index] += 1;
    }
    Ok(Value::list(
        counts
            .into_iter()
            .enumerate()
            .map(|(index, count)| {
                Value::list(vec![
                    Value::Float(min + index as f64 * width),
                    Value::Float(min + (index + 1) as f64 * width),
                    Value::Int(count),
                ])
            })
            .collect(),
    ))
}

fn ema(args: &[Value]) -> Result<Value, String> {
    let values = numbers_non_empty(&args[0], "ema")?;
    let alpha = number(&args[1], "ema")?;
    if !alpha.is_finite() || alpha <= 0.0 || alpha > 1.0 {
        return Err("ema expects 0 < alpha <= 1".into());
    }
    let mut current = values[0];
    let mut output = vec![Value::Float(current)];
    for value in values.into_iter().skip(1) {
        current = alpha * value + (1.0 - alpha) * current;
        output.push(Value::Float(current));
    }
    Ok(Value::list(output))
}

fn pow_int(mut base: i64, exponent: i64) -> Result<i64, String> {
    if exponent < 0 {
        return Err("pow_int expects exponent >= 0".into());
    }
    let mut exponent = exponent as u64;
    let mut result = 1_i64;
    while exponent > 0 {
        if exponent & 1 == 1 {
            result = result
                .checked_mul(base)
                .ok_or_else(|| "pow_int integer overflow".to_string())?;
        }
        exponent >>= 1;
        if exponent > 0 {
            base = base
                .checked_mul(base)
                .ok_or_else(|| "pow_int integer overflow".to_string())?;
        }
    }
    Ok(result)
}

fn clamp_length(args: &[Value]) -> Result<Value, String> {
    let values = vector(&args[0], "clamp_length")?;
    let max = number(&args[1], "clamp_length")?;
    if !max.is_finite() || max < 0.0 {
        return Err("clamp_length expects finite max >= 0".into());
    }
    let length = dot_slice(values, values).sqrt();
    if length <= max || length <= ZERO_EPSILON {
        Ok(args[0].clone())
    } else {
        vector_like(&args[0], values.iter().map(|value| value * max / length))
    }
}

fn poly_eval(coefficients: &[f64], x: f64) -> f64 {
    coefficients.iter().fold(0.0, |accumulator, coefficient| {
        accumulator * x + coefficient
    })
}

fn integrate_trapezoid(args: &[Value]) -> Result<f64, String> {
    let values = numbers_non_empty(&args[0], "integrate_trapezoid")?;
    if values.len() < 2 {
        return Err("integrate_trapezoid requires at least two samples".into());
    }
    let step = number(&args[1], "integrate_trapezoid")?;
    if !step.is_finite() || step <= 0.0 {
        return Err("integrate_trapezoid expects finite step > 0".into());
    }
    Ok(values
        .windows(2)
        .map(|pair| (pair[0] + pair[1]) * 0.5 * step)
        .sum())
}

fn integrate_simpson(args: &[Value]) -> Result<f64, String> {
    let values = numbers_non_empty(&args[0], "integrate_simpson")?;
    if values.len() < 3 || values.len() % 2 == 0 {
        return Err("integrate_simpson requires an odd sample count >= 3".into());
    }
    let step = number(&args[1], "integrate_simpson")?;
    if !step.is_finite() || step <= 0.0 {
        return Err("integrate_simpson expects finite step > 0".into());
    }
    let mut sum = values[0] + values[values.len() - 1];
    for (index, value) in values[1..values.len() - 1].iter().enumerate() {
        sum += if index % 2 == 0 { 4.0 } else { 2.0 } * value;
    }
    Ok(sum * step / 3.0)
}

fn poly_root_bisection(args: &[Value]) -> Result<f64, String> {
    let coefficients = numbers_non_empty(&args[0], "poly_root_bisection")?;
    let mut lo = number(&args[1], "poly_root_bisection")?;
    let mut hi = number(&args[2], "poly_root_bisection")?;
    if !lo.is_finite() || !hi.is_finite() || lo >= hi {
        return Err("poly_root_bisection expects finite lo < hi".into());
    }
    let iterations = args
        .get(3)
        .map(|value| integer(value, "poly_root_bisection"))
        .transpose()?
        .unwrap_or(64);
    if !(1..=256).contains(&iterations) {
        return Err("poly_root_bisection iterations must be in 1..=256".into());
    }
    let tolerance = args
        .get(4)
        .map(|value| number(value, "poly_root_bisection"))
        .transpose()?
        .unwrap_or(DEFAULT_EPSILON);
    if !tolerance.is_finite() || tolerance <= 0.0 {
        return Err("poly_root_bisection tolerance must be finite and > 0".into());
    }
    let mut lo_value = poly_eval(&coefficients, lo);
    let hi_value = poly_eval(&coefficients, hi);
    if lo_value == 0.0 {
        return Ok(lo);
    }
    if hi_value == 0.0 {
        return Ok(hi);
    }
    if lo_value.signum() == hi_value.signum() {
        return Err("poly_root_bisection interval does not bracket a root".into());
    }
    for _ in 0..iterations {
        let middle = (lo + hi) * 0.5;
        let value = poly_eval(&coefficients, middle);
        if value.abs() <= tolerance || (hi - lo) * 0.5 <= tolerance {
            return Ok(middle);
        }
        if value.signum() == lo_value.signum() {
            lo = middle;
            lo_value = value;
        } else {
            hi = middle;
        }
    }
    Ok((lo + hi) * 0.5)
}

enum CurveKind {
    Quadratic,
    Cubic,
    CatmullRom,
    Hermite,
}

fn curve(args: &[Value], kind: CurveKind) -> Result<Value, String> {
    let point_count = match kind {
        CurveKind::Quadratic => 3,
        _ => 4,
    };
    let points = args[..point_count]
        .iter()
        .map(|value| vector(value, "curve"))
        .collect::<Result<Vec<_>, _>>()?;
    for point in &points[1..] {
        require_same_dimension(points[0], point, "curve")?;
    }
    let t = number(&args[point_count], "curve")?;
    let output = (0..points[0].len()).map(|index| {
        let p0 = points[0][index];
        let p1 = points[1][index];
        let p2 = points[2][index];
        match kind {
            CurveKind::Quadratic => {
                let u = 1.0 - t;
                u * u * p0 + 2.0 * u * t * p1 + t * t * p2
            }
            CurveKind::Cubic => {
                let p3 = points[3][index];
                let u = 1.0 - t;
                u * u * u * p0 + 3.0 * u * u * t * p1 + 3.0 * u * t * t * p2 + t * t * t * p3
            }
            CurveKind::CatmullRom => {
                let p3 = points[3][index];
                let t2 = t * t;
                let t3 = t2 * t;
                0.5 * ((2.0 * p1)
                    + (-p0 + p2) * t
                    + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t2
                    + (-p0 + 3.0 * p1 - 3.0 * p2 + p3) * t3)
            }
            CurveKind::Hermite => {
                let tangent0 = p1;
                let point1 = p2;
                let tangent1 = points[3][index];
                let t2 = t * t;
                let t3 = t2 * t;
                (2.0 * t3 - 3.0 * t2 + 1.0) * p0
                    + (t3 - 2.0 * t2 + t) * tangent0
                    + (-2.0 * t3 + 3.0 * t2) * point1
                    + (t3 - t2) * tangent1
            }
        }
    });
    vector_like(&args[0], output)
}

fn closest_point_segment(args: &[Value]) -> Result<Value, String> {
    let point = vector(&args[0], "closest_point_segment")?;
    let a = vector(&args[1], "closest_point_segment")?;
    let b = vector(&args[2], "closest_point_segment")?;
    require_same_dimension(point, a, "closest_point_segment")?;
    require_same_dimension(point, b, "closest_point_segment")?;
    let ab = a.iter().zip(b).map(|(a, b)| b - a).collect::<Vec<_>>();
    let denominator = dot_slice(&ab, &ab);
    if denominator <= ZERO_EPSILON {
        return vector_like(&args[0], a.iter().copied());
    }
    let ap = point
        .iter()
        .zip(a)
        .map(|(point, a)| point - a)
        .collect::<Vec<_>>();
    let t = (dot_slice(&ap, &ab) / denominator).clamp(0.0, 1.0);
    vector_like(&args[0], a.iter().zip(ab).map(|(a, ab)| a + ab * t))
}

fn segment_intersection2(args: &[Value]) -> Result<Value, String> {
    let points = args
        .iter()
        .map(|value| match value {
            Value::Vec2(point) => Ok(*point),
            other => Err(format!(
                "segment_intersection2 expects vec2, got {}",
                other.type_name()
            )),
        })
        .collect::<Result<Vec<_>, String>>()?;
    let p = points[0];
    let r = [points[1][0] - p[0], points[1][1] - p[1]];
    let q = points[2];
    let s = [points[3][0] - q[0], points[3][1] - q[1]];
    let cross = |a: [f64; 2], b: [f64; 2]| a[0] * b[1] - a[1] * b[0];
    let denominator = cross(r, s);
    if denominator.abs() <= ZERO_EPSILON {
        return Ok(Value::Null);
    }
    let q_minus_p = [q[0] - p[0], q[1] - p[1]];
    let t = cross(q_minus_p, s) / denominator;
    let u = cross(q_minus_p, r) / denominator;
    if (0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u) {
        Ok(Value::Vec2([p[0] + t * r[0], p[1] + t * r[1]]))
    } else {
        Ok(Value::Null)
    }
}

fn ray_plane_intersection(args: &[Value]) -> Result<Value, String> {
    let origin = vector(&args[0], "ray_plane_intersection")?;
    let direction = vector(&args[1], "ray_plane_intersection")?;
    let plane_point = vector(&args[2], "ray_plane_intersection")?;
    let plane_normal = vector(&args[3], "ray_plane_intersection")?;
    require_same_dimension(origin, direction, "ray_plane_intersection")?;
    require_same_dimension(origin, plane_point, "ray_plane_intersection")?;
    require_same_dimension(origin, plane_normal, "ray_plane_intersection")?;
    if !matches!(origin.len(), 2 | 3) {
        return Err("ray_plane_intersection supports vec2 or vec3".into());
    }
    let denominator = dot_slice(direction, plane_normal);
    if denominator.abs() <= ZERO_EPSILON {
        return Ok(Value::Null);
    }
    let difference = plane_point
        .iter()
        .zip(origin)
        .map(|(plane, origin)| plane - origin)
        .collect::<Vec<_>>();
    let distance = dot_slice(&difference, plane_normal) / denominator;
    if distance < 0.0 {
        return Ok(Value::Null);
    }
    vector_like(
        &args[0],
        origin
            .iter()
            .zip(direction)
            .map(|(origin, direction)| origin + direction * distance),
    )
}

fn approx_value(a: &Value, b: &Value, epsilon: f64) -> Result<bool, String> {
    match (a, b) {
        (Value::Int(a), Value::Int(b)) => Ok(a == b),
        (Value::Int(_) | Value::Float(_), Value::Int(_) | Value::Float(_)) => {
            let a = a.as_f64().unwrap();
            let b = b.as_f64().unwrap();
            Ok((a - b).abs() <= epsilon)
        }
        (Value::Vec2(a), Value::Vec2(b)) => {
            Ok(a.iter().zip(b).all(|(a, b)| (a - b).abs() <= epsilon))
        }
        (Value::Vec3(a), Value::Vec3(b)) => {
            Ok(a.iter().zip(b).all(|(a, b)| (a - b).abs() <= epsilon))
        }
        (Value::Vec4(a), Value::Vec4(b)) | (Value::Quat(a), Value::Quat(b)) => {
            Ok(a.iter().zip(b).all(|(a, b)| (a - b).abs() <= epsilon))
        }
        (Value::Mat3(a), Value::Mat3(b)) => {
            Ok(a.iter().zip(b).all(|(a, b)| (a - b).abs() <= epsilon))
        }
        (Value::Mat4(a), Value::Mat4(b)) => {
            Ok(a.iter().zip(b).all(|(a, b)| (a - b).abs() <= epsilon))
        }
        _ if a.type_name() == b.type_name() => Ok(a == b),
        _ => Err(format!(
            "approx_eq incompatible types: {} and {}",
            a.type_name(),
            b.type_name()
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn call(native: NativeId, args: &[Value]) -> Value {
        call_math_native(native, args).unwrap().value
    }

    #[test]
    fn vectors_and_matrix_inverse_work() {
        let a = call(
            NativeId::Vec3,
            &[Value::Float(1.0), Value::Float(2.0), Value::Float(3.0)],
        );
        let b = call(
            NativeId::Vec3,
            &[Value::Float(4.0), Value::Float(5.0), Value::Float(6.0)],
        );
        assert_eq!(call(NativeId::Dot, &[a, b]), Value::Float(32.0));

        let matrix = Value::Mat3([2.0, 0.0, 0.0, 0.0, 4.0, 0.0, 0.0, 0.0, 1.0]);
        let inverse = call(NativeId::MatInverse, std::slice::from_ref(&matrix));
        let identity = call(NativeId::MatMul, &[matrix, inverse]);
        assert!(matches!(
            call(
                NativeId::ApproxEq,
                &[
                    identity,
                    Value::Mat3(identity_matrix::<9>(3)),
                    Value::Float(1e-10)
                ]
            ),
            Value::Bool(true)
        ));
    }

    #[test]
    fn seeded_rng_and_noise_are_repeatable() {
        let a = call(NativeId::RngNew, &[Value::Int(42)]);
        let b = call(NativeId::RngNew, &[Value::Int(42)]);
        let av = call(NativeId::RngNextFloat, &[a]);
        let bv = call(NativeId::RngNextFloat, &[b]);
        assert_eq!(av, bv);
        assert_eq!(noise2(1.25, -9.5, 7), noise2(1.25, -9.5, 7));
    }

    #[test]
    fn statistics_and_root_finding_work() {
        let values = Value::list(vec![
            Value::Int(1),
            Value::Int(2),
            Value::Int(3),
            Value::Int(4),
        ]);
        assert_eq!(
            call(NativeId::Mean, std::slice::from_ref(&values)),
            Value::Float(2.5)
        );
        assert_eq!(call(NativeId::Median, &[values]), Value::Float(2.5));
        let root = call(
            NativeId::PolyRootBisection,
            &[
                Value::list(vec![Value::Int(1), Value::Int(0), Value::Int(-2)]),
                Value::Float(0.0),
                Value::Float(2.0),
            ],
        );
        assert!((root.as_f64().unwrap() - 2.0_f64.sqrt()).abs() < 1e-8);
    }

    #[test]
    fn transforms_projection_and_geometry_work() {
        let translation = call(NativeId::Mat4Translation, &[Value::Vec3([3.0, 4.0, 5.0])]);
        assert_eq!(
            call(
                NativeId::TransformPoint,
                &[translation, Value::Vec3([1.0, 2.0, 3.0])]
            ),
            Value::Vec3([4.0, 6.0, 8.0])
        );
        let view = call(
            NativeId::Mat4LookAt,
            &[
                Value::Vec3([0.0, 0.0, 5.0]),
                Value::Vec3([0.0, 0.0, 0.0]),
                Value::Vec3([0.0, 1.0, 0.0]),
            ],
        );
        assert_eq!(
            call(
                NativeId::TransformPoint,
                &[view, Value::Vec3([0.0, 0.0, 5.0])]
            ),
            Value::Vec3([0.0, 0.0, 0.0])
        );
        assert_eq!(
            call(
                NativeId::RayPlaneIntersection,
                &[
                    Value::Vec3([0.0, 1.0, 0.0]),
                    Value::Vec3([0.0, -1.0, 0.0]),
                    Value::Vec3([0.0, 0.0, 0.0]),
                    Value::Vec3([0.0, 1.0, 0.0]),
                ]
            ),
            Value::Vec3([0.0, 0.0, 0.0])
        );
    }

    #[test]
    fn distributions_noise_and_extended_statistics_work() {
        let first = call(NativeId::RngNew, &[Value::Int(7)]);
        let second = call(NativeId::RngNew, &[Value::Int(7)]);
        assert_eq!(
            call(
                NativeId::RngGaussian,
                &[first, Value::Float(0.0), Value::Float(1.0)]
            ),
            call(
                NativeId::RngGaussian,
                &[second, Value::Float(0.0), Value::Float(1.0)]
            )
        );
        assert_eq!(perlin2(5.0, 9.0, 11), 0.0);
        let values = Value::list(vec![
            Value::Int(1),
            Value::Int(2),
            Value::Int(2),
            Value::Int(4),
        ]);
        assert_eq!(
            call(NativeId::Mode, std::slice::from_ref(&values)),
            Value::Float(2.0)
        );
        assert_eq!(
            call(NativeId::PowInt, &[Value::Int(3), Value::Int(4)]),
            Value::Int(81)
        );
        assert_eq!(
            call(NativeId::Histogram, &[values, Value::Int(2)]).len(),
            Some(2)
        );
    }
}
