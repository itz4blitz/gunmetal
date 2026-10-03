# 12. The web client's toolchain, its contracts and its use of the core

Date: 2026-10-03
Status: proposed. It records the technical choices behind the
[client plan](../plan/client-packages.md), which the owner reviews with
that plan. The owner's product choice it rests on, to start the web player
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
click within days and it moves to the real server as server waves 2 to 4
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

2. **One well-known tool per job.**

   | Job | Tool |
   |---|---|
   | Package manager | pnpm |
   | Language | TypeScript, strict |
   | UI | React with React Native for Web |
   | Bundler and dev server | Vite |
   | Unit and component tests | Vitest, jsdom, Testing Library |
   | Coverage | Vitest's V8 coverage, thresholds at 100 per file, branches included |
   | Mutation testing | StrykerJS with its Vitest runner and TypeScript checker |
   | Lint and format | ESLint with typescript-eslint and the React plugins; Prettier |
   | Browser tests | Playwright in Chromium, Firefox and WebKit, with axe |
   | WASM bindings | `wasm-bindgen` |

   The exact package names, the versions seen on the registry and the
   reason for each are in the client plan, because a person checks every
   dependency before it is added (AGENTS.md).

3. **The testing rules mean the same in TypeScript as in Rust.** Coverage
   has no exclusions. The gate fails unless every mutant was killed; a
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

9. **No core logic is written in TypeScript, even as a stand-in.** The
   client reaches core logic only through `CorePort`. The facade crate
   is built in slices as each core module merges: the wave 1 modules
   (queue, shuffle, gain, player state, lyrics, palette, links) by a
   client package, the wave 2 modules (search, decision, Home rows) by
   another, and what needs wave 3 (sync frames, the library held in
   WASM, response decoding) by WP-088. Each export is a direct call with
   conversion only, as WP-088 specifies, so the early slices are that
   package's own work done sooner. Until a slice exists, the screens
   that need it are proved in component tests with scripted doubles that
   hold no rule, and are not wired into the demo. The one thing the
   fixtures answer directly is reading the library by ID and in an order
   the fixture already holds, which is a lookup, not a rule.

10. **Four checks keep the fake and the server in step**, each as soon as
    the Rust side exists: the client's problem codes against the core's
    catalogue; the facade's generated TypeScript declarations against
    `CorePort` at compile time; every call the HTTP adapter makes against
    the committed `openapi.json`; and one conformance suite run against
    both the fake and a real server process over the same synthetic
    library, with the fake's recorded frames re-recorded and compared in
    CI. The last is the test that fails when the two disagree.

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
- **A temporary TypeScript queue and player state machine.** The fastest
  way to a playing demo. Rejected because it is exactly the duplicated
  logic record 1 forbids: it would be tested, mutated and then thrown
  away, and the screens would be proved against rules that are not the
  product's.
- **Waiting for WP-088 before building any player screen.** Rejected by
  the owner's choice to start now.
- **Generating TypeScript types from Rust with an extra crate.** Not
  needed yet: `wasm-bindgen` already writes declarations for the facade,
  and responses are decoded by the core. It can be revisited if the
  generated declarations prove too loose.
- **Stored screenshots for the layout contract.** They catch more than
  positions, but they are binary baselines that differ between machines
  and cannot be read in review. Screenshots are still produced for a
  person to look at; a pixel comparison can be added as one more sweep.

## Consequences

- WP-088 no longer creates `crates/gunmetal-wasm`; it extends it. The
  risks that package records (whether `wasm-bindgen`'s generated code
  passes `unsafe_code = "forbid"`, and how coverage counts the glue)
  arrive in server wave 1's wake instead of wave 3. The backend plan
  needs that change applied.
- The gate needs Node, pnpm, a `wasm32` target and three browser builds,
  and takes longer. The Rust and web halves can run as two jobs.
- The first clickable build depends on six wave 1 core packages merging.
  If one is late, the milestone waits; nothing is written in TypeScript
  to cover for it.
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
