# 34. CI does not run mutation testing

Date: 2026-10-11

Status: accepted, through the owner's request on 2026-10-11 to stop the
mutation shards. A full pass takes about a day, and that is not a merge
gate the project can use
([D-98](../decisions.md#d-98-no-mutation-gate)).

## Context

[Record 15](0015-own-runners-for-the-full-mutation-gate.md) and
[record 29](0029-mutation-build-cache.md) put the full mutation run on
the project's own runners, as ten shards behind a warm job. On
2026-10-11 those shards were still running after more than an hour and
a half, with six of the ten still queued, and the four runners were
busy the whole time. The required `gate` check waits for every shard,
so a pull request whose `checks` job had already passed stayed blocked.
The owner stopped the runs and said to get rid of them.

`scripts/gate.sh` still accepts the switches. A person can still run
one shard. CI does not.

## Decisions

1. **CI has no `mutants` job and no `mutants-warm` job.** Nothing in
   `.github/workflows/ci.yml` installs `cargo-mutants` or sets
   `GATE_MUTANTS_SHARD`.
2. **`checks` skips mutation testing.** It runs `scripts/gate.sh` with
   `GATE_SKIP_MUTANTS=1`. That job, the 32-bit core test, and the two
   self-tests are what `gate` waits for.
3. **The script still accepts the switches.** `GATE_SKIP_MUTANTS`,
   `GATE_MUTANTS_SHARD`, `GATE_MUTANTS_DIFF`, and `GATE_MUTANTS_WARM`
   keep their meanings, and the switch self-test still refuses a value
   it cannot read. They are not a merge check.

## Consequences

A pull request's required CI is the short jobs. It no longer holds the
four runners for a day, and a cancelled or unfinished mutant shard can
no longer fail `gate`. A local `scripts/gate.sh` with no switch still
runs mutation testing; the done-check is `GATE_SKIP_MUTANTS=1
scripts/gate.sh`. Records 15 and 29 stay as the history of the shard
design. This record is what CI does now.
