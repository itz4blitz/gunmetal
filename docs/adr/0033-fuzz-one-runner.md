# 33. Fuzz one harness at a time

Date: 2026-10-11

Status: accepted, through the owner's request on 2026-10-11 that the
pipelines take about ten minutes rather than about ten hours
([D-97](../decisions.md#d-97-fuzz-one-runner)).

## Context

[SEC-MED-029](../security/media-and-parser-safety.md) requires ten
minutes of fuzzing per harness on a pull request that changes
`gunmetal-core`, and one CPU-hour per harness every night. The fuzz
workflow applied that by starting every harness at once. With about
fifty harnesses and four runners, the nightly pass held the whole pool
for about ten hours, and the pull-request pass for about two. The merge
gate uses those same four runners. On 2026-10-11 the nightly run left
the required `checks` and `gate` jobs queued with no runner.

The durations stay. What changes is how many harnesses run at once.

## Decisions

1. **The fuzz matrix and the canary matrix each run one job at a time.**
   Three runners stay available for the merge gate and the other
   workflows. `max-parallel` is 1 on both jobs.
2. **The time budgets stay.** A pull request still fuzzes for 600
   seconds per harness, and a nightly or manual run for 3600 seconds.
   The 5 second input timeout and the 1 GiB RSS limit are unchanged, and
   the canaries still have to fail on the planted crash, timeout and
   out-of-memory input.
3. **A new nightly run waits for the one already going.** A full pass on
   one runner takes longer than a day, so cancelling it at the next
   cron would drop harnesses before they had their hour. A new push to
   a pull request still cancels that pull request's fuzz run.

## Consequences

A full nightly pass takes about two days on one runner, and the
following night's run waits. Each harness still gets its hour, but not
all of them inside one calendar night. A pull request that changes the
parsers still fuzzes every harness for ten minutes, on one runner, so
that run is hours long and does not block the other three. The required
merge check remains `checks` plus `gate`, and `gate` still waits for
the full mutation shards. This record does not shorten those shards.
