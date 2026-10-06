# 16. Forgejo on the Premier tailnet is the source of truth

Date: 2026-10-06
Status: accepted, through the owner's direction on 2026-10-06
([D-89](../decisions.md#d-89-source-of-truth-forge))

## Context

Until this record, every clone URL, pull request and Cursor cloud-agent
remote named GitHub (`https://github.com/PremierStudio/gunmetal`). The
owner directed on 2026-10-06 that Gunmetal cut over to the Premier Studio
Forgejo that already runs on the Unraid host and is reached over
Tailscale. That forge has hosted the `PremierStudio` organisation since
2026-08-08, at `https://git.taild1bbf.ts.net`.

Cursor's cloud VMs are not members of the Premier tailnet. They cannot
resolve `*.taild1bbf.ts.net`, and they have no Forgejo credentials. GitHub
therefore remains the only forge those VMs can push to until a machine on
this work joins the tailnet.

Vulnerability reporting (D-63) and the signed `security.txt` the site
emits must stay on a public URL. A tailnet Forgejo is not reachable to
reporters who are not on the tailnet.

[Record 15](0015-own-runners-for-the-full-mutation-gate.md) put only the
full mutation shards on the Unraid runners.
[Record 17](0017-all-actions-on-own-runners.md) puts every Actions job
there.

## Decisions

1. **The canonical repository is the Premier Studio Forgejo copy:**
   `https://git.taild1bbf.ts.net/PremierStudio/gunmetal`. Agents and
   people on the tailnet clone, push and open pull requests there.
2. **GitHub is not the working forge.** Pull requests, reviews and merges
   belong on Forgejo. A GitHub copy, if one still exists, is leftover
   hosting, not CI and not the branch people merge.
3. **Disclosure stays on a public URL** (D-63). `SECURITY.md` and the
   site's `security.txt` keep the GitHub advisory URL until a public
   reporting path exists. They do not name the tailnet forge as a
   reporting channel.
4. **CI is the project's own runners**, under
   [record 17](0017-all-actions-on-own-runners.md). Workflow files stay
   in `.github/workflows/` because Forgejo Actions reads that path.
   Nothing is scheduled on GitHub-hosted machines.
5. **Published crate `repository` URLs stay on a publicly-resolvable
   host** until a public git hostname (for example a Cloudflare tunnel in
   front of this Forgejo) exists. A MagicDNS name that returns NXDOMAIN
   off the tailnet is not crate metadata.

## Consequences

Clone instructions name the Forgejo URL. Agent notes say pull requests
belong on Forgejo. This record does not rewrite earlier ADRs that cite
GitHub pull-request numbers; those citations stay as history.
