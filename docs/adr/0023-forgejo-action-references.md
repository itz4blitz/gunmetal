# 23. Third-party Actions are fetched by absolute SHA-pinned URL

Date: 2026-10-08
Status: accepted, through the owner's direction on 2026-10-08
([D-94](../decisions.md#d-94-forgejo-action-references))

## Context

[Record 17](0017-all-actions-on-own-runners.md) put every job on the
Forgejo runners labelled `gunmetal-mutants`. Those runners resolve a
short `uses: owner/repo@sha` against `https://data.forgejo.org`. That
mirror has `actions/checkout` and `actions/upload-artifact`. It does
not have the other actions these workflows pin (`dtolnay/rust-toolchain`,
`taiki-e/install-action`, `Swatinem/rust-cache`, `fsfe/reuse-action`,
`ossf/scorecard-action`, `github/codeql-action`, `JetBrains/qodana-action`).
Every such step failed before it ran, with `repository not found`.

Forgejo also warns that a `permissions:` field is unsupported and will
be ignored. The warning does not fail the job. Removing the field would
fail zizmor's `excessive-permissions` audit and the repository check for
SEC-SUP-012.

The runner image is `node:22-bookworm`. It has `node` and not `jq`.

## Decisions

1. **A third-party action is an absolute `https://github.com/...@<40-hex>`
   reference.** The SHA pin required by SEC-SUP-010 does not change.
   `actions/checkout` and `actions/upload-artifact` stay short names,
   because the mirror has them.
2. **`permissions: {}` stays on every workflow, and each job keeps the
   scopes it asks for.** Forgejo's warning is expected. Do not delete
   the field to silence it.
3. **The gate job reads `needs` with `node`, not `jq`.** The image
   already has `node`.
4. **`gunmetal-mutants` is declared in `.github/actionlint.yaml`.**
   actionlint otherwise treats that label as unknown and fails the
   workflow-lint job. The same file ignores actionlint's complaint that
   an absolute `https://github.com/...@sha` use is empty: its grammar
   only accepts `owner/repo@ref`, and that short form is what the runner
   cannot clone.

## Consequences

A new `uses:` that names `owner/repo@sha` without a host will not clone
on these runners. Point it at the commit on GitHub, still by full SHA.
Do not add a second workflow tree under `.forgejo/workflows/` (record 17).
