# 18. Separate Forgejo tenants and AWS accounts

Date: 2026-10-06
Status: accepted, through the owner's direction on 2026-10-06
([D-91](../decisions.md#d-91-forgejo-tenants))

## Context

[Record 16](0016-forgejo-source-of-truth.md) put Gunmetal on the Premier
Studio Forgejo at `https://git.taild1bbf.ts.net`. Until this record the
forge had one organisation, `PremierStudio`, and the only planned AWS
home for burst Actions runners was `palam-cicd` (account
`178231953954`, `us-east-2`).

On 2026-10-06 the owner directed that Palam be tracked separately from
Premier Studio, and that Justin's and Karlis's personal repositories stay
out of both organisations. Mixing those tenants would put Palam's bill
and runner pool on Premier work (and the reverse), and would drop
personal repos into a client org.

## Decisions

1. **Four tenants on the one Forgejo, never mixed.**
   - `PremierStudio` — Premier Studio team repositories (Gunmetal lives
     here).
   - `Palam` — Palam repositories only. Created 2026-10-06, limited
     visibility (signed-in users).
   - `itz4blitz` — Justin's personal repositories, under his user, not
     under either organisation.
   - `kbdevopz` — Karlis's personal repositories, under his user. The
     account was created 2026-10-06; the initial password is in
     1Password and must be changed on first sign-in.

2. **AWS follows the same split.** Palam burst runners, if any, live in
   `palam-cicd` (`178231953954`, `us-east-2`). Premier burst runners live
   in the `premierstudio` management account (`169011967880`) until a
   dedicated Premier CI account exists. Personal repositories do not use
   Palam's AWS account. Premier work does not create ECS services in
   `palam-cicd`.

3. **Always-on capacity stays on the Unraid site runner.**
   `unraid-premier-01` remains a global runner so every tenant can run
   Actions once the repository unit is on. Burst ECS, when it exists, is
   registered to the tenant that pays for it (organisation-scoped for
   `PremierStudio` and `Palam`; user-scoped for personal accounts if they
   need more than the Unraid floor).

4. **Gunmetal's `runs-on` line does not change.** Every Gunmetal job
   still uses `[self-hosted, gunmetal-mutants]`
   ([record 17](0017-all-actions-on-own-runners.md)). The Unraid runner
   already carries that label. Premier ECS, when added, must advertise
   the same label. Palam and personal workflows choose their own labels
   and must not pin `gunmetal-mutants` unless they intend to share that
   pool.

## Consequences

A Palam repository is created under `https://git.taild1bbf.ts.net/Palam`,
not under `PremierStudio`. A personal repository stays under the person's
user. IaC for Premier runners does not land in `palam-cicd` and is not a
new Gunmetal crate. This record does not stand the ECS services up; it
names where they are allowed to live.
