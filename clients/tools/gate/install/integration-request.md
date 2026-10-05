# CP-001 integration requests

This is a proposal for the integrator, not an enabled CI change. CP-002 must
provide the complete TypeScript gate before `scripts/gate.sh` invokes it.
Native Node test-first results do not supply coverage or mutation proof.

The proven Linux x64/glibc toolchain is Node 24.20.0, native pnpm 12.7.0 and
existing Python 3.12.3. Python 3.11 or later is needed for stdlib `tomllib`;
the actual implicit-node-gyp positive control also requires Python. The pinned
Node tarball includes npm and node-gyp, so neither needs a new npm dependency.

Bootstrap inputs must be fetched from their fixed primary URLs, checked before
extraction and placed outside the project dependency graph:

- Node: https://nodejs.org/dist/v24.20.0/node-v24.20.0-linux-x64.tar.xz
  SHA-256: `2f2c0da162318f0de47665410c7c8c2ed3d36c8f3105de4bbc61176c70a7cbf2`.
- Native pnpm: https://registry.npmjs.org/@pnpm/exe.linux-x64/-/exe.linux-x64-12.7.0.tgz
  SHA-512: `gGW7NJFmr33IJ6KZu+1w90KBtFxMVf/+AUG8aKJpy4v6dRnUd5v84PcH2ejUuxp/fm3zM0IY6h9LwcYPu9dxdg==`.
- Provisional YAML parser: https://registry.npmjs.org/yaml/-/yaml-2.9.1.tgz
  SHA-512: `3NxN8+78OdzbT7C/WjGsyfPAtJaN3FNDsWxv7Y7mcDsT/oOmgW8BpyQQFFBnvZE3j9Y2Sdz1ULFLezL7Eb2yFw==`.

The YAML parser is needed before the project install to inspect a normal pnpm
multi-document lockfile and workspace settings. Importing it from the project
only after an unchecked install would make that first validation circular.
The tested remote bootstrap extracts the checksum-verified parser into an
isolated temporary `node_modules/yaml`; copy only the reviewed `installation.ts`,
`policy.ts` and `verify.ts` check modules beside it; execute the initial check
against the real `clients/` directory with the pinned Node and pnpm on PATH.
This temporary tool copy has no alternate lockfile, installation or supplied
dependency graph. Its parser is the same provisional dependency/version.
The `bootstrap.ts` entrypoint uses only Node builtins until that extraction.
Personal Build 71 proved both a safe fixture with no project `node_modules`
and a loosened setting refusal, while a corrupt cache archive was rejected.
With the checksum-pinned toolchain already provisioned, the integrator runs:

```bash
node clients/tools/gate/install/bootstrap.ts clients
```

An optional third argument supplies a previously downloaded parser archive;
it must match the same hardcoded checksum. An offline execution path must
supply that archive, rather than allowing an unverified parser fallback.

After the initial check, run frozen installation with an explicit
`--ignore-scripts` flag, then run the committed checker again, `verify.ts`,
`licenses.ts`, and the real canary. Avoid npm/pnpm config environment overrides
and user/global config contamination. The checker now reads pnpm's effective
configuration and applies the same settings rules; Personal Build 68 proved
refusal of an actual environment override despite safe workspace settings.

The signature verifier observes actual installed manifests and the resolved
native manager executable, binds their exact identity/integrity to all lockfile
documents and live fixed-registry metadata, compares npm's loaded projection
inventory, and asks the bundled npm for signatures and present attestations.
It copies observed package bytes into a temporary projection; no second install
or lockfile is created. Current observed packages have no runtime dependencies;
a future dependency topology change needs its own inventory-composition proof.

The licence collector reads actual installed MPL source/notice files one at a
time. Only the complete standard SPDX MPL-2.0 text is exempt from the declaration
scan; a standard LICENSE cannot exempt a separate notice or source header.
Unsupported source symlinks fail closed. Personal Build 76 proved those
boundaries. This scanner does not establish historical licensing compatibility;
dependency review still requires a human to inspect licence terms and history.

Once CP-002's gate exists, integrate it into the root definition of done. Keep
Node/pnpm bootstrap and Python runtime checks in both TeamCity and CI/release
execution paths. WP-136 must reuse `installation.ts` and signature/licence
checks rather than duplicate their policy. No Zen execution is authorized.

Additional integrator requests from the plan remain: JavaScript/TypeScript
CodeQL, future client lint/test/mutation/tools CODEOWNERS, wasm/browser toolchain
pins when their owning packages exist, and the WP-124 known workspace-member
skip. This package keeps every registry dependency in the single root client
manifest until that WP-124 correction exists.

Human dependency confirmation and code-owner approval are required before
merge. No package completion claim is made without CP-002's full coverage,
zero-survivor mutation evidence, full remote gate and review.
