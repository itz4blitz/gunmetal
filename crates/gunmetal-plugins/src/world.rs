//! The worlds a plugin may implement.
//!
//! A world is a closed name. An unknown world rejects the package
//! (SEC-EXT-025). The imports a world may link are a reviewed literal: no
//! filesystem, no sockets, no environment, and no WASI preview (SEC-EXT-022).

/// A plugin world this host can link.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum World {
    /// Reports a person's own plays.
    Scrobbler,
    /// Looks up album metadata from evidence the host passes.
    MusicMetadataProvider,
}

/// Why a text is not a world.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorldError {
    /// It is not a world this host names.
    Unknown,
}

/// Why a requested import is not linked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkError {
    /// The world does not import this name.
    NotInWorld {
        /// The import.
        name: String,
    },
    /// The same import was named twice.
    Duplicate {
        /// The import.
        name: String,
    },
}

/// Imports that are never linked, whatever a world asks for.
pub const FORBIDDEN_IMPORTS: &[&str] = &[
    "wasi:cli/environment",
    "wasi:cli/exit",
    "wasi:cli/stderr",
    "wasi:cli/stdin",
    "wasi:cli/stdout",
    "wasi:filesystem/types",
    "wasi:http/outgoing-handler",
    "wasi:sockets/tcp",
    "wasi_snapshot_preview0",
    "wasi_snapshot_preview1",
];

impl World {
    /// The imports both current worlds may link, sorted.
    const HOST_IMPORTS: &'static [&'static str] = &["egress", "kv", "log", "secrets"];

    /// Reads a world.
    ///
    /// # Errors
    ///
    /// [`WorldError::Unknown`] for any other name, including a world with no
    /// `@1`.
    pub fn parse(text: &str) -> Result<Self, WorldError> {
        match text {
            "scrobbler@1" => Ok(Self::Scrobbler),
            "music-metadata-provider@1" => Ok(Self::MusicMetadataProvider),
            _ => Err(WorldError::Unknown),
        }
    }

    /// The manifest spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Scrobbler => "scrobbler@1",
            Self::MusicMetadataProvider => "music-metadata-provider@1",
        }
    }

    /// The imports this world may link. The list is the reviewed literal.
    #[must_use]
    pub const fn imports(self) -> &'static [&'static str] {
        match self {
            Self::Scrobbler | Self::MusicMetadataProvider => Self::HOST_IMPORTS,
        }
    }

    /// Whether `requested` is exactly a subset of this world's imports.
    ///
    /// # Errors
    ///
    /// [`LinkError`] when a name is not in the world, or is named twice.
    pub fn link(self, requested: &[&str]) -> Result<(), LinkError> {
        let mut seen: Vec<&str> = Vec::new();
        for name in requested {
            if seen.contains(name) {
                return Err(LinkError::Duplicate {
                    name: (*name).to_owned(),
                });
            }
            if !self.imports().contains(name) {
                return Err(LinkError::NotInWorld {
                    name: (*name).to_owned(),
                });
            }
            seen.push(name);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{FORBIDDEN_IMPORTS, LinkError, World, WorldError};

    /// Verifies: SEC-EXT-022, SEC-EXT-025
    #[test]
    fn the_worlds_and_their_imports_are_these_literals() {
        assert_eq!(World::parse("scrobbler@1"), Ok(World::Scrobbler));
        assert_eq!(
            World::parse("music-metadata-provider@1"),
            Ok(World::MusicMetadataProvider)
        );
        assert_eq!(World::Scrobbler.as_str(), "scrobbler@1");
        assert_eq!(
            World::MusicMetadataProvider.as_str(),
            "music-metadata-provider@1"
        );
        assert_eq!(
            World::Scrobbler.imports(),
            &["egress", "kv", "log", "secrets"]
        );
        assert_eq!(
            World::MusicMetadataProvider.imports(),
            World::Scrobbler.imports()
        );
        for name in FORBIDDEN_IMPORTS {
            assert!(
                !World::Scrobbler.imports().contains(name),
                "{name} must not be linked"
            );
        }
    }

    /// Verifies: SEC-EXT-025
    #[test]
    fn an_unknown_world_is_refused() {
        for text in ["", "scrobbler", "scrobbler@2", "lyrics@1", "search@1"] {
            assert_eq!(World::parse(text), Err(WorldError::Unknown), "{text}");
        }
    }

    /// Verifies: SEC-EXT-022
    #[test]
    fn linking_keeps_only_the_worlds_imports() {
        assert_eq!(World::Scrobbler.link(&[]), Ok(()));
        assert_eq!(World::Scrobbler.link(&["egress", "log"]), Ok(()));
        assert_eq!(
            World::Scrobbler.link(&["wasi:filesystem/types"]),
            Err(LinkError::NotInWorld {
                name: "wasi:filesystem/types".to_owned(),
            })
        );
        assert_eq!(
            World::Scrobbler.link(&["egress", "egress"]),
            Err(LinkError::Duplicate {
                name: "egress".to_owned(),
            })
        );
        for name in FORBIDDEN_IMPORTS {
            assert_eq!(
                World::MusicMetadataProvider.link(&[name]),
                Err(LinkError::NotInWorld {
                    name: (*name).to_owned(),
                }),
                "{name}"
            );
        }
    }
}
