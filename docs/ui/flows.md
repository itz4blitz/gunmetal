# User flows

Written on 2026-10-02. Status: draft for the project owner's review.

This file walks through the journeys that matter most in Gunmetal, one step
at a time, so that the screens the UI needs and the server work behind each
screen can be read off in the order a person meets them. It is built on the
[feature map](../features/README.md) and refers to features by ID. The
feature map is the source of truth for what a feature does and when it
ships: where this file and the map disagree, the map wins and this file is
a bug. Both stay inside the accepted architecture records,
[ADR 1](../adr/0001-architecture.md) and
[ADR 2](../adr/0002-music-is-first-class.md).

The [security baseline](../security/README.md) outranks both. The owner has
directed that every plan be built around it, so where a flow and the
baseline disagree, the flow follows the baseline and cites the requirement
by its ID (SEC-<AREA>-<NNN>). Each such change is listed in
[Changes made to follow the security baseline](#changes-made-to-follow-the-security-baseline),
and the ones that rest on an open owner decision in the baseline are marked
"owner to confirm" there.

Nothing here is built yet. Speed figures are the map's proposed goals (open
decision 16), not measurements. Rival behaviour comes from the research in
`docs/research/` and the map's "Rivals today" cells, and carries
"(unverified)" wherever the research could not confirm it.

## How to read a flow

Every flow has the same parts.

- **Release.** The release in which the main path first works end to end on
  at least one client. When an earlier release offers a reduced version, or
  a later one changes the path, the release line says so and the flow has a
  section for it.
- **Who and where.** The person, the device and the client.
- **Starts and ends.** The state before the first step and after the last.
- **Steps.** A table whose columns, after the step number, are:
  - *The person* says what they do and what they see.
  - *Surface* names the screen or control, using the names in the map's UI
    surfaces column so the UI plan can collect them.
  - *The server* says what the server must do at that step. Some work runs
    in the shared Rust core on the device (WASM in browsers, UniFFI in
    native apps). The cell then says "on the device", because "no server
    call" is a requirement too.
  - *Features* lists the owning rows. A reference row appears only when it
    adds detail for that step.
  - A security requirement ID in any cell is a test the step must pass; the
    test carries that ID (SEC-STD-004).
- **When it goes wrong.** The branches a designer must draw and a tester
  must cover, including the security failure paths.
- **Rivals today.** Where Plex, Jellyfin and others stand, including where
  they are already good.
- **Other releases.** What arrives earlier or later, by ID.
- **Depends on.** Architecture records and the numbered open decisions in
  the [feature map README](../features/README.md#open-decisions-for-the-project-owner).

Three facts shape every R1 flow.

1. **R1 clients are browsers only.** They are the web client and the
   installable web app (CLI-001, CLI-003), with a phone-width layout
   (CLI-149). Native Android, Android TV and Fire OS apps arrive in R2. The
   desktop shell is Later, because the baseline's release scope puts it
   there (SEC-TM-074, SEC-CLI-069; owner to confirm), so desktops use the
   web client in every release until then.
2. **Browsers reach the server only over HTTPS or on localhost.** Passkeys,
   installation, offline loading and dependable local storage need a secure
   context (CLI-150), and the baseline goes further: over plain HTTP, every
   peer except loopback gets a static redirect or help page that sets no
   cookie, and no sign-in, API or credential of any kind (SEC-NET-001). R1
   browsers therefore use the per-server HTTPS name, the owner's own domain,
   a tailnet name or localhost (SEC-NET-013). There is no plain-HTTP player
   and no password.
3. **A new server is in the home posture.** Requests from addresses outside
   loopback and the private ranges get a static help page, and the owner is
   alerted (SEC-NET-024, SEC-NET-027, SEC-OPS-038). Remote use in R1 means
   the owner declares a reverse proxy or a tailnet (security README, owner
   decision 3). Requests through a public proxy get the internet posture,
   which refuses setup, and administration unless the owner has turned
   remote administration on (SEC-NET-019, SEC-NET-045).

The flows lean on three terms the map defines. **The user log** holds
everything a person authors that a rescan cannot recreate: history, loves,
ratings, playlists, queue state, layouts and household curation. **The
identity store** holds accounts, credential public keys, device keys,
grants and invitations. **The synced library** is each profile's copy of
library metadata on each device. Both durable stores wait on ADR 3 (open
decision 1), so nearly every flow below depends on it. Accounts also wait
on the identity architecture record that the baseline requires before any
server code stores a user (SEC-STD-006).

## Security rules every flow follows

These come from the baseline's first principles. The flows do not repeat
them at every step; they cite them where a step depends on one.

- **Deny by default.** Every route declares who may use it, and every
  object fetch, list, search and sync passes one visibility predicate
  (SEC-IAM-067, SEC-IAM-070). Being on the home network never grants
  anything (SEC-IAM-013, SEC-HIS-004).
- **No passwords.** People sign in with a passkey (with user verification),
  through the household's own identity provider, or by approval from a
  device where they are already signed in. There are no passwords,
  authenticator-app codes, security questions or emailed codes
  (SEC-IAM-025, SEC-IAM-108; owner decision 1).
- **Two sessions and step-up.** A browser session lasts at most 30 days, or
  7 without use, and never authorises administration. Admin screens need a
  separate admin session, created by a passkey check, which ends after 15
  idle minutes or 1 hour. Host-equivalent actions also need a passkey check
  in the previous 5 minutes, called step-up in these flows: trusted
  proxies, posture and remote administration, TLS and naming, egress
  policy, plugins, adapters, backup download and restore, ownership
  transfer, key rotation, creating or promoting administrators, adding or
  removing library roots and browsing the file system (SEC-IAM-041,
  SEC-TM-017). Most of these are the owner's alone (SEC-IAM-075), and an
  identity-provider sign-in alone never satisfies step-up (SEC-IAM-107).
- **Revocation bites on the next request.** Ending a session, removing a
  device, disabling an account or narrowing a grant makes the next request
  fail, stream range requests included, and closes open streams and sockets
  within 5 seconds (SEC-IAM-043, SEC-TM-028, SEC-API-028).
- **Admins see who is playing, not what.** Live sessions show the person,
  device, quality and playback method, and the title only if that person
  has chosen to show it; no admin view shows anyone's history (SEC-PRV-025,
  SEC-TM-054; owner decision 5). Everyone can read what admins can see
  about them, and can start a private session (SEC-IAM-104, SEC-PRV-024).
- **Everyone sees their own security events, and the owner is alerted.**
  Each person reads the security log entries about their own account, and
  the owner reads the whole log with other people's addresses shortened
  (SEC-IAM-097, SEC-OPS-027). The alerts these flows raise are listed in
  [Owner alerts the flows raise](#owner-alerts-the-flows-raise).
- **Secrets travel in fragments and bodies.** Setup codes, invitations,
  pairing codes and share secrets ride in the URL fragment or a request
  body, never a query string, and never reach a log (SEC-NET-036,
  SEC-CLI-013, SEC-IAM-047).

## Flow index

Flows F01 to F12 are the journeys the project asked for, in the order
asked. F13 to F20 cover journeys that the first twelve kept running into:
upgrades, restores, a lost phone, a person's own record, video on a TV,
children, third-party apps and live TV.

| # | Flow | Release | Who | First client |
|---|---|---|---|---|
| [F01](#f01-first-run-server-setup) | First-run server setup | R1 | Owner | Host console, then the web client |
| [F02](#f02-adding-a-music-library-and-watching-it-scan) | Adding a music library and watching it scan | R1 | Owner or admin | Web client (admin) |
| [F03](#f03-signing-in-on-a-new-device) | Signing in on a new device | R1 for browsers; native apps in R2 | Any member | Web client |
| [F04](#f04-pairing-a-tv-by-scanning-a-code-with-a-phone) | Pairing a TV by scanning a code with a phone | R2 | Member with a signed-in phone | Android TV, Google TV or Fire OS app |
| [F05](#f05-playing-an-album-and-editing-the-queue) | Playing an album and editing the queue | R1 | Listener | Web client |
| [F06](#f06-building-a-playlist) | Building a playlist | R1 | Listener | Web client |
| [F07](#f07-downloading-for-offline-and-playing-on-a-plane) | Downloading for offline and playing on a plane | R2 | Listener or viewer | Android app |
| [F08](#f08-moving-playback-from-the-phone-to-another-device) | Moving playback from the phone to another device | R2; R1 offers "continue on this device" | Listener | Android app, TV, web |
| [F09](#f09-searching-across-music-and-video) | Searching across music and video | R1 for music; video in R2 | Anyone | Web client |
| [F10](#f10-sharing-a-library-with-a-friend) | Sharing a library with a friend | R1 through the owner's own address; no open ports in R2 | Owner and friend | Web client |
| [F11](#f11-migrating-from-plex-or-jellyfin) | Migrating from Plex or Jellyfin | R1 for export files and playlists; rival databases Later | Owner | Web client (admin) |
| [F12](#f12-recovering-from-a-failed-playback) | Recovering from a failed playback | R1 for music; video in R2 | Listener, then owner | Web client |
| [F13](#f13-upgrading-the-server-and-rolling-back) | Upgrading the server and rolling back | R1 | Owner | Host, startup page, admin |
| [F14](#f14-rebuilding-a-dead-server-from-a-backup) | Rebuilding a dead server from a backup | R1 | Owner | Host, welcome screen |
| [F15](#f15-losing-a-phone) | Losing a phone | R1 for sessions; downloads in R2 | Member, owner | Web client |
| [F16](#f16-keeping-your-own-record) | Keeping your own record | R1 | Listener | Web client |
| [F17](#f17-watching-a-film-on-the-tv) | Watching a film on the TV | R2 | Viewer | Android TV app |
| [F18](#f18-setting-up-a-childs-profile) | Setting up a child's profile | R2 | Parent | Web client, then TV |
| [F19](#f19-connecting-an-existing-music-app) | Connecting an existing music app | R2 | Listener, owner | A third-party Subsonic app |
| [F20](#f20-adding-an-iptv-playlist-and-watching-a-channel) | Adding an IPTV playlist and watching a channel | R3 | Owner, viewer | Web client (admin), TV app |

Journeys the project has placed in **Later** or **No** are listed in
[Journeys not designed here](#journeys-not-designed-here), with the reason.

---

## F01. First-run server setup

**Release: R1.**

**Who and where.** The owner, first at the host (a terminal, or `docker
logs` for a container), then in a browser.

**Starts** with a binary or container that has never run. **Ends** with an
owner account holding a passkey, the recovery kit saved, setup closed for
good, encrypted daily backups on and the first music library scanning.

| # | The person | Surface | The server | Features |
|---|---|---|---|---|
| 1 | Installs the single binary, the container image or the OS service, with media mounted read-only. Chooses how browsers will reach the server over HTTPS: the project's per-server name (the recommended default, owner to confirm), the owner's own domain, a tailnet name, or this machine only. | Install docs; download page; install prompt | Nothing yet. There is one self-contained binary per target, and container tags pin a version. The official unit and image run as a dedicated non-root account with no capabilities, no host networking and a read-only root filesystem (SEC-OPS-053, SEC-OPS-056, SEC-OPS-057), and every package starts in the home posture (SEC-OPS-038). | ADM-001, ADM-002, ADM-003, ADM-004, ADM-005, ADM-089, ADM-023 |
| 2 | Starts the server. | Console; Startup page | Checks the config against a typed schema and reports mistakes with their line on the console and in the log. Refuses to run as root or with any capability, with no override (SEC-OPS-053), and refuses a data directory on a network filesystem. Generates its keys and root secret from the operating system's random source into files only the service account can read (SEC-OPS-011, SEC-OPS-012). Makes no outbound connection, except that an install that chose the name service registers its random label and obtains its certificate (SEC-OPS-007, SEC-NET-010). Keeps settings, log, cache and scratch space apart. Opens the HTTP listener before the database, so the startup page can say what is happening; that page shows no version, path or error detail (SEC-OPS-050). | ADM-007, ADM-006, ADM-079, ADM-090, ADM-032 |
| 3 | Reads the claim link, its QR code and the one-time setup code on the console, or runs `gunmetal claim-code`. | Console and log message | Generates a single-use code of 128 bits that expires 24 hours after it is made and survives a restart. Shows it only on the host: the console and journal, a terminal QR code, a claim URL that carries the code in its fragment (`https://<name>/claim#<code>`), and a file only the service user can read (SEC-IAM-007). Until the claim it answers only the claim page, its assets and a health check, and accepts no sign-in (SEC-IAM-006, SEC-OPS-001, SEC-OPS-003). | ACC-001, ADM-018 |
| 4 | Scans the QR code with a phone, or opens the claim link on a computer, and picks a language, region and time zone. | Welcome > Language | Serves the claim page only in a secure context: the server's HTTPS name, or localhost on the host itself or through an SSH tunnel (SEC-IAM-008). Refuses a Host it does not recognise, which defeats DNS rebinding (SEC-IAM-010). The page moves the code from the fragment into the request body and removes it from the address bar (SEC-CLI-013). The language choice is held by the page and saved with the claim. | ADM-027 |
| 5 | Types the setup code, if it did not arrive in the link. | Welcome > Setup code | Checks the checksum on the page, so a typo costs no attempt, then compares the code in constant time. Wrong codes are delayed per source on one schedule, never from loopback, and never use up or change the code; each failure is printed on the host console with its address and time (SEC-IAM-008, SEC-API-056, SEC-OPS-004). | ACC-001, ACC-063 |
| 6 | Chooses between setting up a new server and restoring from a backup. Restore continues in F14. | Welcome | Both paths sit behind the setup code (SEC-OPS-008). | ADM-029 |
| 7 | Sees which address the passkey will belong to, and that the choice is permanent for passkeys. Opened on localhost while an HTTPS name exists, the page first moves to that name. With localhost as the only address, it says the passkey will work only on this machine and that one for the real address can be added later. If a reverse proxy is in front of the server without having been declared, the page asks "A reverse proxy at <address> is in front of Gunmetal. Trust it?" | Welcome > "Where will people reach this server?" panel; Welcome > HTTPS; Welcome > Proxy | Fixes the passkey relying-party ID to the configured origin (SEC-IAM-018) and explains its permanence (SEC-NET-072). For the owner's own domain, obtains the certificate by ACME DNS-01 and renews it automatically (SEC-NET-004, SEC-NET-013). A proxy declared here is authorised by the setup code and recorded as a private overlay or as public (SEC-NET-017, SEC-NET-019). Behind a container gateway or NAT, asks the owner to declare the topology (SEC-NET-068). | ADM-021, CLI-150, ADM-022, ACC-098, ACC-099 |
| 8 | Creates the owner account: a name, then a passkey made with Face ID, a fingerprint, Windows Hello or a security key. Optionally links the household's own identity provider. | Welcome > Create owner | Consumes the code, creates the owner and stores the passkey in one transaction, so two claims at once cannot both win (SEC-IAM-009, SEC-OPS-005, SEC-STD-029). Requires a discoverable credential with user verification (SEC-IAM-020). Offers no password and no authenticator-app code (SEC-IAM-025). An identity-provider link never makes anyone the owner, and the owner always keeps a passkey (SEC-IAM-031, SEC-IAM-107). Starts an elevated owner session in a cookie page scripts cannot read (SEC-API-032, SEC-IAM-041). Prints "Claimed by '<passkey name>' from <address> at <time>" on the console and writes the claim to the audit log. | ACC-002, ADM-019, ACC-050, ACC-057, ACC-124 |
| 9 | Saves the recovery kit, one printable page with the recovery codes and the backup recovery key, and confirms by typing its last four characters. Is offered a second passkey, such as a security key. | Welcome > Recovery kit | Issues 10 single-use recovery codes, stored only as peppered hashes (SEC-IAM-089), and the backup recovery key, generated in the browser so that only its public half reaches the server (SEC-PRV-040). If the owner moves on without confirming, the dashboard keeps a reminder until they do. | None yet ([G18](#gaps-and-questions-the-flows-expose)) |
| 10 | Names the server and, if wanted, writes a short message. | Welcome (server name) | Stores two plain-text settings. Neither is shown to anyone who has not signed in: the sign-in page stays generic, each device shows the name after its first sign-in, and an invitation page may show it because the invitation authorises that (SEC-NET-047, SEC-API-005; owner to confirm). | ADM-140 |
| 11 | Reviews privacy. Every feature that could reach the internet is listed, explained and off. Answers the required question about security fixes, choosing "Tell me about security fixes (recommended): fetches a public file, sends nothing about you" or "Not now"; neither is preselected. | Welcome > Privacy | Starts the egress gate with no grants and registers each outbound feature, so the network activity page can account for every connection (SEC-NET-032, SEC-PRV-007). Enables no metadata provider (SEC-PRV-013). Does not continue until the update question has an answer (SEC-OPS-047). | ADM-028, ACC-113, LIB-108, ADM-053, ADM-129 |
| 12 | Answers "Coming from another server?" by skipping, or by handing over Last.fm or ListenBrainz export files and M3U playlists (F11). | Welcome > Import; upload dialog | Should hold the import jobs until the first scan has built enough of the library to match against. Takes files only through the upload route, with type and size limits (SEC-API-085, SEC-API-088). Imported listens go into the owner's own history only. | ADM-030, ADM-042, ADM-043 |
| 13 | Adds music folders, as F02 describes, touching the passkey again if more than 5 minutes have passed. | Welcome > Libraries; step-up prompt | Browsing folders and adding a library root are step-up actions (SEC-IAM-041). Lists directories only, below allowed base paths (SEC-API-022), and refuses a filesystem root or a folder that holds the server's own data (SEC-MED-037). Checks access and storage type and registers the roots in durable settings. | ADM-025, LIB-005, LIB-001 |
| 14 | Finishes and lands in the music library, which fills in while the scan runs. | Welcome > Done; Home; Admin > Dashboard | Removes the setup routes for good, including after a restart, a restore or a damaged identity store (SEC-IAM-009, SEC-OPS-006). Starts the scan in the sandboxed scan worker (SEC-MED-018), switches on daily backups encrypted to the server's backup key and the owner's recovery key (SEC-OPS-041, SEC-OPS-042), and shows the health summary, including the isolation tier and whether the server can be reached from the internet (SEC-MED-024, SEC-OPS-061). | ADM-020, ADM-031, LIB-021, ADM-065, ADM-109 |

**When it goes wrong.**

- **The code has expired.** `gunmetal claim-code` mints a new one. A
  restart does not change a code that is still valid, so a reboot never
  strands a link already scanned (SEC-IAM-007). This settles
  [G1](#gaps-and-questions-the-flows-expose).
- **The code was mistyped.** The checksum catches most slips on the page.
  A wrong code is delayed per source and printed on the console, and it
  never uses up the real code (SEC-IAM-008, SEC-OPS-004).
- **The browser closes after the code is accepted but before the passkey
  exists.** Nothing has been committed, because the code, the owner and the
  passkey are one transaction; the owner starts again with the same code
  (SEC-IAM-009). This also settles G1.
- **The page is opened over plain HTTP from another computer**, such as
  `http://192.168.1.7`. The server answers with a static page that
  redirects to the HTTPS address when one exists, and otherwise explains the
  ways in: open the page on the server itself, use an SSH tunnel to
  localhost, or set up the per-server name, a domain or a tailnet name
  (SEC-NET-001, SEC-NET-071). Setup never falls back to a password over
  plain HTTP. This replaces the old password fallback and settles
  [G2](#gaps-and-questions-the-flows-expose).
- **The home router blocks the per-server name** through its DNS
  rebinding protection. If the HTTPS page has not loaded within a few
  seconds, the redirect page says how to allow the zone, or to use
  localhost or a tailnet name instead (SEC-NET-001, SEC-NET-071).
- **The name service refuses or cannot be reached.** The server keeps
  working on localhost, an own domain or a tailnet, and the console and
  help page say why (SEC-NET-071).
- **Someone else on the network reaches the claim page first.** Without the
  code they get nothing. Their guesses are delayed, shown on the host
  console, and cannot wear out the code (SEC-IAM-007, SEC-IAM-008). A web
  page that tries DNS rebinding is refused by the Host check (SEC-IAM-010).
- **The server is reachable from the internet before it is claimed.**
  Requests from public addresses get the static help page, never the claim
  page, and the owner sees an exposure alert after the claim (SEC-NET-024,
  SEC-NET-027).
- **The config has a mistake, the service runs as root, or the data
  directory is on NFS.** The server refuses to start and says why; there is
  no override for root (SEC-OPS-053). `gunmetal doctor` lists fixes
  (ADM-007, ADM-006, ADM-079, ADM-123).
- **The owner later loses every passkey and device.** The host recovery
  command (`gunmetal admin recover`, ADM-034) talks to the server over a
  local socket only the service account can open, prints an enrolment link
  valid for 15 minutes, ends every owner session and alerts every
  administrator (SEC-IAM-092, SEC-OPS-009). Nothing on the network can do
  this (ACC-004).
- **The identity store loses the owner's credential.** A claimed server with
  no usable owner credential starts locked. Only host recovery leaves that
  state, and setup never reopens (SEC-OPS-006).

**Rivals today.** Jellyfin's wizard is short, local and needs no outside
account, so Gunmetal is only at parity there. Plex's claim token for
headless installs is a clean flow, but it binds the server to a plex.tv
account. Immich's restore button on its welcome screen, which checks that
library folders are readable, is the model for step 6. Jellyfin 12.0 had to
close a hole that let its wizard be re-run, and Navidrome 0.64.2 fixed a
failed setup that left a server with no admin. Those are the reasons
ACC-001 and ADM-020 are rules rather than polish.

**Other releases.**

- R2: film and TV folders in the same step (ADM-026, LIB-002); setting up a
  headless server from the Android app with a device key, which sidesteps
  the secure-context problem (ADM-035). The app claims through a
  password-authenticated key exchange keyed by the setup code and pins the
  server's key, so watching the network reveals nothing (SEC-OPS-010). A TV
  never claims a server: it is a limited device and can never hold owner or
  administrator capabilities (SEC-CLI-024, SEC-IAM-059). Also declarative
  setup from a file (ADM-024) and a next-steps checklist that ticks itself
  (ADM-033).
- R1, not Later: automatic certificates for the owner's domain (ACC-099),
  because the baseline needs own-domain ACME with automatic renewal
  (SEC-NET-004, SEC-NET-013); and the per-server HTTPS name (ADM-023, open
  decision 7) as the install-time default, launched only once its zone is on
  the Public Suffix List and with Certificate Transparency monitoring
  (SEC-NET-010, SEC-NET-069, SEC-NET-070; security README decision 2, owner
  to confirm). If the list entry is not in place, R1 ships with the own
  domain, tailnet and localhost paths only.

**Depends on.** ADR 3 for the identity store (open decision 1); the
identity architecture record (SEC-STD-006); open decision 7 (HTTPS names)
and security README decisions 1, 2, 4 and 14; open decision 15 (no
telemetry).

---

## F02. Adding a music library and watching it scan

**Release: R1.**

**Who and where.** The owner or another admin (ACC-040) in the web client,
from Welcome > Libraries during F01 or from Admin > Libraries at any time,
in an admin session (SEC-IAM-041).

**Starts** with folders of audio files the server can read. **Ends** with a
browsable, playable library on every signed-in device that may see it, a
health report, and background analysis running.

| # | The person | Surface | The server | Features |
|---|---|---|---|---|
| 1 | Chooses "Add library" and the Music kind. | Admin > Libraries; Welcome > Libraries | Admin > Libraries needs an admin session, created by a passkey check and ended after 15 idle minutes or 1 hour; a media session never opens an admin route (SEC-IAM-041). Creates a library record with its kind, roots and settings. R1 offers only music, but the record has room for other kinds, and a folder can be flagged as spoken word so audiobooks stay out of music. | LIB-001, LIB-004, LIB-011, LAT-010 |
| 2 | Touches the passkey if the last check is more than 5 minutes old, then picks one or more folders in the server's folder browser and sees, for each, whether it is readable, what kind of storage it is, and that nothing will be written to it. | Step-up prompt; folder picker with live checks | Browsing the file system and adding a root are step-up actions, audited and announced to every admin (SEC-IAM-041, SEC-TM-017). Lists directories only, never file contents, below the configured browse roots, and only to admins (SEC-API-022). Refuses a filesystem root, a system directory, or a folder that holds the server's own data, cache, configuration or logs, and says why (SEC-MED-037). Checks read permission and storage type (local disk, network share, FUSE mount) and warns before an empty or unreadable folder. Opens roots read-only, and `doctor` warns when the service account could write to one (SEC-MED-038, SEC-OPS-054). Stores each root once, so every item is a root plus a relative path. | ADM-025, LIB-003, LIB-007, ADM-089, LIB-015 |
| 3 | Optionally sets exclusions, how changes are detected and a safety-net schedule. | Library settings > Exclusions; "Watch for changes"; Library settings > Folder > Storage type; Library settings > Schedule | Stores a gitignore-style pattern list per root, a file watcher for local disks, a poll interval and read parallelism for shares and cloud drives, and a rescan schedule. | LIB-006, LIB-014, LIB-015, LIB-013 |
| 4 | Chooses who may see the library. | Admin > Users > Libraries, or a step in the add-library sheet | Writes library grants into each person's policy. Until someone is granted it, a new library is visible only to the owner and administrators (deny by default; SEC-IAM-070). The grants filter the synced library and every fetch, apply from each person's next request (SEC-IAM-076), and an increase in anyone's access alerts the owner (SEC-OPS-032). | ACC-037, MUS-027 |
| 5 | Saves, and the scan starts. | Admin > Dashboard scan card; Admin > Activity | Parses in separate scan-worker processes, never in the server, each single-threaded, reading only file descriptors the server opened, under memory, time and system-call limits (SEC-MED-018, SEC-MED-020 to SEC-MED-022). Runs the scan on a bounded worker pool with a memory ceiling and no helper process per file. Reads headers and indexes only, within a per-file read budget. Visits recently modified folders first, commits in batches and publishes progress events. | LIB-012, LIB-020, LIB-019, LIB-021, LIB-022, ADM-093 |
| 6 | Watches the files found, the bytes read and an estimate of the time left. | Dashboard scan card; activity indicator in the admin header | Counts I/O per root, so the owner can see that the scan read headers, not whole files. | ADM-088, ADM-031 |
| 7 | Opens Home or the album grid while the scan runs, and albums appear batch by batch. | Home (scanning empty state); library views; progress banner | Sends each batch through the library change feed; on the device, the client applies the deltas to its synced copy. | LIB-018, CLI-022, MUS-208, DIS-004, MUS-043 |
| 8 | Plays an album that has already arrived, as F05 describes. | Album page; player | Serves bytes for any committed item under the usual capability URLs (SEC-API-026). | MUS-066, ACC-122 |
| 9 | Sees the library take shape from tags, not folders: credits, release groups, discs and artwork. | Artist page; album page | Builds albums, artists and credits from multi-value tags in every format, splits artist strings with an exception list, uses MusicBrainz IDs as identity, and records the reason for every grouping. Takes embedded and folder artwork, decoded only in the worker by memory-safe decoders with pixel limits, and sends clients only re-encoded JPEG, PNG or WebP sized for each device class (SEC-TM-034, SEC-MED-044 to SEC-MED-046, SEC-CLI-005). Turns `.m3u` files in the folders into playlists whose entries resolve only to items already indexed in the same library; URLs and paths outside it are dropped and reported (SEC-MED-050). | LIB-045, MUS-034, MUS-035, MUS-036, LIB-098, LIB-134, LIB-135, LIB-142, LIB-143, LIB-192 |
| 10 | Reads the summary and the health report when the scan ends. | Admin > Activity; Library health > Problems; Library health > Tag problems; Library health > Sidecar problems; Admin > Review queue | Records parse errors with their location, tag problems with their reasons, and files the supported browsers cannot decode. Holds doubtful groupings, such as two same-titled albums, for a yes. | LIB-193, LIB-194, MUS-044, MUS-229, LIB-099, LIB-051, LIB-068 |
| 11 | Sees background analysis continue at low priority. | Admin > Tasks; Admin > Activity | Measures loudness for untagged tracks in the scan worker, with memory-safe decoders only (SEC-MED-018, SEC-MED-025), as a throttled, checkpointed job that survives restarts and yields to playback. Rebuilds the neighbour table that radio and "more like this" use. | MUS-086, LIB-024, ADM-095, DIS-060 |
| 12 | Later copies new albums into the folder, and they appear on their own. | Recently added row on Home | Picks up the change by watcher or poll and rereads only what changed. Treats a better copy of an existing file as an upgrade, not a new arrival. | LIB-014, LIB-015, LIB-017, LIB-030, DIS-035, DIS-036, DIS-038, MUS-059 |

**When it goes wrong.**

- A folder is unreadable or empty. The picker warns before saving
  (ADM-025).
- A drive or NAS goes offline during the scan or later. Its items grey out
  instead of disappearing, the root shows as offline, an alert goes out, and
  the trash never purges while a root is offline (LIB-032, ADM-108, ADM-116,
  LIB-033).
- Files are moved or renamed. Identity follows them, keeping plays, ratings
  and playlist positions (LIB-028, LIB-029). A file that vanished is listed
  with its last known path (LIB-034).
- A tag would split "AC/DC" into two artists. The exception list prevents
  it, and the split report shows any doubtful case (MUS-035, LIB-038).
- An admin wonders why a track landed on an album. "Why is this here?" and
  the file inspector show raw tags beside the interpreted values (LIB-098,
  ADM-125, LIB-195).
- A hostile or damaged file crashes or hangs the parser. Only the worker
  dies; the server keeps serving and finishes the scan. A file that fails
  twice is quarantined until it changes or an admin retries it, and appears
  in Library health with the reason (SEC-MED-018, SEC-MED-019).
- The host cannot apply the full sandbox (an old kernel, a restrictive
  container). Memory-safe parsing still runs in its own process, with a
  "reduced isolation" notice on the health page; native decoders stay off
  rather than run unconfined (SEC-MED-024).
- The admin session or the step-up has lapsed. The picker asks for the
  passkey again before it lists any folder (SEC-IAM-041).
- The chosen folder holds the server's own data, or is a filesystem root.
  The picker refuses it and says why (SEC-MED-037).
- Decoders for loudness measurement are not approved before R1 (open
  decision 8). Whenever they are, they run only in the scan worker and only
  as memory-safe code (SEC-MED-018, SEC-MED-025). Until then R1 uses
  loudness tags plus a fallback gain, and step 11 measures nothing
  (MUS-089).
- A new library is visible only to the owner and administrators until
  someone is granted it, which the baseline's deny-by-default rule settles
  ([G4](#gaps-and-questions-the-flows-expose)).

**Rivals today.** Navidrome already builds albums from tags, lets people
browse during the first scan and publishes expected scan times by library
size; it is good here. Plex rescans part of a library when files change,
and Jellyfin and Emby monitor local disks in real time, though Linux watch
limits often need raising. Gunmetal's edge is in what the scan avoids: no helper process per
file (Jellyfin 10.9's ffprobe processes, about 700 MB each, exhausted a
4 GB Raspberry Pi 4, #11588), no whole-file reads on cloud drives, change
detection on shares that send no events, and a recorded reason for every
grouping. Scan speed is a design goal until the benchmark against Jellyfin
exists (ADR 1).

**Other releases.**

- R2: film and TV libraries (LIB-002); cancelling and reprioritising jobs
  (LIB-023); opt-in, sandboxed lookups at MusicBrainz and the Cover Art
  Archive (LIB-107, LIB-111, LIB-112), which the baseline recommends
  building in for R1 behind the egress client and the setup question, each
  listing the fields it sends (security README decision 22, owner to
  confirm); editing in the app, with locks
  (LIB-172, LIB-173); a duplicates report (LIB-196); CUE sheets (LIB-071,
  MUS-041); more formats (MUS-033); ratings read from tags (MUS-045);
  remapping a missing file by hand (LIB-035); an I/O profile per root
  (ADM-087).
- Later: acoustic fingerprinting in the review queue (LIB-106); missing
  albums on artist pages (LIB-201).

**Depends on.** Open decision 8 (decoders for loudness); ADR 3 for the
curation log that keeps review-queue answers (open decision 1).

---

## F03. Signing in on a new device

**Release: R1 for browsers; native apps in R2.**

**Who and where.** Any member with an account, in a browser on a new laptop
or phone, reaching the server over HTTPS or on localhost.

**Starts** with an account made in F01 or F10. **Ends** with the device in
the person's list of sessions and devices, holding a synced copy of what
they may see, and showing their home and queue.

| # | The person | Surface | The server | Features |
|---|---|---|---|---|
| 1 | Opens the server's address. | Sign-in page | Serves the sign-in page only over HTTPS on a configured name, or on localhost (SEC-NET-001). The page is generic: no list of users, no server name or message, no version (SEC-IAM-022, SEC-NET-047, SEC-API-005). Shows a notice if the browser lacks a feature the client needs (SEC-API-052). | ACC-007, CLI-002, CLI-150 |
| 2 | Signs in with a passkey, or through the household's identity provider. | Sign-in; Sign-in > Continue with provider | Runs a usernameless WebAuthn ceremony with user verification required, checking type, challenge, origin, flags and signature (SEC-IAM-018 to SEC-IAM-022). Or completes OIDC with the server as a confidential client, using PKCE, state and nonce, so that no provider token reaches the browser (SEC-IAM-026, SEC-CLI-026). Failures look the same whatever the cause and are throttled per source; a passkey is never disabled by failures (SEC-API-056, SEC-API-058). Makes no outbound call except to the household's own identity provider. | ACC-050, ACC-057, ACC-007, ACC-063, ACC-003 |
| 2a | If this browser cannot use a passkey (an old smart-TV browser, a borrowed laptop), chooses "Use another device". The browser shows an 8-character code and a QR code. On a phone where they are already signed in, the person scans the QR code or types the code, reads what is asking, and approves with Face ID or a fingerprint. | Sign-in > Use another device; approval sheet on the signed-in device | Opens a pairing request that holds a non-extractable key the browser generated (SEC-IAM-108). The code lasts 10 minutes, works once and dies after 5 wrong guesses (SEC-IAM-056). The approval sheet shows the browser's self-reported name marked as unverified, its type, "In this home" or "Somewhere else", how long ago it asked and exactly what it will get, and needs a passkey check in the previous 5 minutes (SEC-IAM-058). When the two devices are not on the same local network, the approver must type the code shown on the browser and confirm a matching code shown on both screens; a link or a QR picture is not enough (SEC-IAM-060). Approval always starts on the approving device, so nobody can push a prompt to it (SEC-STD-027). | ACC-062 |
| 3 | Answers one plain question: is this your own device, or a shared one? | Sign-in > "Is this your own device?" | Issues a new session token in a `__Host-` cookie page scripts cannot read (SEC-IAM-038, SEC-API-032). Own device: the session lasts up to 30 days, ends after 7 days unused, and the library may be kept on the device. Shared: library data stays in memory, the cookie dies with the browser, and the session ends after 30 idle minutes (SEC-IAM-041, SEC-CLI-010). A browser signed in by approval is a limited device: it can browse and play, but never administer, approve other devices or change account security (SEC-IAM-108, SEC-CLI-024). Records the device, writes an audit entry, and tells the person's other devices that a new device signed in (SEC-IAM-098). | ACC-079, ACC-124, ACC-068, ADM-110, ACC-071 |
| 4 | On a shared browser, sees a one-line note that nothing is kept after the tab closes. | Banner on first sign-in; Settings > About this connection | Nothing beyond step 3. | CLI-150 |
| 5 | Waits while the device syncs, then sees Home. | Home; sync status indicator; Settings > Storage | Sends a snapshot and then deltas of the synced library, computed per person, so the payload holds nothing outside their grants and restrictions and no one else's history, queue or searches (SEC-API-015, SEC-CLI-020). On an own device the copy is kept per account and deleted at sign-out (SEC-IAM-017, SEC-CLI-009). Sends the profile's preferences and home layout from the user log. | CLI-022, ACC-037, ACC-030, ACC-012, CLI-030, DIS-002, DIS-007, CLI-024 |
| 6 | Is offered the queue they left on another device. | Player bar prompt | Nothing more: the queue document arrived with the sync. | CLI-103, MUS-122 |
| 7 | Optionally installs the web app on the home screen. | Install prompt | Serves the manifest, icons and service worker. The service worker never stores stream URLs (SEC-API-029). | CLI-003 |
| 8 | Later finds this device in their list, with its type, network, rough location and last use, and can end it or every other session. | Account > Sessions and devices | Updates last use in the device registry (SEC-IAM-042). | ACC-068, ACC-069, ACC-070 |

**When it goes wrong.**

- **The passkey prompt is cancelled or the passkey is refused.** One
  uniform message (SEC-API-058). Repeated failures are throttled per
  source, never lock the account, are written as fail2ban lines, and above
  a threshold alert the account holder and the owner (SEC-API-056,
  SEC-IAM-099, SEC-OPS-032).
- **This browser cannot use passkeys and the person has no other signed-in
  device.** They can use a security key, a phone's cross-device passkey, the
  household's identity provider, or the recovery steps below. There is no
  password to fall back to (SEC-IAM-025; security README decision 1, owner
  to confirm).
- **The pairing code expires, or is guessed wrong five times.** The browser
  shows a fresh code; the dead one grants nothing (SEC-IAM-056).
- **A stranger sends the person a QR picture to approve.** The server sees
  the approving phone and the asking browser on different networks, so the
  sheet says "Somewhere else" and demands the typed code and the matching
  code (SEC-IAM-060).
- **A member is locked out.** They use one of their recovery codes, or an
  administrator issues a single-use recovery link, redeemed in person from a
  QR code on the administrator's screen or on a device the member already
  approved. The new passkey starts a 72-hour recovery hold: it cannot remove
  other credentials or export history, the member's other devices can end
  it with one tap, and after an administrator's link the member's history
  stays hidden from it until the hold ends (SEC-IAM-089 to SEC-IAM-091,
  SEC-IAM-106). Nobody can issue such a link for the owner (ACC-064).
- **The owner has lost every device and passkey.** Host recovery, as in
  F01 (SEC-IAM-092, ACC-004, ADM-034).
- **The page is plain HTTP**, such as `http://192.168.1.7`. A static
  redirect or help page, never a sign-in form (SEC-NET-001).
- **The person is away from home and the owner has set up no remote
  path.** The home posture shows them the static help page, and the owner
  is alerted if a forwarded port is letting outsiders reach the server
  (SEC-NET-024, SEC-NET-027). Remote use in R1 needs the owner's reverse
  proxy or tailnet (security README decision 3).
- **The internet is down but the home network is up.** Sign-in and
  playback still work (ACC-003), provided the browser can still resolve the
  server's HTTPS name ([G20](#gaps-and-questions-the-flows-expose)).
- **The server becomes unreachable after sign-in.** The device keeps
  browsing and playing what it holds (CLI-025). A device that never finished
  its first sync has nothing to fall back on and must say so
  ([G12](#gaps-and-questions-the-flows-expose)).
- **The session is ended elsewhere**, because the person signed out
  everywhere or an admin disabled the account. The next request fails, and
  the web client deletes this account's cached data and returns to the
  sign-in page (SEC-IAM-043, SEC-CLI-009).
- **The server was upgraded under an open tab.** The server refuses calls
  from an older web bundle and the client reloads to the new one
  (SEC-CLI-011). Native apps from R2 negotiate or show "update needed"
  (CLI-032, SEC-CLI-065).

**Rivals today.** Plex's sign-in is easy and has two-factor, and one
plex.tv account lists every server; that is convenient but central, and
plex.tv sign-in was down for about two hours on 2026-07-14. Jellyfin has no
built-in two-factor (#26, 1,103 votes), and its main single sign-on plugin
was archived on 2026-05-12. Jellyfin's Quick Connect is a good local way to
sign in a second device. No media server in the research offers passkeys.

**Other releases.**

- R1, not R2: approving a new browser from a signed-in device (ACC-062,
  step 2a) and the "new device signed in" notice (ACC-071, step 3). The
  baseline needs both in R1, because there is no password to fall back on
  (SEC-IAM-108) and every new device must be announced (SEC-IAM-098).
- R2: native apps find the server on the home network (ACC-104), pin the
  server's key from the invitation or pairing and refuse any other
  (SEC-IAM-051), accept a custom address and proxy headers (CLI-028), and
  keep a non-exportable device key in secure hardware instead of a cookie,
  with 10-minute access tokens bound to it (ACC-051, SEC-IAM-048,
  SEC-IAM-050). One app can hold several servers with a key and a data
  partition per server (CLI-018, ACC-014, SEC-CLI-044). Shared devices get
  a profile picker (ACC-019). Small devices sync partially (CLI-023). The
  app says whether it is local, direct or relayed (ACC-105) and switches
  between home and away by itself (CLI-029). "Require strong sign-in"
  (ACC-054) has less left to do, since every sign-in is already a passkey,
  a device key or the household's provider; it can still require a passkey
  rather than the provider alone.
- Later: one sign-in across several Gunmetal servers (ACC-133), which needs
  its own architecture record first (SEC-IAM-081); moving to a new phone in
  one step (ACC-015).

**Depends on.** ADR 3 (identity store and user log); the identity
architecture record (SEC-STD-006); open decision 7 (HTTPS names) and
security README decisions 1 to 3; open decision 16 (initial sync of 100,000
tracks in under 2 minutes, proposed).

---

## F04. Pairing a TV by scanning a code with a phone

**Release: R2.**

**Who and where.** A member with a signed-in phone, and an Android TV,
Google TV or Fire OS device with the Gunmetal app.

**Starts** with the TV app installed and signed out. **Ends** with the TV
holding its own device key, listed as the person's device or as a household
device, and opening the right profile.

| # | The person | Surface | The server | Features |
|---|---|---|---|---|
| 1 | Installs and opens the TV app. | TV app first launch | Answers LAN discovery, so the TV finds the server with no address typed. Discovery is unauthenticated, so it advertises only a generic name and the server's key, and the TV pins the key it reached (SEC-NET-063). Away from home the person picks or types the address. | CLI-006, CLI-008, CLI-017, ACC-104 |
| 2 | Sees a large QR code, an 8-character code beneath it, and the TV's name. | TV sign-in screen | On the device, the TV creates a non-exportable key pair in secure storage (SEC-IAM-048). The server opens a device authorisation holding the TV's public key, with a 256-bit device code bound to that key, so every poll must be signed by it (SEC-IAM-055). The user code is 8 characters from the base-20 alphabet, lasts 10 minutes and works once (SEC-IAM-056). The QR code carries the approval URL on the server's own HTTPS origin, the user code and the server's identity key (SEC-IAM-057). | ACC-061, CLI-027, ACC-051, ACC-063 |
| 3 | Scans the QR code with the Gunmetal app, or with the phone's camera, which opens the approval page in the web client. | Phone approval sheet | The native app refuses if the server key in the QR code differs from the one it pinned, which exposes an impostor server; in a browser the passkey's origin binding does the same job (SEC-IAM-057). The code identifies the request only, so scanning grants nothing. A request nobody scanned never produces a prompt on anyone's phone (SEC-IAM-052, SEC-STD-027). | ACC-061, CLI-034 |
| 4 | Reads what is asking, chooses "my TV" or "household TV", picks the profiles it may open, answers "Show continue-watching on the TV home screen?", and approves with Face ID or a fingerprint. | Phone approval sheet | Shows the TV's self-reported name marked as unverified, its type, "In this home" or "Somewhere else", how long ago it asked, and exactly which account or profiles it will get; approval needs a passkey or biometric check in the previous 5 minutes (SEC-IAM-058, SEC-CLI-059). Never grants owner or administrator capabilities; a household TV needs the approver to hold the household-device capability (SEC-IAM-059). Enrols the TV's public key against the person or the household, with the allowed profiles, and records its class and key protection level; a software-only key is capped below every administrator capability (SEC-IAM-049, SEC-CLI-031). Asks about the TV home screen once, defaulting to on for a single-profile TV and off when several profiles exist (SEC-CLI-061). Writes an audit entry and sends a new-device notice to the person's other devices (SEC-IAM-098). | ACC-021, ACC-016, ACC-071, ADM-110 |
| 5 | Watches the TV move on by itself. | TV | Completes a challenge-response sign-in with the device key. Access tokens last 10 minutes, are bound to the key, and are renewed only by signing a fresh challenge, so there is no refresh token to steal (SEC-IAM-050). No password or token is typed on, or shown by, the TV. The TV is a limited device: it can never approve other devices, change account security or administer (SEC-CLI-024). | ACC-051 |
| 6 | On a household TV, picks a profile; adult profiles ask for a PIN with hidden digits, or an approval from that adult's phone. | TV profile picker; PIN pad | Lists only the household profiles enabled for this TV, by name and picture, with no guest or account identifier (SEC-IAM-061, SEC-CLI-064). Checks the PIN on the server only, against an Argon2id hash; failures are delayed per TV and profile, never permanently, and after 10 the profile's owner gets one alert (SEC-IAM-062). A PIN only switches profiles and authorises nothing else (SEC-IAM-063). An adult profile's history and continue-watching stay hidden until the PIN or the phone unlocks it (SEC-IAM-110). | ACC-019, CLI-050, ACC-020, DIS-142 |
| 7 | Waits while the TV syncs, then sees Home with the left rail. | TV Home; TV rail | Sends the profile's filtered synced library: all metadata, and artwork in TV sizes within the device's budget (SEC-CLI-020). A PIN-protected profile's data is kept on the TV only encrypted under a key the server releases after the PIN, which the TV forgets when it switches away (SEC-IAM-065). | CLI-023, LIB-142, DIS-144, CLI-035, CLI-038 |
| 8 | Later finds or removes the TV. | Account > Sessions and devices; Admin > Household > Devices | On revocation, fails the TV's next request and closes its open connections and streams within 5 seconds (SEC-IAM-043, SEC-NET-034). The TV deletes its credentials and synced data when told (SEC-IAM-053). | ACC-068, ACC-069, ACC-122 |

**When the TV is somewhere else**, such as a grandparent's house, a QR
picture is not enough, because anyone can send one ("scan this to fix
grandma's TV"). The person types the code shown on the TV into the phone,
and confirms a matching code shown on both screens; the prompt reads "Type
the code on the TV to confirm" (SEC-IAM-060). A remote enrolment grants one
named profile at most, never a household TV, and every adult in the
household is alerted (SEC-IAM-059).

**When it goes wrong.**

- **The code expires before anyone scans it.** The TV shows a fresh code
  without a key press; a new code is free (SEC-IAM-056).
- **The wrong person scans it.** The sheet names the TV and says where it
  is, so they decline, and the request lapses unapproved.
- **Someone tries codes by brute force.** A code dies after 5 wrong
  guesses, and code entry is delayed per approver and capped server-wide
  (SEC-IAM-056).
- **A stranger sends a QR picture of their own TV.** The server sees the
  approving phone and the TV on different networks, so the sheet says
  "Somewhere else" and requires the typed code and the matching code. Even
  then the TV can get one named profile at most, and every adult is told
  (SEC-IAM-059, SEC-IAM-060).
- **An impostor server on the home network answers discovery.** The native
  approving app refuses because the server key differs from its pin; a
  browser approval fails because the passkey belongs to the real origin
  (SEC-IAM-057).
- **The phone is away from home.** Approval works wherever the phone can
  reach the server, which in R2 includes iroh (ACC-096), but the two
  devices are then on different paths, so the typed and matching code are
  required (SEC-IAM-060).
- **A household TV turns up on another network**, such as a holiday home.
  It is suspended and the household's adults are alerted, because household
  devices work only on the home network by default (SEC-IAM-109; security
  README decision 16, owner to confirm).
- **The household TV has not been used for 30 days.** It goes dormant and
  any adult wakes it with one tap from a phone; after 365 days dormant it
  is deleted (SEC-IAM-109).
- **The TV cannot reach the server.** Connection status says why (ACC-105),
  and the owner's connectivity check helps (ADM-134).
- **The household has iPhones and no Android phone.** There is no native
  iPhone app in R2 (CLI-005 is Later), so the camera opens the approval page
  in the web client, which the baseline allows for any signed-in personal
  device (SEC-IAM-057). This settles
  [G9](#gaps-and-questions-the-flows-expose).

**Rivals today.** Jellyfin's Quick Connect already signs in a TV with a
six-digit code, locally and for free, once the admin enables it; a QR
version is planned (#2642, 225 votes). Plex and Emby use link codes or PINs
through their account services. Gunmetal's differences are that the phone
names the TV before approving, the TV gets its own revocable key rather
than a copied session, and a household TV can open several profiles with
nobody's password.

**Other releases.**

- R1 has no TV app. A TV's own browser can open the web client over HTTPS
  and sign in by approval from a phone (F03, step 2a), as a limited device
  that can play but never administer (SEC-IAM-108). There is no password to
  type with a remote.
- R2 also gives command-line tools a key through the same pairing routes
  (INT-027), scoped to a subset of the person's own rights and never to
  administration (SEC-EXT-010, SEC-EXT-012).
- Later: Apple TV, if the App Store licence decision allows (CLI-007, open
  decision 3); Samsung and LG (CLI-010); Roku through the Jellyfin adapter
  (CLI-012).

**Depends on.** ADR 3 (identity store); open decision 10 (relays) for
pairing from outside the home; security README decision 16 (household
devices).

---

## F05. Playing an album and editing the queue

**Release: R1.**

**Who and where.** A listener in the web client, in a desktop browser or
the installed web app on a phone.

**Starts** with the library synced to the device. **Ends** with the album
playing gaplessly at an even level, the queue holding the listener's edits,
and any of their signed-in devices able to pick it up.

| # | The person | Surface | The server | Features |
|---|---|---|---|---|
| 1 | Opens an album from Home, search or the artist page. | Album page | Nothing: the page renders on the device from the synced library, with discs and disc titles, other editions and credits. | MUS-054, MUS-012, MUS-008, MUS-001, MUS-208 |
| 2 | Presses Play, or Play disc. | Album page; now-playing bar | On the device, the client builds the queue with the album in the "From" lane and a context ID, and the core marks any track this browser cannot decode. The server applies the queue operation, assigns a version and stores the queue document in the user log. | MUS-116, LAT-009, MUS-229, MUS-122 |
| 3 | Hears the first track start, and sees a badge such as "Original FLAC, 24-bit, played directly". | Now-playing bar; quality badge | Issues a capability URL whose path carries an HMAC-signed token bound to the session, the item, the representation and an expiry of the track's length plus 10 minutes, at most 4 hours (SEC-API-026, SEC-API-027). Media routes accept only that capability, never the cookie (SEC-API-029), and every range request re-checks that the session is live and the person may still see the item (SEC-API-028, SEC-IAM-046). Serves one byte range per request of the original file (SEC-NET-050), or copies its audio frames into fragmented MP4 without re-encoding when the browser's Media Source Extensions need that; that repackaging parses the file, so it runs in a worker process, never in the server (SEC-MED-018). Records the decision and its reason. | ACC-122, MUS-066, MUS-230, MUS-099, ADM-100 |
| 4 | Hears the album play without gaps and at a steady level. | Player; Track info "gain source" | Supplies encoder delay and padding, a seek index and gain values from the scan. On the device, the player fetches the next track early and applies album gain while the album plays in order. | MUS-067, MUS-069, MUS-070, MUS-071, MUS-084, MUS-085, MUS-087, MUS-088, MUS-089, MUS-090 |
| 5 | Uses the lock screen, notification or media keys, and opens the full-screen player and the lyrics. | OS media panel; now-playing bar; full-screen player; lyrics view | Supplies embedded and sidecar lyrics inside the synced library, parsed into a timed-line model within size limits (SEC-MED-049, SEC-API-090). Clients render lyrics, titles and every other string from files as text, never as markup (SEC-CLI-001). | MUS-073, CLI-070, MUS-108, MUS-110, MUS-154, MUS-155, MUS-156, MUS-158 |
| 6 | From another page, picks three songs one after another with "Play next". They will play in the order chosen. | Context menu | Each pick is a small operation against the last queue version the client saw. The server orders the operations and assigns versions. The picks go into the "Up next" lane in order. | MUS-117, MUS-062, DIS-111, MUS-122 |
| 7 | Adds a playlist with "Add to queue" and drags an album to the end. | Context menu; queue drop zones | Applies the same kind of operation. A multi-item drop is one operation. | MUS-118, MUS-063, CLI-062 |
| 8 | Opens the queue to reorder, remove and clear, and sees where each item is playing from. | Queue panel (full height on desktop, a sheet on phones) | Applies the same kind of operation. | MUS-119, MUS-123, CLI-060, CLI-149 |
| 9 | Turns on shuffle (random, spread out, or by album), reorders while shuffled, then reshuffles the rest. | Shuffle button and menu; queue menu | Stores the shuffle seed and order on the queue, so every device sees the same order. | MUS-126, MUS-127, MUS-120, MUS-128 |
| 10 | Switches on "Continue with" so music carries on after the queue, and sees what will play and why. | Queue panel toggle | Supplies the neighbour table. On the device, radio rules in the core pick the tracks. The lane is off by default. | MUS-129, DIS-070, DIS-062, DIS-060 |
| 11 | Loves a track from the bar, sets a sleep timer and saves the session as a playlist. | Now-playing bar; player menu; queue menu | Writes love events to the user log. Saving the queue creates a playlist (F06). | MUS-109, MUS-180, MUS-076, MUS-077, MUS-125 |
| 12 | Closes the tab, and later opens the client on the phone. | Player bar prompt "Continue on this device" | Nothing more: the queue and position are already in the phone's copy. | CLI-103, MUS-122 |
| All | Listens. | None | Records each play with its real timestamp, plus counts and skips, unless a private session is on. A play event holds only the profile, the item, the device, times, position and completion: no address, location or user agent (SEC-PRV-002). Administrators who look at live sessions see that the listener is playing, on which device and how, but not the title unless the listener chose to show it (SEC-PRV-025). | MUS-182, MUS-183, ACC-117 |

**When it goes wrong.**

- **Two devices edit the queue at once.** The server rejects an operation
  built on a stale version, and the client rebases and shows the result
  (MUS-122). This rule has to be written and tested before handoff (F08) is
  built.
- **A track cannot be decoded in this browser, or is damaged.** It is
  dimmed before play, skipped with a reason and listed in the health report
  (MUS-229, MUS-079). F12 covers the rest.
- **The stream URL expires during a long pause or a three-hour mix.** The
  client fetches a fresh one and resumes at the same position with no
  visible error; the baseline makes this a requirement with an
  injected-clock test (SEC-API-027). This settles
  [G10](#gaps-and-questions-the-flows-expose).
- **Access ends while a track is playing**: the listener signs out
  everywhere from another device, an administrator ends the session or
  disables the account, or the library grant is withdrawn. The next range
  request is refused and the control socket closes within 5 seconds, so the
  music stops at the end of what was already buffered. The player shows a
  plain message, and the web client deletes this account's cached data and
  shows the sign-in page (SEC-IAM-043, SEC-API-028, SEC-CLI-009). A leaked
  stream URL stops working at the same moment.
- **Two browser tabs on one profile both try to play the queue.** The last
  Play wins: the other tab pauses at its point on its next sync and its
  bar offers "Continue on this device" (CLI-103), using only the R1 queue
  versions ([G11](#gaps-and-questions-the-flows-expose)).
- **The page is plain HTTP on a LAN address.** There is no player there:
  the server answers with a redirect or help page (SEC-NET-001).
- **A stream limit is reached**, server-wide, for a guest or for a share
  link. The player says which limit and what to do, never failing silently;
  the next track's early fetch for gapless playback counts as the same
  playback (SEC-TM-068, SEC-API-031, ACC-075).
- **iPhone browsers restrict background audio**, so R1's lock-screen and
  background behaviour on iOS is best effort (clients map, dependencies).

**Rivals today.** Plexamp already does gapless playback, loudness levelling
and a good queue, and it is the bar to clear; it runs only in Plex's own
apps, and some features need Plex Pass. Spotify is the reference for queue
lanes. Navidrome's web player will not do gapless (#745), and Jellyfin's
web gapless depends on the browser; gapless is Jellyfin's most-voted music
request (647 votes, open since 2019). Apple Music plays several "play next"
picks in reverse order, which is what MUS-117 fixes.

**Other releases.**

- R2: undoing queue edits (MUS-121); queue history (MUS-124); several
  saved queues (MUS-131); crossfade and album-aware fades (MUS-091,
  MUS-092); an equaliser (MUS-094); gapless and background playback in the
  native apps (MUS-068, MUS-074); the landscape player and info cards
  (MUS-112, MUS-111); handoff (F08).
- Later: a visualiser (MUS-083); listening together (MUS-202).

**Depends on.** ADR 3 (the queue lives in the user log); open decision 9
(amend ADR 2 to name the audio packager); open decision 8 (loudness
measurement). Which containers each browser accepts for the packager is
unverified.

---

## F06. Building a playlist

**Release: R1.**

**Who and where.** A listener in the web client. The flow covers a
hand-built playlist, a smart playlist and an imported one.

**Starts** with a synced library. **Ends** with playlists that follow the
person to every device and survive moves, upgrades and cache rebuilds.

| # | The person | Surface | The server | Features |
|---|---|---|---|---|
| 1 | Creates a playlist from the sidebar. | Sidebar; playlist page | Writes a playlist-created event to the user log with a random public ID. | MUS-132, INT-008 |
| 2 | Adds a track with "Add to playlist", searching their playlists, seeing where the track already is and adding it to several at once. | Add-to-playlist sheet | Writes playlist events that refer to tracks by content identity, not by path. | MUS-133, LIB-028 |
| 3 | Is asked before adding a track that is already there. | Add sheet prompt | Nothing: the check runs on the device. | MUS-134 |
| 4 | Selects twenty tracks across an album and search results and drags them onto the playlist in the sidebar. | Selection bar; sidebar drop target | Applies the batch as one operation. | DIS-110, MUS-063, CLI-062 |
| 5 | Reorders, removes and renames; finds a song inside a long playlist; sorts by date added. | Playlist page search and sort | Writes reorder, remove and rename events. Finding and sorting run on the device. | MUS-132, MUS-135, DIS-091 |
| 6 | Sees a cover made from the albums inside, pins the playlist, and adds it to the Home shortcuts. | Playlist tiles; sidebar; top of Home | Writes pin events. The client composes the cover from artwork it already holds. | MUS-137, MUS-139, DIS-013 |
| 7 | Exports the playlist as M3U8 to use elsewhere. | Playlist menu: export | Writes an M3U8 with paths relative to the library root. | MUS-140 |
| 8 | Starts a smart playlist. | Smart playlist editor | Nothing yet. | MUS-143, DIS-121 |
| 9 | Builds rules from menus, such as "genre is jazz, not played in a year, rating at least four, 50 at random", and watches the matches update. | Rule editor sheet | Nothing while editing: on the device, the core evaluates the rules against the synced library, with seeded randomness so the list is the same on every device. | MUS-144, MUS-145, MUS-146, DIS-119, DIS-120, DIS-122 |
| 10 | Saves, and the playlist keeps itself up to date. | Playlist page | Stores the rule tree in the rule store and syncs it. Evaluates rules on the server only for adapters and sync filters, and again when the library changes. | DIS-119, DIS-121 |
| 11 | Alternatively saves a library filter as a smart playlist, or plays "Loved tracks", which already is one. | Filter sheet "Save as"; Library > Loved | Uses the same rule store. | DIS-105, MUS-149, DIS-046 |
| 12 | Imports an M3U from another player and reads the match report. | Playlist menu: import; upload dialog; Admin > Migration > Review | Takes the file only through the upload route, with its type and size limits and a per-person quota (SEC-API-085, SEC-API-088), and parses it in the core within its budgets. Matches each entry by path relative to a library root, then by tags and duration, only against items in libraries this person can see (SEC-MED-051); URLs and paths outside the libraries are dropped and listed. Records the reason, and sends misses to the review queue. | MUS-140, ADM-043, ADM-044 |

**When it goes wrong.**

- An imported entry matches nothing. It waits in the unmatched queue with
  its reason (ADM-044).
- A track in a playlist is moved or upgraded. The entry follows its
  identity (LIB-029, LIB-030).
- A track is deleted. The entry survives the trash grace period and comes
  back with the file (LIB-033). What it shows after the file is purged is
  not settled ([G6](#gaps-and-questions-the-flows-expose)).
- A rule matches nothing. The editor shows that live, before saving
  (DIS-120).
- An `.m3u` found in a music folder became a playlist (LIB-192), but media
  is read-only (LIB-007), so an edit cannot be written back (G5).
- The playlist is edited on two devices. Both are online in R1, so the
  server orders the events; offline edits arrive in R2 (CLI-094).

**Rivals today.** Plex's smart playlists are good and free. Navidrome
offers more than 80 rule fields but makes people write JSON, and Jellyfin
has no smart playlists in its core (588 votes). Spotify's add-to-playlist
sheet and duplicate prompt are the reference for steps 2 and 3. Gunmetal's
edge is one free rule language with a live preview that drives home rows
now and download rules in R2.

**Other releases.**

- The playlist write API (INT-138) and its scoped tokens (ACC-049) move
  from R1 to R2. The baseline has no scripting credential in R1: API keys
  arrive in R2, scoped to a subset of their creator's own rights, never
  administrative, created only after a passkey check, and listed with the
  person's devices (SEC-IAM-083, SEC-EXT-008 to SEC-EXT-014; security README
  decision 8, owner to confirm). The web client still uses the same routes
  in R1.
- R2: playlist folders (MUS-136); a custom image (MUS-138); shared and
  collaborative playlists (ACC-091); offline edits that merge later
  (CLI-094); stable daily or weekly snapshots (MUS-147); keeping a smart
  playlist downloaded (CLI-080); pinning a list as a Home row (DIS-126);
  Navidrome smart playlist import (MUS-148). A share link to a playlist
  (ACC-086) arrives earlier, in R1 for music (SEC-API-097; security README
  decision 7, owner to confirm; see F10).
- Later: a playlist from a prompt (MUS-153); streaming-service playlist
  import (INT-153).
- No: suggestions inserted into a person's own playlists (MUS-130,
  DIS-182).

**Depends on.** ADR 3 (playlists and rules live in the user log).

---

## F07. Downloading for offline and playing on a plane

**Release: R2.** R1 has no downloads (see below).

**Who and where.** A listener with the Android app (CLI-004), downloading
music and, in this flow, a film and some episodes too.

**Starts** at home with a signed-in phone holding a device key. **Ends**
with downloads that play with no signal, and with plays, ratings and edits
made in the air merged when the phone reconnects.

| # | The person | Surface | The server | Features |
|---|---|---|---|---|
| 1 | Taps the download switch on an album, an artist and a playlist. | Download toggle on album, artist and playlist pages | Checks the person's download right; guests have none by default (SEC-IAM-080). Issues capability URLs and an offline grant: a server-signed record bound to the device key and the list of items, expiring after the administrator's maximum (30 days by default) (SEC-IAM-054, SEC-CLI-036). | CLI-078, ACC-044, CLI-095 |
| 2 | Sets rules: keep "Loved tracks" and one smart playlist downloaded, keep what was played recently, all within a 20 GB cap. | Downloads > Rules; "Keep downloaded" on any smart playlist; Settings > Storage | Supplies the rules in the shared rule language. On the device, eviction makes room for automatic downloads and never removes a hand-picked one. Downloads and the library live in app-private storage, out of device and cloud backups (SEC-CLI-033, SEC-CLI-035). | CLI-080, CLI-081, CLI-082, DIS-119 |
| 3 | Chooses Opus copies to save space and Wi-Fi only. | Download quality setting; Settings > Downloads > Network | Encodes each Opus copy once in the sandboxed worker and caches it for the next device (SEC-MED-066, SEC-OPS-062). There is no SD-card option in R2: downloads stay in app-private internal storage (SEC-CLI-035), and removable storage waits for an encrypted format (SEC-CLI-072, Later; owner to confirm). | CLI-083, MUS-213, CLI-091 |
| 4 | Downloads a film and keeps the next three episodes of a show. | Download button on films and episodes; Show page > Keep next episodes | Serves originals, or a remuxed copy where the device's player prefers another container. Skip markers, chapters, subtitles and fonts travel with each download. | CLI-084, CLI-085, CLI-087, CLI-088, VID-175 |
| 5 | Closes the app. Downloads continue and resume after a dropped connection. | Notification; Downloads screen | Serves resumable ranges and refreshes URLs on request. Streams on other devices take priority over downloads. | CLI-090, ACC-110 |
| 6 | Checks the downloads manager before leaving. | Downloads screen | Returns typed errors that the app turns into plain reasons. | CLI-089, MUS-215 |
| 7 | On the plane, browses as usual; a "Downloaded" filter narrows any list. | All lists; "Downloaded" filter | Nothing: the synced library, the search index and the grant are on the phone. | CLI-026, MUS-209, CLI-025, DIS-084 |
| 8 | Plays an album with lock-screen controls and lyrics, then the film with subtitles. | Player; lock screen; lyrics view | Nothing. | CLI-069, MUS-074, MUS-160, VID-175 |
| 9 | Rates tracks, edits a playlist and stops the film halfway. | Context menu; playlist page; player | Nothing yet: on the device, the edits queue as operations. | CLI-094, MUS-214, VID-118 |
| 10 | Lands and reconnects. | Sync status; History, showing the device | Ingests the offline plays with their real timestamps and removes duplicates. Applies the queued edits under the stated conflict rules, updates resume points and renews the grant. | CLI-093, CLI-094, VID-118, CLI-095 |

**When it goes wrong.**

- Storage fills up. Automatic downloads make room, and hand-picked ones
  stay (CLI-082).
- The phone is on mobile data with "Wi-Fi only" set. Downloads wait and say
  so; the rule is never broken silently (CLI-091). Plexamp downloaded over
  cellular while showing "paused" in September 2026.
- The trip outlasts the grant. Downloads stop playing until the phone makes
  contact, and the Downloads screen shows the expiry beforehand (CLI-095).
  Expiry never interrupts a track or queue already playing, deletes no
  file, and says "Connect to your server once to keep listening offline".
  The grant's age is measured from the last contact with the phone's own
  elapsed-time clock, so changing the date does not extend it
  (SEC-CLI-036). The lifetime is set by an administrator, 30 days by
  default (clients map, open decision 4; SEC-IAM-054).
- The owner revokes a lost phone's downloads. That takes effect only when
  the phone next makes contact, when the phone deletes them (ACC-045,
  CLI-096, SEC-IAM-054); F15 says so plainly.
- An offline edit conflicts with a change made at home. A rare conflict
  notice says which change won (CLI-094).
- The person has no download right. The switch explains why (ACC-044).

**Rivals today.** Plexamp's offline music is good: "Keep Played Music"
arrived in September 2026 and its download cap is gone, but downloads need
Plex Pass. Finamp downloads for free, and Symfonium caches automatically.
Emby is best at letting an admin see and control synced media. Offline is
Jellyfin's most-voted request (1,820 votes). Gunmetal's edge is free,
rule-driven downloads under a cap, offline that uses the same screens, and a
grant the owner can revoke.

**What R1 offers instead.** No downloads. The web client fetches upcoming
tracks early on patchy signal (CLI-099). In a secure context, the installed
web app can browse and search its synced copy with no server (CLI-025), but
it plays only what it has already buffered. Downloads in the browser are
Later (MUS-217). Open decision 5 recommends shipping the Android music app
as an R1 point release, which would bring music downloads forward.

**Other releases.**

- R2 also lets a person start a phone download from another device
  (CLI-097).
- Later: smaller transcoded video downloads (CLI-086); offline music on
  Wear OS and Apple Watch (CLI-128, CLI-129); copying to a folder or drive
  (CLI-100); choosing an SD card or other removable storage (CLI-059),
  only once downloads there are encrypted in seekable authenticated chunks
  under a key in the platform keystore (SEC-CLI-072).

**Depends on.** ADR 3 (grants, rules and queued edits are durable user
data); the remuxer and the sandbox (R2 infrastructure); clients map open
decisions 4 and 5.

---

## F08. Moving playback from the phone to another device

**Release: R2.** R1 offers "continue on this device" (see below).

**Who and where.** A listener playing on the Android app, with a TV app
or the web client on a laptop signed in to the same profile, or a
Chromecast speaker on the network. The desktop shell is Later
(SEC-TM-074), so a computer takes part through the web client.

**Starts** with music playing on the phone. **Ends** with the same queue,
lanes and position playing on the chosen device, and the phone acting as
its remote.

| # | The person | Surface | The server | Features |
|---|---|---|---|---|
| 1 | Taps the device button. | Device picker in the player bar and Now Playing | Keeps a control channel per signed-in session, authenticated before any message, checked against the server's own origins, and closed when its session ends (SEC-IAM-016). Reports which of the profile's players are present. | CLI-101, MUS-197 |
| 2 | Sees the living-room TV, the laptop and a Chromecast speaker, each with its state. A household TV that another profile is using shows only as "In use". | Device picker | Lists only this profile's own players; controlling another person's player is Later (ACC-047, SEC-HIS-014). Household devices this profile may use appear greyed as "In use", with no profile name or title, and the session events behind the list are filtered per recipient (SEC-PRV-022, SEC-API-016). | CLI-101, ACC-121 |
| 3 | Picks the laptop. | Device picker; accept prompt on the target where needed | Sends a transfer command over the control channel, carrying item IDs and the position, never a stream URL or token. The target loads the queue document at the current version, with its lanes, shuffle order and position, asks for its own capability URLs under its own session (SEC-API-026), and starts. The phone stops. | CLI-101, MUS-122, MUS-116, LAT-009, VID-144 |
| 4 | Controls the laptop from the phone, including its volume. | Remote mode in Now Playing | Relays play, pause, skip, seek and volume commands, each authorised per profile, from a closed set with no free text (SEC-HIS-014). | CLI-102, MUS-198 |
| 5 | Edits the queue on the phone while the laptop plays. | Queue panel in remote mode | Applies queue operations to the shared, versioned document, and the target follows them. | MUS-122, MUS-119 |
| 6 | Alternatively casts to the Chromecast speaker. | Cast button; device picker | Issues capability URLs scoped to one item and to the cast session the phone started, expiring within the item's length plus at most an hour (the stream lifetimes of SEC-API-027 stay inside that bound), revocable, and never carrying a session token (SEC-NET-064). The phone refreshes them; the receiver holds no credential of its own, because cast credentials are Later (SEC-TM-074, SEC-CLI-071). Only those representations may be read cross-origin (SEC-API-098). Picks a format the receiver can decode. Away from home, the phone relays. | CLI-106, MUS-203, CLI-110, CLI-111 |
| 7 | Stops or adjusts casting from the notification. | Notification | Nothing beyond the control channel. | CLI-112 |
| 8 | Takes playback back to the phone when leaving the house. | Device picker on the phone | Runs the same transfer in the other direction. | CLI-101 |

**When it goes wrong.**

- Two devices send commands at once. The server orders them by queue
  version under a written conflict rule; CLI-101 requires that rule to be
  designed and tested before CLI-101 or CLI-102 is built.
- The target fails to start. The map does not say what happens
  ([G14](#gaps-and-questions-the-flows-expose)).
- The cast receiver cannot decode the file. The decision engine picks a
  path the receiver can play (CLI-111). URL lifetimes must cover a whole
  film or be refreshable, because receivers cannot join an iroh connection
  (clients map, risks).
- Someone tries to control another person's player. It is refused; control
  with the other person's grant is Later (ACC-047, SEC-HIS-014). Jellyfin
  had a broken-access-control advisory on session control in September
  2026.
- The phone's session is revoked while it is driving the laptop. Its
  control channel closes within 5 seconds and its commands stop working;
  the laptop keeps playing under its own session (SEC-IAM-016,
  SEC-IAM-043). A cast the phone started stops at the next range request,
  because its cast session ends with the phone's (SEC-API-028).

**Rivals today.** Spotify Connect is the reference and is excellent, but it
runs through Spotify's account service. Plexamp hands off between its
players through plex.tv. Jellyfin can already remote-control its sessions
without a vendor account. Gunmetal's edge is one versioned queue that any
of the profile's devices can take over with the same lanes and position,
with no central account.

**What R1 offers instead.** R1's only players are browser tabs, so there is
no device picker. The queue persists on the server, and opening the client
elsewhere offers "Continue on this device" (CLI-103, MUS-122). When a
second tab or device presses Play, the last Play wins and the first one
pauses on its next sync, offering the same prompt
([G11](#gaps-and-questions-the-flows-expose)).

**Other releases.**

- R2 also brings Sonos and whole-home audio through Music Assistant over
  the OpenSubsonic adapter (MUS-206).
- Later: AirPlay (CLI-109); headless and dedicated players (CLI-104);
  synchronised multi-room (CLI-105, a Spotify request with 8,800 votes);
  controlling another person's player with their grant (ACC-047); listening
  together (MUS-202).

**Depends on.** The control-channel design and its conflict rule (CLI-101);
open decision 10 (relays) for control away from home; clients map open
decision 7 (Chromecast receiver).

---

## F09. Searching across music and video

**Release: R1 for music; films, shows, episodes and people join in R2.**

**Who and where.** Anyone, in the web client in R1 and on every client
from R2.

**Starts** with a synced library. **Ends** with the person at, or playing,
the thing they wanted.

| # | The person | Surface | The server | Features |
|---|---|---|---|---|
| 1 | Focuses the search field from any screen and sees recent searches. | Global search field; Search screen | Nothing. Recent searches live only on the device and can be cleared; the server never stores or logs what anyone searched for (SEC-PRV-004). | DIS-083, DIS-089 |
| 2 | Types "amelie", and results appear with each keystroke, grouped by type with chips and with no cap. | Search screen; type chips | Nothing at query time. On the device, the core builds the index from the synced library. If that build is too slow on the reference low-end device, the server ships a prebuilt index segment instead. | DIS-084, DIS-083, MUS-061 |
| 3 | Finds "Amélie" despite the missing accent, a typo and a straight apostrophe. | Search | Nothing. | DIS-085 |
| 4 | Types "christmas" and gets the tag and the genre as well as titles. | Search; genre chips | Sends tags, genres and moods in the sync feed. | DIS-086, MUS-017, MUS-019 |
| 5 | Types a composer's name and gets the person, with their roles. | Search; artist page role tabs | Sends credits with their roles in the sync feed. | DIS-087, MUS-005 |
| 6 | Narrows the search to one library, or to Music. | Scope selector | Nothing. | DIS-088 |
| 7 | Plays a result straight away, queues it, or opens it. | Context menu; album page | Nothing beyond F05. | DIS-111, CLI-034 |
| 8 | Goes back and finds the results and scroll position as they were. | Navigation | Nothing. | DIS-112 |
| All | Never sees anything this profile may not see. | Every result | Builds the sync payload from the person's grants and restrictions, so the index on the device never holds a blocked item, its artwork or its search terms (SEC-CLI-020, SEC-IAM-070). | ACC-030, ACC-037, DIS-140 |

**When it goes wrong.**

- The person is on a plane or the server is down. Search still works from
  the local copy on a device marked as the person's own (DIS-084, CLI-025).
  A browser marked as shared keeps its copy only in memory, so search there
  needs the server after a reload (SEC-CLI-010).
- Nothing matches. The empty state should offer to widen the scope.
- The library is large and the device cheap. Index size and build time are
  measured against the budget: under 50 ms per search at 100,000 tracks, a
  proposed goal (open decision 16).

**Rivals today.** Plex's search has been relevance-ranked since 2023 and
has no per-source cap; it is reasonably good. Spotify is fast and
typo-tolerant (unverified). Jellyfin has no typo tolerance (46 votes),
broke accent matching in 10.11.0, and does not search tags (256 votes since
2019). No rival server searches on the device; Plexamp's offline mode,
since September 2026, includes search.

**Other releases.**

- R2: one box covers films, shows, episodes and people (actor, director,
  character) beside music, as the R1 rows DIS-083 and DIS-087 already
  promise for R2, with kind chips (LAT-013). A
  title held in two libraries or editions shows once with its versions
  (DIS-095). Music and Watch get separate homes (DIS-017), so search needs
  a rule for which kinds it covers
  ([G16](#gaps-and-questions-the-flows-expose)). R2 also adds search by
  lyrics (DIS-093), transliteration and CJK (DIS-092), voice search on TV
  and phone (DIS-094, CLI-044), and a keyboard command palette (DIS-090).
- R3: programmes on air or coming up appear in search (DIS-096, LIV-072).
- Later: results outside the library (DIS-097); several servers at once
  (DIS-099); search plugins (DIS-098); the operating system's own search
  (DIS-174).

**Depends on.** Open decision 16 (performance budgets).

---

## F10. Sharing a library with a friend

**Release: R1, when the owner gives the server an outside address; the
no-open-port path arrives in R2.**

**Who and where.** The owner in the web client's admin section, and a
friend in another house with a browser.

**Starts** with the owner's library and, in R1, a server the owner has made
reachable from outside over HTTPS through their own reverse proxy or a
tailnet. **Ends** with the friend holding an account limited to chosen
libraries, signed in with their own passkey on their own device.

| # | The person | Surface | The server | Features |
|---|---|---|---|---|
| 1 | The owner makes the server reachable from outside: a domain with HTTPS behind a reverse proxy, a tailnet name, or a path prefix next to other services. Touches the passkey to confirm. | Admin > Network; Admin > Network > HTTPS; step-up prompt | Declaring a trusted proxy, and so leaving the home posture, is an owner-only step-up action that is audited and announced (SEC-IAM-041, SEC-TM-017, SEC-OPS-038). Each proxy is declared as a private overlay or as public; requests through a public proxy get the internet posture, with stricter limits, no setup, and no administration unless the owner turns remote administration on (SEC-NET-019, SEC-NET-045). Ignores forwarding headers from anyone not on the list, and takes the client address from the right-most untrusted entry (SEC-NET-016). Serves under the configured base path and serves TLS with the owner's certificate. | ACC-097, ACC-134, ACC-098, ADM-022 |
| 2 | The owner creates an invite: which libraries, how many uses, when it expires. | Admin > Invitations | Stores the invite as a capability with a 128-bit secret, an expiry (7 days by default), a use count (1 by default) and a preset that can be no greater than the inviter's own rights (SEC-IAM-073, SEC-IAM-078). | ACC-080, ACC-037 |
| 3 | The owner sends the link or shows the QR code, and sees which address the link carries. | Admin > Invitations | Builds the link from the configured public address, never from the request's Host header (SEC-NET-015), and puts the secret in the fragment, so it never reaches a server log (SEC-NET-036). Warns when the address cannot be reached from outside ([G7](#gaps-and-questions-the-flows-expose)). | ACC-080 |
| 4 | The friend opens the link and reads, on one screen, who is inviting them to which server, what will happen, and a short privacy notice: what is stored about them, who can see what, what leaves the server, and how to export and delete. | Invite landing in the web client | Serves the landing page only over HTTPS (SEC-IAM-078). The page removes the secret from the address bar before any request and redeems nothing until the friend confirms (SEC-CLI-013). Checks the invite in a POST body, rate-limited, with the same response whatever is wrong with it (SEC-API-096, SEC-API-058). The server's name may show here, because the invitation authorises it (owner to confirm). The privacy notice is generated from the server's real settings, at most five plain sentences (SEC-PRV-053). | ACC-080, ACC-120 |
| 5 | The friend creates an account and enrols a passkey on the spot, or links their own identity provider if the owner offers one. | Invite landing; Account > Sign-in methods | Creates a local account from the invite's policy and enrols the friend's own credential in the same transaction; no password exists (SEC-IAM-079, SEC-IAM-025). Consumes one use of the invite atomically (SEC-STD-029). Logs the redemption and tells the inviter which device redeemed it. | ACC-080, ACC-006, ACC-050 |
| 5a | For an invite that makes the friend a household member, or grants more than one library, the friend reads a short code from their screen to the owner, and the owner confirms it in Admin > Invitations after seeing the friend's device description. A guest invite for a single library skips this step. | Invite landing (waiting for confirmation); Admin > Invitations (pending confirmation) | Leaves the new account pending, with no grants, until the inviter confirms the matching code; a wrong code fails (SEC-IAM-079). This is what stops a forwarded link from quietly letting a stranger in. | ACC-080 |
| 6 | The friend sees only the shared libraries. | Home; library views | Builds the friend's synced library from their grants and checks authorisation on every object fetch (SEC-IAM-070). A guest by default has no downloads, no household devices and no view of other people or their activity (SEC-IAM-080). | ACC-037, ACC-121, ACC-030, MUS-027 |
| 7 | The owner sees that the friend is listening, on which device and how, but not what, unless the friend has chosen to show titles. | Admin > Dashboard (Now playing) | Lists live sessions from the session registry with the person, device, quality and playback method, and the title only by the friend's choice. Each admin look at live sessions is rate-limited and recorded in the friend's own security log (SEC-PRV-025, SEC-IAM-077; owner decision 5, owner to confirm). | ADM-099 |
| 8 | The owner changes the friend's libraries, pauses the account, or removes it. | Admin > Users | Updates grants, which apply from the friend's next request, so the next sync removes what the friend may no longer see (SEC-IAM-076). Disabling ends the account's sessions at once and cuts streams in flight within 5 seconds (SEC-IAM-103, SEC-IAM-043). Deleting keeps the data for a 7-day grace period and offers the friend an export first (SEC-IAM-103). | ACC-037, ACC-008, ACC-122 |

**When it goes wrong.**

- **The owner has no outside address**, or the server is still in the home
  posture. The link would carry an address the friend cannot reach, so the
  invitation screen says so before the link is sent
  ([G7](#gaps-and-questions-the-flows-expose)).
- **The owner forwarded a router port instead of setting up a proxy.** The
  friend gets the static help page, and the owner gets an exposure alert
  with the fix and a "turn on remote access instead" button (SEC-NET-024,
  SEC-NET-027).
- **The link leaks or is forwarded.** It expires, has a use count and can be
  revoked, every redemption is logged, and for anything beyond a
  single-library guest the matching code stops a stranger at step 5a
  (ACC-080, ADM-110, SEC-IAM-079).
- **Someone guesses invite codes.** Redemption is rate-limited per source
  and server-wide, and every failure looks the same (SEC-IAM-101,
  SEC-API-058).
- **The friend's browser cannot make a passkey and they have no other
  device.** They can use a security key, or the owner's identity provider if
  one is set up. There is no password path (SEC-IAM-025; owner decision 1).
- **The friend wants to know what the owner can see.** Account > What admins
  can see shows it from R1, generated from the same policy the server
  enforces (SEC-IAM-104, SEC-PRV-027). This settles G8.

**Rivals today.** Plex is genuinely best here: a friend in another house
gets chosen libraries in the Plex app they already have, with no address to
type. Since 29 April 2025, though, remote video needs Plex Pass for the
owner or a pass for each friend; music is exempt. Jellyfin has no
invitations (its sign-up request #494 has 227 votes), so owners add Wizarr
or jfa-go and run their own proxy. Navidrome and Immich already have
public share links for people with no account.

**Other releases.**

- R1, not R2: the page that shows each person what admins can see about
  them (ACC-115), because the baseline requires it from R1 (SEC-IAM-104,
  SEC-PRV-027). What admins see is fixed by the baseline: no server setting
  can add history or titles, and the only choice is each person's own, to
  show titles (ACC-116, SEC-PRV-025).
- R1, not R2: share links for a music track, album or playlist reach people
  with no account (ACC-086, ACC-087, ACC-088, ACC-089; security README
  decision 7, owner to confirm), always with the baseline's rules: a 128-bit
  secret in the fragment, one object, the sharer taken from the session, 30
  days by default, listen-only unless the owner allows downloads
  server-wide, an optional password under the guessing delays, two streams
  at once, a cap on uses or bytes, and a distinct-address count that
  suspends the link and alerts the sharer; no link-preview metadata unless
  the sharer turns it on; and a page that shows nothing of the sharer, other
  people, the library or the server's name (SEC-API-097, SEC-PRV-031,
  SEC-NET-047). Revoking a link stops it on the next request, streams
  included (SEC-API-028). Managed profiles cannot create them. Video links
  follow in R2, off by default (ACC-092).
- R1, not R2: stream limits that count playing, not browsing, server-wide
  and per guest, with a "too many streams" message (ACC-075, SEC-IAM-102,
  SEC-TM-068).
- R2: native apps reach home over iroh with no open port, no account and no
  fee, and the invite carries the server's node address and key, which the
  app pins (ACC-096, ACC-083, ACC-100, ACC-101, SEC-IAM-051, SEC-NET-060).
  Turning remote access on is an owner step-up action (SEC-IAM-041). An
  onboarding page says "install this app, then tap here" (ACC-082).
  Membership can end on a date (ACC-081). Remote access can be on or off
  per person (ACC-103). Quality caps never transcode silently (ACC-107,
  ACC-108). The house's upload is shared fairly (ACC-109). Policies can be
  named and shared, each with its own stream limits (ACC-038). A
  single-item guest link (ACC-135). Playlists can be collaborative
  (ACC-091).
- Also R2: remote access in the browser with no domain, through the
  project's edge, which forwards only ciphertext so TLS ends on the owner's
  server, is off until the owner turns it on, and gives every edge request
  the internet posture (ACC-102; SEC-NET-041 to SEC-NET-043; security
  README decision 3, owner to confirm).
- Later: asking to join (ACC-085); link previews in chat apps (ACC-090);
  and, with the importers for rival databases (SEC-TM-074; see F11),
  re-inviting people from an old server (ACC-084, ADM-048), through the
  same invitation rules.

**Depends on.** Open decision 10 (who runs relays); open decision 13 (what
admins can see) and security README decisions 3, 5 and 7.

---

## F11. Migrating from Plex or Jellyfin

**Release: R1 for listening-service export files and M3U playlists;
importing from the rival servers' own databases is Later.**

**Who and where.** The owner, in the admin section, with the old server's
files still to hand.

**Starts** with Gunmetal set up (F01) and the same music scanned (F02).
**Ends** with history and playlists attached to the right files, and a list
of what did not match.

| # | The person | Surface | The server | Features |
|---|---|---|---|---|
| 1 | Answers "Coming from another server?" at setup, or opens Migration later. | Welcome > Import; Admin > Migration | Lists the importers this release has. | ADM-030 |
| 2 | Uploads Last.fm or ListenBrainz export files. | Admin > Migration > Listening services; upload dialog | Takes the files through the upload route with its type, size and quota limits (SEC-API-085, SEC-API-088) and parses them in the core within its budgets, with no network grant. Writes them only into the uploader's own history; each other person imports their own from Account > Your data, because no admin may write, read or export another adult's history on their behalf (SEC-PRV-022, SEC-PRV-025). Marks each listen as imported, so a scrobbler never sends it back. | ADM-042, INT-107, MUS-189 |
| 3 | Uploads a folder of M3U playlists exported from the old server, where it can export them. | Admin > Migration > Playlists | Parses M3U and M3U8, keeping only entries that match items in libraries the importer can see (SEC-MED-051). | ADM-043, MUS-140 |
| 4 | Waits for matching. | Admin > Tasks; Admin > Activity | Runs the one matcher every importer shares: path relative to a remapped root, then MusicBrainz IDs, then tags and duration. Records a reason and a confidence for each match. | ADM-044 |
| 5 | Reads the match report and works through the unmatched queue. | Admin > Migration > Review | Holds low-confidence matches for a yes. | ADM-044, LIB-099 |
| 6 | Sees years of plays in history and the playlists in the sidebar. | History page; sidebar | Writes the imported plays and playlists to the user log, keyed by content identity. | MUS-183, MUS-132, LIB-028 |

**When it goes wrong.**

- Paths changed between servers. The matcher remaps roots and falls back to
  tags and duration (ADM-044).
- An import goes badly. In R1, imported events are marked and can be removed
  as a batch (INT-107); a full undo is R2 (ADM-046).
- The owner is leaving Plex. Plex has no built-in music playlist export
  (one forum user spent weeks rebuilding 614 items), so in R1 a Plex
  owner's playlists depend on third-party tools. Play counts and ratings
  held only in Plex's database cannot come across until the Later
  importers. Whether Jellyfin exports playlists is unverified.

**Rivals today.** No media server imports from another one out of the box.
People use third-party tools such as WatchState (about 1.6k stars) and
JellyPlex-Watched, which has a dry-run mode. Moving between Jellyfin
servers is good, because users travel inside its backup. Plex's own
cross-server sync excludes music.

**Other releases.**

- R2: an iTunes library file (ADM-041); a dry run (ADM-045) and an undo
  (ADM-046) for every importer; mirror mode while both servers run
  (INT-116). NFO files are read as they are (LIB-126) and ratings come from
  tags (MUS-045). Friends' existing apps keep working through the Jellyfin
  adapter's music subset (INT-098) and the OpenSubsonic adapter (INT-086,
  F19).
- Later, as the baseline's release table places importers for rival
  databases: music history, ratings and playlists from a copy of Plex's
  database (ADM-036, MUS-141) and video watch state (ADM-037); music and
  video state from Jellyfin and Emby (ADM-038, ADM-039); stars, ratings
  and playlists from Navidrome and other Subsonic servers (ADM-040);
  reading a rival's API instead of a database copy (ADM-047); users brought
  over as invitations with their old library access (ADM-048, ACC-084);
  and syncing again while both servers run (ADM-049). When they come, a
  rival's database is opened read-only in a jailed worker with its schema
  distrusted (SEC-STD-031); reading a rival's API is an egress grant, which
  is an owner step-up action (SEC-IAM-041); each person's imported history
  lands only in their own profile, and the admin who ran the import cannot
  view it afterwards (SEC-PRV-025); and imported users arrive as ordinary
  invitations, with their matching code (SEC-IAM-079). Also Later:
  collections and metadata edits (ADM-050, LIB-182); the Jellyfin adapter's
  video subset (INT-099).
- No: Plex API compatibility (INT-101).

**Depends on.** ADR 3; fixtures from several versions of each rival's
database schema (ADM-036).

---

## F12. Recovering from a failed playback

**Release: R1 for music; the video cases arrive in R2.**

**Who and where.** The listener first, in the web client; then the owner,
in the admin section.

**Starts** with something that will not play, or stops. **Ends** with
either music playing again, or a clear reason and a next step for both the
listener and the owner.

This flow is a set of cases rather than one path. Each row is one way
playback can fail, with what the listener should see.

**Listener cases in R1.**

| Case | What the listener sees | Surface | The server | Features |
|---|---|---|---|---|
| A. This browser cannot decode the format, for example ALAC in some browsers (unverified) | The track is dimmed with a reason before anyone presses play, and the queue skips it with a note. | Track rows; queue; Library health | Sends codec and container per file in the sync feed. On the device, the core's capability probe decides. The health report counts such files by format and browser. | MUS-229, MUS-032 |
| B. The file is damaged | The player skips it with a notice that links to the health report, and never plays noise. | Notice; Library health > Problems | Flags the damage at scan time. The player keeps a true-peak ceiling. | MUS-079, LIB-193 |
| C. The connection drops | Music carries on from what was fetched ahead, a quiet banner says the server is unreachable, and playback resumes when it returns. | Quiet status banner; player | Serves byte ranges, so the player resumes exactly where it stopped. | CLI-099, MUS-070, CLI-025 |
| D. The stream URL expired during a long pause | Nothing visible: the player gets a fresh URL and resumes at the same position. | None | Issues a new capability URL while the session is valid; transparent refresh is a requirement (SEC-API-027). | ACC-122 |
| E. An admin ended the session, the device was revoked, the account was disabled, or the library grant was withdrawn | Playback stops with a plain message (the admin's own words, when there are any), followed by the sign-in page. | Client banner; sign-in page | Refuses the next range request and closes the control socket within 5 seconds (SEC-IAM-043, SEC-API-028). The web client then deletes the account's cached data (SEC-CLI-009). | ADM-102, ACC-069, ACC-008, ACC-122 |
| F. The drive holding the file is offline | The item is greyed out with an offline badge, and pressing play says why. | Greyed-out items | Checks root health, marks the root offline and alerts the admin. | LIB-032, ADM-108, ADM-116 |
| G. The file was moved or deleted | A move inside the library roots changes nothing. A missing file is listed, and it returns with its history if restored within the grace period. | Library health > Missing files | Detects moves and keeps deleted entries in a trash with a grace period. | LIB-029, LIB-034, LIB-033 |
| H. The server is restarting or upgrading | A status page instead of a refused connection, and the queue intact afterwards. | Startup page | Opens its listener before the database. The page shows the state and progress only, with no version, path or error detail (SEC-OPS-050). | ADM-032, MUS-122 |

**Stream limits apply to music from R1.** Case L below is not only a video
case: when a server-wide, per-guest or share-link stream limit refuses
music, the listener sees "Too many streams are playing on this server right
now" or the limit that applies, with what to do, never a silent failure.
The server checks the limit when stream URLs are issued and on every
stream request and answers with a typed "too many" error, and the next
track's early fetch for gapless playback counts as the same playback
(SEC-TM-068, SEC-API-031, SEC-IAM-102, ACC-075). The player's "Not
allowed" state in [player.md](player.md) shows it.

**The owner investigates (R1).**

| # | The person | Surface | The server | Features |
|---|---|---|---|---|
| 1 | Sees which sessions failed and the reason for each. | Admin > Sessions; session detail | Stores the structured reason from the core's decision engine per session, and exposes it in the API. The session list shows the person, device and delivery method, and the title only if that person chose to show it; each look is recorded in that person's own security log (SEC-PRV-025, SEC-IAM-077). | ADM-100, INT-134 |
| 2 | Inspects the file, once the listener has said which one. | Admin > Diagnostics > Inspect a file; Item menu > File info | Runs the core's parsers in the worker through the inspect API and shows raw tags, structure, and any error with its location, all rendered as text (SEC-MED-018, SEC-CLI-001). | ADM-125, LIB-195, MUS-114 |
| 3 | Asks the listener for a report. | Settings > Help > Diagnostics, on the listener's device | On the device, builds the report, redacted by default and shown in full before the listener chooses to send it. Adds a server log excerpt on request; diagnostic logs never hold titles, paths, search terms or secrets at the default level (SEC-PRV-043, SEC-OPS-029). | CLI-033 |
| 4 | Runs the doctor and builds a bundle when asking the project for help. | Admin > Diagnostics; CLI | Runs the check registry, including `doctor --security` (SEC-OPS-061). Builds the bundle from an allowlist of fields, with no database, backups or secrets, and with paths, titles, user names and addresses replaced by pseudonyms; the admin sees all of it before downloading (SEC-PRV-046, SEC-OPS-030). Includes local crash records. | ADM-123, ADM-124, ADM-130 |

**Video cases in R2.**

| Case | What the viewer sees | Surface | The server | Features |
|---|---|---|---|---|
| I. The device cannot open the container | The film plays anyway, and the info overlay says it is being remuxed and why. | Player info overlay | The core's decision engine picks a remux, and the remuxer repackages without touching picture or sound. It runs in a worker process streaming over a pipe, under a step budget, a memory cap and a watchdog, not inside the server (SEC-MED-081; security README decision 9, owner to confirm). | VID-002, VID-003, VID-168, VID-169 |
| J. The codec is unsupported and transcoding is off, unavailable, or not allowed for this person | An error card offers another version, a download for later, or another device. | Player error card | Treats policy as an input to the decision, so the refusal comes with its reason and the alternatives. The sandbox self-test says plainly when transcoding cannot run; when the jail is missing, transcoding is off rather than unconfined (SEC-MED-024, SEC-OPS-062). | VID-010, ACC-043, VID-009, ADM-133, VID-013, CLI-084, CLI-101 |
| K. The link is too slow for the original | A short choice: the original with a bigger buffer, another version, an audio-only conversion, or a download. | Pre-play sheet | Compares the measured link with the file's peak bitrate per segment from the segment map. | VID-024, VID-014 |
| L. Too many streams (music from R1, video from R2) | A card that names the limit. | Player limit card | Counts playing leases, not open apps, and answers the stream over the limit with a typed "too many" error (SEC-TM-068, SEC-API-031). | VID-173, ACC-075 |
| M. A Dolby Vision file on a non-Dolby Vision device | The HDR10 base layer plays, and the overlay names the fallback. | Player info overlay | The remuxer handles the HEVC NAL units. | VID-040 |
| N. A stall nobody can explain | One tap sends the owner a redacted report, and the owner opens the session trace. | Player error card; Admin > Sessions > Trace | Joins the server's decision and bytes sent with the client's buffering and error reports for a short time. The trace names the title only because the viewer chose to send the report; otherwise it shows what any live-session view shows (SEC-PRV-025). | VID-174, ADM-126 |

Before play, a "plays directly here" badge warns of a remux or transcode
in advance (VID-015). Native players receive media only through a stream
callback for one item, never a path or URL, with libmpv's scripts,
playlists and external references switched off (SEC-CLI-047, SEC-CLI-048).

**Rivals today.** Plex and Jellyfin both show playback information, though
how much is unverified; mpv's statistics page and Infuse's overlay are the
best. Jellyfin's Android TV app added a media capability report in 0.19.
The research found no rival that marks an unplayable track before play.
Plexamp sends download telemetry, including titles and the account name,
with no off switch; Gunmetal's reports are read before they are sent.

**Gaps.** In R1 the only fix for case A is another client, and R1 has none
(G13). Case D's silent URL refresh is now a requirement (SEC-API-027),
which settles G10.

**Depends on.** Open decision 9 (the audio packager); the decision engine
in the core (ADR 1, decision 2); the remuxer and sandbox for the R2 cases,
with the remuxer in a worker (security README decision 9).

---

## F13. Upgrading the server and rolling back

**Release: R1.**

**Who and where.** The owner, at the host and in the admin section.

**Starts** with a running server and a new release available. **Ends** with
the new version serving, or the old one back, with nothing lost either way.

| # | The person | Surface | The server | Features |
|---|---|---|---|---|
| 1 | Learns that a new version exists, if the owner answered yes to the security-fix question at setup. | Admin > Updates; dashboard update card | Fetches a static signed feed daily through the egress gate with a plain request that sends no version, cookie or identifier (SEC-SUP-051), and verifies it against a trust root compiled into the binary, refusing a feed that is expired, rolled back or under-signed (SEC-OPS-019). Contacts nobody when the check is off, and the dashboard and `doctor` then say so quietly (SEC-OPS-047). After 7 days without a valid feed it says "Can't confirm you're up to date" (SEC-SUP-050). | ADM-053, ADM-028 |
| 2 | Is told plainly if the running version has a known security problem or is out of support. | Admin banner; Admin > Updates | Compares the running version with the advisory ranges in the feed on the server, so the project never learns which version runs, and alerts the owner and administrators (SEC-OPS-032, SEC-OPS-070). The version is shown only to signed-in administrators (SEC-NET-047). | ADM-054, ACC-126 |
| 3 | Reads whether this release migrates data, can be rolled back, and changes any security default. | Admin > Updates; release notes | Shows the release notes' upgrade block, which every release must carry (SEC-OPS-052). | ADM-060 |
| 4 | Replaces the binary, or pulls the new container tag, and restarts. | Host | Nothing until it starts. The server never replaces its own binary or runs code it downloaded (SEC-OPS-046). | ADM-001, ADM-003 |
| 5 | Sees the startup page while the server migrates. | Startup page | Takes a snapshot and checks its integrity before any migration, runs migrations in transactions, and on failure stays in maintenance mode on the untouched data (SEC-OPS-048). No migration makes the server less strict, and explicit owner choices are kept (SEC-OPS-049). Serves only after the migrations pass. The page shows progress, with no version or error detail (SEC-OPS-050). Upgrades from any older version in one step. | ADM-056, ADM-057, ADM-058, ADM-032 |
| 6 | Is back in, with no full rescan. | Admin > Activity, for example "Re-reading MP4 files after a parser update" | Rereads only files whose parser version changed and keeps derived data such as loudness. | LIB-025, ADM-141 |
| 7 | If the new version misbehaves, stops it and restores the pre-upgrade snapshot, then starts the previous binary. | Console; Startup page ("restore the snapshot first") | An older binary may discard a newer cache and rebuild it from the files, but it refuses to open durable state written in a newer, incompatible format and prints how to restore the pre-upgrade snapshot; it never discards identity, audit or security configuration to start (SEC-OPS-051). | ADM-059, ADM-077, ADM-056 |

**When it goes wrong.**

- A migration fails. It ran in a transaction after a checked snapshot, the
  old data is untouched, the server stays in maintenance mode, and the
  console and log say what failed (ADM-057, SEC-OPS-048).
- The power goes during a write. The user log recovers from a torn write
  (ADM-078).
- The main client will not load. The emergency page still offers status,
  recent log lines, a backup and a restart, after the owner or an
  administrator signs in with a passkey (ADM-113, SEC-IAM-041). Downloading
  the backup there is an owner step-up action (SEC-OPS-045).
- The new version changed a security default. Existing installs keep their
  stricter setting, and the release notes say what changed (SEC-OPS-049,
  SEC-OPS-052).
- A TV app cannot be updated yet. Old clients keep working (CLI-032),
  unless the signed advisory feed lists that client version as insecure, in
  which case the server refuses it with "update required" (SEC-CLI-065).

**Rivals today.** Jellyfin is good at the safety net: it copies the old
database before migrating, 10.11 added a startup page that shows migration
progress, and 12.0 added a mode that runs migrations and exits. Its 10.11
migrations still failed for many people, and it cannot skip versions (12.0
needs 10.10.7 or any 10.11 first). Plex keeps only its routine three-day
database backup, but in practice it is the easiest to roll back, because
reinstalling the older build has fixed past regressions. Plex can also
email owners of a vulnerable version, which Gunmetal cannot do without a
central account; the advisory banner in step 2 is the substitute.

**Other releases.** R2: release channels with free release candidates
(ADM-055), withdrawn releases flagged (ADM-061), and a plugin check before
upgrading (ADM-063). Later: opt-in automatic updates with a health-check
rollback (ADM-064), which can never mean the server rewriting its own
binary (SEC-OPS-046).

**Depends on.** ADR 3 (a forward-compatible user log); open decision 25
(release signing keys); security README decision 4 (the update question).

---

## F14. Rebuilding a dead server from a backup

**Release: R1.**

**Who and where.** The owner, at a fresh install on new hardware, a new
operating system or a container.

**Starts** with the old machine dead or retired, a backup file to hand, and
the recovery kit printed at setup. **Ends** with the same accounts, history,
playlists and settings on the new machine, every library root pointed at
its new path, fresh keys, and every restored device waiting for its owner
to confirm it.

| # | The person | Surface | The server | Features |
|---|---|---|---|---|
| 1 | Before anything breaks, has daily backups that are verified, explained, encrypted and signed. | Admin > Backups; backup detail; dashboard health card | Backs up a consistent snapshot of the user log, the identity store, settings and the audit log with its latest signed checkpoint, never by copying a live database file (SEC-OPS-041, SEC-OPS-024). Encrypts each backup to the server's backup key and the owner's recovery key, and signs it (SEC-OPS-042, SEC-OPS-043, SEC-PRV-039). Never writes backups under a web-served path or into a media root, keeps them 14 days by default, verifies each after writing and alerts on failure (SEC-OPS-045, SEC-PRV-041). Downloading one is an owner step-up action that is audited and alerted (SEC-OPS-045). | ADM-065, ADM-066, ADM-072, ADM-069, ADM-116 |
| 2 | Installs Gunmetal on the new machine, starts it, and enters the setup code. | Console; Welcome > Setup code | As in F01, steps 1 to 5. A restore needs the same setup code as a claim, so nobody else on the network can restore their own backup onto a fresh install (SEC-OPS-008). | ADM-001, ACC-001 |
| 3 | Chooses "Restore from a backup", uploads or picks the file, and enters or scans the backup recovery key from the recovery kit. | Welcome > Restore | Checks the signature before anything else, then decrypts and parses the archive as a stream within limits on size, entry count and expansion, refusing absolute paths, `..`, links and duplicate entries (SEC-OPS-043). Opens any database inside it read-only in a jailed worker (SEC-STD-031). Shows which server made the backup and when, and the signing key's fingerprint; a backup signed by a key this install has never used needs the owner to type that fingerprint (SEC-OPS-043). Checks that the audit log extends the backup's checkpoint (SEC-OPS-024). The recovery key stays in memory only for the restore (SEC-PRV-040). | ADM-029 |
| 4 | Sees where each library root used to be and points it at its new path. | Welcome > Restore > "Where are your libraries now?" | Checks every root is reachable and offers to remap moved roots, with a dry run. Refuses a root that is a filesystem root or holds the server's own data (SEC-MED-037). | ADM-051, LIB-031 |
| 5 | Watches the restore. | Restore progress page | Restores the log and the identity store, then rebuilds the cache from the files, reusing derived data when the backup includes it. Then rotates every symmetric key and invalidates every session and stream URL (SEC-OPS-044). Every restored setting passes the live validators, and anything less strict than today's defaults, such as a trusted-proxy list, is held until the owner confirms it; the server stays in the home posture (SEC-OPS-044, SEC-OPS-038). | ADM-029, ADM-077, ADM-141 |
| 6 | Signs in, and works through the "review devices and access" alert. | Sign-in page; owner alert; Account > Sessions and devices | Uses the restored identity store, with every restored device and credential suspended until its holder or the owner confirms it. The alert lists every restored device, credential, administrator and share, and says that removals made after the backup was taken have been undone (SEC-OPS-044). Writes the restore to the audit log and alerts the owner (SEC-OPS-032). | ACC-050 |

**When it goes wrong.**

- **The server is now reached at a different address.** Every passkey is
  tied to the address it was made for (SEC-IAM-018), and setup warns that
  this choice is permanent (SEC-NET-072), so the restore shows the old
  address from the backup and recommends keeping it. When the address must
  change, the owner enrols a new passkey through host recovery
  (SEC-IAM-092), and members use their recovery codes or an administrator's
  recovery link redeemed in person, each starting the 72-hour recovery hold
  (SEC-IAM-089, SEC-IAM-091, SEC-IAM-106). When the old server is still
  running, members can instead approve the new address from a device still
  signed in to the old one, through the documented and tested migration
  (SEC-NET-072). There is no password fallback
  ([G3](#gaps-and-questions-the-flows-expose)).
- **The recovery kit is lost and the old server is gone.** The backup cannot
  be decrypted. Where the owner's passkey supports the WebAuthn PRF
  extension, the backup key is also wrapped under the passkey, so a synced
  passkey alone can restore (SEC-PRV-040). Otherwise the data is gone, which
  is why setup and the dashboard insist that the kit is saved.
- **The file was tampered with, or came from someone else's server.** The
  signature check refuses it, or the owner sees an unfamiliar key and must
  type its fingerprint to go on (SEC-OPS-043).
- **Someone on the network tries to restore their own backup onto the
  fresh install.** Without the setup code they cannot (SEC-OPS-008). This
  settles [G15](#gaps-and-questions-the-flows-expose).
- **A root cannot be found.** The restore names it and offers the remap
  (ADM-051).
- **A restore on a running server fails.** The UI restore is an owner
  step-up action (SEC-IAM-041). It takes a restore point first and shows a
  preview (ADM-070).
- **A device was revoked after the backup was taken.** It comes back
  suspended and stays so; its old session already failed when keys were
  rotated (SEC-OPS-044).
- **The owner prefers a shell.** `gunmetal restore` does the same on the
  host (ADM-071).

**Rivals today.** Immich's restore button on the welcome screen, with
checks that the library folders are readable, is the model, and Gunmetal
is at parity with it. Jellyfin restores from the command line only
(`--restore-archive`) and needs identical media paths. Plex documents a
manual move on the same operating system only, and does not officially
support moving between systems. Navidrome can remap missing files, but the
remap cannot be undone.

**Other releases.** Encrypted backups (ADM-068) move from R2 to R1: the
baseline allows no unencrypted backup, local or exported (SEC-PRV-039,
SEC-OPS-042, SEC-IAM-105). R2: off-site destinations (ADM-073), which
receive only the encrypted file; an optional cache snapshot for a faster
restore (ADM-067). Later: point-in-time recovery from the log (ADM-076).

**Depends on.** ADR 3 (the backup is the user log and the identity store);
operations decisions on the recovery key and the identity key in backups
(security README decision 25).

---

## F15. Losing a phone

**Release: R1 for sessions; revoking downloads arrives in R2.**

**Who and where.** A member on another device, or the owner acting for
them.

**Starts** with a lost or stolen phone that is signed in. **Ends** with the
phone unable to reach the server, and in R2 its downloads revoked on next
contact.

| # | The person | Surface | The server | Features |
|---|---|---|---|---|
| 1 | Signs in elsewhere and opens their sessions and devices. | Account > Sessions and devices | Lists every session and device (and from R2 every API key and app password) with its name, device class and key level, network type, rough location and last use (SEC-IAM-042). | ACC-068 |
| 2 | Removes the lost phone, or signs out of everything except this device. | Account > Sessions and devices > Remove; "Sign out everywhere else" | Revokes the device so its next request fails and its open streams and sockets close within 5 seconds; its stream URLs stop working at the same moment (SEC-IAM-043, SEC-API-028). "Everywhere else" ends every session but the current one (SEC-IAM-042). | ACC-069, ACC-070, ACC-122 |
| 3 | If the phone held a passkey of its own (one that was not synced), removes that passkey, confirming with another passkey. | Account > Sign-in methods; step-up prompt | Removing a credential needs a passkey check in the previous 5 minutes, is refused for the last credential, tells the account's other devices, and offers to end all other sessions (SEC-IAM-023, SEC-IAM-024, SEC-IAM-042). A synced passkey lives in the phone maker's account, so the screen also says to secure or remove the phone there. | ACC-065, ACC-055, ACC-056 |
| 4 | Checks their security events. | Account > Security events | Shows every audit entry about the person's own account, devices and credentials, with full addresses (SEC-IAM-097, SEC-OPS-027). | ACC-078, ADM-110 |
| 5 | The owner sees the alerts and, if needed, ends the person's sessions or pauses the account. | Owner alert; Admin > Security log; Admin > Users | Disabling ends every session at once (SEC-IAM-103). Administrators can end any session of a non-owner account; only the owner can end the owner's (SEC-IAM-044). Each alert about a device or credential offers "This wasn't me", which revokes it, ends its sessions and makes its stream URLs fail (SEC-OPS-033). | ADM-110, ACC-008 |

**When it goes wrong.**

- **The lost phone was the only device.** The person climbs the recovery
  ladder: a synced passkey on a new phone; a recovery code; their identity
  provider; an administrator's recovery link redeemed in person. A code or a
  link starts the 72-hour recovery hold, which the person's other devices
  can end with one tap (SEC-IAM-089 to SEC-IAM-091, SEC-IAM-106; ACC-064).
  The owner uses host recovery (SEC-IAM-092, ACC-004).
- **The thief tries to use the phone before the person acts.** A browser
  session never authorises administration, and every administrator or
  account-security action needs a fresh passkey check the thief cannot pass
  without the phone's unlock (SEC-IAM-041, SEC-IAM-023).
- **The thief adds a new device from the phone.** Every new device is
  announced on the account's other devices, with "This wasn't me"
  (SEC-IAM-098, SEC-OPS-033).
- **In R2 the phone is offline.** Its downloads keep playing until the grant
  expires, because revocation takes effect only on next contact, when the
  downloads are deleted (CLI-095, CLI-096, SEC-IAM-054). The docs must say
  this plainly (clients map, risks).

**Rivals today.** Plex offers device revocation and "sign out of all
sessions" at plex.tv, recommended after its 2025 breach; that is good.
Jellyfin revokes devices and needed a 12.1 fix to end their open sessions.
Emby is best at controlling synced media. Navidrome's shared stream URLs
kept working after deletion until September 2026, which is the failure
ACC-122 is designed against.

**Other releases.** New-device alerts (ACC-071) move from R2 to R1
(SEC-IAM-098). R2: revoke a device's downloads (ACC-045, CLI-096); device
keys that can be revoked one by one (ACC-051), with open iroh connections
closed within 5 seconds (SEC-NET-034); content-free push alerts that make
the app fetch the details over its own connection (SEC-OPS-036). Later:
spotting shared passwords (ACC-077), which has nothing to spot while there
are no passwords; device and stream limits per account already apply from
R1 (SEC-IAM-102).

**Depends on.** ADR 3 (identity store); clients map open decision 4 (grant
lifetime).

---

## F16. Keeping your own record

**Release: R1.**

**Who and where.** A listener in the web client.

**Starts** with a history that is the person's own. **Ends** with the
person having played something unrecorded, removed a play, dismissed an
item, checked what admins can see and taken a full export.

| # | The person | Surface | The server | Features |
|---|---|---|---|---|
| 1 | Turns on a private session from the player's menu, at most two taps away, then plays something. | Player > Private session; private indicator | Plays through the "not recorded" path: no history, no recommendation signals, and from R2 no scrobbles. Any live-session view an administrator opens omits the title (SEC-PRV-024). From R2, nothing is handed to the operating system's recents or home-screen rows either (SEC-PRV-058). | ACC-117, MUS-185, DIS-053 |
| 2 | Turns it off, and plays are recorded again. | Private indicator | Returns to normal recording. A private session also ends by itself after a stretch without playback that the person picks, 6 hours by default, so nobody stays private by accident for weeks. | ACC-117 |
| 3 | Opens history by date and finds last March. | History page | Nothing at read time: history is in the profile's log on the device. | MUS-183, DIS-050 |
| 4 | Removes a play, or a range of dates, or everything. | History row menu; History > Delete | Writes a removal event, and counts, statistics and recommendations follow. Within 24 hours the play is gone from the database, the history log, derived tables, indexes and caches, and the deletion is re-applied if an older backup is ever restored (SEC-PRV-049, SEC-PRV-050). Devices learn of it through tombstones that carry only IDs (SEC-PRV-052). | MUS-184, ACC-118, DIS-052 |
| 5 | Dismisses an album from Continue listening, then undoes it. | Card context menu; undo toast; Hidden page | Writes a dismiss event, then its reversal. | DIS-022, DIS-023 |
| 6 | Opens "What admins can see about me" and reads, in plain words, that admins can see that they are streaming, on which device and at what quality, and the title only if they choose to show it; that admins cannot see their history, ratings or private playlists in Gunmetal; and that whoever controls the computer could read its files directly. | Account > What admins can see; Account > Privacy (show titles to admins, off by default) | Generates the page from the same policy table the server enforces, so it cannot drift from the truth (SEC-IAM-104, SEC-PRV-027). The title switch starts off (SEC-PRV-023, SEC-PRV-025). | ACC-115, ACC-116 |
| 7 | Reads their own security events, including any time an administrator looked at their live sessions. | Account > Security events | Shows the audit entries about the person's account, devices and credentials, with full addresses, and every admin read of their live sessions or data (SEC-IAM-077, SEC-IAM-097, SEC-OPS-027). | ACC-078 |
| 8 | Exports everything they have told the server, touching the passkey first. | Account > Your data; step-up prompt | Needs a passkey check in the previous 5 minutes and limits how often exports run (SEC-PRV-048). Builds an export from the user log in a documented, versioned format: history, loves, ratings, playlists, hides, layouts and rules, with no other person's data and no secrets (SEC-PRV-047). The download works once, only in this session, and expires within an hour (SEC-PRV-048). | ACC-010, DIS-058, MUS-188, INT-151, LAT-007 |

**When it goes wrong.**

- **The person forgets the private session is on.** It is off by default,
  two taps from the player, clearly shown while on (open decision 14), and
  it ends after the idle period the person chose.
- **The person is in a recovery hold** after using a recovery code or an
  administrator's link. Export is refused until the hold ends, and after an
  administrator's link the history stays hidden until then (SEC-IAM-106).
- **The person wants to leave.** Deleting the account needs a fresh passkey
  check, disables it at once, keeps it restorable for 7 days, offers an
  export first, and says by which date it will also have left the retained
  backups (SEC-PRV-048, SEC-PRV-051, SEC-IAM-103).

**Rivals today.** These are streaming-app habits that media servers lack.
Removing a play has 5,458 votes on Spotify's board and browsing history by
date 2,035; Jellyfin's dismiss-with-undo request has 1,725. No media server
the research checked documents private sessions (unverified).

**Other releases.** The page showing what admins can see (ACC-115) moves
from R2 to R1, as step 6 (SEC-IAM-104). R2: scrobble filters that honour
private listening (INT-105), with scrobbling off until each person links
their own account and only plays after the link time sent (SEC-PRV-033,
SEC-PRV-035); charts and a year in review (MUS-186, MUS-187); hide and
snooze (DIS-054).

**Depends on.** ADR 3; open decisions 13 and 14; security README decision
5 (what admins see).

---

## F17. Watching a film on the TV

**Release: R2.**

**Who and where.** A viewer on the Android TV app paired in F04.

**Starts** with a film library and a signed-in TV. **Ends** with the film
played as the original, resumed later on another device.

| # | The person | Surface | The server | Features |
|---|---|---|---|---|
| 1 | Picks a row from Continue Watching or "Next to watch" (the feature map's Next Up), or browses Films with the left rail. | TV Home; TV rail | Builds the rows from the watch log and the synced library. | DIS-024, DIS-025, CLI-035, CLI-036 |
| 2 | Opens a film and sees a badge saying it plays directly here, its versions and its tracks. | Title detail page; pre-play sheet | Nothing: on the device, the core runs the playback decision against the synced stream index and the TV's capability report. | VID-015, VID-013, VID-014, VID-053, CLI-047, VID-002 |
| 3 | Presses play, and the picture appears at the film's own frame rate. | Player | Issues a session-bound capability URL and serves byte ranges of the original (SEC-API-026, SEC-API-028). The TV's libmpv player decodes it, receiving bytes only through a stream callback for this one item, with scripts, playlists and external references off (SEC-CLI-047, SEC-CLI-048). A container the core did not parse at scan time is remuxed or transcoded on the server instead of handed to libmpv, unless an admin allows direct play for that library (SEC-CLI-049). | VID-001, VID-011, VID-176, CLI-046, VID-012 |
| 4 | Sees styled subtitles in the preferred language, drawn with the player's bundled fonts, or exactly as authored where the library has opted in to embedded fonts. | Player subtitle quick menu | Serves the subtitle stream, parsed into a cue model within limits (SEC-MED-052). Serves no font attached to the file by default; a library may opt in, and each font is then parsed and rewritten by a memory-safe parser in the worker, never passed through (SEC-MED-054; security README decision 12, owner to confirm). | VID-069, VID-068, VID-071, VID-051, VID-052 |
| 5 | Skips the intro. | Player skip button | Supplies skip markers from chapter titles or analysis, stored as logged data. | VID-110, VID-111, VID-112, VID-114 |
| 6 | Checks how it is playing. | Player info overlay | Supplies the decision trace, redacted by role. | VID-168, VID-169 |
| 7 | Stops halfway, then resumes later on the phone. | Resume prompt | Records the position as an event, so resume works on any device, offline included. | VID-118, VID-119 |
| 8 | At the credits, sees the next item or a post-play screen. | Post-play screen; countdown card | Counts the film as watched when its credits start. | VID-122, VID-123 |

**When it goes wrong.** F12 covers cases I to N.

**Rivals today.** Plex, Jellyfin and Emby all direct play on capable
clients and all resume and suggest the next episode; mpv-based players,
Kodi and Infuse already play nearly any file as it is. Gunmetal is at
parity on native direct play. Its edges are the badge before play,
subtitles rendered on the device that never force a transcode (Plex's
Android TV player cannot render ASS), and a plain reason whenever anything
changes. Jellyfin's Android TV app has had HDR and audio passthrough issues
open for years (127 and 50 comments).

**Other releases.** Later: true Dolby Vision output on Dolby Vision
televisions (VID-184, open decision 19); hardware transcoding (VID-006),
off by default and given only render nodes when switched on (SEC-MED-072);
watching together (VID-153, open decision 18); Apple TV (CLI-007).

**Depends on.** The remuxer, sandbox and segment map (R2 infrastructure);
open decisions 19 and 20; security README decisions 9 and 12.

---

## F18. Setting up a child's profile

**Release: R2.**

**Who and where.** A parent in the web client, then a child on the
living-room TV.

**Starts** with a household of adults. **Ends** with a child profile that
can see only what it is allowed, on every device, with nothing leaking
through search, artwork or screensavers.

| # | The person | Surface | The server | Features |
|---|---|---|---|---|
| 1 | Creates a household and adds a child with a name and picture, and no email or credential. Names the child's guardians. | Admin > Household > Add a child | Creates a managed profile linked to a policy, with the designated guardians. Needs the household-profile capability, which parents can be granted (identity design, section 3). A managed profile can never link outside accounts, enable outbound sharing or create share links (SEC-PRV-029). | ACC-016, ACC-018, ACC-011 |
| 2 | Picks a preset: younger child, older child or teen. | Add a child > preset chooser | Copies a built-in policy template. | ACC-023 |
| 3 | Adjusts it: no explicit music, a film and TV rating ceiling for their country, only items labelled for the children, and listening hours. | Child-profile settings, with the rule editor | Stores the rules in the shared rule language, the ceiling per country system, the explicit flag from tags, and the schedule. Enforces all of it inside the one visibility predicate, so it applies to every list, search, recommendation, artwork, lyric, subtitle, stream, share and sync payload alike (SEC-IAM-064). | ACC-024, ACC-026, ACC-027, ACC-028, ACC-025, ACC-032 |
| 4 | Locks the adult profiles with PINs; adding an adult profile to a TV preselects "Add a PIN". | Profile settings; PIN pad | Checks PINs only on the server, against Argon2id hashes; failures are delayed per TV and profile, never permanently (SEC-IAM-062). A PIN only gates switching profiles (SEC-IAM-063), and an adult profile's history stays hidden on the TV until its PIN or its owner's phone unlocks it (SEC-IAM-110). | ACC-020 |
| 5 | The child picks their profile on the TV. | TV profile picker; Kids home | Builds the child's sync payload with blocked items removed before it reaches the device, and checks stream URLs again (SEC-CLI-020, SEC-API-028). A TV locked to the child's profile leaves it only with another profile's PIN or an approval from a guardian's phone (SEC-IAM-066). | ACC-019, DIS-144, ACC-030, DIS-152 |
| 6 | Outside the allowed hours, the child sees a clear message. | Blocked-time message | Enforces the schedule when URLs are issued and in offline grants. | ACC-032 |
| 7 | A parent allows one album without loosening the profile. | Item menu > Allow for a child | Adds the item to the policy's exception list. | ACC-029 |
| 8 | A guardian sees what the child played, and the child's own screens say, in words for their age, that guardians can see it. | Household > child > History | Lets the child's designated guardians, and nobody else, read the child's history (SEC-PRV-029, SEC-PRV-022, SEC-IAM-097). Other household adults do not see it. | ACC-034 |

**When it goes wrong.**

- A restricted item reaches the device through a side path. ACC-030 makes
  "every path obeys the restrictions" an R1 rule with a cross-profile test on
  every route, so the R2 rows only add rules to an enforcement point that
  already exists (SEC-IAM-064, SEC-IAM-070).
- The child keeps guessing an adult's PIN. Each failure adds a delay for
  that TV and profile, the adult can still get in from their own phone, and
  after 10 failures the adult gets one alert in the daily summary
  (SEC-IAM-062).
- The child's TV shows an adult's history on its home screen. It cannot:
  restricted and PIN-protected profiles are never published to the TV's
  home-screen rows (SEC-CLI-061), and children never receive security
  alerts, which go to the owner.

**Rivals today.** Plex is good here: managed users, rating presets and
PIN-protected profiles, though custom rules need Plex Pass and a request to
hide PIN entry has 111 votes. Jellyfin's rating limits are free and support
sub-ratings, but it has no households (#493, 202 votes) and no managed
profiles. Plex removes folder view while restrictions are on, and Jellyfin
filters home artwork but declined a screensaver filter.

**Other releases.** R1 has per-person library access (ACC-037, MUS-027)
and reads the explicit flag (MUS-047), so a separate children's library is
possible, but there are no managed profiles, presets or PINs (security
README decision 17). Later: a daily time allowance (ACC-033), asking a
parent (ACC-035), guest mode (ACC-022). Encrypted per-profile stores on
shared devices (ACC-132) move from Later to R2, because the baseline
requires them for PIN-protected profiles on household devices
(SEC-IAM-065). R3: live TV channel limits (ACC-036).

**Depends on.** ADR 3; the rule language (DIS-119); security README
decisions 5 and 17.

---

## F19. Connecting an existing music app

**Release: R2.**

**Who and where.** The owner enables the adapter; a listener connects
Symfonium or another Subsonic app. This is the interim answer for iPhone
owners, whose native app is Later (CLI-130).

**Starts** with the adapter off, as it is by default. **Ends** with the
third-party app browsing and playing, its plays counted in the same
history, and a key the listener can revoke.

| # | The person | Surface | The server | Features |
|---|---|---|---|---|
| 1 | The owner turns on the OpenSubsonic adapter, touching the passkey to confirm. | Admin > Compatibility; step-up prompt | Enabling an adapter is an owner step-up action (SEC-EXT-051, SEC-IAM-041). The adapter listens on its own port, never reads or sets cookies, and changes nothing about the native API (SEC-EXT-052, SEC-EXT-053). Registers only the reviewed allowlist of adapter routes, with no administrative, user-management, file-browsing or share-management endpoint, behind the same policy layer, limiter and cross-user tests as the native API (SEC-EXT-054, SEC-EXT-055). Nobody is enrolled automatically. | INT-086, ACC-130, INT-094 |
| 2 | The listener creates a key for the app. | Account > Apps and tokens ("Connect a music app", with a QR code) | Mints a random app key of at least 128 bits, shown once and stored only as a keyed hash, scoped at most to browsing, playing, the listener's own playlists, ratings and favourites, and scrobbling (SEC-EXT-056, SEC-EXT-058). It expires and appears with the listener's devices (SEC-IAM-083, SEC-EXT-014). The listener's passkey never reaches the app (SEC-EXT-057). | INT-087, ACC-129, INT-024 |
| 3 | Enters the server's HTTPS address and the key in the app. | Third-party app | Refuses every credential over plain HTTP from any peer but loopback; the setup screen gives the app only the HTTPS address (SEC-EXT-066, SEC-NET-001). Validates the key, with one generic error whatever is wrong and the same limiter as interactive sign-in (SEC-EXT-064, SEC-EXT-015). Serves browsing, search, streaming, cover art, playlists, stars, ratings, scrobbles and the play queue from the native music model through an ID translation layer that exposes no paths, host names, version or other people's names (SEC-EXT-059, SEC-EXT-060). "Now playing" lists only the listener's own sessions (SEC-PRV-032). Advertises only the extensions it implements. | INT-086, INT-089, INT-090 |
| 4 | Plays music. | Third-party app; History (shows which app) | Writes adapter plays to the user log with the device and app, honouring a private session (SEC-EXT-056). | INT-093 |
| 5 | Sees, and later revokes, the connected app. | Account > Apps and tokens; Admin > Compatibility | Lists connected apps with their last use and rough address, and revokes keys one by one or all at once, from the next request (SEC-EXT-014). Disabling the adapter ends all its sessions at once (SEC-EXT-051). | INT-096, INT-021 |

**When it goes wrong.** The app supports only the old password-derived
sign-in. Which Subsonic apps support API keys is unverified (open decision
4). By default the adapter answers that kind of sign-in with an error; a
listener can mark one app key as legacy so that key alone may be sent as a
password, and token-and-salt sign-in stays refused unless the owner
approves a written exception (SEC-EXT-067, SEC-EXT-069; security README
decision 8, owner to confirm).

**Rivals today.** Navidrome, gonic, Ampache and LMS already serve Subsonic
apps well, and the apps themselves are mature; borrowing that ecosystem is
the point. Jellyfin has no Subsonic API, and Plex has none.

**Other releases.** R2 also brings the Jellyfin adapter's music subset for
Finamp, Jellify and Feishin (INT-098), and Music Assistant through
OpenSubsonic (INT-097). Later: OpenSubsonic podcasts and bookmarks
(INT-092).

**Depends on.** Open decision 4 (adapter timing and API keys); security
README decision 8.

---

## F20. Adding an IPTV playlist and watching a channel

**Release: R3.**

**Who and where.** The owner in the admin section, then a viewer on the TV
app.

**Starts** with Live TV switched off, as it is for households without TV.
**Ends** with filtered channels matched to a guide, playing as broadcast on
the TV, and a recording scheduled.

| # | The person | Surface | The server | Features |
|---|---|---|---|---|
| 1 | The owner switches on the Live TV module. | Admin > Modules toggle | Registers live TV routes and jobs only when the module is on. | LIV-001 |
| 2 | Starts the guided set-up and chooses an IPTV playlist. | Admin > Live TV > Set-up wizard | Probes sources and computes guide coverage, saving everything in one transaction at the end. | LIV-002 |
| 3 | Adds an M3U by URL, with headers if the provider needs them, touching the passkey to confirm. | Admin > Live TV > Sources > Add playlist; Source > Advanced; step-up prompt | Each source URL is an egress grant, which is an owner step-up action (SEC-EXT-075, SEC-IAM-041). Fetches through the egress gate with connect-time address checks, refusing loopback, link-local, the server's own and cloud metadata addresses unless the owner allows one named LAN tuner (SEC-NET-067). Keeps provider credentials encrypted as integration secrets where clients never see them (SEC-EXT-050). | LIV-003, LIV-007, LIV-015, LIV-014 |
| 4 | Tests before saving: channel count, a test tune and any HTTP error. | Add-source wizard: test results panel | Probes with the demuxer and writes nothing until confirmed. | LIV-016 |
| 5 | Filters a 20,000-line playlist down to the groups wanted. | Source > Filters, with a live match count | Applies ordered include and exclude rules, with a preview. | LIV-008, LIV-009 |
| 6 | Adds an XMLTV guide and reviews the automatic matches. | Admin > Live TV > Guide sources; Admin > Channels > Mapping | Parses the guide and matches channels in the core, with a confidence for each. | LIV-035, LIV-051, LIV-050 |
| 7 | Decides who may watch and who may record. | Admin > Users > Live TV | Adds live TV capabilities to each policy. | ACC-048 |
| 8 | The viewer opens the guide on the TV. | Guide grid (TV) | Syncs a compact guide to the device, so it browses instantly even when the server is unreachable. | LIV-064, LIV-065 |
| 9 | Picks a channel, and it plays as broadcast. | Live player | Serves the transport stream from a fan-out buffer through the server; clients never fetch a provider's URL or logo themselves, and channel names and programme text are shown as text (SEC-CLI-067). The native client decodes it, so the server does not transcode. | LIV-078, LIV-080 |
| 10 | Records a programme from the guide, free. | Guide and details "Record" | Creates a recording object for the scheduler and recorder, which writes only to a dedicated recordings folder with a quota, apart from the read-only media and the server's state (SEC-OPS-063). | LIV-102 |

**When it goes wrong.** Watching in a browser costs the server CPU, because
browsers need a remux or a sandboxed conversion, and the docs must say so
(LIV-079). When every tuner is busy, the viewer sees a sheet of choices
(LIV-087). What someone else is watching is their activity, so the sheet
names a person or a channel only for people who have chosen to share what
they are playing with the household; otherwise it says only that the tuner
is busy (SEC-PRV-022; owner to confirm). The earlier wording, "sees who
holds each one", contradicted that rule. A
playlist entry that points at a host outside the source's grant is not
fetched unless separately approved (SEC-EXT-075). A guide file with a
DOCTYPE is rejected (SEC-MED-056).

**Rivals today.** Jellyfin and Emby take M3U playlists directly, and
Channels DVR advertises channel changes under a second. Plex needs an
HDHomeRun emulator for M3U (119 votes, open since 2018), and its own
documentation says Live TV needs a server that can transcode. Jellyfin had
a server-side request forgery and file-read hole in its M3U tuner, fixed in
10.11.7 in April 2026, which is why step 3 guards every fetch. Dispatcharr's
ordered regex filters are the model for step 5.

**Depends on.** Open decision 23 (do not start R3 until the remuxer and
sandbox are stable).

---

## Journeys not designed here

These journeys come up in the research but are deliberately not walked
through, because the map or the security baseline places them in Later or
No.

| Journey | Release | Why | Features |
|---|---|---|---|
| Watching together with a guest link | Later | The second most-voted Plex suggestion (2,878 votes). The guest capability ships in R2 as its base, and open decision 18 recommends watch together as the first feature after R2. | VID-153, VID-154, ACC-093, ACC-135 |
| Listening together on one queue | Later | It shares its design with watching together, which comes first. | MUS-202, ACC-094 |
| The same music in several rooms, in step | Later | It needs clock sync and group sessions. | CLI-105, MUS-201 |
| Saving music for offline inside a browser | Later | No rival does it, and browser storage can be evicted. | MUS-217 |
| Using an iPhone app, CarPlay, Apple TV or AirPlay | Later; R2 if the App Store licence decision allows | The blocker is the App Store terms and the AGPL with no contributor agreement, not engineering (open decision 3). | CLI-005, CLI-007, CLI-117, CLI-109 |
| Remote access from a browser with no proxy | R2 (moved from Later) | Not walked through here yet. The baseline places the browser edge in R2 (security README decision 3), and the feature map now does too; a flow should be added once the edge design exists. Every byte through the edge costs relay bandwidth, which is affordable for music, not for video. | ACC-102 |
| Removing an item from the library | Later | The item is hidden and trashed with undo and audit; the file stays on disk, and the owner deletes it on the host. The server never writes to a media root (SEC-TM-042). | ADM-139 |
| Signing in with Google or Apple | No | It needs a central account. | ACC-067 |
| Skipping sign-in because you are at home | No | Header spoofing let remote attackers look local in Emby's 2023 compromise. The baseline forbids it outright (SEC-HIS-004, SEC-IAM-013). | ACC-066 |
| Opening a router port automatically | No | Open ports were the risk behind the 2023 Emby compromise, and iroh needs none. The baseline forbids it outright (SEC-NET-030, SEC-OPS-039). | ACC-106 |
| Signing in with a password, or a password plus an authenticator-app code | No | The baseline removes every password path (SEC-IAM-025); browsers without passkeys sign in by approval from a signed-in device (SEC-IAM-108). Owner to confirm (security README decision 1). | ACC-052, ACC-053 |
| An admin viewing or exporting another adult's play history | No | Admins see who is playing and totals, never what someone listened to (SEC-PRV-025). Owner to confirm (security README decision 5). | ADM-106 as first written; it now offers totals only |
| An admin signing in as another person | No | No impersonation, ever (SEC-PRV-026). | None |
| Using Plex's apps against Gunmetal | No | Plex's apps sign in through plex.tv, so emulating Plex would mean depending on Plex's cloud. | INT-101 |
| Fixing tags in the app and writing them to the files | No | Media folders stay read-only. | MUS-048, LIB-133 |
| Requesting new titles inside Gunmetal | No | Seerr already does it; Gunmetal links to it in R2. | INT-131, INT-126 |
| Asking a voice assistant run by the project | No | It needs a vendor cloud. Casting to Nest and Google speakers in R2 (CLI-106) is the partial answer. | CLI-131 |

## Surfaces the flows pass through

This is the screen list the flows imply, grouped by where it lives, with
the release that first needs it. Names follow the map's UI surfaces column.

| Surface | First needed | Flows |
|---|---|---|
| **Host and recovery** | | |
| Console and log message (claim link, QR code and setup code, failed code attempts, the "claimed by" line, startup errors) | R1 | F01, F14 |
| Command line: `gunmetal claim-code`, `doctor`, `admin recover`, `restore`, `rebuild` | R1 | F01, F03, F12, F13, F14 |
| Startup page (state and progress only) | R1 | F01, F12, F13 |
| Plain-HTTP redirect and help page; home-posture help page | R1 | F01, F03, F05, F10 |
| Emergency page, after an admin passkey sign-in | R1 | F13 |
| **First run** | | |
| Welcome > Language; Welcome > Setup code; the new-or-restore choice | R1 | F01, F14 |
| Welcome > Create owner, with the "Where will people reach this server?" panel, Welcome > HTTPS and Welcome > Proxy | R1 | F01 |
| Welcome > Recovery kit | R1 | F01, F14 |
| Welcome > Privacy, with the required security-fix question | R1 | F01 |
| Welcome > Import | R1 | F01, F11 |
| Welcome > Libraries (folder picker with live checks) | R1 | F01, F02 |
| Welcome > Restore, with signature check, recovery key, root remapping and a progress page | R1 | F14 |
| Welcome > Done | R1 | F01 |
| **Sign-in and account** | | |
| Sign-in page (generic, with "Use another device" and "Is this your own device?") | R1 | F03, F10, F14 |
| Browser pairing: the code on the new browser, and the approval sheet on a signed-in device | R1 | F03, F04 |
| Invite landing, with the privacy notice and the waiting-for-confirmation screen | R1 | F10 |
| Shared-browser banner; Settings > About this connection | R1 | F03 |
| Account > Sign-in methods; Account > Recovery | R1 | F01, F03, F10, F15 |
| Account > Sessions and devices | R1 | F03, F04, F15 |
| Account > Security events | R1 | F15, F16 |
| Account > What admins can see; Account > Privacy | R1 | F10, F16 |
| Step-up prompt (passkey check for sensitive and host-equivalent actions) | R1 | F01, F02, F10, F14, F15, F16, F19, F20 |
| Notice centre with security alerts and "This wasn't me" | R1 | F03, F04, F14, F15 |
| Account > Your data | R1 | F16 |
| Account > Apps and tokens | R2 (security README decision 8) | F19 |
| **Listening and browsing** | | |
| Home, including the scanning empty state, shortcuts and Continue listening | R1 | F02, F03, F05, F06, F16 |
| Library views with filter sheet, sort, view toggle and folder view | R1 | F02, F06, F09 |
| Artist page; album page | R1 | F02, F05, F09 |
| Playlist page; sidebar | R1 | F06 |
| Add-to-playlist sheet | R1 | F05, F06 |
| Rule editor sheet | R1 | F06; download rules in F07 and restrictions in F18 from R2 |
| Context menu; selection bar | R1 | F05, F06, F09 |
| Share sheet and public share page, for music | R1 | F06, F10 |
| Search screen and global search field | R1 | F09 |
| Now-playing bar; full-screen player; queue panel or sheet; lyrics view | R1 | F05, F08 |
| Track info sheet | R1 | F05, F12 |
| "Continue on this device" prompt | R1 | F03, F05, F08 |
| History page; Hidden page; undo toast | R1 | F11, F16 |
| Quiet status banner; dimmed and greyed-out items; client banner | R1 | F12 |
| Settings > Storage; Settings > Help > Diagnostics; install prompt | R1 | F03, F07, F12 |
| **Admin** | | |
| Admin > Dashboard (scan card, health summary, now playing, update card) | R1 | F01, F02, F10, F13 |
| Admin > Libraries and library settings | R1 | F02, F14 |
| Library health (problems, tag problems, missing files, sidecar problems) | R1 | F02, F12 |
| Admin > Review queue | R1 | F02, F06, F11 |
| Admin > Activity; Admin > Tasks | R1 | F02, F11, F13, F15 |
| Admin > Users; Admin > Invitations (with pending confirmations) | R1 | F02, F10, F15 |
| Admin > Network (trusted proxies, posture, exposure status) | R1 | F01, F10 |
| Admin > Sessions (no titles unless the person chose to show them) | R1 | F10, F12 |
| Admin > Diagnostics (doctor, bundle, inspect a file) | R1 | F12 |
| Admin > Backups | R1 | F14 |
| Admin > Updates; admin banner | R1 | F13 |
| Admin > Migration | R1 | F11 |
| Admin > Alerts | R1 | F02, F12, F14 |
| Admin > Security log | R1 | F15 |
| **R2 additions** | | |
| TV sign-in screen; phone approval sheet with the typed and matching code | R2 | F04 |
| Profile picker; PIN pad; Kids home | R2 | F04, F18 |
| TV Home and rail | R2 | F04, F17 |
| Downloads screen and rules; download toggles; Settings > Downloads | R2 | F07, F15 |
| Device picker; remote mode; cast button | R2 | F08 |
| Title detail page; pre-play sheet; video player with info overlay, subtitle menu, skip button, error and limit cards; post-play screen | R2 | F12, F17 |
| Admin > Household; child-profile settings | R2 | F18 |
| Admin > Remote access | R2 | F10 |
| Admin > Compatibility | R2 | F19 |
| **R3 additions** | | |
| Admin > Live TV (set-up wizard, sources, guide sources, mapping) | R3 | F20 |
| Guide grid; live player | R3 | F20 |

## Server work the flows share

Most of the server column repeats across flows. These are the shared pieces
of work, which flows lean on each, and what blocks them.

| Server capability | Features | Flows | First release | Blocked by |
|---|---|---|---|---|
| Setup state machine (fresh, unclaimed, claimed, locked), the claim code and restore-at-setup (SEC-IAM-006 to SEC-IAM-009, SEC-OPS-003 to SEC-OPS-008) | ACC-001, ADM-020 | F01, F14 | R1 | |
| Path classes and postures: the cleartext help page, the home posture, trusted proxies and exposure detection, all from one classifier (SEC-NET-001, SEC-NET-016, SEC-NET-024, SEC-OPS-037) | ACC-097, ACC-134 | F01, F03, F05, F10 | R1 | |
| Identity store and sessions: accounts, passkeys, OIDC links, cookie sessions, the separate admin session and step-up tags, revocation on the next request, the device registry, the shared limiter (SEC-IAM-037, SEC-IAM-041, SEC-IAM-043, SEC-IAM-101) | ACC-002, ACC-050, ACC-057, ACC-063, ACC-065, ACC-068, ACC-079, ACC-124 | F01, F03, F10, F14, F15 | R1 | ADR 3; the identity record (SEC-STD-006) |
| Browser pairing: codes, the approval sheet, the typed and matching code, limited-class browsers (SEC-IAM-056 to SEC-IAM-060, SEC-IAM-108) | ACC-062 | F03, F04 | R1 | The identity record |
| One authorisation function and route table, the visibility predicate and the generated cross-user suite (SEC-IAM-067 to SEC-IAM-071) | ACC-120, ACC-121, ACC-030 | Every flow | R1 | |
| The audit log, each person's security events, owner alerts with "This wasn't me", and notices to an account's devices (SEC-OPS-020 to SEC-OPS-034, SEC-IAM-097, SEC-IAM-098) | ADM-110, ADM-116, ACC-071, ACC-078 | F01, F03, F04, F10, F14, F15, F16 | R1 | |
| Recovery: recovery codes, admin recovery links, the recovery hold and host-only owner recovery (SEC-IAM-089 to SEC-IAM-092, SEC-IAM-106) | ACC-004, ACC-064, ADM-034 | F01, F03, F14, F15 | R1 | |
| The admin-visibility policy, "What admins can see" and private sessions (SEC-PRV-024, SEC-PRV-025, SEC-PRV-027) | ACC-115, ACC-117, ADM-099 | F05, F10, F12, F16 | R1 | Security README decision 5 |
| The user log, with typed, versioned, exportable events | LAT-007, ADM-078, MUS-122, ACC-010 | F05, F06, F07, F11, F13, F14, F16 | R1 | ADR 3 |
| Scan pipeline, job registry and progress events, with parsing in the sandboxed scan worker (SEC-MED-018 to SEC-MED-024) | LIB-019, LIB-020, LIB-021, LIB-022, ADM-093, ADM-095 | F02, F11, F13 | R1 | Open decision 8, for loudness |
| Change feed and sync endpoints, filtered by grants and restrictions | LIB-018, CLI-022, INT-006, ACC-030, ACC-037 | F02, F03, F04, F05, F09, F10, F18 | R1 | |
| Byte serving through short-lived, session-bound, revocable capability URLs (SEC-API-026 to SEC-API-029) | MUS-066, ACC-122 | F05, F07, F08, F12, F15, F17 | R1 | |
| The audio-only packager for browsers, in a worker (SEC-MED-018) | MUS-230 | F05, F12 | R1 | Open decision 9 |
| The queue document and its versioned operations | MUS-122, MUS-116, LAT-009 | F05, F06, F08 | R1 | ADR 3 |
| The playback decision engine with structured reasons, in the core | MUS-099, ADM-100, INT-134, VID-002 | F05, F08, F12, F17 | R1 for audio; R2 for video | |
| The rule engine, in the core | DIS-119 | F06, F07, F18 | R1 | |
| The matcher, in the core | ADM-044, MUS-140 | F06, F11 | R1 | |
| The egress gate and its activity log | ADM-028, ADM-129, LIV-015 | F01, F13, F20 | R1 | |
| Backup, restore and rebuild, with encrypted and signed backups, the recovery kit and key rotation after restore (SEC-OPS-041 to SEC-OPS-045, SEC-PRV-039, SEC-PRV-040) | ADM-065, ADM-068, ADM-070, ADM-077, ADM-141 | F13, F14 | R1 | ADR 3 |
| Health records, diagnostics and the inspect API | LIB-193, ADM-108, ADM-123, ADM-124, ADM-125 | F02, F12 | R1 | |
| Invitations as capabilities, with the privacy notice and the matching-code confirmation (SEC-IAM-078, SEC-IAM-079, SEC-PRV-053) | ACC-080 | F10 | R1 | ADR 3 |
| TV device authorisation and device keys, on the same pairing protocol as browsers (SEC-IAM-048 to SEC-IAM-055) | ACC-061, ACC-051, CLI-027 | F04 | R2 | |
| The control channel and its conflict rule | CLI-101, CLI-102 | F08 | R2 | Its own design |
| Offline grants | CLI-095, ACC-045 | F07, F15 | R2 | Clients map open decision 4 |
| The remuxer in a worker and the transcode sandbox (SEC-MED-081, SEC-MED-064 to SEC-MED-068) | VID-003, VID-005, CLI-083 | F07, F12, F17 | R2 | Security README decision 9 |
| The iroh endpoint and relays | ACC-096, ACC-100 | F04, F08, F10 | R2 | Open decision 10 |
| The adapter layer, on its own port with its own credentials (SEC-EXT-051 to SEC-EXT-066) | INT-086, INT-098, ACC-130 | F11, F19 | R2 | Open decision 4; security README decision 8 |
| Live TV ingest, buffer and recorder | LIV-003, LIV-078, LIV-102 | F20 | R3 | Open decision 23 |

## Owner alerts the flows raise

R1 delivers these in the app only: the notice centre on every device the
recipient is signed in on, with a persistent banner for critical ones
(SEC-OPS-032, SEC-IAM-098). Every alert about a device or credential offers
**It was me** and **It wasn't me**; the second revokes it, ends its sessions
and makes its stream URLs fail (SEC-OPS-033). The alerts marked "never" in
the last column cannot be switched off or suppressed, only batched with
every distinct device listed (SEC-OPS-034). Nobody is alerted about their
own interactive action, re-signing in with a known passkey on a known
device raises nothing, and non-critical alerts are batched into a daily
summary. Alerts never carry another person's activity (SEC-PRV-030).

| Alert | Raised in | Who gets it | Muting |
|---|---|---|---|
| A new device or credential on an account | F03, F04, F15 | That person's other devices; the owner for managed profiles; every admin when the account is an admin (SEC-IAM-098, SEC-OPS-032) | Never on owner or admin accounts |
| A device enrolled remotely by typed code | F04 | Every adult in the household (SEC-IAM-059) | Not set by the baseline |
| Repeated failed sign-ins: 10 for one account or 30 from one source within 15 minutes | F03 | The owner and admins; the account holder for their own account (SEC-OPS-032) | Yes |
| A passkey whose signature counter went backwards | F03 | As for a new credential on that account (SEC-IAM-021) | As for a new credential |
| A new user, an invitation redeemed, or a rise in someone's role or library access | F02, F10 | The owner and admins (SEC-OPS-032) | Never for a new admin; otherwise yes |
| An invitation waiting for the inviter's matching code | F10 | The inviter (SEC-IAM-079) | Not an alert; a task until confirmed or expired |
| A share link suspended for spreading too widely | F10 | The sharer (SEC-API-097) | Not set by the baseline |
| Recovery by code or admin link, and the hold it starts | F03, F15 | Every existing device of that account, with one-tap cancel (SEC-IAM-090, SEC-IAM-106) | Never on owner or admin accounts |
| Owner recovery from the host | F01 | Every admin (SEC-IAM-092, SEC-OPS-009) | Never |
| A request from the internet reached the home posture, or a proxy that was never declared | F01, F03, F10 | The owner, at most once per listener per day with a count (SEC-NET-027, SEC-NET-017) | Never, until acknowledged |
| The posture was left: a public proxy declared or remote access turned on | F10 | Every admin, at the first internet-posture request (SEC-OPS-038, SEC-TM-017) | Not set by the baseline |
| A host-equivalent action (trusted proxies, naming, egress, plugins, adapters, backup download or restore, key rotation, new admin) | F01, F02, F10, F14, F19, F20 | Every admin (SEC-TM-017) | Never for a new admin or a plugin change |
| A backup downloaded or restored, and the "review devices and access" list after a restore | F14 | The owner (SEC-OPS-032, SEC-OPS-044) | Never |
| Keys rotated | F14 | The owner and admins (SEC-OPS-018) | Yes |
| A security setting loosened outside the app (a file edit) | F13 | The owner (SEC-OPS-031) | Not set by the baseline |
| The certificate expires in 30 or 7 days, with "renew now" | F01 | The owner (SEC-NET-072) | Not set by the baseline |
| A certificate for the server's name that the server did not request | F01 | The owner, as a critical alert (SEC-NET-069) | Not set by the baseline |
| The audit log failed verification | F13, F14 | The owner (SEC-OPS-023, SEC-OPS-075) | Never |
| The running version is affected by an advisory, or out of support | F13 | The owner and admins (SEC-OPS-032, SEC-OPS-070) | Banner until updated |
| The recovery kit is not yet confirmed; the security-fix check is off | F01, F13 | The owner, as a dashboard reminder (SEC-PRV-040, SEC-OPS-047) | Until done |
| A PIN guessed wrong 10 times on one TV | F04, F18 | The profile's owner or guardian, in the daily summary (SEC-IAM-062) | Yes |
| A household TV seen on another network | F04 | Every adult in the household (SEC-IAM-109) | Not set by the baseline |

Children and other managed profiles never receive security alerts; the
owner does. On a TV, alerts appear only on an admin's profile, as a banner
that does not interrupt playback.

## Gaps and questions the flows expose

Walking the flows step by step turned up these points that the feature map
does not settle. Each has a recommendation and names the row that should
own the answer. Where the security baseline now settles a gap, the entry
says so and keeps its number.

1. **G1. Half-finished setup and expired codes (F01; owner ACC-001).**
   *Settled by the security baseline.* The claim consumes the code, creates
   the owner and enrols the passkey in one transaction, so a browser that
   closes half-way has committed nothing and the same code still works
   (SEC-IAM-009). The code lasts 24 hours, a restart does not change it,
   and `gunmetal claim-code` mints a new one after it expires (SEC-IAM-007).
   The earlier recommendation to print a fresh code on every restart is
   withdrawn, because it would strand a claim link the owner had already
   scanned.
2. **G2. Passwords over plain HTTP (F01, F03; owner ADM-021).** *Settled by
   the security baseline.* There are no passwords (SEC-IAM-025), and over
   plain HTTP every peer except loopback gets only a redirect or help page
   (SEC-NET-001), so no credential ever crosses the home network
   unencrypted. The secure-context panel now explains how to reach the
   HTTPS address instead.
3. **G3. Passkeys after a move (F14; owners ACC-050 and ADM-029).** A
   passkey belongs to the address it was created for (SEC-IAM-018), so
   restoring under a new address breaks every member's passkeys.
   *Recommendation, following the baseline:* setup says the address is
   permanent for passkeys (SEC-NET-072); the restore step shows the old
   address from the backup, recommends keeping it, and warns when the new
   one differs. If it must change, the owner uses host recovery
   (SEC-IAM-092) and members use recovery codes or an administrator's
   recovery link in person, under the recovery hold (SEC-IAM-089,
   SEC-IAM-091, SEC-IAM-106); a running server can instead move members by
   device-to-device approval (SEC-NET-072). There is no password fallback.
4. **G4. Who sees a new library (F02; owner ACC-037).** *Settled by the
   security baseline's deny-by-default rule* (first principle 1,
   SEC-IAM-070): only the owner and admins see a new library until someone
   is granted access, and the add-library sheet asks.
5. **G5. Playlists that come from files (F06; owner LIB-192).** An `.m3u`
   file in a music folder becomes a playlist, but media is read-only
   (LIB-007), so edits cannot be written back. *Recommendation:* show such
   playlists as read-only with "Duplicate to edit", so a rescan never
   fights a person's edits.
6. **G6. Playlist entries after a purge (F06; owner MUS-132).** The map
   does not say what a playlist shows once a track is purged from the
   trash. *Recommendation:* keep the entry as "missing" with its last known
   title and artist, so a later copy rematches by identity (LIB-028).
7. **G7. The address an invite carries (F10; owner ACC-080).** In R1 an
   invite carries the configured public address (SEC-NET-015), and a
   LAN-only address, or a server still in the home posture, is useless
   outside. *Recommendation:* Admin > Invitations shows the address the
   link will carry and warns when the friend cannot reach it.
8. **G8. What the owner can see, in R1 (F10, F16; owner ACC-115).**
   *Settled by the security baseline.* The page is R1 (SEC-IAM-104,
   SEC-PRV-027), live sessions show no title unless the person chose to
   show it (SEC-PRV-025), and the invitation's privacy notice says the same
   before the friend joins (SEC-PRV-053). ACC-115 moves to R1.
9. **G9. Approving a TV from an iPhone (F04; owner ACC-061).** *Settled by
   the security baseline:* any signed-in personal device, the web client
   included, may approve by opening the approval URL on the server's own
   HTTPS origin, where the passkey's origin binding gives the phishing
   resistance (SEC-IAM-057). The QR code is a link that opens the Android
   app when it is installed and the web client otherwise (CLI-034); test
   both paths.
10. **G10. URL expiry during playback (F05, F12; owner ACC-122).** *Settled
    by the security baseline:* every client must refresh an expired URL
    transparently and resume at the same position, proved by an
    injected-clock end-to-end test (SEC-API-027). It applies to the byte
    path and the packager alike.
11. **G11. Two tabs and one queue in R1 (F05, F08; owner MUS-122).**
    *Settled across the interface documents* (README, decision 9; owner to
    confirm). Without the R2 control channel, two browser tabs on one
    profile can both play, so the last Play wins: play and resume are
    ordinary operations on the versioned queue, which records the issuing
    session by an opaque identifier that is not a credential; the other tab
    sees the new queue version on its next sync, pauses at its point, and
    its bar offers "Continue on this device" (CLI-103), naming no device.
    This needs only the R1 queue versions, and only the profile's own
    sessions can write its queue (SEC-HIS-014, SEC-API-016). player.md's
    separate active-device field is withdrawn in favour of this.
12. **G12. A device that never finished its first sync (F03; owner
    CLI-025).** It cannot fall back to a local copy it does not have.
    *Recommendation:* sync status shows progress and "not yet available
    offline" until the first snapshot lands.
13. **G13. Formats a browser cannot decode (F12; owners MUS-229 and
    MUS-033).** R1 can only explain. MUS-033 sends the R2 extra formats
    through the sandboxed transcoder, but the map does not say whether a
    core format that one browser cannot decode also gets a conversion in
    R2. *Recommendation:* yes, as an Opus stream through the MUS-106 path,
    governed by the same per-person rights as other conversions (VID-010).
14. **G14. A handoff that fails (F08; owner CLI-101).** The map does not
    say what happens when the target does not start. *Recommendation:* the
    source keeps playing until the target confirms, and the device picker
    reports the failure.
15. **G15. Restore needs the setup code (F14; owner ADM-029).** *Settled by
    the security baseline:* restoring onto an unclaimed server needs the
    same setup code as claiming (SEC-OPS-008), and the restore endpoint is
    one of the few an unclaimed server answers (SEC-OPS-003). ADM-029 should
    say so, so nobody on the LAN can restore their own backup onto a fresh
    install.
16. **G16. What search covers once music and video have separate homes
    (F09; owner DIS-083).** *Recommendation:* search covers everything the
    profile can see; opened from the Music or Watch home, it preselects
    that kind's chip, which one tap clears.
17. **G17. The R1 phone gap (F05, F07, F08).** Most R1 compromises in these
    flows come from phones having only the web app: no downloads, no
    dependable background audio on iPhone, no handoff. *Recommendation:*
    settle open decision 5 early. Shipping the native Android music app as
    an R1 point release changes F05, F07 and F08 more than any other single
    decision. It would also bring device keys, offline grants and their
    requirements forward with it (SEC-IAM-048 to SEC-IAM-054).
18. **G18. The recovery kit has no feature row (F01, F14; owner ACC-004).**
    The baseline requires one printable recovery kit with the owner's
    recovery codes and the backup recovery key, a dashboard reminder until
    it is confirmed, and a step-up check to show it again (SEC-PRV-040,
    SEC-IAM-089). The feature map's only recovery-code row is tied to
    two-factor codes (ACC-053), which the baseline removes.
    *Recommendation:* add an R1 accounts row for the recovery kit and
    Account > Recovery, owned with ACC-004.
19. **G19. Phones on the home Wi-Fi with global IPv6 addresses (F01, F03;
    owner ACC-097).** The home posture counts only loopback, private and
    link-local addresses as local (SEC-NET-024), and many home networks give
    phones global IPv6 addresses and prefer them, so a phone on the same
    Wi-Fi can be shown the help page. The fix of counting on-link prefixes
    as local is an open owner decision (operations decision 15).
    *Recommendation:* until it is settled, the help page and `doctor` say
    when this is happening and how to fix it, and the R1 test matrix
    includes a dual-stack home network.
20. **G20. Signing in while the internet is down (F03; owner ACC-003).**
    With the per-server name, the browser must resolve a public name before
    it can reach the server, which can fail while the internet is down unless
    the answer is cached (unverified). *Recommendation:* the docs say so, and
    recommend a local DNS entry, an own-domain name served by the home
    router, or a tailnet name for households that need sign-in during
    outages; native apps over iroh in R2 avoid the problem.

## How these flows feed the next steps

- **For the UI plan**, the surface table above is the screen list. Each
  step's "The person" cell is that screen's job, and each "When it goes
  wrong" branch is a state the screen must design: empty, loading, offline,
  refused or failed.
- **For the server**, each "The server" cell is a behaviour with an
  observable result, so it can be written as a failing test before any
  code. Following the project's testing rules, each behaviour is proved at
  the lowest layer that can observe it: unit tests in the core for the
  matcher, the rule engine, the decision engine and queue operations; API
  tests for authorisation, sessions, capability URLs and sync filtering; and one
  end-to-end test per flow's main path, with its failure branches covered
  below that level.
- **For parallel work**, the shared server table splits into pieces that
  can be built side by side. Nearly every R1 piece writes to the user log or
  the identity store, so ADR 3 has to be accepted first (open decision 1).
- **For security tests**, every requirement ID cited in a flow is a test
  that must carry that ID, and the release fails if one due in that release
  has none (SEC-STD-004). Most of them are generated from the route table
  rather than written per flow: the anonymous-request suite, the
  cross-user matrix, the step-up tag check and the cleartext replay
  (security README, "Security in the test-first process").
- **For order**, the map's R1 build order still applies: the music model
  and scan (F02), the player (F05, F12), the queue and playlists (F05,
  F06), lyrics, search and home (F09), then sign-in, backups and the rest
  of the security baseline (F01, F03, F10, F13 to F16). The flows add two
  constraints. F01's claim and owner account are needed before any flow
  with more than one person can be tested end to end. And the route table,
  the authorisation function, the cleartext rule and the audit log have to
  exist before the first route that serves anyone's data, because the
  generated suites that prove them must cover every route from the first
  one (SEC-IAM-067, SEC-NET-001, SEC-OPS-020).

## Changes made to follow the security baseline

On 2026-10-02 these flows were changed to follow the
[security baseline](../security/README.md). The numbered owner decisions
are those in the baseline's
[open decisions](../security/README.md#open-decisions-for-the-owner); the
changes that rest on one are applied as the baseline recommends and need
the owner to confirm them.

| Where | Was | Now | Requirements | Owner to confirm |
|---|---|---|---|---|
| Every flow; F01, F03, F10, F14, F15 | Passwords with authenticator-app codes where passkeys could not work | No passwords; passkeys, OIDC, and approval from a signed-in device | SEC-IAM-025, SEC-IAM-108 | Yes, decision 1 |
| R1 facts; F01, F03, F05, F09 | Over plain HTTP, an online-only player with password sign-in | Over plain HTTP, only a redirect or help page for every peer except loopback | SEC-NET-001, SEC-NET-024 | No (decision 2 covers how HTTPS is provided) |
| F01 | Per-server HTTPS name and own-domain certificates in Later | Both in R1, the name service only once its zone is on the Public Suffix List | SEC-NET-004, SEC-NET-010, SEC-NET-013, SEC-NET-070 | Yes, decision 2 |
| F01 | Refuses to run as root unless overridden | Refuses root and any capability, with no override | SEC-OPS-053 | Yes, decision 14 |
| F01 | Update check "decided"; setup code with an expiry | A required question with no preselection; a 24-hour code that survives restarts and is reissued with `gunmetal claim-code` | SEC-OPS-047, SEC-IAM-007 | Yes, decision 4 |
| F01, F03 | Sign-in page shows the server's name and message | Generic sign-in page; the name appears after sign-in and on invitations | SEC-NET-047, SEC-API-005 | Yes, decision 25 (web decision 5) |
| F02 | Adding a library needed an admin only | Admin session plus step-up; roots that hold the server's data refused | SEC-IAM-041, SEC-TM-017, SEC-MED-037 | No |
| F03 | Approving a browser from a signed-in device and new-device notices in R2 | Both in R1 | SEC-IAM-108, SEC-IAM-098 | No |
| F03 | "Remember this browser" | "Is this your own device?", with memory-only storage on shared browsers | SEC-CLI-010 | Yes, decision 25 (the default answer) |
| F04 | A QR scan approves a TV wherever it is | Typed and matching code when not on the same local network; remote enrolment never makes a household TV; household TVs local-only | SEC-IAM-059, SEC-IAM-060, SEC-IAM-109 | Yes, decision 16 (local-only) |
| F05, F12 | Silent refresh of expired stream URLs was a gap | A requirement | SEC-API-027 | No |
| F06 | R1 playlist write API with scoped tokens | Tokens and the scripting API in R2 | SEC-IAM-083, SEC-EXT-010 | Yes, decision 8 |
| F07 | Downloads to the SD card in R2 | App-private storage only; removable storage Later, encrypted | SEC-CLI-035, SEC-CLI-072 | Yes, decision 25 (client decision 3) |
| F10 | The owner sees what the friend is playing | The owner sees that the friend is playing, without the title unless the friend opts in | SEC-PRV-025, SEC-IAM-077 | Yes, decision 5 |
| F10 | One-step invitation redemption | Matching-code confirmation for members and multi-library invites; privacy notice first | SEC-IAM-079, SEC-PRV-053 | No |
| F10, F16 | "What admins can see" in R2 | R1 | SEC-IAM-104, SEC-PRV-027 | No |
| F10 | ACC-116 let the owner choose full history for admins | No setting adds history or titles; each person alone chooses to show titles | SEC-PRV-025 | Yes, decision 5 |
| F10, F06 | Music share links in R2 | R1 for music, with per-link limits and a page that reveals nothing of the sharer or server | SEC-API-097, SEC-PRV-031, SEC-NET-047 | Yes, decision 7 |
| F11 | Admins import listening history for the household; rival-database importers in R2 | Each person imports their own; rival-database importers Later, as the baseline's release table places them | SEC-PRV-022, SEC-PRV-025, SEC-STD-031, SEC-TM-074 | Yes, decision 5 |
| F12 | The in-process remuxer | The remuxer in a worker process | SEC-MED-081 | Yes, decision 9 |
| F13 | An older binary rebuilds from a newer log, skipping unknown events | An older binary refuses newer durable state and points to the snapshot | SEC-OPS-051 | No |
| F14 | Encrypted backups in R2; members fall back to passwords after a move | Encrypted and signed backups in R1; restore needs the code, rotates keys and suspends restored devices; recovery ladder after a move | SEC-PRV-039, SEC-OPS-008, SEC-OPS-042 to SEC-OPS-044, SEC-NET-072 | No |
| F15 | Change the password after losing a phone | Remove the phone's own passkey with step-up; "This wasn't me" in alerts | SEC-IAM-023, SEC-OPS-033 | No |
| F17 | Fonts attached to the file served by default | Off by default, opt-in per library with rewriting | SEC-MED-054 | Yes, decision 12 |
| F18 | Every household adult reads a child's history | Only the child's designated guardians | SEC-PRV-029, SEC-PRV-022 | No |
| F18 | Encrypted per-profile stores on shared devices Later | Required in R2 for PIN-protected profiles on household devices | SEC-IAM-065 | No |
| F20 | The tuner-busy sheet shows who holds each tuner | Names a person or channel only for people who share what they play | SEC-PRV-022 | Yes, decision 5 |
| F10; Journeys not designed here | Browser remote access in Later | R2, through the edge, off until the owner turns it on | SEC-NET-041 to SEC-NET-043 | Yes, decision 3 |
| R1 facts; F08 | Native desktop apps in R2; F08 hands playback to "the desktop" app | The desktop shell is Later; F08 hands playback to the web client on a laptop | SEC-TM-074, SEC-CLI-069 | Yes (the release scope; the feature map now agrees) |
| F05, F10, F12 | Stream limits only in R2, as a video case | Server-wide, per-guest and share-link stream limits from R1, with a typed "too many" error that the music player explains | SEC-TM-068, SEC-API-031, SEC-IAM-102 | Yes, decision 25 (the default values) |
| F08 | The picker lists players another person has granted control of; the handoff did not say what it carries | Only the profile's own players, plus household devices shown as "In use"; the handoff carries item IDs and the position, never a URL or token | SEC-HIS-014, SEC-PRV-022, SEC-API-016, SEC-API-026 | No |
| F08 | Cast URLs "refreshable", without saying by whom | The phone refreshes them for its cast session; the receiver holds no credential, because cast credentials are Later | SEC-NET-064, SEC-API-027, SEC-TM-074 | No |
| F10 | Re-inviting people from an old server in R2 | Later, with the rival-database importers, as F11 and the feature map say | SEC-TM-074 | No |

The interface documents were then made consistent with each other. The
decisions that changed these flows (F05's two-tab rule and G11, F08's
laptop target, F17's "Next to watch" row) are recorded in the
[interface README](README.md#decisions-that-made-the-four-documents-consistent).
