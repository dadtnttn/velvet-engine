use super::InputFrame;
use crate::assets::AppManifest;
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use velvet_script_vs3::{
    compile_package_path, float_val, string_val, ProfileEntry, Value, VmLimits, Vs3Module,
    Vs3Session,
};

pub struct ScriptRuntime {
    module: Vs3Module,
    session: Vs3Session,
    package_root: PathBuf,
    namespace: String,
    manifest: AppManifest,
}
impl ScriptRuntime {
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let package_root = normalize_package_root(path.as_ref());
        let module = compile_package_path(&package_root)
            .with_context(|| format!("could not compile VS3 package {}", package_root.display()))?;
        let namespace = discover_namespace(&module)?;
        let mut bootstrap = module
            .session_with_limits(VmLimits {
                max_memory_units: 8_000_000,
                max_instructions: 1_000_000,
                ..VmLimits::default()
            })
            .context("could not start VS3 manifest session")?;
        let manifest_value = bootstrap
            .call(&format!("{namespace}.app_manifest"), &[])
            .context("app_manifest VS3")?;
        let manifest = AppManifest::parse(&manifest_value)?;
        manifest.require_capability("render2d")?;
        manifest.require_capability("input")?;
        let session = module
            .session_with_limits(manifest.limits.vm_limits())
            .context("could not start VS3 app session")?;
        Ok(Self {
            module,
            session,
            package_root,
            namespace,
            manifest,
        })
    }
    pub fn manifest(&self) -> &AppManifest {
        &self.manifest
    }
    pub fn package_root(&self) -> &Path {
        &self.package_root
    }
    pub fn initialize(&mut self) -> Result<Value> {
        self.call("app_init", &[])
    }
    pub fn update(&mut self, dt: f64, input: InputFrame) -> Result<Value> {
        self.call("app_update", &[float_val(dt), input.into_value()])
    }
    pub fn action(&mut self, action: &str) -> Result<Value> {
        self.call("app_action", &[string_val(action)])
    }
    pub fn shutdown(&mut self) -> Result<()> {
        if self.has("app_shutdown") {
            let _ = self.call("app_shutdown", &[])?;
        }
        Ok(())
    }
    pub fn call(&mut self, function: &str, args: &[Value]) -> Result<Value> {
        self.session
            .call(&format!("{}.{}", self.namespace, function), args)
            .with_context(|| format!("VS3 app call `{function}`"))
    }
    /// Hand font advance tables to the optional `app_fonts(metrics)` export
    /// (called once, before `app_init`).
    pub fn provide_fonts(&mut self, metrics: Value) -> Result<()> {
        if self.has("app_fonts") {
            self.call("app_fonts", &[metrics])?;
        }
        Ok(())
    }
    /// Deliver a storage result to the optional `app_storage(result)`
    /// export. Returns the frame it produced, if the export exists.
    pub fn storage(&mut self, result: Value) -> Result<Option<Value>> {
        if self.has("app_storage") {
            self.call("app_storage", &[result]).map(Some)
        } else {
            Ok(None)
        }
    }
    /// Count calls and instructions per function from now on.
    pub fn set_profiling(&mut self, enabled: bool) {
        self.session.set_profiling(enabled);
    }
    /// Per-function costs since profiling was enabled, most expensive first.
    pub fn profile_report(&self) -> Vec<ProfileEntry> {
        self.session.profile_report()
    }
    pub fn has(&self, function: &str) -> bool {
        self.module
            .function_names()
            .iter()
            .any(|name| name == &format!("{}.{}", self.namespace, function))
    }
}
fn discover_namespace(module: &Vs3Module) -> Result<String> {
    let matches = module
        .function_names()
        .into_iter()
        .filter_map(|name| name.strip_suffix(".app_manifest").map(str::to_owned))
        .collect::<Vec<_>>();
    anyhow::ensure!(
        matches.len() == 1,
        "VS3 package must export exactly one `app_manifest`; found {}",
        matches.len()
    );
    let namespace = matches[0].clone();
    for required in ["app_init", "app_update", "app_action"] {
        anyhow::ensure!(
            module
                .function_names()
                .contains(&format!("{namespace}.{required}")),
            "VS3 package is missing `{namespace}.{required}`"
        );
    }
    Ok(namespace)
}
fn normalize_package_root(path: &Path) -> PathBuf {
    if path
        .file_name()
        .is_some_and(|name| name == "velvet.package.toml")
    {
        path.parent().unwrap_or(path).to_path_buf()
    } else {
        path.to_path_buf()
    }
}
