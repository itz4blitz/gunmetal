//! The engine settings a later host must build, and whether a platform may
//! run plugins at all.
//!
//! [`POLICY`] is the closed list SEC-EXT-020 names. This crate does not
//! construct a Wasmtime engine: that crate stays banned until the engine
//! test and the OS sandbox tests pass (SEC-EXT-018, SEC-EXT-021).

use crate::id::PluginId;

/// Which compiler the engine uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Compiler {
    /// Cranelift. Winch is not used.
    Cranelift,
}

/// A setting that is on or off. A bool would hide which is which.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Switch {
    /// The setting is on.
    On,
    /// The setting is off.
    Off,
}

/// Whether this platform can run a plugin.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sandbox {
    /// The transcode worker's profile will be applied before any plugin
    /// bytes are loaded, and the server fixed the plugin's identity at
    /// spawn.
    ProfileApplied,
    /// This platform has no such profile. Plugins stay unavailable.
    Unavailable,
}

/// Why a plugin host process is refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostRefuse {
    /// The platform has no sandbox profile.
    Unavailable,
}

/// The settings the engine must be built with. Every flag is a field so a
/// test can assert it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Policy {
    /// Cranelift, not Winch.
    pub compiler: Compiler,
    /// Spectre mitigations stay on.
    pub spectre_mitigations: Switch,
    /// Signal-based traps stay on.
    pub signal_based_traps: Switch,
    /// Guard regions stay on.
    pub guard_regions: Switch,
    /// Fuel is consumed, so a loop can be stopped deterministically.
    pub consume_fuel: Switch,
    /// Epoch interruption is on, so a deadline covers guest code.
    pub epoch_interruption: Switch,
    /// How often the epoch ticker runs, in milliseconds.
    pub epoch_tick_ms: u64,
    /// memory64 stays off.
    pub memory64: Switch,
    /// Threads stay off.
    pub threads: Switch,
    /// Garbage collection stays off.
    pub gc: Switch,
    /// Exceptions stay off.
    pub exceptions: Switch,
    /// Component-model async stays off.
    pub component_async: Switch,
    /// One memory.
    pub memories: u32,
    /// One table.
    pub tables: u32,
    /// One instance.
    pub instances: u32,
    /// A grow that would pass the cap traps.
    pub trap_on_grow_failure: Switch,
    /// The default linear-memory cap, in bytes.
    pub default_memory_bytes: u64,
}

/// The engine a host must build. Not a constructed runtime.
pub const POLICY: Policy = Policy {
    compiler: Compiler::Cranelift,
    spectre_mitigations: Switch::On,
    signal_based_traps: Switch::On,
    guard_regions: Switch::On,
    consume_fuel: Switch::On,
    epoch_interruption: Switch::On,
    epoch_tick_ms: 10,
    memory64: Switch::Off,
    threads: Switch::Off,
    gc: Switch::Off,
    exceptions: Switch::Off,
    component_async: Switch::Off,
    memories: 1,
    tables: 1,
    instances: 1,
    trap_on_grow_failure: Switch::On,
    default_memory_bytes: 64 * 1024 * 1024,
};

/// Whether a host process may load a plugin.
///
/// # Errors
///
/// [`HostRefuse::Unavailable`] when the sandbox profile is absent. The
/// plugin id is accepted only so a caller cannot forget which plugin the
/// channel belongs to; it does not make an unsandboxed platform available.
pub fn admit(sandbox: Sandbox, plugin: &PluginId) -> Result<PluginId, HostRefuse> {
    match sandbox {
        Sandbox::ProfileApplied => Ok(plugin.clone()),
        Sandbox::Unavailable => Err(HostRefuse::Unavailable),
    }
}

#[cfg(test)]
mod tests {
    use super::{Compiler, HostRefuse, POLICY, Policy, Sandbox, Switch, admit};
    use crate::id::PluginId;

    /// Verifies: SEC-EXT-020
    #[test]
    fn the_engine_policy_is_cranelift_with_the_proposals_off() {
        assert_eq!(
            POLICY,
            Policy {
                compiler: Compiler::Cranelift,
                spectre_mitigations: Switch::On,
                signal_based_traps: Switch::On,
                guard_regions: Switch::On,
                consume_fuel: Switch::On,
                epoch_interruption: Switch::On,
                epoch_tick_ms: 10,
                memory64: Switch::Off,
                threads: Switch::Off,
                gc: Switch::Off,
                exceptions: Switch::Off,
                component_async: Switch::Off,
                memories: 1,
                tables: 1,
                instances: 1,
                trap_on_grow_failure: Switch::On,
                default_memory_bytes: 67_108_864,
            }
        );
    }

    /// Verifies: SEC-EXT-021
    #[test]
    fn plugins_are_unavailable_without_the_sandbox_profile() {
        let plugin = PluginId::parse("org.listenbrainz.scrobbler").expect("id");
        assert_eq!(
            admit(Sandbox::Unavailable, &plugin),
            Err(HostRefuse::Unavailable)
        );
        assert_eq!(admit(Sandbox::ProfileApplied, &plugin), Ok(plugin));
    }
}
