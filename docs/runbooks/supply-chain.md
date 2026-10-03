# Supply-chain incident runbook

Last reviewed: 2026-10-03. For maintainers, covering a malicious
dependency, a compromised maintainer account, a leaked or lost signing
key, and the reporting steps if the EU Cyber Resilience Act applies
(SEC-SUP-054). Rehearse before R1. The signing-key rotation itself is
rehearsed yearly (SEC-OPS-071, WP-136).

## Malicious dependency

1. Identify the crate or npm package and the first bad version from the
   lock file and the advisory.
2. Remove or pin around it in `Cargo.lock` (and the JavaScript lockfile
   once it exists). Do not `cargo update` the whole graph.
3. Rotate any credential that a build script or install script could have
   seen: CI secrets, developer tokens, the `SETTINGS_READER_TOKEN`.
4. Publish a GitHub security advisory with a CVE if shipped code was
   reachable, and add it to the signed feed within 24 hours of the fixed
   release (SEC-SUP-009).
5. Notify packagers of official and known community packages (distributions,
   NAS catalogues, container catalogues) under an embargo of no more than
   7 days (SEC-OPS-069).

## Compromised maintainer account

1. Take the account out of the organisation and revoke its sessions.
2. Review every commit, tag, release and workflow run since the last
   known-good point.
3. Rotate GitHub, crates.io, npm, GHCR, Cloudflare and registrar
   credentials. Coding agents never hold these (SEC-SUP-055).
4. If a release may have shipped, follow the leaked-key steps below and
   tell server owners to verify checksums and provenance.

## Leaked or lost signing key

Follow [signing-key.md](signing-key.md). Rotate TUF root on a test feed
that a released server build then accepts, revoke container images by
digest, and publish a new `SHA256SUMS`.

## EU Cyber Resilience Act

Whether Premier Studio is a manufacturer, a steward or neither is an
owner decision. If it applies, actively exploited vulnerabilities get an
early warning within 24 hours and a notification within 72 hours
(Regulation (EU) 2024/2847 Art. 14). Record the decision in this runbook
when it is made.

## Reserved names

Re-check before each release that the project's names stay reserved
(SEC-SUP-052):

- crates.io: `gunmetal`, `gunmetal-core`
- npm: `gunmetal`, `@gunmetal/web`
- GHCR: `ghcr.io/premierstudio/gunmetal`
- Docker Hub: `premierstudio/gunmetal`
