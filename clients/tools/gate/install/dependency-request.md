`yaml@2.9.1` is a provisional development dependency request for CP-001,
requiring human confirmation before merge. It parses ordinary pnpm lockfiles,
including both environment and project documents, so source and tracking checks
cannot silently omit either graph. It is not bundled into the application.

The official npm registry publishes the version at
2026-09-11T20:30:10.905Z, older than seven days at the start of this work.
Its licence is ISC, on the project's `deny.toml` allow-list. The maintainer is
Eemeli Aro (`eemeli`), and its repository is https://github.com/eemeli/yaml.
It has no dependencies, optional dependencies or peer dependencies.

The exact tarball is https://registry.npmjs.org/yaml/-/yaml-2.9.1.tgz.
The registry checksum is
`sha512-3NxN8+78OdzbT7C/WjGsyfPAtJaN3FNDsWxv7Y7mcDsT/oOmgW8BpyQQFFBnvZE3j9Y2Sdz1ULFLezL7Eb2yFw==`.
Both manifest and direct-dependency reason are added in this same change.

Alternatives considered: pnpm's JSON dependency listing exposes the project
rather than every lockfile document; a custom JSON-only lockfile interferes with
ordinary pnpm updates; writing a security-sensitive YAML parser here creates
avoidable parser risk. `yaml` provides strict parsing, duplicate-key errors,
explicit document diagnostics and alias traversal. Tests require rejection of
escaped exotic sources in both documents, duplicate keys and aliases.

Primary evidence: https://registry.npmjs.org/yaml and https://eemeli.org/yaml/.
Root integrator reviewed this proposal as provisional; that review is not the
owner's dependency approval. CODEOWNERS and Dependabot integration changes also
need code-owner approval before merge.

Pending integration: after CP-002 creates the complete TypeScript gate, the
integrator wires `pnpm --dir clients run gate` into `scripts/gate.sh` and pins the
verified toolchain in CI. Native Node test-first evidence here does not replace
CP-002's coverage or mutation evidence. No gate wiring is added early.
