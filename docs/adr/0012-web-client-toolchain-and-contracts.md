# 12. The web client's toolchain, its contracts and its use of the core

Date: 2026-10-03
Status: proposed. It records the technical choices behind the
[client plan](../plan/client-packages.md), which the owner reviews with
that plan. Revised on 2026-10-03 after two adversarial reviews of the
plan: decisions 2, 9, 10 and 12 changed and decisions 13 to 17 were
added. The choices that answer open register decisions are also recorded
there as delegated technical answers (D-02). On the same day the owner
approved the two Rust crates of decision 13 and the linter change of
decision 2. The owner's product choice it rests on, to start the web player
now against a small fake server, is in the
[decision register](../decisions.md#owner-answers-2026-10-02).

## Context

[Record 1](0001-architecture.md) decides one Rust core, compiled into the
server and, through WebAssembly, the web client (decision 2), and one
React Native interface in TypeScript for every platform (decision 8).
[Record 2](0002-music-is-first-class.md) makes the first usable release a
music server with a web client. [Record 6](0006-workspace-and-dependencies.md)
lays out the Rust workspace and names the `gunmetal-wasm` crate. None of
them says how the client is built, tested or kept in step with the
server, and the [backend plan](../plan/work-packages.md) leaves the client
out.

On 2026-10-03 the owner chose to build the web player in parallel with
the server waves, against a fake server, so that there is a screen to
click within days and it moves to the real server as the server waves
land. Three things follow that are architecture and not just planning:

- The client needs its own toolchain, held to the testing rules in
  CONTRIBUTING.md (test first, deep assertions, 100% coverage, no
  surviving mutant) and to the security baseline's rules for JavaScript.
- A client built before its server needs a contract that makes the later
  swap a configuration change and makes disagreement a failing test.
- The facade that hands the core to the browser (WP-088) is in server
  wave 3, and the client needs the queue, shuffle, gain and player rules
  before then. Record 1 exists so that those rules are written once.

## Decisions

1. **The JavaScript workspace is `clients/`, beside `crates/`.** It has
   its own lockfile and holds every `package.json` in the repository.
   Its packages are split so that the part record 1 shares with native
   apps (`ui`, `app`, `ports`, `tokens`) holds no browser-only code, and
   the web-only parts (`player`, `core-wasm`, `http-server`, `apps/web`)
   are the ones R2 replaces with native modules and UniFFI.

2. **One well-known tool per job, each pinned to its newest release at
   least seven days old.**

   | Job | Tool |
   |---|---|
   | Runtime for the tools | Node 24, the long-term support line |
   | Package manager | pnpm 12 |
   | Language | TypeScript 6.0, strict |
   | UI | React 19 with React Native for Web |
   | Bundler and dev server | Vite 8 |
   | Unit and component tests | Vitest 4, jsdom, Testing Library |
   | Coverage | Vitest's V8 coverage, thresholds at 100 per file, branches included |
   | Mutation testing | StrykerJS 10 with its Vitest runner and TypeScript checker |
   | Lint and format | ESLint 10 with typescript-eslint, the React hooks plugin and the project's own rules; Prettier |
   | Browser tests | Playwright in Chromium, Firefox and WebKit, with axe |
   | WASM bindings and types | `wasm-bindgen`, `tsify`, `serde-wasm-bindgen` |

   Three pins are deliberately not the newest major. TypeScript 6.0,
   because typescript-eslint does not state support for 7. Vitest 4,
   because version 5 was days old and StrykerJS, released before it, has
   not stated support for it. And the linter is ESLint 10, not 9: ESLint
   9 reached end of life on 2026-08-06 by ESLint's own support page, and
   `eslint-plugin-react` supports nothing newer, so the two React rules
   the baseline names are written as the project's own rules instead of
   keeping an unsupported linter. That changes how four baseline
   requirements are verified (SEC-CLI-001, SEC-API-045, SEC-MED-057 and
   SEC-HIS-027 name the plugin's rules), and the owner approved it on
   2026-10-03. `@types/react-native-web` is not used,
   because it depends on `react-native` and would bring React Native and
   Metro into the lockfile; the client owns one small declaration file.
   The exact package names, pins, publication dates and the reason for
   each are in the client plan, because a person checks every dependency
   before it is added (AGENTS.md).

3. **The testing rules mean the same in TypeScript as in Rust.** Coverage
   and mutation apply to one stated set of files (decision 15), and no
   file in it is excluded. The gate fails unless every mutant was killed; a
   timeout is a failure, not a detection. Snapshot assertions are banned,
   because a snapshot is an expected value derived from the code under
   test. Every escape hatch (skipped tests, lint and type suppressions,
   coverage and mutation ignore comments) fails the lint.

4. **The web checks are part of `scripts/gate.sh`.** There is still one
   definition of done. The client asks the integrator for the change; it
   does not edit the script.

5. **JavaScript dependencies follow the baseline from the first
   install.** Frozen lockfile, one registry, install scripts off, a
   seven-day minimum age, verified signatures, a listed reason for every
   direct dependency, a licence allow-list and a deny-list of tracking
   packages (SEC-SUP-033 to SEC-SUP-036, SEC-SUP-029, SEC-CLI-018,
   SEC-CLI-027). The client adds no router, state, schema,
   internationalisation, icon, font, component or analytics package.

6. **The web build renders React Native primitives, and that is proved
   under the production content security policy before any screen is
   written.** The first client package after the gate builds one page
   with React Native for Web and loads it under the exact policy of
   SEC-API-044 in three browser engines. If its run-time styles do not
   pass `style-src 'self'`, the order of fallbacks is the register's
   (D-74), and a loosening is the owner's decision, never a quiet one.

7. **The screens talk to two ports and nothing else.** `CorePort` is
   everything record 1 puts in the core. `ServerPort` is everything
   api-needs.md makes a network call. Each has a stand-in and a real
   implementation, and the only difference between the demo and the
   product is which ones the composition root passes in.

8. **The fake server is fixtures and an in-memory `ServerPort`, in the
   page.** It is not a process and has no socket. It holds no rule: it
   orders queue operations by calling the core, as the real server does,
   and it never sorts, searches, matches or decides. It verifies no
   credential, so no test of it proves a security property of the
   server. The production build fails if it contains any of it.

9. **No core logic is written in TypeScript, even as a stand-in, and no
   client package owns Rust.** The client reaches core logic only
   through `CorePort`. That covers the queue, shuffle, gain, the player
   state, lyrics, search, Home rows, the playback decision, user events
   with their IDs and clock, parsing inbound links (SEC-CLI-025), and the
   rule that turns an artwork colour into a surface that passes the
   contrast check. Each of those rules has a core package; where none
   existed, one was added (the event builder and sink, the inbound-link
   parser split from WP-089, the tint rule, the QR matrix, the
   audit-head check: WP-238 to WP-242). The facade crate has one owner,
   the backend plan, and is built in slices: WP-235 in wave 2 (the
   crate, the mechanism of decision 13, the lint answer of decision 14,
   the catalogue types), WP-236 in wave 2 (the playback rules, merging
   into `wave-2` after WP-235), WP-237 in wave 3 (conversion for the
   added core packages), and WP-088 in
   wave 3 (sync, the library, search, the decision, Home rows, response
   decoding). Each export is a direct call with conversion only; no rule
   is written in the facade. Until a slice exists, the screens
   that need it are not started. The one thing the fixtures answer
   directly is reading the library by ID and in an order the fixture
   already holds, which is a lookup, not a rule.

10. **Four checks keep the fake and the server in step**, each as soon as
    the Rust side exists: every generated problem code has a sentence;
    the generated types of decision 13; every call the HTTP adapter makes
    against the committed `openapi.json`; and one conformance suite run
    against the fake with the hand-written demo library, the fake
    replaying a recording, and a real server process. The real server
    runs with a fixed seed and an injected clock, and recordings are
    compared after decoding, value by value, never by a hash of bytes.
    The run is a step of the gate. It is the test that fails when the
    two disagree, for the `ServerPort` methods it has reached; the
    packages that extend it per server wave are named in the plan.

11. **R1 plays through the browser's own decoders, by two paths the core
    chooses between.** The original file through an audio element, and
    fragmented MP4 from the server's audio packager
    ([record 4](0004-audio-packager.md)) through Media Source for
    sample-accurate gapless joins and exact seeking. The client applies
    the core's gain decision through one Web Audio gain node and computes
    nothing itself. The player state every surface shows is the core's.

12. **The layout contract is checked as measured positions.** The gate
    compares the position and order of the pinned controls with literal
    numbers in three engines, instead of comparing stored screenshots.

13. **Types cross from Rust to TypeScript by generation, never by hand.**
    `wasm-bindgen` alone types only simple exports, and the core cannot
    carry its attributes (record 6). So for each core type that crosses,
    the facade holds a mirror type whose conversion takes the core value
    apart field by field with no catch-all: a core field renamed,
    retyped, added or removed stops the facade compiling. That holds for
    core types with public fields. A type with private fields (`Link`,
    `PublicId`, `Lufs`) is read through its accessors, so a removed or
    retyped accessor is caught and an added one is not; review of the
    core change has to catch that. The mirrors
    derive `serde` and `tsify`, values cross through
    `serde-wasm-bindgen`, and the TypeScript declarations are generated,
    committed under `crates/gunmetal-wasm/types/` and regenerated by an
    xtask check that fails on any difference. The client imports only
    those declarations, and lint refuses a hand-written type for a value
    that crosses a port. `tsify` and `serde-wasm-bindgen` were requested
    through WP-235 and approved by the owner on 2026-10-03; neither is a
    dependency of the core. WP-235 proves the mechanism on WP-005's,
    WP-006's and WP-040's types before anything else is built on it. What the mechanism does
    not catch is a change of behaviour behind unchanged types; the
    conformance suites are for that.

14. **`unsafe`: the core keeps `forbid` with no exception; the facade
    alone may carry the narrowest exception generated code needs.** This
    answers the backend plan's owner decision 34 as a delegated technical
    choice. A `forbid` set by the workspace cannot be lowered by an
    attribute in source, so the exception is in the facade's manifest:
    it does not take the workspace lint table but repeats every
    workspace lint, with only `unsafe_code` lowered as far as generated
    code needs, and a test fails if it stops repeating one.
    `xtask lint-exceptions`, which refuses any manifest that does not
    take the workspace lints except the core's, is changed to permit
    this one manifest too. Hand-written `unsafe` stays refused in the
    facade by an xtask check on the keyword. `wasm-bindgen`'s generated
    items are compiled only for `wasm32`, so the proof is a `wasm32`
    build under the lint in the gate, exporting WP-005's link filter. No other facade slice, and no
    client package that needs one, starts before that build passes.

15. **Coverage and mutation apply to a stated set, and what is outside
    it is outside by where it lives.** The set is each package's `src/`,
    each app's `src/` and the client's own tools. Browser adapters (one
    file per browser interface, in a `browser/` directory) are outside
    it: jsdom has none of those interfaces, and a stand-in would test the
    stand-in. Lint allows no branch, loop or arithmetic in an adapter, so
    the logic stays in `src/`; the browser suite calls every adapter
    export in three engines, and Chromium's script coverage of the
    adapters must be 100%. Adapters are not mutated. Module-level data is
    kept in JSON or returned by functions, so no mutant is static. The
    gate sets fixed worker counts, a fixed timeout and a time budget for
    the shared build machine, in the committed configuration.

16. **There are no client wave branches.** A client package branches
    from, and merges into, the server wave branch that is open and holds
    what it depends on, through that wave's integrator; the wave's one
    pull request into `main` carries both. Client waves are groups by
    what the owner can test, not branches. This keeps one branch per
    server wave and one pull request per wave into `main` (D-01), and
    puts the facade crate on the same branch as the code that calls it.

17. **The client's router matches fixed, secret-free paths and parses
    nothing.** Which item a page shows is kept in the browser's history
    state. Any address with a fragment, a query string or an unknown
    path, and any QR payload, goes whole to the core's parser. Addresses
    for single items arrive in R1.2 with the core's routes for them.

## Alternatives considered

- **Plain React on the DOM for R1, React Native later.** Fewer packages
  now and no question about run-time styles. Rejected because record 1
  decides one React Native codebase, and every screen written on DOM
  elements would be rewritten for the R2 apps. The risk it avoids is
  taken first instead (decision 6).
- **Expo and Metro for the web build.** The usual way to start a React
  Native project. Rejected for R1 because it brings a far larger
  dependency tree than a web-only build needs, against the baseline's
  rule to keep direct dependencies few. The R2 client plan chooses the
  native toolchain; the shared packages do not depend on this choice.
- **npm as the package manager.** It now has the same protections.
  Rejected only because the register already recommends pnpm (D-65).
- **Jest instead of Vitest.** Rejected because Vitest runs the bundler's
  own transform, so tests and the product compile the same way.
- **A fake server as a separate process** (a small Node or Rust HTTP
  server). It would exercise the HTTP adapter earlier. Rejected because
  it is a second server to keep honest, needs a route table that does
  not exist until wave 2, and gives the owner nothing more to click.
- **A network-mocking library in the page.** Rejected: it fakes at the
  HTTP level, which again needs routes that do not exist yet, and adds a
  dependency and a service worker that R1 otherwise does not have.
- **Client wave branches (`client-0` to `client-5`).** The first draft
  had them. Rejected: server wave 3 starts on top of `wave-2`, so a
  facade crate created on a client branch would not be where WP-088
  needs it, and a client pull request would carry a whole unmerged
  server wave to the owner.
- **Client packages owning Rust files in the facade.** The first draft
  had them. Rejected: one crate would have had two owners in two plans.
- **Hand-written TypeScript mirrors of the Rust types.** Rejected: they
  drift silently, which is the failure the contract exists to prevent.
- **`ts-rs` for type generation.** Well known, but its derive sits on
  the type itself, which would put a new dependency into the core.
  `tsify` on mirror types keeps the core's dependency list unchanged.
- **Keeping ESLint 9 for `eslint-plugin-react`.** Rejected: an
  end-of-life linter held for two rules that are small to own.
- **Mock-testing browser interfaces in jsdom to reach 100%.** Rejected:
  it asserts what the mock does. Thin adapters proved in real browsers
  are honest about what was tested.
- **A temporary TypeScript queue and player state machine.** The fastest
  way to a playing demo. Rejected because it is exactly the duplicated
  logic record 1 forbids: it would be tested, mutated and then thrown
  away, and the screens would be proved against rules that are not the
  product's.
- **Waiting for WP-088 before building any player screen.** Rejected by
  the owner's choice to start now.
- **Stored screenshots for the layout contract.** They catch more than
  positions, but they are binary baselines that differ between machines
  and cannot be read in review. Screenshots are still produced for a
  person to look at; a pixel comparison can be added as one more sweep.

## Consequences

- The facade is built by four backend packages instead of one, and
  WP-088 no longer creates the crate. Five small core packages were
  added for rules the client needs. The backend plan carries all of it.
- Two Rust crates join the facade's dependencies (not the core's),
  approved by the owner.
- The questions that were WP-088's risks (the `unsafe` lint, coverage of
  generated glue) are answered first, in wave 2, by WP-235, and
  everything else on the facade waits for that answer.
- The gate needs Node, pnpm, a `wasm32` target and three browser builds,
  and takes longer. The Rust and web halves can run as two jobs, and the
  web half has a stated time budget.
- The first clickable build depends on WP-235 (wave 2) and WP-236
  (wave 2, merging into `wave-2` after WP-235), so the frame and the
  first clickable build land with server wave 2. If a package is late,
  the milestone waits; nothing is written in TypeScript to cover for it. It
  records no plays and no loves, which arrive with WP-237 and WP-240.
- Browser adapters are not mutation-tested. That is a stated limit, kept
  small by the lint that allows no logic in them.
- R1 pages for single items have no address of their own.
- Until server wave 3, the fake's behaviour is checked against the
  server only by types, the route table and review. The fake is kept
  small for that reason.
- The traceability check has to read `Verifies:` lines in TypeScript.
- Sixteen R1 security requirements that the backend plan left
  unassigned, and the client's half of eleven more, now have owners.

## When to revisit

- If React Native for Web cannot run under the policy without
  `'unsafe-inline'` for styles and the owner declines that loosening,
  decision 6's fallback is a styling path that emits only static
  stylesheets, decided in a new record.
- When the R2 client plan chooses the native toolchain, check that the
  shared packages still hold no browser-only code.
- If the web gate's mutation run grows past what a pull request can
  wait for, shard it as the Rust gate plans to, not by excluding code.

## Addition, 2026-10-05: how mirror values cross, and the one exception to the coverage and mutation rules

Decisions 13 and 14 stand as written above. This section records what
building WP-235 found about them and what the owner decided on
2026-10-05 ([pull request 78](https://github.com/PremierStudio/gunmetal/pull/78),
owner question 1). It also records, marked as such, two things WP-235
proposes that the owner has not answered: questions 7 and 3 of that pull
request.

**What was found.**

- The code `wasm-bindgen` and `tsify` generate builds under
  `unsafe_code = "forbid"`, for `wasm32` and for the host. The exception
  decision 14 allows is not needed today: the facade's manifest repeats
  the workspace's lint tables and lowers nothing, and its
  `tests/workspace_rules.rs` refuses any lowering.
- Host coverage does not count that generated code. With `tsify`'s
  derives on the mirror types, the report holds only the functions
  written by hand.
- `tsify` 0.5.8 deprecates the two attributes that let a mirror type be
  the parameter or the return type of an exported function
  (`into_wasm_abi` and `from_wasm_abi`), because a value that fails to
  convert leaks memory. What it offers instead is the wrapper type
  `tsify::Ts<T>`, converted inside the exported function. That
  conversion calls into JavaScript, so a function that performs it
  cannot be called by a test on the host.

**What the owner decided.** In his comment on pull request 78
(2026-10-05, 17:11 UTC) the owner chose "A: `tsify::Ts<T>` and
`Result<_, JsError>`, with a one-line `wasm32`-only wrapper per export".
That decides four things:

1. Mirror values cross as `tsify::Ts<T>`, and an exported function
   returns `Result<_, JsError>`: a value that does not convert becomes
   a JavaScript error.
2. Every export is a plain function, tested on the host under the usual
   rules (100% coverage and no surviving mutant), and a wrapper compiled
   only for `wasm32` that converts the types and calls it. In the
   owner's words, "A wrapper holds no logic: no branch, no arithmetic,
   no decision."
3. The wrappers are the one written exception to the coverage and
   mutation rules in the Rust workspace: host coverage and mutation
   testing cannot see them. The `wasm32` build compiles them, and the
   web client's conformance suites (CP-005 and CP-013) are the first to
   run them.
4. Option B, running the built module in CI, is decided when WP-088
   writes its `wasm32` job.

When this addition was written the exception covered two exports:
`parseLink`, the core's link filter (`links.rs`), and `normaliseText`,
the core's text normalisation (`text.rs`). Each later export adds one
wrapper to it.

**Proposed by WP-235, waiting for the owner's answer.** The rest of this
section is the package's proposal, not a decision. It is built on the
branch so that it can be judged, and it changes if the owner answers
otherwise.

- *The wrappers come from one macro and are not written by hand
  (question 7).* The owner's wording is a one-line wrapper per export.
  WP-235 found a reason to change its form: by its own documentation,
  `cargo-mutants` does not understand conditional compilation, mutates
  a function compiled only for another target and reports each mutant
  as missed. A wrapper written by hand as an ordinary function behind a
  `wasm32` gate would therefore fail the gate with a survivor no test
  can kill. (This was read, not tried.) So the package has one macro,
  `export!` in `crates/gunmetal-wasm/src/export.rs`, write every wrapper,
  as the core's `coded!` writes its enumerations: the tool does not read
  what a macro writes. A line of the macro is given a JavaScript name,
  two function names, parameter names with their types, a mirror type
  and doc comments. It cannot be given an expression, so no wrapper can
  hold a branch, arithmetic or a value of its own, and none can differ
  from the others. The other answer to question 7 keeps the owner's
  form: wrappers by hand, left out of the mutation run by name with an
  exclusion in `scripts/gate.sh`, and a check on the shape of each.
- *A check holds the facade's own files to the macro (with question 7).*
  `xtask facade-wrappers` runs against the repository in xtask's own
  tests, so the gate runs it. It reads text and parses nothing. It fails
  when the macro's source differs by one character from the copy the
  check holds; when another Rust source of the facade holds `cfg` on any
  line but the test gate; when one holds the name `include`, `path` or
  `wasm_bindgen` in code, so that no other file is brought in as code
  and no export is written by hand; and when a line of the facade's
  manifest holds `target`. It does not see what a macro defined in
  another crate writes where the facade calls it, what another crate
  exports, the manifest beyond that one word, a value read from the
  build's environment, a directory reached through a symbolic link, or
  what a wrapper does when it runs. Its own documentation lists those
  limits, and its tests pin all but the last two.
- *The `wasm32` build is a workflow of its own and not a gate step
  (question 3).* Decision 14 puts the `wasm32` build in the gate.
  `.github/workflows/wasm.yml` builds the facade for `wasm32` on every
  pull request, with warnings as errors and under the facade's lint
  tables, finds both exports in the module, and proves that the same
  build refuses `unsafe` written by hand. It is not a step of
  `scripts/gate.sh`, and the job named `gate` does not wait for it.
  Until the owner answers question 3, that part of decision 14 is not
  met.

**Other ways WP-235 considered.** These are the package's reasons for not
taking them, not the owner's; his answer chose option A and no more.

- Keeping `tsify`'s deprecated attributes under `#[expect(deprecated)]`.
  Exports would stay plain functions that host tests call, as the plan
  assumed. Not proposed: it builds on an interface its authors are
  retiring, and keeps the leak.
- Crossing as JSON text. Every export would return a string and be
  testable on the host. Not proposed: the client would parse every
  value a second time, which is the JSON round trip the dependency
  decision of 2026-10-05 turned down with `tsify`'s `json` feature, and
  it adds `serde_json` to the facade.
