//! `xtask lockfile-age override`: the dependency-age rule's override for an
//! urgent security fix. A pull request that carries the override label
//! passes only when a code owner has approved it (SEC-SUP-027).
//!
//! Anyone with triage access can apply a label, so the label alone proves
//! nothing. The approval is a review: a code owner of every lock file, as
//! the base commit's `.github/CODEOWNERS` names them, whose latest deciding
//! review (an approval, a request for changes, or a dismissed review)
//! approves the pull request's head commit. A push after the approval moves
//! the head, so the approval no longer counts and the check fails until a
//! code owner approves again. A team named as an owner cannot be checked
//! without reading its members, so only owners named as `@login` count.
//!
//! The reviews come from the GitHub API, saved by the workflow with
//! `gh api --paginate --slurp`, which writes one array per page inside an
//! outer array. The API lists reviews oldest first.

use std::collections::BTreeSet;

use crate::codeowners;
use crate::json::{self, Value};

/// The lock files whose code owners may approve an override.
pub const LOCKFILES: [&str; 2] = ["Cargo.lock", "fuzz/Cargo.lock"];

/// Something `lockfile-age override` found wrong.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Finding {
    /// A `CODEOWNERS` line whose pattern the check cannot read.
    UnreadableOwners {
        /// The line, counted from 1.
        line: usize,
    },
    /// The reviews are not pages of reviews that each have a state.
    UnreadableReviews,
    /// No code owner of every lock file approves the head commit.
    Unapproved {
        /// The head commit.
        head: String,
    },
}

/// Checks that `reviews` hold a code owner's approval of commit `head`, by
/// the owners `codeowners` names for every lock file.
pub fn check(codeowners: &str, reviews: &str, head: &str) -> Vec<Finding> {
    let owners: Result<Vec<Vec<&str>>, usize> = LOCKFILES
        .iter()
        .map(|path| codeowners::owners(codeowners, path))
        .collect();
    let owners = match owners {
        Ok(owners) => owners,
        Err(line) => return vec![Finding::UnreadableOwners { line }],
    };
    let Some(approvers) = approvers(reviews, head) else {
        return vec![Finding::UnreadableReviews];
    };
    let owns_every_lock_file = |login: &String| {
        owners.iter().all(|named| {
            named.iter().any(|owner| {
                owner
                    .strip_prefix('@')
                    .is_some_and(|owner| owner.eq_ignore_ascii_case(login))
            })
        })
    };
    if approvers.iter().any(owns_every_lock_file) {
        Vec::new()
    } else {
        vec![Finding::Unapproved {
            head: head.to_owned(),
        }]
    }
}

/// The logins, in lowercase, whose latest deciding review in `reviews`
/// approves commit `head`, or `None` when `reviews` is not pages of
/// reviews that each have a state. A review by a deleted account has no
/// login and is passed over.
fn approvers(reviews: &str, head: &str) -> Option<BTreeSet<String>> {
    let reviews = json::parse(reviews)?;
    let mut approvers = BTreeSet::new();
    for page in reviews.as_array()? {
        for review in page.as_array()? {
            let state = review.get("state")?.as_str()?;
            let Some(login) = review
                .get("user")
                .and_then(|user| user.get("login"))
                .and_then(Value::as_str)
            else {
                continue;
            };
            let login = login.to_ascii_lowercase();
            let on_head = review.get("commit_id").and_then(Value::as_str) == Some(head);
            match state {
                "APPROVED" if on_head => {
                    approvers.insert(login);
                }
                "APPROVED" | "CHANGES_REQUESTED" | "DISMISSED" => {
                    approvers.remove(&login);
                }
                _ => {}
            }
        }
    }
    Some(approvers)
}

#[cfg(test)]
mod tests {
    use super::{Finding, LOCKFILES, check};

    /// The head commit of the pull request in these tests.
    const HEAD: &str = "ecdd80bb57125d7ba9641ffaa4d7d2c19d3f3091";

    /// An earlier commit of the same pull request.
    const EARLIER: &str = "6dcb09b5b57875f334f61aebed695e2e4193db5e";

    /// The real file's shape: one owner for every lock file.
    const CODEOWNERS: &str = "/.github/ @ci\nCargo.lock @octocat @PremierStudio/maintainers\n";

    /// One review as the API lists it, by `login` (or a deleted account),
    /// in `state`, on `commit`.
    fn review(login: Option<&str>, state: &str, commit: &str) -> String {
        let user = login.map_or_else(
            || "null".to_owned(),
            |login| {
                format!("{{\"login\":\"{login}\",\"id\":1,\"type\":\"User\",\"site_admin\":false}}")
            },
        );
        format!(
            "{{\"id\":80,\"user\":{user},\"body\":\"Looks \\\"good\\\".\",\"state\":\"{state}\",\"author_association\":\"OWNER\",\"submitted_at\":\"2026-10-03T12:00:00Z\",\"commit_id\":\"{commit}\"}}"
        )
    }

    /// `pages` of reviews, as `gh api --paginate --slurp` saves them.
    fn pages(pages: &[&[String]]) -> String {
        let pages: Vec<String> = pages
            .iter()
            .map(|reviews| format!("[{}]", reviews.join(",")))
            .collect();
        format!("[{}]", pages.join(","))
    }

    /// The finding when nobody approved the head commit.
    fn unapproved() -> Vec<Finding> {
        vec![Finding::Unapproved {
            head: HEAD.to_owned(),
        }]
    }

    /// Verifies: SEC-SUP-027
    #[test]
    fn a_code_owner_approval_of_the_head_commit_passes() {
        let reviews = pages(&[&[
            review(Some("hubot"), "COMMENTED", EARLIER),
            review(Some("octocat"), "APPROVED", HEAD),
        ]]);
        assert_eq!(check(CODEOWNERS, &reviews, HEAD), []);
    }

    /// Verifies: SEC-SUP-027
    #[test]
    fn no_review_or_an_approval_of_an_earlier_commit_fails() {
        assert_eq!(check(CODEOWNERS, "[[]]", HEAD), unapproved());
        let reviews = pages(&[&[review(Some("octocat"), "APPROVED", EARLIER)]]);
        assert_eq!(check(CODEOWNERS, &reviews, HEAD), unapproved());
    }

    /// Verifies: SEC-SUP-027
    #[test]
    fn an_approval_by_anyone_but_a_named_code_owner_fails() {
        // `hubot` owns `.github/` only, and the team cannot be checked.
        let reviews = pages(&[&[
            review(Some("hubot"), "APPROVED", HEAD),
            review(Some("ci"), "APPROVED", HEAD),
            review(None, "APPROVED", HEAD),
        ]]);
        let codeowners = format!("{CODEOWNERS}/.github/ @hubot\n");
        assert_eq!(check(&codeowners, &reviews, HEAD), unapproved());
        assert_eq!(
            check("Cargo.lock @PremierStudio/maintainers\n", &reviews, HEAD),
            unapproved()
        );
    }

    /// Verifies: SEC-SUP-027
    #[test]
    fn a_later_deciding_review_replaces_an_approval() {
        let approved = review(Some("octocat"), "APPROVED", HEAD);
        for (later, expected) in [
            ("CHANGES_REQUESTED", unapproved()),
            ("DISMISSED", unapproved()),
            ("COMMENTED", vec![]),
            ("PENDING", vec![]),
        ] {
            let reviews = pages(&[
                std::slice::from_ref(&approved),
                &[review(Some("octocat"), later, HEAD)],
            ]);
            assert_eq!(check(CODEOWNERS, &reviews, HEAD), expected, "{later}");
        }
        let reviews = pages(&[&[
            review(Some("octocat"), "CHANGES_REQUESTED", EARLIER),
            approved.clone(),
        ]]);
        assert_eq!(check(CODEOWNERS, &reviews, HEAD), []);
        let reviews = pages(&[&[approved, review(Some("octocat"), "APPROVED", EARLIER)]]);
        assert_eq!(check(CODEOWNERS, &reviews, HEAD), unapproved());
    }

    /// Verifies: SEC-SUP-027
    #[test]
    fn logins_match_whatever_their_case() {
        let reviews = pages(&[&[review(Some("OctoCat"), "APPROVED", HEAD)]]);
        assert_eq!(check(CODEOWNERS, &reviews, HEAD), []);
        let reviews = pages(&[&[
            review(Some("OctoCat"), "APPROVED", HEAD),
            review(Some("octocat"), "CHANGES_REQUESTED", HEAD),
        ]]);
        assert_eq!(check(CODEOWNERS, &reviews, HEAD), unapproved());
    }

    /// Verifies: SEC-SUP-027
    #[test]
    fn the_approver_must_own_every_lock_file() {
        assert_eq!(LOCKFILES, ["Cargo.lock", "fuzz/Cargo.lock"]);
        let codeowners = "/Cargo.lock @octocat\n/fuzz/Cargo.lock @hubot\n";
        let one = pages(&[&[review(Some("octocat"), "APPROVED", HEAD)]]);
        assert_eq!(check(codeowners, &one, HEAD), unapproved());
        let both = pages(&[&[
            review(Some("octocat"), "APPROVED", HEAD),
            review(Some("hubot"), "APPROVED", HEAD),
        ]]);
        assert_eq!(check(codeowners, &both, HEAD), unapproved());
        let codeowners = "/Cargo.lock @octocat @hubot\n/fuzz/Cargo.lock @hubot\n";
        assert_eq!(check(codeowners, &both, HEAD), []);
    }

    /// Verifies: SEC-SUP-027
    #[test]
    fn unreadable_code_owners_or_reviews_fail() {
        let approved = pages(&[&[review(Some("octocat"), "APPROVED", HEAD)]]);
        assert_eq!(
            check("Cargo.lock @octocat\n*.md @docs\n", &approved, HEAD),
            [Finding::UnreadableOwners { line: 2 }]
        );
        let stateless =
            review(Some("octocat"), "APPROVED", HEAD).replace("\"state\"", "\"status\"");
        let numbered = review(Some("octocat"), "APPROVED", HEAD).replace("\"APPROVED\"", "1");
        for reviews in [
            String::new(),
            "{}".to_owned(),
            "[{}]".to_owned(),
            pages(&[&[stateless]]),
            pages(&[&[numbered]]),
            format!("{approved} trailing"),
        ] {
            assert_eq!(
                check(CODEOWNERS, &reviews, HEAD),
                [Finding::UnreadableReviews],
                "{reviews}"
            );
        }
    }
}
