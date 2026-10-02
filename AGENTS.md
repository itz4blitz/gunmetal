# Gunmetal: notes for coding agents

- Read [CONTRIBUTING.md](CONTRIBUTING.md) before writing any code. The
  testing rules there are mandatory: test first, deep assertions, 100%
  coverage, zero surviving mutants.
- `scripts/gate.sh` is the definition of done. Run it before reporting work
  as finished, and report its real output.
- Architecture decisions live in `docs/adr`. Add a new record when you make
  one; do not rewrite old records.
- `gunmetal-core` is pure logic: no I/O, no `unsafe`, no panics on any input.
  Malformed media must come back as a typed error.
- Do not add a code directory, crate or dependency until a test needs it.
- Research and design writing lives under `docs/`: `docs/research` (what
  exists and what people want), `docs/features` (the feature map, with stable
  feature IDs), `docs/ui` (surfaces derived from the features) and
  `docs/plan` (work packages derived from both). Code follows the plan, and
  the plan follows the feature map, not the other way round.
