# 15. The project's own runners for the full mutation gate

Date: 2026-10-05
Status: accepted, through the owner's answer on 2026-10-05

## Context

Decision D-01 has two kinds of gate. A package pull request into a wave
branch mutation-tests only the code it changes. Every push to `main` or a
wave branch, every pull request into `main` and the nightly run on `main`
mutation-test the whole workspace, and the job named `gate` is the one
check a pull request into `main` must pass.

The workspace has about 8,000 mutants when this is written, each costing 20
to 35 seconds of build and test. On GitHub's hosted runners the full run was
split into ten shards with a 75-minute limit, and on 2026-10-04 all ten hit
that limit on `wave-1` and on `wave-2`. So no wave could show a green full
gate, and none could merge into `main`. Finishing on hosted runners would
take about sixty shards and sixty to seventy runner-hours for every full
run.

## Decision

The full run's mutation shards run on the project's own GitHub Actions
runners, labelled `gunmetal-mutants`. Everything else stays on GitHub's
runners: the `checks` job, a package pull request's diff-scoped shards, and
every other workflow.

- The `mutants` job chooses its runner from the event. A push, the nightly
  run and a pull request into `main` from this repository go to the
  project's runners. A pull request into a wave branch, and any pull
  request from a fork, stays on GitHub's runners, so nobody outside the
  project runs code on the project's hardware.
- The job is unchanged otherwise: it has a read-only token and no secrets
  (SEC-SUP-012, SEC-SUP-013), every action is pinned (SEC-SUP-010), and it
  runs `scripts/gate.sh` with the same switches.
- The runners are containers on the project's build host with a CPU and
  memory cap, no Docker socket and no new privileges. Their set-up and the
  commands to manage them are kept beside them on the host.
- These runners keep state between jobs. Release workflows never use them:
  release builds need fresh runners (SEC-SUP-015, SEC-SUP-040).

## Consequences

A full gate finishes, in hours that depend on how many runners are online:
ten shards share them. A full run no longer costs hosted runner time.

When the runners are stopped, full-run shards wait in the queue and `gate`
does not pass; package pull requests are not affected.

A pull request from a fork into `main` still runs its full mutation shards
on hosted runners and will time out there. A maintainer has to bring such
a change in through a branch of this repository.

The runners build and test whatever is pushed to the repository's own
branches, so write access to the repository is also the ability to run
code on the build host inside those containers.
