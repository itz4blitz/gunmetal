# Web player work packages

Written on 2026-10-03. Status: draft for the project owner's review.

This is the build plan for the R1 web player: the client that the
[backend plan](work-packages.md) leaves out. On 2026-10-03 the owner chose
to start it now, in parallel with the server waves and against a small
fake server, so that there is a real screen to click within days
([decision register](../decisions.md#owner-answers-2026-10-02), "Web
player"). It turns the R1 parts of [surfaces.md](../ui/surfaces.md),
[flows.md](../ui/flows.md), [player.md](../ui/player.md) and
[design-language.md](../ui/design-language.md) into work packages that one
coding agent each can build, under the rules in
[CONTRIBUTING.md](../../CONTRIBUTING.md) and [AGENTS.md](../../AGENTS.md).
The architecture choices it makes are recorded in
[record 12](../adr/0012-web-client-toolchain-and-contracts.md).

The [feature map](../features/README.md) and the interface documents are
the source of truth. This plan follows them and invents no feature. The
[security baseline](../security/README.md) outranks all of them; where an
interface document and the baseline disagree, the plan follows the
baseline and says so in
[Where this plan follows the baseline over a UI document](#where-this-plan-follows-the-baseline-over-a-ui-document).

Client packages are numbered CP-001 to CP-059. The backend's packages are
WP-NNN, so the two ranges can never collide, and a CP number is never
reused. Client waves are C0 to C5; server waves are plain numbers (wave 0
to wave 6). Claims this plan could not check are marked "(unverified)" and
collected in [What this plan could not verify](#what-this-plan-could-not-verify).
Choices that go beyond the source documents are marked **Proposal**.

Only R1 is planned here. The point releases (R1.1 to R1.3) add to these
surfaces without moving anything, and each package says which parts it
leaves for them.

## Contents

1. [How to use this plan](#how-to-use-this-plan)
2. [Ground rules every client package follows](#ground-rules-every-client-package-follows)
3. [Where the code lives](#where-the-code-lives)
4. [Toolchain](#toolchain)
5. [Dependency policy and the proposed npm packages](#dependency-policy-and-the-proposed-npm-packages)
6. [The gate and CI: requests to the integrator](#the-gate-and-ci-requests-to-the-integrator)
7. [The contract: two ports](#the-contract-two-ports)
8. [The fake server](#the-fake-server)
9. [Core logic before the WASM facade](#core-logic-before-the-wasm-facade)
10. [Playback on the web in R1](#playback-on-the-web-in-r1)
11. [Working in parallel](#working-in-parallel)
12. [Waves at a glance](#waves-at-a-glance)
13. [Client waves, server waves and what the owner can test](#client-waves-server-waves-and-what-the-owner-can-test)
14. [The first clickable milestone](#the-first-clickable-milestone)
15. [Wave C0: foundations](#wave-c0-foundations)
16. [Wave C1: the first clickable player](#wave-c1-the-first-clickable-player)
17. [Wave C2: the whole listening experience](#wave-c2-the-whole-listening-experience)
18. [Wave C3: signing in, settings and account](#wave-c3-signing-in-settings-and-account)
19. [Wave C4: administration](#wave-c4-administration)
20. [Wave C5: the real server](#wave-c5-the-real-server)
21. [Coverage: every R1 surface and flow has a package](#coverage-every-r1-surface-and-flow-has-a-package)
22. [Security requirements the backend plan left to this plan](#security-requirements-the-backend-plan-left-to-this-plan)
23. [Requests to the backend plan](#requests-to-the-backend-plan)
24. [Where this plan follows the baseline over a UI document](#where-this-plan-follows-the-baseline-over-a-ui-document)
25. [Open questions](#open-questions)
26. [What this plan could not verify](#what-this-plan-could-not-verify)

## How to use this plan

### What a package is

A client package has the same form as a backend package:

- **Wave.** The client wave it belongs to. Client waves are grouped by
  what the owner can test at the end of each, so, unlike the backend's
  waves, a package may depend on another package in its own wave. The
  **Depends on** field gives the order. A package may start as soon as
  everything it depends on has merged; packages with no dependency between
  them run at the same time, because they own disjoint paths.
- **Size.** S is about a day of focused work, M is 500 to 1,500 lines of
  code and tests, L is 1,500 to 3,000. Anything bigger has been split.
- **Depends on.** Client packages (CP) and, where the package needs code
  from the Rust workspace, backend packages (WP) with their wave.
- **Owns.** The exact paths it creates or changes. It edits no other path
  except the registry files in [Working in parallel](#working-in-parallel).
- **Serves.** The surfaces (SUR), flows (F) and feature rows it delivers,
  R1 parts only.
- **Security.** The requirement IDs its tests must verify. Each of those
  tests carries a `Verifies:` line, as
  [CONTRIBUTING.md](../../CONTRIBUTING.md#tests-name-the-requirements-they-verify)
  describes; in TypeScript it is a comment on the line above the test
  (see [Ground rules](#ground-rules-every-client-package-follows)).
- **Builds.** What it makes, and what it leaves to another package or a
  later release.
- **Tests.** The tests that prove it, named so the first failing test can
  be written from this page.
- **Done when.** The observable result, on top of the definition of done
  below.

### Definition of done

A client package is done when `scripts/gate.sh` passes on its branch
rebased on the current client wave branch, with the real output reported
(AGENTS.md). From the moment the integrator applies the gate request in
[The gate and CI](#the-gate-and-ci-requests-to-the-integrator), that script
runs the web checks too: format, lint, types, unit and component tests at
100% coverage, mutation testing with no survivor, the production build
with its bundle checks, and the browser suites (the content security
policy sweep, the single-origin sweep, the accessibility sweep and the
layout contract). Until that request is applied, CP-001 and CP-002 report
the output of `pnpm --dir clients run gate` beside the Rust gate's.

A package is also not done until every requirement in its **Security**
field has a test with a `Verifies:` line, every screen and state it adds
is listed in the sweep registry (`clients/e2e/screens.ts`) so the sweeps
cover it, and every string it shows comes from the message catalogue. A
security requirement is never weakened to make a screen fit; the screen
changes instead.

A package that touches Rust, Cargo, Clippy or Qodana files (CP-013, CP-023
and CP-001's check) also runs `scripts/qodana.sh`, as AGENTS.md requires.

## Ground rules every client package follows

The testing rules in CONTRIBUTING.md apply to TypeScript exactly as they
apply to Rust. This table says how each is enforced, so "the gate passed"
means the same thing in both languages.

| Rule (CONTRIBUTING.md) | How the client enforces it | Built by |
|---|---|---|
| 1. Red, then green | The pull request shows the failing run of the first test before the code that passes it. Review refuses a package whose tests were written after the code. | Review |
| 2. Never weaken a test | Lint fails on `.skip`, `.only`, `.todo`, `test.fails`, and on `eslint-disable`, `@ts-ignore`, `@ts-expect-error` without a stated reason, `v8 ignore`, `istanbul ignore` and `Stryker disable` comments. | CP-002 |
| 3. Assert deeply | Tests compare whole values with `toStrictEqual`, or query the rendered tree by role and accessible name and compare the full text. Lint fails on `toBeTruthy`, `toBeFalsy`, `toBeDefined` and on a test with no assertion. | CP-002 |
| 4. Never derive an expected value from the code under test | Snapshot assertions (`toMatchSnapshot`, `toMatchInlineSnapshot`) fail the lint, because a snapshot is written by the code it checks. Expected values are literals, or come from the design documents' own tables. | CP-002 |
| 5. 100% coverage | Vitest's coverage thresholds are 100 for lines, functions, statements and branches, per file, over every source file whether a test imports it or not. JavaScript has real branch coverage, so branches are counted directly. No exclusions. | CP-002 |
| 6. Zero surviving mutants | StrykerJS runs every mutator over every source file. The gate reads Stryker's JSON report and fails unless every mutant was killed; a survivor, a mutant no test covered, a run-time error and a timeout all fail, because a hang is not a detection. Mutants that do not type-check are discarded by the TypeScript checker, as cargo-mutants discards mutants that do not compile. | CP-002 |
| 7. Prove behaviour at the lowest layer | Rules live in the Rust core and are proved there. The client never re-implements one (see [Core logic before the WASM facade](#core-logic-before-the-wasm-facade)). Component tests prove what a screen shows for a given state; browser tests prove wiring, policy and accessibility only. | Every package |

Further rules:

- **No core logic in TypeScript.** Queue verbs, shuffle orders, gain
  decisions, the player state machine, lyric timing, search, Home rows,
  the playback decision, link and URL filtering, text normalisation and
  wire decoding are the core's. The client calls them through `CorePort`.
  A pull request that writes one of them in TypeScript is refused, even
  as a stand-in.
- **Untrusted text is text.** Every string from a file, a provider,
  another person or a device is rendered through the kit's `Text`
  component (CP-006), in a bidirectional isolate, never through an HTML
  sink (SEC-CLI-001, SEC-MED-057). No such string ever reaches a style
  value, a class name, an element ID or a URL the client builds
  (design-language, section 1).
- **Tokens only.** Components use token names from CP-004 and never raw
  colours, sizes or durations. Lint fails on a colour literal outside the
  tokens package.
- **Every string is a message.** Interface text comes from the message
  catalogue (CP-008), so the pseudo-locale build finds hard-coded strings
  (design-language requirement A20). R1 ships English only.
- **Every screen is swept.** A package that adds a route or a state adds
  one line to the sweep registry. The policy, single-origin, hostile
  metadata and accessibility sweeps then visit it in Chromium, Firefox and
  WebKit with the production build.
- **`Verifies:` in TypeScript.** The line is a comment directly above the
  test:

  ```ts
  // Verifies: SEC-CLI-001, SEC-MED-057
  test("a title holding markup is shown as its literal text", () => {
  ```

  The traceability check (WP-127) has to read these files; that is a
  [request to the backend plan](#requests-to-the-backend-plan).
- **The client is never the gatekeeper** (SEC-CLI-015). Hiding a control
  is a courtesy. No client test may stand in for the server test of the
  same restriction.

## Where the code lives

**Proposal.** The JavaScript workspace is `clients/` at the repository
root. The root stays a Rust workspace, and nothing outside `clients/`
holds a `package.json`.

```
clients/
  package.json            workspace root: tooling only, private
  pnpm-workspace.yaml     members and the supply-chain settings
  pnpm-lock.yaml
  tsconfig.base.json, eslint.config.js, prettier.config.js,
  vitest.config.ts, stryker.config.json
  tools/gate/             the web gate's own checks (mutation report, bundle)
  packages/
    tokens/               design tokens, themes, the typeface
    ports/                CorePort and ServerPort, shared types, conformance suites
    core-wasm/            loads the gunmetal-wasm build; implements CorePort
    fixtures/             the demo library and generated media
    fake-server/          in-memory ServerPort and the fixture core
    http-server/          ServerPort over HTTP (the real server)
    player/               the audio engine
    app/                  stores and hooks between the ports and the screens
    ui/                   components and surfaces, written on React Native primitives
  apps/
    web/                  the production entry: composes ui with core-wasm and http-server
    demo/                 the same app composed with fake-server; never shipped
  e2e/                    browser suites and the sweep registry
```

`packages/ui` is the part that record 1 (decision 8) shares with the
native apps in R2. It imports React Native primitives only, so the R2
client plan can build it for Android without rewriting a screen. The
packages under it (`ports`, `app`, `tokens`) hold no DOM code either.
`player`, `core-wasm`, `http-server` and `apps/web` are web-only and are
the parts R2 replaces with native modules and UniFFI.

`apps/web` and `apps/demo` differ only in their composition root: which
`ServerPort` and which `CorePort` they pass in. That is the whole of
"switching from the fake to the real server".

## Toolchain

Each choice is the plainest well-known tool that satisfies the baseline.
Record 12 holds the reasoning and the alternatives.

| Job | Choice | Why |
|---|---|---|
| Package manager | pnpm (12.x) | The register's recommendation (D-65) and the baseline's outline: it refuses versions younger than seven days, refuses a drop in publishing trust, blocks install scripts and exotic sources (SEC-SUP-033, SEC-SUP-034). |
| Language | TypeScript 6.0.x, `strict`, `noUncheckedIndexedAccess`, `exactOptionalPropertyTypes` | Type checking is a gate step. 6.0 and not 7.0, because typescript-eslint's stated range stops below 6.1. |
| UI library | React 19 with React Native for Web | Record 1, decision 8: one React Native interface for every platform. The web build renders React Native primitives through `react-native-web`. |
| Bundler and dev server | Vite 8 | One tool for the dev server and the production build; one entry chunk, hashed file names and a manifest, which is what WP-072 embeds. The dev server binds to loopback in the committed configuration (SEC-CLI-019). |
| Unit and component tests | Vitest with jsdom and Testing Library | Runs TypeScript and JSX with the bundler's own transform; queries by role and name, which is how a screen reader finds a control. |
| Coverage | `@vitest/coverage-v8`, thresholds 100, per file | Rule 5. |
| Mutation testing | StrykerJS with its Vitest runner and TypeScript checker | Rule 6. The one mutation tool for TypeScript with a Vitest runner. |
| Lint | ESLint 9 with typescript-eslint, eslint-plugin-react and eslint-plugin-react-hooks | The baseline names its rules: `react/no-danger`, `react/jsx-no-script-url`, `no-eval`, `no-implied-eval`, `no-restricted-properties` (SEC-CLI-001, SEC-API-045). ESLint 9 and not 10, because eslint-plugin-react's stated range stops at 9. |
| Formatting | Prettier, `--check` in the gate | The same job `cargo fmt` does. |
| Browser tests | Playwright in Chromium, Firefox and WebKit | The baseline's tests are written for it: the policy sweep, storage inspection, the network recorder (SEC-API-044, SEC-CLI-009, SEC-CLI-012). |
| Accessibility | `@axe-core/playwright` in the sweep, plus keyboard-only, reflow, text-size and target-size tests | CLI-136 names axe on the web build as the release gate. |
| WASM bindings | `wasm-bindgen` and its command-line tool, pinned to one version | Record 6 already lists the crate for `gunmetal-wasm`. |

What is deliberately absent: a router library (the client has a closed,
typed route table and the browser's History API; inbound links are parsed
by the core, SEC-CLI-025), a state library (React's own
`useSyncExternalStore` over small stores), a schema library (responses
are decoded by the core, SEC-CLI-021), an internationalisation library
(a typed catalogue and the browser's `Intl`), an icon package and a font
package (both are files in the repository, see CP-004 and CP-006), a list
virtualisation package (React Native for Web's own list is tried first,
against the DIS-100 budget), a UI component library, a CSS framework, any
analytics, error-reporting or telemetry package (SEC-CLI-027), and
`@vitejs/plugin-react` (it adds fast refresh only; the build does not need
it, unverified).

## Dependency policy and the proposed npm packages

The baseline's rules for JavaScript are R1 requirements, and this plan
carries their client half (the backend plan assigns the release
workflow's half to WP-136):

- Installs are frozen to the committed lockfile, resolve only from
  `registry.npmjs.org`, and run with lifecycle scripts and implicit
  `node-gyp` builds off, with an allow-list that starts empty
  (SEC-SUP-033, SEC-CLI-018). A canary install proves it: a local package
  whose `preinstall` script and `binding.gyp` each try to write a marker
  file, and neither marker exists afterwards.
- No version younger than seven days installs, and no version whose
  publishing trust dropped (SEC-SUP-034).
- Every direct dependency of every `package.json` has one sorted line in
  `supply-chain/js-direct-deps.toml` with its reason, and the xtask check
  from WP-124 fails otherwise (SEC-SUP-035).
- Registry signatures, and provenance where present, are verified for
  every installed package (SEC-SUP-036).
- Every package that ships in the bundle uses a licence on the project
  allow-list (SEC-SUP-029). Development tools are checked too, and listed
  when their licence is not on the list.
- A deny-list of analytics, advertising, crash-reporting and tracking
  packages fails the gate if any appears in the lockfile (SEC-CLI-027).
- **Adding a dependency is a request, not an edit.** It is written in the
  pull request with the baseline's checklist
  ([supply-chain-and-release.md](../security/supply-chain-and-release.md#5-dependency-policy)):
  the exact name, what it is for, what was considered instead, its
  licence, its maintainers and its transitive packages. A human confirms
  that the package exists and is the intended one before the integrator
  adds it (AGENTS.md, SEC-STD-035). An agent never adds a package to make
  a test pass.

The packages below are everything R1 is planned to need. Each name was
looked up on the npm registry on 2026-10-03 and exists under that exact
name with the licence shown; the version is the one the registry
answered with, or the newest in the line the plan pins. Whether these
versions work together is unverified until CP-001 and CP-002 install
them, and those two packages settle the exact pins.

**Shipped in the bundle (3):**

| Package | Seen | Licence | Reason |
|---|---|---|---|
| `react` | 19.3.0 | MIT | The UI library (record 1, decision 8). |
| `react-dom` | 19.3.0 | MIT | The web renderer; `react-native-web` needs it. |
| `react-native-web` | 0.21.3 | MIT | React Native primitives in a browser, so the screens are the ones the R2 native apps reuse. It brings eight transitive packages. |

**Development only (21):**

| Package | Seen | Licence | Reason |
|---|---|---|---|
| `typescript` | 6.0.3 | Apache-2.0 | The compiler and type checker. |
| `@types/react` | 19.3.0 | MIT | Types for React. |
| `@types/react-dom` | 19.3.0 | MIT | Types for the web renderer. |
| `@types/react-native-web` | 0.19.3 | MIT | Types for `react-native-web`. It trails the library's version, so its fit with 0.21 is unverified. |
| `@types/node` | 26.6.4 | MIT | Types for the configuration files and the test helpers that run in Node. |
| `vite` | 8.3.2 | MIT | The bundler and the dev server. |
| `vitest` | 5.0.3 | MIT | The test runner. |
| `@vitest/coverage-v8` | 5.0.3 | MIT | Coverage with thresholds. |
| `jsdom` | 30.1.1 | MIT | A DOM for component tests. |
| `@testing-library/react` | 16.3.3 | MIT | Renders components and queries them by role and name. |
| `@testing-library/user-event` | 14.6.7 | MIT | Real keyboard and pointer sequences in component tests. |
| `@stryker-mutator/core` | 10.0.0 | Apache-2.0 | Mutation testing. |
| `@stryker-mutator/vitest-runner` | 10.0.0 | Apache-2.0 | Runs the mutants with Vitest. |
| `@stryker-mutator/typescript-checker` | 10.0.0 | Apache-2.0 | Discards mutants that do not type-check. |
| `eslint` | 9.39.5 | MIT | The linter. |
| `typescript-eslint` | 8.71.0 | MIT | TypeScript parsing and rules for ESLint. |
| `eslint-plugin-react` | 7.37.5 | MIT | `react/no-danger` and `react/jsx-no-script-url`, which the baseline names. |
| `eslint-plugin-react-hooks` | 7.1.1 | MIT | The hook rules; a broken dependency list is a real bug class. |
| `prettier` | 3.9.9 | MIT | Formatting. |
| `@playwright/test` | 1.63.0 | Apache-2.0 | Browser tests in three engines. It downloads browser builds from its own host, outside npm; the version pin fixes which builds. |
| `@axe-core/playwright` | 4.13.0 | MPL-2.0 | The accessibility checks CLI-136 names. Development only, so it is in no shipped artefact; its licence still needs a line on the allow-list review. |

pnpm itself (12.9.1, MIT) is installed by exact version with a checksum,
not through the lockfile.

Two files enter the repository instead of packages, each with a REUSE
licence record and a SHA-256 in a manifest (SEC-SUP-030, SEC-SUP-032):
the Inter variable font as one WOFF2 file limited to weights 400 to 700
(OFL-1.1, allowed for font files only, SEC-SUP-029), and the subset of
Lucide's icon paths the R1 screens use (ISC, with the Feather-derived
ones under MIT), copied into the project's own source as design-language
section 10 requires.

Three needs have no package yet and no decision:

- **A QR code encoder** for invitations, browser pairing and the
  "help sign in" code (ACC-080, ACC-062, ACC-064). **Proposal:** the
  encoder belongs in the core, so the server's console code (WP-080) and
  the client draw the same matrix, and the client only draws squares.
  That is a request to the backend plan. If the owner prefers an npm
  package, it is a dependency request in CP-038.
- **A software bill of materials for the bundle** (SEC-CLI-017,
  SEC-SUP-044). The release workflow owns it (WP-136). The client's part
  is that the lockfile is the complete list.
- **A JavaScript licence check** against the same allow-list file cargo
  uses (SEC-SUP-029). **Proposal:** CP-001 writes it as a small check
  over `pnpm licenses list --json`, with no new package (the command's
  output format is unverified).

## The gate and CI: requests to the integrator

`scripts/gate.sh`, `.github/workflows/ci.yml`, `supply-chain/` and
`.github/CODEOWNERS` belong to the integrator and to code owners. Client
packages do not edit them. CP-001 and CP-002 file these requests in their
pull requests, and the integrator applies them between merges.

1. **`scripts/gate.sh`** gains a web block after the Rust steps, which
   runs only what `clients/package.json` defines, so the script stays the
   one definition of done:

   ```bash
   echo "==> web client"
   pnpm --dir clients install --frozen-lockfile
   pnpm --dir clients run gate
   ```

   `gate` runs, in order: the install canary and settings check, Prettier
   `--check`, ESLint with no warnings allowed, `tsc --noEmit`, Vitest with
   coverage at 100, StrykerJS with the report check, the production build
   with the bundle checks, and the Playwright sweeps. `GATE_OFFLINE=1`
   makes the install use the store only. `GATE_MUTANTS_DIFF=<ref>` limits
   StrykerJS to the files changed since the merge base, as it limits
   cargo-mutants today; every other step always covers the whole
   workspace.
2. **`ci.yml`** installs Node and pnpm by exact version with checksums,
   installs the Playwright browser builds for the pinned version, and
   runs on `client-*` branches as it does on `wave-*`. From CP-013 it
   also installs the `wasm32-unknown-unknown` target and the pinned
   `wasm-bindgen` command-line tool, and the root `Cargo.toml` gains
   `wasm-bindgen` under `[workspace.dependencies]` (record 6 already
   lists it). **Proposal:** the
   Rust and web halves of the gate run as two required jobs from the same
   script (`GATE_PART=rust` and `GATE_PART=web`), so neither waits for
   the other; locally the script runs both.
3. **`.github/CODEOWNERS`** gains `/clients/pnpm-lock.yaml`,
   `/clients/pnpm-workspace.yaml`, `/clients/package.json`, the lint, test
   and mutation configuration files and `/clients/tools/gate/`, because
   they define the gate (SEC-SUP-005).
4. **`supply-chain/js-direct-deps.toml`** (created by WP-124) gains one
   line per package in the tables above, with the reason given there.
5. **Dependabot** gains the npm ecosystem for `/clients` with a cooldown
   of at least seven days (SEC-SUP-028).
6. **CodeQL** already has to cover JavaScript and TypeScript
   (SEC-SUP-018); the request is to turn that language on once
   `clients/` exists.

## The contract: two ports

The screens never talk to a server or to WebAssembly directly. They talk
to two TypeScript interfaces, defined in `clients/packages/ports`
(CP-005):

- **`CorePort`** is everything record 1 puts in the shared Rust core and
  that the client calls on the device: reading the synced library,
  search, Home rows, the playback decision, queue operations and rebase,
  shuffle order, the gain decision, the player state machine, lyric
  positions, the link and URL filter, text normalisation, and decoding
  what the server sends (SEC-CLI-021). Its shape is the shape of the
  WASM facade.
- **`ServerPort`** is everything [api-needs.md](api-needs.md) makes a
  network call: server facts, sign-in ceremonies and sessions, sync
  snapshot and delta as opaque frames, the event channel, capability
  URLs for streams and artwork, queue operations, user-log writes,
  playlists, settings, and the administration capabilities. Its methods
  are named after the capability IDs (API-SYS, API-AUTH, API-SYNC,
  API-QUE and so on), so each one can be traced to the row that asked
  for it.

Each port has exactly two implementations that run in a page:

| Port | Stand-in | Real | Chosen by |
|---|---|---|---|
| `ServerPort` | `fake-server`: in memory, in the page | `http-server`: `fetch` and the event channel against `/api/v1` | The composition root: `apps/demo` or `apps/web` |
| `CorePort` | `fake-server`'s fixture core for the parts the core has not built yet (library reads, and until wave 2, search, Home rows and the decision) | `core-wasm`: the `gunmetal-wasm` build | The composition root, part by part |

Component tests use neither: they pass a scripted double that returns
what the test wrote down and holds no rule.

### Where the protocol types come from

The client does not define the protocol. These Rust packages do, and the
TypeScript types in `ports` are checked against them as each lands:

| Types | Defined by | Server wave | State on 2026-10-03 |
|---|---|---|---|
| Public IDs, the problem catalogue | WP-006 | 0 | On `main` |
| Text normalisation, typed values, the link filter | WP-005 | 0 | On `main` |
| Lyrics: the timed-line model | WP-021 | 1 | In progress |
| The queue document and its operations | WP-025 | 1 | In progress |
| Shuffle modes | WP-026 | 1 | In progress |
| The gain decision | WP-028 | 1 | In progress |
| The player state and its events | WP-030 | 1 | In progress |
| User events (plays, loves) | WP-034 | 1 | In progress |
| Wire frames and version negotiation | WP-039 | 1 | In progress |
| Catalogue records: track, album, artist | WP-040 | 1 | In progress |
| Search results | WP-054 | 2 | Not started |
| The playback decision and track details | WP-055 | 2 | Not started |
| Home rows | WP-059 | 2 | Not started |
| The route table and `openapi.json` | WP-118 | 2 | Not started |
| Stream capability URLs | WP-082 | 3 | Not started |
| The event channel | WP-083 | 3 | Not started |
| Sync snapshot and delta | WP-084 | 3 | Not started |
| The queue service | WP-085 | 3 | Not started |
| The WASM facade | WP-088 | 3 | Not started |
| Server facts and negotiation | WP-089 | 3 | Not started |

### What keeps the fake and the real server in step

Four checks, each arriving as soon as the Rust side it needs exists:

1. **From C0: the problem catalogue.** A test in `ports` reads
   `crates/gunmetal-core/src/problem.rs` and fails when its list of
   problem codes differs from the list the client maps to sentences.
2. **From CP-013 (server wave 1): generated types.** `wasm-bindgen`
   writes TypeScript declarations for the facade. `core-wasm` must
   satisfy `CorePort` at compile time, so a field the core renames or
   retypes fails `tsc` in the gate.
3. **From CP-053 (server wave 2): the route table.** A test reads the
   committed `crates/gunmetal-server/openapi.json` and fails when a call
   the HTTP adapter makes names a route, method or field that is not in
   it. The fake is checked by the same table: every `ServerPort` method
   maps to one route.
4. **From CP-055 (server wave 3): the contract run.** This is the test
   that fails when the fake and the real server disagree. One conformance
   suite, with literal expectations written from WP-119's manifest of a
   small synthetic library, runs twice: against `fake-server` replaying
   recorded frames, and against a real `gunmetal` process serving that
   same library. The same job re-records the frames from the real server
   and fails when the committed recording's hash differs, so the fake
   can never serve a shape the server stopped producing.

Until check 4 exists, nothing but review catches a fake that behaves
differently from the server the backend plan describes. That is the
accepted cost of starting before wave 3, and it is why the fake is kept
small enough to read in one sitting.

## The fake server

The fake is the smallest honest thing: fixtures, and an in-memory
implementation of `ServerPort`, both in TypeScript, both in the page. It
is not a process, it has no socket and it parses no media.

**What it serves, in two stages.**

- **Stage A, from C0.** A hand-written demo library of made-up music
  (CP-011): about eight albums chosen to exercise the R1 rows, including
  a multi-disc album with disc titles, a compilation with Various
  Artists, two artists with the same name, a track with synced lyrics
  and one with plain lyrics, a track marked as one this browser cannot
  play, a track flagged as damaged, and one album whose every text field
  holds the hostile-metadata corpus. The records are JSON in the shape of
  the `ports` types. The audio is short generated tones in WAV, and the
  covers are small generated PNG gradients; both come from a checked-in
  script and are listed in a SHA-256 manifest (SEC-SUP-032). Which music
  the demo plays is a [question for the owner](#product-questions-for-the-owner).
- **Stage B, from CP-055.** Recorded sync frames and responses from the
  real server over WP-119's synthetic library, replayed for tests that
  do not need a server process. The hand-written library stays only as
  the demo's data.

**What it does.** It holds the queue document, loves, plays, playlists
and settings in memory, hands out same-origin URLs for the fixture audio
and artwork with an expiry it honours under an injected clock, answers
sign-in ceremonies without verifying them, and can be told to fail: every
problem code, a revoked session, a stream limit, an unreachable server.
Those switches are how the R1 states in player.md's table become
fixtures.

**What it never does.** It holds no rule. It orders queue operations by
calling `CorePort`, exactly as the real server calls the core. It never
sorts, matches, searches or decides playback; where the core cannot
answer yet, the fixture carries the answer, written by hand (the album
list already in order, the Home rows already chosen, each track's
decision already made). It verifies no credential and enforces no
permission, so no test of the fake proves a security property of the
server.

**How it stays out of the product.** Only `apps/demo` imports it. The
production build fails if the bundle holds any module of `fake-server` or
`fixtures` (CP-056), and the demo shows a permanent "Demo data" label.

## Core logic before the WASM facade

WP-088, the facade, is in server wave 3, because the parts of the core it
wraps last (search, Home rows, the decision) are wave 2. The client needs
the queue, shuffle, gain and player rules in C1. The plan's answer is
that nothing stands in for the core except the core.

- **`CorePort` is written first** (CP-005), as the list of what the
  client needs from the facade. It is the client's side of WP-088.
- **The facade is built in slices, as each core module merges.** CP-013
  creates `crates/gunmetal-wasm` with the wave 1 modules: the queue
  (WP-025), shuffle (WP-026), gain (WP-028), the player state (WP-030),
  lyrics (WP-021), the artwork palette (WP-037) and the link filter and
  text normalisation already on `main` (WP-005). CP-023 adds the wave 2
  modules: search (WP-054), the decision and track details (WP-055) and
  Home rows (WP-059). Each
  export is a direct call into the core with conversion only, which is
  exactly what WP-088 specifies, so this is WP-088's own work done
  earlier, in files WP-088 would otherwise write. WP-088 keeps what
  needs wave 3: applying sync frames to the in-memory library, the
  library reads over it, wire decoding of responses, the `wasm32` build
  job and the size check. This changes who creates the crate, so it is a
  [request to the backend plan](#requests-to-the-backend-plan).
- **Until a slice exists, its part of `CorePort` has no running
  implementation.** Screens that need it are built and proved with
  scripted doubles in component tests, which hold no rule, and they are
  not wired into the demo until the slice merges. No TypeScript
  implementation of a core rule is written, so nothing is thrown away.
- **Reading the library is the one part the fixture answers.** Until
  WP-088 applies real sync frames, album, artist, track and playlist
  reads are lookups in the fixture's JSON, and every ordered list comes
  from the fixture already ordered. A lookup by ID is not a rule. When
  WP-088 lands, CP-054 replaces the lookups with the facade's reads and
  deletes the fixture core.

The risk is schedule, not rework: C1's first clickable milestone needs
CP-013, and CP-013 needs WP-021, WP-025, WP-026, WP-028, WP-030 and
WP-037 merged into `wave-1`. All six were in progress on 2026-10-03. If
one slips, the
milestone waits for it; the plan does not fill the gap with TypeScript.

## Playback on the web in R1

R1 plays music in the browser with the browser's own decoders. libmpv is
for the native apps in R2 (record 1, decision 1). There is no transcoder
in R1 (D-09), so a file the browser cannot decode is never converted; it
is dimmed with the reason (MUS-229).

**Two delivery paths, chosen by the core.** The client reports what this
browser can do, by probing `canPlayType` and `MediaSource.isTypeSupported`
(and `ManagedMediaSource` where it exists), and the core's decision
engine (WP-055) answers for each track: play directly, play through the
packager, or cannot play here, with a structured reason the quality
badge and track details show (MUS-099, MUS-236).

| Path | How | What it gives | Available |
|---|---|---|---|
| Direct | An `<audio>` element given the capability URL of the original file; a second element loads the next track early (MUS-070) | Plays every format the browser decodes natively. MP3, AAC in MP4, FLAC, Opus and Vorbis in Ogg, and PCM WAV are the R1 formats (MUS-032); which of them each browser decodes is unverified per browser, and ALAC is expected only where the engine supports it. Joins between tracks are close but not sample-accurate, and seeking is as exact as the browser makes it. | C1, on the fake; the real route with WP-082 (wave 3) |
| Packaged | Media Source Extensions fed fragmented MP4 that the server's audio packager made by copying frames without re-encoding (MUS-230, record 4); append windows trim encoder delay and padding from the scan's values | Sample-accurate gapless joins (MUS-067, MUS-069) and exact seeking (MUS-071), and formats a browser accepts in fragmented MP4 but not as a file. Which containers each browser's Media Source accepts, including Safari's `ManagedMediaSource`, is unverified. | CP-034 builds it against segments the Rust packager wrote as fixtures (WP-056, wave 2); the real route is WP-105 (wave 4) |

So the direct path is what the first clickable build plays, and it is
complete on its own for a single track. What waits for the packager is
gapless playback proved to the sample, exact seeking, and any format
that only plays through Media Source.

**Levelling.** The audio graph is one Web Audio gain node per element.
The core's gain decision (WP-028) gives the gain, its source and the
clamp; the client applies the number and never computes one (MUS-084 to
MUS-089). Media is same-origin, so the graph can read it.

**The player model.** The engine turns the element's events (`playing`,
`waiting`, `stalled`, `ended`, `error`) into the core's player events and
shows the state the core's machine (WP-030) returns. The bar, the full
player and the browser's media controls all read that one value
(player.md, "One playback model"). The queue is the core's document
(WP-025): the client applies each operation optimistically through
`CorePort`, sends it to the server, and rebases with the core's own
`rebase` when the server rejects a stale version. Shuffle order comes
from the core's seeded generator (WP-026), so every device sees the same
order. When another of the profile's sessions presses Play, this one
pauses at its point on its next sync: the last Play wins (MUS-122).

**Stream URLs.** The client asks for a capability URL when an item is
about to play and never stores one: not in the device copy, Cache
Storage, the media session or a log (SEC-API-029). When a pause outlives
a URL, the engine asks for a fresh one on Play and resumes at the same
position with nothing shown (SEC-API-027). The early fetch of the next
track belongs to the same playback, so it does not count as a second
stream (SEC-TM-068).

**Phones.** R1 on a phone is the browser at phone width (CLI-149). Lock
screen controls come from the browser's Media Session (CLI-070,
MUS-073). Background playback on iPhone is best effort (unverified), and
the app says so.

## Working in parallel

The backend plan's rules apply unchanged
([Working in parallel](work-packages.md#working-in-parallel-shared-files-and-merging)):
every path has one owner, a conflict outside a registry file means two
packages own one path, and a change to an interface another merged
package uses is its own small package in the original owner's files.

**Branches.** **Proposal:** each client wave has an integration branch,
`client-0` to `client-5`, and a package branches from it as `cp/cp-NNN`,
exactly as D-01 sets out for server waves. A client wave starts on top of
the previous client wave's branch, and on top of the server wave branch
it needs code from (`client-1` needs `wave-1` for CP-013). One integrator
agent per client wave merges packages once the gate passes. The owner
reviews and merges each client wave's one pull request into `main`.
Agents never merge into `main`.

**Registry files.** These hold one sorted line per entry and nothing
else. Any package may add its line; a conflict is resolved by keeping
every line and re-sorting.

| File | Owner | What other packages may do |
|---|---|---|
| `clients/package.json`, `clients/pnpm-workspace.yaml`, the lint, test and mutation configuration | CP-001 and CP-002, then the integrator | Nothing. A new dependency or a rule change is a request. |
| `clients/pnpm-lock.yaml` | Nobody | On a conflict, take either side and run `pnpm install --lockfile-only`, never a plain upgrade. |
| A package's own `package.json` | The package that creates it | Add one `workspace:*` line for another workspace member, sorted. Never a registry dependency. |
| `supply-chain/js-direct-deps.toml` | WP-124 | The integrator adds the approved line. |
| Every `index.ts` | Nobody; these are registries | Export lines only, never code. |
| `clients/packages/ui/src/routes.ts` | CP-009 | One line per route: its path pattern, its surface and whether it needs a session or an admin session. |
| `clients/packages/ui/src/actions.ts` | CP-024 | One line per action in player.md's one action list. |
| `clients/packages/ui/src/messages/en/<surface>.ts` | The surface's package | Not shared: each surface owns its own message file. `messages/en/index.ts` is a registry. |
| `clients/e2e/screens.ts` | CP-003 | One line per screen or state the sweeps must visit. |
| `clients/packages/ports/src/` | CP-005 | A new method is the interface-change rule: its own small package, agreed with the backend package that serves it. |
| `clients/apps/web/src/compose.ts`, `clients/apps/demo/src/compose.ts` | CP-003 and CP-012 | One line to register a store or a surface module. |
| `crates/gunmetal-wasm/src/lib.rs` | Nobody; a registry | `mod` lines and re-exports, as for every `lib.rs`. |

## Waves at a glance

| Wave | Packages | Count | What it produces |
|---|---|---:|---|
| C0 | CP-001 to CP-012 | 12 | The workspace and its dependency rules, the TypeScript gate, the app skeleton proved under the production policy, tokens and themes, the two ports, the kit, overlays, messages, the shell and routing, the accessibility and layout harness, the demo library, the fake server and the demo build |
| C1 | CP-013 to CP-022 | 10 | The first slice of the core in the browser; stores; lists and grids; Library, album and artist pages; the audio engine; the playback controller; the now-playing bar; the queue |
| C2 | CP-023 to CP-034 | 12 | The second slice of the core; the context menu; the full-screen player, lyrics, track details and the quality badge; media controls; Home; search and genre pages; playlists; history and private listening; the status layer and every player state; gapless playback through Media Source |
| C3 | CP-035 to CP-043 | 9 | Sign-in with passkeys, the personal-or-shared question and the wipe; first-run setup; invitations; approving a browser; step-up; settings; account, recovery and each person's own security pages |
| C4 | CP-044 to CP-052 | 9 | The admin frame and dashboard; libraries; health and trash; users; security and server settings; network; backups and updates; activity, alerts and diagnostics; the security log |
| C5 | CP-053 to CP-059 | 7 | The HTTP adapter, the sync client and the device copy, the contract run, the production bundle handed to the server, real streaming, the R1 flows end to end in a browser, the browser list and the release accessibility script |

59 packages build the R1 web player. C0 has one fixed order at its start:
CP-001, then CP-002, then everything else. C2, C3 and C4 do not depend on
each other and can run at the same time once C1 has merged; the order
among them is a [question for the owner](#product-questions-for-the-owner).
C5's packages do not wait for C4: each starts as soon as the server
packages it names have merged.

## Client waves, server waves and what the owner can test

"Leaves the fake" means the wave's screens run against a real Gunmetal
server with the owner's own music, with no fake code in the page.

| Client wave | Can be built and clicked on the fake once | Leaves the fake when | What the owner can test at the end of the wave, on the demo |
|---|---|---|---|
| C0 | Server wave 0 is on `main` (it is). WP-124 (wave 1) supplies the dependency list file. | Not applicable: nothing here talks to a server. | Open the demo in a browser. See the Gunmetal frame: the sidebar with Home, Search and Library, the dark, light, black and high-contrast themes, and the phone layout when the window is narrow. Move through it with the keyboard alone. Nothing plays yet. |
| C1 | Server wave 1's lyrics, queue, shuffle, gain, player-state and palette packages have merged (WP-021, WP-025, WP-026, WP-028, WP-030, WP-037), for CP-013. | Server wave 3: sync (WP-084), the facade (WP-088), streams (WP-082), the queue service (WP-085). Artwork needs wave 4 (WP-103). | **The first clickable player.** Browse the made-up library by artist, album and song. Open an album. Press Play and hear it. Use the bar at the bottom: pause, skip, see what is playing. Open the queue, reorder it, remove a track, turn on shuffle and repeat. |
| C2 | Server wave 2's search, decision and Home packages (WP-054, WP-055, WP-059) have merged, for CP-023; the packager (WP-056) for CP-034's fixtures. | Server wave 3: listening activity (WP-086), playlists (WP-093), the event channel (WP-083). Wave 4: stream limits (WP-104), the packaging route (WP-105), history deletion (WP-133). | Everything a listener does. Search as you type. A Home page with what you played. Right-click anything for Play next and Add to queue. The full-screen player, lyrics that follow the song, what format is playing and why. Make a playlist. See your history and remove a play. Turn on a private session. Use the keyboard's media keys. Albums play without a gap between tracks. |
| C3 | Nothing more from the server. | Server wave 3: setup (WP-080), passkeys (WP-081), browser pairing and sign-out (WP-120), accounts and devices (WP-087), invitations (WP-094), recovery codes (WP-063). Wave 4: recovery and step-up (WP-106), data export (WP-108), account deletion (WP-133). | Walk through first-run setup with a made-up setup code. Sign in with a passkey, answer "Is this your own device?", and sign out and see that nothing is left behind. Open an invitation link. Approve another browser. Change the theme and the sound settings. See your devices and remove one. |
| C4 | Nothing more from the server. | Server wave 3: library administration (WP-099), activity (WP-100), alerts (WP-097), backups (WP-090), network settings (WP-073, WP-132), updates (WP-074), users (WP-094). Wave 4: HTTPS by ACME (WP-101), the scan (WP-102). Wave 5: health and trash (WP-110, WP-111). Wave 6: the doctor and security summary (WP-116). | The admin screens with made-up data. Add a music folder and watch a pretend scan. Read the health report. Invite someone and choose their libraries. Look at backups, updates, alerts and the security log. |
| C5 | Not applicable: this wave is the real server. | Each package as its server packages merge: the route table in wave 2, the routes in waves 3 and 4, the full flow tests beside WP-117 in wave 6. | **The real thing.** Start the Gunmetal server, claim it, add your own music folder, and use every screen above with your own library, served by the server itself. |

## The first clickable milestone

The milestone the owner asked for is: browse a library, open an album,
play a track, with the player bar and the queue. It is the end of wave C1
without the artist page, and it needs exactly these 21 packages:

- **All of C0:** CP-001 (workspace), CP-002 (gate), CP-003 (skeleton
  under the policy), CP-004 (tokens), CP-005 (ports), CP-006 (kit),
  CP-007 (overlays), CP-008 (messages), CP-009 (shell and routing),
  CP-010 (accessibility harness), CP-011 (demo library), CP-012 (fake
  server and demo build).
- **From C1:** CP-013 (the core in the browser, first slice), CP-014
  (stores), CP-015 (lists and grids), CP-016 (Library), CP-017 (album
  page), CP-019 (audio engine), CP-020 (playback controller), CP-021
  (now-playing bar), CP-022 (queue).

From the server it needs only the six wave 1 pure-logic packages CP-013
wraps, merged into `wave-1`: WP-021, WP-025, WP-026, WP-028, WP-030 and
WP-037. It needs no server process, no database and no network.

An earlier, smaller milestone falls out of C0 alone: a real screen with
the frame, the themes and the phone layout, and nothing to play.

## Wave C0: foundations

In the **Owns** fields, a path that starts with a package name is under
`clients/packages/`: `ui/src/kit/` means `clients/packages/ui/src/kit/`.
Every surface package also owns its own message file,
`ui/src/messages/en/<surface>.ts`, and adds its lines to the registries;
the fields do not repeat that.

CP-001 merges first and CP-002 second. Everything else in C0 starts when
CP-002 has merged.

### CP-001 Workspace, package manager and dependency policy

- **Wave** C0 · **Size** S · **Depends on** WP-124 (wave 1) for
  `supply-chain/js-direct-deps.toml` and its check. If WP-124 has not
  merged, the lines ride in this package's pull request and the
  integrator adds them when the file exists.
- **Owns** `clients/package.json`, `clients/pnpm-workspace.yaml`,
  `clients/pnpm-lock.yaml`, `clients/tsconfig.base.json`,
  `clients/tools/gate/install/`, `crates/xtask/src/js_install.rs` (with
  its one dispatch line in the xtask registry).
- **Serves** CLI-001 (the web client exists as a workspace).
- **Security.** SEC-SUP-033, SEC-SUP-034, SEC-SUP-035, SEC-SUP-036,
  SEC-CLI-018, SEC-CLI-027, and the JavaScript half of SEC-SUP-029.
- **Builds.** The pnpm workspace with the baseline's settings: a minimum
  release age of seven days, no drop in publishing trust, no exotic
  sources, and an empty build allow-list. The xtask check that lints
  those settings and the lockfile's sources, which WP-136 later calls
  for the release workflow instead of writing a second one. The install
  canary. The signature check. The licence check against the project
  allow-list. The deny-list of tracking packages. The gate requests 1 to
  5 in [The gate and CI](#the-gate-and-ci-requests-to-the-integrator).
- **Tests.** The xtask check fails on a settings file with each rule
  removed or loosened in turn, on a lockfile fixture holding a git
  dependency, a tarball URL and a second registry, and on a non-empty
  build allow-list; each failure names the rule. After the canary
  install neither marker file exists. The licence check fails on a
  fixture listing a banned licence and on a missing one. The deny-list
  check fails on a fixture lockfile naming a listed package.
- **Done when.** A fresh clone installs with the frozen lockfile and no
  script runs, and the gate fails when any rule above is loosened.

### CP-002 The TypeScript quality gate

- **Wave** C0 · **Size** M · **Depends on** CP-001.
- **Owns** `clients/eslint.config.js`, `clients/prettier.config.js`,
  `clients/vitest.config.ts`, `clients/stryker.config.json`,
  `clients/tools/lint/` (the project's own lint rules),
  `clients/tools/gate/report/` (the mutation report check),
  `clients/packages/canary/`.
- **Serves** The testing rules in CONTRIBUTING.md. No feature row of its
  own.
- **Security.** The lint halves of SEC-CLI-001, SEC-API-045, SEC-MED-057,
  SEC-HIS-027, SEC-TM-036 and SEC-STD-012, and SEC-API-050.
- **Builds.** The `gate` script and every step in it but the build and
  the browser suites. Lint rules as errors: the baseline's named rules;
  bans on `innerHTML`, `outerHTML`, `insertAdjacentHTML`,
  `document.write`, `eval`, `new Function`, string timers and
  `javascript:` URLs; a `message` listener without an origin check;
  spread, `Object.assign` and deep merges of untrusted objects; every
  escape hatch in the ground rules table; a colour literal outside the
  tokens package; a literal string as interface text; physical layout
  properties where a logical one exists. The mutation report check. A
  small canary package (one pure function, one component) so the gate
  has something to run from the first day.
- **Tests.** Each check is checked to fail. One fixture file per banned
  construct produces exactly its rule's error. The report check, fed
  literal Stryker reports holding one survivor, one uncovered mutant,
  one timeout and one run-time error, fails on each and passes on a
  report of killed mutants only. A fixture project with one uncovered
  branch fails the coverage step; with one unformatted file, the format
  step; with one type error, the type step.
- **Done when.** `pnpm --dir clients run gate` passes on the canary and
  fails for each planted fault.

### CP-003 Web app skeleton under the production policy

- **Wave** C0 · **Size** M · **Depends on** CP-002.
- **Owns** `clients/apps/web/` (the page, the entry, `compose.ts`, the
  Vite configuration, the `gunmetal-loader` policy, the
  unsupported-browser page), `clients/playwright.config.ts`,
  `clients/e2e/support/`, `clients/e2e/policy/`, `clients/e2e/screens.ts`.
- **Serves** CLI-001, CLI-002 (the unsupported-browser notice), CLI-150.
- **Security.** SEC-API-044 (the client's half: the sweep), SEC-API-045
  (the policy function), SEC-API-049, SEC-API-052, SEC-CLI-012,
  SEC-CLI-019, SEC-PRV-018, SEC-SUP-037.
- **Builds.** The smallest real page: React Native for Web rendering one
  view and one line of text with a token colour and one value that
  changes at run time, built for production and served by a test server
  that sends the exact header of SEC-API-044 and the headers of
  SEC-CLI-004. This is the spike the register asks for under D-74, and
  it settles how the client is styled before any screen is written. The
  order of outcomes is the register's: the policy as written; if React
  Native for Web's style element is refused, a hash for that one empty
  element; only then, and only with the owner's answer, `'unsafe-inline'`
  for styles, never for scripts. The one Trusted Types policy, which
  accepts only same-origin script URLs under the bundle's asset path.
  The start-up check for WebAssembly and a secure context, with a static
  page when either is missing. The browser harness every later package
  uses: the violation listener, the network recorder that fails on any
  origin but the server under test, and the sweep that visits every line
  of `screens.ts` in three engines. The dev server bound to loopback.
- **Tests.** In Chromium, Firefox and WebKit: the page renders with no
  `securitypolicyviolation` event; creating any other Trusted Types
  policy throws; a planted request to another origin fails the sweep; a
  planted inline style in markup fails it. Property tests of the policy
  function: only same-origin URLs under the asset prefix pass. With
  WebAssembly disabled, and on a cleartext non-loopback origin, the
  static page is shown and nothing else runs. The committed dev
  configuration names the loopback address literally. A scan of a
  fixture bundle finds a planted absolute URL.
- **Done when.** The styling route is proved under the policy in all
  three engines, or the owner has answered D-74's fallback.

### CP-004 Design tokens, themes and typeface

- **Wave** C0 · **Size** M · **Depends on** CP-002.
- **Owns** `tokens/`.
- **Serves** CLI-139, CLI-140, CLI-141; design-language sections 4, 6,
  7, 9 and requirements A1, A2, A12 and A18.
- **Security.** SEC-CLI-012 (the font is a file in the bundle),
  SEC-PRV-018 (no stylesheet names another origin or a `data:` source).
- **Builds.** Every token in design-language as typed constants: colour
  for the Dark, Light, OLED black and High contrast themes, spacing, the
  type scale, shape, elevation, motion and the four width classes. The
  static stylesheets the policy requires: custom properties per theme,
  System following the operating system, high contrast when the system
  asks for more contrast, Windows forced colours, and the reduced motion
  set. Inter as one WOFF2 file with its licence record. A small
  stylesheet with no script for the pages the server renders itself
  (SUR-080, SUR-109), which WP-095 and WP-132 embed.
- **Not built here.** Artwork tints: the palette is computed at scan time
  (WP-037) and applied by the pages that use it (CP-017, CP-025).
- **Tests.** Every token in every theme equals the literal value in
  design-language's table. The contrast pairs in its "Measured contrast"
  table are recomputed with a WCAG contrast function written in the test
  and compared with the literal ratios; text pairs meet 4.5:1, primary
  and secondary text 7:1. The generated stylesheet for one theme equals
  a literal expected text. No stylesheet holds a URL other than the font
  file's.
- **Done when.** A theme is a different token file and nothing else.

### CP-005 Ports and conformance suites

- **Wave** C0 · **Size** M · **Depends on** CP-002.
- **Owns** `ports/`.
- **Serves** CLI-022 (the read interface every screen uses); player.md,
  "What the player needs from the server and the core".
- **Security.** SEC-STD-012 (the model's half), SEC-CLI-021 (every port
  method returns a value or a typed problem, never throws).
- **Builds.** `CorePort` and `ServerPort`, as described in
  [The contract](#the-contract-two-ports). IDs typed by kind. The
  problem type and the map from each problem code to a message key. Data
  keyed by untrusted strings held in `Map`, never merged into objects.
  The conformance suites, exported as functions that take an
  implementation. The hostile-metadata corpus as literal payloads
  (script elements, event-handler attributes, `javascript:` URLs,
  bidirectional overrides, very long strings, lone surrogates). The
  written list of what the client needs from the facade, which is this
  plan's request to WP-088.
- **Tests.** The client's problem codes equal the list in
  `crates/gunmetal-core/src/problem.rs`. Each conformance suite fails
  against a double that gives one wrong answer. Payloads whose keys are
  `__proto__`, `constructor` and `prototype` pass through the model and
  leave `Object.prototype` unchanged.
- **Done when.** A screen can be written against the ports with no
  implementation behind them.

### CP-006 UI kit: text, controls and icons

- **Wave** C0 · **Size** M · **Depends on** CP-003, CP-004, CP-005.
- **Owns** `ui/package.json`, `ui/src/kit/`, `ui/src/icons/`.
- **Serves** CLI-135, CLI-138, CLI-142, CLI-159; design-language
  sections 6, 8 and 10 and requirements A4, A6 and A10.
- **Security.** SEC-CLI-001, SEC-CLI-002, SEC-CLI-028, SEC-API-046,
  SEC-API-047, SEC-MED-057, SEC-MED-058, SEC-HIS-027, SEC-TM-036,
  SEC-STD-015.
- **Builds.** `Text` for untrusted strings (a text node in a
  bidirectional isolate, with truncation), headings, button, icon button
  (an accessible name is required by its type), internal link, the
  outside link that shows as a link only when the core's filter accepts
  it and opens only after a sheet naming the destination host, text
  field, the code field for setup, pairing and recovery codes, switch,
  slider, badge, progress ring, the focus ring, and the R1 icon set as
  SVG elements with no style inside them.
- **Tests.** Every corpus payload rendered through `Text`, a row and a
  heading appears as its literal text, and no element, attribute or
  handler is created. Link cases (`javascript:`, `data:`, `file:`, mixed
  case, embedded whitespace) are shown as plain text; an accepted link
  carries `rel="noopener noreferrer"` and opens nothing before the
  sheet. The code field's `type`, `autocomplete`, `autocorrect` and
  `spellcheck` attributes equal the literal expected set, and it accepts
  paste. Each control exposes its role, name and state. Targets are 44
  by 44 on touch and 32 by 32 with a pointer.
- **Done when.** No later package needs a raw primitive for text or a
  control.

### CP-007 Overlays, focus and notices

- **Wave** C0 · **Size** M · **Depends on** CP-006.
- **Owns** `ui/src/overlays/`.
- **Serves** CLI-138, CLI-142; design-language section 11 and
  requirements A5, A13, A14 and A21.
- **Security.** SEC-CLI-001 (text inside notices and prompts).
- **Builds.** Dialog, sheet (from the bottom at the compact class, from
  the side at medium), popover, menu, tooltip, toast, banner, and the
  announcer for status messages. Focus moves in, is held by modals only,
  starts on the safe choice in a security prompt, and returns where it
  was.
- **Tests.** Escape closes each overlay and focus returns to the opener.
  Tab never leaves a modal and never gets stuck in anything else. A
  status message is announced without moving focus. A tooltip can be
  hovered, dismissed and stays until dismissed. A confirmation opens
  with focus on Cancel.
- **Done when.** Every R1 sheet, menu and prompt can be built from these
  parts.

### CP-008 Messages and the pseudo-locale

- **Wave** C0 · **Size** S · **Depends on** CP-006.
- **Owns** `ui/src/messages/catalogue.ts`, `ui/src/messages/format.ts`,
  `ui/src/messages/pseudo.ts`, `ui/src/messages/en/index.ts`.
- **Serves** Design-language requirement A20; CLI-139. Translations
  themselves are R1.2 (CLI-146).
- **Security.** SEC-CLI-001 (a parameter is always text).
- **Builds.** A typed catalogue: each key has a template with named
  parameters, and a missing or misspelt parameter fails type checking.
  Plurals, numbers, dates and durations through the browser's `Intl`.
  The pseudo-locale build, which lengthens and accents every message so
  hard-coded and clipped strings show.
- **Tests.** A message with each plural form gives the literal expected
  text. A corpus payload passed as a parameter appears literally. The
  pseudo-locale of a literal input equals a literal output.
- **Done when.** A screen cannot show interface text that is not a
  message.

### CP-009 App shell, routing and width classes (SUR-001)

- **Wave** C0 · **Size** M · **Depends on** CP-007, CP-008.
- **Owns** `ui/src/shell/`, `ui/src/router/`, `ui/src/routes.ts`.
- **Serves** SUR-001; CLI-001, CLI-031, CLI-060, CLI-149, DIS-112.
- **Security.** SEC-CLI-013 (the router takes a secret out of the
  address bar and the history before any request is made).
- **Builds.** The closed, typed route table and the History glue: Back
  and Forward return to the exact scroll position. The frame at each
  width class: sidebar, content, right pane and bar at wide; the pane
  over the content at expanded; a rail and a side sheet at medium; a
  bottom tab bar with the bar above it at compact. Home, Search and
  Library in fixed places, a slot for the bar, the right pane, the
  status indicator and the account entry. Panes resize and collapse by
  pointer and by keyboard. Sign-in and setup routes show nothing
  persistent.
- **Tests.** At 360, 800, 1200 and 1600 pixels wide the landmarks
  present, and their order, equal a literal list. Back restores a
  literal scroll offset. An unknown path shows "not found" and makes no
  port call. With a secret in the fragment, the address and every
  history entry hold no secret before the first port call is recorded.
- **Done when.** The owner can move between three empty destinations at
  every width, by pointer and by keyboard.

### CP-010 Accessibility and layout-contract harness

- **Wave** C0 · **Size** M · **Depends on** CP-003, CP-009.
- **Owns** `clients/e2e/a11y/`, `clients/e2e/layout/`.
- **Serves** SUR-000; CLI-031, CLI-135, CLI-136, CLI-138, CLI-139,
  CLI-140, CLI-142, MUS-113; design-language requirements A3 to A6, A8
  to A12 and A18.
- **Security.** None of its own.
- **Builds.** Sweeps over `screens.ts`: axe with no violation; a
  keyboard-only walk that reaches every control and is never trapped;
  reflow at 320 pixels with no sideways scrolling; text at 200%; reduced
  motion forced on; forced colours with the focus ring still visible;
  target sizes. The layout-contract helper, which compares the measured
  position and order of named controls with literal values.
- **Tests.** Each sweep fails on a planted page: an unnamed button, a
  focus trap, an overflowing row, a 20-pixel target, an animation that
  ignores reduced motion. The layout helper fails when a control moves
  by one pixel.
- **Done when.** Adding a screen to the registry is all a package does
  to be held to CLI-136.

### CP-011 Demo library and generated media

- **Wave** C0 · **Size** M · **Depends on** CP-005.
- **Owns** `fixtures/`.
- **Serves** The owner's choice of 2026-10-03 (something to click);
  MUS-032 (a fixture of each R1 state of a track).
- **Security.** SEC-SUP-030, SEC-SUP-032, SEC-API-046 (the hostile
  album).
- **Builds.** The stage A library described in
  [The fake server](#the-fake-server), as records in the shape of the
  `ports` types, with the lists the core would compute written out by
  hand. A checked-in script that writes the tone files and cover images,
  and the SHA-256 manifest. Licence records for every file.
- **Tests.** Running the script twice gives byte-identical files that
  match the manifest. A tone file's header fields equal literal values.
  Every ID is unique and every reference resolves. The hostile album
  holds a corpus payload in every text field.
- **Done when.** The demo has music to list and to play.

### CP-012 Fake server, fixture core and the demo build

- **Wave** C0 · **Size** M · **Depends on** CP-003, CP-005, CP-011.
- **Owns** `fake-server/`, `clients/apps/demo/`.
- **Serves** The owner's choice of 2026-10-03.
- **Security.** SEC-CLI-019 (the demo server binds to loopback). It
  verifies nothing else: it holds no control, and no test of it proves
  the server.
- **Builds.** The in-memory `ServerPort`, the fixture core for library
  reads, the failure switches, the injected clock, and the demo
  composition root with its permanent "Demo data" label and a
  `pnpm --dir clients demo` script.
- **Tests.** The `ServerPort` conformance suite passes. Each switch
  produces its exact problem. A stream URL is refused after its expiry
  under the injected clock. A queue operation is passed to the injected
  `CorePort` and the fake stores what it returns, with no change of its
  own. The fixture core returns each list in the fixture's order.
- **Done when.** The demo opens to the shell from CP-009 with the demo
  library behind it.

## Wave C1: the first clickable player

### CP-013 The core in the browser, first slice

- **Wave** C1 · **Size** M · **Depends on** CP-005; WP-005 (wave 0),
  WP-021, WP-025, WP-026, WP-028, WP-030, WP-037 (wave 1).
- **Owns** `crates/gunmetal-wasm/Cargo.toml`,
  `crates/gunmetal-wasm/src/` (one file per module: `queue.rs`,
  `shuffle.rs`, `gain.rs`, `player.rs`, `lyrics.rs`, `links.rs`,
  `text.rs`, `palette.rs`), `core-wasm/`.
- **Serves** Record 1, decision 2; MUS-077, MUS-084, MUS-085, MUS-087 to
  MUS-089, MUS-116 to MUS-119, MUS-122, MUS-126, MUS-154, MUS-155.
- **Security.** SEC-CLI-021 (a value the core rejects comes back as a
  typed problem), SEC-CLI-002 and SEC-API-047 (the core's link filter is
  the one the browser uses).
- **Builds.** The crate record 6 names, with a `wasm-bindgen` export for
  each function the client calls in those modules, each a direct call
  into the core with conversion only. The loader, which fetches and
  compiles the module from the server's own origin, and the TypeScript
  that makes it a `CorePort`. The `wasm32` build as a step of the web
  gate. See
  [Core logic before the WASM facade](#core-logic-before-the-wasm-facade).
- **Not built here.** Search, the decision and Home rows (CP-023). Sync
  frames, the library held in WASM, response decoding and the size check
  (WP-088).
- **Tests.** Native unit tests of every conversion, so the crate is
  covered on the host. In Vitest, against the real module: three "play
  next" picks play in the order chosen; a stale operation is rejected
  with the current version; the same seed gives the same shuffle order;
  a gain grid row gives its literal decision; every legal player
  transition in player.md's diagram, and a rejected illegal one; the
  line for a literal lyric position. Type checking fails if the
  generated declarations stop satisfying `CorePort`.
- **Risks.** `wasm-bindgen`'s generated code and the workspace's
  `unsafe_code = "forbid"` (record 6, decision 9), and coverage of the
  generated glue, are both unverified; they were WP-088's risks and
  arrive earlier with this package.
- **Done when.** The demo's queue, shuffle, gain and player state are
  the core's own.

### CP-014 Stores: session, library and loves

- **Wave** C1 · **Size** M · **Depends on** CP-005, CP-012.
- **Owns** `app/package.json`, `app/src/session/`, `app/src/library/`,
  `app/src/loves/`.
- **Serves** CLI-022, DIS-002, DIS-045, LIB-004, MUS-027, MUS-180,
  MUS-208.
- **Security.** SEC-STD-012.
- **Builds.** Small stores with a subscribe function and the hooks over
  them: who is signed in, which library is selected, the reads every
  page makes (an album, an artist, a list in a given order), and loves.
  A read never waits on the network. A love is a user-log write; when
  the server cannot be reached the control is disabled with "Needs the
  server" (D-71).
- **Tests.** With scripted ports: each hook returns the literal value
  for the port's answer; a problem becomes a typed error state; a change
  notifies each subscriber once.
- **Done when.** A page gets its data from one hook and holds no port.

### CP-015 Lists, grids, rows and tiles (SUR-023)

- **Wave** C1 · **Size** L · **Depends on** CP-007, CP-014.
- **Owns** `ui/src/lists/`.
- **Serves** SUR-023 in R1: DIS-051, DIS-100, DIS-104, LIB-142, LIB-146,
  MUS-001, MUS-002, MUS-020, MUS-021, MUS-040, MUS-180, MUS-182,
  MUS-229.
- **Security.** SEC-CLI-001 (rows and tiles over the hostile album).
- **Builds.** The endless list and grid, drawing only what is on screen.
  The track row: title, every credited artist linked, duration, play
  count, the heart, the format badge, and the dimmed state with its
  reason. The tile, with artwork in the size the layout needs and a
  gradient placeholder from the core's palette. The sort menu; the order
  itself comes from the core's sort fields. The explicit badge, the view
  toggle, filters, the alphabet jump and find-in-list are R1.1.
- **Tests.** A list of 100,000 synthetic rows mounts no more than the
  window and its overscan, at every scroll position sampled, and holds
  the DIS-019 render budget (register D-87) in the reference profile.
  Each row state shows its literal text. Arrow keys, Home and End move
  through a list; a row's controls are reachable by Tab.
- **Done when.** Every R1 list and grid is one of these components.

### CP-016 Library (SUR-022)

- **Wave** C1 · **Size** M · **Depends on** CP-009, CP-015.
- **Owns** `ui/src/surfaces/library/`.
- **Serves** SUR-022 in R1: ADM-031, DIS-046, LIB-004, LIB-021, MUS-027,
  MUS-043, MUS-132, MUS-149, MUS-208.
- **Security.** SEC-CLI-001.
- **Builds.** The Artists, Albums, Songs, Playlists and Loved tabs; the
  library switcher when a person can see more than one library; a
  library that fills in while the first scan runs; the empty library
  that says what is happening.
- **Tests.** Each tab lists the fixture's items in the fixture's order.
  One library shows no switcher; two show it. The empty and scanning
  states show their literal text.
- **Done when.** The owner can browse the demo library three ways.

### CP-017 Album page (SUR-025)

- **Wave** C1 · **Size** M · **Depends on** CP-015, CP-020.
- **Owns** `ui/src/surfaces/album/`.
- **Serves** SUR-025 in R1: DIS-045, DIS-050, DIS-051, LIB-134, LIB-135,
  LIB-146, MUS-001, MUS-002, MUS-003, MUS-011, MUS-012, MUS-021,
  MUS-039, MUS-054, MUS-180; F05 steps 1 and 2.
- **Security.** SEC-CLI-001.
- **Builds.** The header with artwork, the album artist apart from track
  artists and linked credits, over a wash of the album's palette where
  the text still passes the contrast check and a neutral surface where
  it does not; disc headers with disc titles and "Play disc"; the track
  list; Play and Shuffle.
- **Tests.** The multi-disc fixture shows its disc titles and plays one
  disc. The compilation shows each track's own artist. Play sends the
  album as the From lane, starting at the chosen track. A palette that
  fails contrast gives the neutral surface.
- **Done when.** The owner can open an album and start it.

### CP-018 Artist page (SUR-024)

- **Wave** C1 · **Size** M · **Depends on** CP-015, CP-020.
- **Owns** `ui/src/surfaces/artist/`.
- **Serves** SUR-024 in R1: DIS-045, DIS-050, DIS-051, LIB-040, MUS-001,
  MUS-003, MUS-004, MUS-006, MUS-011, MUS-051, MUS-052, MUS-180,
  MUS-182.
- **Security.** SEC-CLI-001.
- **Builds.** The header with the line that tells same-name artists
  apart, the albums, "Appears on", "All songs", Play and Shuffle.
  Release-type sections, role tabs and the artist image are R1.1.
- **Tests.** The two same-name fixture artists show different
  disambiguation lines and different albums. A compilation appears under
  "Appears on" and not among the albums.
- **Done when.** Every credited name on a row or a page leads here.

### CP-019 Audio engine

- **Wave** C1 · **Size** L · **Depends on** CP-005, CP-013.
- **Owns** `player/`.
- **Serves** ACC-122, MUS-066, MUS-070, MUS-071, MUS-079, MUS-084 to
  MUS-089 (applying the core's decision), MUS-229 (the capability
  report).
- **Security.** SEC-API-027, SEC-API-029.
- **Builds.** The direct path in
  [Playback on the web](#playback-on-the-web-in-r1): an output interface
  with one web implementation over two audio elements and a gain node
  each; the capability probe; the translation of element events into the
  core's player events; early loading of the next item; silent refresh
  of an expired URL; the buffered range; volume. The element is created
  behind a seam so component tests script it.
- **Tests.** With a scripted element: each sequence of element events
  gives a literal sequence of core events; the gain set on the node is
  the decision's number; a pause past the URL's expiry under the
  injected clock asks for a new URL and resumes at the same position
  with no error state; the next item starts loading at the set point and
  is requested as part of the same playback; a stream URL never appears
  in a store, a log line or the media session. In three browser engines,
  a tone fixture plays and its position advances. Stubbed probe answers
  give a literal capability report.
- **Done when.** The demo makes sound, levelled, with the next track
  ready.

### CP-020 Playback controller

- **Wave** C1 · **Size** M · **Depends on** CP-013, CP-014, CP-019.
- **Owns** `app/src/playback/`.
- **Serves** CLI-093, LAT-009, MUS-077, MUS-079, MUS-116 to MUS-119,
  MUS-122, MUS-123, MUS-126, MUS-182, MUS-229.
- **Security.** None of its own: the rules are the core's and the
  enforcement is the server's.
- **Builds.** The queue store: each action (play, play next, add, play
  last, move, remove, clear, clear Up next, shuffle and its mode,
  repeat, stop after) becomes a core operation applied optimistically,
  sent to the server, and rebased by the core when the server rejects a
  stale version. The player store holding the core's state. Moving on at
  the end of an item, skipping a damaged or unplayable one. Pausing when
  another session pressed Play. Play events with real timestamps, kept
  in memory and sent when the server is reachable.
- **Tests.** With scripted ports: each action sends the literal
  operation with the version it was built on; a stale rejection calls
  the core's rebase with the pending operations and the store holds what
  the core returned; a sync showing another session's Play pauses this
  one at its position; a play event holds exactly the fields the user
  log allows.
- **Done when.** Every player surface reads and changes playback through
  this one store.

### CP-021 Now-playing bar (SUR-002)

- **Wave** C1 · **Size** M · **Depends on** CP-009, CP-020.
- **Owns** `ui/src/surfaces/bar/`.
- **Serves** SUR-002 in R1: ACC-117 (the indicator's place), DIS-045,
  MUS-001, MUS-066, MUS-099, MUS-108, MUS-109, MUS-113, MUS-122,
  MUS-180.
- **Security.** SEC-CLI-001.
- **Builds.** The bar at every width class: artwork, title, linked
  credits, the progress line, play and pause, next, the heart, the
  compact quality badge, and on wide layouts the lyrics and queue
  toggles, Options, volume and the device slot, which is empty in R1 and
  keeps its place. No bar while nothing is queued.
- **Tests.** The layout contract: the bar's position and the order of
  its controls equal literal values at each width class. Each player
  state fixture shows its literal presentation. Every control has a name
  and a state.
- **Done when.** Playback is visible and controllable on every screen.

### CP-022 Queue (SUR-011)

- **Wave** C1 · **Size** M · **Depends on** CP-015, CP-020.
- **Owns** `ui/src/surfaces/queue/`.
- **Serves** SUR-011 in R1: CLI-060, CLI-149, LAT-006, MUS-116, MUS-119,
  MUS-122, MUS-123, MUS-229.
- **Security.** SEC-CLI-001.
- **Builds.** The queue as the full-height right pane at wide, over the
  content at expanded, a side sheet at medium and a sheet at compact.
  The "Up next" and "From" lanes, each item saying where it is playing
  from, unplayable items dimmed with the reason. Reorder by drag and by
  the menu's "Move up", "Move down" and "Move to top of Up next";
  remove, with the button always shown on touch. "Clear Up next" and
  "Clear queue" ask first and say how many items will go; removing one
  row acts at once (interface decision 2).
- **Tests.** Each edit sends the literal operation. A move between lanes
  by keyboard alone. The clear prompts state the literal count and open
  with focus on Cancel. The layout contract for queue access.
- **Done when.** The first clickable milestone is met.

## Wave C2: the whole listening experience

### CP-023 The core in the browser, second slice

- **Wave** C2 · **Size** S · **Depends on** CP-013; WP-054, WP-055,
  WP-059 (wave 2).
- **Owns** `crates/gunmetal-wasm/src/search.rs`, `decision.rs`,
  `home.rs`; `core-wasm/src/search.ts`, `decision.ts`, `home.ts`.
- **Serves** DIS-001, DIS-020, DIS-021, DIS-035, DIS-036, DIS-038,
  DIS-083 to DIS-085, MUS-050, MUS-059, MUS-061, MUS-099, MUS-149,
  MUS-229, MUS-236.
- **Security.** SEC-CLI-021.
- **Builds.** Exports for the search index, the playback decision with
  its track details summary, and Home rows. The demo's records are now
  passed through the core's own catalogue types to build the index and
  the rows, so the fixture's hand-written search, decision and Home
  answers are deleted, and a fixture record the core's types refuse
  fails the gate.
- **Tests.** Native conversion tests. Against the real module: a literal
  query over the demo library returns literal grouped results; a stubbed
  capability report gives each fixture track its literal decision and
  reasons; the Home rows for a literal set of plays.
- **Done when.** Search, the badge and Home in the demo are the core's.

### CP-024 Context menu and the action list (SUR-004)

- **Wave** C2 · **Size** M · **Depends on** CP-007, CP-020.
- **Owns** `ui/src/surfaces/menu/`, `ui/src/actions.ts`.
- **Serves** SUR-004 in R1: DIS-045, DIS-111, INT-008, MUS-062, MUS-117,
  MUS-118.
- **Security.** SEC-CLI-001.
- **Builds.** player.md's one action list: each action has one
  identifier, one message, the items it applies to and when it is
  enabled, and menus and media handlers all call it. The menu, opened by
  right-click, the "more" button, long-press and the keyboard's menu
  key, with Play next, Add to queue, Play last, Add to playlist, Love,
  Go to artist, Go to album, Info, and "Copy ID" in developer mode.
  Later packages add their actions as lines (Remove this play from
  CP-032, Rescan from CP-045).
- **Tests.** The same item shows the same actions in the same order from
  every opener. Three "Play next" picks send three operations in order.
  The menu is fully usable by keyboard and closes with Escape.
- **Done when.** Every row and tile offers the same menu.

### CP-025 Full-screen player (SUR-010)

- **Wave** C2 · **Size** M · **Depends on** CP-021, CP-024.
- **Owns** `ui/src/surfaces/player/`.
- **Serves** SUR-010 in R1: MUS-001, MUS-002, MUS-039, MUS-070, MUS-071,
  MUS-077, MUS-087, MUS-099, MUS-109, MUS-110, MUS-113, MUS-116,
  MUS-122, MUS-123, MUS-126, MUS-227.
- **Security.** SEC-CLI-001.
- **Builds.** Large artwork over the album's palette, with the neutral
  fallback; the title and the credit as tagged; the "Playing from" link;
  the always-visible scrubber with the buffered range; transport;
  shuffle with its two R1 modes, repeat and stop-after; the heart; the
  badge; buttons for the queue, lyrics and info; the options menu. It
  fills the content area on wide layouts and is a sheet at compact,
  closed by a button as well as a swipe. If the owner answers yes to the
  [shortcut question](#product-questions-for-the-owner), the six player
  keys and their off switch are built here.
- **Tests.** The scrubber is a slider with a spoken value, moved by the
  arrow keys. The layout contract for the scrubber and the queue, lyrics
  and device positions. Every state fixture's literal presentation.
- **Done when.** The owner can run the player from one screen.

### CP-026 Lyrics (SUR-012)

- **Wave** C2 · **Size** S · **Depends on** CP-013, CP-021.
- **Owns** `ui/src/surfaces/lyrics/`.
- **Serves** SUR-012 in R1: LIB-067, MUS-154, MUS-155.
- **Security.** SEC-CLI-001, SEC-MED-057.
- **Builds.** Plain lyrics, and synced lyrics with the current line
  marked from the core's position lookup, in the right pane, the content
  area or a panel in the player. "This file has no lyrics." when there
  are none. Word-by-word timing and staying open across tracks are R1.1.
- **Tests.** At literal positions the marked line is the literal
  expected one. Lyrics holding markup appear as text with their line
  breaks kept and no element created.
- **Done when.** Lyrics follow the demo's synced track.

### CP-027 Track details and the quality badge (SUR-016)

- **Wave** C2 · **Size** S · **Depends on** CP-007, CP-023.
- **Owns** `ui/src/surfaces/details/`, `ui/src/kit/quality-badge/`.
- **Serves** SUR-016: MUS-021, MUS-032, MUS-034, MUS-036, MUS-037,
  MUS-067, MUS-069, MUS-084, MUS-089, MUS-099, MUS-229, MUS-236.
- **Security.** SEC-CLI-001, SEC-MED-057.
- **Builds.** The badge, compact ("Original") and in full on hover,
  focus or tap, in design-language's words. The read-only details view
  from the core's summary: title and credit as tagged, the album, the
  format, the decision and its reason, whether the track joins the next
  without a gap, and where its level came from.
- **Tests.** Each fixture decision gives its literal sentence, including
  "Can't play in this browser: ALAC". The view offers no edit control
  and shows no file path.
- **Done when.** A person can always see what is playing and why.

### CP-028 Browser media controls (SUR-053)

- **Wave** C2 · **Size** S · **Depends on** CP-020.
- **Owns** `app/src/media-session/`.
- **Serves** SUR-053 in R1: CLI-070, MUS-073.
- **Security.** SEC-API-029 (the artwork handed to the browser is never
  a capability URL).
- **Builds.** The Media Session: metadata, position, and the handlers in
  player.md's table (play, pause, stop, next, previous with the restart
  rule, seek), each calling the action list.
- **Tests.** With a scripted media session: after every play, pause,
  seek and queue edit, what the browser holds equals the player's state
  literally. Signing out clears it.
- **Done when.** The keyboard's media keys and a phone's lock screen
  drive the demo.

### CP-029 Home (SUR-020)

- **Wave** C2 · **Size** M · **Depends on** CP-015, CP-020, CP-023.
- **Owns** `ui/src/surfaces/home/`.
- **Serves** SUR-020 in R1: DIS-001, DIS-002, DIS-004, DIS-020, DIS-021,
  DIS-035, DIS-036, DIS-140, MUS-050, MUS-059, MUS-149.
- **Security.** SEC-CLI-001.
- **Builds.** The rows the core returns, Continue listening first, then
  Recently played, Recently added by album and Loved tracks; the
  first-run cards and empty states. Rows scroll with arrows on wide
  layouts and stack at compact. An arrangeable Home is R1.2.
- **Tests.** For literal row data the page shows the literal rows in
  order. A profile with no plays shows the empty states. Home draws
  without waiting for any port call to resolve.
- **Done when.** The demo opens on a Home made of what was played.

### CP-030 Search and genre pages (SUR-032, SUR-028)

- **Wave** C2 · **Size** M · **Depends on** CP-009, CP-015, CP-023.
- **Owns** `ui/src/surfaces/search/`, `ui/src/surfaces/browse/`.
- **Serves** SUR-032 and SUR-028 in R1: CLI-022, DIS-083, DIS-084,
  DIS-085, DIS-109, LIB-053, MUS-017, MUS-060, MUS-061; F09.
- **Security.** SEC-PRV-004 (the client's half: what is typed never
  leaves the device).
- **Builds.** The search field at the top of the sidebar, reached by Tab
  (no shortcut in R1, interface decision 6), and the Search tab at
  compact with Browse above the results when the field is empty. Results
  as you type, grouped by type with type chips. Genre pages. Recent
  searches, scopes and mood and label pages are R1.1.
- **Tests.** While a query is typed, no `ServerPort` method is called.
  Literal queries, with a typing mistake and without accents, show the
  core's literal groups. A genre page lists the fixture's albums for
  that genre.
- **Done when.** The owner can find anything in the demo by typing.

### CP-031 Playlists (SUR-026, SUR-015)

- **Wave** C2 · **Size** M · **Depends on** CP-015, CP-020, CP-024.
- **Owns** `ui/src/surfaces/playlist/`,
  `ui/src/surfaces/add-to-playlist/`, `app/src/playlists/`.
- **Serves** SUR-026 and SUR-015 in R1: DIS-046, MUS-132, MUS-133,
  MUS-149; F06.
- **Security.** SEC-CLI-001 (playlist names are untrusted text).
- **Builds.** The playlist store, whose edits are operations on a
  versioned document sent to the server. The playlist page: play,
  shuffle, reorder by drag and by menu, remove, rename. The
  add-to-playlist sheet with recent playlists, a search field and "New
  playlist". Playlists in the sidebar. Loved tracks as a list. Edits are
  disabled with "Needs the server" when it cannot be reached (D-71).
  Imports, covers and the duplicate warning are R1.1.
- **Tests.** Each edit sends the literal operation. Creating a playlist
  from the sheet adds the item to it. A playlist named with a corpus
  payload shows it literally in the sidebar, the page and the sheet.
- **Done when.** The owner can build a playlist and play it.

### CP-032 History and private listening (SUR-029)

- **Wave** C2 · **Size** M · **Depends on** CP-015, CP-024.
- **Owns** `ui/src/surfaces/history/`, `app/src/history/`,
  `app/src/private-session/`.
- **Serves** SUR-029: ACC-117, ACC-118, CLI-093, DIS-021, DIS-050,
  DIS-052, DIS-053, DIS-186, DIS-187, DIS-189, MUS-183, MUS-184,
  MUS-185, MUS-233, MUS-234; F16.
- **Security.** SEC-PRV-024, SEC-PRV-022 (the client's half: the page
  holds only this person's plays).
- **Builds.** History by day with the device each play came from; remove
  a play, a range of dates or everything; how long history is kept;
  "Only you can see this". Private session: an item in the player's
  options, first in the list, so it is two interactions away; the
  indicator in the bar, the player and the account menu for as long as
  it is on; its end time. While it is on, the controller passes the
  private mode to the core's event sink, which drops the events.
- **Tests.** Private session is reached in two interactions from the
  full player and from the wide bar. With it on, playing a track sends
  no play event and adds nothing to History or Home. Removing a play
  sends the literal erasure and the row is gone.
- **Done when.** A person can see and erase their own record.

### CP-033 Status layer, notice centre and player states (SUR-003)

- **Wave** C2 · **Size** M · **Depends on** CP-007, CP-012, CP-020.
- **Owns** `ui/src/surfaces/status/`, `app/src/notices/`.
- **Serves** SUR-003 in R1: ACC-003, ACC-071, ACC-075, ADM-085, CLI-002,
  DIS-002, LIB-032, MUS-042, MUS-043, MUS-079, MUS-229; the R1 rows of
  player.md's States table; F12.
- **Security.** SEC-API-031, SEC-API-072, SEC-OPS-033, SEC-TM-040,
  SEC-TM-068 (each in its client half: the refusal or problem is shown
  in plain words and never fails silently).
- **Builds.** The connection indicator, banners and toasts in their
  fixed places. The first-scan banner. The notice centre with this
  account's security notices, each with "It was me" and "It wasn't me".
  "Your session has ended". Every R1 player state as a fixture driven by
  the fake's switches: loading, buffering, slow link, cannot play here
  with the grouped skip notice, damaged file, playback error with "Try
  again", "Skip" and "Details", server unreachable, end of queue,
  playing elsewhere, signed out, and not allowed, which says which
  stream limit was reached and what to do.
- **Tests.** Each state fixture shows its literal sentence and actions.
  "Details" shows only the problem type and the request identifier.
  Skipping two unplayable tracks gives one notice. A notice is announced
  without taking focus.
- **Done when.** Nothing in the player fails silently.

### CP-034 Gapless playback through Media Source

- **Wave** C2 · **Size** L · **Depends on** CP-019, CP-023; WP-056
  (wave 2).
- **Owns** `player/src/mse/`, `fixtures/packaged/`,
  `crates/xtask/src/client_segments.rs` (with its dispatch line).
- **Serves** MUS-067, MUS-069, MUS-070, MUS-071, and the client's half
  of MUS-230.
- **Security.** SEC-CLI-021 (a malformed segment is a typed problem, not
  a crash), SEC-API-029.
- **Builds.** The packaged path in
  [Playback on the web](#playback-on-the-web-in-r1): a second output
  that feeds Media Source (or `ManagedMediaSource`) with the packager's
  initialisation and media segments, sets append windows and timestamp
  offsets from the trim values, seeks through the seek index and evicts
  what has played. The xtask command that runs the Rust packager over
  generated tones to write the segment fixtures, with their manifest.
- **Tests.** With a scripted source buffer: for literal trim values the
  append window and offset equal literal numbers. In each engine in the
  support list: two tracks cut from one continuous tone play across the
  join with no gap in the buffered range, and a seek lands on the
  literal position. Whether each engine lets a test capture the output
  to compare samples is unverified; where it does, the join is compared
  with the known tone.
- **Done when.** The demo's gapless album plays as one piece.

## Wave C3: signing in, settings and account

### CP-035 Sign-in (SUR-070)

- **Wave** C3 · **Size** M · **Depends on** CP-009, CP-012.
- **Owns** `ui/src/surfaces/sign-in/`, `app/src/auth/`.
- **Serves** SUR-070 in R1: ACC-003, ACC-004, ACC-007, ACC-050, ACC-062,
  ACC-063, ACC-079, CLI-150, CLI-155; F03; design-language requirement
  A15.
- **Security.** SEC-CLI-010 (the question), SEC-CLI-028.
- **Builds.** A generic page with no user list, server name or version.
  The passkey button, which runs the browser's own ceremony with the
  server's challenge and names no account. One message for any failure,
  and plain delay messages. After sign-in, "Is this your own device?"
  with two explicit answers. "Use another device" and "Use a recovery
  code". The enrolment screen for an owner recovering from the host.
  There is no password field anywhere.
- **Tests.** With a scripted credentials interface: the ceremony's calls
  equal literal values; every failure shows the same sentence; the
  answer to the question is passed to the server as the literal mode.
  Before sign-in the page's whole text equals a literal that names
  nothing about the server. In Chromium, a virtual authenticator signs
  in end to end.
- **Done when.** The demo can be entered with a passkey.

### CP-036 Storage modes and the wipe

- **Wave** C3 · **Size** M · **Depends on** CP-014, CP-035.
- **Owns** `app/src/storage/`.
- **Serves** ACC-065, ACC-069, ACC-070, CLI-022, CLI-155, CLI-156.
- **Security.** SEC-CLI-009, SEC-CLI-010, SEC-IAM-017, SEC-PRV-019,
  SEC-TM-058, SEC-IAM-043 (the client's half: it wipes before it shows
  anything).
- **Builds.** One storage layer that every store goes through. In a
  shared browser everything stays in memory. In a personal browser the
  catalogue and artwork may be kept in IndexedDB, in a partition per
  account. In both modes the layer refuses credentials, capability URLs
  and history (see
  [Where this plan follows the baseline](#where-this-plan-follows-the-baseline-over-a-ui-document)).
  `wipeAccount`, called on sign-out, on a "session revoked" answer and
  on an account switch: it deletes the account's databases, origin
  private files, Cache Storage and web storage, clears memory and the
  media session, unregisters any service worker and goes to sign-in.
- **Tests.** In three engines: fill the copy, sign out, and every store
  is empty, no service worker is registered and Back shows no data; the
  same after a revocation from a second session, with nothing drawn
  before the wipe. In shared mode nothing is ever written to a store.
  The layer refuses a history record and a credential record by type and
  at run time.
- **Done when.** Signing out of the demo leaves nothing behind.

### CP-037 Welcome: first-run setup (SUR-082)

- **Wave** C3 · **Size** L · **Depends on** CP-035, CP-040.
- **Owns** `ui/src/surfaces/welcome/`, `ui/src/admin/folder-picker/`.
- **Serves** SUR-082 in R1: ACC-001, ACC-002, ACC-050, ACC-113, ADM-018,
  ADM-019, ADM-021, ADM-022, ADM-025, ADM-028, ADM-029, ADM-031,
  ADM-053, ADM-069, ADM-143, LIB-001, LIB-005, LIB-021, LIB-108; F01 and
  the welcome screen's part of F14.
- **Security.** SEC-CLI-013, SEC-CLI-028, SEC-OPS-047 (the client's
  half: the required question has no answer preselected and cannot be
  skipped).
- **Builds.** The R1 steps in surfaces.md's order: the setup code, from
  the link's fragment or typed; where people will reach the server, and
  the paths to HTTPS; the owner's account with a passkey; the recovery
  kit, confirmed by typing its last four characters, and the offer of a
  second passkey; privacy choices and the security-fix question; the
  first music folder, with the folder picker and its live checks; the
  scan starting. "Restore from a backup instead". Language and the
  provider lookups are later releases.
- **Tests.** A code in the fragment is gone from the address and the
  history before the first request and travels in a request body. The
  security-fix step cannot be passed without choosing, and neither
  answer starts selected. The recovery kit step refuses a wrong last
  four. The picker shows each refusal's reason. No step has a password
  field.
- **Done when.** The owner can walk the whole first run on the demo.

### CP-038 Invitation landing (SUR-071)

- **Wave** C3 · **Size** M · **Depends on** CP-035.
- **Owns** `ui/src/surfaces/invite/`.
- **Serves** SUR-071 in R1: ACC-080; the friend's side of F10.
- **Security.** SEC-CLI-013, SEC-CLI-001.
- **Builds.** What will happen, the privacy notice the server generates,
  account creation with the person's own passkey, and, where the
  invitation needs the inviter to confirm, the waiting screen with the
  short matching code. Nothing is redeemed until the person confirms.
- **Tests.** The secret leaves the address and history before any
  request, is sent only in a request body, and only after the confirm
  press. Every failure shows the same sentence. The notice's text is
  rendered literally.
- **Done when.** An invitation link becomes an account on the demo.

### CP-039 Approving another browser (SUR-061)

- **Wave** C3 · **Size** M · **Depends on** CP-035, CP-040.
- **Owns** `ui/src/surfaces/pairing/`.
- **Serves** SUR-061 in R1: ACC-062; F03's approval branch.
- **Security.** SEC-CLI-013, SEC-CLI-028, and the client's half of
  SEC-IAM-058 and SEC-CLI-024.
- **Builds.** On the new browser, the code and its QR form, valid for
  ten minutes. On the approving device, the sheet: the requesting
  device's own name marked as unverified, its type, "In this home" or
  "Somewhere else", how long ago it asked, and what it will get, with
  addresses behind "Details"; the typed-code step when the two are not
  on one network; Approve after a passkey check, and Deny. The sheet is
  never offered on a limited device. The QR form waits for the encoder
  (see [Requests to the backend plan](#requests-to-the-backend-plan)).
- **Tests.** The sheet's text for a literal request equals a literal,
  with the claimed name shown as text and labelled. Approve calls the
  step-up prompt first. In shared mode the approval route shows "not
  available on this device".
- **Done when.** A second browser can be let in from the first.

### CP-040 Step-up prompt and upload dialog (SUR-008, SUR-009)

- **Wave** C3 · **Size** M · **Depends on** CP-007, CP-035.
- **Owns** `ui/src/surfaces/step-up/`, `ui/src/surfaces/upload/`,
  `app/src/step-up/`.
- **Serves** SUR-008 and SUR-009: ACC-056, ACC-125.
- **Security.** The client's half of SEC-IAM-041 and SEC-IAM-023.
- **Builds.** The prompt that says what will change and asks for the
  passkey, in its three kinds: opening admin screens, a host-equivalent
  action, and a change to one's own sign-in methods, export or deletion.
  When the server answers that a fresh check is needed, the prompt opens
  and the action is retried once. The upload dialog: the chosen file,
  what it is for, and a typed error; R1 uses it for backups only.
- **Tests.** A "fresh check needed" problem opens the prompt and the
  original call is repeated once after the check; Cancel changes
  nothing. The prompt's text names the action literally. The dialog
  shows the server's typed refusal for a wrong type and an oversized
  file.
- **Done when.** Every sensitive action in C3 and C4 goes through one
  prompt.

### CP-041 Account menu and Settings (SUR-006, SUR-073, SUR-074, SUR-076, SUR-077)

- **Wave** C3 · **Size** M · **Depends on** CP-036.
- **Owns** `ui/src/surfaces/account-menu/`, `ui/src/surfaces/settings/`,
  `app/src/settings/`.
- **Serves** ACC-012, ACC-017, ACC-113, ACC-114, ACC-117, CLI-031,
  CLI-141, CLI-150, DIS-053, LIB-146, MUS-087, MUS-088, MUS-126,
  MUS-185.
- **Security.** SEC-CLI-009 (Sign out runs the wipe), SEC-CLI-010
  ("About this connection" says which mode this browser is in),
  SEC-SUP-031 (the "Source code" link for the running commit).
- **Builds.** The account menu at the foot of the sidebar, and behind an
  avatar on Home at compact: the person's name, the private-session
  badge, Settings, Account, Sessions and devices, Security events, What
  admins can see, History, Admin for administrators on a personal
  device, and Sign out. Settings: the section list; Privacy; loudness
  mode and target, and the shuffle mode; the five theme choices; "About
  this connection"; Preview features. Settings are one record per key,
  replaced whole.
- **Tests.** Each setting change sends the literal key and value. The
  theme choice applies the literal token set. On a shared browser the
  Admin entry is absent. Sign out calls the wipe before anything else.
- **Done when.** A person can make the demo look and sound as they like.

### CP-042 Account, recovery and your data (SUR-078, SUR-133)

- **Wave** C3 · **Size** M · **Depends on** CP-040, CP-041.
- **Owns** `ui/src/surfaces/account/`, `ui/src/surfaces/recovery/`.
- **Serves** SUR-078 and SUR-133 in R1: ACC-002, ACC-009, ACC-010,
  ACC-050, ACC-055, ACC-065, ACC-136, ACC-137, ACC-138, ADM-143,
  DIS-058, DIS-188, INT-151, LAT-007, MUS-188.
- **Security.** The client's half of SEC-IAM-023, SEC-IAM-106 and
  SEC-PRV-048.
- **Builds.** The person's passkeys, named, with add and remove and the
  prompt while only one exists; the links to the pages in CP-043; "Your
  data", with the export and deleting the account; recovery codes, shown
  once; for the owner, the recovery kit's status; and, during a recovery
  hold, what is restricted and when it ends.
- **Tests.** Adding or removing a passkey, starting an export and
  deleting the account each open the step-up prompt first. The last
  passkey has no remove control. During a hold the export and removal
  controls are disabled with the literal reason.
- **Done when.** A person can manage how they sign in and take their
  data away.

### CP-043 Your devices, security events and what admins can see (SUR-130, SUR-131, SUR-132)

- **Wave** C3 · **Size** M · **Depends on** CP-041.
- **Owns** `ui/src/surfaces/devices/`,
  `ui/src/surfaces/security-events/`,
  `ui/src/surfaces/admin-visibility/`.
- **Serves** ACC-068, ACC-069, ACC-070, ACC-071, ACC-078, ACC-115,
  ACC-139, ADM-110; F15.
- **Security.** The client's half of SEC-IAM-042, SEC-IAM-097 and
  SEC-IAM-104, and SEC-OPS-033.
- **Builds.** Every session and device with its class, rough location
  and last use, "This device" marked, remove one, and "Sign out
  everywhere else". The person's own security events with filters and
  "This wasn't me". The page that says, in the server's own generated
  words, what admins can and cannot see.
- **Tests.** Removing a device sends the literal call and the row goes.
  "This wasn't me" revokes the device in the entry. The visibility page
  renders the server's statements literally and adds none of its own.
- **Done when.** A lost phone can be signed out from the demo.

## Wave C4: administration

Every admin surface opens only inside an admin session that the step-up
prompt starts, and works at the wide and the phone widths (surfaces.md,
"Setup and administration"). Hiding the entry is a courtesy; the server
refuses the routes whatever the client shows (SEC-CLI-015, SEC-CLI-024).

### CP-044 Admin frame and dashboard (SUR-083)

- **Wave** C4 · **Size** M · **Depends on** CP-040, CP-041.
- **Owns** `ui/src/admin/frame/`, `ui/src/admin/dashboard/`,
  `app/src/admin/`.
- **Serves** SUR-083 in R1: ACC-127, ADM-031, ADM-034, ADM-054, ADM-065,
  ADM-072, ADM-083, ADM-108, ADM-116, ADM-142, ADM-144, MUS-042.
- **Security.** The client's half of SEC-IAM-041 (the admin session and
  its end) and SEC-NET-047 (the version appears only here).
- **Builds.** The admin navigation and the session that opens it, with
  the return to the prompt when it ends. The dashboard cards: backups,
  library roots, free space, the scan, the security card, owner alerts
  with critical ones as a banner, advisories, recovery used, updates,
  and for the owner "Rotate all server secrets".
- **Tests.** Opening Admin asks for the passkey first. An expired admin
  session returns to the prompt and shows no admin data. Each card shows
  the literal text for its fixture state.
- **Done when.** An administrator sees at a glance what needs attention.

### CP-045 Libraries and library settings (SUR-085)

- **Wave** C4 · **Size** L · **Depends on** CP-037, CP-044.
- **Owns** `ui/src/admin/libraries/`.
- **Serves** SUR-085 in R1: ACC-037, ADM-025, ADM-085, ADM-108, LIB-001,
  LIB-003, LIB-004, LIB-007, LIB-012, LIB-013, LIB-014, LIB-015,
  LIB-038, LIB-136, MUS-035; F02.
- **Security.** The client's half of SEC-IAM-041 for adding, removing
  and browsing roots.
- **Builds.** The library list with each root's health; per library its
  folders, schedule, watching, storage type, artist splitting rules,
  artwork order, "Media is read-only" and who may see it; add a library;
  scan now; the Rescan action for the context menu.
- **Tests.** Adding a folder opens the step-up prompt, then the picker.
  A new library is shown as visible to the owner and admins only. Scan
  now sends the literal call and the scan card moves.
- **Done when.** The owner can add music and watch it arrive.

### CP-046 Library health and trash (SUR-086, SUR-105)

- **Wave** C4 · **Size** M · **Depends on** CP-044.
- **Owns** `ui/src/admin/health/`, `ui/src/admin/trash/`.
- **Serves** ADM-086, LIB-014, LIB-032, LIB-033, LIB-038, LIB-049,
  LIB-068, LIB-193, LIB-205, LIB-206, LIB-207, MUS-006, MUS-035,
  MUS-038, MUS-044, MUS-079, MUS-155, MUS-229.
- **Security.** SEC-CLI-001 (file names and tag values are untrusted
  text).
- **Builds.** The health report with its filters: damaged and unreadable
  files, quarantined files with the reason and a retry, the scan
  worker's isolation level, same-name collisions, sidecar problems,
  moved files, offline roots and watch warnings. The trash: items, when
  they went missing, when they will be purged, restore and purge now.
- **Tests.** Each problem kind shows its literal line and action. A file
  name holding a corpus payload appears literally. Purge asks first and
  states the count.
- **Done when.** Every problem the scanner found has a place and a next
  step.

### CP-047 Users and invitations (SUR-090)

- **Wave** C4 · **Size** L · **Depends on** CP-044.
- **Owns** `ui/src/admin/users/`.
- **Serves** SUR-090 in R1: ACC-005, ACC-006, ACC-008, ACC-037, ACC-064,
  ACC-076, ACC-080, ADM-052, MUS-027; the owner's side of F10.
- **Security.** The client's half of SEC-IAM-044, SEC-IAM-079 and
  SEC-PRV-025.
- **Builds.** Users with their libraries, role, device count, last
  sign-in, device cap and enable switch; ending a person's sessions;
  "help sign in"; invitations with their address, expiry, uses and
  status, and confirming a waiting invitee's matching code; handing the
  server to a new owner.
- **Tests.** A user row shows exactly the literal fields and nothing
  about what the person plays. An invitation's preset cannot offer a
  library the inviter lacks. Transfer asks both parties for the passkey
  check.
- **Done when.** The owner can invite a friend to chosen libraries.

### CP-048 Security and server settings (SUR-092, SUR-103)

- **Wave** C4 · **Size** M · **Depends on** CP-044.
- **Owns** `ui/src/admin/security-settings/`,
  `ui/src/admin/server-settings/`.
- **Serves** ACC-063, ACC-075, ACC-079, ADM-001, ADM-090, ADM-111.
- **Security.** The client's half of SEC-IAM-041 and SEC-IAM-025.
- **Builds.** Guessing protection and its state; session lifetimes,
  which can only be shortened; the recovery-hold length; remote
  administration. Storage locations; retention; stream limits; About
  with the version, build and target.
- **Tests.** A lifetime field refuses a value above the limit. The page
  holds no control for passwords or for turning sign-in off. Each change
  opens the step-up prompt.
- **Done when.** The owner can tighten the server's rules.

### CP-049 Network and remote access (SUR-093)

- **Wave** C4 · **Size** M · **Depends on** CP-044.
- **Owns** `ui/src/admin/network/`.
- **Serves** SUR-093 in R1: ACC-097, ACC-098, ACC-099, ADM-022, ADM-028,
  ADM-129.
- **Security.** The client's half of SEC-IAM-041; SEC-CLI-001 (proxy and
  host names are untrusted text).
- **Builds.** The posture in plain words; trusted proxies, with any
  undeclared proxy seen and "Trust it"; HTTPS for the owner's domain, a
  supplied certificate or a tailnet name, with its expiry; the
  reverse-proxy and tailnet recipes; privacy choices; the network
  activity page.
- **Tests.** Each posture fixture gives its literal sentence. Trusting a
  proxy opens the step-up prompt. The activity page lists each
  destination with the feature that caused it.
- **Done when.** The owner can see how the server is reached and what it
  contacts.

### CP-050 Backups and updates (SUR-098, SUR-099)

- **Wave** C4 · **Size** M · **Depends on** CP-040, CP-044.
- **Owns** `ui/src/admin/backups/`, `ui/src/admin/updates/`.
- **Serves** ACC-013, ADM-053, ADM-054, ADM-056, ADM-060, ADM-065,
  ADM-066, ADM-068, ADM-069, ADM-072.
- **Security.** SEC-SUP-050 (the client's half: the stale-feed warning
  and the offline status line), and the client's half of SEC-OPS-045.
- **Builds.** The backup list with status, verification and encryption,
  each backup's contents in plain words, retention, download and upload,
  the recovery kit's status and "Make a new kit". Updates: whether the
  check is on, the version, available updates, advisories, the
  end-of-support date, each release's notes, and "Can't confirm you're
  up to date". Restore from these screens is R1.2.
- **Tests.** Download opens the step-up prompt and is offered to the
  owner only. A feed seven days stale shows the literal warning; offline
  mode shows the quiet line instead.
- **Done when.** The owner can check that backups exist and the server
  is current.

### CP-051 Activity, alerts and diagnostics (SUR-100, SUR-101, SUR-102)

- **Wave** C4 · **Size** M · **Depends on** CP-044.
- **Owns** `ui/src/admin/activity/`, `ui/src/admin/alerts/`,
  `ui/src/admin/diagnostics/`.
- **Serves** ACC-078, ADM-034, ADM-077, ADM-080, ADM-083, ADM-095,
  ADM-110, ADM-116, ADM-119, ADM-122, ADM-123, DIS-038, LIB-016,
  LIB-017, LIB-019, LIB-022, LIB-029.
- **Security.** The client's half of SEC-OPS-034 and SEC-PRV-025.
- **Builds.** Scan and job activity with the indicator in the admin
  header; alert rules and destinations, with the critical alerts that
  cannot be muted; log settings; the doctor's results, the write queue
  and "rebuild the cache". The task list with run and cancel, and the
  diagnostic bundle, are R1.2.
- **Tests.** A critical alert has no mute control. An activity entry
  names files and libraries and never a play. The doctor's fixture
  results show their literal lines.
- **Done when.** An administrator can see what the server did and is
  doing.

### CP-052 Security log and the audit anchor (SUR-108)

- **Wave** C4 · **Size** M · **Depends on** CP-036, CP-044.
- **Owns** `ui/src/admin/security-log/`, `app/src/audit-anchor/`.
- **Serves** SUR-108: ADM-110, ADM-145.
- **Security.** SEC-OPS-075, and the client's half of SEC-OPS-027.
- **Builds.** Events newest first with filters; the result of the last
  chain verification and "verify now"; exporting checkpoints; the
  owner's investigation mode. The anchor: in a personal browser an
  admin's client keeps the latest signed checkpoint head, and at sign-in
  asks the core whether the server's log extends it, alerting when it
  does not. The check itself is the core's (see
  [Requests to the backend plan](#requests-to-the-backend-plan)).
- **Tests.** With a scripted core: an older stored head and a log that
  does not extend it raises the literal alert; one that does raises
  nothing and the newer head replaces the old. In shared mode no head is
  kept. Other people's addresses are shown shortened.
- **Done when.** A rewritten log is noticed by a client that saw the old
  one.

## Wave C5: the real server

Each package here starts when the server packages it names have merged.
None waits for C4.

### CP-053 HTTP adapter and event channel

- **Wave** C5 · **Size** M · **Depends on** CP-005; WP-118 (wave 2),
  WP-083, WP-089 (wave 3).
- **Owns** `http-server/`.
- **Serves** ACC-124, CLI-001, INT-005, INT-023.
- **Security.** SEC-CLI-011 (the client's half: the build identifier on
  every call, and a reload on the typed answer), SEC-CLI-021,
  SEC-API-072, SEC-TM-058 (no script ever holds a credential).
- **Builds.** `ServerPort` over `fetch` against `/api/v1` on the page's
  own origin, with the session cookie the browser holds and scripts
  cannot read. Every response is handed to the core to decode; a failure
  is a typed problem. The uniform 401 ends the session and runs the
  wipe. The event channel with reconnection. The check of every call
  against `openapi.json`.
- **Tests.** With a scripted `fetch`: each method sends the literal
  request; no request carries a secret in its URL; a malformed, an
  oversized and a truncated response each become the typed error screen;
  the "reload required" answer reloads once. A call naming a route
  absent from the fixture description fails the check.
- **Done when.** `apps/web` composes with no fake module.

### CP-054 Sync client and the device copy

- **Wave** C5 · **Size** L · **Depends on** CP-023, CP-036, CP-053;
  WP-084, WP-088 (wave 3).
- **Owns** `app/src/sync/`, `core-wasm/src/library.ts`.
- **Serves** CLI-022, CLI-093, DIS-002, LIB-021, MUS-043, MUS-208.
- **Security.** SEC-CLI-021, SEC-PRV-019, and the client's half of
  SEC-CLI-020 (a removal deletes everything that pointed at the item).
- **Builds.** A snapshot, then deltas from a cursor, pulled when the
  event channel nudges and on a timer without it; the "take a fresh
  snapshot" answer; removals. Frames go to the core, which holds the
  library; the library reads in `CorePort` now come from the facade, and
  the fixture core's lookups are deleted. The frames are kept through
  the storage layer according to the browser's mode. Until the first
  snapshot lands, the app says the library is not ready.
- **Tests.** With recorded frames: after a snapshot the reads return the
  literal library; a delta with a removal takes the item out of every
  list, the queue and the playlists; a "resnapshot" answer replaces the
  copy. In shared mode no frame reaches a store.
- **Done when.** The owner's own library appears in the client.

### CP-055 The contract run against the real server

- **Wave** C5 · **Size** M · **Depends on** CP-053, CP-054; WP-119
  (wave 2) and the wave 3 route packages.
- **Owns** `clients/e2e/contract/`, `fixtures/recorded/`,
  `.github/workflows/web-contract.yml`.
- **Serves** The owner's choice of 2026-10-03: swapping the fake for the
  server is a configuration change.
- **Security.** None of its own.
- **Builds.** Check 4 in
  [What keeps the fake and the real server in step](#what-keeps-the-fake-and-the-real-server-in-step):
  the job builds the server, writes WP-119's small synthetic library,
  starts the server on loopback, runs the conformance suite against it
  and against the fake replaying the recording, re-records, and compares
  hashes. The workflow sets `permissions: {}` and pins its actions.
- **Tests.** These are the tests. The job is checked to fail: a fake
  altered to answer one field differently fails the suite, and a stale
  recording fails the hash comparison.
- **Done when.** A change on either side that the other does not follow
  fails CI.

### CP-056 Production bundle and the handover to the server

- **Wave** C5 · **Size** M · **Depends on** CP-003; WP-072 (wave 3).
- **Owns** `clients/tools/gate/bundle/`, `clients/apps/web/build-info/`.
- **Serves** CLI-001.
- **Security.** SEC-CLI-012, SEC-SUP-037, SEC-STD-017, SEC-SUP-031, and
  the client CI's half of SEC-CLI-016.
- **Builds.** The production build's output in the form WP-072 embeds: a
  manifest of exact paths with their types and SHA-256 digests, hashed
  file names, no source maps, no service worker in R1, the build
  identifier, and the source repository and commit for the "Source code"
  link. The bundle checks in the gate: no module of `fake-server` or
  `fixtures`, no absolute URL to another origin, no secret, and the size
  against the DIS-019 load budget. Two builds of one commit give the
  same digests, and the build needs no network after the install.
- **Tests.** Each check fails on a fixture bundle with the fault
  planted. The manifest of a small fixture build equals a literal.
- **Done when.** The server serves the client it was built with, from
  its own binary.

### CP-057 Real streaming

- **Wave** C5 · **Size** M · **Depends on** CP-019, CP-034, CP-053;
  WP-082 (wave 3), WP-103, WP-104, WP-105 (wave 4).
- **Owns** `player/src/streams/`.
- **Serves** ACC-075, ACC-122, LIB-142, MUS-066, MUS-230.
- **Security.** SEC-API-027, SEC-API-029, and the client's half of
  SEC-API-028 and SEC-IAM-043.
- **Builds.** Capability URLs for originals, packaged segments and
  artwork in the layout's sizes, asked for when needed and refreshed
  silently; a refused range ends playback with the plain message and,
  when the session is gone, the wipe; the typed stream-limit refusal.
- **Tests.** Against the real server: a pause past expiry under an
  injected clock resumes with no error; revoking the session mid-track
  stops playback at the next range request and the client wipes before
  showing anything; a capture of network and storage finds no capability
  URL in Cache Storage, the media session or a log.
- **Done when.** The owner's own files play, gaplessly, from the server.

### CP-058 The R1 flows, end to end in a browser

- **Wave** C5 · **Size** L · **Depends on** every client package; the R1
  server packages, beside WP-117 (wave 6).
- **Owns** `clients/e2e/flows/`.
- **Serves** The browser side of F01, F02, F03, F05, F06, F09, F10, F12,
  F15 and F16, and the browser steps of F13 and F14.
- **Security.** SEC-TM-053 (the client's half), SEC-API-044, SEC-API-046,
  SEC-API-049, SEC-CLI-009, SEC-CLI-012, SEC-CLI-013, SEC-CLI-027,
  SEC-PRV-019, SEC-TM-058.
- **Builds.** One test per flow's main path, driving a real browser
  against a real server process with a synthetic library; failure
  branches stay at the lower layers, as flows.md asks. The whole-client
  security runs: every screen under the server's own policy header in
  three engines; the full suite behind a proxy that refuses every host
  but the server; the hostile library scanned by the real server and
  shown literally on every surface; storage inspected after sign-in in
  both modes and after sign-out; the server's access log searched for
  the invitation, pairing and setup secrets.
- **Tests.** These are the tests.
- **Done when.** R1's journeys pass in a browser against the server that
  ships.

### CP-059 Browser support list and the release accessibility script

- **Wave** C5 · **Size** S · **Depends on** CP-010, CP-058.
- **Owns** `docs/ui/browser-support.md`, `docs/ui/accessibility-script.md`.
- **Serves** CLI-002, CLI-135, CLI-136.
- **Security.** SEC-API-052 (the review of each browser feature's
  fallback).
- **Builds.** The published list of supported browsers, which is also
  the test matrix; the written screen-reader script that is run by hand
  before each release (design-language requirements A13 and A21).
- **Tests.** A test fails when the Playwright projects differ from the
  published list.
- **Done when.** "Supported" means tested, and the release checklist has
  its manual accessibility pass.

## Coverage: every R1 surface and flow has a package

Every surface whose first release is R1 in
[surfaces.md's index](../ui/surfaces.md#surface-index):

| Surface | Package | Surface | Package |
|---|---|---|---|
| SUR-000 Every surface | CP-010 | SUR-070 Sign-in | CP-035 |
| SUR-001 Navigation shell | CP-009 | SUR-071 Invite landing | CP-038 |
| SUR-002 Now-playing bar | CP-021 | SUR-073 Settings | CP-041 |
| SUR-003 Status layer and notice centre | CP-033 | SUR-074 Playback and sound | CP-041 |
| SUR-004 Context menu | CP-024 | SUR-076 Appearance | CP-041 |
| SUR-006 Account menu | CP-041 | SUR-077 This device | CP-041 |
| SUR-008 Step-up prompt | CP-040 | SUR-078 Account | CP-042 |
| SUR-009 Upload dialog | CP-040 | SUR-082 Welcome | CP-037 |
| SUR-010 Full-screen player | CP-025 | SUR-083 Admin dashboard | CP-044 |
| SUR-011 Queue | CP-022 | SUR-085 Libraries | CP-045 |
| SUR-012 Lyrics view | CP-026 | SUR-086 Library health | CP-046 |
| SUR-015 Add-to-playlist sheet | CP-031 | SUR-090 Users and invitations | CP-047 |
| SUR-016 Track details | CP-027 | SUR-092 Sign-in and security settings | CP-048 |
| SUR-020 Home | CP-029 | SUR-093 Network and remote access | CP-049 |
| SUR-022 Library | CP-016 | SUR-098 Backups and export | CP-050 |
| SUR-023 Lists and grids | CP-015 | SUR-099 Updates | CP-050 |
| SUR-024 Artist page | CP-018 | SUR-100 Tasks and activity | CP-051 |
| SUR-025 Album page | CP-017 | SUR-101 Alerts and logs | CP-051 |
| SUR-026 Playlist page | CP-031 | SUR-102 Diagnostics | CP-051 |
| SUR-028 Browse pages | CP-030 | SUR-103 Server settings and about | CP-048 |
| SUR-029 History page | CP-032 | SUR-105 Trash | CP-046 |
| SUR-032 Search | CP-030 | SUR-108 Security log | CP-052 |
| SUR-053 System media controls | CP-028 | SUR-130 Sessions and devices | CP-043 |
| SUR-061 Pairing and approval | CP-039 | SUR-131 Security events | CP-043 |
| SUR-132 What admins can see | CP-043 | SUR-133 Recovery | CP-042 |

R1 surfaces with no client package, and why:

- **SUR-080 Startup page and SUR-109 help pages** are rendered by the
  server itself (WP-095, WP-132) and work with no script. The client
  plan supplies their stylesheet (CP-004).
- **SUR-110 Command line, SUR-111 Project site, SUR-112 API reference
  page** are outside the client. WP-089 serves the API reference.

The R1 flows:

| Flow | Packages |
|---|---|
| F01 First-run setup | CP-037, CP-045; end to end in CP-058 |
| F02 Adding a library and watching it scan | CP-045, CP-046, CP-051, CP-016 |
| F03 Signing in on a new device | CP-035, CP-036, CP-039 |
| F05 Playing an album and editing the queue | CP-017, CP-019 to CP-022, CP-024 to CP-028, CP-034 |
| F06 Building a playlist (by hand) | CP-031 |
| F09 Searching (music) | CP-030 |
| F10 Sharing a library with a friend | CP-047, CP-038 |
| F12 Recovering from a failed playback | CP-033, CP-027, CP-046 |
| F13 Upgrading and rolling back | CP-050 (the browser's part; the startup page is the server's) |
| F14 Rebuilding from a backup | CP-037 (restore at the welcome screen) |
| F15 Losing a phone (sessions) | CP-043, CP-036 |
| F16 Keeping your own record | CP-032, CP-042 |

The feature rows the backend plan lists as "client only" are served
here: CLI-031 (CP-010, CP-021), CLI-060 and CLI-149 (CP-009), CLI-070
and MUS-073 (CP-028), CLI-135, CLI-136, CLI-138 to CLI-140 and CLI-142
(CP-006, CP-007, CP-010), CLI-141 (CP-004, CP-041), DIS-100 and DIS-104
(CP-015), DIS-109 (CP-030), DIS-111 (CP-024), DIS-112 (CP-009), MUS-052
(CP-018), MUS-108 and MUS-113 (CP-021), MUS-227 (CP-025). The browser
halves it names are CLI-156 (CP-036) and CLI-159 (CP-006). CLI-002, which
it left without an owner, is CP-059.

## Security requirements the backend plan left to this plan

The backend plan's
[security coverage table](work-packages.md#security-coverage-every-r1-requirement-has-a-package)
leaves 16 R1 requirements unassigned as web-client behaviour, and the
client's part of 11 more. Each now has a package whose tests carry its
`Verifies:` line.

| Requirement | Short name | Package |
|---|---|---|
| SEC-TM-036 | Render untrusted text | CP-006 (lint: CP-002) |
| SEC-TM-053 | No telemetry (the client's half) | CP-058 (the deny-all proxy run); CP-001 (the deny-list) |
| SEC-TM-058 | Session only in an HttpOnly cookie (the client's half) | CP-036, CP-053, CP-058 |
| SEC-IAM-017 | No session or token in browser storage | CP-036 |
| SEC-API-045 | Build fails on HTML-string and code-string sinks | CP-002 (lint), CP-003 (the policy) |
| SEC-API-046 | Text from files is rendered as text | CP-006, CP-011, CP-058 |
| SEC-API-049 | No third-party loads (the client's half) | CP-003, CP-058 |
| SEC-API-050 | Message listeners check the origin | CP-002 |
| SEC-API-052 | Start-up feature check | CP-003, CP-059 |
| SEC-CLI-001 | Render every untrusted string as text | CP-002 (lint), CP-006, and every surface package |
| SEC-CLI-009 | Delete the account's data on sign-out (the client's half) | CP-036, CP-058 |
| SEC-CLI-010 | Personal or shared (the client's half) | CP-035, CP-036 |
| SEC-CLI-013 | Secrets in the fragment, off the address bar | CP-009, CP-037, CP-038, CP-039, CP-058 |
| SEC-CLI-018 | Installs from the lockfile, scripts off (client CI) | CP-001 |
| SEC-CLI-019 | Dev servers bind to loopback | CP-003, CP-012 |
| SEC-CLI-027 | No analytics or tracking | CP-001, CP-058 |
| SEC-CLI-028 | Code fields mask and turn off suggestions | CP-006 |
| SEC-MED-057 | Media strings as text nodes in a bidirectional isolate | CP-002 (lint), CP-006 |
| SEC-MED-058 | Metadata URLs are links only after parsing | CP-006 |
| SEC-OPS-075 | The audit anchor (the client's half) | CP-052 |
| SEC-PRV-019 | No Activity data in storage unless personal | CP-036, CP-058 |
| SEC-SUP-033 | Frozen installs, one registry, no scripts (client CI) | CP-001 |
| SEC-SUP-034 | Seven-day minimum age (client CI) | CP-001 |
| SEC-SUP-035 | Direct dependencies listed with reasons | CP-001, then every dependency request |
| SEC-SUP-036 | Registry signatures verified (client CI) | CP-001 |
| SEC-HIS-027 | All media text as text; sinks fail the lint | CP-002 (lint), CP-006 |
| SEC-STD-012 | Untrusted keys in `Map`; no merges | CP-002 (lint), CP-005 |

## Requests to the backend plan

This plan changes nothing in [work-packages.md](work-packages.md) beyond
its one pointer line. These are the changes it needs there, for that
plan's owner and integrator to apply or refuse.

1. **WP-088 no longer creates `crates/gunmetal-wasm`.** CP-013 creates
   it in wave 1's wake and CP-023 extends it, one file per core module,
   exactly as WP-088 would have written them. WP-088 keeps sync frames,
   the in-memory library and its reads, response decoding, the size
   check and `wasm.yml`. The risks WP-088 records (the `unsafe` lint,
   coverage of generated glue) arrive with CP-013.
2. **What the client needs from the facade** is the `CorePort` list in
   CP-005. Beyond what WP-088 already names, it asks for: reads over the
   in-memory library (an item by ID, a list in a named order), decoding
   of route responses as well as sync frames (SEC-CLI-021), the applied
   gain as a linear factor so the client does no arithmetic, the
   audit-head extension check (SEC-OPS-075), and a QR matrix.
3. **A QR encoder in the core**, shared by the server's console code and
   the client. If refused, CP-038 and CP-039 file an npm dependency
   request instead.
4. **WP-127's traceability check reads TypeScript.** It has to find
   `// Verifies:` lines under `clients/`, or the requirements in the
   table above will look untested.
5. **WP-136 calls CP-001's xtask check** of the package-manager settings
   for the release workflow, instead of writing a second one.
6. **The D-74 spike is CP-003**, not WP-072. WP-072 embeds the manifest
   CP-056 writes, and its tests can move from the two-file stand-in to
   the real bundle when CP-056 lands.
7. **WP-095 and WP-132 embed CP-004's stylesheet** for the startup and
   help pages, so those pages share the tokens.
8. **WP-115's list render budget** (DIS-100) is CP-015's test, as that
   plan already expects.
9. **CI and the gate** take the six requests in
   [The gate and CI](#the-gate-and-ci-requests-to-the-integrator).

## Where this plan follows the baseline over a UI document

1. **The Trusted Types directive.** The client security guidance shows a
   starting policy with `trusted-types 'none'`. The requirement itself
   (SEC-API-044, SEC-API-045) names one policy, `gunmetal-loader`. The
   plan follows the requirement (CP-003).
2. **History in browser storage.** api-needs.md and SEC-PRV-019 allow
   Activity data in IndexedDB once a person marks the browser as their
   own. SEC-TM-058 says the web client must not persist history in
   browser storage at all. The two baseline rows disagree, so R1 takes
   the stricter one: history, the queue and loves live in memory in both
   modes and come back from the server on load, which R1 needs anyway
   (D-86). Only the catalogue and artwork persist in a personal browser
   (CP-036). The baseline's owner should reconcile the two rows; if
   SEC-PRV-019's reading wins, CP-036 gains one record class.
3. **The preselected answer to "Is this your own device?"** The client
   security guidance preselects "Yes, keep me signed in". The feature
   map (CLI-155) says two explicit answers and no preselection, and the
   register lists it as an open owner choice. The plan builds no
   preselection until the owner answers
   ([question 4](#product-questions-for-the-owner)).
4. **Run-time styles.** design-language allows no style element with
   text and no style attribute, and marks React Native for Web's fit as
   unverified. The plan does not assume it fits: CP-003 proves it under
   the real policy before any screen is written, and a loosening is the
   owner's decision (D-74).

## Open questions

### Product questions for the owner

Each is multiple choice, with the option this plan recommends first.

1. **How do you want to open each demo build?**
   - **A (recommended).** One command on your own machine; the demo
     opens in your browser at a local address. Nothing is hosted and
     nothing leaves the machine.
   - B. A private preview link for each client wave, holding only the
     made-up library. It needs a place to host it and a rule that the
     preview never talks to a real server.
   - C. Both.
2. **What should the demo play?**
   - **A (recommended).** Made-up artists and albums with generated
     tones and generated covers. Nothing to license, and the same on
     every machine.
   - B. A few public-domain or CC0 recordings added to the repository,
     so the demo sounds like music. Each adds a binary file and a
     provenance record.
   - C. Tones in tests, and a small CC0 set only in the demo.
3. **Player keyboard shortcuts in R1** (register D-73, still open).
   - **A (recommended).** Ship Space, Shift+Left, Shift+Right, Shift+N,
     Shift+P and M, with the off switch WCAG requires. A web player with
     no Space for play and pause feels broken.
   - B. No shortcuts in R1; they arrive with the command palette in R2.
4. **"Is this your own device?" at sign-in.**
   - **A (recommended).** Two answers, neither preselected, as the
     feature map says (CLI-155). Nobody keeps data on a shared computer
     by pressing Enter.
   - B. "Yes, keep me signed in" preselected, as the security guidance
     sketches. One less decision at every sign-in.
5. **Which browsers does R1 call supported** (CLI-002)? The list is also
   the test matrix.
   - **A (recommended).** The two latest versions of Chrome, Edge,
     Firefox and Safari, on computers and phones, tested in CI through
     the Chromium, Firefox and WebKit engines, with a pass by hand on
     real Safari and an iPhone before each release.
   - B. The same, plus Firefox ESR.
   - C. Only the latest version of each.
6. **After the first clickable build, what comes next?** C2, C3 and C4
   do not depend on each other.
   - **A (recommended).** Finish listening (C2) and build sign-in and
     account (C3) at the same time; administration (C4) after. The
     player gets good while the pieces the real server needs first are
     made ready.
   - B. Sign-in and administration first (C3, C4), so the real server is
     usable from a browser soonest; the rest of listening after.
   - C. Listening only (C2) until server wave 3 lands.

### Technical choices settled here

Each is recorded in
[record 12](../adr/0012-web-client-toolchain-and-contracts.md), with the
alternatives.

1. **The workspace is `clients/`**, with its own lockfile. The root
   stays a Rust workspace, and the dependency check finds every
   `package.json` in one place.
2. **pnpm**, as the register recommends (D-65).
3. **React Native for Web from the first screen**, because record 1
   decides one React Native interface. Writing the web client on plain
   DOM components first would mean rewriting every screen for R2.
4. **Vite, Vitest, StrykerJS, ESLint, Prettier, Playwright and axe**:
   one well-known tool per job, each named by the baseline or needed by
   a testing rule.
5. **TypeScript 6.0 and ESLint 9**, one major version behind the newest
   of each, because the lint packages the baseline names do not yet
   state support for the newest.
6. **No router, state, schema, internationalisation, icon, font,
   component or virtual-list package.** Each is small enough to own, or
   already belongs to the core.
7. **Two ports**, with the fake and the real server as two
   implementations of one interface, chosen in one file.
8. **The fake is in the page, in memory.** A second server process would
   be a second thing to keep honest.
9. **No core logic in TypeScript, even temporarily.** The facade is
   built in slices as the core's modules merge.
10. **Recorded frames from the real server** replace hand-written
    fixtures in tests from wave 3, and CI re-records and compares them.
11. **The layout contract is measured positions**, not stored
    screenshots: the gate compares the position and order of the pinned
    controls with literal numbers. It is exact, readable in review, and
    adds no binary baselines that differ by machine. Screenshots are
    still produced for a person to look at. If the owner wants pixel
    comparison as well, it is one more sweep.
12. **History is not kept in browser storage in R1**, the stricter of
    two baseline rows.
13. **Client waves use the server waves' branch and merge rules**
    (D-01), on `client-N` branches.
14. **The web gate is part of `scripts/gate.sh`**, not a second script,
    so there is still one definition of done.

## What this plan could not verify

- Whether React Native for Web's run-time styles pass `style-src 'self'`
  in each engine. CP-003 settles it.
- Whether Vite's production output needs any Trusted Types sink beyond
  the one loader policy, and whether Vite compiles JSX without its React
  plugin.
- Whether the package versions in the tables work together. Only their
  existence, names and licences were checked against the npm registry on
  2026-10-03. `@types/react-native-web` trails the library it types.
- The exact names of pnpm 12's settings for release age, trust policy,
  exotic sources and build allow-lists, and the output format of its
  licence listing.
- Whether StrykerJS can limit a run to changed lines, as cargo-mutants
  does, or only to changed files.
- Which audio formats each supported browser decodes from a file, which
  containers each accepts in Media Source, and how Safari's
  `ManagedMediaSource` differs. These were already unverified in the
  feature map (MUS-032, MUS-067, MUS-230).
- Whether a browser test can capture decoded output to prove a gapless
  join to the sample in each engine.
- Whether Playwright's virtual passkey authenticator exists outside
  Chromium.
- Background playback in iPhone browsers (already unverified in
  CLI-003).
- Whether `wasm-bindgen`'s generated code passes the workspace's
  `unsafe_code = "forbid"`, and how coverage counts its glue (already
  unverified in WP-088 and record 6).
- Whether WP-127's traceability check, which is still being built, can
  read `Verifies:` lines in TypeScript files.
- The state of the wave 1 packages CP-013 needs. Their branches existed
  on 2026-10-03; none had merged into `wave-1`.
