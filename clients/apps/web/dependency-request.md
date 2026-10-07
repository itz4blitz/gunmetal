# CP-003 dependency requests

The owner approved the client plan's named toolchain, including these two
packages, as the recommended defaults. This records the baseline checklist
for the first use of each.

## `react-native-web@0.21.3`

- **What it is for.** React Native primitives (`View`, `Text`) in a browser so
  the first page, and later every screen, is the interface the R2 native apps
  reuse (record 1, decision 8; CP-003).
- **What was considered instead.** DOM elements, which would split the web
  screens from the native ones; `@types/react-native-web`, which pulls
  `react-native` and Metro into the lockfile (the tree record 12 avoids).
- **Licence.** MIT (Meta).
- **Maintainers.** The `react-native-web` project on npm, Meta.
- **Transitive packages.** The plan names eight. None is a tracking package.
- **Published.** 2026-09-25, older than seven days at the start of this work.
- **Tarball.** `https://registry.npmjs.org/react-native-web/-/react-native-web-0.21.3.tgz`

It ships in the bundle.

## `vite@8.3.1`

- **What it is for.** The production bundler and the loopback development
  server (CP-003, SEC-CLI-019). Already present as a transitive of Vitest;
  this makes the import in `apps/web/vite.config.ts` a direct dependency.
- **What was considered instead.** esbuild or Rollup on their own, which would
  still need a dev server; Webpack, which the plan rejected.
- **Licence.** MIT (VoidZero).
- **Published.** 2026-09-24, older than seven days at the start of this work.
- **Tarball.** `https://registry.npmjs.org/vite/-/vite-8.3.1.tgz`

Development only.
