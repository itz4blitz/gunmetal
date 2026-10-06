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

[Record 15](0015-own-runners-for-the-full-mutation-gate.md) put the full
mutation gate on the project's own GitHub Actions runners. That CI path
is unchanged until a Forgejo Actions or TeamCity job is the required
check.

## Decisions

1. **The canonical repository is the Premier Studio Forgejo copy:**
   `https://git.taild1bbf.ts.net/PremierStudio/gunmetal`. Agents and
   people on the tailnet clone, push and open pull requests there.
2. **GitHub `https://github.com/PremierStudio/gunmetal` is a mirror**,
   not the source of truth. It stays so that Cursor cloud agents, public
   clone instructions that cannot see the tailnet, crate metadata and
   GitHub Actions can keep working. A later change can retire the mirror
   once those paths have a reachable Forgejo replacement.
3. **Disclosure stays on GitHub** (D-63). `SECURITY.md` and the site's
   `security.txt` keep the GitHub advisory URL. They do not name the
   tailnet forge as a reporting channel.
4. **CI stays GitHub Actions** under record 15 until a named Forgejo or
   TeamCity workflow is the required `gate` check. `.github/` is not
   moved by this record.
5. **Published crate `repository` URLs stay on GitHub** until a
   publicly-resolvable git hostname (for example a Cloudflare tunnel in
   front of the same Forgejo) exists. A MagicDNS name that returns
   NXDOMAIN off the tailnet is not crate metadata.

## Consequences

Clone instructions in the README show the Forgejo URL first and the
GitHub mirror second. Agent notes say pull requests belong on Forgejo
when the environment can reach it, and on the GitHub mirror otherwise.
This record does not rewrite earlier ADRs that cite GitHub pull-request
numbers; those citations stay as history.
