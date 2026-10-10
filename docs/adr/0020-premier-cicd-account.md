# 20. Dedicated Premier CI account

Date: 2026-10-06
Status: accepted, through the owner's direction on 2026-10-06
([D-93](../decisions.md#d-93-premier-ci-account))

## Context

[Record 18](0018-forgejo-tenants-and-ecs.md) said Premier burst must not
live in `palam-cicd`, and parked it in the `premierstudio` management
account until a dedicated Premier CI account existed. [Record 19](0019-copy-palam-ci-shape.md)
said the pool copies Palam's `ph-ci` shape. On 2026-10-06 the owner said
to make that work, and to keep a separate AWS account for it.

## Decisions

1. **Premier Forgejo burst lives in `premier-cicd`.** Account
   `109792548422`, email `aws-premier-cicd@premierstudio.ai`, region
   `us-east-2`, created 2026-10-06 under Root. IaC is
   `PremierStudio/premier-cicd` on the forge, not a Gunmetal crate and
   not PalamHealth's `infra/ci-runners`.
2. **Do not reuse Palam's SCPs, budgets, state bucket, or runner token.**
   This account gets its own operating SCP, halt SCP, daily/monthly
   budgets, Terraform state, Tailscale key, and Forgejo org runner
   registration.
3. **`palam-cicd` (`178231953954`) stays Palam's.** `ph-ci` is
   untouched. The Unraid site runner stays the always-on floor.

## Consequences

Record 18's "until a dedicated account exists" clause is this account.
SSO profile `premier-cicd` targets `109792548422`. Gunmetal
`runs-on` stays `[self-hosted, gunmetal-mutants]`.
