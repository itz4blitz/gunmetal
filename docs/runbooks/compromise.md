# Compromise runbook for server owners

Last reviewed: 2026-10-03. For an owner who thinks a Gunmetal server was
broken into (ADM-148, SEC-OPS-072). The project never acts on anyone's
server: there is no kill switch (ACC-128). These steps run on the host.

CI extracts the `runbook` block below and checks the presence and order of
its commands (`cargo run -p xtask -- repo`). It does not run them yet:
running them end to end against a test server waits for the package that
ships the `gunmetal` command line.

## Before you start

Work from a machine you trust, not from the possibly compromised server's
web UI. You need host access (SSH or a console) and a backup older than
the compromise.

## Steps

1. **Verify the audit log** (`gunmetal audit verify`). If the chain is
   broken, treat the live log as hostile and continue from a backup
   checkpoint (ADM-145).
2. **Rotate every server key** (`gunmetal keys rotate`). This re-encrypts
   stored secrets, ends every session and signed URL, and writes an audit
   record (ADM-144).
3. **Revoke devices** (`gunmetal admin devices revoke-all`). Pairing
   devices again is cheaper than leaving an attacker's device enrolled.
4. **Review admins and plugins** (`gunmetal admin users review`). Confirm
   every administrator and every plugin is one you meant to have.
5. **Restore from a backup older than the compromise**
   (`gunmetal restore --before COMPROMISE`). Use a backup whose checkpoint
   the audit log still extends.
6. **Update** (`gunmetal update`). Install the latest fixed release after
   the restore.

```runbook
gunmetal audit verify
gunmetal keys rotate
gunmetal admin devices revoke-all
gunmetal admin users review
gunmetal restore --before COMPROMISE
gunmetal update
```

## After

Watch the security summary and the audit log for a few days. If an
advisory listed indicators of exploitation, search the restored log for
those events (SEC-OPS-067).
