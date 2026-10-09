//! The closed manifest parser.
//!
//! A text becomes a [`Manifest`] only here. Every key is visited. A key the
//! schema does not name is refused, and a permission that is absent is not
//! granted (SEC-EXT-025).

use toml::Spanned;
use toml::de::{DeInteger, DeTable, DeValue};

use crate::id::{IdError, PluginId};
use crate::record::{
    DEFAULT_DEADLINE_SECS, DEFAULT_KV_MIB, DEFAULT_MEMORY_MIB, DEFAULT_OUTBOUND_PER_MINUTE,
    ExactHost, HostRefuse, MAX_DEADLINE_SECS, MAX_KV_MIB, MAX_MEMORY_MIB, Manifest, Mode,
    NetworkGrant, SecretDecl,
};
use crate::scope::{Scope, ScopeError};
use crate::version::{PluginVersion, VersionError};
use crate::world::{World, WorldError};

/// The longest plugin name, in bytes.
pub const MAX_NAME: usize = crate::record::MAX_NAME;

/// The longest network `why`, in bytes.
pub const MAX_WHY: usize = crate::record::MAX_WHY;

/// The longest secret name, in bytes.
pub const MAX_SECRET_NAME: usize = 32;

/// The longest secret label, in bytes.
pub const MAX_SECRET_LABEL: usize = 80;

/// The most outbound requests a manifest may declare in one minute.
pub const MAX_OUTBOUND_PER_MINUTE: u32 = 60;

/// Why a manifest was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManifestError {
    /// The text is not TOML.
    Syntax {
        /// The line the problem is on, counting from 1.
        line: usize,
        /// What the TOML reader said.
        message: String,
    },
    /// The manifest names a key the schema does not have.
    UnknownKey {
        /// The key, with a `.` between each part.
        key: String,
        /// The line it is on, counting from 1.
        line: usize,
    },
    /// A required key is absent.
    Missing {
        /// The key.
        key: &'static str,
    },
    /// A key's value is the wrong type, or a mode the schema does not name.
    Invalid {
        /// The key.
        key: &'static str,
        /// The line it is on, counting from 1.
        line: usize,
    },
    /// The plugin id is not an id.
    Id(IdError),
    /// The version is not three plain numbers.
    Version(VersionError),
    /// The world is not one this host names.
    World(WorldError),
    /// A scope is not one a plugin may hold.
    Scope(ScopeError),
    /// A host grant is not a public host.
    Host(HostRefuse),
    /// A text is longer than its limit.
    TooLong {
        /// The key.
        key: &'static str,
        /// The most bytes it may have.
        limit: usize,
        /// How many bytes it has.
        got: usize,
    },
    /// A number is outside 1 to its maximum.
    Limit {
        /// The key.
        key: &'static str,
        /// The largest allowed value.
        max: u32,
        /// The value that was read.
        got: u32,
    },
    /// The same host is granted twice.
    DuplicateHost {
        /// The host, in lower case.
        host: String,
    },
    /// The same scope is named twice.
    DuplicateScope(Scope),
    /// The same secret name is declared twice.
    DuplicateSecret {
        /// The name.
        name: String,
    },
}

/// Reads a closed manifest.
///
/// An absent permission is not granted.
///
/// # Errors
///
/// A [`ManifestError`] when the text is not TOML, names a key the schema
/// does not have, or gives a key a value it cannot take.
pub fn parse(text: &str) -> Result<Manifest, ManifestError> {
    let parsed = DeTable::parse(text).map_err(|error| ManifestError::Syntax {
        line: syntax_line(text, error.span()),
        message: error.message().to_owned(),
    })?;
    read_root(text, parsed.get_ref())?.finish()
}

/// Fields gathered while every key is visited. Absent permissions stay empty.
struct Parts {
    id: Option<PluginId>,
    name: Option<String>,
    version: Option<PluginVersion>,
    world: Option<World>,
    mode: Option<Mode>,
    network: Vec<NetworkGrant>,
    scopes: Vec<Scope>,
    secrets: Vec<SecretDecl>,
    memory_mib: u32,
    deadline_secs: u32,
    kv_mib: u32,
    outbound_per_minute: u32,
}

impl Parts {
    fn bare() -> Self {
        Self {
            id: None,
            name: None,
            version: None,
            world: None,
            mode: None,
            network: Vec::new(),
            scopes: Vec::new(),
            secrets: Vec::new(),
            memory_mib: DEFAULT_MEMORY_MIB,
            deadline_secs: DEFAULT_DEADLINE_SECS,
            kv_mib: DEFAULT_KV_MIB,
            outbound_per_minute: DEFAULT_OUTBOUND_PER_MINUTE,
        }
    }

    fn finish(self) -> Result<Manifest, ManifestError> {
        Ok(Manifest {
            id: required(self.id, "plugin.id")?,
            name: required(self.name, "plugin.name")?,
            version: required(self.version, "plugin.version")?,
            world: required(self.world, "plugin.world")?,
            mode: required(self.mode, "plugin.mode")?,
            network: self.network,
            scopes: self.scopes,
            secrets: self.secrets,
            memory_mib: self.memory_mib,
            deadline_secs: self.deadline_secs,
            kv_mib: self.kv_mib,
            outbound_per_minute: self.outbound_per_minute,
        })
    }
}

/// The line of `text`, counting from 1, that byte `offset` is on.
fn line_of(text: &str, offset: usize) -> usize {
    text.bytes()
        .take(offset)
        .filter(|byte| *byte == b'\n')
        .count()
        + 1
}

/// The line a TOML syntax error names, or 1 when the reader gave no span.
fn syntax_line(text: &str, span: Option<std::ops::Range<usize>>) -> usize {
    span.map_or(1, |span| line_of(text, span.start))
}

/// A required value, or [`ManifestError::Missing`].
fn required<T>(value: Option<T>, key: &'static str) -> Result<T, ManifestError> {
    value.ok_or(ManifestError::Missing { key })
}

/// An unknown key at `line`.
fn unknown(key: String, line: usize) -> ManifestError {
    ManifestError::UnknownKey { key, line }
}

/// A value of the wrong type at `line`.
fn invalid(key: &'static str, line: usize) -> ManifestError {
    ManifestError::Invalid { key, line }
}

/// `value` as a string, or [`ManifestError::Invalid`].
fn string<'a>(
    value: &'a Spanned<DeValue<'a>>,
    key: &'static str,
    line: usize,
) -> Result<&'a str, ManifestError> {
    value.get_ref().as_str().ok_or_else(|| invalid(key, line))
}

/// `value` as a table, or [`ManifestError::Invalid`].
fn as_table<'a>(
    value: &'a Spanned<DeValue<'a>>,
    key: &'static str,
    line: usize,
) -> Result<&'a DeTable<'a>, ManifestError> {
    value.get_ref().as_table().ok_or_else(|| invalid(key, line))
}

/// `value` as an array, or [`ManifestError::Invalid`].
fn array<'a>(
    value: &'a Spanned<DeValue<'a>>,
    key: &'static str,
    line: usize,
) -> Result<&'a [Spanned<DeValue<'a>>], ManifestError> {
    value
        .get_ref()
        .as_array()
        .map(toml::de::DeArray::as_ref)
        .ok_or_else(|| invalid(key, line))
}

/// `value` as a boolean, or [`ManifestError::Invalid`].
fn boolean(
    value: &Spanned<DeValue<'_>>,
    key: &'static str,
    line: usize,
) -> Result<bool, ManifestError> {
    value.get_ref().as_bool().ok_or_else(|| invalid(key, line))
}

/// A TOML integer as a `u32`, or `None` when it is signed or does not fit.
fn unsigned(integer: &DeInteger<'_>) -> Option<u32> {
    let text = integer.as_str();
    let digits = text.strip_prefix('+').unwrap_or(text);
    u32::from_str_radix(digits, integer.radix()).ok()
}

/// `value` as a `u32`, or [`ManifestError::Invalid`].
fn integer(
    value: &Spanned<DeValue<'_>>,
    key: &'static str,
    line: usize,
) -> Result<u32, ManifestError> {
    value
        .get_ref()
        .as_integer()
        .and_then(unsigned)
        .ok_or_else(|| invalid(key, line))
}

/// `text` when it is at most `limit` bytes.
fn limited(key: &'static str, limit: usize, text: &str) -> Result<String, ManifestError> {
    let got = text.len();
    if got > limit {
        Err(ManifestError::TooLong { key, limit, got })
    } else {
        Ok(text.to_owned())
    }
}

/// A present resource, which must be in `1..=max`.
fn resource(
    value: &Spanned<DeValue<'_>>,
    key: &'static str,
    max: u32,
    line: usize,
) -> Result<u32, ManifestError> {
    let got = integer(value, key, line)?;
    if (1..=max).contains(&got) {
        Ok(got)
    } else {
        Err(ManifestError::Limit { key, max, got })
    }
}

/// Reads the root table. Every key is visited.
fn read_root(text: &str, table: &DeTable<'_>) -> Result<Parts, ManifestError> {
    let mut parts = Parts::bare();
    for (key, value) in table {
        let name = key.get_ref().as_ref();
        let line = line_of(text, key.span().start);
        match name {
            "plugin" => read_plugin(text, as_table(value, "plugin", line)?, &mut parts)?,
            "permissions" => {
                read_permissions(text, as_table(value, "permissions", line)?, &mut parts)?;
            }
            "resources" => read_resources(text, as_table(value, "resources", line)?, &mut parts)?,
            _ => return Err(unknown(name.to_owned(), line)),
        }
    }
    Ok(parts)
}

/// Reads `[plugin]`.
fn read_plugin(text: &str, table: &DeTable<'_>, parts: &mut Parts) -> Result<(), ManifestError> {
    for (key, value) in table {
        let name = key.get_ref().as_ref();
        let line = line_of(text, key.span().start);
        match name {
            "id" => parts.id = Some(plugin_id(value, line)?),
            "name" => parts.name = Some(plugin_name(value, line)?),
            "version" => parts.version = Some(plugin_version(value, line)?),
            "world" => parts.world = Some(plugin_world(value, line)?),
            "mode" => parts.mode = Some(plugin_mode(value, line)?),
            _ => return Err(unknown(format!("plugin.{name}"), line)),
        }
    }
    Ok(())
}

/// `plugin.id`.
fn plugin_id(value: &Spanned<DeValue<'_>>, line: usize) -> Result<PluginId, ManifestError> {
    PluginId::parse(string(value, "plugin.id", line)?).map_err(ManifestError::Id)
}

/// `plugin.name`.
fn plugin_name(value: &Spanned<DeValue<'_>>, line: usize) -> Result<String, ManifestError> {
    limited("plugin.name", MAX_NAME, string(value, "plugin.name", line)?)
}

/// `plugin.version`.
fn plugin_version(
    value: &Spanned<DeValue<'_>>,
    line: usize,
) -> Result<PluginVersion, ManifestError> {
    PluginVersion::parse(string(value, "plugin.version", line)?).map_err(ManifestError::Version)
}

/// `plugin.world`.
fn plugin_world(value: &Spanned<DeValue<'_>>, line: usize) -> Result<World, ManifestError> {
    World::parse(string(value, "plugin.world", line)?).map_err(ManifestError::World)
}

/// `plugin.mode`.
fn plugin_mode(value: &Spanned<DeValue<'_>>, line: usize) -> Result<Mode, ManifestError> {
    match string(value, "plugin.mode", line)? {
        "per-user" => Ok(Mode::PerUser),
        "server" => Ok(Mode::Server),
        _ => Err(invalid("plugin.mode", line)),
    }
}

/// Reads `[permissions]`. An absent child is an empty grant.
fn read_permissions(
    text: &str,
    table: &DeTable<'_>,
    parts: &mut Parts,
) -> Result<(), ManifestError> {
    for (key, value) in table {
        let name = key.get_ref().as_ref();
        let line = line_of(text, key.span().start);
        match name {
            "network" => parts.network = read_network(text, value, line)?,
            "gunmetal" => parts.scopes = read_gunmetal(text, value, line)?,
            "secrets" => parts.secrets = read_secrets(text, value, line)?,
            _ => return Err(unknown(format!("permissions.{name}"), line)),
        }
    }
    Ok(())
}

/// `permissions.network`, an array of host grants.
fn read_network(
    text: &str,
    value: &Spanned<DeValue<'_>>,
    line: usize,
) -> Result<Vec<NetworkGrant>, ManifestError> {
    let mut grants = Vec::new();
    for item in array(value, "permissions.network", line)? {
        let row = as_table(
            item,
            "permissions.network",
            line_of(text, item.span().start),
        )?;
        push_host(&mut grants, read_grant(text, row)?)?;
    }
    Ok(grants)
}

/// One network row. `host` and `why` are required; any other key is refused.
fn read_grant(text: &str, table: &DeTable<'_>) -> Result<NetworkGrant, ManifestError> {
    let mut host = None;
    let mut why = None;
    for (key, value) in table {
        let name = key.get_ref().as_ref();
        let line = line_of(text, key.span().start);
        match name {
            "host" => host = Some(read_host(value, line)?),
            "why" => why = Some(read_why(value, line)?),
            _ => return Err(unknown(format!("permissions.network.{name}"), line)),
        }
    }
    Ok(NetworkGrant {
        host: required(host, "permissions.network.host")?,
        why: required(why, "permissions.network.why")?,
    })
}

/// A public host grant.
fn read_host(value: &Spanned<DeValue<'_>>, line: usize) -> Result<ExactHost, ManifestError> {
    ExactHost::parse_public(string(value, "permissions.network.host", line)?)
        .map_err(ManifestError::Host)
}

/// The reason shown for a host grant.
fn read_why(value: &Spanned<DeValue<'_>>, line: usize) -> Result<String, ManifestError> {
    limited(
        "permissions.network.why",
        MAX_WHY,
        string(value, "permissions.network.why", line)?,
    )
}

/// Refuses a host already granted, compared in its canonical spelling.
fn push_host(grants: &mut Vec<NetworkGrant>, grant: NetworkGrant) -> Result<(), ManifestError> {
    if grants.iter().any(|existing| existing.host == grant.host) {
        return Err(ManifestError::DuplicateHost {
            host: grant.host.as_str().to_owned(),
        });
    }
    grants.push(grant);
    Ok(())
}

/// `permissions.gunmetal`. Absent `scopes` means none.
fn read_gunmetal(
    text: &str,
    value: &Spanned<DeValue<'_>>,
    line: usize,
) -> Result<Vec<Scope>, ManifestError> {
    let table = as_table(value, "permissions.gunmetal", line)?;
    let mut scopes = Vec::new();
    for (key, value) in table {
        let name = key.get_ref().as_ref();
        let key_line = line_of(text, key.span().start);
        match name {
            "scopes" => scopes = read_scopes(text, value, key_line)?,
            _ => return Err(unknown(format!("permissions.gunmetal.{name}"), key_line)),
        }
    }
    Ok(scopes)
}

/// `permissions.gunmetal.scopes`.
fn read_scopes(
    text: &str,
    value: &Spanned<DeValue<'_>>,
    line: usize,
) -> Result<Vec<Scope>, ManifestError> {
    let mut scopes = Vec::new();
    for item in array(value, "permissions.gunmetal.scopes", line)? {
        let spelling = string(
            item,
            "permissions.gunmetal.scopes",
            line_of(text, item.span().start),
        )?;
        let scope = Scope::parse(spelling).map_err(ManifestError::Scope)?;
        if scopes.contains(&scope) {
            return Err(ManifestError::DuplicateScope(scope));
        }
        scopes.push(scope);
    }
    Ok(scopes)
}

/// `permissions.secrets`, an array of secret declarations.
fn read_secrets(
    text: &str,
    value: &Spanned<DeValue<'_>>,
    line: usize,
) -> Result<Vec<SecretDecl>, ManifestError> {
    let mut secrets = Vec::new();
    for item in array(value, "permissions.secrets", line)? {
        let row = as_table(
            item,
            "permissions.secrets",
            line_of(text, item.span().start),
        )?;
        push_secret(&mut secrets, read_secret(text, row)?)?;
    }
    Ok(secrets)
}

/// One secret row. `name`, `label` and `per_user` are required.
fn read_secret(text: &str, table: &DeTable<'_>) -> Result<SecretDecl, ManifestError> {
    let mut name = None;
    let mut label = None;
    let mut per_user = None;
    for (key, value) in table {
        let field = key.get_ref().as_ref();
        let line = line_of(text, key.span().start);
        match field {
            "name" => name = Some(secret_name(value, line)?),
            "label" => label = Some(secret_label(value, line)?),
            "per_user" => per_user = Some(boolean(value, "permissions.secrets.per_user", line)?),
            _ => return Err(unknown(format!("permissions.secrets.{field}"), line)),
        }
    }
    Ok(SecretDecl {
        name: required(name, "permissions.secrets.name")?,
        label: required(label, "permissions.secrets.label")?,
        per_user: required(per_user, "permissions.secrets.per_user")?,
    })
}

/// A secret's name.
fn secret_name(value: &Spanned<DeValue<'_>>, line: usize) -> Result<String, ManifestError> {
    limited(
        "permissions.secrets.name",
        MAX_SECRET_NAME,
        string(value, "permissions.secrets.name", line)?,
    )
}

/// A secret's label.
fn secret_label(value: &Spanned<DeValue<'_>>, line: usize) -> Result<String, ManifestError> {
    limited(
        "permissions.secrets.label",
        MAX_SECRET_LABEL,
        string(value, "permissions.secrets.label", line)?,
    )
}

/// Refuses a secret name already declared.
fn push_secret(secrets: &mut Vec<SecretDecl>, secret: SecretDecl) -> Result<(), ManifestError> {
    if secrets.iter().any(|existing| existing.name == secret.name) {
        return Err(ManifestError::DuplicateSecret { name: secret.name });
    }
    secrets.push(secret);
    Ok(())
}

/// Reads `[resources]`. An absent field keeps its default.
fn read_resources(text: &str, table: &DeTable<'_>, parts: &mut Parts) -> Result<(), ManifestError> {
    for (key, value) in table {
        let name = key.get_ref().as_ref();
        let line = line_of(text, key.span().start);
        match name {
            "memory_mib" => {
                parts.memory_mib = resource(value, "resources.memory_mib", MAX_MEMORY_MIB, line)?;
            }
            "deadline_secs" => {
                parts.deadline_secs =
                    resource(value, "resources.deadline_secs", MAX_DEADLINE_SECS, line)?;
            }
            "kv_mib" => parts.kv_mib = resource(value, "resources.kv_mib", MAX_KV_MIB, line)?,
            "outbound_per_minute" => {
                parts.outbound_per_minute = resource(
                    value,
                    "resources.outbound_per_minute",
                    MAX_OUTBOUND_PER_MINUTE,
                    line,
                )?;
            }
            _ => return Err(unknown(format!("resources.{name}"), line)),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        MAX_NAME, MAX_OUTBOUND_PER_MINUTE, MAX_SECRET_LABEL, MAX_SECRET_NAME, MAX_WHY,
        ManifestError, parse,
    };
    use crate::id::{IdError, PluginId};
    use crate::record::{
        DEFAULT_DEADLINE_SECS, DEFAULT_KV_MIB, DEFAULT_MEMORY_MIB, DEFAULT_OUTBOUND_PER_MINUTE,
        ExactHost, HostRefuse, MAX_DEADLINE_SECS, MAX_KV_MIB, MAX_MEMORY_MIB, Manifest, Mode,
        NetworkGrant, SecretDecl,
    };
    use crate::scope::{Scope, ScopeError};
    use crate::version::{PluginVersion, VersionError};
    use crate::world::{World, WorldError};
    use gunmetal_egress::destination::HostError;
    use proptest::prelude::*;
    // Qodana does not expand `proptest!` or resolve these through `prelude::*`.
    use proptest::prop_assert_eq;
    use proptest::prop_assume;

    const PLUGIN: &str = "\
[plugin]
id = \"org.listenbrainz.scrobbler\"
name = \"ListenBrainz\"
version = \"1.2.0\"
world = \"scrobbler@1\"
mode = \"per-user\"
";

    const FIXTURE: &str = "\
[plugin]
id = \"org.listenbrainz.scrobbler\"
name = \"ListenBrainz\"
version = \"1.2.0\"
world = \"scrobbler@1\"
mode = \"per-user\"

[[permissions.network]]
host = \"api.listenbrainz.org\"
why = \"Send the tracks you play to your ListenBrainz account\"

[permissions.gunmetal]
scopes = [\"events:self\"]

[[permissions.secrets]]
name = \"token\"
label = \"Your ListenBrainz user token\"
per_user = true

[resources]
memory_mib = 32
";

    fn id() -> PluginId {
        PluginId::parse("org.listenbrainz.scrobbler").expect("id")
    }

    fn version(text: &str) -> PluginVersion {
        PluginVersion::parse(text).expect("version")
    }

    fn host(text: &str) -> ExactHost {
        ExactHost::parse_public(text).expect("host")
    }

    fn grant(name: &str, why: &str) -> NetworkGrant {
        NetworkGrant {
            host: host(name),
            why: why.to_owned(),
        }
    }

    fn bare() -> Manifest {
        Manifest {
            id: id(),
            name: "ListenBrainz".to_owned(),
            version: version("1.2.0"),
            world: World::Scrobbler,
            mode: Mode::PerUser,
            network: Vec::new(),
            scopes: Vec::new(),
            secrets: Vec::new(),
            memory_mib: 64,
            deadline_secs: 5,
            kv_mib: 1,
            outbound_per_minute: 60,
        }
    }

    fn fixture() -> Manifest {
        Manifest {
            id: id(),
            name: "ListenBrainz".to_owned(),
            version: version("1.2.0"),
            world: World::Scrobbler,
            mode: Mode::PerUser,
            network: vec![grant(
                "api.listenbrainz.org",
                "Send the tracks you play to your ListenBrainz account",
            )],
            scopes: vec![Scope::EventsSelf],
            secrets: vec![SecretDecl {
                name: "token".to_owned(),
                label: "Your ListenBrainz user token".to_owned(),
                per_user: true,
            }],
            memory_mib: 32,
            deadline_secs: 5,
            kv_mib: 1,
            outbound_per_minute: 60,
        }
    }

    /// Verifies: SEC-EXT-025, SEC-EXT-023, SEC-EXT-033
    #[test]
    fn the_text_limits_are_these_numbers() {
        assert_eq!(MAX_NAME, 64);
        assert_eq!(MAX_NAME, crate::record::MAX_NAME);
        assert_eq!(MAX_WHY, 160);
        assert_eq!(MAX_WHY, crate::record::MAX_WHY);
        assert_eq!(MAX_SECRET_NAME, 32);
        assert_eq!(MAX_SECRET_LABEL, 80);
        assert_eq!(MAX_OUTBOUND_PER_MINUTE, 60);
        assert_eq!(DEFAULT_MEMORY_MIB, 64);
        assert_eq!(MAX_MEMORY_MIB, 256);
        assert_eq!(DEFAULT_DEADLINE_SECS, 5);
        assert_eq!(MAX_DEADLINE_SECS, 60);
        assert_eq!(DEFAULT_KV_MIB, 1);
        assert_eq!(MAX_KV_MIB, 16);
        assert_eq!(DEFAULT_OUTBOUND_PER_MINUTE, 60);
    }

    /// Verifies: SEC-EXT-025
    #[test]
    fn the_security_doc_fixture_is_this_manifest() {
        assert_eq!(parse(FIXTURE), Ok(fixture()));
    }

    /// Verifies: SEC-EXT-025
    #[test]
    fn an_absent_permission_is_not_granted() {
        let expected = bare();
        assert_eq!(parse(PLUGIN), Ok(expected.clone()));
        assert_eq!(
            parse(&format!("{PLUGIN}[permissions]\n")),
            Ok(expected.clone())
        );
        assert_eq!(
            parse(&format!("permissions.network = []\n{PLUGIN}")),
            Ok(expected.clone())
        );
        assert_eq!(
            parse(&format!("{PLUGIN}[permissions.gunmetal]\n")),
            Ok(expected.clone())
        );
        assert_eq!(
            parse(&format!("{PLUGIN}[permissions.gunmetal]\nscopes = []\n")),
            Ok(expected.clone())
        );
        assert_eq!(
            parse(&format!("permissions.secrets = []\n{PLUGIN}")),
            Ok(expected.clone())
        );
        assert_eq!(
            parse(&format!("{PLUGIN}permissions.network = []\n")),
            Err(ManifestError::UnknownKey {
                key: "plugin.permissions".to_owned(),
                line: 7,
            })
        );
        assert_eq!(parse(&format!("{PLUGIN}[resources]\n")), Ok(expected));
    }

    /// Verifies: SEC-EXT-025
    #[test]
    fn an_unknown_key_is_refused_by_its_dotted_name() {
        assert_eq!(
            parse("[plugin]\nid = \"org.listenbrainz.scrobbler\"\nextra = \"no\""),
            Err(ManifestError::UnknownKey {
                key: "plugin.extra".to_owned(),
                line: 3,
            })
        );
        assert_eq!(
            parse("extra = 1\n"),
            Err(ManifestError::UnknownKey {
                key: "extra".to_owned(),
                line: 1,
            })
        );
        assert_eq!(
            parse("[permissions.files]\npath = \"/\"\n"),
            Err(ManifestError::UnknownKey {
                key: "permissions.files".to_owned(),
                line: 1,
            })
        );
        assert_eq!(
            parse("[resources]\nmemory_mib = 32\ncpu = 1\n"),
            Err(ManifestError::UnknownKey {
                key: "resources.cpu".to_owned(),
                line: 3,
            })
        );
        assert_eq!(
            parse("[[permissions.network]]\nextra = \"no\"\n"),
            Err(ManifestError::UnknownKey {
                key: "permissions.network.extra".to_owned(),
                line: 2,
            })
        );
        assert_eq!(
            parse("[permissions.gunmetal]\nextra = 1\n"),
            Err(ManifestError::UnknownKey {
                key: "permissions.gunmetal.extra".to_owned(),
                line: 2,
            })
        );
        assert_eq!(
            parse("[[permissions.secrets]]\nextra = 1\n"),
            Err(ManifestError::UnknownKey {
                key: "permissions.secrets.extra".to_owned(),
                line: 2,
            })
        );
        assert_eq!(
            parse("[plugin]\nID = \"org.listenbrainz.scrobbler\"\n"),
            Err(ManifestError::UnknownKey {
                key: "plugin.ID".to_owned(),
                line: 2,
            })
        );
    }

    /// Verifies: SEC-EXT-025
    #[test]
    fn an_unknown_world_is_refused() {
        for world in ["lyrics@1", "scrobbler", "scrobbler@2", ""] {
            assert_eq!(
                parse(
                    &format!("{PLUGIN}world = \"{world}\"\n")
                        .replace("world = \"scrobbler@1\"\n", "",)
                ),
                Err(ManifestError::World(WorldError::Unknown)),
                "{world}"
            );
        }
        assert_eq!(
            parse(&PLUGIN.replace("world = \"scrobbler@1\"", "world = \"lyrics@1\"")),
            Err(ManifestError::World(WorldError::Unknown))
        );
    }

    /// Verifies: SEC-EXT-010
    #[test]
    fn an_administrator_scope_is_refused() {
        assert_eq!(
            parse(&format!(
                "{PLUGIN}[permissions.gunmetal]\nscopes = [\"admin:library\"]\n"
            )),
            Err(ManifestError::Scope(ScopeError::Admin))
        );
    }

    /// Verifies: SEC-EXT-026
    #[test]
    fn an_address_is_not_a_public_host() {
        assert_eq!(
            parse(&format!(
                "{PLUGIN}[[permissions.network]]\nhost = \"1.2.3.4\"\nwhy = \"no\"\n"
            )),
            Err(ManifestError::Host(HostRefuse::Address))
        );
        assert_eq!(
            parse(&format!(
                "{PLUGIN}[[permissions.network]]\nhost = \"::1\"\nwhy = \"no\"\n"
            )),
            Err(ManifestError::Host(HostRefuse::Address))
        );
    }

    /// Verifies: SEC-EXT-026
    #[test]
    fn a_wildcard_host_is_refused() {
        assert_eq!(
            parse(&format!(
                "{PLUGIN}[[permissions.network]]\nhost = \"*.listenbrainz.org\"\nwhy = \"no\"\n"
            )),
            Err(ManifestError::Host(HostRefuse::Wildcard))
        );
        assert_eq!(
            parse(&format!(
                "{PLUGIN}[[permissions.network]]\nhost = \"api.*.org\"\nwhy = \"no\"\n"
            )),
            Err(ManifestError::Host(HostRefuse::Wildcard))
        );
    }

    /// Verifies: SEC-EXT-025
    #[test]
    fn a_missing_plugin_id_is_refused() {
        assert_eq!(parse(""), Err(ManifestError::Missing { key: "plugin.id" }));
        assert_eq!(
            parse("[plugin]\nname = \"ListenBrainz\"\n"),
            Err(ManifestError::Missing { key: "plugin.id" })
        );
        assert_eq!(
            parse("[resources]\nmemory_mib = 32\n"),
            Err(ManifestError::Missing { key: "plugin.id" })
        );
    }

    /// Verifies: SEC-EXT-023, SEC-EXT-025
    #[test]
    fn memory_above_the_cap_is_refused() {
        assert_eq!(
            parse(&format!("{PLUGIN}[resources]\nmemory_mib = 257\n")),
            Err(ManifestError::Limit {
                key: "resources.memory_mib",
                max: 256,
                got: 257,
            })
        );
    }

    /// Verifies: SEC-EXT-026
    #[test]
    fn the_same_host_twice_is_refused() {
        assert_eq!(
            parse(&format!(
                "{PLUGIN}[[permissions.network]]\nhost = \"api.listenbrainz.org\"\nwhy = \"one\"\n[[permissions.network]]\nhost = \"api.listenbrainz.org\"\nwhy = \"two\"\n"
            )),
            Err(ManifestError::DuplicateHost {
                host: "api.listenbrainz.org".to_owned(),
            })
        );
        assert_eq!(
            parse(&format!(
                "{PLUGIN}[[permissions.network]]\nhost = \"api.listenbrainz.org\"\nwhy = \"one\"\n[[permissions.network]]\nhost = \"API.ListenBrainz.Org\"\nwhy = \"two\"\n"
            )),
            Err(ManifestError::DuplicateHost {
                host: "api.listenbrainz.org".to_owned(),
            })
        );
    }

    /// Verifies: SEC-EXT-025
    #[test]
    fn text_that_is_not_toml_is_a_syntax_error() {
        assert_eq!(
            parse("not toml"),
            Err(ManifestError::Syntax {
                line: 1,
                message: "key with no value, expected `=`".to_owned(),
            })
        );
        assert_eq!(
            parse("a = 1\n[\n"),
            Err(ManifestError::Syntax {
                line: 2,
                message: "unquoted keys cannot be empty, expected letters, numbers, `-`, `_`"
                    .to_owned(),
            })
        );
        assert_eq!(
            parse("id = \"a\"\nid = \"b\"\n"),
            Err(ManifestError::Syntax {
                line: 2,
                message: "duplicate key".to_owned(),
            })
        );
        assert_eq!(
            parse("\r"),
            Err(ManifestError::Syntax {
                line: 1,
                message: "carriage return must be followed by newline, expected newline".to_owned(),
            })
        );
        let deep = (0..81)
            .map(|index| format!("k{index}"))
            .collect::<Vec<_>>()
            .join(".");
        assert_eq!(
            parse(&format!("{deep} = 1\n")),
            Err(ManifestError::Syntax {
                line: 1,
                message: "recursion limit".to_owned(),
            })
        );
    }

    /// Verifies: SEC-EXT-025, SEC-EXT-023, SEC-EXT-033
    #[test]
    fn a_server_plugin_with_every_grant_is_kept_whole() {
        let text = "\
[plugin]
id = \"org.musicbrainz.lookup\"
name = \"MusicBrainz\"
version = \"0.1.0\"
world = \"music-metadata-provider@1\"
mode = \"server\"

[[permissions.network]]
host = \"MusicBrainz.Org\"
why = \"Look up a release\"

[[permissions.network]]
host = \"coverartarchive.org\"
why = \"Fetch cover art\"

[permissions.gunmetal]
scopes = [\"library:read\", \"events:self\"]

[[permissions.secrets]]
name = \"token\"
label = \"Token\"
per_user = false

[[permissions.secrets]]
name = \"Token\"
label = \"User\"
per_user = true

[resources]
memory_mib = 256
deadline_secs = 60
kv_mib = 16
outbound_per_minute = 1
";
        assert_eq!(
            parse(text),
            Ok(Manifest {
                id: PluginId::parse("org.musicbrainz.lookup").expect("id"),
                name: "MusicBrainz".to_owned(),
                version: version("0.1.0"),
                world: World::MusicMetadataProvider,
                mode: Mode::Server,
                network: vec![
                    grant("musicbrainz.org", "Look up a release"),
                    grant("coverartarchive.org", "Fetch cover art"),
                ],
                scopes: vec![Scope::LibraryRead, Scope::EventsSelf],
                secrets: vec![
                    SecretDecl {
                        name: "token".to_owned(),
                        label: "Token".to_owned(),
                        per_user: false,
                    },
                    SecretDecl {
                        name: "Token".to_owned(),
                        label: "User".to_owned(),
                        per_user: true,
                    },
                ],
                memory_mib: 256,
                deadline_secs: 60,
                kv_mib: 16,
                outbound_per_minute: 1,
            })
        );
    }

    /// Verifies: SEC-EXT-025
    #[test]
    fn each_required_plugin_field_is_required() {
        let cases = [
            ("id = \"org.listenbrainz.scrobbler\"\n", "plugin.name"),
            ("name = \"ListenBrainz\"\n", "plugin.id"),
            (
                "id = \"org.listenbrainz.scrobbler\"\nname = \"ListenBrainz\"\n",
                "plugin.version",
            ),
            (
                "id = \"org.listenbrainz.scrobbler\"\nname = \"ListenBrainz\"\nversion = \"1.2.0\"\n",
                "plugin.world",
            ),
            (
                "id = \"org.listenbrainz.scrobbler\"\nname = \"ListenBrainz\"\nversion = \"1.2.0\"\nworld = \"scrobbler@1\"\n",
                "plugin.mode",
            ),
        ];
        for (body, key) in cases {
            assert_eq!(
                parse(&format!("[plugin]\n{body}")),
                Err(ManifestError::Missing { key }),
                "{key}"
            );
        }
    }

    /// Verifies: SEC-EXT-025
    #[test]
    fn a_known_key_of_the_wrong_type_is_invalid() {
        let cases = [
            ("plugin = \"no\"\n", "plugin", 1),
            ("[plugin]\nid = 1\n", "plugin.id", 2),
            ("[plugin]\nname = 1\n", "plugin.name", 2),
            ("[plugin]\nversion = 1\n", "plugin.version", 2),
            ("[plugin]\nworld = 1\n", "plugin.world", 2),
            ("[plugin]\nmode = true\n", "plugin.mode", 2),
            ("permissions = 1\n", "permissions", 1),
            (
                "permissions.network = \"api.listenbrainz.org\"\n",
                "permissions.network",
                1,
            ),
            (
                "[permissions.network]\nhost = \"api.listenbrainz.org\"\n",
                "permissions.network",
                1,
            ),
            (
                "[permissions]\ngunmetal = \"events:self\"\n",
                "permissions.gunmetal",
                2,
            ),
            (
                "[permissions.gunmetal]\nscopes = \"events:self\"\n",
                "permissions.gunmetal.scopes",
                2,
            ),
            (
                "permissions.secrets = \"token\"\n",
                "permissions.secrets",
                1,
            ),
            ("resources = 1\n", "resources", 1),
            (
                "[resources]\nmemory_mib = \"32\"\n",
                "resources.memory_mib",
                2,
            ),
            (
                "[resources]\ndeadline_secs = true\n",
                "resources.deadline_secs",
                2,
            ),
            ("[resources]\nkv_mib = 1.5\n", "resources.kv_mib", 2),
            (
                "[resources]\noutbound_per_minute = 1979-05-27\n",
                "resources.outbound_per_minute",
                2,
            ),
        ];
        for (text, key, line) in cases {
            assert_eq!(
                parse(text),
                Err(ManifestError::Invalid { key, line }),
                "{text}"
            );
        }
    }

    /// Verifies: SEC-EXT-025
    #[test]
    fn an_id_version_or_mode_is_refused_for_the_reason_it_fails() {
        let long = format!("a.{}", "b".repeat(64));
        let cases = [
            ("", ManifestError::Id(IdError::Empty)),
            ("listenbrainz", ManifestError::Id(IdError::Shape)),
            (
                "Org.Scrobbler",
                ManifestError::Id(IdError::BadCharacter { index: 0 }),
            ),
            (
                long.as_str(),
                ManifestError::Id(IdError::TooLong { got: long.len() }),
            ),
        ];
        for (text, expected) in cases {
            assert_eq!(
                parse(&PLUGIN.replace("org.listenbrainz.scrobbler", text)),
                Err(expected),
                "{text}"
            );
        }
        assert_eq!(
            parse(&PLUGIN.replace("1.2.0", "1.2")),
            Err(ManifestError::Version(VersionError::Shape))
        );
        assert_eq!(
            parse(&PLUGIN.replace("1.2.0", "01.2.0")),
            Err(ManifestError::Version(VersionError::LeadingZero))
        );
        assert_eq!(
            parse(&PLUGIN.replace("1.2.0", "1.2.a")),
            Err(ManifestError::Version(VersionError::Component))
        );
        assert_eq!(
            parse(&PLUGIN.replace("per-user", "server")),
            Ok(Manifest {
                mode: Mode::Server,
                ..bare()
            })
        );
        assert_eq!(
            parse(&PLUGIN.replace("per-user", "PER-USER")),
            Err(ManifestError::Invalid {
                key: "plugin.mode",
                line: 6,
            })
        );
        assert_eq!(
            parse(&PLUGIN.replace("per-user", "per_user")),
            Err(ManifestError::Invalid {
                key: "plugin.mode",
                line: 6,
            })
        );
    }

    /// Verifies: SEC-EXT-025
    #[test]
    fn a_name_or_why_past_its_limit_is_refused_and_the_limit_is_kept() {
        let name = "a".repeat(MAX_NAME);
        let mut kept = bare();
        kept.name.clone_from(&name);
        assert_eq!(parse(&PLUGIN.replace("ListenBrainz", &name)), Ok(kept));
        let over = "a".repeat(MAX_NAME + 1);
        assert_eq!(
            parse(&PLUGIN.replace("ListenBrainz", &over)),
            Err(ManifestError::TooLong {
                key: "plugin.name",
                limit: MAX_NAME,
                got: over.len(),
            })
        );
        let wide = "é".repeat(33);
        assert_eq!(
            parse(&PLUGIN.replace("ListenBrainz", &wide)),
            Err(ManifestError::TooLong {
                key: "plugin.name",
                limit: MAX_NAME,
                got: wide.len(),
            })
        );
        let why = "w".repeat(MAX_WHY);
        let mut with_why = bare();
        with_why.network = vec![grant("api.listenbrainz.org", &why)];
        assert_eq!(
            parse(&format!(
                "{PLUGIN}[[permissions.network]]\nhost = \"api.listenbrainz.org\"\nwhy = \"{why}\"\n"
            )),
            Ok(with_why)
        );
        let why_over = "w".repeat(MAX_WHY + 1);
        assert_eq!(
            parse(&format!(
                "{PLUGIN}[[permissions.network]]\nhost = \"api.listenbrainz.org\"\nwhy = \"{why_over}\"\n"
            )),
            Err(ManifestError::TooLong {
                key: "permissions.network.why",
                limit: MAX_WHY,
                got: why_over.len(),
            })
        );
    }

    /// Verifies: SEC-EXT-026
    #[test]
    fn a_host_that_is_not_a_public_name_is_refused() {
        let cases = [
            ("localhost", HostRefuse::NoDot),
            ("bad_host.com", HostRefuse::Name(HostError::BadLabel)),
            (
                "api.listenbrainz.org.",
                HostRefuse::Name(HostError::BadLabel),
            ),
            ("", HostRefuse::Name(HostError::BadLabel)),
        ];
        for (text, reason) in cases {
            assert_eq!(
                parse(&format!(
                    "{PLUGIN}[[permissions.network]]\nhost = \"{text}\"\nwhy = \"no\"\n"
                )),
                Err(ManifestError::Host(reason)),
                "{text}"
            );
        }
        let mut expected = bare();
        expected.network = vec![grant("api.listenbrainz.org", "because")];
        assert_eq!(
            parse(&format!(
                "{PLUGIN}[[permissions.network]]\nhost = \"API.ListenBrainz.Org\"\nwhy = \"because\"\n"
            )),
            Ok(expected)
        );
    }

    /// Verifies: SEC-EXT-025
    #[test]
    fn a_network_row_must_name_a_host_and_a_why() {
        assert_eq!(
            parse(&format!("{PLUGIN}[[permissions.network]]\nwhy = \"no\"\n")),
            Err(ManifestError::Missing {
                key: "permissions.network.host",
            })
        );
        assert_eq!(
            parse(&format!(
                "{PLUGIN}[[permissions.network]]\nhost = \"api.listenbrainz.org\"\n"
            )),
            Err(ManifestError::Missing {
                key: "permissions.network.why",
            })
        );
        assert_eq!(
            parse(&format!(
                "{PLUGIN}[[permissions.network]]\nhost = 1\nwhy = \"no\"\n"
            )),
            Err(ManifestError::Invalid {
                key: "permissions.network.host",
                line: 8,
            })
        );
        assert_eq!(
            parse(&format!(
                "{PLUGIN}[[permissions.network]]\nhost = \"api.listenbrainz.org\"\nwhy = 1\n"
            )),
            Err(ManifestError::Invalid {
                key: "permissions.network.why",
                line: 9,
            })
        );
        assert_eq!(
            parse("permissions.network = [\n\"no\"\n]\n"),
            Err(ManifestError::Invalid {
                key: "permissions.network",
                line: 2,
            })
        );
        assert_eq!(
            parse(&format!(
                "{PLUGIN}[permissions.gunmetal]\nscopes = [\n1\n]\n"
            )),
            Err(ManifestError::Invalid {
                key: "permissions.gunmetal.scopes",
                line: 9,
            })
        );
    }

    /// Verifies: SEC-EXT-025
    #[test]
    fn a_scope_outside_the_vocabulary_is_refused_and_a_repeat_is_refused() {
        let cases = [
            ("", ScopeError::Empty),
            ("*", ScopeError::Wildcard),
            ("library:*", ScopeError::Wildcard),
            ("all", ScopeError::Wildcard),
            ("events:self ", ScopeError::Unknown),
            ("credentials:read", ScopeError::Unknown),
        ];
        for (text, reason) in cases {
            assert_eq!(
                parse(&format!(
                    "{PLUGIN}[permissions.gunmetal]\nscopes = [\"{text}\"]\n"
                )),
                Err(ManifestError::Scope(reason)),
                "{text:?}"
            );
        }
        assert_eq!(
            parse(&format!(
                "{PLUGIN}[permissions.gunmetal]\nscopes = [\"events:self\", \"history:read\", \"events:self\"]\n"
            )),
            Err(ManifestError::DuplicateScope(Scope::EventsSelf))
        );
    }

    /// Verifies: SEC-EXT-025
    #[test]
    fn a_secret_must_name_a_label_and_whether_it_is_per_user() {
        let name = "n".repeat(MAX_SECRET_NAME);
        let label = "l".repeat(MAX_SECRET_LABEL);
        let mut expected = bare();
        expected.secrets = vec![SecretDecl {
            name: name.clone(),
            label: label.clone(),
            per_user: false,
        }];
        assert_eq!(
            parse(&format!(
                "{PLUGIN}[[permissions.secrets]]\nname = \"{name}\"\nlabel = \"{label}\"\nper_user = false\n"
            )),
            Ok(expected)
        );
        let name_over = "n".repeat(MAX_SECRET_NAME + 1);
        assert_eq!(
            parse(&format!(
                "{PLUGIN}[[permissions.secrets]]\nname = \"{name_over}\"\nlabel = \"l\"\nper_user = true\n"
            )),
            Err(ManifestError::TooLong {
                key: "permissions.secrets.name",
                limit: MAX_SECRET_NAME,
                got: name_over.len(),
            })
        );
        let label_over = "l".repeat(MAX_SECRET_LABEL + 1);
        assert_eq!(
            parse(&format!(
                "{PLUGIN}[[permissions.secrets]]\nname = \"n\"\nlabel = \"{label_over}\"\nper_user = true\n"
            )),
            Err(ManifestError::TooLong {
                key: "permissions.secrets.label",
                limit: MAX_SECRET_LABEL,
                got: label_over.len(),
            })
        );
        assert_eq!(
            parse(&format!(
                "{PLUGIN}[[permissions.secrets]]\nlabel = \"l\"\nper_user = true\n"
            )),
            Err(ManifestError::Missing {
                key: "permissions.secrets.name",
            })
        );
        assert_eq!(
            parse(&format!(
                "{PLUGIN}[[permissions.secrets]]\nname = \"n\"\nper_user = true\n"
            )),
            Err(ManifestError::Missing {
                key: "permissions.secrets.label",
            })
        );
        assert_eq!(
            parse(&format!(
                "{PLUGIN}[[permissions.secrets]]\nname = \"n\"\nlabel = \"l\"\n"
            )),
            Err(ManifestError::Missing {
                key: "permissions.secrets.per_user",
            })
        );
    }

    /// Verifies: SEC-EXT-025
    #[test]
    fn a_secret_of_the_wrong_shape_is_refused() {
        assert_eq!(
            parse(&format!(
                "{PLUGIN}[[permissions.secrets]]\nname = 1\nlabel = \"l\"\nper_user = true\n"
            )),
            Err(ManifestError::Invalid {
                key: "permissions.secrets.name",
                line: 8,
            })
        );
        assert_eq!(
            parse(&format!(
                "{PLUGIN}[[permissions.secrets]]\nname = \"n\"\nlabel = 1\nper_user = true\n"
            )),
            Err(ManifestError::Invalid {
                key: "permissions.secrets.label",
                line: 9,
            })
        );
        assert_eq!(
            parse(&format!(
                "{PLUGIN}[[permissions.secrets]]\nname = \"n\"\nlabel = \"l\"\nper_user = \"true\"\n"
            )),
            Err(ManifestError::Invalid {
                key: "permissions.secrets.per_user",
                line: 10,
            })
        );
        assert_eq!(
            parse(&format!(
                "{PLUGIN}[[permissions.secrets]]\nname = \"token\"\nlabel = \"a\"\nper_user = true\n[[permissions.secrets]]\nname = \"token\"\nlabel = \"b\"\nper_user = false\n"
            )),
            Err(ManifestError::DuplicateSecret {
                name: "token".to_owned(),
            })
        );
        assert_eq!(
            parse("permissions.secrets = [\n1\n]\n"),
            Err(ManifestError::Invalid {
                key: "permissions.secrets",
                line: 2,
            })
        );
    }

    /// Verifies: SEC-EXT-023, SEC-EXT-033, SEC-EXT-025
    #[test]
    fn a_resource_outside_its_range_is_refused_and_the_bounds_are_kept() {
        let bounds = [
            ("memory_mib", 1, 256, "resources.memory_mib"),
            ("deadline_secs", 1, 60, "resources.deadline_secs"),
            ("kv_mib", 1, 16, "resources.kv_mib"),
            (
                "outbound_per_minute",
                1,
                60,
                "resources.outbound_per_minute",
            ),
        ];
        for (field, low, high, key) in bounds {
            let mut at_low = bare();
            let mut at_high = bare();
            if field == "memory_mib" {
                at_low.memory_mib = low;
                at_high.memory_mib = high;
            } else if field == "deadline_secs" {
                at_low.deadline_secs = low;
                at_high.deadline_secs = high;
            } else if field == "kv_mib" {
                at_low.kv_mib = low;
                at_high.kv_mib = high;
            } else {
                at_low.outbound_per_minute = low;
                at_high.outbound_per_minute = high;
            }
            assert_eq!(
                parse(&format!("{PLUGIN}[resources]\n{field} = {low}\n")),
                Ok(at_low),
                "{field} low"
            );
            assert_eq!(
                parse(&format!("{PLUGIN}[resources]\n{field} = {high}\n")),
                Ok(at_high),
                "{field} high"
            );
            assert_eq!(
                parse(&format!("{PLUGIN}[resources]\n{field} = 0\n")),
                Err(ManifestError::Limit {
                    key,
                    max: high,
                    got: 0
                }),
                "{field} zero"
            );
            assert_eq!(
                parse(&format!("{PLUGIN}[resources]\n{field} = {}\n", high + 1)),
                Err(ManifestError::Limit {
                    key,
                    max: high,
                    got: high + 1,
                }),
                "{field} over"
            );
        }
        assert_eq!(
            parse(&format!("{PLUGIN}[resources]\nmemory_mib = 4294967295\n")),
            Err(ManifestError::Limit {
                key: "resources.memory_mib",
                max: 256,
                got: 4_294_967_295,
            })
        );
        assert_eq!(
            parse(&format!("{PLUGIN}[resources]\nmemory_mib = 4294967296\n")),
            Err(ManifestError::Invalid {
                key: "resources.memory_mib",
                line: 8,
            })
        );
        assert_eq!(
            parse(&format!("{PLUGIN}[resources]\nmemory_mib = -1\n")),
            Err(ManifestError::Invalid {
                key: "resources.memory_mib",
                line: 8,
            })
        );
    }

    /// Verifies: SEC-EXT-023
    #[test]
    fn an_integer_is_read_in_its_radix_including_a_leading_plus() {
        let mut expected = bare();
        expected.memory_mib = 32;
        for spelling in ["+32", "0x20", "0b100000", "0o40", "3_2"] {
            assert_eq!(
                parse(&format!("{PLUGIN}[resources]\nmemory_mib = {spelling}\n")),
                Ok(expected.clone()),
                "{spelling}"
            );
        }
        assert_eq!(
            parse(&format!("{PLUGIN}[resources]\nmemory_mib = +257\n")),
            Err(ManifestError::Limit {
                key: "resources.memory_mib",
                max: 256,
                got: 257,
            })
        );
        assert_eq!(
            parse(&format!("{PLUGIN}[resources]\nmemory_mib = 0x101\n")),
            Err(ManifestError::Limit {
                key: "resources.memory_mib",
                max: 256,
                got: 257,
            })
        );
    }

    /// Verifies: SEC-EXT-025
    #[test]
    fn dotted_keys_and_inline_tables_are_the_same_schema() {
        let dotted = "\
plugin.id = \"org.listenbrainz.scrobbler\"
plugin.name = \"ListenBrainz\"
plugin.version = \"1.2.0\"
plugin.world = \"scrobbler@1\"
plugin.mode = \"per-user\"
permissions.network = [{ host = \"api.listenbrainz.org\", why = \"because\" }]
";
        let mut expected = bare();
        expected.network = vec![grant("api.listenbrainz.org", "because")];
        assert_eq!(parse(dotted), Ok(expected));
    }

    proptest! {
        /// Verifies: SEC-EXT-025
        #[test]
        fn a_root_key_outside_the_schema_is_refused_by_name(key in "[a-z][a-z0-9_]{0,16}") {
            prop_assume!(key != "plugin" && key != "permissions" && key != "resources");
            prop_assert_eq!(
                parse(&format!("{key} = 1\n")),
                Err(ManifestError::UnknownKey { key, line: 1 })
            );
        }

        /// Verifies: SEC-EXT-025
        #[test]
        fn a_plugin_key_outside_the_schema_is_refused_by_name(key in "[a-z][a-z0-9_]{0,12}") {
            prop_assume!(!matches!(key.as_str(), "id" | "name" | "version" | "world" | "mode"));
            prop_assert_eq!(
                parse(&format!("[plugin]\n{key} = 1\n")),
                Err(ManifestError::UnknownKey { key: format!("plugin.{key}"), line: 2 })
            );
        }

        /// Verifies: SEC-EXT-025
        #[test]
        fn a_permission_key_outside_the_schema_is_refused_by_name(key in "[a-z][a-z0-9_]{0,12}") {
            prop_assume!(!matches!(key.as_str(), "network" | "gunmetal" | "secrets"));
            prop_assert_eq!(
                parse(&format!("[permissions]\n{key} = 1\n")),
                Err(ManifestError::UnknownKey { key: format!("permissions.{key}"), line: 2 })
            );
        }

        /// Verifies: SEC-EXT-025
        #[test]
        fn a_resource_key_outside_the_schema_is_refused_by_name(key in "[a-z][a-z0-9_]{0,12}") {
            prop_assume!(!matches!(
                key.as_str(),
                "memory_mib" | "deadline_secs" | "kv_mib" | "outbound_per_minute"
            ));
            prop_assert_eq!(
                parse(&format!("[resources]\n{key} = 1\n")),
                Err(ManifestError::UnknownKey { key: format!("resources.{key}"), line: 2 })
            );
        }
    }
}
