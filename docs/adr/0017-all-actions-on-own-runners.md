# 17. Every Actions job runs on the project's own runners

Date: 2026-10-06
Status: accepted, through the owner's direction on 2026-10-06
([D-90](../decisions.md#d-90-actions-runners))

## Context

[Record 15](0015-own-runners-for-the-full-mutation-gate.md) put the full
mutation shards on the Unraid runners labelled `gunmetal-mutants`, and
left everything else on GitHub-hosted `ubuntu-latest`: the `checks` job,
wave-branch diffs, bot and fork pull requests, and every other workflow.

[Record 16](0016-forgejo-source-of-truth.md) moved the source of truth to
Forgejo on the Premier tailnet, and left CI on GitHub Actions until a
named replacement existed.

On 2026-10-06 the owner directed both remaining halves closed: the
project lives on Forgejo, not GitHub, and Actions must use the
self-hosted runners for every job.

## Decisions

1. **Every workflow job runs on `[self-hosted, gunmetal-mutants]`.** There
   is no GitHub-hosted `ubuntu-latest` path, including for bots, forks,
   wave-branch diffs, `checks`, CodeQL, Scorecard and the lint jobs.
2. **Mutation shards keep a 15-hour limit on those runners.** The 75-minute
   hosted limit is gone, because nothing is hosted.
3. **The workflow files stay under `.github/workflows/`.** Forgejo Actions
   reads that path. A second copy under `.forgejo/workflows/` is not added,
   so the two forges cannot start two runs of the same job from two trees.
4. **GitHub-hosted compute is not part of the gate.** A green check that
   ran on `ubuntu-latest` is not the project's CI. Record 15's hosted
   exceptions no longer apply. The settings that keep outsiders off the
   build host (private repository, forking off) still do; this record
   does not replace them.

## Consequences

`workspace_rules` pins every job's `runs-on` line. Changing one job back
to a hosted runner fails that test. Release workflows, when they exist,
still must not use these runners (record 15): they keep state between
jobs. CodeQL and Scorecard now run on the same hardware; they remain
GitHub-oriented tools until a later record replaces them.
