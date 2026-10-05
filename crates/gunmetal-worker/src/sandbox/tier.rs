//! The isolation tier table of SEC-MED-024, as a pure function of what
//! was enforced.
//!
//! Memory-safe parsing needs a floor: a separate process, resource limits
//! and `no_new_privs`. Above the floor, seccomp, Landlock and namespaces
//! are added where the system has them, and the report carries a "reduced
//! isolation" notice when any of the three is missing. Below the floor the
//! work is off. Native decoders need every control, so they are on only at
//! the full tier. Nothing here, and no argument anywhere in the sandbox,
//! lets work run with less (SEC-TM-045).

/// Which isolation controls hold for a worker.
///
/// Each field is an independent control from the SEC-MED-024 table, not a
/// mode flag, so a state machine would hide combinations the table names.
#[expect(
    clippy::struct_excessive_bools,
    reason = "each bool is one isolation control from the SEC-MED-024 table"
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Enforced {
    /// The work runs in a process of its own, started by the launcher.
    pub process: bool,
    /// The resource limits are set and core dumps are off.
    pub limits: bool,
    /// `no_new_privs` is set.
    pub no_new_privs: bool,
    /// The seccomp allowlist is installed.
    pub seccomp: bool,
    /// A Landlock ruleset that grants no filesystem access is enforced.
    pub landlock: bool,
    /// The worker runs in namespaces of its own.
    pub namespaces: bool,
}

/// The isolation tier a worker reached.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tier {
    /// Every control holds.
    Full,
    /// The floor holds, and at least one of seccomp, Landlock and
    /// namespaces is missing.
    Reduced,
    /// The floor does not hold, so no media is read.
    Off,
}

/// A tier and the words that explain it to the administrator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TierReport {
    /// The tier reached.
    pub tier: Tier,
    /// The notice for the health page and `doctor`, in plain language.
    /// `None` at the full tier.
    pub notice: Option<String>,
}

impl TierReport {
    /// Whether the memory-safe parsers may run: at the floor or above.
    #[must_use]
    pub fn memory_safe_parsing(&self) -> bool {
        self.tier != Tier::Off
    }

    /// Whether a native decoder may run: only at the full tier.
    #[must_use]
    pub fn native_decoders(&self) -> bool {
        self.tier == Tier::Full
    }
}

/// The names of the controls that do not hold, in table order.
fn missing(controls: [(bool, &'static str); 3]) -> Vec<&'static str> {
    controls
        .into_iter()
        .filter(|(holds, _)| !holds)
        .map(|(_, name)| name)
        .collect()
}

/// Joins names as prose: "a", "a and b", "a, b and c".
fn prose(names: &[&str]) -> String {
    let last = names.len().saturating_sub(1);
    [names[..last].join(", "), names[last..].concat()]
        .into_iter()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" and ")
}

impl Enforced {
    /// Nothing holds: what the server assumes of a worker that did not
    /// start or did not report.
    pub const NONE: Self = Self {
        process: false,
        limits: false,
        no_new_privs: false,
        seccomp: false,
        landlock: false,
        namespaces: false,
    };

    /// Applies the tier table.
    #[must_use]
    pub fn report(self) -> TierReport {
        let floor = missing([
            (self.process, "a separate process"),
            (self.limits, "resource limits"),
            (self.no_new_privs, "the no-new-privileges flag"),
        ]);
        let above = missing([
            (self.seccomp, "system call filtering (seccomp)"),
            (self.landlock, "Landlock"),
            (self.namespaces, "namespaces"),
        ]);
        if !floor.is_empty() {
            return TierReport {
                tier: Tier::Off,
                notice: Some(format!(
                    "Media scanning is off: the worker could not start with {}. \
                     Gunmetal does not read media files without that.",
                    prose(&floor)
                )),
            };
        }
        if !above.is_empty() {
            return TierReport {
                tier: Tier::Reduced,
                notice: Some(format!(
                    "Reduced isolation: media workers run without {}. \
                     They still run in a separate process with resource limits \
                     and no new privileges.",
                    prose(&above)
                )),
            };
        }
        TierReport {
            tier: Tier::Full,
            notice: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Enforced, Tier, TierReport, prose};

    /// Every control holds.
    const ALL: Enforced = Enforced {
        process: true,
        limits: true,
        no_new_privs: true,
        seccomp: true,
        landlock: true,
        namespaces: true,
    };

    /// A reduced report with this notice.
    fn reduced(notice: &str) -> TierReport {
        TierReport {
            tier: Tier::Reduced,
            notice: Some(notice.to_owned()),
        }
    }

    /// An off report with this notice.
    fn off(notice: &str) -> TierReport {
        TierReport {
            tier: Tier::Off,
            notice: Some(notice.to_owned()),
        }
    }

    #[test]
    fn names_join_as_prose() {
        assert_eq!(prose(&[]), "");
        assert_eq!(prose(&["a"]), "a");
        assert_eq!(prose(&["a", "b"]), "a and b");
        assert_eq!(prose(&["a", "b", "c"]), "a, b and c");
    }

    /// Verifies: SEC-MED-024
    #[test]
    fn every_control_holding_is_the_full_tier_with_no_notice() {
        let report = ALL.report();
        assert_eq!(
            report,
            TierReport {
                tier: Tier::Full,
                notice: None
            }
        );
        assert!(report.memory_safe_parsing());
        assert!(report.native_decoders());
    }

    /// Verifies: SEC-MED-024, SEC-TM-045
    #[test]
    fn a_missing_control_above_the_floor_is_the_reduced_tier_with_its_notice() {
        let cases = [
            (
                Enforced {
                    seccomp: false,
                    ..ALL
                },
                "Reduced isolation: media workers run without system call filtering (seccomp). \
                 They still run in a separate process with resource limits and no new privileges.",
            ),
            (
                Enforced {
                    landlock: false,
                    ..ALL
                },
                "Reduced isolation: media workers run without Landlock. \
                 They still run in a separate process with resource limits and no new privileges.",
            ),
            (
                Enforced {
                    namespaces: false,
                    ..ALL
                },
                "Reduced isolation: media workers run without namespaces. \
                 They still run in a separate process with resource limits and no new privileges.",
            ),
            (
                Enforced {
                    seccomp: false,
                    namespaces: false,
                    ..ALL
                },
                "Reduced isolation: media workers run without system call filtering (seccomp) \
                 and namespaces. \
                 They still run in a separate process with resource limits and no new privileges.",
            ),
            (
                Enforced {
                    seccomp: false,
                    landlock: false,
                    namespaces: false,
                    ..ALL
                },
                "Reduced isolation: media workers run without system call filtering (seccomp), \
                 Landlock and namespaces. \
                 They still run in a separate process with resource limits and no new privileges.",
            ),
        ];
        for (enforced, notice) in cases {
            let report = enforced.report();
            assert_eq!(report, reduced(notice));
            assert!(report.memory_safe_parsing());
            assert!(!report.native_decoders());
        }
    }

    /// Verifies: SEC-MED-024, SEC-TM-045
    #[test]
    fn a_missing_floor_control_turns_the_work_off_with_its_notice() {
        let cases = [
            (
                Enforced {
                    process: false,
                    ..ALL
                },
                "Media scanning is off: the worker could not start with a separate process. \
                 Gunmetal does not read media files without that.",
            ),
            (
                Enforced {
                    limits: false,
                    ..ALL
                },
                "Media scanning is off: the worker could not start with resource limits. \
                 Gunmetal does not read media files without that.",
            ),
            (
                Enforced {
                    no_new_privs: false,
                    ..ALL
                },
                "Media scanning is off: the worker could not start with \
                 the no-new-privileges flag. \
                 Gunmetal does not read media files without that.",
            ),
            (
                Enforced {
                    limits: false,
                    seccomp: false,
                    ..ALL
                },
                "Media scanning is off: the worker could not start with resource limits. \
                 Gunmetal does not read media files without that.",
            ),
            (
                Enforced::NONE,
                "Media scanning is off: the worker could not start with a separate process, \
                 resource limits and the no-new-privileges flag. \
                 Gunmetal does not read media files without that.",
            ),
        ];
        for (enforced, notice) in cases {
            let report = enforced.report();
            assert_eq!(report, off(notice));
            assert!(!report.memory_safe_parsing());
            assert!(!report.native_decoders());
        }
    }

    /// Verifies: SEC-MED-024, SEC-TM-045
    #[test]
    fn the_table_holds_for_every_combination_of_controls() {
        for bits in 0_u8..64 {
            let bit = |n: u8| bits & (1 << n) != 0;
            let enforced = Enforced {
                process: bit(0),
                limits: bit(1),
                no_new_privs: bit(2),
                seccomp: bit(3),
                landlock: bit(4),
                namespaces: bit(5),
            };
            // The table again, written as arithmetic on the bits: the
            // floor is the low three, the full tier is all six.
            let expected = [Tier::Off, Tier::Reduced, Tier::Full]
                [usize::from(bits & 0b111 == 0b111) + usize::from(bits == 0b11_1111)];
            let report = enforced.report();
            assert_eq!(report.tier, expected);
            assert_eq!(report.notice.is_none(), bits == 0b11_1111);
            assert_eq!(report.memory_safe_parsing(), bits & 0b111 == 0b111);
            assert_eq!(report.native_decoders(), bits == 0b11_1111);
        }
    }
}
