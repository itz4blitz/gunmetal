# 22. Client plugins are declarative data in a registry that R1 keeps empty

Date: 2026-10-07
Status: accepted, through the owner's standing decision that everything above
critical core functionality is delegated to plugins. It has no entry in the
[decision register](../decisions.md) yet; the owner adds one if he wants it
tracked there.

## Context

The integrations map decided three things at once. Everything beyond critical
core is a plugin, so the platform grows without the core growing. Nothing
plugin-shaped runs in R1: "R1 runs no plugins at all: no WebAssembly runtime is
linked until the sandbox requirements pass" (INT-054). And no plugin ever
reaches the web client as script: "script injection is an XSS hole. Declarative
home rows and themes (INT-081) meet the same demand" (INT-074). INT-081 said a
declarative home-row and theme extension point "needs its own versioned plugin
interface and permission, added through an architecture record"; this is that
record.

The demo already carried the slot table
(`clients/packages/fake-server/src/plugin-slots.ts`), whose two client-plane
rows are `home-row` and `theme-pack`, both bound to INT-081. But the client had
no contract for what a client-plane plugin even is: no typed manifest, no
grants, no consent shape, and no rule forcing a contribution to be data rather
than code.

## Decisions

1. **A client plugin is a manifest plus a contribution, both plain data.**
   The manifest (`ClientPluginManifest` in `packages/ports/src/plugins`) names
   an id, a title, a version, the literal plane `client`, one slot
   (`home-row` or `theme-pack`), the grants it asks for, and an optional
   homepage. The contribution is tagged data shaped for that slot: a home row
   is `recent`, `loved`, or `custom` naming the albums it shows; a theme pack
   names themes built on Gunmetal's own four. Nothing in the type is a
   function, because a function would be code, and code is what INT-074
   refuses.
2. **Admission is validation, never evaluation.** The registry
   (`packages/ui/src/plugins/registry.ts`) treats a record as untrusted input
   and checks it in a fixed order — plane, id shape, version shape, known
   slot, known grants, the slot's own grant, contribution slot, duplicate id —
   refusing it with a typed reason. It never imports, fetches, or executes
   anything a plugin declared.
3. **Nothing is drawn without consent, and R1 consents to nothing.** A
   registered record is dark until a consent record names its manifest. The
   demo's consent fixture is empty (`demoPluginConsents()` returns `[]`), so
   the data itself proves the default: a plugin that ships in R1 is visible in
   the slot table and enabled nowhere.
4. **The registry stays data-only in R1.** No runtime, no network, no dynamic
   import, no script tag. The R2 sandbox (INT-054) is a server-side matter;
   this record only fixes that the client plane receives typed data through
   INT-081 and draws it, on every platform, the way it draws its own rows.
5. **Slots and grants are versioned and additive.** The slot union and the
   grant union are closed; an R2 record adds to them and versions the
   interface, per INT-081. An unknown grant or slot in a manifest is untrusted
   input to refuse, never a permission to honour.

## Consequences

R1 ships zero loaded plugins and the registry holds data only: the demo's
Extensions pane can show what a server would advertise without anything being
run or fetched. Consent UI does not exist yet; it arrives with R2 and the
permission review sheet (INT-055), and until then the empty ledger keeps every
record dark. Because contributions are data, the eventual home-row and theme
drawers are render-side work over values already typed here, not a security
review. New slots or grants change a closed union, so a manifest written for
one interface version cannot silently claim a permission it never declared.
