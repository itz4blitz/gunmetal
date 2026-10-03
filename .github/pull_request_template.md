## What this changes

-

## Testing

- [ ] `scripts/gate.sh` passed, or this pull request is into a wave branch
      and `GATE_MUTANTS_DIFF=<wave-ref> scripts/gate.sh` passed.

## Security fix

If this change fixes a vulnerability, every box is required
(SEC-TM-003, SEC-OPS-066, SEC-HIS-064):

- [ ] A regression test named after the advisory ID (for example
      `ghsa_xxxx_xxxx_xxxx_rejects_the_payload`) fails on the parent
      commit and passes here.
- [ ] A class review searched the server, core, clients and adapters for
      the same root cause, and the result is written in this description.
- [ ] The root cause is recorded in the advisory.

## Agents

Pull requests written by a coding agent carry the `agent-written` label
and are reviewed as outside contributions. Agents run without release
credentials (SEC-SUP-055).
