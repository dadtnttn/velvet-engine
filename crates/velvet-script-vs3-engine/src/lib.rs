//! Optional VelvetEngine adapters for VS3.
//!
//! The VS3 language core (`velvet-script-vs3`) must stay free of engine
//! subsystems; everything here is an explicit, optional bridge a host opts
//! into. Today this crate carries the checked `velvet-math` conversions that
//! used to live inside the language core.
//!
//! ```ignore
//! use velvet_script_vs3_engine::{from_engine_vec2, to_engine_vec2};
//!
//! let value = from_engine_vec2(velvet_math::Vec2::new(1.0, 2.0));
//! let back = to_engine_vec2(&value)?;
//! ```

#![deny(missing_docs)]

use thiserror::Error;
use velvet_script_vs3::{mat3_val, mat4_val, vec2_val, vec3_val, Value};

/// Failure converting a precision-preserving VS3 value to engine `f32` math.
#[derive(Debug, Error, Clone, PartialEq)]
pub enum Vs3MathBridgeError {
    /// Runtime value has the wrong mathematical type.
    #[error("expected `{expected}`, found `{found}`")]
    Type {
        /// Required VS3 type.
        expected: &'static str,
        /// Actual VS3 type.
        found: &'static str,
    },
    /// A component cannot be represented as finite `f32`.
    #[error("{kind} component {index} is outside finite f32 range: {value}")]
    Range {
        /// Mathematical value kind.
        kind: &'static str,
        /// Zero-based component index.
        index: usize,
        /// Rejected value.
        value: f64,
    },
}

/// Promote an engine `f32` vector to a VS3 `f64` vector.
pub fn from_engine_vec2(value: velvet_math::Vec2) -> Value {
    vec2_val(f64::from(value.x), f64::from(value.y))
}

/// Promote an engine `f32` vector to a VS3 `f64` vector.
pub fn from_engine_vec3(value: velvet_math::Vec3) -> Value {
    vec3_val(f64::from(value.x), f64::from(value.y), f64::from(value.z))
}

/// Promote an engine column-major `f32` matrix to VS3 `f64`.
pub fn from_engine_mat3(value: velvet_math::Mat3) -> Value {
    mat3_val([
        value.x_axis[0] as f64,
        value.x_axis[1] as f64,
        value.x_axis[2] as f64,
        value.y_axis[0] as f64,
        value.y_axis[1] as f64,
        value.y_axis[2] as f64,
        value.z_axis[0] as f64,
        value.z_axis[1] as f64,
        value.z_axis[2] as f64,
    ])
}

/// Promote an engine column-major `f32` matrix to VS3 `f64`.
pub fn from_engine_mat4(value: velvet_math::Mat4) -> Value {
    let mut output = [0.0; 16];
    for (target, source) in output.iter_mut().zip(
        value
            .x_axis
            .into_iter()
            .chain(value.y_axis)
            .chain(value.z_axis)
            .chain(value.w_axis),
    ) {
        *target = f64::from(source);
    }
    mat4_val(output)
}

/// Convert a VS3 `vec2` to finite engine `f32` components.
pub fn to_engine_vec2(value: &Value) -> Result<velvet_math::Vec2, Vs3MathBridgeError> {
    let Value::Vec2(values) = value else {
        return Err(bridge_type("vec2", value));
    };
    let values = to_f32_array("vec2", values)?;
    Ok(velvet_math::Vec2::new(values[0], values[1]))
}

/// Convert a VS3 `vec3` to finite engine `f32` components.
pub fn to_engine_vec3(value: &Value) -> Result<velvet_math::Vec3, Vs3MathBridgeError> {
    let Value::Vec3(values) = value else {
        return Err(bridge_type("vec3", value));
    };
    let values = to_f32_array("vec3", values)?;
    Ok(velvet_math::Vec3::new(values[0], values[1], values[2]))
}

/// Convert a VS3 column-major `mat3` to finite engine `f32` components.
pub fn to_engine_mat3(value: &Value) -> Result<velvet_math::Mat3, Vs3MathBridgeError> {
    let Value::Mat3(values) = value else {
        return Err(bridge_type("mat3", value));
    };
    let values = to_f32_array("mat3", values)?;
    Ok(velvet_math::Mat3::from_cols(
        values[0..3].try_into().unwrap(),
        values[3..6].try_into().unwrap(),
        values[6..9].try_into().unwrap(),
    ))
}

/// Convert a VS3 column-major `mat4` to finite engine `f32` components.
pub fn to_engine_mat4(value: &Value) -> Result<velvet_math::Mat4, Vs3MathBridgeError> {
    let Value::Mat4(values) = value else {
        return Err(bridge_type("mat4", value));
    };
    Ok(velvet_math::Mat4::from_cols_array(to_f32_array(
        "mat4", values,
    )?))
}

fn bridge_type(expected: &'static str, value: &Value) -> Vs3MathBridgeError {
    Vs3MathBridgeError::Type {
        expected,
        found: value.type_name(),
    }
}

fn to_f32_array<const N: usize>(
    kind: &'static str,
    values: &[f64; N],
) -> Result<[f32; N], Vs3MathBridgeError> {
    let mut output = [0.0; N];
    for (index, (target, value)) in output.iter_mut().zip(values).enumerate() {
        if !value.is_finite() || value.abs() > f64::from(f32::MAX) {
            return Err(Vs3MathBridgeError::Range {
                kind,
                index,
                value: *value,
            });
        }
        *target = *value as f32;
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use velvet_script_vs3::vec3_val;

    #[test]
    fn engine_math_bridge_is_explicit_checked_and_column_major() {
        // Engine f32 → script f64 keeps every component.
        let engine_vector = velvet_math::Vec3::new(1.25, -2.5, 3.75);
        let promoted = from_engine_vec3(engine_vector);
        assert_eq!(promoted, vec3_val(1.25, -2.5, 3.75));

        // Script → engine round-trips through finite narrowing.
        let converted = to_engine_vec3(&promoted).unwrap();
        assert_eq!(converted, engine_vector);

        // Wrong shape is a typed error, never a panic.
        let wrong = Err(Vs3MathBridgeError::Type {
            expected: "vec2",
            found: "string",
        });
        assert_eq!(
            to_engine_vec2(&velvet_script_vs3::string_val("nope")),
            wrong
        );

        // Column-major layout survives mat3 promotion/conversion.
        let cols = velvet_math::Mat3::from_cols([1.0, 2.0, 3.0], [4.0, 5.0, 6.0], [7.0, 8.0, 9.0]);
        let mat = to_engine_mat3(&from_engine_mat3(cols)).unwrap();
        assert_eq!(mat, cols);

        // Non-finite components are rejected with the offending index.
        let huge = vec3_val(f64::INFINITY, 0.0, 0.0);
        assert_eq!(
            to_engine_vec3(&huge),
            Err(Vs3MathBridgeError::Range {
                kind: "vec3",
                index: 0,
                value: f64::INFINITY,
            })
        );

        // Mat4 keeps all sixteen columns in order.
        let identity = velvet_math::Mat4::IDENTITY;
        assert_eq!(to_engine_mat4(&from_engine_mat4(identity)), Ok(identity));

        // Vec2 helpers cover the 2D path.
        let two = velvet_math::Vec2::new(8.0, -4.0);
        assert_eq!(to_engine_vec2(&from_engine_vec2(two)), Ok(two));
    }
}
