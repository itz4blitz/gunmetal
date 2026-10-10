# 31. GitHub is the source of truth, and the Forgejo is retired

Date: 2026-10-09

Status: accepted, through the owner's request on 2026-10-09 that the
project move off the Premier Forgejo, reach full parity on GitHub, and
have that enforced everywhere
([D-96](../decisions.md#d-96-source-of-truth-github)).

## Context

[Record 16](0016-forgejo-source-of-truth.md) made the Premier Forgejo
the source of truth, and records 17, 23 and 30 shaped Actions around its
self-hosted runners. The public GitHub repository drifted behind the
Forgejo, and its sync had dropped records 16 to 20 and 23 from the public
tree while the Forgejo also grew two records whose numbers collided with
the store records (two 23s and two 24s). The owner asked for the move:
full parity on GitHub, off the Forgejo, enforced everywhere.

## Decisions

1. **GitHub is the source of truth:**
   `https://github.com/itz4blitz/gunmetal`. Everything clones, pushes and
   opens pull requests there, and nowhere else. This supersedes
   [record 16](0016-forgejo-source-of-truth.md) (D-89).
2. **Actions run on GitHub-hosted `ubuntu-latest` runners** under this
   repository's workflows. This supersedes the self-hosted runner shape
   of [record 17](0017-all-actions-on-own-runners.md) and
   [record 29](0029-forgejo-action-references.md) and the
   no-`permissions:` rule of [record 30](0030-no-actions-permissions-field.md)
   (D-90, D-94, D-95): on GitHub, `permissions:` works and stays, and
   third-party actions are referenced as `owner/repo@sha`.
3. **The Premier Forgejo is retired to a read-only archive.** Nothing is
   pushed to it again and no pull request is opened there. Its branches
   were carried to GitHub so nothing is lost.
4. **Parity is total.** This merge restores the records the public sync
   had dropped (16 to 20 and 23), renumbers the two records that had
   collided with the store records to 29 and 30, and keeps the decision
   register rows D-89 to D-95 as history.
5. **The extensions review home stays `itz4blitz/gunmetal-extensions`**
   ([record 27](0027-extensions-review-on-personal-github.md)).

## Consequences

- `AGENTS.md` names GitHub only, so every coding agent lands its work
  there; the Forgejo remote is removed from the working copies.
- The gate, qodana and the mutation shards run as GitHub Actions. A
  macOS machine still cannot build the workspace (the worker's seccomp
  crate is Linux-only), so CI stays the definition of done.
- The retired Forgejo is treated as an archive: if it ever answers, read
  it, never follow it.
