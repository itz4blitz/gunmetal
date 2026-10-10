# 28. A merged extension pull request is listed in the store

Date: 2026-10-09
Status: accepted, through the owner's request on 2026-10-09 that a
pull request into the extensions repository's `main` is the only way a
record becomes available in the store. It has no entry in the
[decision register](../decisions.md) yet; the owner adds one if he wants
it tracked there.

## Context

[Record 27](0027-extensions-review-on-personal-github.md) named
`itz4blitz/gunmetal-extensions` as the review home and said this build
does not fetch that repository. The owner then asked for the pipeline to
be the door: a pull request merged into `main` lists the record in the
store.

SEC-EXT-018 is unchanged. Listing a JSON record is not loading or running
an extension.

## Decisions

1. **The only way into the store index is a pull request merged to `main`**
   of `itz4blitz/gunmetal-extensions`. The `package` check has to pass.
   A direct push does not land. The merge publishes `catalog.json`.
2. **The listen host fetches that one pinned release URL** and serves it
   at `/extensions/catalog.json`. The browser does not contact GitHub.
   The response is JSON. It is not executed.
3. **The store lists those records.** `on` means this server already runs
   the job. Anything else is in the store and not installed. A listing
   does not install the record.

## Consequences

A new record appears in the store after the release is published and the
host's one-minute cache expires. The player still does not download or
run an extension package.
