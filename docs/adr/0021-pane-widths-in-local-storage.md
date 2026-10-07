# 21. Pane widths are kept in localStorage

Date: 2026-10-07
Status: accepted, through the owner's request on 2026-10-07 that the
sidebars be resizable and remember their width in local storage. It has no
entry in the [decision register](../decisions.md) yet; the owner adds one
if he wants it tracked there.

## Context

CLI-060 asks for a wide three-pane layout with resizable panes. A width a
person drags has to survive a reload, or the feature is a nuisance. The web
client had used no browser storage at all until now, and the security
baseline is strict about what may go there: no session or API token
(SEC-IAM-017, SEC-API-032), no Activity, Identity or Secret data unless the
browser was marked a personal device (SEC-PRV-019), and everything deleted
at sign-out (SEC-CLI-009).

## Decisions

1. **The sidebar and queue widths are stored in `localStorage`** under the
   single key `gunmetal.layout.v1`, as `{"sidebar":<px>,"queue":<px>}`.
   Two pixel counts are layout, not Activity, Identity or Secret data, so
   SEC-PRV-019's personal-device condition does not apply to them.
2. **Nothing else goes under that key, and no other key is added by this
   record.** A second preference needs its own look at SEC-PRV-019.
3. **A stored value is untrusted input.** `parsePaneWidths` accepts only a
   JSON object with finite numbers, clamps each to the pane's limits
   (sidebar 200 to 420, queue 280 to 560) and falls back to the default
   (256 and 340) for anything else. The widths reach the page only as two
   CSS custom properties written through the CSSOM.
4. **The UI package never touches storage.** `Shell` takes a `LayoutStore`
   (`read`, `write`) from the composition root, as it takes playback.
   `apps/demo/src/browser/layout-store.ts` is the one adapter that calls
   `localStorage`; blocked or full storage reads as "nothing stored" and
   drops the write.

## Consequences

When sign-out exists (wave C5), SEC-CLI-009 already requires the client to
clear `localStorage`, which removes this key too: a signed-out browser
returns to the default layout. No requirement changes. The end-to-end test
that SEC-TM-058 asks for ("storage holds nothing from the server") stays
true, because the widths come from the person's own drag, not the server;
that test should allow exactly this key and assert its shape.
