# 29. The public repository is itz4blitz/gunmetal

Date: 2026-10-09
Status: accepted, through the owner's direction on 2026-10-09 that the
project has to be open source, and that Forgejo on the Premier tailnet
cannot be that public home. It has no entry in the
[decision register](../decisions.md) yet; the owner adds one if he wants
it tracked there.

## Context

[Record 16](0016-forgejo-source-of-truth.md) named the tailnet Forgejo as
the canonical repository. That host is private. People who are not on the
tailnet cannot clone it or open a pull request. An open-source project
needs a public git URL.

`itz4blitz/gunmetal-extensions` was already public, because the
PremierStudio GitHub organisation has Actions disabled. The player
repository itself was still only on the tailnet and on a private GitHub
copy.

## Decisions

1. **The public repository is** `https://github.com/itz4blitz/gunmetal`.
   Outsiders clone it and open pull requests there.
2. **The tailnet Forgejo is not a public clone URL.** Record 16 stays as
   the record of that private host. It is not rewritten.
3. **Public GitHub Actions stay off** until a runner setup exists that
   does not let a fork pull request reach the private runners. A green
   check on a GitHub-hosted machine is still not the project's gate
   ([record 17](0017-all-actions-on-own-runners.md)).
4. **Vulnerability reports go to this public repository's security
   advisories**, not to the private PremierStudio GitHub repository.

## Consequences

The README, the contributing notes, and `security.txt` name
`itz4blitz/gunmetal`. The private runners are unchanged.
