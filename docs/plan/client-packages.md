# Web player work packages

Written on 2026-10-03. Status: draft for the project owner's review.
Revised the same day after an adversarial review (see
[Review notes](#review-notes)).

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

Client packages are numbered CP-001 to CP-062. The backend's packages are
WP-NNN, so the two ranges can never collide, and a CP number is never
reused. Client packages are TypeScript only. The Rust the client needs,
the facade crate, is three backend packages, WP-235 to WP-237, which this
plan asked for and which are specified in the backend plan. Client waves
are C0 to C5; server waves are plain numbers (wave 0 to wave 6). Claims
this plan could not check are marked "(unverified)" and collected in
[What this plan could not verify](#what-this-plan-could-not-verify).
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
27. [Review notes](#review-notes)

## How to use this plan

### What a package is

A client package has the same form as a backend package:

- **Wave.** The client wave it belongs to. Client waves are groups by
  what the owner can test at the end of each; they are not branches (see
  [Working in parallel](#working-in-parallel)). Unlike the backend's
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
  No client package owns a path under `crates/`.
- **Serves.** The surfaces (SUR), flows (F) and feature rows it delivers,
  R1 parts only.
- **Security.** The trust boundaries (TB) and threats (TM-T) from the
  [threat model](../security/threat-model.md) the package touches, which
  SEC-TM-001 requires of every work package, and the requirement IDs its
  tests must verify. Each of those tests carries a `Verifies:` line, as
  [CONTRIBUTING.md](../../CONTRIBUTING.md#tests-name-the-requirements-they-verify)
  describes; in TypeScript it is a comment on the line above the test
  (see [Ground rules](#ground-rules-every-client-package-follows)).
- **Builds.** What it makes, and what it leaves to another package or a
  later release.
- **Tests.** The tests that prove it, named so the first failing test can
  be written from this page.
- **Done when.** A result the package's own tests can observe, on top of
  the definition of done below.

### Definition of done

A client package is done when `scripts/gate.sh` passes on its branch
rebased on the server wave branch it merges into, with the real output
reported (AGENTS.md). From the moment the integrator applies the gate
request in
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

No client package changes a Rust, Cargo, Clippy or Qodana file, so none
runs `scripts/qodana.sh`; the backend packages WP-235 to WP-237 do.

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
| 5. 100% coverage | Vitest's coverage thresholds are 100 for lines, functions, statements and branches, per file, over every file in the covered set below, whether a test imports it or not. JavaScript has real branch coverage, so branches are counted directly. No file in the set is excluded. | CP-002 |
| 6. Zero surviving mutants | StrykerJS runs every mutator over the same set. The gate reads Stryker's JSON report and fails unless every mutant was killed; a survivor, an uncovered mutant, an ignored mutant, a run-time error and a timeout all fail, because a hang is not a detection. Mutants that do not type-check are discarded by the TypeScript checker, as cargo-mutants discards mutants that do not compile. | CP-002 |
| 7. Prove behaviour at the lowest layer | Rules live in the Rust core and are proved there. The client never re-implements one (see [Core logic before the WASM facade](#core-logic-before-the-wasm-facade)). Component tests prove what a screen shows for a given state; browser tests prove wiring, policy and accessibility only. | Every package |

### What is covered and mutated, and what is not

Rules 5 and 6 apply to one stated set of files, and everything outside it
is outside by where it lives, never by an ignore comment or an exclude
list that can grow.

**The covered set** is `clients/packages/*/src/**/*.{ts,tsx}`,
`clients/apps/*/src/**/*.{ts,tsx}` and `clients/tools/**/*.ts`, less test
files. Vitest runs it in jsdom (Node for `tools`), and StrykerJS mutates
all of it.

**Outside the set, by construction:**

| What | Where it lives | Why Vitest cannot run it | How it is still proved |
|---|---|---|---|
| Browser adapters: the one place each browser interface is touched (audio elements and Web Audio, Media Source, IndexedDB, origin private files and Cache Storage, WebAuthn, the Media Session, loading the WASM module) | `clients/packages/*/browser/*.ts`, one file per interface | jsdom has none of these interfaces, and a stand-in would test the stand-in | Lint allows no branch, loop or arithmetic in an adapter: each exported function passes its arguments to the browser and returns what comes back, so the logic around it stays in `src/` and is covered and mutated there. The browser suite calls every adapter export in Chromium, Firefox and WebKit, and Chromium's own script coverage, collected by Playwright, must be 100% for every adapter file. Adapters are not mutated; the no-logic lint is what keeps that honest. |
| The browser harness | `clients/e2e/` | It is test code | Each sweep and helper is checked to fail on a planted page (CP-003, CP-010). |
| Configuration | The files at the root of `clients/` | They are data read by tools | Tests read each committed file and compare the settings that matter with literals (the loopback address, the thresholds, the mutated globs, the Stryker and Playwright worker counts). |
| The generated glue for the WASM module | Build output, never committed | It is generated | The facade's own tests (WP-235) and the client's conformance suites against the real module. |
| Generated type declarations | `crates/gunmetal-wasm/types/`, committed | Declarations hold no code | The drift check in the gate (see [The contract](#the-contract-two-ports)). |

**Module-level data.** Tokens, messages, routes and the action list are
data. A mutant in a module-level constant is "static": Stryker cannot tell
which tests cover it and reruns the whole suite for it. So token values
live in JSON files, which are not mutated and are proved value by value
against design-language's tables, and every other table in the covered
set is returned by a function, so per-test coverage applies. Lint fails on
a module-level object or array literal in the covered set.

CP-002's canary holds one of each kind (a pure function, a component, a
data table, a browser adapter), so what coverage, mutation and the browser
run report for each is known on the first day.

Further rules:

- **No core logic in TypeScript.** Queue verbs, shuffle orders, gain
  decisions, the player state machine, lyric timing, search, Home rows,
  the playback decision, user-event envelopes and their clock, parsing
  inbound links, tint and contrast rules, link and URL filtering, text
  normalisation and wire decoding are the core's. The client calls them
  through `CorePort`. A pull request that writes one of them in
  TypeScript is refused, even as a stand-in.
- **No hand-written copy of a Rust type.** Every type that has a Rust
  definition reaches TypeScript as a generated declaration (see
  [The contract](#the-contract-two-ports)).
- **Untrusted text is text.** Every string from a file, a provider,
  another person or a device is rendered through the kit's `Text`
  component (CP-006), in a bidirectional isolate, never through an HTML
  sink (SEC-CLI-001, SEC-MED-057). No such string ever reaches a style
  value, a class name, an element ID or a URL the client builds
  (design-language, section 1).
- **Inbound links are parsed by the core** (SEC-CLI-025). No client code
  takes a path, a fragment or a QR payload apart. Until the core's parser
  reaches the browser, the client's addresses are a fixed list of
  secret-free paths (CP-009).
- **Tokens only.** Components use token names from CP-004 and never raw
  colours, sizes or durations. Lint fails on a colour literal outside the
  tokens package.
- **Every string is a message.** Interface text comes from the message
  catalogue (CP-008), so the pseudo-locale build finds hard-coded strings
  (design-language requirement A20). R1 ships English only.
- **Every screen is swept.** A package that adds a route or a state adds
  one line to the sweep registry. The policy, single-origin, hostile
  metadata and accessibility sweeps then visit it in Chromium, Firefox and
  WebKit. Until `apps/web` can render screens (CP-053), the sweeps visit
  `apps/demo`, built with the same production configuration and served by
  CP-003's test server under the exact header of SEC-API-044; from CP-053
  they visit both. A test in CP-010 fails when a route in `routes.ts` or
  a state fixture has no line in the registry, so forgetting the line
  cannot pass.
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
  tools/lint/             the project's own lint rules
  packages/
    tokens/               design tokens, themes, the typeface
    ports/                CorePort and ServerPort, the conformance suites
    core-wasm/            loads the gunmetal-wasm build; implements CorePort
    fixtures/             the demo library and generated media
    fake-server/          in-memory ServerPort and the fixture core
    http-server/          ServerPort over HTTP (the real server)
    player/               the audio engine
    app/                  stores and hooks between the ports and the screens
    ui/                   components and surfaces, on React Native primitives
  apps/
    web/                  the production entry: ui with core-wasm and http-server
    demo/                 the same app with fake-server; never shipped
  e2e/                    browser suites and the sweep registry
```

Inside a package, `src/` is the covered set and `browser/` holds its
browser adapters, if it has any.

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
| Runtime for the tools | Node 24 (the long-term support line), 24.15 or later | jsdom's stated range needs 24.15; the build machine has 24.20.0. The version is pinned in `clients/package.json` and in CI. |
| Package manager | pnpm 12 | The register's recommendation (D-65) and the baseline's outline: it refuses versions younger than seven days, refuses a drop in publishing trust, blocks install scripts and exotic sources (SEC-SUP-033, SEC-SUP-034). |
| Language | TypeScript 6.0, `strict`, `noUncheckedIndexedAccess`, `exactOptionalPropertyTypes` | Type checking is a gate step. 6.0 and not 7.0, because typescript-eslint's stated range stops below 6.1. |
| UI library | React 19 with React Native for Web | Record 1, decision 8: one React Native interface for every platform. The web build renders React Native primitives through `react-native-web`. |
| Bundler and dev server | Vite 8 | One tool for the dev server and the production build; one entry chunk, hashed file names and a manifest. The dev server binds to loopback in the committed configuration (SEC-CLI-019). |
| Unit and component tests | Vitest 4 with jsdom and Testing Library | Runs TypeScript and JSX with the bundler's own transform; queries by role and name, which is how a screen reader finds a control. Vitest 4 and not 5: version 5 was days old on 2026-10-03 and StrykerJS, released before it, has not stated support for it. |
| Coverage | `@vitest/coverage-v8`, thresholds 100, per file | Rule 5. |
| Mutation testing | StrykerJS 10 with its Vitest runner and TypeScript checker | Rule 6. The one mutation tool for TypeScript with a Vitest runner. |
| Lint | ESLint 10 with typescript-eslint, eslint-plugin-react-hooks and the project's own rules | ESLint 9 reached end of life on 2026-08-06 (eslint.org's version-support page, read 2026-10-03), and eslint-plugin-react's stated range stops at 9. So the two React rules the baseline names, `react/no-danger` and `react/jsx-no-script-url`, are written as the project's own rules in `clients/tools/lint`, beside the other bans the baseline asks for (SEC-CLI-001, SEC-API-045). |
| Formatting | Prettier, `--check` in the gate | The same job `cargo fmt` does. |
| Browser tests | Playwright in Chromium, Firefox and WebKit | The baseline's tests are written for it: the policy sweep, storage inspection, the network recorder (SEC-API-044, SEC-CLI-009, SEC-CLI-012). |
| Accessibility | `@axe-core/playwright` in the sweep, plus keyboard-only, reflow, text-size and target-size tests | CLI-136 names axe on the web build as the release gate. |
| WASM bindings and types | `wasm-bindgen`, with `tsify` and `serde-wasm-bindgen` | Record 6 lists `wasm-bindgen`. The other two are Rust dependency requests in WP-235; they generate the TypeScript declarations (see [The contract](#the-contract-two-ports)). |

What is deliberately absent: a router library (the client's addresses are
a closed list, and inbound links are parsed by the core, SEC-CLI-025), a
state library (React's own `useSyncExternalStore` over small stores), a
schema library (responses are decoded by the core, SEC-CLI-021), an
internationalisation library (a typed catalogue and the browser's `Intl`),
an icon package and a font package (both are files in the repository, see
CP-004 and CP-006), a list virtualisation package (React Native for Web's
own list is tried first, against the DIS-100 budget), a UI component
library, a CSS framework, any analytics, error-reporting or telemetry
package (SEC-CLI-027), an IndexedDB stand-in for tests (storage is behind
a browser adapter and proved in real browsers), `eslint-plugin-react`
(above), `@types/react-native-web` (below) and `@vitejs/plugin-react` (it
adds fast refresh only; the build does not need it, unverified).

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
  publishing trust dropped (SEC-SUP-034). pnpm itself is installed
  outside the lockfile, so the same seven-day rule is applied to it by
  hand, and CP-001's check compares its pinned version's publication date.
- Every direct dependency of every `package.json` has one sorted line in
  `supply-chain/js-direct-deps.toml` with its reason (SEC-SUP-035). The
  check WP-124 wrote (`xtask js-deps`) reads `dependencies`,
  `devDependencies`, `optionalDependencies` and `peerDependencies` of
  every `package.json` outside `node_modules` and `target`, fails on a
  name the list lacks and on a listed name no manifest uses. So:
  - **The package that adds a dependency adds its line in the same pull
    request**, as the backend plan's shared-file table already says. The
    file is under code-owner review, so the human check still happens
    before the merge. A line cannot be added ahead of the manifest, or
    after it.
  - **Workspace members** are named in each other's manifests with
    `workspace:*`. The check has to skip a `workspace:` version whose
    name is a workspace member; that is a
    [request to WP-124](#requests-to-the-backend-plan). Until it is
    applied, every tool stays in the one root manifest, member manifests
    name no dependency at all, and members import each other through the
    TypeScript path map.
  - **Registry packages appear only in `dependencies` and
    `devDependencies`.** `peerDependencies` and `optionalDependencies`
    may name workspace members only, and CP-001's check fails otherwise.
  - **Fixture manifests hold no dependencies.** The install canary's
    `package.json` and the fixture projects of CP-002 name no package,
    so the check passes over them. They live under `clients/tools/`.
- Registry signatures, and provenance where present, are verified for
  every installed package (SEC-SUP-036).
- Every package that ships in the bundle uses a licence on the project
  allow-list (SEC-SUP-029). Development tools are checked too, and listed
  when their licence is not on the list.
- A deny-list of analytics, advertising, crash-reporting and tracking
  packages fails the gate if any appears in the lockfile (SEC-CLI-027).
- **Adding a dependency is a request.** It is written in the pull request
  with the baseline's checklist
  ([supply-chain-and-release.md](../security/supply-chain-and-release.md#5-dependency-policy)):
  the exact name, what it is for, what was considered instead, its
  licence, its maintainers and its transitive packages. A human confirms
  that the package exists and is the intended one before the merge
  (AGENTS.md, SEC-STD-035). An agent never adds a package to make a test
  pass.

The packages below are everything R1 is planned to need. Each name was
looked up on the npm registry on 2026-10-03 and exists under that exact
name with the licence shown. The version is the newest release in the
chosen line that was at least seven days old that day, with its
publication date, so the list obeys the plan's own age rule. Whether
these versions work together is unverified until CP-001 and CP-002
install them, and those two packages settle the exact pins.

**Shipped in the bundle (3):**

| Package | Pin | Published | Licence | Reason |
|---|---|---|---|---|
| `react` | 19.3.0 | 2026-09-09 | MIT | The UI library (record 1, decision 8). |
| `react-dom` | 19.3.0 | 2026-09-09 | MIT | The web renderer; `react-native-web` needs it. |
| `react-native-web` | 0.21.3 | 2026-09-25 | MIT | React Native primitives in a browser, so the screens are the ones the R2 native apps reuse. It brings eight transitive packages. |

**Development only (20):**

| Package | Pin | Published | Licence | Reason |
|---|---|---|---|---|
| `typescript` | 6.0.3 | 2026-04-16 | Apache-2.0 | The compiler and type checker. |
| `@types/react` | 19.3.0 | 2026-09-09 | MIT | Types for React. |
| `@types/react-dom` | 19.3.0 | 2026-09-09 | MIT | Types for the web renderer. |
| `@types/node` | 24.19.0 | 2026-09-25 | MIT | Types for the configuration files and the checks that run in Node 24. |
| `vite` | 8.3.1 | 2026-09-24 | MIT | The bundler and the dev server. |
| `vitest` | 4.1.11 | 2026-08-18 | MIT | The test runner. Its stated range includes Vite 8. |
| `@vitest/coverage-v8` | 4.1.11 | 2026-08-18 | MIT | Coverage with thresholds. |
| `jsdom` | 30.1.1 | 2026-09-22 | MIT | A DOM for component tests. |
| `@testing-library/react` | 16.3.3 | 2026-08-27 | MIT | Renders components and queries them by role and name. |
| `@testing-library/dom` | 10.4.2 | 2026-09-13 | MIT | A required peer of `@testing-library/react`. |
| `@testing-library/user-event` | 14.6.7 | 2026-09-02 | MIT | Real keyboard and pointer sequences in component tests. |
| `@stryker-mutator/core` | 10.0.0 | 2026-08-14 | Apache-2.0 | Mutation testing. |
| `@stryker-mutator/vitest-runner` | 10.0.0 | 2026-08-14 | Apache-2.0 | Runs the mutants with Vitest. |
| `@stryker-mutator/typescript-checker` | 10.0.0 | 2026-08-14 | Apache-2.0 | Discards mutants that do not type-check. |
| `eslint` | 10.11.0 | 2026-09-18 | MIT | The linter. |
| `typescript-eslint` | 8.70.1 | 2026-09-21 | MIT | TypeScript parsing and rules for ESLint. Its stated range includes ESLint 10 and TypeScript below 6.1. |
| `eslint-plugin-react-hooks` | 7.1.1 | 2026-04-17 | MIT | The hook rules; a broken dependency list is a real bug class. Its stated range includes ESLint 10. |
| `prettier` | 3.9.9 | 2026-09-23 | MIT | Formatting. |
| `@playwright/test` | 1.63.0 | 2026-09-04 | Apache-2.0 | Browser tests in three engines. It downloads browser builds from its own host, outside npm; the version pin fixes which builds. |
| `@axe-core/playwright` | 4.13.0 | 2026-08-11 | MPL-2.0 | The accessibility checks CLI-136 names. Development only, so it is in no shipped artefact; its licence still needs a line on the allow-list review. |

pnpm itself is pinned to 12.7.0 (published 2026-09-25, MIT), installed by
exact version with a checksum.

**Not `@types/react-native-web`.** It declares a dependency on
`react-native`, which would pull React Native itself, with Metro and its
command-line packages, into the lockfile: the tree record 12 avoids. CP-006
writes one small declaration file for the primitives `ui` uses
(`ui/src/react-native.d.ts`), checked by the component tests that render
each primitive.

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

`scripts/gate.sh`, `.github/workflows/ci.yml` and `.github/CODEOWNERS`
belong to the integrator and to code owners. Client packages do not edit
them. CP-001 and CP-002 file these requests in their pull requests, and
the integrator applies them between merges.

1. **`scripts/gate.sh`** gains a web block, which runs only what
   `clients/package.json` defines, so the script stays the one definition
   of done:

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
2. **Build order.** Once the facade exists (WP-235), the web block builds
   in this order, because each step needs the one before: the facade for
   `wasm32` and its bindings; the client bundle, which holds the WASM
   module; from CP-056, the server with that bundle embedded (WP-072);
   then the contract run and the flows against that server (CP-055,
   CP-058). The Rust half of the gate does not need the bundle: without
   it the server serves WP-072's stand-in.
3. **Fixed limits for a shared machine.** The build machine has 24 cores
   capped at 16 and runs other agents. The web gate therefore sets,
   in the committed configuration and not from the machine it runs on:
   StrykerJS `concurrency` 4, Vitest at most 4 workers, Playwright 2
   workers, and a fixed StrykerJS timeout (`timeoutMS` 60000 with
   `timeoutFactor` 3), so load does not turn a slow run into a false
   timeout. The time budget for the whole web block is 20 minutes on that
   profile for a diff-scoped run and 45 minutes for a full run. When a
   full run passes its budget, StrykerJS is split by package into
   parallel CI jobs; nothing is excluded to make it fit.
4. **`ci.yml`** installs Node 24 and pnpm by exact version with
   checksums, the `wasm32-unknown-unknown` target and the pinned
   `wasm-bindgen` command-line tool, and the Playwright browser builds
   for the pinned version. **Proposal:** the Rust and web halves of the
   gate run as two required jobs from the same script (`GATE_PART=rust`
   and `GATE_PART=web`), so neither waits for the other; locally the
   script runs both. No new branch pattern is needed, because client
   packages merge into the server wave branches.
5. **`.github/CODEOWNERS`** gains `/clients/pnpm-lock.yaml`,
   `/clients/pnpm-workspace.yaml`, `/clients/package.json`, the lint, test
   and mutation configuration files, `/clients/tools/` and
   `/crates/gunmetal-wasm/`, because they define the gate or cross a
   trust boundary (SEC-SUP-005).
6. **Dependabot** gains the npm ecosystem for `/clients` with a cooldown
   of at least seven days (SEC-SUP-028).
7. **CodeQL** already has to cover JavaScript and TypeScript
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
  positions, building user events and keeping their clock, parsing
  inbound links, fragments and QR payloads into typed routes
  (SEC-CLI-025), turning an artwork colour into surfaces that pass the
  contrast check, the link and URL filter, text normalisation, and
  decoding what the server sends (SEC-CLI-021). Its shape is the shape
  of the WASM facade.
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
| `CorePort` | `fake-server`'s fixture core, for reading the library only, until WP-088 holds the library in WASM | `core-wasm`: the `gunmetal-wasm` build | The composition root, part by part |

Component tests use neither: they pass a scripted double that returns
what the test wrote down and holds no rule.

### How types cross from Rust to TypeScript

`wasm-bindgen` alone writes precise declarations only for simple
exported items. The core's types cannot carry its attribute (record 6
keeps bindings out of the core), and rich values such as a queue
operation would cross as an untyped value. So the mechanism is this, and
record 12 decides it:

1. **Mirror types in the facade.** For each core type that crosses,
   `gunmetal-wasm` holds a mirror struct or enum with a conversion from
   and to the core type. The conversion takes the core value apart field
   by field with no catch-all, so a field the core renames, retypes, adds
   or removes fails to compile in the Rust gate.
2. **Declarations are generated from the mirrors.** The mirrors derive
   `serde`'s traits and `tsify`'s, values cross through
   `serde-wasm-bindgen`, and the build writes a TypeScript declaration
   for every mirror and every export. `tsify` and `serde-wasm-bindgen`
   are Rust dependency requests in WP-235, with the owner's approval
   (`tsify` 0.5.8, MIT or Apache-2.0, crates.io, github.com/madonoharu/tsify;
   `serde-wasm-bindgen` 0.6.5, MIT, github.com/RReverser/serde-wasm-bindgen;
   both seen on crates.io on 2026-10-03; `tsify-next` is a different
   crate and is not the one meant).
3. **The declarations are committed and drift-checked.** They live in
   `crates/gunmetal-wasm/types/`. An xtask command regenerates them and
   the gate fails when the committed copy differs, as it does for
   `openapi.json`. A change to a type therefore shows in review as a
   diff of the declaration.
4. **The client uses only those declarations.** `ports` re-exports them;
   lint fails on a hand-written interface or type alias in `ports` that
   describes a value crossing a port. A mirror that changed fails `tsc`
   wherever the client used the old shape.

WP-235 proves all four steps on types already on `main` (the link filter
and text types of WP-005, the IDs and problem codes of WP-006) before any
other slice of the facade is written.

**Types with no Rust definition yet.** Some `ServerPort` answers (most of
C3 and C4) belong to server packages that are not written. For those
only, the surface package writes a provisional type in
`ports/src/provisional/`, in a file that names the backend package that
will define it. When that package merges, the facade's decoder for its
response (WP-088) brings the generated type, and the provisional file is
deleted. CP-058 is not done while the directory holds a file.

### Where the protocol types come from

The client does not define the protocol. These Rust packages do, and the
facade packages bring their types to TypeScript:

| Types | Defined by | Server wave | Reaches the client through |
|---|---|---|---|
| Public IDs, the problem catalogue | WP-006 | 0 | WP-235 |
| Text normalisation, typed values, the link filter | WP-005 | 0 | WP-235 |
| Lyrics: the timed-line model | WP-021 | 1 | WP-236 |
| The queue document and its operations | WP-025 | 1 | WP-236 |
| Shuffle modes | WP-026 | 1 | WP-236 |
| The gain decision | WP-028 | 1 | WP-236 |
| The player state and its events | WP-030 | 1 | WP-236 |
| Catalogue records: track, album, artist | WP-040 | 1 | WP-236 |
| User events (plays, loves) and their clock | WP-034 | 1 | WP-237 |
| Inbound links as typed routes | WP-089 today; a split is requested | 3 today | WP-237 once split |
| Tint surfaces from an artwork colour | No package yet; requested | | WP-237 once it exists |
| Wire frames and version negotiation | WP-039 | 1 | WP-088 |
| Search results | WP-054 | 2 | WP-088 |
| The playback decision and track details | WP-055 | 2 | WP-088 |
| Home rows | WP-059 | 2 | WP-088 |
| Sync snapshot and delta | WP-084 | 3 | WP-088 (frames are opaque to TypeScript) |
| Route responses | The wave 3 and 4 route packages, listed in `openapi.json` (WP-118, wave 2) | 2 to 4 | WP-088's response decoding |

On 2026-10-03 the wave 0 packages were on `main`, the wave 1 packages
were merging into `wave-1`, and nothing later had started.

### What keeps the fake and the real server in step

Four checks, each arriving as soon as the Rust side it needs exists:

1. **From C0: the problem catalogue.** The client's list of problem
   codes is the generated type from WP-235, and a test fails when a code
   has no sentence in the message catalogue.
2. **From WP-235: generated types.** As described above. What this
   catches: a core field renamed, retyped, added or removed (the facade
   stops compiling); a mirror changed without regenerating (the drift
   check); a client use of the old shape (`tsc`). What it does not catch:
   a change in what a function does with the same types, which is what
   the conformance suites are for.
3. **From CP-053 (server wave 2): the route table.** A test reads the
   committed `crates/gunmetal-server/openapi.json` and fails when a call
   the HTTP adapter makes names a route, method or field that is not in
   it. The fake is checked by the same table: every `ServerPort` method
   maps to one route.
4. **From CP-055 (server wave 3): the contract run.** One conformance
   suite, with literal expectations, runs against three things: the fake
   with the hand-written demo library the owner clicks, the fake
   replaying frames recorded from the server, and a real `gunmetal`
   process. The real server and the recording use WP-119's small
   synthetic library with a fixed seed and an injected clock. Recordings
   are compared after decoding, value by value, never by a hash of the
   bytes, because IDs, clocks and cursors differ between runs. The run is
   a step of `scripts/gate.sh`, so it is a required check, not a side
   workflow. It grows with the server: CP-055 covers what wave 3 serves,
   CP-057 adds streams and stream limits, and CP-058 adds the wave 4 to
   6 answers and every failure switch the fake has. Until CP-058 is
   done, the parts of the fake it has not reached are checked only by
   types and review, and each surface package says so.

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
  holds the hostile-metadata corpus. The records are JSON typed by the
  generated catalogue types (WP-236). The audio is short generated tones
  in WAV, and the covers are small generated PNG gradients; both come
  from a checked-in script and are listed in a SHA-256 manifest
  (SEC-SUP-032).
- **Stage B, from CP-055.** Sync frames and responses recorded from the
  real server over WP-119's synthetic library, replayed for tests that
  do not need a server process. The hand-written library stays as the
  demo's data, and the contract run covers it too.

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
list already in order). It verifies no credential and enforces no
permission, so no test of the fake proves a security property of the
server.

**How it stays out of the product.** Only `apps/demo` imports it. The
production build fails if the bundle holds any module of `fake-server` or
`fixtures` (CP-056), and the demo shows a permanent "Demo data" label.

## Core logic before the WASM facade

WP-088, the facade, was one package in server wave 3, because the parts
of the core it wraps last (search, Home rows, the decision) are wave 2.
The client needs the queue, shuffle, gain and player rules in C1. The
plan's answer is that nothing stands in for the core except the core.

- **`CorePort` is written first** (CP-005), as the list of what the
  client needs from the facade.
- **The facade has one owner, the backend plan, and is built in slices
  as the core's modules merge.** `crates/gunmetal-wasm` is Rust, so its
  packages are backend packages on the server wave branches:

  | Package | Wave | What it exports | Needs |
  |---|---|---|---|
  | WP-235 | 1 | Creates the crate. The type mechanism above, the lint exception below, the link filter and text normalisation, IDs and problem codes. | WP-005, WP-006, WP-008 (wave 0) |
  | WP-236 | 2 | The queue, shuffle, gain, the player state, lyrics, and the catalogue record types. | WP-235; WP-021, WP-025, WP-026, WP-028, WP-030, WP-040 (wave 1) |
  | WP-237 | 2 | User events: building an event with its ID and clock, receiving a clock, and the event sink that drops events in private mode. If the backend plan accepts the two requests, also the inbound-link parser and the tint rule. | WP-235; WP-034 (wave 1) |
  | WP-088 | 3 | What is left: sync frames into the in-memory library and its reads, search, the decision and track details, Home rows, response decoding, the `wasm32` build job and the size check. | As before, plus WP-235 |

  Each export is a direct call into the core with conversion only, which
  is what WP-088 always specified. The backend plan carries these
  entries; this pull request adds them there.
- **`unsafe` (owner decision 34 of the backend plan), settled as a
  technical answer.** The core keeps `unsafe_code = "forbid"` with no
  exception. The facade crate alone may carry the narrowest lint
  exception `wasm-bindgen`'s generated code needs, with a stated reason;
  hand-written `unsafe` stays refused there by an xtask check that fails
  on the keyword anywhere in the crate's source, and the exception is on
  the xtask exception list. `wasm-bindgen`'s generated items are compiled
  only for `wasm32`, so the Rust gate on the host would never see the
  lint fire; WP-235 therefore builds the crate for `wasm32` under the
  lint in the gate. **No other facade slice, and no client package that
  depends on one, starts until WP-235's `wasm32` build passes under that
  lint with the link filter exported.**
- **Until a slice exists, its part of `CorePort` has no running
  implementation and no types.** Screens that need it are not started;
  nothing is written in TypeScript to cover for it.
- **Reading the library is the one part the fixture answers.** Until
  WP-088 applies real sync frames, album, artist, track and playlist
  reads are lookups in the fixture's JSON, and every ordered list comes
  from the fixture already ordered. A lookup by ID is not a rule. When
  WP-088 lands, CP-054 replaces the lookups with the facade's reads and
  deletes the fixture core.

The risk is schedule, not rework: C1's first clickable milestone needs
WP-236, and WP-236 needs six wave 1 packages merged. If one slips, the
milestone waits; the plan does not fill the gap with TypeScript.

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
badge and track details show (MUS-099, MUS-236). Until the decision
engine reaches the browser (WP-088), the demo plays every fixture track
directly and shows no badge.

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
clamp, and the facade gives the factor to set; the client applies the
number and never computes one (MUS-084, MUS-085, MUS-087 to MUS-089).
Media is same-origin, so the graph can read it.

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

**Plays and loves are user events**, with a client-made event ID and a
hybrid logical clock, both the core's (WP-034). They are made in one
place, the event sink (CP-060), which arrives in C2 with WP-237. The
first clickable build records nothing: it plays, and that is all.

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

**Branches: there are no client wave branches.** The project builds on
one branch per server wave, and the owner merges one pull request per
wave into `main` (D-01). Client packages follow that exactly:

- A client package branches as `cp/cp-NNN` from the server wave branch
  that is open when it starts, and its pull request merges back into
  that same branch, through that wave's integrator, once the gate
  passes. The wave's one pull request into `main` carries the server
  packages and the client packages that merged into it.
- A package starts on the earliest open wave branch that holds
  everything it depends on. C0 needs only `main` and WP-235, so it
  merges into `wave-1`. The first clickable milestone needs WP-236, so
  its packages merge into `wave-2`. Packages that need WP-088 merge into
  `wave-3`, and so on. Each wave branch starts on top of the one before
  (the register's "Building order" answer), so a later branch always
  holds the earlier client work and the facade crate.
- A server wave closes when its server packages have merged and the full
  gate passes. A client package still open then retargets to the next
  wave branch; it does not hold the wave open.
- The facade crate is on the same branch as the client code that calls
  it, because both merge into the same wave branch. Nothing is carried
  between two chains of branches.

Client waves C0 to C5 are therefore not branches. They say what the
owner can test and in what order the work unlocks. Agents never merge
into `main`.

**Registry files.** These hold one sorted line per entry and nothing
else. Any package may add its line; a conflict is resolved by keeping
every line and re-sorting.

| File | Owner | What other packages may do |
|---|---|---|
| `clients/package.json`, `clients/pnpm-workspace.yaml`, the lint, test and mutation configuration | CP-001 and CP-002, then the integrator | A package whose dependency request is approved adds its one line to the manifest in its own pull request. A rule change is a request. |
| `clients/pnpm-lock.yaml` | Nobody | On a conflict, take either side and run `pnpm install --lockfile-only`, never a plain upgrade. |
| A package's own `package.json` | The package that creates it | Add one `workspace:*` line for another workspace member, sorted. Never a registry dependency. |
| `supply-chain/js-direct-deps.toml` | WP-124 | The package that adds a direct dependency adds its sorted line with the reason, in the same pull request (SEC-SUP-035). |
| Every `index.ts` | Nobody; these are registries | Export lines only, never code. |
| `clients/packages/ui/src/routes.ts` | CP-009 | One line per address: its fixed path, its surface and whether it needs a session or an admin session. |
| `clients/packages/ui/src/actions.ts` | CP-024 | One line per action in player.md's one action list. |
| `clients/packages/ui/src/messages/en/<surface>.ts` | The surface's package | Not shared: each surface owns its own message file. `messages/en/index.ts` is a registry. |
| `clients/e2e/screens.ts` | CP-003 | One line per screen or state the sweeps must visit. |
| `clients/packages/ports/src/` | CP-005 | A new method is the interface-change rule: its own small package, agreed with the backend package that serves it. |
| `clients/packages/ports/src/provisional/` | Nobody; a registry directory | One file per answer whose Rust type does not exist yet, naming the backend package that will define it. |
| `clients/apps/web/src/compose.ts`, `clients/apps/demo/src/compose.ts` | CP-003 and CP-012 | One line to register a store, a surface module or a control for a slot. |
| `crates/gunmetal-wasm/` | The backend plan: WP-235, WP-236, WP-237, WP-088 | Nothing. A client package that needs a new export asks for it under the interface-change rule. |

## Waves at a glance

| Wave | Packages | Count | What it produces |
|---|---|---:|---|
| C0 | CP-001 to CP-012 | 12 | The workspace and its dependency rules, the TypeScript gate, the app skeleton proved under the production policy, tokens and themes, the two ports, the kit, overlays, messages, the shell and routing, the accessibility and layout harness, the demo library, the fake server and the demo build |
| C1 | CP-013 to CP-022 | 10 | The core in the browser; stores; lists and grids; Library, album and artist pages; the audio engine; the playback controller; the now-playing bar; the queue |
| C2 | CP-023 to CP-034, CP-060, CP-061 | 14 | The demo library inside the core; plays and loves; artwork tints; the context menu; the full-screen player, lyrics, track details and the quality badge; media controls; Home; search and genre pages; playlists; history and private listening; the status layer and every player state; gapless playback through Media Source |
| C3 | CP-035 to CP-043, CP-062 | 10 | Inbound links through the core; sign-in with passkeys, the personal-or-shared question and the wipe; first-run setup; invitations; approving a browser; step-up; settings; account, recovery and each person's own security pages |
| C4 | CP-044 to CP-052 | 9 | The admin frame and dashboard; libraries; health and trash; users; security and server settings; network; backups and updates; activity, alerts and diagnostics; the security log |
| C5 | CP-053 to CP-059 | 7 | The HTTP adapter, the sync client and the device copy, the contract run, the production bundle handed to the server, real streaming, the R1 flows end to end in a browser, the browser list and the release accessibility script |

62 packages build the R1 web player. The order between waves, read from
the Depends-on fields:

- C0 has one fixed order at its start: CP-001, then CP-002, then
  everything else. CP-005 also needs WP-235, and CP-011 and CP-012 need
  WP-236.
- C1 needs C0 and WP-236.
- C2 and C3 each need C1 and do not need each other. Inside C3, the
  screens that read a link (setup, invitation, pairing) need CP-062.
- C4 needs C3: every admin surface opens through the step-up prompt and
  the settings pages (CP-040, CP-041), libraries reuse the welcome
  screen's folder picker (CP-037), and the audit anchor uses the storage
  layer (CP-036). C4 does not need C2.
- C5 is not one block. CP-053 and CP-056 need only C0 and their server
  packages. CP-054 needs CP-036 (C3) and CP-023 (C2). CP-055 needs
  CP-054. CP-057 needs CP-034 (C2). CP-058 and CP-059 need everything.

## Client waves, server waves and what the owner can test

"Leaves the fake" means the wave's screens run against a real Gunmetal
server with the owner's own music, with no fake code in the page. "Merges
into" is the earliest server wave branch that holds what the wave needs.

| Client wave | Merges into | Needs from the server before it can be built | Leaves the fake when | What the owner can test at the end of the wave, on the demo |
|---|---|---|---|---|
| C0 | `wave-1`; CP-011 and CP-012 into `wave-2` | Wave 0 on `main` (it is). WP-235 (wave 1) for the generated types, and WP-124 (wave 1) for the dependency list file. The demo library and the fake (CP-011, CP-012) are typed by the catalogue types, so they need WP-236 (wave 2). | Not applicable: nothing here talks to a server. | Open the demo in a browser. See the Gunmetal frame: the sidebar with Home, Search and Library, the dark, light, black and high-contrast themes, and the phone layout when the window is narrow. Move through it with the keyboard alone. Nothing plays yet. |
| C1 | `wave-2` | WP-236 (wave 2), which needs WP-021, WP-025, WP-026, WP-028, WP-030 and WP-040 (wave 1). | Server wave 3: sync (WP-084), the facade's library (WP-088), streams (WP-082), the queue service (WP-085). Artwork needs wave 4 (WP-103). | **The first clickable player.** Browse the made-up library by artist, album and song. Open an album. Press Play and hear it. Use the bar at the bottom: pause, skip, see what is playing. Open the queue, reorder it, remove a track, turn on shuffle and repeat. Nothing is remembered yet: no hearts, no history. |
| C2 | `wave-2` for what needs WP-237 (hearts, plays, history, private listening, the context menu, the full-screen player, lyrics, playlists, media keys); `wave-3` for what needs WP-088 (search, Home, track details and the badge) or the packager's fixtures (gapless, WP-056) | WP-237 (wave 2), which needs WP-034. WP-088 (wave 3), which needs WP-054, WP-055 and WP-059. | Server wave 3: listening activity (WP-086), playlists (WP-093), the event channel (WP-083). Wave 4: stream limits (WP-104), the packaging route (WP-105), history deletion (WP-133). | Everything a listener does. Search as you type. A Home page with what you played. Right-click anything for Play next and Add to queue. Love a track. The full-screen player, lyrics that follow the song, what format is playing and why. Make a playlist. See your history and remove a play. Turn on a private session. Use the keyboard's media keys. Albums play without a gap between tracks. |
| C3 | `wave-2` | WP-237 (wave 2) for the link parser, if the backend plan accepts that request; otherwise the invitation, pairing and setup-link screens wait for WP-089 (wave 3) and merge into `wave-3`. | Server wave 3: setup (WP-080), passkeys (WP-081), browser pairing and sign-out (WP-120), accounts and devices (WP-087), invitations (WP-094), recovery codes (WP-063). Wave 4: recovery and step-up (WP-106), data export (WP-108), account deletion (WP-133). | Walk through first-run setup with a made-up setup code. Sign in with a passkey, answer "Is this your own device?", and sign out and see that nothing is left behind. Open an invitation link. Approve another browser. Change the theme and the sound settings. See your devices and remove one. |
| C4 | The wave branch open when C3 has merged | Nothing more from the server. | Server wave 3: library administration (WP-099), activity (WP-100), alerts (WP-097), backups (WP-090), network settings (WP-073, WP-132), updates (WP-074), users (WP-094). Wave 4: HTTPS by ACME (WP-101), the scan (WP-102). Wave 5: health and trash (WP-110, WP-111). Wave 6: the doctor and security summary (WP-116). | The admin screens with made-up data. Add a music folder and watch a pretend scan. Read the health report. Invite someone and choose their libraries. Look at backups, updates, alerts and the security log. |
| C5 | `wave-3` to `wave-6`, package by package | The route table (WP-118, wave 2), the routes in waves 3 and 4, and for the full flow tests the whole server, beside WP-117 in wave 6. | This wave is the move. | **The real thing.** Start the Gunmetal server, claim it, add your own music folder, and use every screen above with your own library, served by the server itself. |

The register's answer says the player moves to the real server "as the
server waves land". For listening that is waves 2 to 4. The admin screens
finish later, because their server packages are in waves 5 and 6.

## The first clickable milestone

The milestone the owner asked for is: browse a library, open an album,
play a track, with the player bar and the queue. It is kept as small as
it can be. It is the end of wave C1 without the artist page, and it
records nothing: hearts and play history arrive in C2.

It needs exactly these 21 client packages:

- **All of C0:** CP-001 (workspace), CP-002 (gate), CP-003 (skeleton
  under the policy), CP-004 (tokens), CP-005 (ports), CP-006 (kit),
  CP-007 (overlays), CP-008 (messages), CP-009 (shell and routing),
  CP-010 (accessibility harness), CP-011 (demo library), CP-012 (fake
  server and demo build).
- **From C1:** CP-013 (the core in the browser), CP-014 (stores), CP-015
  (lists and grids), CP-016 (Library), CP-017 (album page), CP-019
  (audio engine), CP-020 (playback controller), CP-021 (now-playing
  bar), CP-022 (queue).

From the backend it needs exactly these packages merged:

- **WP-235** (wave 1), the facade crate with the type mechanism and the
  lint answer proved. It needs only WP-005, WP-006 and WP-008, which are
  on `main`.
- **WP-236** (wave 2), the facade's first slice, and the six wave 1
  packages it wraps: WP-021 (lyrics), WP-025 (queue), WP-026 (shuffle),
  WP-028 (gain), WP-030 (player state) and WP-040 (catalogue types).
- **WP-124** (wave 1), for the dependency list file, with the small
  change this plan asks of it.

It does not need WP-034 (user events), WP-037 (palette), WP-039 (the wire
codec), WP-088 or any server package: no server process, no database and
no network. What that leaves out of the milestone, by design: hearts and
play counts (C2, CP-060), the quality badge and track details (C2,
CP-027), artwork tints and coloured placeholders (C2, CP-061; tiles show
a neutral placeholder until then), and addresses for single items (the
album page is reached by clicking, and Back and reload return to it from
the browser's own history state; a link to an album is R1.2, CLI-034).

An earlier, smaller milestone falls out of CP-001 to CP-010 alone, on
`wave-1`: a real screen with the frame, the themes and the phone layout,
and nothing to play.

## Wave C0: foundations

In the **Owns** fields, a path that starts with a package name is under
`clients/packages/`: `ui/src/kit/` means `clients/packages/ui/src/kit/`.
Every surface package also owns its own message file,
`ui/src/messages/en/<surface>.ts`, and adds its lines to the registries;
the fields do not repeat that.

CP-001 merges first and CP-002 second. Everything else in C0 starts when
CP-002 has merged. CP-005 also waits for WP-235, and CP-011 and CP-012
for WP-236.

### CP-001 Workspace, package manager and dependency policy

- **Wave** C0 · **Size** S · **Depends on** WP-124 (wave 1), for
  `supply-chain/js-direct-deps.toml` and `xtask js-deps`, with the change
  this plan asks of it.
- **Owns** `clients/package.json`, `clients/pnpm-workspace.yaml`,
  `clients/pnpm-lock.yaml`, `clients/tsconfig.base.json`,
  `clients/tools/gate/install/`.
- **Serves** CLI-001 (the web client exists as a workspace).
- **Security.** Boundaries TB12; threats TM-T41, TM-T42. Verifies
  SEC-SUP-033, SEC-SUP-034, SEC-SUP-035, SEC-SUP-036, SEC-CLI-018,
  SEC-CLI-027, and the JavaScript half of SEC-SUP-029.
- **Builds.** The pnpm workspace with the baseline's settings: a minimum
  release age of seven days, no drop in publishing trust, no exotic
  sources, and an empty build allow-list. The pinned Node and pnpm
  versions. A check, in TypeScript under `tools/gate/install/`, of those
  settings, of the lockfile's sources, of pnpm's own age, and that no
  manifest names a registry package under `peerDependencies` or
  `optionalDependencies`; WP-136 runs the same check in the release
  workflow instead of writing a second one. The install canary, whose
  fixture manifest names no dependency. The signature check. The licence
  check against the project allow-list. The deny-list of tracking
  packages. Each tool's line in `supply-chain/js-direct-deps.toml`, added
  in this pull request with the manifest that uses it. The gate requests
  in [The gate and CI](#the-gate-and-ci-requests-to-the-integrator).
- **Tests.** The settings check fails on a settings file with each rule
  removed or loosened in turn, on a lockfile fixture holding a git
  dependency, a tarball URL and a second registry, on a non-empty build
  allow-list, and on a manifest fixture with a registry name under
  `peerDependencies`; each failure names the rule. After the canary
  install neither marker file exists. The licence check fails on a
  fixture listing a banned licence and on a missing one. The deny-list
  check fails on a fixture lockfile naming a listed package.
- **Done when.** The settings check and the canary pass on the committed
  workspace and fail for each loosened fixture.

### CP-002 The TypeScript quality gate

- **Wave** C0 · **Size** M · **Depends on** CP-001.
- **Owns** `clients/eslint.config.js`, `clients/prettier.config.js`,
  `clients/vitest.config.ts`, `clients/stryker.config.json`,
  `clients/tools/lint/` (the project's own lint rules),
  `clients/tools/gate/report/` (the mutation report check),
  `clients/packages/canary/`.
- **Serves** The testing rules in CONTRIBUTING.md. No feature row of its
  own.
- **Security.** Boundaries TB4, TB12; threats TM-T07, TM-T41. Verifies the
  lint halves of SEC-CLI-001, SEC-API-045, SEC-MED-057, SEC-HIS-027,
  SEC-TM-036 and SEC-STD-012, and SEC-API-050.
- **Builds.** The `gate` script and every step in it but the build and
  the browser suites, over exactly the set in
  [What is covered and mutated](#what-is-covered-and-mutated-and-what-is-not):
  the coverage `include` and Stryker's `mutate` are those three globs and
  nothing else, and neither configuration holds an exclude entry other
  than test files. The fixed worker counts, timeout and time budget of
  gate request 3. Lint rules as errors: the project's own `no-danger` and
  `jsx-no-script-url`, which do what the two React rules the baseline
  names do; bans on `innerHTML`, `outerHTML`, `insertAdjacentHTML`,
  `document.write`, `eval`, `new Function`, string timers and
  `javascript:` URLs; a `message` listener without an origin check;
  spread, `Object.assign` and deep merges of untrusted objects; every
  escape hatch in the ground rules table; a colour literal outside the
  tokens package; a literal string as interface text; physical layout
  properties where a logical one exists; a module-level object or array
  literal in the covered set; a hand-written type in `ports` for a value
  that crosses a port; and any branch, loop or arithmetic in a file under
  a `browser/` directory. The mutation report check. The canary package:
  one pure function, one component, one data table returned by a
  function, and one browser adapter with the browser test that calls it.
- **Tests.** Each check is checked to fail. One fixture file per banned
  construct produces exactly its rule's error. The report check, fed
  literal Stryker reports holding one survivor, one uncovered mutant,
  one ignored mutant, one timeout and one run-time error, fails on each
  and passes on a report of killed mutants only. A fixture project with
  one uncovered branch fails the coverage step; with one unformatted
  file, the format step; with one type error, the type step; with a
  branch in its adapter, the lint. A test reads the committed
  configuration and compares the globs, the thresholds and the worker
  counts with literals. The canary's run records how long a static
  mutant and a covered mutant each take, and the pull request reports
  both.
- **Done when.** `pnpm --dir clients run gate` passes on the canary,
  inside the time budget on the build machine, and fails for each planted
  fault.

### CP-003 Web app skeleton under the production policy

- **Wave** C0 · **Size** M · **Depends on** CP-002.
- **Owns** `clients/apps/web/` (the page, the entry, `compose.ts`, the
  Vite configuration, the `gunmetal-loader` policy, the
  unsupported-browser page), `clients/playwright.config.ts`,
  `clients/e2e/support/`, `clients/e2e/policy/`, `clients/e2e/screens.ts`.
- **Serves** CLI-001, CLI-002 (the unsupported-browser notice), CLI-150.
- **Security.** Boundaries TB1, TB2, TB4; threats TM-T07, TM-T08, TM-T41.
  Verifies SEC-API-044 (the client's half: the sweep), SEC-API-045 (the
  policy function), SEC-API-049, SEC-API-052, SEC-CLI-012, SEC-CLI-019,
  SEC-PRV-018, SEC-SUP-037.
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
  of `screens.ts` in three engines, and collects Chromium's script
  coverage for every file under a `browser/` directory, failing below
  100%. Until CP-053 the sweeps visit `apps/demo`, built with this same
  production configuration and served by the same test server under the
  same header; `apps/web` is swept as well from the start, for the
  shell and the unsupported-browser page. The dev server bound to
  loopback.
- **Tests.** In Chromium, Firefox and WebKit: the page renders with no
  `securitypolicyviolation` event; creating any other Trusted Types
  policy throws; a planted request to another origin fails the sweep; a
  planted inline style in markup fails it. Property tests of the policy
  function: only same-origin URLs under the asset prefix pass. With
  WebAssembly disabled, and on a cleartext non-loopback origin, the
  static page is shown and nothing else runs. The committed dev
  configuration names the loopback address literally. A scan of a
  fixture bundle finds a planted absolute URL.
- **Done when.** Its sweep passes under the policy in all three
  engines, or the owner has answered D-74's fallback.

### CP-004 Design tokens, themes and typeface

- **Wave** C0 · **Size** M · **Depends on** CP-002.
- **Owns** `tokens/`.
- **Serves** CLI-139, CLI-140, CLI-141; design-language sections 4, 6,
  7, 9 and requirements A1, A2, A12 and A18.
- **Security.** Boundaries TB4; threats TM-T07. Verifies SEC-CLI-012 (the
  font is a file in the bundle), SEC-PRV-018 (no stylesheet names another
  origin or a `data:` source).
- **Builds.** Every token in design-language, as JSON data files with
  typed accessors: colour
  for the Dark, Light, OLED black and High contrast themes, spacing, the
  type scale, shape, elevation, motion and the four width classes. The
  static stylesheets the policy requires: custom properties per theme,
  System following the operating system, high contrast when the system
  asks for more contrast, Windows forced colours, and the reduced motion
  set. Inter as one WOFF2 file with its licence record. A small
  stylesheet with no script for the pages the server renders itself
  (SUR-080, SUR-109), which WP-095 and WP-132 embed.
- **Not built here.** Artwork tints. The rule is the core's and one
  client module applies it (CP-061).
- **Tests.** Every token in every theme equals the literal value in
  design-language's table. The contrast pairs in its "Measured contrast"
  table are recomputed with a WCAG contrast function written in the test
  and compared with the literal ratios; text pairs meet 4.5:1, primary
  and secondary text 7:1. The generated stylesheet for one theme equals
  a literal expected text. No stylesheet holds a URL other than the font
  file's.
- **Done when.** Its tests find every token value and contrast ratio
  equal to design-language's.

### CP-005 Ports and conformance suites

- **Wave** C0 · **Size** M · **Depends on** CP-002; WP-235 (wave 1).
- **Owns** `ports/`.
- **Serves** CLI-022 (the read interface every screen uses); player.md,
  "What the player needs from the server and the core".
- **Security.** Boundaries TB4; threats TM-T07, TM-T62. Verifies
  SEC-STD-012 (the model's half), SEC-CLI-021 (every port method returns a
  value or a typed problem, never throws).
- **Builds.** `CorePort` and `ServerPort`, as described in
  [The contract](#the-contract-two-ports). `CorePort` is assembled from
  the facade's generated declarations, one re-export line per part, so
  it grows as the facade does and holds no hand-written copy of a Rust
  type; `ports/src/core.ts` is a registry. IDs and problems are the
  generated types. The map from each problem code to a message key. Data
  keyed by untrusted strings held in `Map`, never merged into objects.
  The conformance suites, exported as functions that take an
  implementation. The hostile-metadata corpus as literal payloads
  (script elements, event-handler attributes, `javascript:` URLs,
  bidirectional overrides, very long strings, lone surrogates). The
  written list of what the client needs from the facade.
- **Tests.** Every generated problem code has a message key, and the
  map names no code the generated type lacks. Each conformance suite
  fails against a double that gives one wrong answer. Payloads whose
  keys are `__proto__`, `constructor` and `prototype` pass through the
  model and leave `Object.prototype` unchanged. The lint from CP-002
  fails on a fixture that adds a hand-written type to `ports`.
- **Done when.** A component test can be written against the ports with
  scripted doubles and types that come only from the facade.

### CP-006 UI kit: text, controls and icons

- **Wave** C0 · **Size** M · **Depends on** CP-003, CP-004, CP-005.
- **Owns** `ui/package.json`, `ui/src/kit/`, `ui/src/icons/`,
  `ui/src/react-native.d.ts` (the declarations for the React Native
  primitives `ui` uses, in place of a types package).
- **Serves** CLI-135, CLI-138, CLI-142, CLI-159; design-language
  sections 6, 8 and 10 and requirements A4, A6 and A10.
- **Security.** Boundaries TB4; threats TM-T07, TM-T47. Verifies
  SEC-CLI-001, SEC-CLI-002, SEC-CLI-028, SEC-API-046, SEC-API-047,
  SEC-MED-057, SEC-MED-058, SEC-HIS-027, SEC-TM-036, SEC-STD-015.
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
- **Done when.** Its corpus, link, field and control tests pass.

### CP-007 Overlays, focus and notices

- **Wave** C0 · **Size** M · **Depends on** CP-006.
- **Owns** `ui/src/overlays/`.
- **Serves** CLI-138, CLI-142; design-language section 11 and
  requirements A5, A13, A14 and A21.
- **Security.** Boundaries TB4; threats TM-T07. Verifies SEC-CLI-001 (text
  inside notices and prompts).
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
- **Done when.** Its focus and announcement tests pass for every
  overlay.

### CP-008 Messages and the pseudo-locale

- **Wave** C0 · **Size** S · **Depends on** CP-006.
- **Owns** `ui/src/messages/catalogue.ts`, `ui/src/messages/format.ts`,
  `ui/src/messages/pseudo.ts`, `ui/src/messages/en/index.ts`.
- **Serves** Design-language requirement A20; CLI-139. Translations
  themselves are R1.2 (CLI-146).
- **Security.** Boundaries TB4; threats TM-T07. Verifies SEC-CLI-001 (a
  parameter is always text).
- **Builds.** A typed catalogue: each key has a template with named
  parameters, and a missing or misspelt parameter fails type checking.
  Plurals, numbers, dates and durations through the browser's `Intl`.
  The pseudo-locale build, which lengthens and accents every message so
  hard-coded and clipped strings show.
- **Tests.** A message with each plural form gives the literal expected
  text. A corpus payload passed as a parameter appears literally. The
  pseudo-locale of a literal input equals a literal output.
- **Done when.** Its tests format each literal message, and the
  pseudo-locale output equals its literal.

### CP-009 App shell, routing and width classes (SUR-001)

- **Wave** C0 · **Size** M · **Depends on** CP-007, CP-008.
- **Owns** `ui/src/shell/`, `ui/src/router/`, `ui/src/routes.ts`.
- **Serves** SUR-001; CLI-001, CLI-031, CLI-060, CLI-149, DIS-112.
- **Security.** Boundaries TB4; threats TM-T07, TM-T65. Verifies
  SEC-CLI-025 (the client's half: this router parses nothing).
- **Builds.** In-app navigation only. The addresses are a closed list of
  fixed, secret-free paths, one per destination (Home, Search, Library,
  Settings and so on), matched by exact comparison; a path that is not
  on the list is "not found". Which item a page shows (an album, an
  artist) is kept in the browser's history state, which the client wrote
  itself and reads back through the generated ID types; it is never
  taken from the address. So Back, Forward and reload return to the
  exact page and scroll position (DIS-112), and a link to an item is
  R1.2 (CLI-034), when the core's parser gains that route. The router
  never reads a fragment or a query string: an address that carries one
  is handed whole to the core's parser by CP-062, and until CP-062
  merges such an address shows "not found" and is not in the demo. The
  frame at each width class: sidebar, content, right pane and bar at
  wide; the pane over the content at expanded; a rail and a side sheet
  at medium; a bottom tab bar with the bar above it at compact. Home,
  Search and Library in fixed places, a slot for the bar, the right
  pane, the status indicator and the account entry. Panes resize and
  collapse by pointer and by keyboard. Sign-in and setup routes show
  nothing persistent.
- **Tests.** At 360, 800, 1200 and 1600 pixels wide the landmarks
  present, and their order, equal a literal list. Back restores a
  literal scroll offset and the item in the history state. An unknown
  path, a known path with a fragment and a known path with a query
  string each show "not found" and make no port call. A history state
  holding a malformed ID shows "not found".
- **Done when.** Its tests show three empty destinations reachable at
  every width, by pointer and by keyboard.

### CP-010 Accessibility and layout-contract harness

- **Wave** C0 · **Size** M · **Depends on** CP-003, CP-009.
- **Owns** `clients/e2e/a11y/`, `clients/e2e/layout/`.
- **Serves** SUR-000; CLI-031, CLI-135, CLI-136, CLI-138, CLI-139,
  CLI-140, CLI-142, MUS-113; design-language requirements A3 to A6, A8
  to A12 and A18.
- **Security.** Boundaries TB4; threats TM-T69. Verifies no requirement of
  its own.
- **Builds.** Sweeps over `screens.ts`: axe with no violation; a
  keyboard-only walk that reaches every control and is never trapped;
  reflow at 320 pixels with no sideways scrolling; text at 200%; reduced
  motion forced on; forced colours with the focus ring still visible;
  target sizes. The layout-contract helper, which compares the measured
  position and order of named controls with literal values. The
  completeness test: every line of `routes.ts` and every state fixture
  has a line in `screens.ts`.
- **Tests.** The completeness test fails for a planted route with no
  registry line. Each sweep fails on a planted page: an unnamed button, a
  focus trap, an overflowing row, a 20-pixel target, an animation that
  ignores reduced motion. The layout helper fails when a control moves
  by one pixel.
- **Done when.** Each sweep and the completeness test fail on their
  planted faults and pass on the shell.

### CP-011 Demo library and generated media

- **Wave** C0 · **Size** M · **Depends on** CP-005; WP-236 (wave 2),
  for the generated catalogue types.
- **Owns** `fixtures/`.
- **Serves** The owner's choice of 2026-10-03 (something to click);
  MUS-032 (a fixture of each R1 state of a track).
- **Security.** Boundaries TB4, TB12; threats TM-T07, TM-T41. Verifies
  SEC-SUP-030, SEC-SUP-032, SEC-API-046 (the hostile album).
- **Builds.** The stage A library described in
  [The fake server](#the-fake-server), as records typed by the
  generated catalogue types, with each ordered list written out by
  hand. A checked-in script that writes the tone files and cover images,
  and the SHA-256 manifest. Licence records for every file.
- **Tests.** Running the script twice gives byte-identical files that
  match the manifest. A tone file's header fields equal literal values.
  Every ID is unique and every reference resolves. The hostile album
  holds a corpus payload in every text field.
- **Done when.** The script's output matches the manifest, and every
  record type-checks against the generated catalogue types.

### CP-012 Fake server, fixture core and the demo build

- **Wave** C0 · **Size** M · **Depends on** CP-003, CP-005, CP-011.
- **Owns** `fake-server/`, `clients/apps/demo/`.
- **Serves** The owner's choice of 2026-10-03.
- **Security.** Boundaries TB4; threats TM-T15, TM-T69. Verifies
  SEC-CLI-019 (the demo server binds to loopback). It verifies nothing
  else: it holds no control, and no test of it proves the server.
- **Builds.** The in-memory `ServerPort`, the fixture core for library
  reads, the failure switches, the injected clock, and the demo
  composition root with its permanent "Demo data" label and a
  `pnpm --dir clients demo` script.
- **Tests.** The `ServerPort` conformance suite passes. Each switch
  produces its exact problem. A stream URL is refused after its expiry
  under the injected clock. A queue operation is passed to the injected
  `CorePort` and the fake stores what it returns, with no change of its
  own. The fixture core returns each list in the fixture's order.
- **Done when.** The conformance suite passes against it, and the demo
  build's start page renders the shell in the browser sweep.

## Wave C1: the first clickable player

### CP-013 The core in the browser

- **Wave** C1 · **Size** S · **Depends on** CP-005; WP-235 (wave 1),
  WP-236 (wave 2).
- **Owns** `core-wasm/`.
- **Serves** Record 1, decision 2; MUS-077, MUS-084, MUS-085, MUS-087 to
  MUS-089, MUS-116 to MUS-119, MUS-122, MUS-126, MUS-154, MUS-155 (each
  as the core's rule reaching the browser).
- **Security.** Boundaries TB4; threats TM-T62. Verifies SEC-CLI-021 (a
  value the core rejects comes back as a typed problem), SEC-CLI-002 and
  SEC-API-047 (the core's link filter is the one the browser uses).
- **Stop condition.** This package does not start until WP-235 has
  merged with its `wasm32` build passing under the facade's lint
  exception (see
  [Core logic before the WASM facade](#core-logic-before-the-wasm-facade)).
- **Builds.** The loader: one browser adapter that fetches and compiles
  the module from the server's own origin, and the code in `src/` that
  hands its exports to the composition root as `CorePort`. The module's
  exports already have the generated types, so a later slice of the
  facade (WP-237, WP-088) needs no change here: its exports appear when
  the module is rebuilt. No Rust.
- **Tests.** In Vitest, against the real module built from the branch:
  three "play next" picks play in the order chosen; a stale operation is
  rejected with the current version; the same seed gives the same
  shuffle order; a gain grid row gives its literal decision and factor;
  every legal player transition in player.md's diagram, and a rejected
  illegal one; the line for a literal lyric position; a `javascript:`
  link is refused. In three browser engines the adapter loads the module
  under the production policy. A missing module gives the typed
  "unsupported" problem.
- **Done when.** Its conformance tests pass against the real module in
  Node and in the three engines.

### CP-014 Stores: session and library

- **Wave** C1 · **Size** M · **Depends on** CP-005, CP-012.
- **Owns** `app/package.json`, `app/src/session/`, `app/src/library/`.
- **Serves** CLI-022, DIS-002, LIB-004, MUS-027, MUS-208.
- **Security.** Boundaries TB4, TB5; threats TM-T62. Verifies SEC-STD-012.
- **Builds.** Small stores with a subscribe function and the hooks over
  them: who is signed in, which library is selected, and the reads every
  page makes (an album, an artist, a list in a given order). A read
  never waits on the network. Loves are not here: they are user events
  (CP-060).
- **Tests.** With scripted ports: each hook returns the literal value
  for the port's answer; a problem becomes a typed error state; a change
  notifies each subscriber once.
- **Done when.** Its tests show a component reading an album through one
  hook with no port in its props.

### CP-015 Lists, grids, rows and tiles (SUR-023)

- **Wave** C1 · **Size** L · **Depends on** CP-007, CP-010, CP-014.
- **Owns** `ui/src/lists/`.
- **Serves** SUR-023 in R1: DIS-100, DIS-104, LIB-142, LIB-146,
  MUS-001, MUS-002, MUS-020, MUS-021, MUS-040, MUS-229. (Hearts and
  play counts, DIS-051, MUS-180 and MUS-182, reach these rows with
  CP-060.)
- **Security.** Boundaries TB4; threats TM-T07, TM-T09. Verifies
  SEC-CLI-001 (rows and tiles over the hostile album).
- **Builds.** The endless list and grid, drawing only what is on screen.
  The track row: title, every credited artist linked, duration, play
  count, a fixed place for the heart (empty until CP-060), the format
  badge, and the dimmed state with its reason. The tile, with artwork
  in the size the layout needs and a neutral placeholder (CP-061 colours
  it). The sort menu; the order
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
- **Security.** Boundaries TB4; threats TM-T07, TM-T15. Verifies
  SEC-CLI-001.
- **Builds.** The Artists, Albums, Songs, Playlists and Loved tabs; the
  library switcher when a person can see more than one library; a
  library that fills in while the first scan runs; the empty library
  that says what is happening.
- **Tests.** Each tab lists the fixture's items in the fixture's order.
  One library shows no switcher; two show it. The empty and scanning
  states show their literal text.
- **Done when.** Its tests list the demo library by artist, album and
  song.

### CP-017 Album page (SUR-025)

- **Wave** C1 · **Size** M · **Depends on** CP-009, CP-015, CP-020.
- **Owns** `ui/src/surfaces/album/`.
- **Serves** SUR-025 in R1: DIS-050, LIB-134, LIB-135, LIB-146,
  MUS-001, MUS-002, MUS-003, MUS-011, MUS-012, MUS-021, MUS-054; F05
  steps 1 and 2. (The heart is CP-060's and the tinted header CP-061's.)
- **Security.** Boundaries TB4; threats TM-T07. Verifies SEC-CLI-001.
- **Builds.** The header with artwork, the album artist apart from track
  artists and linked credits, on the theme's own surface; disc headers
  with disc titles and "Play disc"; the track
  list; Play and Shuffle.
- **Tests.** The multi-disc fixture shows its disc titles and plays one
  disc. The compilation shows each track's own artist. Play sends the
  album as the From lane, starting at the chosen track.
- **Done when.** Its tests open the multi-disc fixture and send its
  Play operation.

### CP-018 Artist page (SUR-024)

- **Wave** C1 · **Size** M · **Depends on** CP-009, CP-015, CP-020.
- **Owns** `ui/src/surfaces/artist/`.
- **Serves** SUR-024 in R1: DIS-050, LIB-040, MUS-001, MUS-003,
  MUS-004, MUS-006, MUS-011, MUS-051, MUS-052.
- **Security.** Boundaries TB4; threats TM-T07. Verifies SEC-CLI-001.
- **Builds.** The header with the line that tells same-name artists
  apart, the albums, "Appears on", "All songs", Play and Shuffle.
  Release-type sections, role tabs and the artist image are R1.1.
- **Tests.** The two same-name fixture artists show different
  disambiguation lines and different albums. A compilation appears under
  "Appears on" and not among the albums.
- **Done when.** Its tests reach the page from a credited name on a row.

### CP-019 Audio engine

- **Wave** C1 · **Size** L · **Depends on** CP-005, CP-013.
- **Owns** `player/`.
- **Serves** ACC-122, MUS-066, MUS-070, MUS-071, MUS-079, MUS-084,
  MUS-085, MUS-087 to MUS-089 (applying the core's decision), MUS-229
  (the capability report).
- **Security.** Boundaries TB4; threats TM-T16, TM-T28. Verifies
  SEC-API-027, SEC-API-029.
- **Builds.** The direct path in
  [Playback on the web](#playback-on-the-web-in-r1): an output interface
  with one web implementation over two audio elements and a gain node
  each; the capability probe; the translation of element events into the
  core's player events; early loading of the next item; silent refresh
  of an expired URL; the buffered range; volume. The audio element and
  the gain node are reached only through browser adapters in
  `player/browser/`, so everything in `player/src/` runs in Vitest with
  a scripted adapter, and the adapters themselves run in the browser
  suite.
- **Tests.** With a scripted element: each sequence of element events
  gives a literal sequence of core events; the gain set on the node is
  the decision's number; a pause past the URL's expiry under the
  injected clock asks for a new URL and resumes at the same position
  with no error state; the next item starts loading at the set point and
  is requested as part of the same playback; a stream URL never appears
  in a store, a log line or the media session. In three browser engines,
  a tone fixture plays and its position advances. Stubbed probe answers
  give a literal capability report.
- **Done when.** Its browser test plays a tone fixture in three engines
  with the gain factor the core gave.

### CP-020 Playback controller

- **Wave** C1 · **Size** M · **Depends on** CP-013, CP-014, CP-019.
- **Owns** `app/src/playback/`.
- **Serves** LAT-009, MUS-077, MUS-079, MUS-116 to MUS-119, MUS-122,
  MUS-123, MUS-126, MUS-229.
- **Security.** Boundaries TB4; threats TM-T16, TM-T17. Verifies no
  requirement of its own: the rules are the core's and the enforcement is
  the server's.
- **Builds.** The queue store: each action (play, play next, add, play
  last, move, remove, clear, clear Up next, shuffle and its mode,
  repeat, stop after) becomes a core operation applied optimistically,
  sent to the server, and rebased by the core when the server rejects a
  stale version. The player store holding the core's state. Moving on at
  the end of an item, skipping a damaged or unplayable one. Pausing when
  another session pressed Play. It announces "an item started", "an item
  ended" and "an item was skipped" to whoever subscribed, and records
  nothing itself: play events are CP-060's.
- **Tests.** With scripted ports: each action sends the literal
  operation with the version it was built on; a stale rejection calls
  the core's rebase with the pending operations and the store holds what
  the core returned; a sync showing another session's Play pauses this
  one at its position; a subscriber receives the literal sequence of
  announcements for a played, a skipped and an ended item.
- **Done when.** Its tests drive a queue from Play to the end of an
  album through this one store.

### CP-021 Now-playing bar (SUR-002)

- **Wave** C1 · **Size** M · **Depends on** CP-009, CP-010, CP-020.
- **Owns** `ui/src/surfaces/bar/`.
- **Serves** SUR-002 in R1: ACC-117 (the indicator's place), MUS-001,
  MUS-066, MUS-108, MUS-113, MUS-122. (The heart, MUS-109, is CP-060's
  and the quality badge, MUS-099, CP-027's.)
- **Security.** Boundaries TB4; threats TM-T07. Verifies SEC-CLI-001.
- **Builds.** The bar at every width class: artwork, title, linked
  credits, the progress line, play and pause, next, fixed places for
  the heart and the compact quality badge (empty until CP-060 and
  CP-027), and on wide layouts the lyrics and queue
  toggles, Options, volume and the device slot, which is empty in R1 and
  keeps its place. No bar while nothing is queued.
- **Tests.** The layout contract: the bar's position and the order of
  its controls equal literal values at each width class. Each player
  state fixture shows its literal presentation. Every control has a name
  and a state.
- **Done when.** Its layout contract and state fixtures pass at every
  width class.

### CP-022 Queue (SUR-011)

- **Wave** C1 · **Size** M · **Depends on** CP-009, CP-010, CP-015,
  CP-020.
- **Owns** `ui/src/surfaces/queue/`.
- **Serves** SUR-011 in R1: CLI-060, CLI-149, LAT-006, MUS-116, MUS-119,
  MUS-122, MUS-123, MUS-229.
- **Security.** Boundaries TB4; threats TM-T07, TM-T17. Verifies
  SEC-CLI-001.
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
- **Done when.** Its tests edit a queue by pointer and by keyboard and
  send each literal operation. With it merged, the first clickable
  milestone's packages are complete.

## Wave C2: the whole listening experience

### CP-023 The demo library inside the core

- **Wave** C2 · **Size** S · **Depends on** CP-012, CP-013; WP-088
  (wave 3).
- **Owns** `fake-server/src/core-library/`.
- **Serves** DIS-001, DIS-020, DIS-021, DIS-035, DIS-036, DIS-038,
  DIS-083 to DIS-085, MUS-050, MUS-059, MUS-061, MUS-099, MUS-149,
  MUS-229, MUS-236 (each as the core's rule reaching the demo).
- **Security.** Boundaries TB4; threats TM-T62. Verifies SEC-CLI-021.
- **Builds.** The demo's records are loaded into the library WP-088
  holds in WASM, through the entry this plan asks WP-088 to export
  (loading from catalogue records, which is what applying a decoded
  frame does). Search, the playback decision with its track details
  summary, and Home rows then come from the core, over the demo
  library. A fixture record the core refuses fails the gate. If WP-088
  declines that entry, this package waits for CP-055's recorded frames
  and loads those.
- **Tests.** Against the real module: a literal query over the demo
  library returns literal grouped results; a stubbed capability report
  gives each fixture track its literal decision and reasons; the Home
  rows for a literal set of plays.
- **Done when.** Its tests get search results, decisions and Home rows
  for the demo library from the real module.

### CP-024 Context menu and the action list (SUR-004)

- **Wave** C2 · **Size** M · **Depends on** CP-007, CP-020.
- **Owns** `ui/src/surfaces/menu/`, `ui/src/actions.ts`.
- **Serves** SUR-004 in R1: DIS-111, INT-008, MUS-062, MUS-117,
  MUS-118.
- **Security.** Boundaries TB4; threats TM-T07. Verifies SEC-CLI-001.
- **Builds.** player.md's one action list: each action has one
  identifier, one message, the items it applies to and when it is
  enabled, and menus and media handlers all call it. The menu, opened by
  right-click, the "more" button, long-press and the keyboard's menu
  key, with Play next, Add to queue, Play last, Go to artist, Go to album,
  and "Copy ID" in developer mode. Other packages add their actions as
  lines: Love (CP-060), Info (CP-027), Add to playlist (CP-031), Remove
  this play (CP-032), Rescan (CP-045).
- **Tests.** The same item shows the same actions in the same order from
  every opener. Three "Play next" picks send three operations in order.
  The menu is fully usable by keyboard and closes with Escape.
- **Done when.** Its tests open the same menu from every opener on a
  row and a tile.

### CP-025 Full-screen player (SUR-010)

- **Wave** C2 · **Size** M · **Depends on** CP-010, CP-021, CP-024.
- **Owns** `ui/src/surfaces/player/`.
- **Serves** SUR-010 in R1: MUS-001, MUS-002, MUS-070, MUS-071,
  MUS-077, MUS-087, MUS-110, MUS-113, MUS-116, MUS-122, MUS-123,
  MUS-126, MUS-227.
- **Security.** Boundaries TB4; threats TM-T07. Verifies SEC-CLI-001.
- **Builds.** Large artwork on the theme's surface (CP-061 tints it);
  the title and the credit as tagged; the "Playing from" link;
  the always-visible scrubber with the buffered range; transport;
  shuffle with its two R1 modes, repeat and stop-after; the places for
  the heart and the badge; buttons for the queue, lyrics and info; the
  options menu. It
  fills the content area on wide layouts and is a sheet at compact,
  closed by a button as well as a swipe. The six player keys (Space,
  Shift+Left, Shift+Right, Shift+N, Shift+P and M) and their off switch
  are built here, under the default answer to the
  [shortcut question](#product-questions-for-the-owner).
- **Tests.** The scrubber is a slider with a spoken value, moved by the
  arrow keys. The layout contract for the scrubber and the queue, lyrics
  and device positions. Every state fixture's literal presentation.
- **Done when.** Its layout contract, state fixtures and keyboard tests
  pass.

### CP-026 Lyrics (SUR-012)

- **Wave** C2 · **Size** S · **Depends on** CP-013, CP-021.
- **Owns** `ui/src/surfaces/lyrics/`.
- **Serves** SUR-012 in R1: LIB-067, MUS-154, MUS-155.
- **Security.** Boundaries TB4; threats TM-T07. Verifies SEC-CLI-001,
  SEC-MED-057.
- **Builds.** Plain lyrics, and synced lyrics with the current line
  marked from the core's position lookup, in the right pane, the content
  area or a panel in the player. "This file has no lyrics." when there
  are none. Word-by-word timing and staying open across tracks are R1.1.
- **Tests.** At literal positions the marked line is the literal
  expected one. Lyrics holding markup appear as text with their line
  breaks kept and no element created.
- **Done when.** Its tests mark the literal line at each literal
  position.

### CP-027 Track details and the quality badge (SUR-016)

- **Wave** C2 · **Size** S · **Depends on** CP-007, CP-023.
- **Owns** `ui/src/surfaces/details/`, `ui/src/kit/quality-badge/`.
- **Serves** SUR-016: MUS-021, MUS-032, MUS-034, MUS-036, MUS-037,
  MUS-067, MUS-069, MUS-084, MUS-089, MUS-099, MUS-229, MUS-236.
- **Security.** Boundaries TB4; threats TM-T07, TM-T10. Verifies
  SEC-CLI-001, SEC-MED-057.
- **Builds.** The badge, registered for the places the bar and the
  full-screen player keep for it, compact ("Original") and in full on hover,
  focus or tap, in design-language's words. The read-only details view
  from the core's summary: title and credit as tagged, the album, the
  format, the decision and its reason, whether the track joins the next
  without a gap, and where its level came from. The Info action for the
  context menu.
- **Tests.** Each fixture decision gives its literal sentence, including
  "Can't play in this browser: ALAC". The view offers no edit control
  and shows no file path.
- **Done when.** Its tests show each fixture decision's literal
  sentence in the badge and the details view.

### CP-028 Browser media controls (SUR-053)

- **Wave** C2 · **Size** S · **Depends on** CP-020.
- **Owns** `app/src/media-session/`.
- **Serves** SUR-053 in R1: CLI-070, MUS-073.
- **Security.** Boundaries TB4, TB5; threats TM-T16. Verifies SEC-API-029
  (the artwork handed to the browser is never a capability URL).
- **Builds.** The Media Session: metadata, position, and the handlers in
  player.md's table (play, pause, stop, next, previous with the restart
  rule, seek), each calling the action list.
- **Tests.** With a scripted media session: after every play, pause,
  seek and queue edit, what the browser holds equals the player's state
  literally. Signing out clears it.
- **Done when.** Its tests show the scripted media session equal to the
  player after every change.

### CP-029 Home (SUR-020)

- **Wave** C2 · **Size** M · **Depends on** CP-015, CP-020, CP-023,
  CP-060.
- **Owns** `ui/src/surfaces/home/`.
- **Serves** SUR-020 in R1: DIS-001, DIS-002, DIS-004, DIS-020, DIS-021,
  DIS-035, DIS-036, DIS-140, MUS-050, MUS-059, MUS-149.
- **Security.** Boundaries TB4; threats TM-T07, TM-T15. Verifies
  SEC-CLI-001.
- **Builds.** The rows the core returns, Continue listening first, then
  Recently played, Recently added by album and Loved tracks; the
  first-run cards and empty states. Rows scroll with arrows on wide
  layouts and stack at compact. An arrangeable Home is R1.2.
- **Tests.** For literal row data the page shows the literal rows in
  order. A profile with no plays shows the empty states. Home draws
  without waiting for any port call to resolve.
- **Done when.** Its tests show the literal rows for literal plays and
  the empty states for none.

### CP-030 Search and genre pages (SUR-032, SUR-028)

- **Wave** C2 · **Size** M · **Depends on** CP-009, CP-015, CP-023.
- **Owns** `ui/src/surfaces/search/`, `ui/src/surfaces/browse/`.
- **Serves** SUR-032 and SUR-028 in R1: CLI-022, DIS-083, DIS-084,
  DIS-085, DIS-109, LIB-053, MUS-017, MUS-060, MUS-061; F09.
- **Security.** Boundaries TB4; threats TM-T07, TM-T18. Verifies
  SEC-PRV-004 (the client's half: what is typed never leaves the device).
- **Builds.** The search field at the top of the sidebar, reached by Tab
  (no shortcut in R1, interface decision 6), and the Search tab at
  compact with Browse above the results when the field is empty. Results
  as you type, grouped by type with type chips. Genre pages. Recent
  searches, scopes and mood and label pages are R1.1.
- **Tests.** While a query is typed, no `ServerPort` method is called.
  Literal queries, with a typing mistake and without accents, show the
  core's literal groups. A genre page lists the fixture's albums for
  that genre.
- **Done when.** Its tests show the core's groups for each literal
  query and no server call while typing.

### CP-031 Playlists (SUR-026, SUR-015)

- **Wave** C2 · **Size** M · **Depends on** CP-015, CP-020, CP-024,
  CP-060.
- **Owns** `ui/src/surfaces/playlist/`,
  `ui/src/surfaces/add-to-playlist/`, `app/src/playlists/`.
- **Serves** SUR-026 and SUR-015 in R1: DIS-046, MUS-132, MUS-133,
  MUS-149; F06.
- **Security.** Boundaries TB4; threats TM-T07, TM-T12. Verifies
  SEC-CLI-001 (playlist names are untrusted text).
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
- **Done when.** Its tests create a playlist from the sheet, reorder it
  and send each literal operation.

### CP-032 History and private listening (SUR-029)

- **Wave** C2 · **Size** M · **Depends on** CP-015, CP-024, CP-060.
- **Owns** `ui/src/surfaces/history/`, `app/src/history/`,
  `app/src/private-session/`.
- **Serves** SUR-029: ACC-117, ACC-118, CLI-093, DIS-021, DIS-050,
  DIS-052, DIS-053, DIS-186, DIS-187, DIS-189, MUS-183, MUS-184,
  MUS-185, MUS-233, MUS-234; F16.
- **Security.** Boundaries TB4, TB5; threats TM-T18. Verifies SEC-PRV-024,
  SEC-PRV-022 (the client's half: the page holds only this person's
  plays).
- **Builds.** History by day with the device each play came from; remove
  a play, a range of dates or everything; how long history is kept;
  "Only you can see this". Private session: an item in the player's
  options, first in the list, so it is two interactions away; the
  indicator in the bar, the player and the account menu for as long as
  it is on; its end time. Turning it on sets the mode on the event sink
  (CP-060), and the core drops the events.
- **Tests.** Private session is reached in two interactions from the
  full player and from the wide bar. With it on, playing a track sends
  no play event and adds nothing to History or Home. Removing a play
  sends the literal erasure and the row is gone.
- **Done when.** Its tests remove a play and show nothing recorded
  while a private session is on.

### CP-033 Status layer, notice centre and player states (SUR-003)

- **Wave** C2 · **Size** M · **Depends on** CP-007, CP-012, CP-020.
- **Owns** `ui/src/surfaces/status/`, `app/src/notices/`.
- **Serves** SUR-003 in R1: ACC-003, ACC-071, ACC-075, ADM-085, CLI-002,
  DIS-002, LIB-032, MUS-042, MUS-043, MUS-079, MUS-229; the R1 rows of
  player.md's States table; F12.
- **Security.** Boundaries TB4; threats TM-T09, TM-T10, TM-T16. Verifies
  SEC-API-031, SEC-API-072, SEC-OPS-033, SEC-TM-040, SEC-TM-068 (each in
  its client half: the refusal or problem is shown in plain words and
  never fails silently).
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
- **Done when.** Every R1 row of player.md's States table has a fixture
  whose literal presentation its tests assert.

### CP-034 Gapless playback through Media Source

- **Wave** C2 · **Size** L · **Depends on** CP-019, CP-023; WP-056
  (wave 2).
- **Owns** `player/src/mse/`, `player/browser/media-source.ts`,
  `fixtures/packaged/`.
- **Serves** MUS-067, MUS-069, MUS-070, MUS-071, and the client's half
  of MUS-230.
- **Security.** Boundaries TB4; threats TM-T28, TM-T62. Verifies
  SEC-CLI-021 (a malformed segment is a typed problem, not a crash),
  SEC-API-029.
- **Builds.** The packaged path in
  [Playback on the web](#playback-on-the-web-in-r1): a second output,
  behind its own browser adapter, that feeds Media Source (or
  `ManagedMediaSource`) with the packager's initialisation and media
  segments, sets append windows and timestamp
  offsets from the trim values, seeks through the seek index and evicts
  what has played. The segment fixtures, with their manifest, written by
  an xtask command that runs the Rust packager over generated tones;
  that command is Rust, so it is a request to the backend plan (WP-056).
- **Tests.** With a scripted source buffer: for literal trim values the
  append window and offset equal literal numbers. In each engine in the
  support list: two tracks cut from one continuous tone play across the
  join with no gap in the buffered range, and a seek lands on the
  literal position. Whether each engine lets a test capture the output
  to compare samples is unverified; where it does, the join is compared
  with the known tone.
- **Done when.** Its browser test plays two fixture tracks across the
  join with a contiguous buffered range in each engine.

### CP-060 The event sink: plays and loves

- **Wave** C2 · **Size** M · **Depends on** CP-015, CP-020, CP-021;
  WP-237 (wave 2).
- **Owns** `app/src/events/`, `ui/src/kit/heart/`.
- **Serves** CLI-093, DIS-045, DIS-051, MUS-109, MUS-180, MUS-182.
- **Security.** Boundaries TB4, TB5; threats TM-T18. Verifies SEC-PRV-024
  (the client's half: in private mode nothing is queued for upload).
- **Builds.** The event sink, the one place the client makes a user
  event. It subscribes to the playback controller's announcements and to
  the heart, asks the core (WP-237) to build each event with its ID and
  clock, hands every event to the core's sink with the current mode, so
  the core drops it in private mode, keeps what the core returns in
  memory, sends it when the server is reachable, and passes the server's
  clock back to the core on each sync. No ID, clock value or event
  field is made in TypeScript. The heart control, registered for the
  places rows, the album page, the bar and the full-screen player keep
  for it. When the server cannot be reached, a play is kept and sent
  later, and the heart is disabled with "Needs the server" (D-71). The
  private mode's switch is CP-032's.
- **Tests.** With a scripted core and server: a played item gives one
  call to the core's event builder and the returned event is sent
  unchanged; with the mode set to private nothing is sent and nothing is
  kept; plays made while the server is unreachable are sent in order
  when it returns; a love toggles the heart's pressed state and sends
  the core's event; the heart is disabled, with the literal reason,
  while the server is unreachable. Against the real module, a play
  event's fields are exactly the ones the user log allows.
- **Done when.** Its tests show a play and a love leaving as the core's
  events, and neither leaving in private mode.

### CP-061 Artwork tints

- **Wave** C2 · **Size** S · **Depends on** CP-004, CP-015, CP-017;
  WP-237 (wave 2) with the tint rule this plan asks the backend plan
  for.
- **Owns** `ui/src/tint/`.
- **Serves** MUS-039, MUS-110; design-language section 5 and
  requirement A1's hue sweep.
- **Security.** Boundaries TB4; threats TM-T62. Verifies SEC-CLI-021 (a
  palette value out of range is dropped and the page is untinted),
  SEC-API-044 (the colours are written as custom properties through the
  CSSOM, never as a style attribute).
- **Builds.** One module that every tinted surface uses. It passes the
  synced base colour, the theme and the token colours that will sit on
  the surface to the core, and writes the surface colours the core
  returns. The rule itself (lightness and chroma by theme, bringing the
  colour into sRGB by reducing chroma, the contrast check against every
  token, the 0.01 lightness steps, no tint for covers with almost no
  chroma, and the placeholder hue from an item's ID) is the core's, so
  it is written once and is the same on every client. The album header
  wash, the artist and playlist headers, the full-screen player's
  background and the coloured tile placeholder are registered here. In
  the high-contrast theme nothing is tinted.
- **Tests.** Against the real module, sweeping the hue in steps finer
  than 5 degrees at zero, half and the full chroma cap, with the token
  file's own values: every pair in design-language's "The guarantee"
  table meets its floor in the dark and the light theme. With a scripted
  core: the custom properties written equal the core's literal answer;
  an out-of-range colour writes nothing; the high-contrast theme writes
  nothing.
- **Done when.** Its sweep passes from the token file, and no other
  package computes a colour.

## Wave C3: signing in, settings and account

### CP-035 Sign-in (SUR-070)

- **Wave** C3 · **Size** M · **Depends on** CP-009, CP-012.
- **Owns** `ui/src/surfaces/sign-in/`, `app/src/auth/`.
- **Serves** SUR-070 in R1: ACC-003, ACC-004, ACC-007, ACC-050, ACC-062,
  ACC-063, ACC-079, CLI-150, CLI-155; F03; design-language requirement
  A15.
- **Security.** Boundaries TB1, TB2, TB4; threats TM-T04, TM-T10, TM-T47.
  Verifies SEC-CLI-010 (the question), SEC-CLI-028.
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
- **Done when.** Its Chromium test signs in with a virtual
  authenticator and its component tests show the one failure sentence.

### CP-036 Storage modes and the wipe

- **Wave** C3 · **Size** M · **Depends on** CP-014, CP-035.
- **Owns** `app/src/storage/`.
- **Serves** ACC-065, ACC-069, ACC-070, CLI-022, CLI-155, CLI-156.
- **Security.** Boundaries TB5; threats TM-T38, TM-T39, TM-T40. Verifies
  SEC-CLI-009, SEC-CLI-010, SEC-IAM-017, SEC-PRV-019, SEC-TM-058,
  SEC-IAM-043 (the client's half: it wipes before it shows anything).
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
  One more record class exists for administrators only: the audit
  checkpoint head, kept in a personal browser and refused in a shared
  one (SEC-OPS-075, used by CP-052). IndexedDB, origin private files and
  Cache Storage are reached through browser adapters in `app/browser/`.
- **Tests.** In three engines: fill the copy, sign out, and every store
  is empty, no service worker is registered and Back shows no data; the
  same after a revocation from a second session, with nothing drawn
  before the wipe. In shared mode nothing is ever written to a store.
  The layer refuses a history record and a credential record by type and
  at run time.
- **Done when.** Its three-engine tests find every store empty after
  sign-out and after revocation.

### CP-037 Welcome: first-run setup (SUR-082)

- **Wave** C3 · **Size** L · **Depends on** CP-035, CP-040, CP-062.
- **Owns** `ui/src/surfaces/welcome/`, `ui/src/admin/folder-picker/`.
- **Serves** SUR-082 in R1: ACC-001, ACC-002, ACC-050, ACC-113, ADM-018,
  ADM-019, ADM-021, ADM-022, ADM-025, ADM-028, ADM-029, ADM-031,
  ADM-053, ADM-069, ADM-143, LIB-001, LIB-005, LIB-021, LIB-108; F01 and
  the welcome screen's part of F14.
- **Security.** Boundaries TB1, TB2, TB4, TB11; threats TM-T06, TM-T65.
  Verifies SEC-CLI-013, SEC-CLI-025, SEC-CLI-028, SEC-OPS-047 (the
  client's half: the required question has no answer preselected and
  cannot be skipped).
- **Builds.** The R1 steps in surfaces.md's order: the setup code, as
  the typed route CP-062 got from the core's parser or typed by hand;
  where people will reach the server, and
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
- **Done when.** Its tests walk every R1 step with the fake and refuse
  each wrong input.

### CP-038 Invitation landing (SUR-071)

- **Wave** C3 · **Size** M · **Depends on** CP-035, CP-062.
- **Owns** `ui/src/surfaces/invite/`.
- **Serves** SUR-071 in R1: ACC-080; the friend's side of F10.
- **Security.** Boundaries TB1, TB4; threats TM-T47, TM-T65. Verifies
  SEC-CLI-013, SEC-CLI-025, SEC-CLI-001.
- **Builds.** The screen the invitation's typed route opens (CP-062):
  what will happen, the privacy notice the server generates,
  account creation with the person's own passkey, and, where the
  invitation needs the inviter to confirm, the waiting screen with the
  short matching code. Nothing is redeemed until the person confirms.
- **Tests.** The secret leaves the address and history before any
  request, is sent only in a request body, and only after the confirm
  press. Every failure shows the same sentence. The notice's text is
  rendered literally.
- **Done when.** Its tests redeem an invitation only after the confirm
  press, with the secret in the request body.

### CP-039 Approving another browser (SUR-061)

- **Wave** C3 · **Size** M · **Depends on** CP-035, CP-040, CP-062.
- **Owns** `ui/src/surfaces/pairing/`.
- **Serves** SUR-061 in R1: ACC-062; F03's approval branch.
- **Security.** Boundaries TB4, TB5; threats TM-T64. Verifies SEC-CLI-013,
  SEC-CLI-025, SEC-CLI-028, and the client's half of SEC-IAM-058 and
  SEC-CLI-024.
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
- **Done when.** Its tests show the approval sheet's literal text and
  the step-up prompt before Approve.

### CP-040 Step-up prompt and upload dialog (SUR-008, SUR-009)

- **Wave** C3 · **Size** M · **Depends on** CP-007, CP-035.
- **Owns** `ui/src/surfaces/step-up/`, `ui/src/surfaces/upload/`,
  `app/src/step-up/`.
- **Serves** SUR-008 and SUR-009: ACC-056, ACC-125.
- **Security.** Boundaries TB4, TB11; threats TM-T13. Verifies the
  client's half of SEC-IAM-041 and SEC-IAM-023.
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
- **Done when.** Its tests retry a call once after the check and change
  nothing on Cancel.

### CP-041 Account menu and Settings (SUR-006, SUR-073, SUR-074, SUR-076, SUR-077)

- **Wave** C3 · **Size** M · **Depends on** CP-036.
- **Owns** `ui/src/surfaces/account-menu/`, `ui/src/surfaces/settings/`,
  `app/src/settings/`.
- **Serves** ACC-012, ACC-017, ACC-113, ACC-114, ACC-117, CLI-031,
  CLI-141, CLI-150, DIS-053, LIB-146, MUS-087, MUS-088, MUS-126,
  MUS-185.
- **Security.** Boundaries TB4, TB5; threats TM-T40. Verifies SEC-CLI-009
  (Sign out runs the wipe), SEC-CLI-010 ("About this connection" says
  which mode this browser is in), SEC-SUP-031 (the "Source code" link for
  the running commit).
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
- **Done when.** Its tests send each setting's literal key and value
  and apply each theme's token set.

### CP-042 Account, recovery and your data (SUR-078, SUR-133)

- **Wave** C3 · **Size** M · **Depends on** CP-040, CP-041.
- **Owns** `ui/src/surfaces/account/`, `ui/src/surfaces/recovery/`.
- **Serves** SUR-078 and SUR-133 in R1: ACC-002, ACC-009, ACC-010,
  ACC-050, ACC-055, ACC-065, ACC-136, ACC-137, ACC-138, ADM-143,
  DIS-058, DIS-188, INT-151, LAT-007, MUS-188.
- **Security.** Boundaries TB4, TB11; threats TM-T13, TM-T63. Verifies the
  client's half of SEC-IAM-023, SEC-IAM-106 and SEC-PRV-048.
- **Builds.** The person's passkeys, named, with add and remove and the
  prompt while only one exists; the links to the pages in CP-043; "Your
  data", with the export and deleting the account; recovery codes, shown
  once; for the owner, the recovery kit's status; and, during a recovery
  hold, what is restricted and when it ends.
- **Tests.** Adding or removing a passkey, starting an export and
  deleting the account each open the step-up prompt first. The last
  passkey has no remove control. During a hold the export and removal
  controls are disabled with the literal reason.
- **Done when.** Its tests open the step-up prompt before each
  sensitive action and hide what a hold restricts.

### CP-043 Your devices, security events and what admins can see (SUR-130, SUR-131, SUR-132)

- **Wave** C3 · **Size** M · **Depends on** CP-041.
- **Owns** `ui/src/surfaces/devices/`,
  `ui/src/surfaces/security-events/`,
  `ui/src/surfaces/admin-visibility/`.
- **Serves** ACC-068, ACC-069, ACC-070, ACC-071, ACC-078, ACC-115,
  ACC-139, ADM-110; F15.
- **Security.** Boundaries TB4, TB5; threats TM-T16, TM-T18, TM-T38.
  Verifies the client's half of SEC-IAM-042, SEC-IAM-097 and SEC-IAM-104,
  and SEC-OPS-033.
- **Builds.** Every session and device with its class, rough location
  and last use, "This device" marked, remove one, and "Sign out
  everywhere else". The person's own security events with filters and
  "This wasn't me". The page that says, in the server's own generated
  words, what admins can and cannot see.
- **Tests.** Removing a device sends the literal call and the row goes.
  "This wasn't me" revokes the device in the entry. The visibility page
  renders the server's statements literally and adds none of its own.
- **Done when.** Its tests remove a device and revoke one from a
  security event.

### CP-062 Inbound links through the core

- **Wave** C3 · **Size** S · **Depends on** CP-009; WP-237 (wave 2)
  with the link parser, if the backend plan accepts the split of
  `deeplink.rs`; otherwise WP-089 (wave 3) and WP-088's export of it.
- **Owns** `ui/src/inbound/`.
- **Serves** ACC-001, ACC-062, ACC-080 (the links R1 issues: the claim
  link, invitations, browser pairing and recovery enrolment).
- **Security.** Boundaries TB1, TB4; threats TM-T47, TM-T65. Verifies
  SEC-CLI-025, SEC-CLI-013.
- **Builds.** The one entry for any address that carries a fragment, a
  query string or a path outside CP-009's list, and for a scanned QR
  payload. It takes the secret out of the address bar and the history
  first, then hands the whole original string to the core's parser and
  receives a typed route or "not recognised". A recognised route opens
  its confirmation screen (CP-037, CP-038, CP-039, CP-035's recovery
  entry) with the parsed values; nothing is redeemed before the person
  confirms. No TypeScript splits, decodes or matches any part of the
  string.
- **Tests.** With a scripted core: the string the core receives equals
  the original address literally; the address and every history entry
  hold no secret before the first port call; "not recognised" shows the
  "not found" page and makes no server call; each typed route opens its
  literal screen. Against the real module: an R1 claim link, invitation
  link and pairing link each give their literal typed route.
- **Done when.** Its tests show every R1 link reaching its confirmation
  screen through the core's parser and through nothing else.

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
- **Security.** Boundaries TB11; threats TM-T10, TM-T13. Verifies the
  client's half of SEC-IAM-041 (the admin session and its end) and
  SEC-NET-047 (the version appears only here).
- **Builds.** The admin navigation and the session that opens it, with
  the return to the prompt when it ends. The dashboard cards: backups,
  library roots, free space, the scan, the security card, owner alerts
  with critical ones as a banner, advisories, recovery used, updates,
  and for the owner "Rotate all server secrets".
- **Tests.** Opening Admin asks for the passkey first. An expired admin
  session returns to the prompt and shows no admin data. Each card shows
  the literal text for its fixture state.
- **Done when.** Its tests show each card's literal text and no admin
  data after the session ends.

### CP-045 Libraries and library settings (SUR-085)

- **Wave** C4 · **Size** L · **Depends on** CP-037, CP-044.
- **Owns** `ui/src/admin/libraries/`.
- **Serves** SUR-085 in R1: ACC-037, ADM-025, ADM-085, ADM-108, LIB-001,
  LIB-003, LIB-004, LIB-007, LIB-012, LIB-013, LIB-014, LIB-015,
  LIB-038, LIB-136, MUS-035; F02.
- **Security.** Boundaries TB9, TB11; threats TM-T13, TM-T55. Verifies the
  client's half of SEC-IAM-041 for adding, removing and browsing roots.
- **Builds.** The library list with each root's health; per library its
  folders, schedule, watching, storage type, artist splitting rules,
  artwork order, "Media is read-only" and who may see it; add a library;
  scan now; the Rescan action, added to the context menu's list once
  CP-024 has merged.
- **Tests.** Adding a folder opens the step-up prompt, then the picker.
  A new library is shown as visible to the owner and admins only. Scan
  now sends the literal call and the scan card moves.
- **Done when.** Its tests add a folder through the prompt and the
  picker and send the literal scan call.

### CP-046 Library health and trash (SUR-086, SUR-105)

- **Wave** C4 · **Size** M · **Depends on** CP-044.
- **Owns** `ui/src/admin/health/`, `ui/src/admin/trash/`.
- **Serves** ADM-086, LIB-014, LIB-032, LIB-033, LIB-038, LIB-049,
  LIB-068, LIB-193, LIB-205, LIB-206, LIB-207, MUS-006, MUS-035,
  MUS-038, MUS-044, MUS-079, MUS-155, MUS-229.
- **Security.** Boundaries TB9, TB11; threats TM-T07, TM-T56. Verifies
  SEC-CLI-001 (file names and tag values are untrusted text).
- **Builds.** The health report with its filters: damaged and unreadable
  files, quarantined files with the reason and a retry, the scan
  worker's isolation level, same-name collisions, sidecar problems,
  moved files, offline roots and watch warnings. The trash: items, when
  they went missing, when they will be purged, restore and purge now.
- **Tests.** Each problem kind shows its literal line and action. A file
  name holding a corpus payload appears literally. Purge asks first and
  states the count.
- **Done when.** Its tests show each problem kind's literal line and
  action.

### CP-047 Users and invitations (SUR-090)

- **Wave** C4 · **Size** L · **Depends on** CP-044.
- **Owns** `ui/src/admin/users/`.
- **Serves** SUR-090 in R1: ACC-005, ACC-006, ACC-008, ACC-037, ACC-064,
  ACC-076, ACC-080, ADM-052, MUS-027; the owner's side of F10.
- **Security.** Boundaries TB11; threats TM-T18, TM-T65. Verifies the
  client's half of SEC-IAM-044, SEC-IAM-079 and SEC-PRV-025.
- **Builds.** Users with their libraries, role, device count, last
  sign-in, device cap and enable switch; ending a person's sessions;
  "help sign in"; invitations with their address, expiry, uses and
  status, and confirming a waiting invitee's matching code; handing the
  server to a new owner.
- **Tests.** A user row shows exactly the literal fields and nothing
  about what the person plays. An invitation's preset cannot offer a
  library the inviter lacks. Transfer asks both parties for the passkey
  check.
- **Done when.** Its tests create an invitation limited to the
  inviter's libraries and show no play data on a user row.

### CP-048 Security and server settings (SUR-092, SUR-103)

- **Wave** C4 · **Size** M · **Depends on** CP-044.
- **Owns** `ui/src/admin/security-settings/`,
  `ui/src/admin/server-settings/`.
- **Serves** ACC-063, ACC-075, ACC-079, ADM-001, ADM-090, ADM-111.
- **Security.** Boundaries TB11; threats TM-T13. Verifies the client's
  half of SEC-IAM-041 and SEC-IAM-025.
- **Builds.** Guessing protection and its state; session lifetimes,
  which can only be shortened; the recovery-hold length; remote
  administration. Storage locations; retention; stream limits; About
  with the version, build and target.
- **Tests.** A lifetime field refuses a value above the limit. The page
  holds no control for passwords or for turning sign-in off. Each change
  opens the step-up prompt.
- **Done when.** Its tests refuse a lifetime above the limit and find
  no password control.

### CP-049 Network and remote access (SUR-093)

- **Wave** C4 · **Size** M · **Depends on** CP-044.
- **Owns** `ui/src/admin/network/`.
- **Serves** SUR-093 in R1: ACC-097, ACC-098, ACC-099, ADM-022, ADM-028,
  ADM-129.
- **Security.** Boundaries TB1, TB8, TB11; threats TM-T05, TM-T07.
  Verifies the client's half of SEC-IAM-041; SEC-CLI-001 (proxy and host
  names are untrusted text).
- **Builds.** The posture in plain words; trusted proxies, with any
  undeclared proxy seen and "Trust it"; HTTPS for the owner's domain, a
  supplied certificate or a tailnet name, with its expiry; the
  reverse-proxy and tailnet recipes; privacy choices; the network
  activity page.
- **Tests.** Each posture fixture gives its literal sentence. Trusting a
  proxy opens the step-up prompt. The activity page lists each
  destination with the feature that caused it.
- **Done when.** Its tests show each posture's literal sentence and the
  activity list.

### CP-050 Backups and updates (SUR-098, SUR-099)

- **Wave** C4 · **Size** M · **Depends on** CP-040, CP-044.
- **Owns** `ui/src/admin/backups/`, `ui/src/admin/updates/`.
- **Serves** ACC-013, ADM-053, ADM-054, ADM-056, ADM-060, ADM-065,
  ADM-066, ADM-068, ADM-069, ADM-072.
- **Security.** Boundaries TB10, TB11, TB12; threats TM-T43, TM-T59.
  Verifies SEC-SUP-050 (the client's half: the stale-feed warning and the
  offline status line), and the client's half of SEC-OPS-045.
- **Builds.** The backup list with status, verification and encryption,
  each backup's contents in plain words, retention, download and upload,
  the recovery kit's status and "Make a new kit". Updates: whether the
  check is on, the version, available updates, advisories, the
  end-of-support date, each release's notes, and "Can't confirm you're
  up to date". Restore from these screens is R1.2.
- **Tests.** Download opens the step-up prompt and is offered to the
  owner only. A feed seven days stale shows the literal warning; offline
  mode shows the quiet line instead.
- **Done when.** Its tests show the stale-feed warning, the offline
  line and the owner-only download.

### CP-051 Activity, alerts and diagnostics (SUR-100, SUR-101, SUR-102)

- **Wave** C4 · **Size** M · **Depends on** CP-044.
- **Owns** `ui/src/admin/activity/`, `ui/src/admin/alerts/`,
  `ui/src/admin/diagnostics/`.
- **Serves** ACC-078, ADM-034, ADM-077, ADM-080, ADM-083, ADM-095,
  ADM-110, ADM-116, ADM-119, ADM-122, ADM-123, DIS-038, LIB-016,
  LIB-017, LIB-019, LIB-022, LIB-029.
- **Security.** Boundaries TB11; threats TM-T18, TM-T61. Verifies the
  client's half of SEC-OPS-034 and SEC-PRV-025.
- **Builds.** Scan and job activity with the indicator in the admin
  header; alert rules and destinations, with the critical alerts that
  cannot be muted; log settings; the doctor's results, the write queue
  and "rebuild the cache". The task list with run and cancel, and the
  diagnostic bundle, are R1.2.
- **Tests.** A critical alert has no mute control. An activity entry
  names files and libraries and never a play. The doctor's fixture
  results show their literal lines.
- **Done when.** Its tests show the doctor's literal lines and no mute
  control on a critical alert.

### CP-052 Security log and the audit anchor (SUR-108)

- **Wave** C4 · **Size** M · **Depends on** CP-036, CP-044.
- **Owns** `ui/src/admin/security-log/`, `app/src/audit-anchor/`.
- **Serves** SUR-108: ADM-110, ADM-145.
- **Security.** Boundaries TB10, TB11; threats TM-T61. Verifies
  SEC-OPS-075, and the client's half of SEC-OPS-027.
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
- **Done when.** Its tests raise the alert for a log that does not
  extend the stored head.

## Wave C5: the real server

Each package here starts when the server packages it names have merged.
None waits for C4.

### CP-053 HTTP adapter and event channel

- **Wave** C5 · **Size** M · **Depends on** CP-005; WP-118 (wave 2),
  WP-083, WP-089 (wave 3).
- **Owns** `http-server/`.
- **Serves** ACC-124, CLI-001, INT-005, INT-023.
- **Security.** Boundaries TB4; threats TM-T16, TM-T62. Verifies
  SEC-CLI-011 (the client's half: the build identifier on every call, and
  a reload on the typed answer), SEC-CLI-021, SEC-API-072, SEC-TM-058 (no
  script ever holds a credential).
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
- **Done when.** Its tests send every method's literal request, and the
  bundle check finds no fake module in `apps/web`.

### CP-054 Sync client and the device copy

- **Wave** C5 · **Size** L · **Depends on** CP-023, CP-036, CP-053;
  WP-084, WP-088 (wave 3).
- **Owns** `app/src/sync/`.
- **Serves** CLI-022, CLI-093, DIS-002, LIB-021, MUS-043, MUS-208.
- **Security.** Boundaries TB4, TB5; threats TM-T15, TM-T39, TM-T62.
  Verifies SEC-CLI-021, SEC-PRV-019, and the client's half of SEC-CLI-020
  (a removal deletes everything that pointed at the item).
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
- **Done when.** Its tests apply recorded frames and read the literal
  library back through the facade.

### CP-055 The contract run against the real server

- **Wave** C5 · **Size** M · **Depends on** CP-053, CP-054; WP-119
  (wave 2) and the wave 3 route packages.
- **Owns** `clients/e2e/contract/`, `fixtures/recorded/`.
- **Serves** The owner's choice of 2026-10-03: swapping the fake for the
  server is a configuration change.
- **Security.** Boundaries TB4; threats TM-T69. Verifies no requirement of
  its own.
- **Builds.** Check 4 in
  [What keeps the fake and the real server in step](#what-keeps-the-fake-and-the-real-server-in-step),
  as a step of the web gate, so it is a required check. It writes
  WP-119's small synthetic library with a fixed seed, starts the server
  on loopback with an injected clock, and runs the conformance suite
  three times: against the real server, against the fake replaying the
  recording, and against the fake with the hand-written demo library
  (with that library's own literal expectations). It re-records and
  compares the recording with the committed one after decoding, value by
  value; fields that are random by design (IDs, cursors) are compared by
  kind and by where they are reused, not by value. This package covers
  every `ServerPort` method whose route exists in wave 3. CP-057 adds
  streams and stream limits, and CP-058 adds the wave 4 to 6 answers and
  every failure switch; each of those packages owns its added cases.
- **Tests.** These are the tests. The run is checked to fail: a fake
  altered to answer one field differently fails the suite, a stale
  recording fails the comparison, and two runs of the real server with
  the same seed and clock decode to equal values.
- **Done when.** For every `ServerPort` method in its scope, a change on
  either side that the other does not follow fails the gate.

### CP-056 Production bundle and the handover to the server

- **Wave** C5 · **Size** M · **Depends on** CP-003; WP-072 (wave 3).
- **Owns** `clients/tools/gate/bundle/`, `clients/apps/web/build-info/`.
- **Serves** CLI-001.
- **Security.** Boundaries TB4, TB12; threats TM-T07, TM-T41. Verifies
  SEC-CLI-012, SEC-SUP-037, SEC-STD-017, SEC-SUP-031, and the client CI's
  half of SEC-CLI-016.
- **Builds.** The production build's output in a stated form: a
  manifest of exact paths with their types and SHA-256 digests, hashed
  file names, no source maps, no service worker in R1, the build
  identifier, and the source repository and commit for the "Source code"
  link. The bundle checks in the gate: no module of `fake-server` or
  `fixtures`, no absolute URL to another origin, no secret, and the size
  against the DIS-019 load budget. Two builds of one commit give the
  same digests, and the build needs no network after the install.
  Putting the bundle into the server binary is not this package's: WP-072
  owns `crates/gunmetal-server/src/webapp/` and reads this manifest (see
  [Requests to the backend plan](#requests-to-the-backend-plan)).
- **Tests.** Each check fails on a fixture bundle with the fault
  planted. The manifest of a small fixture build equals a literal.
- **Done when.** The build writes a manifest that equals the files on
  disk, and every bundle check passes on it and fails on its fixture.

### CP-057 Real streaming

- **Wave** C5 · **Size** M · **Depends on** CP-019, CP-034, CP-053;
  WP-082 (wave 3), WP-103, WP-104, WP-105 (wave 4).
- **Owns** `player/src/streams/`.
- **Serves** ACC-075, ACC-122, LIB-142, MUS-066, MUS-230.
- **Security.** Boundaries TB4; threats TM-T09, TM-T16. Verifies
  SEC-API-027, SEC-API-029, and the client's half of SEC-API-028 and
  SEC-IAM-043.
- **Builds.** Capability URLs for originals, packaged segments and
  artwork in the layout's sizes, asked for when needed and refreshed
  silently; a refused range ends playback with the plain message and,
  when the session is gone, the wipe; the typed stream-limit refusal.
  It adds streams, expiry and stream limits to the contract run
  (CP-055).
- **Tests.** Against the real server: a pause past expiry under an
  injected clock resumes with no error; revoking the session mid-track
  stops playback at the next range request and the client wipes before
  showing anything; a capture of network and storage finds no capability
  URL in Cache Storage, the media session or a log.
- **Done when.** Its tests against the real server pass: silent
  refresh, the cut on revocation, and no capability URL in any store.

### CP-058 The R1 flows, end to end in a browser

- **Wave** C5 · **Size** L · **Depends on** every client package; the R1
  server packages, beside WP-117 (wave 6).
- **Owns** `clients/e2e/flows/`.
- **Serves** The browser side of F01, F02, F03, F05, F06, F09, F10, F12,
  F15 and F16, and the browser steps of F13 and F14.
- **Security.** Boundaries TB1, TB2, TB4, TB5; threats TM-T07, TM-T33,
  TM-T40. Verifies SEC-TM-053 (the client's half), SEC-API-044,
  SEC-API-046, SEC-API-049, SEC-CLI-009, SEC-CLI-012, SEC-CLI-013,
  SEC-CLI-027, SEC-PRV-019, SEC-TM-058.
- **Builds.** One test per flow's main path, driving a real browser
  against a real server process with a synthetic library; failure
  branches stay at the lower layers, as flows.md asks. The whole-client
  security runs: every screen under the server's own policy header in
  three engines; the full suite behind a proxy that refuses every host
  but the server; the hostile library scanned by the real server and
  shown literally on every surface; storage inspected after sign-in in
  both modes and after sign-out; the server's access log searched for
  the invitation, pairing and setup secrets. It adds to the contract run
  (CP-055) every answer from server waves 4 to 6 and every failure
  switch the fake has, and it is not done while `ports/src/provisional/`
  holds a file.
- **Tests.** These are the tests.
- **Done when.** Every flow test passes against the real server, the
  contract run covers every `ServerPort` method, and no provisional type
  remains.

### CP-059 Browser support list and the release accessibility script

- **Wave** C5 · **Size** S · **Depends on** CP-010, CP-058.
- **Owns** `docs/ui/browser-support.md`, `docs/ui/accessibility-script.md`.
- **Serves** CLI-002, CLI-135, CLI-136.
- **Security.** Boundaries TB4; threats TM-T69. Verifies SEC-API-052 (the
  review of each browser feature's fallback).
- **Builds.** The published list of supported browsers, which is also
  the test matrix; the written screen-reader script that is run by hand
  before each release (design-language requirements A13 and A21).
- **Tests.** A test fails when the Playwright projects differ from the
  published list.
- **Done when.** The test comparing the list with the Playwright
  projects passes, and the script is written.

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
| F03 Signing in on a new device | CP-035, CP-036, CP-039, CP-062 |
| F05 Playing an album and editing the queue | CP-017, CP-019 to CP-022, CP-024 to CP-028, CP-034, CP-060 |
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
| SEC-CLI-013 | Secrets in the fragment, off the address bar | CP-062, CP-037, CP-038, CP-039, CP-058 |
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
| SEC-SUP-035 | Direct dependencies listed with reasons | CP-001, then every package that adds a dependency |
| SEC-SUP-036 | Registry signatures verified (client CI) | CP-001 |
| SEC-HIS-027 | All media text as text; sinks fail the lint | CP-002 (lint), CP-006 |
| SEC-STD-012 | Untrusted keys in `Map`; no merges | CP-002 (lint), CP-005 |

SEC-CLI-025 (one core parser for every inbound link) is WP-089's in the
backend plan. Its client half, that no client code parses an address, is
CP-009's and CP-062's.

## Requests to the backend plan

This plan owns no Rust. What it needs from the backend plan is listed
here. Requests 1 and 2 are applied to [work-packages.md](work-packages.md)
in the same pull request as this plan, so the two documents agree. The
rest are for that plan's owner and integrator to apply or refuse.

1. **The facade is four backend packages, not one (applied).** WP-235
   (wave 1) creates `crates/gunmetal-wasm` and proves the type mechanism
   and the lint answer on WP-005 and WP-006. WP-236 (wave 2) exports the
   queue, shuffle, gain, the player state, lyrics and the catalogue
   types. WP-237 (wave 2) exports user events and the event sink. WP-088
   (wave 3) keeps the rest and no longer creates the crate. The numbers
   follow the backend plan's highest, WP-234.
2. **Owner decision 34, `unsafe` in the facade (applied as a technical
   answer).** See
   [Core logic before the WASM facade](#core-logic-before-the-wasm-facade)
   and the register.
3. **Split the inbound-link parser out of WP-089.** `deeplink.rs` and
   SEC-CLI-025 are WP-089's, in wave 3, but the parser is a pure core
   function over typed values that needs only WP-005 and WP-006. The
   request is a small wave 1 package (WP-239 is the number this plan
   suggests) that owns `crates/gunmetal-core/src/deeplink.rs` with its
   unit, property and fuzz tests, leaving WP-089 the resolution routes;
   WP-237 then exports it. Until then the client's router matches only
   fixed, secret-free paths (CP-009), and the screens that read a link
   (CP-037, CP-038, CP-039, through CP-062) are not started.
4. **A core function for artwork tints.** design-language section 5
   gives the rule: lightness and chroma by theme, reduction into sRGB,
   the contrast check against every token on the surface, 0.01 lightness
   steps, no tint below a chroma floor, and the placeholder hue from an
   item's ID. No backend package owns it; WP-037 makes the candidates and
   nothing a browser calls. The request is a small wave 1 core package
   (WP-238 is the number this plan suggests) owning
   `crates/gunmetal-core/src/tint.rs`, which takes the candidate and the
   token colours as numbers and returns the surface colours; WP-237 then
   exports it, and CP-061 is its one caller.
5. **What the client needs from WP-088**, beyond what it names: reads
   over the in-memory library (an item by ID, a list in a named order);
   loading that library from catalogue records as well as from frames
   (CP-023); decoding of route responses as well as sync frames, with
   mirror types, so C3 and C4's provisional types can be deleted
   (SEC-CLI-021); the applied gain as a factor; the audit-head extension
   check (SEC-OPS-075); and a QR matrix.
6. **A QR encoder in the core**, shared by the server's console code and
   the client. If refused, CP-038 and CP-039 file an npm dependency
   request instead.
7. **WP-124: two small changes to `xtask js-deps`.** Skip a dependency
   whose version starts with `workspace:` and whose name is a workspace
   member, so workspace packages can name each other. And decide, in one
   place, that a fixture manifest is a manifest: this plan keeps its
   fixture manifests free of dependencies so the check needs no skip
   list; if WP-124 prefers a skip for a fixtures directory, CP-001
   follows it.
8. **WP-127's traceability check and docs lint read this plan and
   TypeScript.** It has to find `// Verifies:` lines under `clients/`,
   and to accept `CP-NNN` entries in `docs/plan` under the SEC-TM-001
   rule.
9. **WP-136 runs CP-001's check** of the package-manager settings in the
   release workflow, instead of writing a second one.
10. **WP-072 owns putting the bundle into the server binary.** It owns
    `crates/gunmetal-server/src/webapp/` and already serves "from bytes
    embedded in the binary, from a build-time manifest", tested with a
    two-file stand-in. The workspace allows no build script
    (SEC-SUP-026), so the request is that WP-072 states the mechanism,
    for example an xtask command that reads CP-056's manifest and writes
    the asset table the server includes, behind a cargo feature whose
    absence leaves the stand-in. The build order is then: facade for
    `wasm32`, client bundle, that xtask step, server. The Rust half of
    the gate never needs the bundle.
11. **The D-74 spike is CP-003**, not WP-072.
12. **WP-095 and WP-132 embed CP-004's stylesheet** for the startup and
    help pages, so those pages share the tokens.
13. **WP-115's list render budget** (DIS-100) is CP-015's test, as that
    plan already expects.
14. **Segment fixtures from the packager.** An xtask command, owned by
    WP-056 or a small package after it, that runs the audio packager
    over generated tones and writes fragmented MP4 fixtures with a
    manifest, for CP-034.
15. **CI and the gate** take the requests in
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
   map (CLI-155) says two explicit answers and no preselection. The plan
   builds no preselection
   ([question 4](#product-questions-for-the-owner)).
4. **Run-time styles.** design-language allows no style element with
   text and no style attribute, and marks React Native for Web's fit as
   unverified. The plan does not assume it fits: CP-003 proves it under
   the real policy before any screen is written, and a loosening is the
   owner's decision (D-74).
5. **The names of two lint rules.** The verification columns of
   SEC-CLI-001, SEC-API-045, SEC-MED-057 and SEC-HIS-027 name
   `react/no-danger` and `react/jsx-no-script-url`, rules of
   `eslint-plugin-react`. That plugin does not state support for ESLint
   10, and ESLint 9 is past its end of life. The requirements themselves
   forbid the constructs, not a plugin, so the plan meets them with the
   project's own rules of the same effect (CP-002) on a supported
   linter. The baseline's owner may want the columns to name the
   construct.
6. **Item addresses.** surfaces.md has Back return to the exact place
   (DIS-112), which the plan keeps through the browser's history state.
   It does not give each album its own address in R1, because every
   inbound address must be parsed by the core (SEC-CLI-025) and the
   core's R1 route set holds only the links R1 issues; links to items are
   R1.2 (CLI-034).

## Open questions

### Product questions for the owner

On 2026-10-03 the recommended option of each question was accepted by
default, so the build is not held up. They are defaults, not answers in
the register: the owner may change any of them, and each says what would
change.

1. **How do you want to open each demo build?**
   - **A (default accepted).** One command on your own machine; the demo
     opens in your browser at a local address. Nothing is hosted and
     nothing leaves the machine.
   - B. A private preview link for each client wave, holding only the
     made-up library. It needs a place to host it and a rule that the
     preview never talks to a real server.
   - C. Both.
2. **What should the demo play?**
   - **A (default accepted).** Made-up artists and albums with generated
     tones and generated covers. Nothing to license, and the same on
     every machine.
   - B. A few public-domain or CC0 recordings added to the repository,
     so the demo sounds like music. Each adds a binary file and a
     provenance record.
   - C. Tones in tests, and a small CC0 set only in the demo.
3. **Player keyboard shortcuts in R1** (register D-73).
   - **A (default accepted).** Ship Space, Shift+Left, Shift+Right,
     Shift+N, Shift+P and M, with the off switch WCAG requires (CP-025).
   - B. No shortcuts in R1; they arrive with the command palette in R2.
     CP-025 then drops them.
4. **"Is this your own device?" at sign-in.**
   - **A (default accepted).** Two answers, neither preselected, as the
     feature map says (CLI-155). Nobody keeps data on a shared computer
     by pressing Enter.
   - B. "Yes, keep me signed in" preselected, as the security guidance
     sketches. One less decision at every sign-in.
5. **Which browsers does R1 call supported** (CLI-002)? The list is also
   the test matrix.
   - **A (default accepted).** The two latest versions of Chrome, Edge,
     Firefox and Safari, on computers and phones, tested in CI through
     the Chromium, Firefox and WebKit engines, with a pass by hand on
     real Safari and an iPhone before each release.
   - B. The same, plus Firefox ESR.
   - C. Only the latest version of each.
6. **After the first clickable build, what comes next?** Listening (C2)
   and sign-in and account (C3) each need only C1 and can be built at
   the same time. Administration (C4) needs C3, because every admin
   screen opens through C3's step-up prompt and settings, so it can
   never come before C3.
   - **A (default accepted).** Listening and sign-in at the same time;
     administration after sign-in. The player gets good while the pieces
     the real server needs first are made ready.
   - B. Sign-in first, then administration, with listening held until
     both are done. The real server is usable from a browser soonest,
     and the player stays at the first clickable build for longer.
   - C. Listening only, with sign-in and administration held until
     server wave 3 lands.

D-74 (what to do if React Native for Web cannot run under the style
policy) is not defaulted. It stays the owner's, and it only arises if
CP-003's spike fails.

### Technical choices settled here

Each is recorded in
[record 12](../adr/0012-web-client-toolchain-and-contracts.md), with the
alternatives. The ones that answer an open register decision are also in
the register's technical answers of 2026-10-03.

1. **The workspace is `clients/`**, with its own lockfile. The root
   stays a Rust workspace, and the dependency check finds every
   `package.json` in one place.
2. **pnpm**, as the register recommends (D-65), on Node 24.
3. **React Native for Web from the first screen**, because record 1
   decides one React Native interface. Writing the web client on plain
   DOM components first would mean rewriting every screen for R2.
4. **Vite, Vitest, StrykerJS, ESLint, Prettier, Playwright and axe**:
   one well-known tool per job, each named by the baseline or needed by
   a testing rule, each pinned to its newest release at least seven days
   old.
5. **TypeScript 6.0, Vitest 4 and ESLint 10.** TypeScript 6.0 because
   typescript-eslint does not yet state support for 7. Vitest 4 because
   StrykerJS has not stated support for 5. ESLint 10 because 9 is past
   its end of life; the two React rules the baseline names become the
   project's own.
6. **No router, state, schema, internationalisation, icon, font,
   component, virtual-list or storage stand-in package, and no types
   package for React Native for Web.** Each is small enough to own, or
   already belongs to the core.
7. **Two ports**, with the fake and the real server as two
   implementations of one interface, chosen in one file.
8. **The fake is in the page, in memory.** A second server process would
   be a second thing to keep honest.
9. **No core logic in TypeScript, even temporarily**, and no client
   package owns Rust. The facade is backend packages, built in slices as
   the core's modules merge.
10. **Types cross from Rust by generation.** Mirror types in the facade
    with exhaustive conversions, declarations generated by `tsify`,
    committed and drift-checked, and no hand-written copy in the client.
11. **`unsafe`:** forbidden in the core with no exception; the facade
    alone carries the narrowest exception generated code needs, proved
    on `wasm32` by WP-235 before any other slice starts.
12. **No client wave branches.** Client packages merge into the open
    server wave branch.
13. **The contract run compares decoded values under a fixed seed and
    clock**, covers the hand-written demo library as well as the
    recording, and is a step of the gate.
14. **Coverage and mutation apply to one stated set of files.** Browser
    adapters are outside it by where they live, hold no logic by lint,
    and are covered in real browsers.
15. **The layout contract is measured positions**, not stored
    screenshots: the gate compares the position and order of the pinned
    controls with literal numbers. It is exact, readable in review, and
    adds no binary baselines that differ by machine. Screenshots are
    still produced for a person to look at. If the owner wants pixel
    comparison as well, it is one more sweep.
16. **History is not kept in browser storage in R1**, the stricter of
    two baseline rows.
17. **Item pages have no address of their own in R1**; the router
    matches fixed paths and the core parses everything else.
18. **The web gate is part of `scripts/gate.sh`**, not a second script,
    so there is still one definition of done.

## What this plan could not verify

- Whether React Native for Web's run-time styles pass `style-src 'self'`
  in each engine. CP-003 settles it.
- Whether Vite's production output needs any Trusted Types sink beyond
  the one loader policy, and whether Vite compiles JSX without its React
  plugin.
- Whether the pinned versions work together. Their existence, names,
  licences, publication dates and stated peer ranges were read from the
  npm registry on 2026-10-03; nothing was installed.
- Whether `tsify`'s derive and `serde-wasm-bindgen` compile under the
  facade's lint exception, write declarations precise enough for every
  mirror (data-carrying enums above all), and count sensibly under the
  coverage tool. WP-235 settles it, and nothing else starts until it
  has.
- Whether `wasm-bindgen`'s generated code needs an exception to
  `unsafe_code` at all on `wasm32`, and how narrow it can be.
- The exact names of pnpm 12's settings for release age, trust policy,
  exotic sources and build allow-lists, and the output format of its
  licence listing.
- Whether StrykerJS can limit a run to changed lines, as cargo-mutants
  does, or only to changed files; and how long a static mutant costs,
  which CP-002's canary measures.
- Whether Playwright's script coverage is precise enough to hold browser
  adapters to 100%, and that it exists only in Chromium.
- Whether the web gate fits its time budget on the build machine.
- Which audio formats each supported browser decodes from a file, which
  containers each accepts in Media Source, and how Safari's
  `ManagedMediaSource` differs. These were already unverified in the
  feature map (MUS-032, MUS-067, MUS-230).
- Whether a browser test can capture decoded output to prove a gapless
  join to the sample in each engine.
- Whether Playwright's virtual passkey authenticator exists outside
  Chromium.
- Whether a page's history state survives a reload in every supported
  browser, which the R1 item pages rely on.
- Background playback in iPhone browsers (already unverified in
  CLI-003).
- Whether WP-127's traceability check and docs lint, which were still
  being built, read TypeScript and `CP-NNN` entries.
- Whether the server can be made to produce equal decoded values across
  two runs with a fixed seed and clock, which the contract run needs.

## Review notes

An adversarial review of the first draft on 2026-10-03 found six high,
eight medium and three low problems. All were checked against the
sources. One sub-point was wrong (WP-124's check does read
`peerDependencies` and `optionalDependencies`); the rest were right and
are applied. What changed:

- **User events.** Loves and play events left the first clickable
  milestone. They are made only by the event sink (CP-060), over the
  facade's user-event slice (WP-237).
- **Inbound links.** Only the core parses them (SEC-CLI-025). The router
  matches fixed paths (CP-009), CP-062 hands everything else to the
  core, and the backend plan is asked to split the parser out of WP-089.
- **Types.** TypeScript types are generated from mirror types in the
  facade, committed and drift-checked. Check 2 now says what it catches.
- **The facade crate has one owner.** Its Rust is WP-235, WP-236, WP-237
  and WP-088; client packages are TypeScript only. `unsafe` has a
  technical answer and a stop condition.
- **Branches.** There are no client wave branches. The false statement
  that C2, C3 and C4 are independent is corrected: C4 needs C3.
- **Dependencies.** The adding package adds its own line to the list;
  WP-124 is asked for a `workspace:` skip; registry names are refused in
  peer and optional dependencies.
- **Threat model.** Every package names its trust boundaries and threats
  (SEC-TM-001).
- **Coverage and mutation** have a stated set, stated exclusions with
  how each is proved, a canary with each kind of file, and fixed
  concurrency, timeout and time budget.
- **Sweeps** have a defined target before CP-053 and a completeness
  test.
- **The contract run** compares decoded values under a fixed seed and
  clock, covers the hand-written library, has owners for each server
  wave's additions and is a gate step.
- **Pins.** Every tool is the newest release at least seven days old;
  Vitest 4; ESLint 10 with two owned rules; an owned declaration file
  instead of `@types/react-native-web`; `@testing-library/dom` listed;
  Node 24.
- **The bundle embed** is WP-072's, with a stated build order.
- **Artwork tints** live once, in the core, behind CP-061.
- **Dependencies between packages** that were missing are declared, and
  the bar's badge and heart are places that later packages fill.
- **"Done when" lines** state what the package's own tests observe.
