# Security policy

Last reviewed: 2026-10-03. This policy is reviewed at least once a year and
before each release.

## Reporting a vulnerability

Report vulnerabilities privately through GitHub's private vulnerability
reporting: on this repository's **Security** tab, choose **Report a
vulnerability**. Do not open a public issue, discussion or pull request for
a security problem.

GitHub offers private vulnerability reporting only on public repositories,
and this repository is still private. Until it is public, people who can
read it report to the maintainer, @itz4blitz, privately rather than in an
issue.

For now, reports go through GitHub only: on 2026-10-02 the owner decided
not to run a separate email alias or security@ mailbox yet
([decision register](docs/decisions.md#owner-answers-2026-10-02)).
SEC-OPS-064 still requires an alias that reaches two people before R1 is
tagged; this file will list it then.

A useful report says which component and commit or version is affected, how
to reproduce the problem (a minimal input file or request is ideal), and
what an attacker gains.

## Scope

In scope:

- Code in this repository: the shared core (`gunmetal-core`) and, as they
  arrive, the server, the web client, the native clients and the
  compatibility adapters.
- How Gunmetal is built and released: the workflows in `.github/`, the
  scripts in `scripts/`, the dependency policy (`deny.toml`,
  `supply-chain/`) and, once they exist, release artifacts, official
  container images, published compose and NAS templates, and the project
  site.
- Vulnerabilities in a dependency that Gunmetal's use makes exploitable.
  Report the dependency itself to its maintainers as well.

Out of scope:

- Problems that need an attacker who already controls the host, the
  server's account or its configuration files.
- Denial of service by sheer traffic volume against project
  infrastructure.
- Social engineering of maintainers or users.

The security design Gunmetal is held to is in
[docs/security](docs/security/README.md). Behaviour that breaks one of its
first principles is in scope even when the requirement behind it is not yet
implemented.

## Supported versions

Gunmetal is pre-alpha and has no releases, so no version receives security
fixes yet. Fixes land on `main`.

When releases begin, the planned policy is that before 1.0 only the latest
release receives security fixes. From 1.0, the latest minor plus the
previous minor for 90 days after its successor ships. This section will
list each supported version and the date its fixes end. End-of-support
dates also go in the signed advisory feed so unsupported servers can show
a banner (SEC-OPS-070, SEC-SUP-056).

## What happens after you report

| Step | Target |
|---|---|
| Acknowledge the report | Within 7 days |
| Triage: confirm or reject it, and agree a severity | Within 14 days |
| Fix or mitigation released | Within 90 days |

Shorter fix targets by severity: 14 days for Critical, 30 days for High.
Every report is tracked against these times (SEC-OPS-065). Coordinated
disclosure is no later than 90 days after the report.

Every fix starts with a regression test that reproduces the problem, is
named after its advisory ID, and fails before the fix (SEC-TM-003,
SEC-OPS-066, SEC-HIS-064). The fix includes a class review: a search of
the server, core, clients and adapters for the same root cause. We keep
you informed at each step, and tell you if a target will be missed and
why.

## Coordinated disclosure

- The embargo lasts until a fix is released, and at most 90 days from the
  report by default. It is shorter if the problem is being exploited, and
  longer only by agreement with the reporter.
- Please keep the details private until the advisory is published.
- Reporters are credited in the advisory by the name or handle they choose,
  or not at all if they prefer.
- We will not pursue or support legal action against anyone who researches
  in good faith: who reports promptly, avoids harming users and their data,
  and tests only against installations they own or have permission to test.

Before public disclosure, packagers of official and known community
packages are notified under an embargo of no more than 7 days, using the
contact list in [docs/runbooks/supply-chain.md](docs/runbooks/supply-chain.md)
(SEC-OPS-069).

## Advisories and CVEs

Each fixed vulnerability is published as a GitHub security advisory for
this repository, with the affected and fixed versions, a CWE and a CVSS v4.0
score. The CVE is requested from that advisory: GitHub is a CVE Numbering
Authority and assigns it. Published advisories flow into the GitHub Advisory
Database and OSV, with affected and fixed ranges in OSV form (SEC-OPS-067).
If a crate published to crates.io is affected, the advisory is also filed
with RustSec. Once the signed in-product advisory feed exists, each advisory
is added to it as well, so administrators learn about fixes even without a
GitHub account. The changelog's Security section links every advisory.
Each advisory also names a workaround that needs no update where one exists,
and the audit-log events owners can check for exploitation.

The fix is developed in the draft advisory's temporary private fork. GitHub
provides repository security advisories for public repositories only, so
this process starts once the repository is public.

Every release has a unique version (SEC-SUP-056).

## Security roles

Until a second trusted maintainer is named (required before R1 is tagged),
the maintainer holds every role. Deputies stay unfilled until that person
exists; the TUF threshold keys (SEC-SUP-049) must then be held by different
people.

| Role | Holder | Deputy |
|---|---|---|
| Security lead | @itz4blitz | pending a second maintainer |
| Release manager | @itz4blitz | pending a second maintainer |
| Incident lead | @itz4blitz | pending a second maintainer |

## Succession

If a maintainer cannot be reached for 30 days, the deputy for each role
takes over that role. Until a deputy is named, there is no automatic
successor: the project does not tag or release, and reports wait with the
maintainer. Naming a second maintainer before R1 fills the deputies and
this plan.

## Coding agents

Coding agents and other automation working on the repository run without
access to release credentials, signing keys or maintainers' authenticated
browser and CLI sessions (SEC-SUP-055). Their changes go through the same
reviews and checks as any contributor's. Agents never merge into `main`,
tag or release.
