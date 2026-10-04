//! The resource limits a worker runs under (SEC-MED-021).

use rustix::process::Resource;

/// A sandbox profile: the kind of work a worker is confined for.
///
/// R1 has one profile. Every profile applies the whole confinement; a
/// profile only chooses the limits' values, and nothing in it can turn a
/// control off (SEC-TM-045).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Profile {
    /// Scanning: reading metadata and artwork out of media files with the
    /// memory-safe parsers.
    Scan,
}

/// One resource limit: the soft value the kernel acts on and the hard
/// value the worker can never raise it past.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Limit {
    /// The resource.
    pub(crate) resource: Resource,
    /// The soft limit.
    pub(crate) soft: u64,
    /// The hard limit.
    pub(crate) hard: u64,
}

impl Limit {
    /// A limit whose soft and hard values are the same.
    const fn fixed(resource: Resource, value: u64) -> Self {
        Self {
            resource,
            soft: value,
            hard: value,
        }
    }
}

impl Profile {
    /// The word that names this profile in the worker's arguments.
    #[must_use]
    pub(crate) const fn word(self) -> &'static str {
        match self {
            Self::Scan => "scan",
        }
    }

    /// Every profile, for reading the word back.
    pub(crate) const ALL: [Self; 1] = [Self::Scan];

    /// The limits the worker sets on itself, in the order it sets them.
    ///
    /// - Address space: 512 MiB, the default until packaging limits are
    ///   measured (decision D-03's technical answer).
    /// - Core files: none, so a crash leaves no dump of what was parsed.
    /// - Open files: 32.
    /// - CPU time: 60 seconds. The hard limit is one second later, so the
    ///   kernel first sends `SIGXCPU`, which names the cause, and only
    ///   then `SIGKILL`.
    /// - Processes: none, which also refuses a second thread. The kernel
    ///   does not apply this limit to a process with `CAP_SYS_RESOURCE` or
    ///   `CAP_SYS_ADMIN`, so it holds only while the server does not run
    ///   as root. A root server's worker that runs without seccomp has
    ///   nothing that stops it forking; WP-078 or the packaging package
    ///   must refuse to start workers as root or say so in the notice.
    /// - File size: none; a worker writes only to its socket.
    #[must_use]
    pub(crate) const fn limits(self) -> [Limit; 6] {
        match self {
            Self::Scan => [
                Limit::fixed(Resource::As, 536_870_912),
                Limit::fixed(Resource::Core, 0),
                Limit::fixed(Resource::Nofile, 32),
                Limit {
                    resource: Resource::Cpu,
                    soft: 60,
                    hard: 61,
                },
                Limit::fixed(Resource::Nproc, 0),
                Limit::fixed(Resource::Fsize, 0),
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Limit, Profile};
    use rustix::process::Resource;

    #[test]
    fn the_scan_profile_is_named_scan() {
        assert_eq!(Profile::Scan.word(), "scan");
        assert_eq!(Profile::ALL, [Profile::Scan]);
    }

    #[test]
    fn the_scan_profile_limits_memory_cores_files_cpu_processes_and_file_size() {
        assert_eq!(
            Profile::Scan.limits(),
            [
                Limit {
                    resource: Resource::As,
                    soft: 512 * 1024 * 1024,
                    hard: 512 * 1024 * 1024,
                },
                Limit {
                    resource: Resource::Core,
                    soft: 0,
                    hard: 0,
                },
                Limit {
                    resource: Resource::Nofile,
                    soft: 32,
                    hard: 32,
                },
                Limit {
                    resource: Resource::Cpu,
                    soft: 60,
                    hard: 61,
                },
                Limit {
                    resource: Resource::Nproc,
                    soft: 0,
                    hard: 0,
                },
                Limit {
                    resource: Resource::Fsize,
                    soft: 0,
                    hard: 0,
                },
            ]
        );
    }
}
