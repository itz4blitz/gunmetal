//! Index freshness, refresh, revocation advice and audit events.
//!
//! A fetch is trusted only when its timestamp is still live, neither version
//! has rolled back, an unchanged snapshot still names the same snapshot and
//! targets hashes, and the root met its signature threshold (SEC-EXT-036).
//! A revocation can only advise disabling a version; the plugin keeps running
//! (SEC-EXT-041). A refresh names no server and no installed plugin
//! (SEC-EXT-042). Each host action is an audit entry (SEC-EXT-044).

/// Index metadata the client already trusts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Trusted {
    /// The trusted timestamp version.
    pub timestamp_version: u64,
    /// The trusted snapshot version.
    pub snapshot_version: u64,
    /// The trusted snapshot hash.
    pub snapshot_hash: [u8; 32],
    /// The trusted targets hash.
    pub targets_hash: [u8; 32],
    /// When the trusted timestamp expires, in milliseconds.
    pub expires_at_ms: i64,
}

/// Index metadata from one refresh.
///
/// Signature counts and hashes are inputs. This is the freshness decision,
/// not a client that downloads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fetched {
    /// The fetched timestamp version.
    pub timestamp_version: u64,
    /// When the fetched timestamp expires, in milliseconds.
    pub timestamp_expires_at_ms: i64,
    /// The fetched snapshot version.
    pub snapshot_version: u64,
    /// The snapshot hash named by the timestamp.
    pub snapshot_hash: [u8; 32],
    /// When the fetched snapshot expires, in milliseconds.
    pub snapshot_expires_at_ms: i64,
    /// The targets hash named by the snapshot.
    pub targets_hash: [u8; 32],
    /// The targets version named by the snapshot.
    pub targets_version: u64,
    /// How many root signatures the threshold requires.
    pub root_threshold: u8,
    /// How many valid root signatures the fetch carried.
    pub root_signatures: u8,
}

/// Why fetched index metadata is not trusted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexError {
    /// The timestamp has expired, so the client freezes.
    Expired,
    /// A snapshot or timestamp version is older than the trusted one.
    Rollback,
    /// The snapshot or targets hash changed without a new snapshot version.
    Mix,
    /// Fewer root signatures than the root threshold.
    Threshold,
}

/// Accepts fetched index metadata, or refuses it.
///
/// Expiry is checked first, then rollback, then mix-and-match, then the
/// root threshold, so two faults produce the earlier error. Freeze is the
/// timestamp expiry only: `snapshot_expires_at_ms` and `targets_version` do
/// not decide freshness. A lower snapshot or timestamp version is rollback.
/// An unchanged snapshot version with a different snapshot or targets hash
/// is mix-and-match.
///
/// A passing fetch replaces the trusted timestamp version, snapshot version,
/// snapshot hash, targets hash and timestamp expiry.
///
/// # Errors
///
/// [`IndexError::Expired`] when `now_ms` is at or after
/// `fetched.timestamp_expires_at_ms`.
/// [`IndexError::Rollback`] when the snapshot or timestamp version is lower
/// than the trusted version.
/// [`IndexError::Mix`] when the snapshot version is unchanged and the
/// snapshot hash or the targets hash differs.
/// [`IndexError::Threshold`] when `root_signatures` is below `root_threshold`.
#[must_use = "a refused fetch must not replace trusted metadata"]
pub fn accept(trusted: &Trusted, fetched: &Fetched, now_ms: i64) -> Result<Trusted, IndexError> {
    if now_ms >= fetched.timestamp_expires_at_ms {
        return Err(IndexError::Expired);
    }
    if fetched.snapshot_version < trusted.snapshot_version
        || fetched.timestamp_version < trusted.timestamp_version
    {
        return Err(IndexError::Rollback);
    }
    if fetched.snapshot_version == trusted.snapshot_version
        && fetched.snapshot_hash != trusted.snapshot_hash
    {
        return Err(IndexError::Mix);
    }
    if fetched.snapshot_version == trusted.snapshot_version
        && fetched.targets_hash != trusted.targets_hash
    {
        return Err(IndexError::Mix);
    }
    if fetched.root_signatures < fetched.root_threshold {
        return Err(IndexError::Threshold);
    }
    Ok(Trusted {
        timestamp_version: fetched.timestamp_version,
        snapshot_version: fetched.snapshot_version,
        snapshot_hash: fetched.snapshot_hash,
        targets_hash: fetched.targets_hash,
        expires_at_ms: fetched.timestamp_expires_at_ms,
    })
}

/// A version the signed index marks revoked, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Revocation {
    /// The plugin version the revocation names.
    pub version: String,
    /// The reason shown to the owner.
    pub reason: String,
}

/// The only effect an index revocation may have.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexEffect {
    /// Recommend that the owner disable the version.
    AdviseDisable,
}

/// Why a revocation effect is refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EffectError {
    /// The index named an effect other than advice.
    Other,
}

/// Maps a revocation effect word to the only effect the index may have.
///
/// `advise-disable` is advice. `disable`, `delete`, `kill` and every other
/// word are refused. The index has no other effect.
///
/// # Errors
///
/// [`EffectError::Other`] when `kind` is not `advise-disable`.
#[must_use = "an effect other than advice must not be applied"]
pub fn revocation_effect(kind: &str) -> Result<IndexEffect, EffectError> {
    match kind {
        "advise-disable" => Ok(IndexEffect::AdviseDisable),
        _ => Err(EffectError::Other),
    }
}

/// An owner notice for a revoked installed version.
///
/// The plugin is still running. The index cannot stop it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notice {
    /// The plugin the notice is about.
    pub plugin: String,
    /// The installed version that was revoked.
    pub version: String,
    /// The reason from the matching revocation.
    pub reason: String,
    /// Always true: the index cannot stop the plugin.
    pub still_running: bool,
}

/// A notice when `installed_version` is in `revocations`, otherwise nothing.
///
/// The first match supplies the reason. The plugin is not disabled:
/// `still_running` is always true.
#[must_use = "dropping a revocation notice hides it from the owner"]
pub fn notice(plugin: &str, installed_version: &str, revocations: &[Revocation]) -> Option<Notice> {
    let matched = revocations
        .iter()
        .find(|revocation| revocation.version == installed_version)?;
    Some(Notice {
        plugin: plugin.to_owned(),
        version: matched.version.clone(),
        reason: matched.reason.clone(),
        still_running: true,
    })
}

/// A refresh the client may send. It carries no identifier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefreshRequest {
    /// The method.
    pub method: &'static str,
    /// The index path.
    pub path: &'static str,
    /// The headers sent with the refresh.
    pub headers: Vec<(&'static str, &'static str)>,
}

/// The only refresh: `GET /index/timestamp`, with a fixed user agent.
///
/// No cookie, server id, installed list, or other identifier.
#[must_use]
pub fn refresh_request() -> RefreshRequest {
    RefreshRequest {
        method: "GET",
        path: "/index/timestamp",
        headers: vec![("user-agent", "Gunmetal"), ("accept", "application/json")],
    }
}

/// A host action that must be written to the security audit log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuditKind {
    /// A plugin was installed.
    Install,
    /// A plugin was updated.
    Update,
    /// A permission was granted.
    Grant,
    /// A plugin was suspended.
    Suspension,
    /// A revocation notice was shown.
    RevocationNotice,
    /// An adapter was enabled.
    AdapterEnable,
    /// An adapter was disabled.
    AdapterDisable,
    /// A webhook was created.
    WebhookCreate,
    /// A webhook was changed.
    WebhookChange,
    /// A webhook was disabled.
    WebhookDisable,
}

/// One security audit entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditEntry {
    /// What happened.
    pub kind: AuditKind,
    /// The plugin, adapter, or webhook the entry names.
    pub plugin: String,
    /// The outcome, such as `refused`.
    pub outcome: &'static str,
}

/// Records `kind` for `plugin` with `outcome`, unchanged.
#[must_use]
pub fn audit(kind: AuditKind, plugin: &str, outcome: &'static str) -> AuditEntry {
    AuditEntry {
        kind,
        plugin: plugin.to_owned(),
        outcome,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AuditEntry, AuditKind, EffectError, Fetched, IndexEffect, IndexError, Notice,
        RefreshRequest, Revocation, Trusted, accept, audit, notice, refresh_request,
        revocation_effect,
    };

    const SNAPSHOT: [u8; 32] = [0x11; 32];
    const TARGETS: [u8; 32] = [0x22; 32];
    const OTHER_SNAPSHOT: [u8; 32] = [0x33; 32];
    const OTHER_TARGETS: [u8; 32] = [0x44; 32];

    fn trusted_at(timestamp_version: u64, snapshot_version: u64) -> Trusted {
        Trusted {
            timestamp_version,
            snapshot_version,
            snapshot_hash: SNAPSHOT,
            targets_hash: TARGETS,
            expires_at_ms: 100,
        }
    }

    fn fetched_at(timestamp_version: u64, snapshot_version: u64) -> Fetched {
        Fetched {
            timestamp_version,
            timestamp_expires_at_ms: 1_000,
            snapshot_version,
            snapshot_hash: OTHER_SNAPSHOT,
            snapshot_expires_at_ms: 40,
            targets_hash: OTHER_TARGETS,
            targets_version: 1,
            root_threshold: 1,
            root_signatures: 3,
        }
    }

    /// Verifies: SEC-EXT-036
    #[test]
    fn a_fresh_consistent_fetch_replaces_trusted_metadata() {
        let trusted = trusted_at(4, 7);
        let fetched = Fetched {
            timestamp_version: 9,
            timestamp_expires_at_ms: 2_500,
            snapshot_version: 8,
            snapshot_hash: OTHER_SNAPSHOT,
            snapshot_expires_at_ms: 40,
            targets_hash: OTHER_TARGETS,
            targets_version: 1,
            root_threshold: 2,
            root_signatures: 5,
        };
        assert_eq!(
            accept(&trusted, &fetched, 2_499),
            Ok(Trusted {
                timestamp_version: 9,
                snapshot_version: 8,
                snapshot_hash: OTHER_SNAPSHOT,
                targets_hash: OTHER_TARGETS,
                expires_at_ms: 2_500,
            })
        );
        assert_eq!(
            accept(&trusted, &fetched, -1),
            Ok(Trusted {
                timestamp_version: 9,
                snapshot_version: 8,
                snapshot_hash: OTHER_SNAPSHOT,
                targets_hash: OTHER_TARGETS,
                expires_at_ms: 2_500,
            })
        );
    }

    /// Verifies: SEC-EXT-036
    #[test]
    fn an_equal_version_with_the_same_hashes_is_kept() {
        let trusted = Trusted {
            timestamp_version: 6,
            snapshot_version: 6,
            snapshot_hash: SNAPSHOT,
            targets_hash: TARGETS,
            expires_at_ms: 100,
        };
        let fetched = Fetched {
            timestamp_version: 6,
            timestamp_expires_at_ms: 1_000,
            snapshot_version: 6,
            snapshot_hash: SNAPSHOT,
            snapshot_expires_at_ms: 1,
            targets_hash: TARGETS,
            targets_version: 0,
            root_threshold: 2,
            root_signatures: 2,
        };
        assert_eq!(
            accept(&trusted, &fetched, 999),
            Ok(Trusted {
                timestamp_version: 6,
                snapshot_version: 6,
                snapshot_hash: SNAPSHOT,
                targets_hash: TARGETS,
                expires_at_ms: 1_000,
            })
        );
    }

    /// Verifies: SEC-EXT-036
    #[test]
    fn an_expired_timestamp_freezes_the_client() {
        let trusted = trusted_at(4, 7);
        let fetched = Fetched {
            timestamp_expires_at_ms: 2_500,
            snapshot_expires_at_ms: 9_000,
            ..fetched_at(9, 8)
        };
        assert_eq!(accept(&trusted, &fetched, 2_500), Err(IndexError::Expired));
        assert_eq!(accept(&trusted, &fetched, 2_501), Err(IndexError::Expired));
        let at_min = Fetched {
            timestamp_expires_at_ms: i64::MIN,
            snapshot_expires_at_ms: i64::MAX,
            ..fetched_at(9, 8)
        };
        assert_eq!(
            accept(&trusted, &at_min, i64::MIN),
            Err(IndexError::Expired)
        );
        let at_max = Fetched {
            timestamp_expires_at_ms: i64::MAX,
            snapshot_expires_at_ms: i64::MIN,
            ..fetched_at(9, 8)
        };
        assert_eq!(
            accept(&trusted, &at_max, i64::MAX),
            Err(IndexError::Expired)
        );
        assert_eq!(
            accept(
                &trusted,
                &Fetched {
                    timestamp_expires_at_ms: 0,
                    snapshot_expires_at_ms: 9_000,
                    ..fetched_at(9, 8)
                },
                0,
            ),
            Err(IndexError::Expired)
        );
    }

    /// Verifies: SEC-EXT-036
    #[test]
    fn a_lower_snapshot_version_is_rollback() {
        let trusted = Trusted {
            timestamp_version: 3,
            snapshot_version: 10,
            snapshot_hash: SNAPSHOT,
            targets_hash: TARGETS,
            expires_at_ms: 100,
        };
        let fetched = Fetched {
            timestamp_version: 10,
            timestamp_expires_at_ms: 500,
            snapshot_version: 9,
            snapshot_hash: SNAPSHOT,
            snapshot_expires_at_ms: 9_000,
            targets_hash: TARGETS,
            targets_version: 10,
            root_threshold: 1,
            root_signatures: 2,
        };
        assert_eq!(accept(&trusted, &fetched, 499), Err(IndexError::Rollback));
        assert_eq!(
            accept(
                &Trusted {
                    snapshot_version: u64::MAX,
                    ..trusted
                },
                &Fetched {
                    snapshot_version: u64::MAX - 1,
                    targets_version: u64::MAX,
                    ..fetched
                },
                499,
            ),
            Err(IndexError::Rollback)
        );
    }

    /// Verifies: SEC-EXT-036
    #[test]
    fn a_lower_timestamp_version_is_rollback() {
        let trusted = Trusted {
            timestamp_version: 10,
            snapshot_version: 3,
            snapshot_hash: SNAPSHOT,
            targets_hash: TARGETS,
            expires_at_ms: 100,
        };
        let fetched = Fetched {
            timestamp_version: 9,
            timestamp_expires_at_ms: 500,
            snapshot_version: 10,
            snapshot_hash: SNAPSHOT,
            snapshot_expires_at_ms: 9_000,
            targets_hash: TARGETS,
            targets_version: 10,
            root_threshold: 1,
            root_signatures: 2,
        };
        assert_eq!(accept(&trusted, &fetched, 499), Err(IndexError::Rollback));
        assert_eq!(
            accept(
                &Trusted {
                    timestamp_version: u64::MAX,
                    ..trusted
                },
                &Fetched {
                    timestamp_version: u64::MAX - 1,
                    snapshot_version: u64::MAX,
                    targets_version: u64::MAX,
                    ..fetched
                },
                499,
            ),
            Err(IndexError::Rollback)
        );
    }

    /// Verifies: SEC-EXT-036
    #[test]
    fn the_same_snapshot_version_with_a_different_snapshot_hash_is_mix() {
        let trusted = trusted_at(5, 4);
        let fetched = Fetched {
            timestamp_version: 6,
            timestamp_expires_at_ms: 500,
            snapshot_version: 4,
            snapshot_hash: OTHER_SNAPSHOT,
            snapshot_expires_at_ms: 9_000,
            targets_hash: TARGETS,
            targets_version: 6,
            root_threshold: 1,
            root_signatures: 1,
        };
        assert_eq!(accept(&trusted, &fetched, 499), Err(IndexError::Mix));
    }

    /// Verifies: SEC-EXT-036
    #[test]
    fn the_same_snapshot_version_with_a_different_targets_hash_is_mix() {
        let trusted = trusted_at(5, 4);
        let fetched = Fetched {
            timestamp_version: 6,
            timestamp_expires_at_ms: 500,
            snapshot_version: 4,
            snapshot_hash: SNAPSHOT,
            snapshot_expires_at_ms: 9_000,
            targets_hash: OTHER_TARGETS,
            targets_version: 6,
            root_threshold: 1,
            root_signatures: 1,
        };
        assert_eq!(accept(&trusted, &fetched, 499), Err(IndexError::Mix));
    }

    /// Verifies: SEC-EXT-036
    #[test]
    fn a_newer_snapshot_may_carry_new_hashes() {
        let trusted = Trusted {
            timestamp_version: 5,
            snapshot_version: 4,
            snapshot_hash: SNAPSHOT,
            targets_hash: TARGETS,
            expires_at_ms: 100,
        };
        let fetched = Fetched {
            timestamp_version: 5,
            timestamp_expires_at_ms: 800,
            snapshot_version: 5,
            snapshot_hash: OTHER_SNAPSHOT,
            snapshot_expires_at_ms: 1,
            targets_hash: OTHER_TARGETS,
            targets_version: 0,
            root_threshold: 1,
            root_signatures: 1,
        };
        assert_eq!(
            accept(&trusted, &fetched, 799),
            Ok(Trusted {
                timestamp_version: 5,
                snapshot_version: 5,
                snapshot_hash: OTHER_SNAPSHOT,
                targets_hash: OTHER_TARGETS,
                expires_at_ms: 800,
            })
        );
        assert_eq!(
            accept(
                &Trusted {
                    snapshot_version: 0,
                    timestamp_version: 0,
                    ..trusted
                },
                &Fetched {
                    snapshot_version: u64::MAX,
                    timestamp_version: u64::MAX,
                    targets_version: 0,
                    ..fetched
                },
                799,
            ),
            Ok(Trusted {
                timestamp_version: u64::MAX,
                snapshot_version: u64::MAX,
                snapshot_hash: OTHER_SNAPSHOT,
                targets_hash: OTHER_TARGETS,
                expires_at_ms: 800,
            })
        );
    }

    /// Verifies: SEC-EXT-036
    #[test]
    fn root_signatures_below_the_threshold_are_refused() {
        let trusted = trusted_at(4, 4);
        let below = [(1_u8, 2_u8), (0, 1), (3, 4), (254, 255)];
        for (root_signatures, root_threshold) in below {
            assert_eq!(
                accept(
                    &trusted,
                    &Fetched {
                        timestamp_version: 4,
                        snapshot_version: 4,
                        snapshot_hash: SNAPSHOT,
                        targets_hash: TARGETS,
                        root_threshold,
                        root_signatures,
                        timestamp_expires_at_ms: 1_000,
                        snapshot_expires_at_ms: 1,
                        targets_version: 4,
                    },
                    999,
                ),
                Err(IndexError::Threshold),
                "{root_signatures} of {root_threshold}"
            );
        }
    }

    /// Verifies: SEC-EXT-036
    #[test]
    fn root_signatures_at_or_above_the_threshold_are_accepted() {
        let trusted = trusted_at(4, 4);
        let enough = [(0_u8, 0_u8), (1, 1), (2, 2), (255, 255), (3, 1)];
        for (root_signatures, root_threshold) in enough {
            assert_eq!(
                accept(
                    &trusted,
                    &Fetched {
                        timestamp_version: 4,
                        snapshot_version: 4,
                        snapshot_hash: SNAPSHOT,
                        targets_hash: TARGETS,
                        root_threshold,
                        root_signatures,
                        timestamp_expires_at_ms: 1_000,
                        snapshot_expires_at_ms: 1,
                        targets_version: 0,
                    },
                    999,
                ),
                Ok(Trusted {
                    timestamp_version: 4,
                    snapshot_version: 4,
                    snapshot_hash: SNAPSHOT,
                    targets_hash: TARGETS,
                    expires_at_ms: 1_000,
                }),
                "{root_signatures} of {root_threshold}"
            );
        }
        assert_eq!(
            accept(
                &trusted,
                &Fetched {
                    timestamp_expires_at_ms: i64::MIN + 1,
                    snapshot_expires_at_ms: i64::MIN,
                    timestamp_version: 4,
                    snapshot_version: 4,
                    snapshot_hash: SNAPSHOT,
                    targets_hash: TARGETS,
                    targets_version: 0,
                    root_threshold: 0,
                    root_signatures: 0,
                },
                i64::MIN,
            ),
            Ok(Trusted {
                timestamp_version: 4,
                snapshot_version: 4,
                snapshot_hash: SNAPSHOT,
                targets_hash: TARGETS,
                expires_at_ms: i64::MIN + 1,
            })
        );
    }

    /// Verifies: SEC-EXT-036
    #[test]
    fn expiry_is_reported_before_rollback_mix_and_threshold() {
        let trusted = Trusted {
            timestamp_version: 10,
            snapshot_version: 10,
            snapshot_hash: SNAPSHOT,
            targets_hash: TARGETS,
            expires_at_ms: 1,
        };
        assert_eq!(
            accept(
                &trusted,
                &Fetched {
                    timestamp_version: 9,
                    timestamp_expires_at_ms: 100,
                    snapshot_version: 9,
                    snapshot_hash: OTHER_SNAPSHOT,
                    snapshot_expires_at_ms: 9_000,
                    targets_hash: OTHER_TARGETS,
                    targets_version: 1,
                    root_threshold: 3,
                    root_signatures: 1,
                },
                100,
            ),
            Err(IndexError::Expired)
        );
        assert_eq!(
            accept(
                &trusted,
                &Fetched {
                    timestamp_version: 11,
                    timestamp_expires_at_ms: 50,
                    snapshot_version: 9,
                    snapshot_hash: SNAPSHOT,
                    snapshot_expires_at_ms: 9_000,
                    targets_hash: TARGETS,
                    targets_version: 11,
                    root_threshold: 2,
                    root_signatures: 0,
                },
                50,
            ),
            Err(IndexError::Expired)
        );
    }

    /// Verifies: SEC-EXT-036
    #[test]
    fn rollback_is_reported_before_mix_and_threshold() {
        let trusted = Trusted {
            timestamp_version: 10,
            snapshot_version: 10,
            snapshot_hash: SNAPSHOT,
            targets_hash: TARGETS,
            expires_at_ms: 1,
        };
        assert_eq!(
            accept(
                &trusted,
                &Fetched {
                    timestamp_version: 9,
                    timestamp_expires_at_ms: 200,
                    snapshot_version: 10,
                    snapshot_hash: OTHER_SNAPSHOT,
                    snapshot_expires_at_ms: 1,
                    targets_hash: OTHER_TARGETS,
                    targets_version: 1,
                    root_threshold: 3,
                    root_signatures: 1,
                },
                199,
            ),
            Err(IndexError::Rollback)
        );
        assert_eq!(
            accept(
                &trusted,
                &Fetched {
                    timestamp_version: 11,
                    timestamp_expires_at_ms: 200,
                    snapshot_version: 9,
                    snapshot_hash: SNAPSHOT,
                    snapshot_expires_at_ms: 1,
                    targets_hash: TARGETS,
                    targets_version: 11,
                    root_threshold: 2,
                    root_signatures: 0,
                },
                199,
            ),
            Err(IndexError::Rollback)
        );
    }

    /// Verifies: SEC-EXT-036
    #[test]
    fn mix_is_reported_before_threshold() {
        let trusted = Trusted {
            timestamp_version: 10,
            snapshot_version: 10,
            snapshot_hash: SNAPSHOT,
            targets_hash: TARGETS,
            expires_at_ms: 1,
        };
        assert_eq!(
            accept(
                &trusted,
                &Fetched {
                    timestamp_version: 10,
                    timestamp_expires_at_ms: 200,
                    snapshot_version: 10,
                    snapshot_hash: OTHER_SNAPSHOT,
                    snapshot_expires_at_ms: 1,
                    targets_hash: TARGETS,
                    targets_version: 10,
                    root_threshold: 3,
                    root_signatures: 1,
                },
                199,
            ),
            Err(IndexError::Mix)
        );
        assert_eq!(
            accept(
                &trusted,
                &Fetched {
                    timestamp_version: 10,
                    timestamp_expires_at_ms: 200,
                    snapshot_version: 10,
                    snapshot_hash: SNAPSHOT,
                    snapshot_expires_at_ms: 1,
                    targets_hash: OTHER_TARGETS,
                    targets_version: 10,
                    root_threshold: 3,
                    root_signatures: 1,
                },
                199,
            ),
            Err(IndexError::Mix)
        );
    }

    /// Verifies: SEC-EXT-041
    #[test]
    fn only_advise_disable_is_an_index_effect() {
        assert_eq!(
            revocation_effect("advise-disable"),
            Ok(IndexEffect::AdviseDisable)
        );
        for kind in [
            "disable",
            "delete",
            "kill",
            "advise",
            "advise-disable-now",
            "xadvise-disable",
            "Advise-disable",
            "advise_disable",
            "advise-disable ",
            " advise-disable",
            "",
        ] {
            assert_eq!(revocation_effect(kind), Err(EffectError::Other), "{kind}");
        }
    }

    /// Verifies: SEC-EXT-041
    #[test]
    fn a_matching_revocation_advises_and_leaves_the_plugin_running() {
        let revocations = [
            Revocation {
                version: "0.1.0".to_owned(),
                reason: "not this version".to_owned(),
            },
            Revocation {
                version: "1.2.3".to_owned(),
                reason: "signed by a leaked key".to_owned(),
            },
            Revocation {
                version: "1.2.3".to_owned(),
                reason: "later duplicate".to_owned(),
            },
        ];
        assert_eq!(
            notice("org.example.plugin", "1.2.3", &revocations),
            Some(Notice {
                plugin: "org.example.plugin".to_owned(),
                version: "1.2.3".to_owned(),
                reason: "signed by a leaked key".to_owned(),
                still_running: true,
            })
        );
        assert_eq!(
            notice(
                "org.example.plugin",
                "",
                &[Revocation {
                    version: String::new(),
                    reason: String::new(),
                }],
            ),
            Some(Notice {
                plugin: "org.example.plugin".to_owned(),
                version: String::new(),
                reason: String::new(),
                still_running: true,
            })
        );
    }

    /// Verifies: SEC-EXT-041
    #[test]
    fn a_revocation_for_another_version_is_not_a_notice() {
        assert_eq!(notice("org.example.plugin", "1.2.3", &[]), None);
        assert_eq!(
            notice(
                "org.example.plugin",
                "1.2.3",
                &[Revocation {
                    version: "1.2.4".to_owned(),
                    reason: "signed by a leaked key".to_owned(),
                }],
            ),
            None
        );
        assert_eq!(
            notice(
                "org.example.plugin",
                "1.2",
                &[Revocation {
                    version: "1.2.3".to_owned(),
                    reason: "a prefix is not the version".to_owned(),
                }],
            ),
            None
        );
        assert_eq!(
            notice(
                "org.example.plugin",
                "1.2.3 ",
                &[Revocation {
                    version: "1.2.3".to_owned(),
                    reason: "trailing space is not the version".to_owned(),
                }],
            ),
            None
        );
    }

    /// Verifies: SEC-EXT-042
    #[test]
    fn a_refresh_names_no_server_and_no_installed_plugin() {
        assert_eq!(
            refresh_request(),
            RefreshRequest {
                method: "GET",
                path: "/index/timestamp",
                headers: vec![("user-agent", "Gunmetal"), ("accept", "application/json"),],
            }
        );
    }

    /// Verifies: SEC-EXT-044
    #[test]
    fn an_install_refusal_is_the_audit_entry() {
        assert_eq!(
            audit(AuditKind::Install, "org.example.plugin", "refused"),
            AuditEntry {
                kind: AuditKind::Install,
                plugin: "org.example.plugin".to_owned(),
                outcome: "refused",
            }
        );
    }

    /// Verifies: SEC-EXT-044
    #[test]
    fn every_audit_kind_is_recorded_as_given() {
        let cases = [
            (AuditKind::Install, "org.example.plugin", "refused"),
            (AuditKind::Update, "org.example.plugin", "held"),
            (AuditKind::Grant, "org.example.other", "allowed"),
            (AuditKind::Suspension, "org.example.plugin", "suspended"),
            (AuditKind::RevocationNotice, "org.example.plugin", "shown"),
            (AuditKind::AdapterEnable, "adapter.subsonic", "enabled"),
            (AuditKind::AdapterDisable, "adapter.jellyfin", "disabled"),
            (AuditKind::WebhookCreate, "webhook", "created"),
            (AuditKind::WebhookChange, "webhook", "changed"),
            (AuditKind::WebhookDisable, "webhook", "disabled"),
        ];
        for (kind, plugin, outcome) in cases {
            assert_eq!(
                audit(kind.clone(), plugin, outcome),
                AuditEntry {
                    kind,
                    plugin: plugin.to_owned(),
                    outcome,
                }
            );
        }
        assert_eq!(
            audit(AuditKind::Grant, "", ""),
            AuditEntry {
                kind: AuditKind::Grant,
                plugin: String::new(),
                outcome: "",
            }
        );
    }
}
