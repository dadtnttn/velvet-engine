# VS3 Advanced Mathematics Contract

Status: implemented alpha contract for edition 3.

## Numeric model

- `int` is signed `i64`. Overflow, division by zero, and invalid remainder are runtime errors.
- `float` is IEEE-754 `f64`.
- Script vectors, matrices, and quaternions use `f64`; engine render types remain `f32`.
- Host conversions between script `f64` and engine `f32` are explicit and checked.
- Angles use radians. Use `degrees` and `radians` for conversion.
- `==` is exact. `approx_eq(a, b [, epsilon])` performs tolerant numeric comparison.
- Domain-sensitive functions report the function, contract, and received value.
- Mathematical values are immutable. Random streams and lists are mutable references.

Reserved constants: `PI`, `TAU`, `E`, `EPSILON`, `INFINITY`, and `NAN`.

## Scalar API

Existing: `abs`, `min`, `max`, `floor`, `ceil`, `clamp`, `sin`, `cos`, `sqrt`, `pow`, `lerp`.

Advanced:

- Trigonometry: `tan`, `asin`, `acos`, `atan`, `atan2`.
- Exponential: `exp`, `ln`, `log2`, `log10`, `cbrt`.
- Rounding: `round`, `trunc`, `fract`, `sign`.
- Ranges: `hypot`, `degrees`, `radians`, `inverse_lerp`, `remap`, `smoothstep`.
- Inspection: `is_finite`, `is_nan`, `is_infinite`, `approx_eq`.
- Integer algorithms: `gcd`, `lcm`.

`sqrt` rejects negative or non-finite inputs. Logarithms reject values less than or equal to zero. `asin` and `acos` require `[-1, 1]`. Operations that overflow to a non-finite value report an error where checked.

## Vector API

Types: `vec2`, `vec3`, and `vec4`.

```velvet
let velocity: vec3 = vec3(2, 0, 4)
let next: vec3 = position + velocity * delta
let height: float = next.y
```

Constructors splat one argument. Missing trailing components in multi-argument construction are zero. Components are available through `.x`, `.y`, `.z`, `.w` and numeric indexing.

Operators:

- Vector `+`, `-`, component-wise `*`, component-wise `/`.
- Vector-scalar `*` and `/`; scalar-vector `*`.
- Unary `-`.

Functions: `dot`, `cross`, `length`, `normalize`, `distance`, `angle_between`, `reflect`, `refract`, `project`, `vec_min`, `vec_max`, `clamp_length`, and `vec_lerp`.

`cross(vec2, vec2)` returns a scalar. `cross(vec3, vec3)` returns `vec3`. Normalization, projection, and angle calculation reject zero-length inputs.

## Matrix and quaternion API

Types: column-major `mat3`, column-major `mat4`, and `quat(x, y, z, w)`.

Functions:

- `mat3_identity`, `mat4_identity`, `mat3`, `mat4`.
- `mat_mul`, `mat_transpose`, `mat_determinant`, `mat_inverse`.
- `mat3_translation`, `mat3_scale`, `mat3_rotation`.
- `mat4_translation`, `mat4_scale`, `mat4_rotation_x`, `mat4_rotation_y`, `mat4_rotation_z`.
- `mat4_orthographic`, `mat4_perspective`, `mat4_look_at`.
- `transform_point`, `transform_vector`.
- `quat_identity`, `quat_axis_angle`, `quat_mul`, `quat_rotate`.
- `quat_normalize`, `quat_inverse`, `quat_slerp`, `quat_euler`.

Matrix multiplication and quaternion multiplication also support `*`. Singular matrix inversion and zero-length quaternion operations report domain errors.

The Rust host bridge exposes `from_engine_vec2`, `from_engine_vec3`,
`from_engine_mat3`, and `from_engine_mat4` to promote engine values without loss.
The matching `to_engine_*` functions reject wrong types, non-finite components, and
values outside the finite `f32` range instead of silently truncating them.

## Curves and geometry

- `quadratic_bezier(p0, p1, p2, t)`.
- `cubic_bezier(p0, p1, p2, p3, t)`.
- `catmull_rom(p0, p1, p2, p3, t)`.
- `hermite(p0, tangent0, p1, tangent1, t)`.
- `closest_point_segment(point, a, b)`.
- `segment_intersection2(a0, a1, b0, b1)` returns `vec2` or `null`.
- `ray_plane_intersection(origin, direction, plane_point, plane_normal)` returns a vector or `null`.

Curve points must share a vector dimension.

## Deterministic random and procedural noise

```velvet
state { random: rng = rng_new(42) }
let roll = rng_range_int(random, 1, 7)
```

The random stream uses a stable PCG sequence:

- `rng_new`, `rng_next_float`, `rng_range_int`, `rng_range_float`, `rng_bool`.
- `rng_gaussian` and `rng_exponential`.
- `shuffle`, `choose`, `weighted_choose`.

Noise is pure and explicitly seeded:

- `noise1(x, seed)`.
- `noise2(x, y, seed)`.
- `perlin2(x, y, seed)`.
- `fbm2(x, y, seed, octaves [, lacunarity [, gain]])`.
- `turbulence2(x, y, seed, octaves [, lacunarity [, gain]])`.
- `domain_warp2(x, y, seed, octaves, strength)`.

`fbm2` limits octaves to `1..=16`. Equal seeds and arguments produce equal results.

## Statistics and numerical methods

Statistics accept finite numeric lists:

- `sum`, `product`, `mean`, `median`.
- `mode`, `histogram`, `ema`.
- `variance(values [, sample])`, `stddev(values [, sample])`.
- `quantile`, `covariance`, `correlation`, `moving_average`.

Integer helpers also include `is_even`, `is_odd`, and checked `pow_int`.

Numerical methods:

- `poly_eval(coefficients, x)` uses Horner evaluation, highest-degree coefficient first.
- `integrate_trapezoid(samples, step)`.
- `integrate_simpson(samples, step)` requires an odd sample count of at least three.
- `poly_root_bisection(coefficients, lo, hi [, iterations [, tolerance]])`.

Bisection requires a bracketed root and limits iterations to `1..=256`.

## Sandbox and cost model

Every native has shared metadata: stable id, arity, parameter kinds, result kind, purity, and base instruction cost. Collection algorithms charge additional instructions proportional to input length. `fbm2` charges per octave. This prevents a single native call from bypassing VM budgets.

Random streams are session-local values. They do not read operating-system entropy, time, or global state. Mathematical functions perform no host I/O.

## Tooling

- Semantic analysis validates annotations, native arity, known argument types, vector components, and immutable math assignments.
- LSP completion is generated from the native registry.
- Hover displays result kind, purity, and base cost.
- CLI accepts `v2:`, `v3:`, `v4:`, `q:`, `m3:`, and `m4:` comma-separated arguments.
- VS Code grammar highlights all mathematical types, constants, and natives.
