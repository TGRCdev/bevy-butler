use bevy_app::{App, Plugin};
use bevy_log::{debug, info, warn};
use bevy_utils::{HashMap, HashSet};
use std::any::{type_name, TypeId};
use std::sync::LazyLock;

pub use bevy_app;
pub use linkme;
use linkme::distributed_slice;

pub type ButlerRegistry = HashMap<TypeId, HashSet<fn(&mut App)>>;

#[distributed_slice]
pub static BUTLER_SLICE: [&'static dyn ButlerStaticSystem];

pub static BUTLER_REGISTRY: LazyLock<ButlerRegistry> = LazyLock::new(|| {
    let mut registry = ButlerRegistry::new();

    let mut sys_count = 0;
    for system in BUTLER_SLICE {
        let (plugin, func) = system.registry_entry();
        let duplicate_system = !registry.entry(plugin).or_default().insert(func);

        assert!(!duplicate_system, "Tried to insert a butler system twice?");
        sys_count += 1;
    }

    info!("Loaded {sys_count} systems for {} plugins", registry.len());
    registry
});

pub trait ButlerPlugin: Plugin {
    type Marker;

    fn register_butler_plugins(app: &mut App) {
        match BUTLER_REGISTRY.get(&TypeId::of::<Self>()) {
            None => warn!(
                "Butler plugin {} registered, but no systems registered?",
                type_name::<Self>()
            ),
            Some(funcs) => {
                for func in funcs {
                    (func)(app)
                }

                debug!("{} loaded {} systems", type_name::<Self>(), funcs.len());
            }
        }
    }

    /// Used to implement a marker that is only accessible by pub(crate)
    fn _marker() -> Self::Marker;
}

pub trait ButlerSystem
where
    Self: 'static + Sync + Send,
{
    type Plugin: ButlerPlugin;

    fn system(&self) -> fn(&mut App);
}

// dyn-compatible form of ButlerSystem<Plugin>
pub trait ButlerStaticSystem
where
    Self: 'static + Sync + Send,
{
    fn registry_entry(&self) -> (TypeId, fn(&mut App));
}

impl<TSys, TPlugin> ButlerStaticSystem for TSys
where
    TSys: ButlerSystem<Plugin = TPlugin>,
    TPlugin: ButlerPlugin,
{
    fn registry_entry(&self) -> (TypeId, fn(&mut App)) {
        (TypeId::of::<TPlugin>(), self.system())
    }
}
