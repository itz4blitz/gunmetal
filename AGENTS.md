# Gunmetal: notes for coding agents

- Read [CONTRIBUTING.md](CONTRIBUTING.md) before writing any code. The
  testing rules there are mandatory: test first, deep assertions, 100%
  coverage, zero surviving mutants.
- `scripts/gate.sh` is the definition of done. Run it before reporting work
  as finished, and report its real output.
- When the turn changes Rust, Cargo, Clippy, or Qodana files, `scripts/qodana.sh`
  has to pass before the work is finished, and its real output is part of the
  report. It fails when Cargo did not load, when the sanity check still
  reports a name the compiler accepts, or when the scan reports any problem.
  The project token stays in `QODANA_TOKEN` or
  `~/.config/qodana/token`, never in the tree. Claude Code, Cursor, Codex, and
  Grok stop hooks run that scan and refuse to end the turn while it fails.
  Grok reads `.grok/hooks/qodana.json` after this folder is trusted.
- Architecture decisions live in `docs/adr`. Add a new record when you make
  one; do not rewrite old records.
- `gunmetal-core` is pure logic: no I/O, no `unsafe`, no panics on any input.
  Malformed media must come back as a typed error.
- Do not add a code directory, crate or dependency until a test needs it.
- The source of truth is Forgejo on the Premier tailnet:
  `https://git.taild1bbf.ts.net/PremierStudio/gunmetal`
  ([D-89](docs/decisions.md#d-89-source-of-truth-forge),
  [record 16](docs/adr/0016-forgejo-source-of-truth.md)). Clone, push and
  open pull requests there. GitHub is not the working forge. Actions run
  only on the project's self-hosted runners
  ([D-90](docs/decisions.md#d-90-actions-runners),
  [record 17](docs/adr/0017-all-actions-on-own-runners.md)). Palam,
  Premier Studio, and personal repos (`itz4blitz`, `kbdevopz`) are
  separate tenants; Premier burst does not live in `palam-cicd`
  ([D-91](docs/decisions.md#d-91-forgejo-tenants),
  [record 18](docs/adr/0018-forgejo-tenants-and-ecs.md)).
  Burst runners copy Palam's `ph-ci` shape, not a new ECS stack
  ([D-92](docs/decisions.md#d-92-runner-shape),
  [record 19](docs/adr/0019-copy-palam-ci-shape.md)). Premier burst
  lives in AWS account `premier-cicd` `109792548422`, IaC
  `PremierStudio/premier-cicd`
  ([D-93](docs/decisions.md#d-93-premier-ci-account),
  [record 20](docs/adr/0020-premier-cicd-account.md)).
- Pull requests written by a coding agent carry the `agent-written` label
  and are reviewed exactly like a contribution from an outside contributor:
  a human reads every line, confirms that each new dependency exists and is
  the intended crate, and approves before merge. Agents never merge into
  `main`, tag or release (SEC-STD-035, SEC-SUP-055). During the build,
  package pull requests merge into their wave branch (`wave-0`, `wave-1`,
  ...) once the gate passes, through the wave's integrator agent; the owner
  reviews and merges each wave's single pull request into `main`
  ([decision D-01](docs/decisions.md#owner-answers-2026-10-02)). Changes to the paths in
  `.github/CODEOWNERS` also need a code owner's approval (SEC-SUP-005).
- Research and design writing lives under `docs/`: `docs/research` (what
  exists and what people want), `docs/features` (the feature map, with stable
  feature IDs), `docs/ui` (surfaces derived from the features) and
  `docs/plan` (work packages derived from both). Code follows the plan, and
  the plan follows the feature map, not the other way round.
