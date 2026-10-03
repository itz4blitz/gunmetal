# Signing-key compromise playbook

Last reviewed: 2026-10-03. For maintainers, when a tag-signing key, a TUF
offline key or a container-signing identity may be leaked or lost
(SEC-SUP-054, SEC-OPS-071). The yearly rehearsal performs a real root
rotation on a test feed that a released server build then accepts.

## Assume the key is burned

Do not wait for confirmation. An unused leaked key is cheaper than a
signed malicious release.

## TUF root rotation

1. Generate a new root key on a hardware token held by a different person
   from the remaining threshold keys (SEC-STD-036, SEC-SUP-049).
2. Sign a new root metadata that removes the burned key and adds the new
   one, using the threshold of remaining old keys plus the new key as the
   TUF spec requires.
3. Publish the rotated root to the test feed first. A released server
   binary must follow the chain (SEC-OPS-071).
4. Only then publish to the production feed. Snapshot and timestamp stay
   online keys; they cannot add a release.

## GitHub tag-signing key

1. Remove the burned key from `.github/allowed_signers`.
2. Add the replacement key, signed in from the hardware token.
3. Move or delete no existing `v*` tag; tags are immutable (SEC-SUP-003).
4. If a tag was created with the burned key after the leak, do not
   retag. Publish the next version from a new commit.

## Container images

1. Cosign-revoke the image digest (or publish a fresh digest and tell
   owners the old one is not trusted).
2. Never push the same exact-version tag twice (SEC-SUP-043).
3. Record the revoked digests in the advisory.

## Tell owners

Publish a GitHub security advisory and a feed record. Point at the
[compromise runbook](compromise.md) for what they run on their server.
