//! Reference `velvet.water` host module: shallow-water gameplay queries.
//!
//! The host owns a [`ShallowWater`] sim (bedrock usually comes from a
//! terrain heightfield). Scripts sample surface/depth/flow, trigger rain
//! and splashes, and advance the sim in fixed steps:
//!
//! ```ignore
//! // @edition 3
//! function depth_at(x, z) {
//!     return yield(["velvet.water.sample", [x, z]])
//! }
//! ```

use velvet_liquid::{ShallowParams, ShallowWater};
use velvet_script_vs3::{
    validate_request, FunctionDescriptor, HostOutcome, HostRequest, ModuleDescriptor,
    ParamDescriptor, Purity, Value, Vs3AbiType, Vs3Host,
};

/// Capability required by mutating water services.
pub const WATER_CAPABILITY: &str = "water";

fn float_param(name: &str) -> ParamDescriptor {
    ParamDescriptor::new(name, Vs3AbiType::Float)
}

fn water_function(
    name: &str,
    params: Vec<ParamDescriptor>,
    result: Vs3AbiType,
    pure: bool,
    doc: &str,
) -> FunctionDescriptor {
    FunctionDescriptor {
        name: name.into(),
        service: format!("velvet.water.{name}"),
        params,
        result: result.clone(),
        purity: if pure { Purity::Pure } else { Purity::Impure },
        capabilities: if pure {
            Vec::new()
        } else {
            vec![WATER_CAPABILITY.into()]
        },
        doc: doc.into(),
        ..FunctionDescriptor::new(result)
    }
}

/// Descriptor for the `velvet.water` module.
pub fn water_module() -> ModuleDescriptor {
    ModuleDescriptor::new(
        "velvet.water",
        "1.0.0",
        vec![
            water_function(
                "sample",
                vec![float_param("x"), float_param("z")],
                Vs3AbiType::Float,
                true,
                "sample(x, z): surface height in world units",
            ),
            water_function(
                "depth",
                vec![float_param("x"), float_param("z")],
                Vs3AbiType::Float,
                true,
                "depth(x, z): water depth in world units",
            ),
            water_function(
                "flow_x",
                vec![float_param("x"), float_param("z")],
                Vs3AbiType::Float,
                true,
                "flow_x(x, z): solver-internal discharge component",
            ),
            water_function(
                "flow_z",
                vec![float_param("x"), float_param("z")],
                Vs3AbiType::Float,
                true,
                "flow_z(x, z): solver-internal discharge component",
            ),
            water_function(
                "splash",
                vec![
                    float_param("x"),
                    float_param("z"),
                    float_param("radius"),
                    float_param("amount"),
                ],
                Vs3AbiType::Null,
                false,
                "splash(x, z, radius, amount): gaussian puddle",
            ),
            water_function(
                "rain",
                vec![float_param("amount")],
                Vs3AbiType::Null,
                false,
                "rain(amount): uniform depth added",
            ),
            water_function(
                "advance",
                vec![ParamDescriptor::new("steps", Vs3AbiType::Int)],
                Vs3AbiType::Int,
                false,
                "advance(steps): run steps (capped), return steps run",
            ),
        ],
    )
    .expect("built-in descriptors are valid")
}

/// Upper bound for one `advance` call (keeps frames bounded).
pub const MAX_ADVANCE_STEPS: i64 = 1200;

/// Reference host for [`water_module`].
#[derive(Debug)]
pub struct WaterHost {
    sim: ShallowWater,
}

impl WaterHost {
    /// New host over bedrock (must match params resolution²).
    pub fn new(params: ShallowParams, bedrock: Vec<f32>) -> Result<Self, String> {
        Ok(Self {
            sim: ShallowWater::new(params, bedrock)?,
        })
    }

    /// Inner sim (for assertions / coupling).
    pub fn sim(&self) -> &ShallowWater {
        &self.sim
    }

    /// Mutable sim (for test setup).
    pub fn sim_mut(&mut self) -> &mut ShallowWater {
        &mut self.sim
    }
}

fn as_f32(value: &Value) -> Result<f32, String> {
    match value {
        Value::Float(f) => Ok(*f as f32),
        Value::Int(i) => Ok(*i as f32),
        other => Err(format!("expected number, got {other}")),
    }
}

impl Vs3Host for WaterHost {
    fn call(&mut self, request: &HostRequest) -> HostOutcome {
        let module = water_module();
        let Some(function) = module
            .functions
            .values()
            .find(|f| f.service == request.service)
        else {
            return HostOutcome::Failed(format!(
                "service `{}` is not part of velvet.water",
                request.service
            ));
        };
        if let Err(message) = validate_request(function, request) {
            return HostOutcome::Failed(message);
        }
        let args = match &request.payload {
            Value::List(items) => items.borrow().clone(),
            other => {
                return HostOutcome::Failed(format!("water payload must be a list, got {other}"));
            }
        };
        // Bilinear world sampling shared by the query services.
        let sample_world = |args: &[Value]| -> Result<(f32, f32), String> {
            Ok((as_f32(&args[0])?, as_f32(&args[1])?))
        };
        match function.name.as_str() {
            "sample" => {
                let (x, z) = match sample_world(&args) {
                    Ok(v) => v,
                    Err(message) => return HostOutcome::Failed(message),
                };
                HostOutcome::Ready(Value::Float(self.sim.sample_surface_world(x, z) as f64))
            }
            "depth" => {
                let (x, z) = match sample_world(&args) {
                    Ok(v) => v,
                    Err(message) => return HostOutcome::Failed(message),
                };
                HostOutcome::Ready(Value::Float(self.sim.sample_depth_world(x, z) as f64))
            }
            "flow_x" => {
                let (x, z) = match sample_world(&args) {
                    Ok(v) => v,
                    Err(message) => return HostOutcome::Failed(message),
                };
                HostOutcome::Ready(Value::Float(self.sim.velocity_world(x, z).0 as f64))
            }
            "flow_z" => {
                let (x, z) = match sample_world(&args) {
                    Ok(v) => v,
                    Err(message) => return HostOutcome::Failed(message),
                };
                HostOutcome::Ready(Value::Float(self.sim.velocity_world(x, z).1 as f64))
            }
            "splash" => {
                let parsed: Result<Vec<f32>, String> = args.iter().map(as_f32).collect();
                match parsed {
                    Ok(v) => {
                        self.sim.splash(v[0], v[1], v[2].max(0.01), v[3].max(0.0));
                        HostOutcome::Ready(Value::Null)
                    }
                    Err(message) => HostOutcome::Failed(message),
                }
            }
            "rain" => match as_f32(&args[0]) {
                Ok(amount) => {
                    self.sim.rain(amount.max(0.0));
                    HostOutcome::Ready(Value::Null)
                }
                Err(message) => HostOutcome::Failed(message),
            },
            "advance" => match &args[0] {
                Value::Int(steps) => {
                    if *steps < 0 || *steps > MAX_ADVANCE_STEPS {
                        return HostOutcome::Failed(format!(
                            "advance steps must be 0..={MAX_ADVANCE_STEPS}"
                        ));
                    }
                    for _ in 0..*steps {
                        self.sim.step();
                    }
                    HostOutcome::Ready(Value::Int(*steps))
                }
                other => HostOutcome::Failed(format!("advance expects int steps, got {other}")),
            },
            other => HostOutcome::Failed(format!("unknown water function `{other}`")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use velvet_script_vs3::conformance;

    fn num(f: f64) -> Value {
        Value::Float(f)
    }

    fn host() -> WaterHost {
        let params = ShallowParams {
            resolution: 16,
            size: 16.0,
            ..Default::default()
        };
        WaterHost::new(params, vec![0.0; 16 * 16]).unwrap()
    }

    fn call(host: &mut WaterHost, service: &str, args: Vec<Value>) -> HostOutcome {
        let request = HostRequest::new(service, Value::list(args)).unwrap();
        host.call(&request)
    }

    #[test]
    fn descriptor_registers_seven_services() {
        let module = water_module();
        assert_eq!(module.functions.len(), 7);
        assert!(module.functions["advance"]
            .capabilities
            .contains(&WATER_CAPABILITY.to_string()));
        assert!(module.functions["sample"].capabilities.is_empty());
    }

    #[test]
    fn queries_tracks_rain_and_advance() {
        let mut host = host();
        assert!(matches!(
            call(&mut host, "velvet.water.rain", vec![num(0.5)]),
            HostOutcome::Ready(_)
        ));
        // Flat bed: surface == depth everywhere.
        assert!(matches!(
            call(&mut host, "velvet.water.sample", vec![num(0.0), num(0.0)]),
            HostOutcome::Ready(Value::Float(h)) if (h - 0.5).abs() < 1e-3
        ));
        assert!(matches!(
            call(&mut host, "velvet.water.advance", vec![Value::Int(10)]),
            HostOutcome::Ready(Value::Int(10))
        ));
        // Still water: no flow.
        assert!(matches!(
            call(&mut host, "velvet.water.flow_x", vec![num(1.0), num(1.0)]),
            HostOutcome::Ready(Value::Float(v)) if v.abs() < 1e-3
        ));
        // Advance caps keep frames bounded.
        assert!(matches!(
            call(
                &mut host,
                "velvet.water.advance",
                vec![Value::Int(MAX_ADVANCE_STEPS + 1)]
            ),
            HostOutcome::Failed(_)
        ));
    }

    #[test]
    fn splash_raises_local_depth() {
        let mut host = host();
        call(
            &mut host,
            "velvet.water.splash",
            vec![num(0.0), num(0.0), num(2.0), num(1.0)],
        );
        assert!(matches!(
            call(&mut host, "velvet.water.depth", vec![num(0.0), num(0.0)]),
            HostOutcome::Ready(Value::Float(h)) if h > 0.5
        ));
    }

    #[test]
    fn water_host_conforms_to_host_contract() {
        let mut host = host();
        conformance::expect_failed(&mut host, "velvet.water.nope", vec![]);
        conformance::expect_failed(&mut host, "velvet.water.sample", vec![num(0.0)]);
        conformance::expect_ready(
            &mut host,
            "velvet.water.depth",
            vec![num(0.0), num(0.0)],
            Value::Float(0.0),
        );
    }
}
