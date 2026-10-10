# 27. Extension records are reviewed in itz4blitz/gunmetal-extensions

Date: 2026-10-08
Status: accepted, through the owner's request on 2026-10-08 that the
extensions review home move to his personal GitHub account so Actions can
run. It has no entry in the [decision register](../decisions.md) yet; the
owner adds one if he wants it tracked there.

## Context

[Record 26](0026-extensions-store-on-github.md) named
`PremierStudio/gunmetal-extensions`. GitHub Actions is disabled for that
organization, so the pack check could not run. The owner asked to move the
repository to `itz4blitz`.

Record 22 and SEC-EXT-018 are unchanged. This build still does not fetch or
run an extension. The plugin host remains WP-225. Linking a WebAssembly
runtime before SEC-EXT-019 to SEC-EXT-034 pass is still refused.

## Decisions

1. **The review home is** `https://github.com/itz4blitz/gunmetal-extensions`.
   `officialExtensionsRepository()` names that owner. This build does not
   fetch it.
2. **A pull request is how a record is approved.** `main` requires a pull
   request. A merge does not install the record on a server.
3. **The pack check writes one JSON file per extension.** Those files are
   the reviewed packages. They are not code, and the player does not
   download them.

## Consequences

The store shows `itz4blitz/gunmetal-extensions`. The old organization URL
redirects there. The sandbox host is not started by this move.
