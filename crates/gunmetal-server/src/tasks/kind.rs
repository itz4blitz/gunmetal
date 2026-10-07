//! The closed set of R1 task kinds (WP-070). Point-release kinds are added
//! by the packages that handle them, one variant each.

/// One kind of heavy job the runner knows. The set is closed so a package
/// can request a kind before the package that handles it has merged: a
/// request for a kind with no handler stays queued and is reported as
/// waiting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TaskKind {
    /// A full library scan.
    LibraryScan,
    /// A rescan of one path inside a configured root.
    PathRefresh,
    /// A backup of the durable state.
    Backup,
    /// A trash purge.
    Purge,
    /// A rebuild of the cache from the library and the user log.
    Rebuild,
}

impl TaskKind {
    /// Every R1 kind, in declaration order.
    pub const ALL: [Self; 5] = [
        Self::LibraryScan,
        Self::PathRefresh,
        Self::Backup,
        Self::Purge,
        Self::Rebuild,
    ];

    /// The stable inventory name stored in the cache.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::LibraryScan => "library_scan",
            Self::PathRefresh => "path_refresh",
            Self::Backup => "backup",
            Self::Purge => "purge",
            Self::Rebuild => "rebuild",
        }
    }

    /// The kind whose inventory name is `text`.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "library_scan" => Some(Self::LibraryScan),
            "path_refresh" => Some(Self::PathRefresh),
            "backup" => Some(Self::Backup),
            "purge" => Some(Self::Purge),
            "rebuild" => Some(Self::Rebuild),
            _ => None,
        }
    }
}
