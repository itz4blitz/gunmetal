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
  a bot, and one from a fork. The job's time limit follows the same
  condition: 75 minutes on GitHub's runners, 15 hours on the project's.
- A bot's pull request stays on GitHub's runners because nobody has read
  what it carries. Dependabot's branch is in this repository, but it
  brings new versions of crates and actions, and their build scripts,
  macros and code would run on the build host. Any other GitHub App that
  can write to the repository can open a pull request from one of its
  branches in the same way. The workflow tests the type of the pull
  request's author (`github.event.pull_request.user.type != 'User'`),
  which never changes. It fails closed: an author of any type but a
  person's account, or of no type, stays on GitHub's runners. It does not
  test `github.actor`, which names whoever caused the event that started
  the run: that stops being the bot when a maintainer pushes to the
  branch, updates it or reopens the pull request, while the unread change
  is still in it. For a bot account's pull request this line of the
  workflow is the only control; no setting stands behind it. The
  workspace rules test pins the line and the time limit's line whole.
- The author test holds only for an app acting as its own bot account.
  An app that opens a pull request on a person's authorisation is
  recorded by GitHub as that person, and so is a coding agent that pushes
  with a person's account. Both are a person's pull requests to this
  workflow, and a person's pull request into `main` runs on the build
  host. Of thirty public pull requests sampled on 2026-10-05 whose text
  links a ChatGPT Codex task, twenty-eight have a person as author.
- The job is unchanged otherwise: it has a read-only token and no secrets
  (SEC-SUP-012, SEC-SUP-013), every action is pinned (SEC-SUP-010), and it
  runs `scripts/gate.sh` with the same switches.
- The runners are four containers on the project's build host, registered
  to this repository alone. As read on 2026-10-05 from their compose file
  and from the running containers: each is capped at 4 CPUs, 12 GB and
  4,096 processes, has `no-new-privileges` set, is not privileged, has no
  Docker socket, and runs its jobs as an unprivileged user with no
  capabilities. The compose file sets no network, so by inference, not by
  test, a job can reach the internet, the build host itself, other
  containers' published ports and the host's local network. Their set-up
  and the commands to manage them are kept beside them on the host.
- These runners keep state between jobs, so release workflows must not
  use them. That rule is made here, in the spirit of SEC-SUP-015 (no
  restored caches in a release build) and SEC-SUP-040 (a build that two
  runners reproduce); neither requirement states it, and nothing enforces
  it yet.

## What keeps outsiders' code off the build host

Not the workflow. A pull request runs the workflow file of its own branch,
so it can rewrite the line that chooses the runner. The fork test in that
line sends an honest fork pull request to GitHub's runners and does
nothing against a hostile one.

Three settings are the control against people outside the project. This
is how they read through GitHub's API on 2026-10-05:

- **The repository is private** (`private` is `true`). Only people and
  apps the organisation has given access can read it or open a pull
  request.
- **Forking is off** (`allow_forking` is `false` on the repository, and
  `members_can_fork_private_repositories` is `false` on the organisation).
  Someone who can read the repository cannot fork it, so no fork pull
  request can exist. The repository has no forks.
- **Workflows from fork pull requests are off**
  (`run_workflows_from_fork_pull_requests` is `false` in the
  organisation's setting for private repositories and in the
  repository's own). If a fork did exist, its pull request would start no
  workflow.

All three must stay as they are for as long as these runners are
registered to the repository. Proposed, and waiting for the owner's
answer: before the repository is made public the runners are removed from
it, or a new record replaces this one, because anyone can fork a public
repository and the third setting covers private repositories only.

## Who can run code on the build host

Write access to the repository is the ability to run code inside those
containers, because the runners build and test whatever is pushed to the
repository's own branches. On 2026-10-05 that is:

- **Three accounts with admin rights:** `itz4blitz`, `kbdevopz` and
  `towersofscenery`. A fourth account has read access only.
- **Coding agents acting as those accounts.** Their merges into a wave
  branch, and the lock-file changes in them, run on the build host before
  the owner reads the wave.
- **What every full run executes:** the build scripts and procedural
  macros of every locked crate, and the pinned actions the job uses.
- **GitHub Apps installed on the organisation with write access to
  repository contents.** Six of the eleven installed apps have it. On
  every repository: `slack` and `expo`, which may also edit workflows,
  and `cloudflare-workers-and-pages`, which has administration rights:
  it can change a repository's visibility, its forking setting and
  `main`'s ruleset, and register runners. On selected repositories:
  `chatgpt-codex-connector`, which may also edit workflows and usually
  opens its pull requests in the name of the person who asked,
  `coderabbitai` and `premierstudio-local-dev`; whether this repository
  is among those selected was not checked.

The workflow holds only part of that. A pull request an app opens as its
own bot account stays on GitHub's runners, by the author test above.
Three things it does not hold: an app that may edit workflows can rewrite
the line in its own pull request; an app or agent acting on a person's
authorisation is a person to it; and a push straight to a wave branch is
an ordinary full run, whoever pushed. The wave branches have no ruleset;
only `main` has one. Creating a new branch whose name begins `wave-` is
also a push the trigger matches, and needs only write access to
contents. So the list of installed apps and their permissions, and the
absence of protection on existing and new wave branches, are settings
this decision depends on. Whether to narrow the apps to the repositories
that need them and to protect the `wave-` names is the owner's to
decide.

Nothing watches these settings when this record is written. The `Settings
drift` workflow is on the wave branches but not yet on `main`, so its
daily schedule has never run. When it runs it will download the
repository's and the organisation's settings, which is where the first two
are found, but it compares neither: in those answers it checks secret
scanning, push protection and the two-factor requirement. It does not
download the third at all. Until it runs and compares all three, a change
to any of them goes unnoticed.

## Consequences

If the runners work as intended, a full gate finishes, in hours that
depend on how many runners are online: ten shards share them. On four
runners ten shards are three rounds, which by the estimate of 4.5 to 8
hours a shard is 13.5 to 24 hours. Twenty shards (a push's full run
beside a pull request's) are five rounds, and the last would wait 18 to
32 hours for a runner; GitHub cancels a job that has waited 24. The shards
of a person's full run no longer cost hosted runner time. The `checks`
job still runs on GitHub's runners, and so do all ten shards of a bot's
pull request into `main`, to the 75-minute limit, each time it is updated.

When the runners are stopped, full-run shards wait in the queue and `gate`
does not pass; package pull requests are not affected.

The gate's result now depends on the build host: its kernel, its Docker
version and the containers' settings are part of what a full run tests
against, and they differ from GitHub's runners (see "What is not proven").

A bot's pull request into `main` runs its full mutation shards on
GitHub's runners with the 75-minute limit, which the full runs on `wave-1`
and `wave-2` did not fit in, so it will not pass `gate`. Proposed, and
waiting for the owner's answer: a maintainer who has read the update
brings it in through a branch and pull request of their own, which go to
the project's runners. The same would hold for a pull request from a fork
into `main`, if the settings above ever allowed one.

The nightly run uses `main`'s copy of the workflow, so it moves to the
project's runners only when this change reaches `main`.

The workflow cancels a branch's run when a newer commit is pushed to that
branch, and that is unchanged. A full run now takes hours, so every merge
into a wave branch restarts that wave's full run, and it finishes only
when nothing merges for that long. While a wave has a pull request into
`main` open, each push starts two full runs, one for the push and one for
the pull request, and twenty shards share the runners.

## What is not proven

No job has run on these runners when this record is written. The four
runners are registered and show as online, and that is all that has been
seen. The pull request that brings this change goes into a wave branch, so
its own shards run on GitHub's runners and prove nothing about the
project's.

One failure is already known, so it is not an open question. Inside the
runner containers, as read on 2026-10-05, the kernel is 6.18.38-Unraid,
`/sys/kernel/security` does not exist, Yama is absent, and Docker 29.5.3's
default profile has already installed a seccomp filter. The worker
sandbox's tests on `wave-1` read `/sys/kernel/security/lsm` and assume a
thread starts with no filter, so there every shard would fail on the
unmutated tree. Pull request #88 changes those tests and is the
prerequisite: a full run before it is on the branch proves nothing about
mutation testing on these runners. The first proof is the first full run
with both this change and #88 on `wave-1`.

Until that run finishes, these are expectations and not results: that the
job's steps (the toolchain, the cache, the tool installer and
`scripts/gate.sh`) work on these runners at all; that the worker's
confinement tests pass under Docker's default profile, in particular
whether the Landlock calls get through it; the 4.5 to 8 hours for a
shard; and that 15 hours is enough. The bot rule is untried as well: no
bot has opened a pull request in this repository yet.
