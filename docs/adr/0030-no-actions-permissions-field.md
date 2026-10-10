# 30. Workflows do not set a `permissions` field

Date: 2026-10-08
Status: accepted, through the owner's direction on 2026-10-08
([D-95](../decisions.md#d-95-no-actions-permissions-field))

## Context

[record 29](0029-forgejo-action-references.md) kept `permissions:` because
SEC-SUP-012 and zizmor ask for it, and said Forgejo's warning could stay.
The owner rejected that. Forgejo 16 prints, on every job:

> Job … or its workflow has a permissions field, which is not supported
> in Forgejo and will be ignored. Use Authorized Integrations to grant
> capabilities to this job instead.

The field does not change what the job token can do. Authorized
Integrations are a user setting for a JWT, not a workflow key that
replaces `permissions:`. There is no workflow syntax that keeps the
field and silences the warning.

## Decisions

1. **Workflow files do not contain a `permissions` key**, at the top or
   on a job. This supersedes decision 2 of record 23. It does not
   rewrite that record.
2. **zizmor still has to fail a workflow that uses default permissions.**
   That proof is the scratch canary in `workflow-lint.yml`. The real
   workflow files are listed under `excessive-permissions` in
   `.github/zizmor.yml`, so the lint job does not fail on the absence
   the forge requires.
3. **Do not add Authorized Integrations just to quiet this warning.**
   They grant extra API reach. These jobs do not need it.

## Consequences

A new workflow must not add `permissions:`. Adding one puts the warning
back on every job of that workflow. The scratch canary is the place
that still names the field.
