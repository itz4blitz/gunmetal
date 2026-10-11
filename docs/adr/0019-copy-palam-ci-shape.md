# 19. Forgejo burst runners copy Palam's `ph-ci` shape

Date: 2026-10-06
Status: accepted, through the owner's direction on 2026-10-06
([D-92](../decisions.md#d-92-runner-shape))

## Context

[Record 18](0018-forgejo-tenants-and-ecs.md) split tenants and said
Premier burst compute must not live in `palam-cicd`. The next sentence
in that conversation still assumed a new ECS Fargate stack for Forgejo.

Palam already has a working burst pool: Terraform in
`PalamHealth/PalamHealth` `infra/ci-runners`, state in
`s3://ph-ci-tfstate-178231953954`, ephemeral Ubuntu spot instances in
`palam-cicd` / `us-east-2`, label `ph-ci` only, max 9, public IPv4, no
NAT, daily/monthly budgets and a halt SCP. It is not an ECS cluster.
The module is `github-aws-runners/terraform-aws-github-runner` v6.8.3,
which talks to GitHub, not Forgejo.

On 2026-10-06 the owner asked why Forgejo would be installed a different
way than that pool.

## Decisions

1. **Do not invent a new ECS Fargate stack for Forgejo.** Burst capacity
   copies Palam's shape: Terraform, a dedicated account, ephemeral VMs
   that scale from queued jobs, and a spend fuse (budgets + halt).
2. **The only justified differences are the agent and how it reaches the
   forge.** Gunmetal's forge is Forgejo on the Premier tailnet, so the
   box runs a Forgejo runner, not the GitHub runner binary, and it must
   join the tailnet (or another path the forge already has). That is
   user-data, not a different orchestrator.
3. **Palam's existing `ph-ci` pool stays as it is.** It keeps serving
   `PalamHealth/PalamHealth` on GitHub. Premier and personal Forgejo
   jobs do not register to it. Palam Forgejo jobs, when they exist, get
   their own pool in `palam-cicd` with the same shape, not a move of
   `ph-ci` onto ECS.

## Consequences

Record 18's tenant split still holds. Mentions of "Premier ECS" in that
record are the abandoned path. IaC for Premier burst still does not land
in `palam-cicd` and is not a new Gunmetal crate.
