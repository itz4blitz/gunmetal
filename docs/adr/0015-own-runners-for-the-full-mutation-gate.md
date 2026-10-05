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

- The `mutants` job chooses its runner from the event. A push to `main` or
  a wave branch, the nightly run, and a pull request into any branch that
  is not a wave branch go to the project's runners. Three kinds of pull
  request stay on GitHub's runners: one into a wave branch, one opened by
  Dependabot, and one from a fork. The job's time limit follows the same
  condition: 75 minutes on GitHub's runners, 15 hours on the project's.
- A Dependabot pull request stays on GitHub's runners because of what it
  carries. Its branch is in this repository, but it brings new versions of
  crates and actions that nobody has read yet, and their build scripts,
  macros and code would run on the build host. The workflow tests the pull
  request's author (`github.event.pull_request.user.login`), which never
  changes. It does not test `github.actor`, which names whoever caused the
  event that started the run: that stops being Dependabot when a
  maintainer pushes to the branch, updates it or reopens the pull request,
  while the unread dependencies are still in it. For Dependabot this line
  of the workflow is the only control; no setting stands behind it.
- The job is unchanged otherwise: it has a read-only token and no secrets
  (SEC-SUP-012, SEC-SUP-013), every action is pinned (SEC-SUP-010), and it
  runs `scripts/gate.sh` with the same switches.
- The runners are containers on the project's build host with a CPU and
  memory cap, no Docker socket and no new privileges. They are registered
  to this repository alone. Their set-up and the commands to manage them
  are kept beside them on the host.
- These runners keep state between jobs. Release workflows never use them:
  release builds need fresh runners (SEC-SUP-015, SEC-SUP-040).

## What keeps outsiders' code off the build host

Not the workflow. A pull request runs the workflow file of its own branch,
so it can rewrite the line that chooses the runner. The fork test in that
line sends an honest fork pull request to GitHub's runners and does
nothing against a hostile one.

Three settings are the control. This is how they read through GitHub's API
on 2026-10-05:

- **The repository is private** (`private` is `true`). Only people the
  organisation has given access can read it or open a pull request.
- **Forking is off** (`allow_forking` is `false` on the repository, and
  `members_can_fork_private_repositories` is `false` on the organisation).
  Someone who can read the repository cannot fork it, so no fork pull
  request can exist.
- **Workflows from fork pull requests are off**
  (`run_workflows_from_fork_pull_requests` is `false` in the
  organisation's setting for private repositories). If a fork did exist,
  its pull request would start no workflow. The repository's own value
  could not be read when this was written; GitHub documents that a
  repository cannot turn on what its organisation has turned off.

All three must stay as they are for as long as these runners are
registered to the repository. Before the repository is made public the
runners are removed from it, or a new record replaces this one: anyone can
fork a public repository, and the third setting covers private
repositories only.

Nothing watches these settings when this record is written. The `Settings
drift` workflow downloads the repository's and the organisation's settings
every day, which is where the first two are found, but it compares
neither: in those answers it checks secret scanning, push protection and
the two-factor requirement. It does not download the third at all. Until
it compares all three, a change to any of them goes unnoticed.

## Consequences

If the runners work as intended, a full gate finishes, in hours that
depend on how many runners are online: ten shards share them. A full run
no longer costs hosted runner time.

When the runners are stopped, full-run shards wait in the queue and `gate`
does not pass; package pull requests are not affected.

A Dependabot pull request into `main` runs its full mutation shards on
GitHub's runners with the 75-minute limit, which the full runs on `wave-1`
and `wave-2` did not fit in. Once those waves are in `main` it cannot pass
`gate`. A maintainer who has read the update brings it in through a branch
and pull request of their own, which go to the project's runners. The
same would hold for a pull request from a fork into `main`, if the
settings above ever allowed one.

The workflow cancels a branch's run when a newer commit is pushed to that
branch, and that is unchanged. A full run now takes hours, so every merge
into a wave branch restarts that wave's full run, and it finishes only
when nothing merges for that long. While a wave has a pull request into
`main` open, each push starts two full runs, one for the push and one for
the pull request, and twenty shards share the runners.

The runners build and test whatever is pushed to the repository's own
branches, so write access to the repository is also the ability to run
code on the build host inside those containers.

## What is not proven

No job has run on these runners when this record is written. The four
runners are registered and show as online, and that is all that has been
seen. The pull request that brings this change goes into a wave branch, so
its own shards run on GitHub's runners and prove nothing about the
project's. The first proof is the first full run after it merges: the push
to `wave-1`.

Until that run finishes, these are expectations and not results: that the
job's steps (the toolchain, the cache, the tool installer and
`scripts/gate.sh`) work on these runners at all; the 4.5 to 8 hours for a
shard; and that 15 hours is enough. The Dependabot rule is untried as
well: Dependabot has opened no pull request in this repository yet.
