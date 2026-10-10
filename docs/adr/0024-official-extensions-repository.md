# 24. Extension records are reviewed in itz4blitz/gunmetal-extensions

Date: 2026-10-08
Status: accepted, through the owner's request on 2026-10-08 that Gunmetal
point at an official extensions repository, with review and membership
controlled there. It has no entry in the [decision register](../decisions.md)
yet; the owner adds one if he wants it tracked there.

## Context

[Record 22](0022-client-plugins-are-declarative-data.md) keeps client plugins
as data and forbids the client from fetching or running them. INT-066 put
first-party plugins in the main Gunmetal repository. The owner asked for a
separate official repository: version control, pull-request review, and the
ability to add or remove people, without those records living in the player
repository.

SEC-EXT-018 still forbids loading or executing plugin code until the sandbox
requirements pass. INT-054 keeps the plugin host in R2. A pointer is not a
downloader.

## Decisions

1. **The catalogue source is**
   `https://github.com/itz4blitz/gunmetal-extensions` on GitHub.
   `officialExtensionsRepository()` is that closed identity. This build does
   not clone it, fetch it, or run it.
2. **Write access is that repository's membership**, not a setting in the
   player. Adding or removing a reviewer is a GitHub access change.
3. **A merged change there is a reviewed record. It does not install itself
   on a server.** Gunmetal names the repository. It does not treat an open
   pull request as something to run.
4. **The interface id stays `gunmetal.extensions/1`.** A package targets an
   id in that interface. The GitHub repository is where the record is
   reviewed. They are not the same string.
5. **This does not add a third-party index.** INT-067 remains a later owner
   action: a signed index the owner adds after checking its key. This record
   only names the project's own extensions repository.

## Consequences

The Extensions page shows `itz4blitz/gunmetal-extensions` as where a
record is maintained, and says a pull request there does not install it.
INT-066's "main repository" is narrowed for extension records only; the
plugin host is still R2, and record 22 still forbids the client from
fetching a contribution. Creating the GitHub repository and its team is
repository administration, not a player feature.
