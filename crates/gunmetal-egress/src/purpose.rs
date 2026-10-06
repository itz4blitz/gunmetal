//! Why the server reaches out: a closed list of purposes.
//!
//! Every outbound request names a [`Purpose`]. The list is the egress
//! inventory of the threat model (docs/security/threat-model.md, "Egress
//! inventory"), one variant per row, and holds only the rows the release
//! being built uses: in R1, certificate issuance and the update feed
//! (SEC-TM-075, SEC-OPS-060). A later purpose joins with the package that
//! first uses it. There is no purpose for scrobbling or for any other
//! external account, so no R1 code can reach such a service (SEC-PRV-034 to
//! SEC-PRV-036).

use crate::limits::Limits;

/// Why the server is making an outbound request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Purpose {
    /// Obtaining and renewing the server's own certificate: the configured
    /// certificate authority, and the owner's DNS provider for the DNS-01
    /// record.
    Acme,
    /// Reading the project's update and advisory feed.
    UpdateFeed,
}

/// What happens when the other side answers with a redirect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Redirects {
    /// The redirect is not followed, and the request ends there.
    Refused,
    /// A redirect to the same host is followed, at most
    /// [`MAX_REDIRECTS`](crate::redirect::MAX_REDIRECTS) times, and each
    /// hop passes the gate like a first request.
    SameHost,
}

/// The rules that come with a purpose, whatever the owner granted it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rules {
    /// The purpose's row in the egress inventory.
    pub inventory: &'static str,
    /// Whether the purpose may be used before the server is claimed.
    pub before_claim: bool,
    /// What happens to a redirect.
    pub redirects: Redirects,
    /// The time and size limits of each request.
    pub limits: Limits,
}

impl Purpose {
    /// Every purpose, in the order of the inventory.
    pub const ALL: [Self; 2] = [Self::Acme, Self::UpdateFeed];

    /// The purpose's rules.
    #[must_use]
    pub const fn rules(self) -> Rules {
        match self {
            Self::Acme => Rules {
                inventory: "ACME certificate issuance",
                before_claim: true,
                redirects: Redirects::Refused,
                limits: Limits::DEFAULT,
            },
            Self::UpdateFeed => Rules {
                inventory: "Update and advisory feed",
                before_claim: false,
                redirects: Redirects::Refused,
                limits: Limits::DEFAULT,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Purpose, Redirects, Rules};
    use crate::limits::Limits;
    use core::time::Duration;

    /// The threat model, which holds the egress inventory.
    const THREAT_MODEL: &str = include_str!("../../../docs/security/threat-model.md");

    /// The rows of the egress inventory as the threat model writes them:
    /// each row's purpose, default and release.
    fn inventory() -> Vec<(String, String, String)> {
        let (_, after) = THREAT_MODEL
            .split_once("### Egress inventory")
            .expect("the threat model has an egress inventory");
        after
            .lines()
            .skip_while(|line| !line.starts_with('|'))
            .take_while(|line| line.starts_with('|'))
            .skip(2)
            .map(|line| {
                let cells: Vec<&str> = line.split('|').map(str::trim).collect();
                (
                    cells[1].to_owned(),
                    cells[2].to_owned(),
                    cells[5].to_owned(),
                )
            })
            .collect()
    }

    /// The purposes are the inventory's R1 rows, in its order and under its
    /// names, and the ones that may be used before the claim are the ones
    /// whose default says so.
    ///
    /// Verifies: SEC-TM-075
    /// Supports: SEC-OPS-060
    #[test]
    fn the_purposes_are_the_inventory_rows_due_in_r1() {
        let due: Vec<(String, bool)> = inventory()
            .into_iter()
            .filter(|(_, _, release)| release == "R1")
            .map(|(purpose, default, _)| (purpose, default.contains("allowed before the claim")))
            .collect();
        assert_eq!(
            due,
            [
                ("ACME certificate issuance".to_owned(), true),
                ("Update and advisory feed".to_owned(), false),
            ]
        );
        let built: Vec<(String, bool)> = Purpose::ALL
            .iter()
            .map(|purpose| {
                let rules = purpose.rules();
                (rules.inventory.to_owned(), rules.before_claim)
            })
            .collect();
        assert_eq!(built, due);
    }

    /// The list has no purpose for scrobbling, an external account, naming,
    /// certificate transparency monitoring, single sign-on or a metadata
    /// provider.
    ///
    /// Verifies: SEC-PRV-034, SEC-PRV-035, SEC-PRV-036
    #[test]
    fn the_only_purposes_are_certificate_issuance_and_the_update_feed() {
        // This match stops compiling when a purpose is added, so the list
        // cannot grow without this test changing.
        let named = Purpose::ALL.map(|purpose| match purpose {
            Purpose::Acme => "certificate issuance",
            Purpose::UpdateFeed => "update feed",
        });
        assert_eq!(named, ["certificate issuance", "update feed"]);
    }

    /// Supports: SEC-EXT-003, SEC-API-078
    #[test]
    fn no_purpose_follows_redirects_and_each_has_the_baseline_limits() {
        let limits = Limits {
            connect: Duration::from_secs(5),
            total: Duration::from_secs(30),
            body: 8 * 1024 * 1024,
        };
        assert_eq!(
            Purpose::Acme.rules(),
            Rules {
                inventory: "ACME certificate issuance",
                before_claim: true,
                redirects: Redirects::Refused,
                limits,
            }
        );
        assert_eq!(
            Purpose::UpdateFeed.rules(),
            Rules {
                inventory: "Update and advisory feed",
                before_claim: false,
                redirects: Redirects::Refused,
                limits,
            }
        );
    }
}
