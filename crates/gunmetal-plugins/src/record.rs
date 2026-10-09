//! The reviewed record a manifest names.
//!
//! Fields are data. A text becomes a [`Manifest`] only through the parser.
//! These types let a test build the literal it expects.

use crate::id::PluginId;
use crate::scope::Scope;
use crate::version::PluginVersion;
use crate::world::World;
use gunmetal_egress::destination::{Host, HostError};

/// The default linear-memory cap, in mebibytes (SEC-EXT-023).
pub const DEFAULT_MEMORY_MIB: u32 = 64;

/// The most memory a manifest may declare, in mebibytes.
pub const MAX_MEMORY_MIB: u32 = 256;

/// The default call deadline, in seconds.
pub const DEFAULT_DEADLINE_SECS: u32 = 5;

/// The longest deadline, for a scheduled task.
pub const MAX_DEADLINE_SECS: u32 = 60;

/// The default key-value store, in mebibytes (SEC-EXT-033).
pub const DEFAULT_KV_MIB: u32 = 1;

/// The most key-value store a manifest may declare, in mebibytes.
pub const MAX_KV_MIB: u32 = 16;

/// The default outbound requests in a minute.
pub const DEFAULT_OUTBOUND_PER_MINUTE: u32 = 60;

/// How many calls may run at once.
pub const MAX_CONCURRENT_CALLS: u32 = 2;

/// The longest `why` string, in bytes.
pub const MAX_WHY: usize = 160;

/// The longest plugin name, in bytes.
pub const MAX_NAME: usize = 64;

/// Who turns a plugin on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Mode {
    /// Each person turns it on for themselves.
    PerUser,
    /// The owner turns it on for the server.
    Server,
}

impl Mode {
    /// The manifest spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PerUser => "per-user",
            Self::Server => "server",
        }
    }
}

/// Why a public host grant is refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostRefuse {
    /// The text contains `*`.
    Wildcard,
    /// It is an IP address. A public grant names a host, not an address.
    Address,
    /// It is not a DNS name.
    Name(HostError),
    /// It has no dot, so it is not a public host name.
    NoDot,
}

/// A public host, in lower case, with no trailing dot and no address.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ExactHost(String);

impl ExactHost {
    /// Reads a public host grant.
    ///
    /// # Errors
    ///
    /// A [`HostRefuse`] for a wildcard, an address, a name that is not DNS,
    /// or a name with no dot.
    pub fn parse_public(text: &str) -> Result<Self, HostRefuse> {
        if text.contains('*') {
            return Err(HostRefuse::Wildcard);
        }
        match Host::parse(text) {
            Ok(Host::Address(_)) => Err(HostRefuse::Address),
            Ok(Host::Name(name)) => {
                if name.as_str().contains('.') {
                    Ok(Self(name.as_str().to_owned()))
                } else {
                    Err(HostRefuse::NoDot)
                }
            }
            Err(error) => Err(HostRefuse::Name(error)),
        }
    }

    /// The host, in lower case.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// One host a plugin may ask the egress client to reach, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkGrant {
    /// The exact host.
    pub host: ExactHost,
    /// The reason shown on the consent screen, as text.
    pub why: String,
}

/// A reviewed plugin record. A text becomes one only through the parser.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Manifest {
    /// The stable id.
    pub id: PluginId,
    /// The name people see.
    pub name: String,
    /// The package version.
    pub version: PluginVersion,
    /// The world it implements.
    pub world: World,
    /// Who turns it on.
    pub mode: Mode,
    /// Hosts it may reach. Empty means no network.
    pub network: Vec<NetworkGrant>,
    /// Scopes it may use. Empty means none.
    pub scopes: Vec<Scope>,
    /// Secrets it asks to keep.
    pub secrets: Vec<SecretDecl>,
    /// Linear memory, in mebibytes.
    pub memory_mib: u32,
    /// Call deadline, in seconds.
    pub deadline_secs: u32,
    /// Key-value store, in mebibytes.
    pub kv_mib: u32,
    /// Outbound requests allowed in one minute.
    pub outbound_per_minute: u32,
}

/// A secret the plugin asks to keep.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretDecl {
    /// The secret's name inside the plugin.
    pub name: String,
    /// The label the person sees.
    pub label: String,
    /// Whether each person has their own value.
    pub per_user: bool,
}

#[cfg(test)]
mod tests {
    use super::{
        DEFAULT_DEADLINE_SECS, DEFAULT_KV_MIB, DEFAULT_MEMORY_MIB, DEFAULT_OUTBOUND_PER_MINUTE,
        ExactHost, HostRefuse, MAX_CONCURRENT_CALLS, MAX_DEADLINE_SECS, MAX_KV_MIB, MAX_MEMORY_MIB,
        MAX_NAME, MAX_WHY, Mode,
    };
    use gunmetal_egress::destination::HostError;

    /// Verifies: SEC-EXT-023, SEC-EXT-033
    #[test]
    fn the_baseline_limits_are_these_numbers() {
        assert_eq!(DEFAULT_MEMORY_MIB, 64);
        assert_eq!(MAX_MEMORY_MIB, 256);
        assert_eq!(DEFAULT_DEADLINE_SECS, 5);
        assert_eq!(MAX_DEADLINE_SECS, 60);
        assert_eq!(DEFAULT_KV_MIB, 1);
        assert_eq!(MAX_KV_MIB, 16);
        assert_eq!(DEFAULT_OUTBOUND_PER_MINUTE, 60);
        assert_eq!(MAX_CONCURRENT_CALLS, 2);
        assert_eq!(MAX_WHY, 160);
        assert_eq!(MAX_NAME, 64);
    }

    /// Verifies: SEC-EXT-026
    #[test]
    fn a_public_host_is_a_lower_case_name_and_not_an_address() {
        let host = ExactHost::parse_public("API.ListenBrainz.Org").expect("a name");
        assert_eq!(host.as_str(), "api.listenbrainz.org");
        assert_eq!(
            ExactHost::parse_public("*.listenbrainz.org"),
            Err(HostRefuse::Wildcard)
        );
        assert_eq!(ExactHost::parse_public("1.2.3.4"), Err(HostRefuse::Address));
        assert_eq!(ExactHost::parse_public("::1"), Err(HostRefuse::Address));
        assert_eq!(ExactHost::parse_public("localhost"), Err(HostRefuse::NoDot));
        assert_eq!(
            ExactHost::parse_public("bad_host.com"),
            Err(HostRefuse::Name(HostError::BadLabel))
        );
        assert_eq!(
            ExactHost::parse_public("api.listenbrainz.org."),
            Err(HostRefuse::Name(HostError::BadLabel))
        );
    }

    #[test]
    fn a_mode_spells_its_manifest_word() {
        assert_eq!(Mode::PerUser.as_str(), "per-user");
        assert_eq!(Mode::Server.as_str(), "server");
    }
}
