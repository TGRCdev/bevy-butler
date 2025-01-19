use bevy_app::App;
use std::{any::TypeId, collections::HashMap, sync::LazyLock};

#[cfg(any(target_arch = "wasm32", feature = "inventory"))]
pub use inventory;
#[cfg(not(any(target_arch = "wasm32", feature = "inventory")))]
pub use linkme;

pub use bevy_app;
pub use bevy_ecs;
pub use bevy_log;

pub struct ButlerRegistryEntryFactory(fn() -> TypeId, fn(&mut bevy_app::App));

impl ButlerRegistryEntryFactory {
    pub const fn new(type_factory: fn() -> TypeId, sys_factory: fn(&mut bevy_app::App)) -> Self {
        ButlerRegistryEntryFactory(type_factory, sys_factory)
    }
}

pub struct ButlerRegistry(HashMap<TypeId, Vec<fn(&mut App)>>);

impl ButlerRegistry {
    pub(crate) fn get_system_factories(
        &'static self,
        marker: TypeId,
    ) -> &'static [fn(&mut bevy_app::App)] {
        self.0
            .get(&marker)
            .map(|v| v.as_slice())
            .unwrap_or_default()
    }
}

#[cfg(not(any(target_arch="wasm32", feature = "inventory")))]
#[linkme::distributed_slice]
pub static BUTLER_SLICE: [ButlerRegistryEntryFactory] = [..];

#[cfg(any(target_arch = "wasm32", feature = "inventory"))]
::inventory::collect!(ButlerRegistryEntryFactory);

#[cfg(target_arch="wasm32")]
unsafe extern "C" {
    unsafe fn __wasm_call_ctors();
}


pub static BUTLER_REGISTRY: LazyLock<ButlerRegistry> = LazyLock::new(|| {
    #[cfg(target_arch="wasm32")]
    unsafe { __wasm_call_ctors(); }

    #[cfg(not(any(target_arch = "wasm32", feature = "inventory")))]
    let iter = BUTLER_SLICE.into_iter();
    #[cfg(any(target_arch = "wasm32", feature = "inventory"))]
    let iter = ::inventory::iter::<ButlerRegistryEntryFactory>.into_iter();

    let mut count = 0;
    let mut registry: HashMap<TypeId, Vec<fn(&mut App)>> = HashMap::new();
    iter.for_each(|ButlerRegistryEntryFactory(type_factory, sys_factory)| {
        registry
            .entry(type_factory())
            .or_default()
            .push(*sys_factory);
        count += 1;
    });

    bevy_log::debug!("Building ButlerRegistry from {count} entries");

    ButlerRegistry(registry)
});

#[cfg(not(any(target_arch="wasm32",feature="inventory")))]
#[macro_export]
macro_rules! butler_entry {
    ($static_ident:ident, $entry:expr) => {
        #[::bevy_butler::__internal::linkme::distributed_slice(::bevy_butler::__internal::BUTLER_SLICE)]
        #[linkme(crate = ::bevy_butler::__internal::linkme)]
        static $static_ident: ::bevy_butler::__internal::ButlerRegistryEntryFactory = 
            $entry;
    };
}

#[cfg(any(target_arch="wasm32",feature="inventory"))]
#[macro_export]
macro_rules! butler_entry {
    ($static_ident:ident, $entry:expr) => {
        ::bevy_butler::__internal::inventory::submit!($entry);
    };
}