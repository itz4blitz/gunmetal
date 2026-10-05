# CP-001 integration requests

This is a proposal for the integrator, not an enabled CI change. CP-002 must
provide the complete TypeScript gate before `scripts/gate.sh` invokes it.
Native Node test-first results do not supply coverage or mutation proof.

The proven Linux x64/glibc toolchain is Node 24.20.0, native pnpm 12.7.0 and
existing Python 3.12.3. Only the canary's implicit-node-gyp positive control
needs Python, because node-gyp runs it; no check reads a policy through it.
The pinned Node tarball includes npm and node-gyp, so neither needs a new npm
dependency.

No check lets a program found on `PATH` decide a result. The parser archive is
unpacked in-process by the same strict reader that verifies the manager's
archive, the licence allow-list is read from `deny.toml` in-process, and pnpm
is run only from the verified layout below. Decoy `tar`, `python3` and `pnpm`
programs placed first on `PATH` are shown never to run. This matters once the
gate is started through `pnpm run`, which puts `node_modules/.bin` on `PATH`.

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
The tested remote bootstrap unpacks the checksum-verified parser, in-process,
into an isolated temporary `node_modules/yaml`; copy only the reviewed `installation.ts`,
`native-pnpm.ts`, `policy.ts` and `verify.ts` check modules beside it; execute
the initial check against the real `clients/` directory. It must place native
pnpm in this exact layout and export `GUNMETAL_NATIVE_PNPM_ROOT` to it; the
check does not discover pnpm from `PATH`:

```
$GUNMETAL_NATIVE_PNPM_ROOT/pnpm.tgz
$GUNMETAL_NATIVE_PNPM_ROOT/node_modules/@pnpm/exe.linux-x64/{package.json,pnpm,...}
$GUNMETAL_NATIVE_PNPM_ROOT/bin/pnpm -> ../node_modules/@pnpm/exe.linux-x64/pnpm
```

Build that layout with the committed builder, which needs only the pinned Node
and no project install. It refuses any archive but the pinned one and never
writes into a root that already exists:

```bash
node clients/tools/gate/install/native-pnpm-layout.ts "$downloaded_pnpm_tgz" "$GUNMETAL_NATIVE_PNPM_ROOT"
```

Before every use `native-pnpm.ts` re-hashes the retained archive against the
pinned SHA-512, requires the bin entry to resolve to the package executable,
compares that executable and its `package.json` byte for byte with the entries
of the archive it has just checked, and requires the metadata to be
`@pnpm/exe.linux-x64@12.7.0`. It does not check the other files beside the
executable or the Node runtime, and it cannot stop a writer that replaces the
executable between the comparison and the exec, so the layout root must not be
writable by anything else during a run. TeamCity may prepend the bin directory
to `PATH` for tools that need it, but the policy and verification code use the
verified path directly and never run a `pnpm` found on `PATH`.
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

The head of `verify.ts` lists exactly what that verifier establishes. In short:
the manager is the pinned archive and its lock entry carries the pinned
checksum; every installed directory names a locked identity; nothing an
importer depends on, and no package of a project lock document, is missing;
and the registry's metadata and signatures agree with the lockfile for those
identities. It does not hash the files of an installed project package: that
they are the locked archive's bytes rests on pnpm's frozen install with
`verifyStoreIntegrity`. A dependency with platform-optional packages or a
peer-suffixed version is not understood yet and fails closed.

The canary now gives its own verdict. It prints what each install left behind
and exits non-zero unless the secure install, run under the workspace settings
exactly as committed, installed both canaries without running their scripts,
and the control, with scripts switched on, ran both.

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

Request for the owner of `crates/xtask/src/js_deps.rs`: the client check now
reads a reason in `supply-chain/js-direct-deps.toml` as one double-quoted
string with nothing after it, so `yaml = "" # "why"` is refused. The Rust
reader of the same file still takes the text between the first and last quote
as the reason, so that line passes there. The two readers should agree; this
package does not edit Rust.

Additional integrator requests from the plan remain: JavaScript/TypeScript
CodeQL, future client lint/test/mutation/tools CODEOWNERS, wasm/browser toolchain
pins when their owning packages exist, and the WP-124 known workspace-member
skip. This package keeps every registry dependency in the single root client
manifest until that WP-124 correction exists.

Human dependency confirmation and code-owner approval are required before
merge. No package completion claim is made without CP-002's full coverage,
zero-survivor mutation evidence, full remote gate and review.
