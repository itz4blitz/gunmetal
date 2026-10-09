# 23. Plugin host decisions live in gunmetal-plugins, and no runtime is linked

Date: 2026-10-08
Status: accepted, through the owner's request on 2026-10-08 that the
sandbox work start. It has no entry in the [decision register](../decisions.md)
yet; the owner adds one if he wants it tracked there.

## Context

Work package 225 is the WebAssembly plugin host. SEC-EXT-018 forbids the
server from loading a runtime until SEC-EXT-019 to SEC-EXT-034 are
implemented and passing. The design sketch in the plugins security document
also says a revocation disables a plugin, and that a same-permission update
installs immediately. SEC-EXT-041 and SEC-EXT-040, and the feature rows
INT-164 and INT-163, say the opposite: the index may only advise, and an
update stays inactive until the owner approves it.

## Decisions

1. **The host's decisions are a crate of their own,** `crates/gunmetal-plugins`.
   It parses a closed manifest, admits exact public hosts, caps calls and
   quotas, refuses an archive, and decides install, update, consent, and
   index freshness. It does not open a socket, spawn a process, or verify a
   signature. Those stay with the egress client, the worker sandbox, and the
   secrets crate.
2. **No WebAssembly runtime is linked.** `wasmtime` stays on the deny list.
   The engine settings SEC-EXT-020 names are a value, [`POLICY`](../../crates/gunmetal-plugins/src/engine.rs),
   not a constructed engine. The server does not depend on this crate.
3. **Plugins are unavailable without the transcode worker's sandbox profile.**
   A platform that cannot apply that profile does not load a plugin
   (SEC-EXT-021). This record does not claim that profile has been tested
   for a plugin process.
4. **A revocation advises. It does not disable.** SEC-EXT-041 wins over the
   design sketch. An update that is not yet approved leaves the installed
   version running (SEC-EXT-040). A lower version is refused.

## Consequences

The player still does not download or run an extension package. Lifting
SEC-EXT-018 needs the engine built from `POLICY`, the OS sandbox tests, and
the fuzz targets, in a later change that also removes `wasmtime` from the
deny list for this crate only.
