//! # velvet-script-bytecode
//!
//! Opcode definitions, constants, compiled modules, line tables, and
//! disassembly helpers for the Velvet Script VM.

#![deny(missing_docs)]

use std::collections::HashMap;
use std::fmt::Write as _;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Bytecode format version carried in module metadata.
pub const BYTECODE_VERSION: u16 = 2;

/// Native host function identifiers shared by compiler and VM.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u16)]
pub enum NativeId {
    /// `print(…)`
    Print = 0,
    /// `abs(x)`
    Abs = 1,
    /// `min(a, b)`
    Min = 2,
    /// `max(a, b)`
    Max = 3,
    /// `floor(x)`
    Floor = 4,
    /// `ceil(x)`
    Ceil = 5,
    /// `clamp(x, lo, hi)`
    Clamp = 6,
    /// `len(x)`
    Len = 7,
    /// `concat(…)`
    Concat = 8,
    /// `str(x)`
    Str = 9,
    /// `sin(x)`
    Sin = 10,
    /// `cos(x)`
    Cos = 11,
    /// `sqrt(x)`
    Sqrt = 12,
    /// `pow(x, y)`
    Pow = 13,
    /// `lerp(a, b, t)`
    Lerp = 14,
    /// `hash_sha256(s) -> hex str`
    HashSha256 = 15,
    /// `hex_encode(s)` (utf-8 bytes)
    HexEncode = 16,
    /// `base64_encode(s)`
    Base64Encode = 17,
    /// `show(id [, expression [, at]])` — presentation host: show sprite (state only).
    PresentShow = 18,
    /// `set_bg(path)` — presentation host: set background path.
    PresentSetBg = 19,
    /// `ui_flag(name, on)` — presentation host: set UI flag (bool).
    PresentUiFlag = 20,
    /// `ui_flag_get(name) -> bool` — read UI flag.
    PresentUiFlagGet = 21,
    /// `hide(id)` — hide / remove shown sprite.
    PresentHide = 22,
    /// `type_of(value) -> string`.
    TypeOf = 23,
    /// `list_push(list, value) -> list`.
    ListPush = 24,
    /// `list_pop(list) -> value|null`.
    ListPop = 25,
    /// `map_has(map, key) -> bool`.
    MapHas = 26,
    /// `map_keys(map) -> list<string>`.
    MapKeys = 27,
    /// `assert(condition [, message])`.
    Assert = 28,
    /// `fail(message)` raises a runtime error.
    Fail = 29,
    /// `tan(x)`.
    Tan = 30,
    /// `asin(x)`.
    Asin = 31,
    /// `acos(x)`.
    Acos = 32,
    /// `atan(x)`.
    Atan = 33,
    /// `atan2(y, x)`.
    Atan2 = 34,
    /// `exp(x)`.
    Exp = 35,
    /// `ln(x)`.
    Ln = 36,
    /// `log2(x)`.
    Log2 = 37,
    /// `log10(x)`.
    Log10 = 38,
    /// `cbrt(x)`.
    Cbrt = 39,
    /// `round(x)`.
    Round = 40,
    /// `trunc(x)`.
    Trunc = 41,
    /// `fract(x)`.
    Fract = 42,
    /// `sign(x)`.
    Sign = 43,
    /// `hypot(x, y)`.
    Hypot = 44,
    /// `degrees(radians)`.
    Degrees = 45,
    /// `radians(degrees)`.
    Radians = 46,
    /// `inverse_lerp(a, b, value)`.
    InverseLerp = 47,
    /// `remap(value, in_min, in_max, out_min, out_max)`.
    Remap = 48,
    /// `smoothstep(edge0, edge1, value)`.
    Smoothstep = 49,
    /// `is_finite(x)`.
    IsFinite = 50,
    /// `is_nan(x)`.
    IsNan = 51,
    /// `is_infinite(x)`.
    IsInfinite = 52,
    /// `approx_eq(a, b [, epsilon])`.
    ApproxEq = 53,
    /// `gcd(a, b)`.
    Gcd = 54,
    /// `lcm(a, b)`.
    Lcm = 55,
    /// `vec2(x [, y])`.
    Vec2 = 56,
    /// `vec3(x [, y [, z]])`.
    Vec3 = 57,
    /// `vec4(x [, y [, z [, w]]])`.
    Vec4 = 58,
    /// `dot(a, b)`.
    Dot = 59,
    /// `cross(a, b)`.
    Cross = 60,
    /// `length(v)`.
    Length = 61,
    /// `normalize(v)`.
    Normalize = 62,
    /// `distance(a, b)`.
    Distance = 63,
    /// `angle_between(a, b)`.
    AngleBetween = 64,
    /// `reflect(v, normal)`.
    Reflect = 65,
    /// `project(v, onto)`.
    Project = 66,
    /// `vec_lerp(a, b, t)`.
    VecLerp = 67,
    /// `mat3_identity()`.
    Mat3Identity = 68,
    /// `mat4_identity()`.
    Mat4Identity = 69,
    /// `mat3(...)` from nine column-major numbers.
    Mat3 = 70,
    /// `mat4(...)` from sixteen column-major numbers.
    Mat4 = 71,
    /// `mat_mul(a, b)`.
    MatMul = 72,
    /// `mat_transpose(m)`.
    MatTranspose = 73,
    /// `mat_determinant(m)`.
    MatDeterminant = 74,
    /// `mat_inverse(m)`.
    MatInverse = 75,
    /// `transform_point(m, point)`.
    TransformPoint = 76,
    /// `transform_vector(m, vector)`.
    TransformVector = 77,
    /// `quat(x, y, z, w)`.
    Quat = 78,
    /// `quat_identity()`.
    QuatIdentity = 79,
    /// `quat_axis_angle(axis, radians)`.
    QuatAxisAngle = 80,
    /// `quat_mul(a, b)`.
    QuatMul = 81,
    /// `quat_rotate(q, vector)`.
    QuatRotate = 82,
    /// `quat_normalize(q)`.
    QuatNormalize = 83,
    /// `quat_inverse(q)`.
    QuatInverse = 84,
    /// `quat_slerp(a, b, t)`.
    QuatSlerp = 85,
    /// `rng_new(seed)`.
    RngNew = 86,
    /// `rng_next_float(rng)`.
    RngNextFloat = 87,
    /// `rng_range_int(rng, min, max)`.
    RngRangeInt = 88,
    /// `rng_range_float(rng, min, max)`.
    RngRangeFloat = 89,
    /// `rng_bool(rng, probability)`.
    RngBool = 90,
    /// `shuffle(rng, list)`.
    Shuffle = 91,
    /// `choose(rng, list)`.
    Choose = 92,
    /// `weighted_choose(rng, weights)`.
    WeightedChoose = 93,
    /// `noise1(x, seed)`.
    Noise1 = 94,
    /// `noise2(x, y, seed)`.
    Noise2 = 95,
    /// `fbm2(x, y, seed, octaves [, lacunarity [, gain]])`.
    Fbm2 = 96,
    /// `sum(values)`.
    Sum = 97,
    /// `product(values)`.
    Product = 98,
    /// `mean(values)`.
    Mean = 99,
    /// `median(values)`.
    Median = 100,
    /// `variance(values [, sample])`.
    Variance = 101,
    /// `stddev(values [, sample])`.
    Stddev = 102,
    /// `quantile(values, q)`.
    Quantile = 103,
    /// `covariance(a, b [, sample])`.
    Covariance = 104,
    /// `correlation(a, b)`.
    Correlation = 105,
    /// `moving_average(values, window)`.
    MovingAverage = 106,
    /// `poly_eval(coefficients, x)`.
    PolyEval = 107,
    /// `integrate_trapezoid(samples, step)`.
    IntegrateTrapezoid = 108,
    /// `integrate_simpson(samples, step)`.
    IntegrateSimpson = 109,
    /// `poly_root_bisection(coefficients, lo, hi [, iterations [, tolerance]])`.
    PolyRootBisection = 110,
    /// `quadratic_bezier(p0, p1, p2, t)`.
    QuadraticBezier = 111,
    /// `cubic_bezier(p0, p1, p2, p3, t)`.
    CubicBezier = 112,
    /// `catmull_rom(p0, p1, p2, p3, t)`.
    CatmullRom = 113,
    /// `hermite(p0, tangent0, p1, tangent1, t)`.
    Hermite = 114,
    /// `closest_point_segment(point, a, b)`.
    ClosestPointSegment = 115,
    /// `segment_intersection2(a0, a1, b0, b1)`.
    SegmentIntersection2 = 116,
    /// `refract(v, normal, eta)`.
    Refract = 117,
    /// `mat3_translation(vec2)`.
    Mat3Translation = 118,
    /// `mat3_scale(vec2)`.
    Mat3Scale = 119,
    /// `mat3_rotation(radians)`.
    Mat3Rotation = 120,
    /// `mat4_translation(vec3)`.
    Mat4Translation = 121,
    /// `mat4_scale(vec3)`.
    Mat4Scale = 122,
    /// `mat4_rotation_x(radians)`.
    Mat4RotationX = 123,
    /// `mat4_rotation_y(radians)`.
    Mat4RotationY = 124,
    /// `mat4_rotation_z(radians)`.
    Mat4RotationZ = 125,
    /// `mat4_orthographic(left, right, bottom, top, near, far)`.
    Mat4Orthographic = 126,
    /// `mat4_perspective(fov_y, aspect, near, far)`.
    Mat4Perspective = 127,
    /// `mat4_look_at(eye, target, up)`.
    Mat4LookAt = 128,
    /// `rng_gaussian(rng, mean, stddev)`.
    RngGaussian = 129,
    /// `rng_exponential(rng, rate)`.
    RngExponential = 130,
    /// `perlin2(x, y, seed)`.
    Perlin2 = 131,
    /// `turbulence2(x, y, seed, octaves [, lacunarity [, gain]])`.
    Turbulence2 = 132,
    /// `domain_warp2(x, y, seed, octaves, strength)`.
    DomainWarp2 = 133,
    /// `mode(values)`.
    Mode = 134,
    /// `histogram(values, bins)`.
    Histogram = 135,
    /// `ema(values, alpha)`.
    Ema = 136,
    /// `is_even(value)`.
    IsEven = 137,
    /// `is_odd(value)`.
    IsOdd = 138,
    /// `pow_int(base, exponent)`.
    PowInt = 139,
    /// `vec_min(a, b)`.
    VecMin = 140,
    /// `vec_max(a, b)`.
    VecMax = 141,
    /// `clamp_length(vector, max)`.
    ClampLength = 142,
    /// `quat_euler(x, y, z)`.
    QuatEuler = 143,
    /// `ray_plane_intersection(origin, direction, plane_point, plane_normal)`.
    RayPlaneIntersection = 144,
}

impl NativeId {
    /// From raw id.
    pub fn from_u16(id: u16) -> Option<Self> {
        Some(match id {
            0 => Self::Print,
            1 => Self::Abs,
            2 => Self::Min,
            3 => Self::Max,
            4 => Self::Floor,
            5 => Self::Ceil,
            6 => Self::Clamp,
            7 => Self::Len,
            8 => Self::Concat,
            9 => Self::Str,
            10 => Self::Sin,
            11 => Self::Cos,
            12 => Self::Sqrt,
            13 => Self::Pow,
            14 => Self::Lerp,
            15 => Self::HashSha256,
            16 => Self::HexEncode,
            17 => Self::Base64Encode,
            18 => Self::PresentShow,
            19 => Self::PresentSetBg,
            20 => Self::PresentUiFlag,
            21 => Self::PresentUiFlagGet,
            22 => Self::PresentHide,
            23 => Self::TypeOf,
            24 => Self::ListPush,
            25 => Self::ListPop,
            26 => Self::MapHas,
            27 => Self::MapKeys,
            28 => Self::Assert,
            29 => Self::Fail,
            30 => Self::Tan,
            31 => Self::Asin,
            32 => Self::Acos,
            33 => Self::Atan,
            34 => Self::Atan2,
            35 => Self::Exp,
            36 => Self::Ln,
            37 => Self::Log2,
            38 => Self::Log10,
            39 => Self::Cbrt,
            40 => Self::Round,
            41 => Self::Trunc,
            42 => Self::Fract,
            43 => Self::Sign,
            44 => Self::Hypot,
            45 => Self::Degrees,
            46 => Self::Radians,
            47 => Self::InverseLerp,
            48 => Self::Remap,
            49 => Self::Smoothstep,
            50 => Self::IsFinite,
            51 => Self::IsNan,
            52 => Self::IsInfinite,
            53 => Self::ApproxEq,
            54 => Self::Gcd,
            55 => Self::Lcm,
            56 => Self::Vec2,
            57 => Self::Vec3,
            58 => Self::Vec4,
            59 => Self::Dot,
            60 => Self::Cross,
            61 => Self::Length,
            62 => Self::Normalize,
            63 => Self::Distance,
            64 => Self::AngleBetween,
            65 => Self::Reflect,
            66 => Self::Project,
            67 => Self::VecLerp,
            68 => Self::Mat3Identity,
            69 => Self::Mat4Identity,
            70 => Self::Mat3,
            71 => Self::Mat4,
            72 => Self::MatMul,
            73 => Self::MatTranspose,
            74 => Self::MatDeterminant,
            75 => Self::MatInverse,
            76 => Self::TransformPoint,
            77 => Self::TransformVector,
            78 => Self::Quat,
            79 => Self::QuatIdentity,
            80 => Self::QuatAxisAngle,
            81 => Self::QuatMul,
            82 => Self::QuatRotate,
            83 => Self::QuatNormalize,
            84 => Self::QuatInverse,
            85 => Self::QuatSlerp,
            86 => Self::RngNew,
            87 => Self::RngNextFloat,
            88 => Self::RngRangeInt,
            89 => Self::RngRangeFloat,
            90 => Self::RngBool,
            91 => Self::Shuffle,
            92 => Self::Choose,
            93 => Self::WeightedChoose,
            94 => Self::Noise1,
            95 => Self::Noise2,
            96 => Self::Fbm2,
            97 => Self::Sum,
            98 => Self::Product,
            99 => Self::Mean,
            100 => Self::Median,
            101 => Self::Variance,
            102 => Self::Stddev,
            103 => Self::Quantile,
            104 => Self::Covariance,
            105 => Self::Correlation,
            106 => Self::MovingAverage,
            107 => Self::PolyEval,
            108 => Self::IntegrateTrapezoid,
            109 => Self::IntegrateSimpson,
            110 => Self::PolyRootBisection,
            111 => Self::QuadraticBezier,
            112 => Self::CubicBezier,
            113 => Self::CatmullRom,
            114 => Self::Hermite,
            115 => Self::ClosestPointSegment,
            116 => Self::SegmentIntersection2,
            117 => Self::Refract,
            118 => Self::Mat3Translation,
            119 => Self::Mat3Scale,
            120 => Self::Mat3Rotation,
            121 => Self::Mat4Translation,
            122 => Self::Mat4Scale,
            123 => Self::Mat4RotationX,
            124 => Self::Mat4RotationY,
            125 => Self::Mat4RotationZ,
            126 => Self::Mat4Orthographic,
            127 => Self::Mat4Perspective,
            128 => Self::Mat4LookAt,
            129 => Self::RngGaussian,
            130 => Self::RngExponential,
            131 => Self::Perlin2,
            132 => Self::Turbulence2,
            133 => Self::DomainWarp2,
            134 => Self::Mode,
            135 => Self::Histogram,
            136 => Self::Ema,
            137 => Self::IsEven,
            138 => Self::IsOdd,
            139 => Self::PowInt,
            140 => Self::VecMin,
            141 => Self::VecMax,
            142 => Self::ClampLength,
            143 => Self::QuatEuler,
            144 => Self::RayPlaneIntersection,
            _ => return None,
        })
    }

    /// Raw id.
    pub fn as_u16(self) -> u16 {
        self as u16
    }

    /// Script-visible name.
    pub fn name(self) -> &'static str {
        match self {
            Self::Print => "print",
            Self::Abs => "abs",
            Self::Min => "min",
            Self::Max => "max",
            Self::Floor => "floor",
            Self::Ceil => "ceil",
            Self::Clamp => "clamp",
            Self::Len => "len",
            Self::Concat => "concat",
            Self::Str => "str",
            Self::Sin => "sin",
            Self::Cos => "cos",
            Self::Sqrt => "sqrt",
            Self::Pow => "pow",
            Self::Lerp => "lerp",
            Self::HashSha256 => "hash_sha256",
            Self::HexEncode => "hex_encode",
            Self::Base64Encode => "base64_encode",
            // Classic lexer reserves `show`/`hide` as story statements — use present_*.
            Self::PresentShow => "present_show",
            Self::PresentSetBg => "set_bg",
            Self::PresentUiFlag => "ui_flag",
            Self::PresentUiFlagGet => "ui_flag_get",
            Self::PresentHide => "present_hide",
            Self::TypeOf => "type_of",
            Self::ListPush => "list_push",
            Self::ListPop => "list_pop",
            Self::MapHas => "map_has",
            Self::MapKeys => "map_keys",
            Self::Assert => "assert",
            Self::Fail => "fail",
            Self::Tan => "tan",
            Self::Asin => "asin",
            Self::Acos => "acos",
            Self::Atan => "atan",
            Self::Atan2 => "atan2",
            Self::Exp => "exp",
            Self::Ln => "ln",
            Self::Log2 => "log2",
            Self::Log10 => "log10",
            Self::Cbrt => "cbrt",
            Self::Round => "round",
            Self::Trunc => "trunc",
            Self::Fract => "fract",
            Self::Sign => "sign",
            Self::Hypot => "hypot",
            Self::Degrees => "degrees",
            Self::Radians => "radians",
            Self::InverseLerp => "inverse_lerp",
            Self::Remap => "remap",
            Self::Smoothstep => "smoothstep",
            Self::IsFinite => "is_finite",
            Self::IsNan => "is_nan",
            Self::IsInfinite => "is_infinite",
            Self::ApproxEq => "approx_eq",
            Self::Gcd => "gcd",
            Self::Lcm => "lcm",
            Self::Vec2 => "vec2",
            Self::Vec3 => "vec3",
            Self::Vec4 => "vec4",
            Self::Dot => "dot",
            Self::Cross => "cross",
            Self::Length => "length",
            Self::Normalize => "normalize",
            Self::Distance => "distance",
            Self::AngleBetween => "angle_between",
            Self::Reflect => "reflect",
            Self::Project => "project",
            Self::VecLerp => "vec_lerp",
            Self::Mat3Identity => "mat3_identity",
            Self::Mat4Identity => "mat4_identity",
            Self::Mat3 => "mat3",
            Self::Mat4 => "mat4",
            Self::MatMul => "mat_mul",
            Self::MatTranspose => "mat_transpose",
            Self::MatDeterminant => "mat_determinant",
            Self::MatInverse => "mat_inverse",
            Self::TransformPoint => "transform_point",
            Self::TransformVector => "transform_vector",
            Self::Quat => "quat",
            Self::QuatIdentity => "quat_identity",
            Self::QuatAxisAngle => "quat_axis_angle",
            Self::QuatMul => "quat_mul",
            Self::QuatRotate => "quat_rotate",
            Self::QuatNormalize => "quat_normalize",
            Self::QuatInverse => "quat_inverse",
            Self::QuatSlerp => "quat_slerp",
            Self::RngNew => "rng_new",
            Self::RngNextFloat => "rng_next_float",
            Self::RngRangeInt => "rng_range_int",
            Self::RngRangeFloat => "rng_range_float",
            Self::RngBool => "rng_bool",
            Self::Shuffle => "shuffle",
            Self::Choose => "choose",
            Self::WeightedChoose => "weighted_choose",
            Self::Noise1 => "noise1",
            Self::Noise2 => "noise2",
            Self::Fbm2 => "fbm2",
            Self::Sum => "sum",
            Self::Product => "product",
            Self::Mean => "mean",
            Self::Median => "median",
            Self::Variance => "variance",
            Self::Stddev => "stddev",
            Self::Quantile => "quantile",
            Self::Covariance => "covariance",
            Self::Correlation => "correlation",
            Self::MovingAverage => "moving_average",
            Self::PolyEval => "poly_eval",
            Self::IntegrateTrapezoid => "integrate_trapezoid",
            Self::IntegrateSimpson => "integrate_simpson",
            Self::PolyRootBisection => "poly_root_bisection",
            Self::QuadraticBezier => "quadratic_bezier",
            Self::CubicBezier => "cubic_bezier",
            Self::CatmullRom => "catmull_rom",
            Self::Hermite => "hermite",
            Self::ClosestPointSegment => "closest_point_segment",
            Self::SegmentIntersection2 => "segment_intersection2",
            Self::Refract => "refract",
            Self::Mat3Translation => "mat3_translation",
            Self::Mat3Scale => "mat3_scale",
            Self::Mat3Rotation => "mat3_rotation",
            Self::Mat4Translation => "mat4_translation",
            Self::Mat4Scale => "mat4_scale",
            Self::Mat4RotationX => "mat4_rotation_x",
            Self::Mat4RotationY => "mat4_rotation_y",
            Self::Mat4RotationZ => "mat4_rotation_z",
            Self::Mat4Orthographic => "mat4_orthographic",
            Self::Mat4Perspective => "mat4_perspective",
            Self::Mat4LookAt => "mat4_look_at",
            Self::RngGaussian => "rng_gaussian",
            Self::RngExponential => "rng_exponential",
            Self::Perlin2 => "perlin2",
            Self::Turbulence2 => "turbulence2",
            Self::DomainWarp2 => "domain_warp2",
            Self::Mode => "mode",
            Self::Histogram => "histogram",
            Self::Ema => "ema",
            Self::IsEven => "is_even",
            Self::IsOdd => "is_odd",
            Self::PowInt => "pow_int",
            Self::VecMin => "vec_min",
            Self::VecMax => "vec_max",
            Self::ClampLength => "clamp_length",
            Self::QuatEuler => "quat_euler",
            Self::RayPlaneIntersection => "ray_plane_intersection",
        }
    }

    /// All natives in id order.
    pub fn all() -> &'static [NativeId] {
        &[
            Self::Print,
            Self::Abs,
            Self::Min,
            Self::Max,
            Self::Floor,
            Self::Ceil,
            Self::Clamp,
            Self::Len,
            Self::Concat,
            Self::Str,
            Self::Sin,
            Self::Cos,
            Self::Sqrt,
            Self::Pow,
            Self::Lerp,
            Self::HashSha256,
            Self::HexEncode,
            Self::Base64Encode,
            Self::PresentShow,
            Self::PresentSetBg,
            Self::PresentUiFlag,
            Self::PresentUiFlagGet,
            Self::PresentHide,
            Self::TypeOf,
            Self::ListPush,
            Self::ListPop,
            Self::MapHas,
            Self::MapKeys,
            Self::Assert,
            Self::Fail,
            Self::Tan,
            Self::Asin,
            Self::Acos,
            Self::Atan,
            Self::Atan2,
            Self::Exp,
            Self::Ln,
            Self::Log2,
            Self::Log10,
            Self::Cbrt,
            Self::Round,
            Self::Trunc,
            Self::Fract,
            Self::Sign,
            Self::Hypot,
            Self::Degrees,
            Self::Radians,
            Self::InverseLerp,
            Self::Remap,
            Self::Smoothstep,
            Self::IsFinite,
            Self::IsNan,
            Self::IsInfinite,
            Self::ApproxEq,
            Self::Gcd,
            Self::Lcm,
            Self::Vec2,
            Self::Vec3,
            Self::Vec4,
            Self::Dot,
            Self::Cross,
            Self::Length,
            Self::Normalize,
            Self::Distance,
            Self::AngleBetween,
            Self::Reflect,
            Self::Project,
            Self::VecLerp,
            Self::Mat3Identity,
            Self::Mat4Identity,
            Self::Mat3,
            Self::Mat4,
            Self::MatMul,
            Self::MatTranspose,
            Self::MatDeterminant,
            Self::MatInverse,
            Self::TransformPoint,
            Self::TransformVector,
            Self::Quat,
            Self::QuatIdentity,
            Self::QuatAxisAngle,
            Self::QuatMul,
            Self::QuatRotate,
            Self::QuatNormalize,
            Self::QuatInverse,
            Self::QuatSlerp,
            Self::RngNew,
            Self::RngNextFloat,
            Self::RngRangeInt,
            Self::RngRangeFloat,
            Self::RngBool,
            Self::Shuffle,
            Self::Choose,
            Self::WeightedChoose,
            Self::Noise1,
            Self::Noise2,
            Self::Fbm2,
            Self::Sum,
            Self::Product,
            Self::Mean,
            Self::Median,
            Self::Variance,
            Self::Stddev,
            Self::Quantile,
            Self::Covariance,
            Self::Correlation,
            Self::MovingAverage,
            Self::PolyEval,
            Self::IntegrateTrapezoid,
            Self::IntegrateSimpson,
            Self::PolyRootBisection,
            Self::QuadraticBezier,
            Self::CubicBezier,
            Self::CatmullRom,
            Self::Hermite,
            Self::ClosestPointSegment,
            Self::SegmentIntersection2,
            Self::Refract,
            Self::Mat3Translation,
            Self::Mat3Scale,
            Self::Mat3Rotation,
            Self::Mat4Translation,
            Self::Mat4Scale,
            Self::Mat4RotationX,
            Self::Mat4RotationY,
            Self::Mat4RotationZ,
            Self::Mat4Orthographic,
            Self::Mat4Perspective,
            Self::Mat4LookAt,
            Self::RngGaussian,
            Self::RngExponential,
            Self::Perlin2,
            Self::Turbulence2,
            Self::DomainWarp2,
            Self::Mode,
            Self::Histogram,
            Self::Ema,
            Self::IsEven,
            Self::IsOdd,
            Self::PowInt,
            Self::VecMin,
            Self::VecMax,
            Self::ClampLength,
            Self::QuatEuler,
            Self::RayPlaneIntersection,
        ]
    }
}

/// Resolve a script identifier to a native id.
pub fn lookup_native(name: &str) -> Option<NativeId> {
    match name {
        "print" => Some(NativeId::Print),
        "abs" => Some(NativeId::Abs),
        "min" => Some(NativeId::Min),
        "max" => Some(NativeId::Max),
        "floor" => Some(NativeId::Floor),
        "ceil" => Some(NativeId::Ceil),
        "clamp" => Some(NativeId::Clamp),
        "len" => Some(NativeId::Len),
        "concat" => Some(NativeId::Concat),
        "str" => Some(NativeId::Str),
        "sin" => Some(NativeId::Sin),
        "cos" => Some(NativeId::Cos),
        "sqrt" => Some(NativeId::Sqrt),
        "pow" => Some(NativeId::Pow),
        "lerp" => Some(NativeId::Lerp),
        "hash_sha256" => Some(NativeId::HashSha256),
        "hex_encode" => Some(NativeId::HexEncode),
        "base64_encode" => Some(NativeId::Base64Encode),
        "present_show" | "show_sprite" => Some(NativeId::PresentShow),
        "set_bg" => Some(NativeId::PresentSetBg),
        "ui_flag" => Some(NativeId::PresentUiFlag),
        "ui_flag_get" => Some(NativeId::PresentUiFlagGet),
        "present_hide" | "hide_sprite" => Some(NativeId::PresentHide),
        "type_of" => Some(NativeId::TypeOf),
        "list_push" => Some(NativeId::ListPush),
        "list_pop" => Some(NativeId::ListPop),
        "map_has" => Some(NativeId::MapHas),
        "map_keys" => Some(NativeId::MapKeys),
        "assert" => Some(NativeId::Assert),
        "fail" => Some(NativeId::Fail),
        "tan" => Some(NativeId::Tan),
        "asin" => Some(NativeId::Asin),
        "acos" => Some(NativeId::Acos),
        "atan" => Some(NativeId::Atan),
        "atan2" => Some(NativeId::Atan2),
        "exp" => Some(NativeId::Exp),
        "ln" => Some(NativeId::Ln),
        "log2" => Some(NativeId::Log2),
        "log10" => Some(NativeId::Log10),
        "cbrt" => Some(NativeId::Cbrt),
        "round" => Some(NativeId::Round),
        "trunc" => Some(NativeId::Trunc),
        "fract" => Some(NativeId::Fract),
        "sign" => Some(NativeId::Sign),
        "hypot" => Some(NativeId::Hypot),
        "degrees" => Some(NativeId::Degrees),
        "radians" => Some(NativeId::Radians),
        "inverse_lerp" => Some(NativeId::InverseLerp),
        "remap" => Some(NativeId::Remap),
        "smoothstep" => Some(NativeId::Smoothstep),
        "is_finite" => Some(NativeId::IsFinite),
        "is_nan" => Some(NativeId::IsNan),
        "is_infinite" => Some(NativeId::IsInfinite),
        "approx_eq" => Some(NativeId::ApproxEq),
        "gcd" => Some(NativeId::Gcd),
        "lcm" => Some(NativeId::Lcm),
        "vec2" => Some(NativeId::Vec2),
        "vec3" => Some(NativeId::Vec3),
        "vec4" => Some(NativeId::Vec4),
        "dot" => Some(NativeId::Dot),
        "cross" => Some(NativeId::Cross),
        "length" => Some(NativeId::Length),
        "normalize" => Some(NativeId::Normalize),
        "distance" => Some(NativeId::Distance),
        "angle_between" => Some(NativeId::AngleBetween),
        "reflect" => Some(NativeId::Reflect),
        "project" => Some(NativeId::Project),
        "vec_lerp" => Some(NativeId::VecLerp),
        "mat3_identity" => Some(NativeId::Mat3Identity),
        "mat4_identity" => Some(NativeId::Mat4Identity),
        "mat3" => Some(NativeId::Mat3),
        "mat4" => Some(NativeId::Mat4),
        "mat_mul" => Some(NativeId::MatMul),
        "mat_transpose" => Some(NativeId::MatTranspose),
        "mat_determinant" => Some(NativeId::MatDeterminant),
        "mat_inverse" => Some(NativeId::MatInverse),
        "transform_point" => Some(NativeId::TransformPoint),
        "transform_vector" => Some(NativeId::TransformVector),
        "quat" => Some(NativeId::Quat),
        "quat_identity" => Some(NativeId::QuatIdentity),
        "quat_axis_angle" => Some(NativeId::QuatAxisAngle),
        "quat_mul" => Some(NativeId::QuatMul),
        "quat_rotate" => Some(NativeId::QuatRotate),
        "quat_normalize" => Some(NativeId::QuatNormalize),
        "quat_inverse" => Some(NativeId::QuatInverse),
        "quat_slerp" => Some(NativeId::QuatSlerp),
        "rng_new" => Some(NativeId::RngNew),
        "rng_next_float" => Some(NativeId::RngNextFloat),
        "rng_range_int" => Some(NativeId::RngRangeInt),
        "rng_range_float" => Some(NativeId::RngRangeFloat),
        "rng_bool" => Some(NativeId::RngBool),
        "shuffle" => Some(NativeId::Shuffle),
        "choose" => Some(NativeId::Choose),
        "weighted_choose" => Some(NativeId::WeightedChoose),
        "noise1" => Some(NativeId::Noise1),
        "noise2" => Some(NativeId::Noise2),
        "fbm2" => Some(NativeId::Fbm2),
        "sum" => Some(NativeId::Sum),
        "product" => Some(NativeId::Product),
        "mean" => Some(NativeId::Mean),
        "median" => Some(NativeId::Median),
        "variance" => Some(NativeId::Variance),
        "stddev" => Some(NativeId::Stddev),
        "quantile" => Some(NativeId::Quantile),
        "covariance" => Some(NativeId::Covariance),
        "correlation" => Some(NativeId::Correlation),
        "moving_average" => Some(NativeId::MovingAverage),
        "poly_eval" => Some(NativeId::PolyEval),
        "integrate_trapezoid" => Some(NativeId::IntegrateTrapezoid),
        "integrate_simpson" => Some(NativeId::IntegrateSimpson),
        "poly_root_bisection" => Some(NativeId::PolyRootBisection),
        "quadratic_bezier" => Some(NativeId::QuadraticBezier),
        "cubic_bezier" => Some(NativeId::CubicBezier),
        "catmull_rom" => Some(NativeId::CatmullRom),
        "hermite" => Some(NativeId::Hermite),
        "closest_point_segment" => Some(NativeId::ClosestPointSegment),
        "segment_intersection2" => Some(NativeId::SegmentIntersection2),
        "refract" => Some(NativeId::Refract),
        "mat3_translation" => Some(NativeId::Mat3Translation),
        "mat3_scale" => Some(NativeId::Mat3Scale),
        "mat3_rotation" => Some(NativeId::Mat3Rotation),
        "mat4_translation" => Some(NativeId::Mat4Translation),
        "mat4_scale" => Some(NativeId::Mat4Scale),
        "mat4_rotation_x" => Some(NativeId::Mat4RotationX),
        "mat4_rotation_y" => Some(NativeId::Mat4RotationY),
        "mat4_rotation_z" => Some(NativeId::Mat4RotationZ),
        "mat4_orthographic" => Some(NativeId::Mat4Orthographic),
        "mat4_perspective" => Some(NativeId::Mat4Perspective),
        "mat4_look_at" => Some(NativeId::Mat4LookAt),
        "rng_gaussian" => Some(NativeId::RngGaussian),
        "rng_exponential" => Some(NativeId::RngExponential),
        "perlin2" => Some(NativeId::Perlin2),
        "turbulence2" => Some(NativeId::Turbulence2),
        "domain_warp2" => Some(NativeId::DomainWarp2),
        "mode" => Some(NativeId::Mode),
        "histogram" => Some(NativeId::Histogram),
        "ema" => Some(NativeId::Ema),
        "is_even" => Some(NativeId::IsEven),
        "is_odd" => Some(NativeId::IsOdd),
        "pow_int" => Some(NativeId::PowInt),
        "vec_min" => Some(NativeId::VecMin),
        "vec_max" => Some(NativeId::VecMax),
        "clamp_length" => Some(NativeId::ClampLength),
        "quat_euler" => Some(NativeId::QuatEuler),
        "ray_plane_intersection" => Some(NativeId::RayPlaneIntersection),
        _ => None,
    }
}

/// Resolve a reserved VS3 mathematical constant.
pub fn lookup_math_constant(name: &str) -> Option<f64> {
    Some(match name {
        "PI" => std::f64::consts::PI,
        "TAU" => std::f64::consts::TAU,
        "E" => std::f64::consts::E,
        "EPSILON" => f64::EPSILON,
        "INFINITY" => f64::INFINITY,
        "NAN" => f64::NAN,
        _ => return None,
    })
}

/// Coarse runtime value kinds used by native metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeType {
    /// Any value.
    Any,
    /// Integer or float.
    Number,
    /// Null.
    Null,
    /// Boolean.
    Bool,
    /// Signed integer.
    Int,
    /// Floating point.
    Float,
    /// String.
    String,
    /// List.
    List,
    /// Map.
    Map,
    /// Two-component vector.
    Vec2,
    /// Three-component vector.
    Vec3,
    /// Four-component vector.
    Vec4,
    /// Any vector dimension.
    Vector,
    /// 3x3 matrix.
    Mat3,
    /// 4x4 matrix.
    Mat4,
    /// Either matrix dimension.
    Matrix,
    /// Quaternion.
    Quat,
    /// Deterministic random stream.
    Rng,
}

/// Observable-effect classification for a native function.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativePurity {
    /// No mutation or output.
    Pure,
    /// Mutates state, prints, fails, or talks to presentation state.
    Impure,
}

/// Shared metadata consumed by semantic analysis, VM, and tooling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeSpec {
    /// Stable id.
    pub id: NativeId,
    /// Script-visible name.
    pub name: &'static str,
    /// Minimum arguments.
    pub min_args: u8,
    /// Maximum arguments.
    pub max_args: u8,
    /// Positional types. The final entry repeats for variadic calls.
    pub parameters: &'static [NativeType],
    /// Coarse result type.
    pub result: NativeType,
    /// Purity classification.
    pub purity: NativePurity,
    /// Base instruction charge.
    pub base_cost: u16,
}

impl NativeId {
    /// Shared native metadata.
    pub fn spec(self) -> NativeSpec {
        use NativeId as N;
        use NativePurity::{Impure, Pure};
        use NativeType as T;

        let (min_args, max_args) = match self {
            N::Print | N::Concat => (0, u8::MAX),
            N::Mat3Identity | N::Mat4Identity | N::QuatIdentity => (0, 0),
            N::PresentShow => (1, 3),
            N::Assert | N::Variance | N::Stddev => (1, 2),
            N::ApproxEq => (2, 3),
            N::Vec2 => (1, 2),
            N::Vec3 => (1, 3),
            N::Vec4 => (1, 4),
            N::Fbm2 | N::Turbulence2 => (4, 6),
            N::Covariance => (2, 3),
            N::PolyRootBisection => (3, 5),
            N::Mat3 => (9, 9),
            N::Mat4 => (16, 16),
            N::Remap | N::CubicBezier | N::CatmullRom | N::Hermite | N::DomainWarp2 => (5, 5),
            N::Mat4Orthographic => (6, 6),
            N::QuadraticBezier => (4, 4),
            N::Clamp
            | N::Lerp
            | N::InverseLerp
            | N::Smoothstep
            | N::VecLerp
            | N::RngRangeInt
            | N::RngRangeFloat
            | N::QuatSlerp
            | N::Noise2
            | N::ClosestPointSegment
            | N::Refract
            | N::Mat4LookAt
            | N::RngGaussian
            | N::Perlin2
            | N::QuatEuler => (3, 3),
            N::Quat | N::SegmentIntersection2 | N::Mat4Perspective | N::RayPlaneIntersection => {
                (4, 4)
            }
            N::Min
            | N::Max
            | N::Pow
            | N::Atan2
            | N::Hypot
            | N::Gcd
            | N::Lcm
            | N::Dot
            | N::Cross
            | N::Distance
            | N::AngleBetween
            | N::Reflect
            | N::Project
            | N::MatMul
            | N::TransformPoint
            | N::TransformVector
            | N::QuatAxisAngle
            | N::QuatMul
            | N::QuatRotate
            | N::RngBool
            | N::Shuffle
            | N::Choose
            | N::WeightedChoose
            | N::Noise1
            | N::Quantile
            | N::Correlation
            | N::MovingAverage
            | N::PolyEval
            | N::IntegrateTrapezoid
            | N::IntegrateSimpson
            | N::MapHas
            | N::ListPush
            | N::PresentUiFlag
            | N::RngExponential
            | N::Histogram
            | N::Ema
            | N::PowInt
            | N::VecMin
            | N::VecMax
            | N::ClampLength => (2, 2),
            _ => (1, 1),
        };

        let parameters: &'static [T] = match self {
            N::Print | N::Concat => &[T::Any],
            N::Min | N::Max | N::Pow | N::Atan2 | N::Hypot => &[T::Number, T::Number],
            N::Clamp | N::Lerp | N::InverseLerp | N::Smoothstep => {
                &[T::Number, T::Number, T::Number]
            }
            N::Remap => &[T::Number, T::Number, T::Number, T::Number, T::Number],
            N::Vec2 => &[T::Number, T::Number],
            N::Vec3 => &[T::Number, T::Number, T::Number],
            N::Vec4 | N::Quat => &[T::Number, T::Number, T::Number, T::Number],
            N::Dot | N::Cross | N::Distance | N::AngleBetween | N::Reflect | N::Project => {
                &[T::Vector, T::Vector]
            }
            N::Normalize | N::Length => &[T::Any],
            N::VecLerp => &[T::Vector, T::Vector, T::Number],
            N::Refract => &[T::Vector, T::Vector, T::Number],
            N::Mat3 | N::Mat4 => &[T::Number],
            N::MatMul => &[T::Matrix, T::Matrix],
            N::MatTranspose | N::MatDeterminant | N::MatInverse => &[T::Matrix],
            N::Mat3Translation | N::Mat3Scale => &[T::Vec2],
            N::Mat3Rotation | N::Mat4RotationX | N::Mat4RotationY | N::Mat4RotationZ => {
                &[T::Number]
            }
            N::Mat4Translation | N::Mat4Scale => &[T::Vec3],
            N::Mat4Orthographic => &[
                T::Number,
                T::Number,
                T::Number,
                T::Number,
                T::Number,
                T::Number,
            ],
            N::Mat4Perspective => &[T::Number, T::Number, T::Number, T::Number],
            N::Mat4LookAt => &[T::Vec3, T::Vec3, T::Vec3],
            N::TransformPoint | N::TransformVector => &[T::Matrix, T::Vector],
            N::QuatAxisAngle => &[T::Vec3, T::Number],
            N::QuatMul => &[T::Quat, T::Quat],
            N::QuatRotate => &[T::Quat, T::Vec3],
            N::QuatNormalize | N::QuatInverse => &[T::Quat],
            N::QuatSlerp => &[T::Quat, T::Quat, T::Number],
            N::RngNew => &[T::Int],
            N::RngNextFloat => &[T::Rng],
            N::RngRangeInt => &[T::Rng, T::Int, T::Int],
            N::RngRangeFloat => &[T::Rng, T::Number, T::Number],
            N::RngBool => &[T::Rng, T::Number],
            N::RngGaussian => &[T::Rng, T::Number, T::Number],
            N::RngExponential => &[T::Rng, T::Number],
            N::Shuffle | N::Choose | N::WeightedChoose => &[T::Rng, T::List],
            N::Noise1 => &[T::Number, T::Int],
            N::Noise2 => &[T::Number, T::Number, T::Int],
            N::Perlin2 => &[T::Number, T::Number, T::Int],
            N::Fbm2 => &[T::Number, T::Number, T::Int, T::Int, T::Number, T::Number],
            N::Turbulence2 => &[T::Number, T::Number, T::Int, T::Int, T::Number, T::Number],
            N::DomainWarp2 => &[T::Number, T::Number, T::Int, T::Int, T::Number],
            N::Sum | N::Product | N::Mean | N::Median => &[T::List],
            N::Variance | N::Stddev => &[T::List, T::Bool],
            N::Quantile => &[T::List, T::Number],
            N::Covariance => &[T::List, T::List, T::Bool],
            N::Correlation => &[T::List, T::List],
            N::MovingAverage => &[T::List, T::Int],
            N::Mode => &[T::List],
            N::Histogram => &[T::List, T::Int],
            N::Ema => &[T::List, T::Number],
            N::IsEven | N::IsOdd => &[T::Int],
            N::PowInt => &[T::Int, T::Int],
            N::VecMin | N::VecMax => &[T::Vector, T::Vector],
            N::ClampLength => &[T::Vector, T::Number],
            N::QuatEuler => &[T::Number, T::Number, T::Number],
            N::PolyEval => &[T::List, T::Number],
            N::IntegrateTrapezoid | N::IntegrateSimpson => &[T::List, T::Number],
            N::PolyRootBisection => &[T::List, T::Number, T::Number, T::Int, T::Number],
            N::QuadraticBezier => &[T::Vector, T::Vector, T::Vector, T::Number],
            N::CubicBezier | N::CatmullRom | N::Hermite => {
                &[T::Vector, T::Vector, T::Vector, T::Vector, T::Number]
            }
            N::ClosestPointSegment => &[T::Vector, T::Vector, T::Vector],
            N::SegmentIntersection2 => &[T::Vec2, T::Vec2, T::Vec2, T::Vec2],
            N::RayPlaneIntersection => &[T::Vector, T::Vector, T::Vector, T::Vector],
            _ => &[T::Any],
        };

        let result = match self {
            N::Print
            | N::PresentShow
            | N::PresentSetBg
            | N::PresentUiFlag
            | N::PresentHide
            | N::Assert
            | N::Fail => T::Null,
            N::Str | N::Concat | N::HashSha256 | N::HexEncode | N::Base64Encode | N::TypeOf => {
                T::String
            }
            N::IsFinite
            | N::IsNan
            | N::IsInfinite
            | N::ApproxEq
            | N::RngBool
            | N::MapHas
            | N::IsEven
            | N::IsOdd => T::Bool,
            N::Len | N::Gcd | N::Lcm | N::RngRangeInt | N::WeightedChoose | N::PowInt => T::Int,
            N::Abs | N::Min | N::Max | N::Clamp => T::Number,
            N::Vec2 => T::Vec2,
            N::Vec3 | N::QuatRotate => T::Vec3,
            N::Vec4 => T::Vec4,
            N::Reflect
            | N::Project
            | N::VecLerp
            | N::QuadraticBezier
            | N::CubicBezier
            | N::CatmullRom
            | N::Hermite
            | N::ClosestPointSegment
            | N::TransformPoint
            | N::TransformVector
            | N::Refract
            | N::VecMin
            | N::VecMax
            | N::ClampLength => T::Vector,
            N::Mat3Identity | N::Mat3 | N::Mat3Translation | N::Mat3Scale | N::Mat3Rotation => {
                T::Mat3
            }
            N::Mat4Identity
            | N::Mat4
            | N::Mat4Translation
            | N::Mat4Scale
            | N::Mat4RotationX
            | N::Mat4RotationY
            | N::Mat4RotationZ
            | N::Mat4Orthographic
            | N::Mat4Perspective
            | N::Mat4LookAt => T::Mat4,
            N::MatMul | N::MatTranspose | N::MatInverse => T::Matrix,
            N::Quat
            | N::QuatIdentity
            | N::QuatAxisAngle
            | N::QuatMul
            | N::QuatNormalize
            | N::QuatInverse
            | N::QuatSlerp => T::Quat,
            N::QuatEuler => T::Quat,
            N::RngNew => T::Rng,
            N::ListPush | N::MapKeys | N::Shuffle | N::MovingAverage | N::Histogram | N::Ema => {
                T::List
            }
            N::ListPop
            | N::Choose
            | N::Normalize
            | N::Cross
            | N::SegmentIntersection2
            | N::RayPlaneIntersection => T::Any,
            _ => T::Float,
        };

        let purity = if matches!(
            self,
            N::Print
                | N::PresentShow
                | N::PresentSetBg
                | N::PresentUiFlag
                | N::PresentUiFlagGet
                | N::PresentHide
                | N::ListPush
                | N::ListPop
                | N::Assert
                | N::Fail
                | N::RngNextFloat
                | N::RngRangeInt
                | N::RngRangeFloat
                | N::RngBool
                | N::RngGaussian
                | N::RngExponential
                | N::Shuffle
                | N::Choose
                | N::WeightedChoose
        ) {
            Impure
        } else {
            Pure
        };
        let base_cost = match self {
            N::MatInverse | N::PolyRootBisection | N::Fbm2 | N::Turbulence2 | N::DomainWarp2 => 24,
            N::MatMul | N::QuatSlerp | N::Correlation => 8,
            N::Noise2
            | N::Perlin2
            | N::CubicBezier
            | N::CatmullRom
            | N::Hermite
            | N::IntegrateSimpson => 7,
            N::Median
            | N::Variance
            | N::Stddev
            | N::Quantile
            | N::Covariance
            | N::MovingAverage
            | N::TransformPoint
            | N::TransformVector => 5,
            N::Sin
            | N::Cos
            | N::Tan
            | N::Asin
            | N::Acos
            | N::Atan
            | N::Atan2
            | N::Exp
            | N::Ln
            | N::Log2
            | N::Log10
            | N::Pow
            | N::Sqrt => 3,
            _ => 1,
        };

        NativeSpec {
            id: self,
            name: self.name(),
            min_args,
            max_args,
            parameters,
            result,
            purity,
            base_cost,
        }
    }
}

/// Bytecode opcodes (u8).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Op {
    /// Push constant by u16 index.
    Constant = 1,
    /// Push null.
    Null = 2,
    /// Push true.
    True = 3,
    /// Push false.
    False = 4,
    /// Pop top.
    Pop = 5,
    /// Define global from top (name index u16).
    DefineGlobal = 6,
    /// Get global (name index u16).
    GetGlobal = 7,
    /// Set global (name index u16).
    SetGlobal = 8,
    /// Get local slot u8.
    GetLocal = 9,
    /// Set local slot u8.
    SetLocal = 10,
    /// Add.
    Add = 11,
    /// Subtract.
    Sub = 12,
    /// Multiply.
    Mul = 13,
    /// Divide.
    Div = 14,
    /// Remainder.
    Rem = 15,
    /// Negate.
    Neg = 16,
    /// Not.
    Not = 17,
    /// Equal.
    Eq = 18,
    /// Not equal.
    Ne = 19,
    /// Less.
    Lt = 20,
    /// Less equal.
    Le = 21,
    /// Greater.
    Gt = 22,
    /// Greater equal.
    Ge = 23,
    /// Jump forward u16 bytes.
    Jump = 24,
    /// Jump if false u16, keeps value.
    JumpIfFalse = 25,
    /// Jump if true u16.
    JumpIfTrue = 26,
    /// Jump backward u16.
    Loop = 27,
    /// Call with u8 argc.
    Call = 28,
    /// Return top (or null).
    Return = 29,
    /// Build list with u16 count.
    MakeList = 30,
    /// Print top (debug host).
    Print = 31,
    /// Halt VM.
    Halt = 32,
    /// Duplicate top.
    Dup = 33,
    /// Call native host function: u16 native_id, u8 argc.
    NativeCall = 34,
    /// Yield top value to host / coroutine (cooperative).
    Yield = 35,
    /// Index get: stack `[.., list|map|str, index]` → value.
    GetIndex = 36,
    /// Index set: stack `[.., container, index, value]` → value (also stores).
    SetIndex = 37,
    /// Length of string or list (pops one, pushes int).
    Len = 38,
    /// Build map with u16 entry count (2*count stack values: k,v pairs).
    MakeMap = 39,
    /// Compound index update: container, index, rhs; followed by arithmetic opcode byte.
    UpdateIndex = 40,
}

impl Op {
    /// From raw byte.
    pub fn from_u8(b: u8) -> Option<Self> {
        Some(match b {
            1 => Self::Constant,
            2 => Self::Null,
            3 => Self::True,
            4 => Self::False,
            5 => Self::Pop,
            6 => Self::DefineGlobal,
            7 => Self::GetGlobal,
            8 => Self::SetGlobal,
            9 => Self::GetLocal,
            10 => Self::SetLocal,
            11 => Self::Add,
            12 => Self::Sub,
            13 => Self::Mul,
            14 => Self::Div,
            15 => Self::Rem,
            16 => Self::Neg,
            17 => Self::Not,
            18 => Self::Eq,
            19 => Self::Ne,
            20 => Self::Lt,
            21 => Self::Le,
            22 => Self::Gt,
            23 => Self::Ge,
            24 => Self::Jump,
            25 => Self::JumpIfFalse,
            26 => Self::JumpIfTrue,
            27 => Self::Loop,
            28 => Self::Call,
            29 => Self::Return,
            30 => Self::MakeList,
            31 => Self::Print,
            32 => Self::Halt,
            33 => Self::Dup,
            34 => Self::NativeCall,
            35 => Self::Yield,
            36 => Self::GetIndex,
            37 => Self::SetIndex,
            38 => Self::Len,
            39 => Self::MakeMap,
            40 => Self::UpdateIndex,
            _ => return None,
        })
    }

    /// Encode as raw byte.
    pub fn to_u8(self) -> u8 {
        self as u8
    }

    /// Mnemonic for disassembly.
    pub fn mnemonic(self) -> &'static str {
        match self {
            Self::Constant => "CONSTANT",
            Self::Null => "NULL",
            Self::True => "TRUE",
            Self::False => "FALSE",
            Self::Pop => "POP",
            Self::DefineGlobal => "DEFINE_GLOBAL",
            Self::GetGlobal => "GET_GLOBAL",
            Self::SetGlobal => "SET_GLOBAL",
            Self::GetLocal => "GET_LOCAL",
            Self::SetLocal => "SET_LOCAL",
            Self::Add => "ADD",
            Self::Sub => "SUB",
            Self::Mul => "MUL",
            Self::Div => "DIV",
            Self::Rem => "REM",
            Self::Neg => "NEG",
            Self::Not => "NOT",
            Self::Eq => "EQ",
            Self::Ne => "NE",
            Self::Lt => "LT",
            Self::Le => "LE",
            Self::Gt => "GT",
            Self::Ge => "GE",
            Self::Jump => "JUMP",
            Self::JumpIfFalse => "JUMP_IF_FALSE",
            Self::JumpIfTrue => "JUMP_IF_TRUE",
            Self::Loop => "LOOP",
            Self::Call => "CALL",
            Self::Return => "RETURN",
            Self::MakeList => "MAKE_LIST",
            Self::Print => "PRINT",
            Self::Halt => "HALT",
            Self::Dup => "DUP",
            Self::NativeCall => "NATIVE_CALL",
            Self::Yield => "YIELD",
            Self::GetIndex => "GET_INDEX",
            Self::SetIndex => "SET_INDEX",
            Self::Len => "LEN",
            Self::MakeMap => "MAKE_MAP",
            Self::UpdateIndex => "UPDATE_INDEX",
        }
    }

    /// Operand size in bytes following the opcode.
    pub fn operand_len(self) -> usize {
        match self {
            Self::Constant
            | Self::DefineGlobal
            | Self::GetGlobal
            | Self::SetGlobal
            | Self::Jump
            | Self::JumpIfFalse
            | Self::JumpIfTrue
            | Self::Loop
            | Self::MakeList
            | Self::MakeMap => 2,
            Self::GetLocal | Self::SetLocal | Self::Call | Self::UpdateIndex => 1,
            Self::NativeCall => 3, // u16 id + u8 argc
            _ => 0,
        }
    }
}

/// Constant pool values.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Constant {
    /// Null.
    Null,
    /// Bool.
    Bool(bool),
    /// Integer.
    Int(i64),
    /// Float.
    Float(f64),
    /// String.
    String(String),
    /// Function index into module.functions.
    Function(u16),
    /// Native host function id.
    Native(u16),
}

/// Source mapping for a bytecode offset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SourceMapEntry {
    /// Bytecode offset.
    pub offset: u32,
    /// Line.
    pub line: u32,
    /// Column.
    pub column: u32,
}

/// Compact line-table row: bytecode range mapped to one source line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct LineTableEntry {
    /// Inclusive start offset.
    pub start: u32,
    /// Exclusive end offset.
    pub end: u32,
    /// Source line (1-based).
    pub line: u32,
    /// Source column (1-based) at range start.
    pub column: u32,
}

/// Module-level metadata for tooling and versioning.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ModuleMetadata {
    /// Bytecode format version.
    pub version: u16,
    /// Optional source path.
    pub source_path: Option<String>,
    /// Optional content hash (FNV-1a 64 of source bytes when known).
    pub source_hash: Option<u64>,
    /// Free-form compiler / tool stamp.
    pub compiler: Option<String>,
}

impl ModuleMetadata {
    /// Metadata for the current bytecode format.
    pub fn current() -> Self {
        Self {
            version: BYTECODE_VERSION,
            source_path: None,
            source_hash: None,
            compiler: Some("velvet-script-compiler".into()),
        }
    }

    /// Attach source path and optional hash.
    pub fn with_source(mut self, path: impl Into<String>, hash: Option<u64>) -> Self {
        self.source_path = Some(path.into());
        self.source_hash = hash;
        self
    }
}

/// FNV-1a 64-bit hash of bytes (stable, no deps).
pub fn fnv1a64(bytes: &[u8]) -> u64 {
    const OFFSET: u64 = 0xcbf29ce484222325;
    const PRIME: u64 = 0x100000001b3;
    let mut h = OFFSET;
    for b in bytes {
        h ^= u64::from(*b);
        h = h.wrapping_mul(PRIME);
    }
    h
}

/// A compiled function / chunk.
#[derive(Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Chunk {
    /// Function name.
    pub name: String,
    /// Arity.
    pub arity: u8,
    /// Local slot count (including params).
    pub locals: u8,
    /// Bytecode.
    pub code: Vec<u8>,
    /// Constants.
    pub constants: Vec<Constant>,
    /// Source map (sparse).
    pub source_map: Vec<SourceMapEntry>,
}

impl Chunk {
    /// Create empty named chunk.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            arity: 0,
            locals: 0,
            code: Vec::new(),
            constants: Vec::new(),
            source_map: Vec::new(),
        }
    }

    /// Emit opcode.
    pub fn emit_op(&mut self, op: Op) {
        self.code.push(op as u8);
    }

    /// Emit u8.
    pub fn emit_u8(&mut self, v: u8) {
        self.code.push(v);
    }

    /// Emit u16 little-endian.
    pub fn emit_u16(&mut self, v: u16) {
        self.code.extend_from_slice(&v.to_le_bytes());
    }

    /// Current code length.
    pub fn len(&self) -> usize {
        self.code.len()
    }

    /// Empty.
    pub fn is_empty(&self) -> bool {
        self.code.is_empty()
    }

    /// Add constant; return index.
    pub fn add_constant(&mut self, c: Constant) -> u16 {
        // Intern simple duplicates for strings/ints.
        for (i, existing) in self.constants.iter().enumerate() {
            if existing == &c {
                return i as u16;
            }
        }
        let idx = self.constants.len() as u16;
        self.constants.push(c);
        idx
    }

    /// Emit constant op.
    pub fn emit_constant(&mut self, c: Constant) {
        let idx = self.add_constant(c);
        self.emit_op(Op::Constant);
        self.emit_u16(idx);
    }

    /// Emit `NativeCall` with native id and argc.
    pub fn emit_native_call(&mut self, native_id: u16, argc: u8) {
        self.emit_op(Op::NativeCall);
        self.emit_u16(native_id);
        self.emit_u8(argc);
    }

    /// Patch u16 at offset.
    pub fn patch_u16(&mut self, offset: usize, value: u16) {
        let bytes = value.to_le_bytes();
        self.code[offset] = bytes[0];
        self.code[offset + 1] = bytes[1];
    }

    /// Record source location at current offset.
    pub fn map_source(&mut self, line: u32, column: u32) {
        let offset = self.code.len() as u32;
        if let Some(last) = self.source_map.last() {
            if last.offset == offset {
                return;
            }
        }
        self.source_map.push(SourceMapEntry {
            offset,
            line,
            column,
        });
    }

    /// Lookup line/col for bytecode offset.
    pub fn location_at(&self, offset: usize) -> Option<(u32, u32)> {
        let mut best = None;
        for e in &self.source_map {
            if e.offset as usize <= offset {
                best = Some((e.line, e.column));
            } else {
                break;
            }
        }
        best
    }

    /// Build a compact line table covering the whole code buffer.
    ///
    /// Consecutive offsets that share the same line are merged.
    pub fn line_table(&self) -> Vec<LineTableEntry> {
        if self.code.is_empty() {
            return Vec::new();
        }
        let mut table = Vec::new();
        let mut cursor = 0u32;
        let mut map_i = 0usize;
        let mut cur_line = 1u32;
        let mut cur_col = 1u32;
        // Apply any entry at offset 0.
        while map_i < self.source_map.len() && self.source_map[map_i].offset == 0 {
            cur_line = self.source_map[map_i].line;
            cur_col = self.source_map[map_i].column;
            map_i += 1;
        }
        let code_len = self.code.len() as u32;
        while cursor < code_len {
            let next_change = self
                .source_map
                .get(map_i)
                .map(|e| e.offset)
                .unwrap_or(code_len);
            let end = next_change.min(code_len);
            if end > cursor {
                table.push(LineTableEntry {
                    start: cursor,
                    end,
                    line: cur_line,
                    column: cur_col,
                });
            }
            cursor = end;
            if map_i < self.source_map.len() && self.source_map[map_i].offset == cursor {
                cur_line = self.source_map[map_i].line;
                cur_col = self.source_map[map_i].column;
                map_i += 1;
            } else if cursor < code_len && next_change == cursor {
                // advance past duplicate
                map_i += 1;
            }
            // If stuck (no progress), advance one byte.
            if end == cursor && cursor < code_len {
                // consume remaining map entries at this offset
                while map_i < self.source_map.len() && self.source_map[map_i].offset == cursor {
                    cur_line = self.source_map[map_i].line;
                    cur_col = self.source_map[map_i].column;
                    map_i += 1;
                }
                let next = self
                    .source_map
                    .get(map_i)
                    .map(|e| e.offset)
                    .unwrap_or(code_len);
                if next > cursor {
                    table.push(LineTableEntry {
                        start: cursor,
                        end: next,
                        line: cur_line,
                        column: cur_col,
                    });
                    cursor = next;
                } else {
                    break;
                }
            }
        }
        // Merge adjacent same-line ranges.
        let mut merged: Vec<LineTableEntry> = Vec::new();
        for e in table {
            if let Some(last) = merged.last_mut() {
                if last.line == e.line && last.end == e.start {
                    last.end = e.end;
                    continue;
                }
            }
            merged.push(e);
        }
        merged
    }

    /// Line number for each bytecode byte (dense). Length equals `code.len()`.
    pub fn dense_line_table(&self) -> Vec<u32> {
        let mut lines = vec![1u32; self.code.len()];
        let table = self.line_table();
        for e in table {
            let start = e.start as usize;
            let end = (e.end as usize).min(lines.len());
            for slot in lines.iter_mut().take(end).skip(start) {
                *slot = e.line;
            }
        }
        lines
    }

    /// Encode chunk header + code + constants into a simple binary blob.
    ///
    /// Layout (little-endian):
    /// - magic `VCHK` (4)
    /// - arity u8, locals u8, name_len u16, name bytes
    /// - const_count u16, then each constant
    /// - code_len u32, code bytes
    /// - map_count u16, then (offset u32, line u32, column u32)*
    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(b"VCHK");
        buf.push(self.arity);
        buf.push(self.locals);
        let name_bytes = self.name.as_bytes();
        buf.extend_from_slice(&(name_bytes.len() as u16).to_le_bytes());
        buf.extend_from_slice(name_bytes);
        buf.extend_from_slice(&(self.constants.len() as u16).to_le_bytes());
        for c in &self.constants {
            encode_constant(c, &mut buf);
        }
        buf.extend_from_slice(&(self.code.len() as u32).to_le_bytes());
        buf.extend_from_slice(&self.code);
        buf.extend_from_slice(&(self.source_map.len() as u16).to_le_bytes());
        for e in &self.source_map {
            buf.extend_from_slice(&e.offset.to_le_bytes());
            buf.extend_from_slice(&e.line.to_le_bytes());
            buf.extend_from_slice(&e.column.to_le_bytes());
        }
        buf
    }

    /// Decode a chunk previously produced by [`Chunk::encode`].
    pub fn decode(bytes: &[u8]) -> Result<Self, String> {
        let mut i = 0usize;
        let read = |i: &mut usize, n: usize| -> Result<&[u8], String> {
            if *i + n > bytes.len() {
                return Err("truncated chunk blob".into());
            }
            let s = &bytes[*i..*i + n];
            *i += n;
            Ok(s)
        };
        if read(&mut i, 4)? != b"VCHK" {
            return Err("bad chunk magic".into());
        }
        let arity = read(&mut i, 1)?[0];
        let locals = read(&mut i, 1)?[0];
        let name_len = u16::from_le_bytes(read(&mut i, 2)?.try_into().unwrap()) as usize;
        let name = String::from_utf8(read(&mut i, name_len)?.to_vec())
            .map_err(|_| "invalid chunk name utf8".to_string())?;
        let const_count = u16::from_le_bytes(read(&mut i, 2)?.try_into().unwrap()) as usize;
        let mut constants = Vec::with_capacity(const_count);
        for _ in 0..const_count {
            constants.push(decode_constant(bytes, &mut i)?);
        }
        let code_len = u32::from_le_bytes(read(&mut i, 4)?.try_into().unwrap()) as usize;
        let code = read(&mut i, code_len)?.to_vec();
        let map_count = u16::from_le_bytes(read(&mut i, 2)?.try_into().unwrap()) as usize;
        let mut source_map = Vec::with_capacity(map_count);
        for _ in 0..map_count {
            let offset = u32::from_le_bytes(read(&mut i, 4)?.try_into().unwrap());
            let line = u32::from_le_bytes(read(&mut i, 4)?.try_into().unwrap());
            let column = u32::from_le_bytes(read(&mut i, 4)?.try_into().unwrap());
            source_map.push(SourceMapEntry {
                offset,
                line,
                column,
            });
        }
        Ok(Self {
            name,
            arity,
            locals,
            code,
            constants,
            source_map,
        })
    }
}

fn encode_constant(c: &Constant, buf: &mut Vec<u8>) {
    match c {
        Constant::Null => buf.push(0),
        Constant::Bool(b) => {
            buf.push(1);
            buf.push(u8::from(*b));
        }
        Constant::Int(i) => {
            buf.push(2);
            buf.extend_from_slice(&i.to_le_bytes());
        }
        Constant::Float(f) => {
            buf.push(3);
            buf.extend_from_slice(&f.to_bits().to_le_bytes());
        }
        Constant::String(s) => {
            buf.push(4);
            let b = s.as_bytes();
            buf.extend_from_slice(&(b.len() as u32).to_le_bytes());
            buf.extend_from_slice(b);
        }
        Constant::Function(idx) => {
            buf.push(5);
            buf.extend_from_slice(&idx.to_le_bytes());
        }
        Constant::Native(idx) => {
            buf.push(6);
            buf.extend_from_slice(&idx.to_le_bytes());
        }
    }
}

fn decode_constant(bytes: &[u8], i: &mut usize) -> Result<Constant, String> {
    if *i >= bytes.len() {
        return Err("truncated constant".into());
    }
    let tag = bytes[*i];
    *i += 1;
    match tag {
        0 => Ok(Constant::Null),
        1 => {
            if *i >= bytes.len() {
                return Err("truncated bool const".into());
            }
            let b = bytes[*i] != 0;
            *i += 1;
            Ok(Constant::Bool(b))
        }
        2 => {
            if *i + 8 > bytes.len() {
                return Err("truncated int const".into());
            }
            let v = i64::from_le_bytes(bytes[*i..*i + 8].try_into().unwrap());
            *i += 8;
            Ok(Constant::Int(v))
        }
        3 => {
            if *i + 8 > bytes.len() {
                return Err("truncated float const".into());
            }
            let bits = u64::from_le_bytes(bytes[*i..*i + 8].try_into().unwrap());
            *i += 8;
            Ok(Constant::Float(f64::from_bits(bits)))
        }
        4 => {
            if *i + 4 > bytes.len() {
                return Err("truncated string len".into());
            }
            let len = u32::from_le_bytes(bytes[*i..*i + 4].try_into().unwrap()) as usize;
            *i += 4;
            if *i + len > bytes.len() {
                return Err("truncated string bytes".into());
            }
            let s = String::from_utf8(bytes[*i..*i + len].to_vec())
                .map_err(|_| "invalid string utf8".to_string())?;
            *i += len;
            Ok(Constant::String(s))
        }
        5 => {
            if *i + 2 > bytes.len() {
                return Err("truncated function const".into());
            }
            let v = u16::from_le_bytes(bytes[*i..*i + 2].try_into().unwrap());
            *i += 2;
            Ok(Constant::Function(v))
        }
        6 => {
            if *i + 2 > bytes.len() {
                return Err("truncated native const".into());
            }
            let v = u16::from_le_bytes(bytes[*i..*i + 2].try_into().unwrap());
            *i += 2;
            Ok(Constant::Native(v))
        }
        other => Err(format!("unknown constant tag {other}")),
    }
}

/// Compiled module.
#[derive(Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct BytecodeModule {
    /// Optional file name.
    pub file: Option<String>,
    /// Module metadata (version, hash, tool stamp).
    pub metadata: ModuleMetadata,
    /// Functions (index 0 is often `<script>` main).
    pub functions: Vec<Chunk>,
    /// Global names in definition order.
    pub globals: Vec<String>,
    /// Name → function index for exports.
    pub exports: HashMap<String, u16>,
}

impl BytecodeModule {
    /// Create empty module.
    pub fn new() -> Self {
        Self {
            metadata: ModuleMetadata::current(),
            ..Default::default()
        }
    }

    /// Main script chunk index if present.
    pub fn main_index(&self) -> Option<u16> {
        self.exports
            .get("<script>")
            .copied()
            .or(if self.functions.is_empty() {
                None
            } else {
                Some(0)
            })
    }

    /// Lookup export by name.
    pub fn export(&self, name: &str) -> Option<u16> {
        self.exports.get(name).copied()
    }
}

/// Pretty-print a single chunk as a disassembly listing.
pub fn disassemble_chunk(chunk: &Chunk) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "== {} (arity={}, locals={}) ==",
        chunk.name, chunk.arity, chunk.locals
    );
    if !chunk.constants.is_empty() {
        let _ = writeln!(out, "; constants:");
        for (i, c) in chunk.constants.iter().enumerate() {
            let _ = writeln!(out, ";   [{i}] {}", format_constant(c));
        }
    }
    let mut ip = 0usize;
    while ip < chunk.code.len() {
        let line_note = chunk
            .location_at(ip)
            .map(|(l, c)| format!(" ; L{l}:{c}"))
            .unwrap_or_default();
        let start = ip;
        let op_byte = chunk.code[ip];
        ip += 1;
        let Some(op) = Op::from_u8(op_byte) else {
            let _ = writeln!(out, "{start:04X}  ??? {op_byte:02X}{line_note}");
            continue;
        };
        match op {
            Op::Constant => {
                let idx = read_u16(&chunk.code, &mut ip);
                let pretty = chunk
                    .constants
                    .get(idx as usize)
                    .map(format_constant)
                    .unwrap_or_else(|| "<bad>".into());
                let _ = writeln!(
                    out,
                    "{start:04X}  {} {}  ({}){line_note}",
                    op.mnemonic(),
                    idx,
                    pretty
                );
            }
            Op::DefineGlobal | Op::GetGlobal | Op::SetGlobal => {
                let idx = read_u16(&chunk.code, &mut ip);
                let _ = writeln!(out, "{start:04X}  {} {}{line_note}", op.mnemonic(), idx);
            }
            Op::Jump | Op::JumpIfFalse | Op::JumpIfTrue | Op::Loop | Op::MakeList | Op::MakeMap => {
                let off = read_u16(&chunk.code, &mut ip);
                let _ = writeln!(out, "{start:04X}  {} {}{line_note}", op.mnemonic(), off);
            }
            Op::GetLocal | Op::SetLocal | Op::Call => {
                let slot = chunk.code.get(ip).copied().unwrap_or(0);
                ip += 1;
                let _ = writeln!(out, "{start:04X}  {} {}{line_note}", op.mnemonic(), slot);
            }
            Op::UpdateIndex => {
                let arithmetic = chunk.code.get(ip).copied().unwrap_or(0);
                ip += 1;
                let arithmetic = Op::from_u8(arithmetic).map(Op::mnemonic).unwrap_or("<bad>");
                let _ = writeln!(
                    out,
                    "{start:04X}  {} {}{line_note}",
                    op.mnemonic(),
                    arithmetic
                );
            }
            Op::NativeCall => {
                let id = read_u16(&chunk.code, &mut ip);
                let argc = chunk.code.get(ip).copied().unwrap_or(0);
                ip += 1;
                let _ = writeln!(
                    out,
                    "{start:04X}  {} id={} argc={}{line_note}",
                    op.mnemonic(),
                    id,
                    argc
                );
            }
            _ => {
                let _ = writeln!(out, "{start:04X}  {}{line_note}", op.mnemonic());
            }
        }
    }
    out
}

/// Disassemble every function in a module.
pub fn disassemble_module(module: &BytecodeModule) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "; module file={:?} version={}",
        module.file, module.metadata.version
    );
    if let Some(h) = module.metadata.source_hash {
        let _ = writeln!(out, "; source_hash={h:#x}");
    }
    let _ = writeln!(out, "; globals: {:?}", module.globals);
    let _ = writeln!(
        out,
        "; exports: {:?}",
        module.exports.keys().collect::<Vec<_>>()
    );
    for (i, chunk) in module.functions.iter().enumerate() {
        let _ = writeln!(out, "\n; --- function [{i}] ---");
        out.push_str(&disassemble_chunk(chunk));
    }
    out
}

fn read_u16(code: &[u8], ip: &mut usize) -> u16 {
    let lo = code.get(*ip).copied().unwrap_or(0) as u16;
    let hi = code.get(*ip + 1).copied().unwrap_or(0) as u16;
    *ip += 2;
    lo | (hi << 8)
}

fn format_constant(c: &Constant) -> String {
    match c {
        Constant::Null => "null".into(),
        Constant::Bool(b) => b.to_string(),
        Constant::Int(i) => i.to_string(),
        Constant::Float(f) => f.to_string(),
        Constant::String(s) => format!("\"{s}\""),
        Constant::Function(i) => format!("fn#{i}"),
        Constant::Native(i) => format!("native#{i}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emit_and_patch() {
        let mut c = Chunk::new("test");
        c.emit_op(Op::Jump);
        let at = c.len();
        c.emit_u16(0);
        c.patch_u16(at, 10);
        assert_eq!(c.code[at], 10);
        assert_eq!(c.code[at + 1], 0);
    }

    #[test]
    fn constant_intern() {
        let mut c = Chunk::new("t");
        let a = c.add_constant(Constant::Int(1));
        let b = c.add_constant(Constant::Int(1));
        assert_eq!(a, b);
        assert_eq!(c.constants.len(), 1);
    }

    #[test]
    fn op_roundtrip_all() {
        for b in 1u8..=40 {
            let op = Op::from_u8(b).expect("op defined");
            assert_eq!(op.to_u8(), b);
            assert!(!op.mnemonic().is_empty());
        }
        assert!(Op::from_u8(0).is_none());
        assert!(Op::from_u8(41).is_none());
    }

    #[test]
    fn encode_decode_chunk() {
        let mut c = Chunk::new("fold");
        c.arity = 2;
        c.locals = 3;
        c.map_source(1, 1);
        c.emit_constant(Constant::Int(42));
        c.emit_constant(Constant::String("hi".into()));
        c.emit_constant(Constant::Native(3));
        c.emit_op(Op::Add);
        c.emit_native_call(1, 2);
        c.map_source(2, 4);
        c.emit_op(Op::Return);
        let bytes = c.encode();
        let d = Chunk::decode(&bytes).unwrap();
        assert_eq!(d.name, "fold");
        assert_eq!(d.arity, 2);
        assert_eq!(d.locals, 3);
        assert_eq!(d.constants, c.constants);
        assert_eq!(d.code, c.code);
        assert_eq!(d.source_map, c.source_map);
    }

    #[test]
    fn disassemble_contains_mnemonics() {
        let mut c = Chunk::new("main");
        c.emit_constant(Constant::Int(1));
        c.emit_op(Op::Print);
        c.emit_op(Op::Return);
        let text = disassemble_chunk(&c);
        assert!(text.contains("CONSTANT"));
        assert!(text.contains("PRINT"));
        assert!(text.contains("RETURN"));
        assert!(text.contains("== main"));
    }

    #[test]
    fn line_table_merges_ranges() {
        let mut c = Chunk::new("lt");
        c.map_source(10, 1);
        c.emit_op(Op::Null);
        c.emit_op(Op::Pop);
        c.map_source(11, 2);
        c.emit_op(Op::True);
        let table = c.line_table();
        assert!(!table.is_empty());
        assert!(table.iter().any(|e| e.line == 10));
        assert!(table.iter().any(|e| e.line == 11));
        let dense = c.dense_line_table();
        assert_eq!(dense.len(), c.code.len());
        assert_eq!(dense[0], 10);
    }

    #[test]
    fn module_metadata_and_disasm() {
        let mut m = BytecodeModule::new();
        m.file = Some("x.vel".into());
        m.metadata = ModuleMetadata::current().with_source("x.vel", Some(fnv1a64(b"hi")));
        let mut c = Chunk::new("<script>");
        c.emit_op(Op::Null);
        c.emit_op(Op::Return);
        m.functions.push(c);
        m.exports.insert("<script>".into(), 0);
        let text = disassemble_module(&m);
        assert!(text.contains("version="));
        assert!(text.contains("<script>"));
        assert_eq!(m.main_index(), Some(0));
    }

    #[test]
    fn fnv_stable() {
        assert_eq!(fnv1a64(b""), 0xcbf29ce484222325);
        assert_ne!(fnv1a64(b"a"), fnv1a64(b"b"));
    }

    #[test]
    fn operand_lens() {
        assert_eq!(Op::NativeCall.operand_len(), 3);
        assert_eq!(Op::Constant.operand_len(), 2);
        assert_eq!(Op::Call.operand_len(), 1);
        assert_eq!(Op::UpdateIndex.operand_len(), 1);
        assert_eq!(Op::Yield.operand_len(), 0);
    }

    #[test]
    fn native_registry_is_contiguous_typed_and_backward_compatible() {
        for (index, native) in NativeId::all().iter().copied().enumerate() {
            assert_eq!(native.as_u16(), index as u16);
            assert_eq!(NativeId::from_u16(index as u16), Some(native));
            assert_eq!(lookup_native(native.name()), Some(native));
            let spec = native.spec();
            assert_eq!(spec.id, native);
            assert_eq!(spec.name, native.name());
            assert!(spec.min_args <= spec.max_args);
            assert!(spec.base_cost > 0);
        }
        assert_eq!(NativeId::Fail.as_u16(), 29);
        assert_eq!(NativeId::Tan.as_u16(), 30);
        assert_eq!(NativeId::RayPlaneIntersection.as_u16(), 144);
        assert_eq!(lookup_math_constant("PI"), Some(std::f64::consts::PI));
    }
}

/// VS2 opcode catalog.
pub mod opcodes_vs2;
