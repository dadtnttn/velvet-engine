//! Application builder and run loop.

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use tracing::{debug, error, info};
use velvet_core::config::EngineConfig;
use velvet_core::diagnostics::{Diagnostics, FrameStats};
use velvet_core::plugin::{PluginError, PluginId};
use velvet_core::RunMode;
use velvet_events::{AppExit, Events};
use velvet_time::{FixedTime, Time};

use crate::change_tick::ChangeTicks;
use crate::exclusive::{run_exclusive_commands, ExclusiveSystemQueue};
use crate::ordering::SystemOrderGraph;
use crate::plugin::{Plugin, PluginRegistration};
use crate::schedule::{ScheduleLabel, Schedules};
use crate::system::SystemId;
use crate::world_resources::Resources;

/// Exit code returned from [`App::run`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AppExitCode(pub i32);

/// Custom main-loop strategy.
pub trait Runner: Send {
    /// Run the application until exit.
    fn run(&mut self, app: &mut App) -> AppExitCode;
}

/// Default headless runner: pumps schedules without a window.
pub struct HeadlessRunner {
    /// Maximum frames to run (`None` = until exit requested).
    pub max_frames: Option<u64>,
    /// Simulated delta seconds per frame.
    pub delta_secs: f32,
}

impl Default for HeadlessRunner {
    fn default() -> Self {
        Self {
            max_frames: None,
            delta_secs: 1.0 / 60.0,
        }
    }
}

impl Runner for HeadlessRunner {
    fn run(&mut self, app: &mut App) -> AppExitCode {
        app.finish_plugins().expect("plugin finish");
        app.run_startup();
        let mut frames = 0u64;
        while !app.should_exit() {
            if let Some(max) = self.max_frames {
                if frames >= max {
                    break;
                }
            }
            app.tick_frame(self.delta_secs);
            frames += 1;
        }
        app.run_schedule(ScheduleLabel::Shutdown);
        AppExitCode(app.exit_code())
    }
}

/// Central application object.
pub struct App {
    config: EngineConfig,
    resources: Resources,
    schedules: Schedules,
    events: Events,
    plugins: Vec<PluginRegistration>,
    plugins_built: bool,
    plugins_finished: bool,
    runner: Option<Box<dyn Runner>>,
    exit_requested: Arc<AtomicBool>,
    exit_code: i32,
    startup_done: bool,
    /// System ordering graph (optional constraints).
    order_graph: SystemOrderGraph,
    /// Exclusive systems queued for end-of-stage / explicit flush.
    exclusive_queue: ExclusiveSystemQueue,
    /// Resource change ticks.
    change_ticks: ChangeTicks,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    /// Create a new app with default configuration and core resources.
    pub fn new() -> Self {
        let mut app = Self {
            config: EngineConfig::default(),
            resources: Resources::new(),
            schedules: Schedules::new(),
            events: Events::new(),
            plugins: Vec::new(),
            plugins_built: false,
            plugins_finished: false,
            runner: Some(Box::new(HeadlessRunner {
                max_frames: Some(0),
                ..Default::default()
            })),
            exit_requested: Arc::new(AtomicBool::new(false)),
            exit_code: 0,
            startup_done: false,
            order_graph: SystemOrderGraph::new(),
            exclusive_queue: ExclusiveSystemQueue::new(),
            change_ticks: ChangeTicks::new(),
        };
        app.resources.insert(Time::new());
        app.resources.insert(FixedTime::default());
        app.resources.insert(Diagnostics::default());
        app.resources.insert(app.config.clone());
        app.events.add_event::<AppExit>();
        app
    }

    /// Create with explicit config.
    pub fn with_config(config: EngineConfig) -> Self {
        let mut app = Self::new();
        app.set_config(config);
        app
    }

    /// Replace engine configuration.
    pub fn set_config(&mut self, config: EngineConfig) {
        self.config = config.clone();
        self.resources.insert(config);
    }

    /// Borrow configuration.
    pub fn config(&self) -> &EngineConfig {
        &self.config
    }

    /// Run mode shortcut.
    pub fn run_mode(&self) -> RunMode {
        self.config.mode
    }

    /// Insert a resource.
    pub fn insert_resource<T: crate::world_resources::Resource>(&mut self, value: T) -> &mut Self {
        let added = !self.resources.contains::<T>();
        self.resources.insert(value);
        if added {
            self.change_ticks.mark_added::<T>();
        } else {
            self.change_ticks.mark_changed::<T>();
        }
        self
    }

    /// System ordering graph.
    pub fn order_graph(&self) -> &SystemOrderGraph {
        &self.order_graph
    }

    /// Mutable system ordering graph.
    pub fn order_graph_mut(&mut self) -> &mut SystemOrderGraph {
        &mut self.order_graph
    }

    /// Exclusive system queue.
    pub fn exclusive_queue(&self) -> &ExclusiveSystemQueue {
        &self.exclusive_queue
    }

    /// Mutable exclusive system queue.
    pub fn exclusive_queue_mut(&mut self) -> &mut ExclusiveSystemQueue {
        &mut self.exclusive_queue
    }

    /// Queue an exclusive system to run on next flush.
    pub fn queue_exclusive<F>(&mut self, name: impl Into<String>, f: F) -> SystemId
    where
        F: for<'a> FnMut(&'a mut App) + Send + Sync + 'static,
    {
        self.exclusive_queue.push(name, f)
    }

    /// Drain and run all queued exclusive systems.
    pub fn flush_exclusive(&mut self) {
        let mut cmds = self.exclusive_queue.drain();
        run_exclusive_commands(self, &mut cmds);
    }

    /// Resource change ticks.
    pub fn change_ticks(&self) -> &ChangeTicks {
        &self.change_ticks
    }

    /// Mutable change ticks.
    pub fn change_ticks_mut(&mut self) -> &mut ChangeTicks {
        &mut self.change_ticks
    }

    /// Mark resource `T` as changed at the current tick.
    pub fn mark_resource_changed<T: crate::world_resources::Resource>(&mut self) {
        self.change_ticks.mark_changed::<T>();
    }

    /// Immutable resource.
    pub fn resource<T: crate::world_resources::Resource>(&self) -> Option<&T> {
        self.resources.get::<T>()
    }

    /// Mutable resource.
    pub fn resource_mut<T: crate::world_resources::Resource>(&mut self) -> Option<&mut T> {
        self.resources.get_mut::<T>()
    }

    /// Resource storage.
    pub fn resources(&self) -> &Resources {
        &self.resources
    }

    /// Mutable resources.
    pub fn resources_mut(&mut self) -> &mut Resources {
        &mut self.resources
    }

    /// Event registry.
    pub fn events(&self) -> &Events {
        &self.events
    }

    /// Mutable events.
    pub fn events_mut(&mut self) -> &mut Events {
        &mut self.events
    }

    /// Schedules.
    pub fn schedules(&self) -> &Schedules {
        &self.schedules
    }

    /// Mutable schedules.
    pub fn schedules_mut(&mut self) -> &mut Schedules {
        &mut self.schedules
    }

    /// Register a plugin (built later during [`Self::build_plugins`] / [`Self::run`]).
    pub fn add_plugin<P: Plugin>(&mut self, plugin: P) -> &mut Self {
        let id = plugin.id();
        let dependencies = plugin.dependencies().to_vec();
        self.plugins.push(PluginRegistration {
            plugin: Box::new(plugin),
            id,
            dependencies,
        });
        self
    }

    /// Add a system to a schedule.
    pub fn add_system<F>(&mut self, label: ScheduleLabel, system: F) -> &mut Self
    where
        F: for<'a> FnMut(&'a mut App) + Send + Sync + 'static,
    {
        self.schedules.add_system(label, system);
        self
    }

    /// Add a system and return its id.
    pub fn add_system_id<F>(&mut self, label: ScheduleLabel, system: F) -> SystemId
    where
        F: for<'a> FnMut(&'a mut App) + Send + Sync + 'static,
    {
        self.schedules.add_system(label, system)
    }

    /// Override the runner.
    pub fn set_runner<R: Runner + 'static>(&mut self, runner: R) -> &mut Self {
        self.runner = Some(Box::new(runner));
        self
    }

    /// Request application exit.
    pub fn request_exit(&self) {
        self.exit_requested.store(true, Ordering::SeqCst);
    }

    /// Request exit with code.
    pub fn request_exit_with_code(&mut self, code: i32) {
        self.exit_code = code;
        self.request_exit();
    }

    /// Whether exit was requested.
    pub fn should_exit(&self) -> bool {
        self.exit_requested.load(Ordering::SeqCst)
    }

    /// Exit code.
    pub fn exit_code(&self) -> i32 {
        self.exit_code
    }

    /// Shared exit flag for window integrations.
    pub fn exit_flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.exit_requested)
    }

    /// Topologically sort plugins and call `build`.
    pub fn build_plugins(&mut self) -> Result<(), PluginError> {
        if self.plugins_built {
            return Ok(());
        }

        let order = resolve_plugin_order(&self.plugins)?;
        // Rebuild plugins list in order.
        let mut by_id: HashMap<PluginId, PluginRegistration> = HashMap::new();
        for reg in self.plugins.drain(..) {
            by_id.insert(reg.id.clone(), reg);
        }
        let mut ordered = Vec::with_capacity(order.len());
        for id in &order {
            if let Some(reg) = by_id.remove(id) {
                ordered.push(reg);
            }
        }
        self.plugins = ordered;

        // Enable filter from config if non-empty.
        let enabled_filter: Option<HashSet<String>> = if self.config.plugins.is_empty() {
            None
        } else {
            Some(self.config.plugins.iter().cloned().collect())
        };

        // Build requires splitting plugin boxes because of borrow rules.
        let plugins = std::mem::take(&mut self.plugins);
        for reg in &plugins {
            if let Some(filter) = &enabled_filter {
                if !filter.contains(reg.id.as_str()) && !filter.contains(reg.plugin.name()) {
                    debug!(plugin = %reg.id, "skipping disabled plugin");
                    continue;
                }
            }
            if !reg.plugin.is_enabled(self) {
                debug!(plugin = %reg.id, "plugin reports disabled");
                continue;
            }
            info!(plugin = %reg.id, "building plugin");
            reg.plugin.build(self).map_err(|e| {
                error!(plugin = %reg.id, error = %e, "plugin build failed");
                e
            })?;
        }
        self.plugins = plugins;
        self.plugins_built = true;
        Ok(())
    }

    /// Call `finish` on all plugins.
    pub fn finish_plugins(&mut self) -> Result<(), PluginError> {
        if !self.plugins_built {
            self.build_plugins()?;
        }
        if self.plugins_finished {
            return Ok(());
        }
        let plugins = std::mem::take(&mut self.plugins);
        for reg in &plugins {
            reg.plugin.finish(self).map_err(|e| {
                error!(plugin = %reg.id, error = %e, "plugin finish failed");
                e
            })?;
        }
        self.plugins = plugins;
        self.plugins_finished = true;
        Ok(())
    }

    /// Run startup schedules once.
    pub fn run_startup(&mut self) {
        if self.startup_done {
            return;
        }
        for label in ScheduleLabel::startup_order() {
            self.run_schedule(*label);
        }
        self.startup_done = true;
    }

    /// Run all systems in a schedule label.
    pub fn run_schedule(&mut self, label: ScheduleLabel) {
        // Extract systems to avoid simultaneous borrows of `self`.
        let mut systems = {
            let stage = self.schedules.stage_mut(label);
            std::mem::take(stage.systems_mut())
        };

        for sys in systems.iter_mut() {
            sys.run(self);
        }

        *self.schedules.stage_mut(label).systems_mut() = systems;
    }

    /// Advance one frame with the given raw delta seconds.
    pub fn tick_frame(&mut self, raw_delta_secs: f32) {
        self.change_ticks.advance_world();

        {
            let time = self.resources.get_mut::<Time>().expect("Time resource");
            time.advance(raw_delta_secs);
        }
        self.change_ticks.mark_changed::<Time>();

        let scaled = self
            .resources
            .get::<Time>()
            .map(|t| t.scaled_delta_secs())
            .unwrap_or(0.0);

        let fixed_steps = {
            let fixed = self
                .resources
                .get_mut::<FixedTime>()
                .expect("FixedTime resource");
            fixed.drain_steps(scaled)
        };
        self.change_ticks.mark_changed::<FixedTime>();

        self.run_schedule(ScheduleLabel::First);
        self.flush_exclusive();
        self.run_schedule(ScheduleLabel::PreUpdate);
        for _ in 0..fixed_steps {
            self.run_schedule(ScheduleLabel::FixedUpdate);
        }
        self.run_schedule(ScheduleLabel::Update);
        self.flush_exclusive();
        self.run_schedule(ScheduleLabel::PostUpdate);
        self.run_schedule(ScheduleLabel::PreRender);
        self.run_schedule(ScheduleLabel::Render);
        self.run_schedule(ScheduleLabel::PostRender);
        self.run_schedule(ScheduleLabel::Last);
        self.flush_exclusive();

        self.events.update();

        // Diagnostics
        let frame = self
            .resources
            .get::<Time>()
            .map(|t| t.frame_count())
            .unwrap_or(0);
        let delta = self
            .resources
            .get::<Time>()
            .map(|t| t.delta_secs())
            .unwrap_or(0.0);
        if let Some(diag) = self.resources.get_mut::<Diagnostics>() {
            diag.record(FrameStats {
                frame,
                delta_secs: delta,
                fixed_steps,
                ..Default::default()
            });
        }

        // Honor AppExit events
        // Readers would need storage; check pending via resource pattern later.
        if self.should_exit() {
            debug!("exit requested");
        }
    }

    /// Build plugins and run the configured runner.
    pub fn run(&mut self) -> AppExitCode {
        if let Err(e) = self.build_plugins() {
            error!("failed to build plugins: {e}");
            return AppExitCode(1);
        }
        if let Err(e) = self.finish_plugins() {
            error!("failed to finish plugins: {e}");
            return AppExitCode(1);
        }

        let mut runner = self
            .runner
            .take()
            .unwrap_or_else(|| Box::new(HeadlessRunner::default()));
        let code = runner.run(self);
        self.runner = Some(runner);
        code
    }

    /// Number of registered plugins.
    pub fn plugin_count(&self) -> usize {
        self.plugins.len()
    }
}

/// Kahn topological sort for plugin dependencies.
fn resolve_plugin_order(plugins: &[PluginRegistration]) -> Result<Vec<PluginId>, PluginError> {
    let mut ids = Vec::new();
    let mut deps: HashMap<PluginId, Vec<PluginId>> = HashMap::new();
    let mut present = HashSet::new();

    for reg in plugins {
        if !present.insert(reg.id.clone()) {
            return Err(PluginError::Duplicate(reg.id.clone()));
        }
        ids.push(reg.id.clone());
        deps.insert(reg.id.clone(), reg.dependencies.clone());
    }

    // Missing deps
    for reg in plugins {
        for d in &reg.dependencies {
            if !present.contains(d) {
                return Err(PluginError::MissingDependency {
                    plugin: reg.id.clone(),
                    missing: d.clone(),
                });
            }
        }
    }

    let mut incoming: HashMap<PluginId, usize> = ids.iter().map(|id| (id.clone(), 0)).collect();
    let mut forward: HashMap<PluginId, Vec<PluginId>> =
        ids.iter().map(|id| (id.clone(), Vec::new())).collect();

    for reg in plugins {
        for d in &reg.dependencies {
            // edge d -> reg.id (dependency first)
            forward.get_mut(d).unwrap().push(reg.id.clone());
            *incoming.get_mut(&reg.id).unwrap() += 1;
        }
    }

    let mut queue: VecDeque<PluginId> = incoming
        .iter()
        .filter(|(_, c)| **c == 0)
        .map(|(id, _)| id.clone())
        .collect();
    // Stable order: sort zero-incoming by original registration index.
    queue
        .make_contiguous()
        .sort_by_key(|id| ids.iter().position(|x| x == id).unwrap_or(0));

    let mut ordered = Vec::with_capacity(ids.len());
    while let Some(id) = queue.pop_front() {
        ordered.push(id.clone());
        if let Some(children) = forward.get(&id) {
            for child in children {
                let count = incoming.get_mut(child).unwrap();
                *count -= 1;
                if *count == 0 {
                    queue.push_back(child.clone());
                }
            }
        }
    }

    if ordered.len() != ids.len() {
        let leftover: Vec<_> = ids
            .iter()
            .filter(|id| !ordered.contains(id))
            .map(|id| id.as_str().to_string())
            .collect();
        return Err(PluginError::Cycle(leftover.join(" -> ")));
    }

    Ok(ordered)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugin::Plugin;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    struct CounterPlugin {
        name: &'static str,
        deps: Vec<PluginId>,
        counter: Arc<AtomicUsize>,
    }

    impl Plugin for CounterPlugin {
        fn name(&self) -> &'static str {
            self.name
        }
        fn dependencies(&self) -> &[PluginId] {
            &self.deps
        }
        fn build(&self, _app: &mut App) -> Result<(), PluginError> {
            self.counter.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    #[test]
    fn plugin_order_respects_deps() {
        let counter = Arc::new(AtomicUsize::new(0));
        let mut app = App::new();
        app.add_plugin(CounterPlugin {
            name: "b",
            deps: vec![PluginId::new("a")],
            counter: Arc::clone(&counter),
        });
        app.add_plugin(CounterPlugin {
            name: "a",
            deps: vec![],
            counter: Arc::clone(&counter),
        });
        app.build_plugins().unwrap();
        assert_eq!(counter.load(Ordering::SeqCst), 2);
        assert_eq!(app.plugins[0].id.as_str(), "a");
        assert_eq!(app.plugins[1].id.as_str(), "b");
    }

    #[test]
    fn detects_cycle() {
        let counter = Arc::new(AtomicUsize::new(0));
        let mut app = App::new();
        app.add_plugin(CounterPlugin {
            name: "a",
            deps: vec![PluginId::new("b")],
            counter: Arc::clone(&counter),
        });
        app.add_plugin(CounterPlugin {
            name: "b",
            deps: vec![PluginId::new("a")],
            counter,
        });
        let err = app.build_plugins().unwrap_err();
        assert!(matches!(err, PluginError::Cycle(_)));
    }

    #[test]
    fn headless_frame_loop() {
        let mut app = App::new();
        let ticks = Arc::new(AtomicUsize::new(0));
        let ticks_c = Arc::clone(&ticks);
        app.add_system(ScheduleLabel::Update, move |_app| {
            ticks_c.fetch_add(1, Ordering::SeqCst);
        });
        app.set_runner(HeadlessRunner {
            max_frames: Some(5),
            delta_secs: 1.0 / 60.0,
        });
        let code = app.run();
        assert_eq!(code.0, 0);
        assert_eq!(ticks.load(Ordering::SeqCst), 5);
        assert_eq!(app.resource::<Time>().unwrap().frame_count(), 5);
    }

    #[test]
    fn fixed_update_runs_multiple() {
        let mut app = App::new();
        let fixed = Arc::new(AtomicUsize::new(0));
        let fixed_c = Arc::clone(&fixed);
        app.add_system(ScheduleLabel::FixedUpdate, move |_app| {
            fixed_c.fetch_add(1, Ordering::SeqCst);
        });
        // Three full fixed steps at 60 Hz (avoid float edge of exact multiples).
        let step = 1.0 / 60.0;
        app.tick_frame(step * 3.0 + 0.0001);
        assert_eq!(fixed.load(Ordering::SeqCst), 3);
    }
}
