//! What the server checks and changes about its own process before it
//! opens anything.
//!
//! - **No root, no capabilities, no override.** [`Privileges::probe`] reads
//!   the effective user and the effective and permitted capability sets,
//!   and [`Privileges::check`] is the pure rule over them: user 0 is
//!   refused, and so is any capability in either set (SEC-OPS-053, ADM-006).
//!   Nothing in the command line, the environment or the configuration file
//!   turns the check off.
//! - **No core dumps.** [`disable_core_dumps`] sets `RLIMIT_CORE` to 0 and
//!   clears the dumpable flag, so a crash writes no memory image holding
//!   keys, and no other process of the same user can attach to the server
//!   (SEC-STD-023).

use rustix::io::Errno;
use rustix::process::{DumpableBehavior, Resource, Rlimit};

/// Who the process runs as, as the kernel reports it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Privileges {
    /// The effective user ID.
    pub euid: u32,
    /// The effective capability set, one bit per capability.
    pub effective: u64,
    /// The permitted capability set, one bit per capability.
    pub permitted: u64,
}

/// Why the server refuses to run with the privileges it has.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrivilegeError {
    /// The effective user is root.
    Root,
    /// The process holds at least one capability.
    Capabilities {
        /// The effective set.
        effective: u64,
        /// The permitted set.
        permitted: u64,
    },
}

impl Privileges {
    /// Reads this process's effective user and capability sets.
    ///
    /// # Errors
    ///
    /// The kernel's error when the capability sets cannot be read.
    pub fn probe() -> Result<Self, Errno> {
        rustix::thread::capabilities(None).map(|sets| Self {
            euid: rustix::process::geteuid().as_raw(),
            effective: sets.effective.bits(),
            permitted: sets.permitted.bits(),
        })
    }

    /// Whether the server may run with these privileges.
    ///
    /// # Errors
    ///
    /// [`PrivilegeError::Root`] for user 0, whatever its capabilities, and
    /// [`PrivilegeError::Capabilities`] for any other user that holds an
    /// effective or permitted capability.
    pub const fn check(&self) -> Result<(), PrivilegeError> {
        if self.euid == 0 {
            return Err(PrivilegeError::Root);
        }
        if self.effective != 0 || self.permitted != 0 {
            return Err(PrivilegeError::Capabilities {
                effective: self.effective,
                permitted: self.permitted,
            });
        }
        Ok(())
    }
}

impl PrivilegeError {
    /// What to tell the admin, naming the fix.
    #[must_use]
    pub fn message(&self) -> String {
        match self {
            Self::Root => "Gunmetal does not run as root, and no setting changes that. Run it as a dedicated unprivileged user that owns the data directory: User=gunmetal in a systemd unit, or --user in a container.".to_owned(),
            Self::Capabilities {
                effective,
                permitted,
            } => format!(
                "Gunmetal does not run with Linux capabilities (effective {effective:#x}, permitted {permitted:#x}), and no setting changes that. Remove AmbientCapabilities= from its systemd unit, --cap-add from its container and any file capabilities from its binary."
            ),
        }
    }
}

/// Sets the core file size limit to 0 and clears the dumpable flag
/// (SEC-STD-023).
///
/// # Errors
///
/// The kernel's error when either cannot be set.
pub fn disable_core_dumps() -> Result<(), Errno> {
    let none = Rlimit {
        current: Some(0),
        maximum: Some(0),
    };
    rustix::process::setrlimit(Resource::Core, none)
        .and_then(|()| rustix::process::set_dumpable_behavior(DumpableBehavior::NotDumpable))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn privileges(euid: u32, effective: u64, permitted: u64) -> Privileges {
        Privileges {
            euid,
            effective,
            permitted,
        }
    }

    /// Verifies: SEC-OPS-053, SEC-TM-041
    #[test]
    fn refuses_root_whatever_its_capabilities() {
        assert_eq!(privileges(0, 0, 0).check(), Err(PrivilegeError::Root));
        assert_eq!(
            privileges(0, 0x1ff_ffff_ffff, 0x1ff_ffff_ffff).check(),
            Err(PrivilegeError::Root)
        );
    }

    /// Verifies: SEC-OPS-053
    #[test]
    fn refuses_any_effective_or_permitted_capability() {
        // CAP_NET_BIND_SERVICE is bit 10.
        let cases = [(0x400, 0), (0, 0x400), (0x400, 0x400), (1, 0), (0, 1 << 40)];
        for (effective, permitted) in cases {
            assert_eq!(
                privileges(1000, effective, permitted).check(),
                Err(PrivilegeError::Capabilities {
                    effective,
                    permitted
                })
            );
        }
    }

    #[test]
    fn accepts_an_unprivileged_user_without_capabilities() {
        assert_eq!(privileges(1, 0, 0).check(), Ok(()));
        assert_eq!(privileges(u32::MAX, 0, 0).check(), Ok(()));
    }

    #[test]
    fn names_the_fix_for_each_refusal() {
        assert_eq!(
            PrivilegeError::Root.message(),
            "Gunmetal does not run as root, and no setting changes that. Run it as a dedicated unprivileged user that owns the data directory: User=gunmetal in a systemd unit, or --user in a container."
        );
        assert_eq!(
            PrivilegeError::Capabilities {
                effective: 0x400,
                permitted: 0x1400
            }
            .message(),
            "Gunmetal does not run with Linux capabilities (effective 0x400, permitted 0x1400), and no setting changes that. Remove AmbientCapabilities= from its systemd unit, --cap-add from its container and any file capabilities from its binary."
        );
    }

    #[test]
    fn probes_the_test_process_as_unprivileged() {
        // The suite must not run as root: root ignores the permissions other
        // tests rely on, and the server would refuse to start.
        let euid = rustix::process::geteuid().as_raw();
        assert_ne!(euid, 0);
        assert_eq!(Privileges::probe(), Ok(privileges(euid, 0, 0)));
    }
}
