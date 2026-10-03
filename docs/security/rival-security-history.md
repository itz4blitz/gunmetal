# Lessons from rivals' security history

Research date: 2026-10-02. Web tools were available and used. Incidents were
checked against CVE records (cveawg.mitre.org), the projects' own GitHub
security advisories, vendor advisories, CISA's Known Exploited
Vulnerabilities catalogue and the original researchers' write-ups. The
session's web-search budget ran out near the end, so the last checks were
made by fetching primary sources directly. Standards identifiers were taken
from the source texts: OWASP ASVS 5.0.0 (the `v5.0.0` tag of the OWASP/ASVS
repository), the OWASP Top 10:2025 site, the OWASP API Security Top 10 2023
(still the latest edition on the project site), NIST SP 800-63B-4 (final,
July 2025) and NIST SP 800-218 v1.1. SP 800-218 Rev. 1 (SSDF 1.2) was only an
initial public draft (December 2025) when its NIST page was read, so this
file cites v1.1 task IDs. Anything not confirmed from a source is marked
"(unverified)".

Releases used below: **R1** Music (server, web client, music library and
player), **R1.1**, **R1.2** and **R1.3** (the point releases after R1:
playlist files, metadata providers and image uploads in R1.1; OIDC, share
links and diagnostic bundles in R1.2; discovery and analysis in R1.3),
**R2** Video (movies and TV, remuxer, sandboxed transcoding, native
TV and mobile clients, and the project name service), **R3** Live (M3U and live TV), **Later**. Anything
that exists in R1 gets its security requirement in R1, even if the feature
grows later.

## Summary

The rivals keep failing in the same few ways, and often fail the same way
twice. Jellyfin fixed path traversal in its HLS segment controller in 2021
and again in 2026. It fixed FFmpeg argument injection in 2023, again in 2025
when the first fix was bypassed, and twice more in 2026. Emby, Jellyfin,
Sonarr, Cleanuparr and Navidrome all trusted forwarded-for headers or
"local network" status between 2023 and 2026, and each paid for it with an
authentication bypass or a rate-limit bypass. Navidrome's Subsonic adapter
had three separate authentication failures between December 2023 and
September 2026. The lesson is
that per-endpoint care does not hold. Gunmetal has to remove each failure
class by construction: a type, a single choke point or a banned API, backed
by a test that fails the build. This is what NIST SSDF task RV.3.3 asks for:
eradicate the class, not the instance.

The key decisions this file recommends:

1. **Where a request comes from is never who it is.** No passwordless local
   sign-in, no "disable authentication for local addresses", and forwarding
   headers are ignored unless the admin lists a trusted proxy (SEC-HIS-001
   to 004). Emby's 2023 mass compromise of about 1,200 servers started
   here.
2. **Deny by default, proven by enumeration.** A route cannot be registered
   without an authorization policy. The anonymous routes are a short,
   reviewed list. Media bytes need a session or a signed, object-bound
   stream token. A fresh server cannot be claimed without a one-time setup
   code shown only on the host (SEC-HIS-005 to 008).
3. **The acting user comes only from the session, and every object read goes
   through one authorization function.** Client-supplied `userId` and
   `ownerId` fields are rejected. A generated test suite replays every route
   as a second user (SEC-HIS-009 to 014). This is the most common class
   across Plex, Jellyfin, Navidrome and Overseerr.
4. **Untrusted strings never become file paths, command-line arguments, SQL
   identifiers or HTML.** Files the server writes are named by content hash.
   Filesystem access goes through confined directory handles. FFmpeg gets
   its arguments from a typed builder and its input as a file descriptor.
   Queries use fixed column enums. The web client renders metadata as text
   only (SEC-HIS-015 to 021, 027 to 033, 038).
5. **One outbound HTTP client, which checks addresses at connect time.**
   Every server-side fetch goes through it, so a DNS name that resolves to
   a private address, a redirect, or a `file:` URL cannot reach the inside
   (SEC-HIS-023 to 026).
6. **No big C toolkits on hostile input inside the server.** No SVG
   rasterising, no ImageMagick, no XML entities, no pickle-style
   deserialisation. Images are decoded by memory-safe code and re-encoded
   before they are served (SEC-HIS-030, 034 to 036).
7. **Credentials never appear in URLs and are never stored reversibly.**
   Signing keys live outside the database. Session tokens are stored only as
   keyed hashes. Secrets fail closed and have no fallbacks. There is no
   network-reachable password recovery, and every sign-in path shares one
   verifier and one rate limiter (SEC-HIS-041 to 051).
8. **Exposure is opt-in, and patch status is visible.** No UPnP port
   mapping. Discovery answers only the local link. A signed, anonymous
   advisory feed warns admins when they are running a vulnerable version.
   The project keeps no power to disable anyone's server (SEC-HIS-052 to
   055). Plex still had about 314,000 internet-facing servers on affected
   versions weeks after a fix. According to press reports, a LastPass
   engineer was compromised through a Plex flaw that had been patched
   almost three years earlier.
9. **No way to run code through the API, and a hardened CI.** No API
   installs native code, scripts or templates. Plugins, when they come, are
   sandboxed WebAssembly with explicit grants. CI workflows never run
   untrusted pull-request code with secrets (SEC-HIS-056 to 059). This
   answers the backdoor plugin in the Emby attack, Kodi's poisoned add-on
   repositories and Jellyfin's CVSS 10.0 CI flaw.
10. **Every fix is a class sweep plus a replayed exploit.** A "rival exploit
    replay" test suite reproduces every incident in this file against
    Gunmetal, so these classes stay closed (SEC-HIS-064 to 066).

Security must not make the product painful, or users will turn it off. The
usable path is built in. TVs pair by showing a code that a signed-in phone
approves. Household members get device credentials, so nobody needs the LAN
shortcut that sank Emby. Rate limiting slows guessing without locking out a
family member. A server behind a reverse proxy gets a one-click "trust this
proxy" prompt instead of a configuration file. Every section of the design
guidance says how its control stays usable.

### Incident ledger

Newest first. Each row gives the root cause in one sentence and the
Gunmetal requirements that close the class. Identifiers are as published in
the CVE record or the project's advisory.

| Date | Product | Incident | Root cause (one sentence) | Gunmetal requirements |
|---|---|---|---|---|
| 2026-09 | Immich | SVG upload reaches ImageMagick's MSL coder and runs code (GHSA-q89f-h332-8q2h, High) | A parser differential let an "SVG" skip libvips and reach ImageMagick, whose coders execute scripts and write files | SEC-HIS-030, 036 |
| 2026-09 | Jellyfin | Path traversal in virtual-folder management deletes or moves any directory (GHSA-6828-c7cx-hvqm, CVSS 9.1), reachable anonymously while the setup wizard is open | A library name supplied by the caller was joined onto a base path, and a rooted or `..` name escaped it during an unauthenticated setup window | SEC-HIS-008, 015, 016 |
| 2026-09 | Jellyfin | Any user can remote-control other users' sessions (GHSA-4vx8-xhc9-qg6x, CVSS 5.4); no patched version was listed when read | The control check only null-checked its inputs, and session IDs were an MD5 of client name and device ID, so they could be computed | SEC-HIS-010, 012, 014 |
| 2026-09 | Jellyfin | IDOR in playlist items through a `userId` query parameter (GHSA-9x85-gx46-6522) | The endpoint authorized as whichever user the caller named | SEC-HIS-009, 010 |
| 2026-09 | Jellyfin | Legacy HLS segment endpoints lacked authorization and allowed path escape (GHSA-hw73-62vp-9gxf), five years after the same controller's 2021 traversal | Old alias routes kept running outside the newer authorization and path checks | SEC-HIS-007, 015, 016 |
| 2026-09 | Jellyfin | SVG images resolve external references, exhausting memory and enabling SSRF and file read (GHSA-cf3c-8m59-2vhx, CVSS 8.1) | Server-side SVG rendering used a library whose defaults load external files and URLs | SEC-HIS-030 |
| 2026-09 | Navidrome | Subsonic API allowed unlimited password guessing (GHSA-p994-r776-mw52, CVSS 7.4) | The rate limiter covered the web and Jellyfin sign-ins but not the Subsonic path | SEC-HIS-046, 048, 051 |
| 2026-09 | Navidrome | Share creation trusted a client-supplied `userId` (GHSA-82gh-4ggp-gfg5, CVSS 7.1); share endpoints lacked ownership checks (GHSA-3g4p-jhv2-xrxf) | Ownership came from the request body, not the session | SEC-HIS-009, 010 |
| 2026-09 | Navidrome | Public share stream URLs kept working after the share expired or was deleted (GHSA-wp9c-pw66-c6j2) | The stream handler checked the token signature but skipped its expiry claim and never looked up the share | SEC-HIS-042, 050 |
| 2026-09 | Navidrome | Symlinks in a library served any file the process could read (GHSA-r5qr-m328-qcf4); the first fix regressed in 0.63.0 to 0.63.1 | The scanner classified files by the link's name, not by the target or its content | SEC-HIS-016, 017, 064 |
| 2026-09 | Navidrome | SSRF through M3U `#EXTALBUMARTURL` and agent-supplied image URLs (GHSA-8hjf-6h34-82hr); cross-library file read through the same tag (GHSA-vwq6-xrw5-phpg) | URLs and local paths from playlist files were fetched or served with no address check and no per-library authorization | SEC-HIS-018, 023, 024 |
| 2026-09 | Navidrome | Plugin SSRF guard bypassed with DNS names such as `127.0.0.1.nip.io` (GHSA-pr2j-mfc8-qjcc) | The guard inspected the hostname string, not the address actually connected to | SEC-HIS-023 |
| 2026-09 | Navidrome | Login rate limit bypassed by rotating spoofed `X-Forwarded-For`, `X-Real-IP` and `True-Client-IP` (GHSA-f295-6wp9-qqfg) | A middleware rewrote the client address from forwarding headers sent by anyone | SEC-HIS-002, 003, 048 |
| 2026-09 | Navidrome | Last.fm link callback was unauthenticated and took the user ID from the query (GHSA-8jrh-w926-8rvw) | An OAuth-style callback was registered outside the authenticated routes and had no state binding to the user | SEC-HIS-005, 009 |
| 2026-07 | Immich | OIDC discovery, token, userinfo and JWKS fetches ran with TLS verification disabled (GHSA-hfvf-5c8x-8rc4) | A development flag that disables certificate checks was passed unconditionally | SEC-HIS-026 |
| 2026-06 | Immich | One-click account takeover through a `javascript:` URL in the sign-in `continue` parameter (CVE-2026-53662, CVSS 9.6; `main` builds only), then an incomplete fix (GHSA-qp2h-w794-2vhf) | A redirect target from the URL was passed to the router with no scheme or origin check | SEC-HIS-032, 064 |
| 2026-06 | Jellyfin | FFmpeg argument injection through a double quote in a subtitle file name, reachable through an unauthenticated subtitle endpoint (CVE-2026-48793, CVSS 8.8) | A file path was placed into a quoted FFmpeg argument string without escaping | SEC-HIS-006, 020 |
| 2026-06 | Jellyfin | MKV attachment file name used as an extraction path (CVE-2026-49246) | A name read from inside a media file was joined onto a directory, and the join accepted `..` and rooted paths | SEC-HIS-015, 016 |
| 2026-06 | Jellyfin | Stored XSS through the `Client` header of a sign-in request, shown in the admin's user-access tab (CVE-2026-49220, CVSS 5.7) | Client-reported metadata was rendered as HTML in an admin page | SEC-HIS-027, 033 |
| 2026-05 | Tautulli | Authentication off by default, plus a newsletter template directory loaded from any path, gave code execution (CVE-2026-41065) | An empty password setting disabled authentication, and the template engine runs code blocks from whatever directory an admin names | SEC-HIS-004, 008, 021, 056 |
| 2026-05 | Cleanuparr | Spoofed local address in `X-Forwarded-For` logs an attacker in as admin (CVE-2026-44183, CVSS 9.8) | The trusted-network check took the leftmost, client-controlled forwarding entry | SEC-HIS-001, 003 |
| 2026-04 | Jellyfin | Subtitle upload `Format` field allowed path traversal, chained through a `.strm` file to read the database, become admin and get root code execution (CVE-2026-35031, CVSS 9.9) | A field meant as a file extension became part of a write path, and a delegated "upload subtitles" right reached the filesystem by name | SEC-HIS-015, 016, 044 |
| 2026-04 | Jellyfin | Any user could add an M3U tuner that read local files and fetched any URL (CVE-2026-35032) | Tuner sources accepted file paths and arbitrary URLs, and Live TV management was on by default for new users | SEC-HIS-013, 023, 025 |
| 2026-04 | Jellyfin | Unauthenticated FFmpeg injection through a lower-case `h264-level` parameter that skipped a regex check (CVE-2026-35033) | Validation was a pattern attribute that a different spelling of the parameter avoided, and the value went into the command line | SEC-HIS-020, 037 |
| 2026-03 | Sonarr | Authentication bypass when authentication was "disabled for local addresses" (CVE-2026-30975, CVSS 8.1) | The local-address check believed `X-Forwarded-For` | SEC-HIS-001, 002, 004 |
| 2026-03 | Jellyfin | `pull_request_target` workflow in the iOS repository ran fork code with near-full write permissions (CVE-2026-31852, CVSS 10.0) | CI gave secrets and write tokens to a job that checked out untrusted code | SEC-HIS-058 |
| 2026-02 | Navidrome | Song comment tag rendered with `dangerouslySetInnerHTML`, stealing the token from `localStorage` (CVE-2026-25578) | Tag text from a media file was treated as HTML, and the session token was readable by script | SEC-HIS-027, 029 |
| 2026-02 | Navidrome | Unbounded `size` on cover-art and share-image endpoints exhausted memory and disk (CVE-2026-25579, CVSS 9.2); a negative size later bypassed the clamp (GHSA-f22h-6qxh-rqq2) | A numeric parameter sized an allocation and a cache file with no upper or lower bound | SEC-HIS-030, 037 |
| 2025-12 | Emby | Weak forgotten-password mechanism gave full admin access with only network access (CVE-2025-64113, CVSS v4 9.3); a quick fix was pushed through a bundled plugin's auto-update | A password-recovery flow tied to a file on the server was reachable and abusable over the network | SEC-HIS-049, 054 |
| 2025-11 | Emby | Stored XSS in the admin devices list through the `X-Emby-Client` header (CVE-2025-64325) | Client-reported metadata was rendered as HTML in an admin page | SEC-HIS-027, 033 |
| 2025-09 | Tautulli | Unauthenticated path traversal in `real_pms_image_proxy` read the config file holding the Plex token and JWT secret (CVE-2025-58761, CVSS 8.6) | A string-prefix check on a path was followed by `..` segments | SEC-HIS-015, 016, 044 |
| 2025-08 | Plex | `/myplex/account` returned the server owner's account and admin token to any user the server was shared with (CVE-2025-34158, CVSS 8.5); fixed in 1.42.1 on 2025-08-08; Censys counted about 314,000 exposed instances on affected versions on 2025-08-25 | An account endpoint lacked an owner-only check and returned another account's credential | SEC-HIS-010, 011, 054 |
| 2025-08 | Plex | plex.tv database accessed; emails, usernames and hashed passwords taken; every user told to reset (third such breach after 2015 and 2022) | A central account store is one target holding every user's credentials | SEC-HIS-061 |
| 2025-05 | Navidrome | Transcoding settings (which contain command parameters) writable by non-admins (CVE-2025-48948) | Function-level admin checks were missing on a settings API that controls a command line | SEC-HIS-013, 021 |
| 2025-04 | Jellyfin | FFmpeg argument injection bypassing the 2023 fix (CVE-2025-31499) | The 2023 fix sanitised some parameters and missed others | SEC-HIS-020, 064 |
| 2025-04 | Jellyfin | `X-Forwarded-For` trusted with an empty trusted-proxy list, letting remote attackers pass as LAN and restart the server (CVE-2025-32012) | LAN devices were authorized for some actions, and LAN-ness came from a spoofable header | SEC-HIS-001, 002 |
| 2025-02 | Navidrome | Subsonic sign-in with a non-existent user and the MD5 of an empty password plus salt succeeded (CVE-2025-27112) | A missing user was treated as a user with an empty password | SEC-HIS-046, 047 |
| 2024-12 | Navidrome | JWT signing secret stored in plain text in the database (CVE-2024-56362, CVSS 7.1) | Anyone who could read the database could forge tokens for any user | SEC-HIS-044 |
| 2024-09 | Navidrome | SQL injection through parameter names, an ORM leak that filtered on the password column, and sign-in with `%` as the username (CVE-2024-47062, CVSS 9.4) | Request parameter names became SQL identifiers and username lookup used `LIKE` | SEC-HIS-038 |
| 2024-09 | Jellyfin | SVG avatar read the admin's token from `localStorage` and raised the attacker to admin (CVE-2024-43801) | User-uploaded SVG was served inline from the app's origin, where tokens were readable by script | SEC-HIS-029, 030, 031 |
| 2024-04 | Navidrome | Playlist `ownerId` writable by the caller (CVE-2024-32963, CVSS 8.1) | Mass assignment of an ownership field | SEC-HIS-009 |
| 2023-12 | Navidrome | Subsonic authentication bypass with a JWT signed by the hard-coded key "not so secret" (CVE-2023-51442, CVSS 8.6) | A start-up ordering bug fell back to a built-in key instead of failing | SEC-HIS-043 |
| 2023-12 | Jellyfin | FFmpeg argument injection through `videoCodec` and `audioCodec` on an unauthenticated stream endpoint (CVE-2023-49096, CVSS 7.7); admins could set a custom FFmpeg path, including a UNC path, that the server executed (CVE-2023-48702) | Request strings went into a command line, and the executable path was an API setting | SEC-HIS-006, 020, 021 |
| 2023-11 | Plex | "Week in Review" emails showed friends what users had watched; users also reported email opt-outs re-enabled | Activity sharing was on by default and preference state was not reliable | SEC-HIS-060 |
| 2023-05 | Emby | Spoofed `X-Forwarded-For` (CVE-2021-25827) and proxy-header spoofing (CVE-2023-33193, CVSS 9.1) let attackers in as local admins on servers with passwordless local sign-in; about 1,200 servers got a credential-stealing plugin (`helper.dll`); Emby used its update channel to stop affected servers starting | "Local network" was a sign-in factor, the network test trusted headers, and admins could install native plugins | SEC-HIS-001 to 004, 054, 056, 062 |
| 2023-04 | Jellyfin | `/ClientLog/Document` path traversal (CVE-2023-30626) chained with stored XSS in device names (CVE-2023-30627, CVSS 9.1) to set the encoder path and load a plugin DLL | Client name and version from the token became a path, device names rendered as HTML, and the API could point the server at an executable | SEC-HIS-015, 021, 027, 033, 056 |
| 2023-03 | Plex | CVE-2020-5741 added to CISA KEV on 2023-03-10; press reports, and Plex's statement to them, tie it to the compromise of a LastPass engineer's home machine running a version almost three years out of date | An admin-reachable upload fed Python `pickle` deserialisation, and the server was never updated | SEC-HIS-035, 054, 056 |
| 2023-02 | Kodi (forum) | A former admin's account was used to download database backups; data on about 400,000 users was offered for sale | A stale privileged account and downloadable backups in a project-run service | SEC-HIS-061 |
| 2022-11 | Sonarr | Authentication made mandatory in v4 because private trackers saw many instances left open to the internet | Authentication had been optional, and non-experts exposed servers anyway | SEC-HIS-004, 008 |
| 2022-08 | Plex | plex.tv database accessed; emails, usernames and bcrypt-hashed passwords taken; forced reset | A central account store is one target holding every user's credentials | SEC-HIS-061 |
| 2021-05 | Jellyfin | Unauthenticated SSRF through `/Images/Remote` and remote-search image endpoints (CVE-2021-29490) | Anonymous requests could name a URL for the server to fetch | SEC-HIS-023, 024 |
| 2021-03 | Jellyfin | Unauthenticated arbitrary file read through HLS segment and image-by-name endpoints (CVE-2021-21402) | Request paths were joined to directories without confinement | SEC-HIS-006, 015, 016 |
| 2021-03 | Jellyfin | Issue #5415 listed unauthenticated stream, subtitle, image and audio endpoints, tokens not bound to user IDs, API keys in query strings, and tokens in `localStorage` | Authorization was added per endpoint instead of designed in | SEC-HIS-005, 006, 009, 029, 041 |
| 2021-02 | Plex | Exposed servers abused for SSDP reflection and amplification DDoS on UDP 32410/32414, about 4.7 to 1 (NETSCOUT ASERT; month from press reports, since the ASERT page shows a revision date that could not be reconciled) | A discovery responder was reachable from the internet, often through UPnP port mappings, and answered spoofed sources | SEC-HIS-052, 053 |
| 2020-10 | Emby | SSRF through `Items/RemoteSearch/Image` and its `ImageUrl` parameter (CVE-2020-26948) | The server fetched URLs named by the client | SEC-HIS-023, 024 |
| 2020-05 | Plex | Camera Upload let an admin-token holder plant a pickled `Dict` file that ran Python code (CVE-2020-5741, CVSS 7.2) | Native object deserialisation of uploaded data | SEC-HIS-035, 056 |
| 2019-04 | Airsonic | Remember-me cookies built with MD5 and the fixed key `airsonic` (CVE-2019-10907); recovered passwords generated with `java.util.Random`, whose 48-bit seed can be brute-forced (CVE-2019-10908) | A compiled-in key and a non-cryptographic generator protected credentials | SEC-HIS-043, 049 |
| 2018-12 | Subsonic | CSRF on internet-radio settings led to SSRF (CVE-2018-20228; earlier CVE-2017-9413 for podcast and radio settings) | State-changing requests had no CSRF protection, and the server fetched stream URLs the request supplied | SEC-HIS-023, 029 |
| 2018-09 | Kodi (add-ons) | Third-party add-on repositories (Bubbles, Gaia, XvBMC) shipped a Monero miner to Windows and Linux; ESET counted 4,774 infections | Add-ons are unsandboxed code from unsigned third-party repositories | SEC-HIS-056, 057 |
| 2018-08 | Plex | XXE in the SSDP/UPnP XML parser, reachable without authentication on the LAN (CVE-2018-13415) | An XML parser resolved external entities in discovery messages | SEC-HIS-034, 053 |
| 2017-05 | Kodi, VLC, Popcorn Time, Stremio | "Hacked in Translation": Kodi's zip subtitle extraction overwrote its own subtitle add-on (CVE-2017-8314); Popcorn Time and Stremio rendered subtitle text as HTML in Electron; VLC had a subtitle parser overflow. Gaming the OpenSubtitles ranking made the malicious file the top automatic pick | Subtitles from the internet were trusted as archives, HTML and well-formed input | SEC-HIS-019, 027, 036, 039, 040 |
| 2017-02 | Kodi (Chorus2) | Encoded `../` in the web interface's image path read arbitrary files (CVE-2017-5982) | A URL path was decoded and joined onto a directory | SEC-HIS-015, 016 |
| 2015 | Plex | Forum and account breach of about 327,000 accounts; The Register reports weak salting (month unverified) | A central account store with weak password storage | SEC-HIS-045, 061 |
| Design | Subsonic API | Credentials travel as `p=` (plain or `enc:` hex) or `t=md5(password+salt)` in the query string | The protocol needs the server to know the plain password and puts credentials in URLs; Navidrome therefore stores passwords reversibly | SEC-HIS-041, 045, 051 |
| Design | Plex | Plex's own help article shows the `X-Plex-Token` as a URL query parameter | Credentials in URLs leak into logs, history, referrers and screenshots | SEC-HIS-041 |

Two corrections to claims found elsewhere: CVE-2025-34158 is rated CVSS 8.5
(High) in its CVE record and needs a login as a user the server is shared
with, although some press called it maximum severity and unauthenticated.
CVE-2026-31852 is rated 10.0 in its CVE record.

## Threats

| ID | Threat | Who (the attacker or failure) | Impact | Likelihood | Mitigated by (requirement IDs) |
|---|---|---|---|---|---|
| T-HIS-01 | Request pretends to come from the local network, or rotates forged addresses, to skip sign-in or rate limits | Internet attacker scanning for exposed media servers | Admin takeover, credential harvesting at scale (Emby 2023) | High | SEC-HIS-001, 002, 003, 004, 048 |
| T-HIS-02 | Forgotten, legacy or callback endpoint with no authentication | Internet attacker with a scanner or a guessed item ID | Media and metadata disclosure, a foothold for SSRF or injection chains | High | SEC-HIS-005, 006, 007, 024 |
| T-HIS-03 | A fresh or misconfigured server is claimed before its owner finishes setup | Anyone who reaches the port first: LAN neighbour or internet scanner | Full ownership of the server | Medium | SEC-HIS-004, 008, 055 |
| T-HIS-04 | Signed-in user acts on another user's objects, sessions or devices, or reads the owner's credentials | Friend or household member the server is shared with | Privacy loss, privilege escalation to owner | High | SEC-HIS-009, 010, 011, 012, 013, 014 |
| T-HIS-05 | Untrusted name (upload field, tag, attachment, playlist entry, archive entry, symlink, URL path) reaches the filesystem | Low-privilege user, anyone who can drop files on a NAS share, author of a malicious media file | Arbitrary file read or write, database theft, code execution (Jellyfin CVE-2026-35031) | High | SEC-HIS-015, 016, 017, 018, 019, 044 |
| T-HIS-06 | Injection into FFmpeg or helper command lines | Anonymous caller with an item ID; author of a file with a hostile name | File read or write, code execution | Medium in R1, High from R2 | SEC-HIS-020, 021, 022 |
| T-HIS-07 | Server-side request forgery through image URLs, metadata, playlists, tuners, plugins or OIDC | User, metadata source, playlist author, compromised identity-provider path | Internal network and cloud-metadata access, local file read | Medium | SEC-HIS-018, 023, 024, 025, 026 |
| T-HIS-08 | Stored or reflected XSS from metadata, device names, SVG or redirects, reaching an admin's browser | Author of a malicious file; low-privilege user | Token theft, then admin takeover | High | SEC-HIS-027, 028, 029, 030, 031, 032, 033 |
| T-HIS-09 | Hostile file processed by a complex or memory-unsafe parser (XML entities, ImageMagick, pickle, subtitle parsers, player scripting) | Author of a malicious file; LAN attacker sending discovery packets | Code execution, file read | Medium | SEC-HIS-030, 034, 035, 036, 039, 040 |
| T-HIS-10 | Unbounded parameters exhaust memory, disk or CPU | Any user, or anyone holding a share link | Outage, full disk | High | SEC-HIS-030, 037 |
| T-HIS-11 | Request data becomes SQL structure, or wildcard matching weakens lookups | Low-privilege user | Database dump, sign-in bypass (Navidrome CVE-2024-47062) | Medium | SEC-HIS-038 |
| T-HIS-12 | Credential theft or forgery through URLs, logs, reversible storage, plaintext or fallback keys, weak recovery, or database read | Anyone who sees logs, proxies, backups or the database file; attacker with a file-read bug | Account and admin takeover | High | SEC-HIS-041, 043, 044, 045, 049 |
| T-HIS-13 | Password guessing or bypass through a weaker side door (adapter, unknown-user path, unthrottled API) | Internet attacker | Account takeover | High | SEC-HIS-046, 047, 048, 051 |
| T-HIS-14 | Revocation that does not revoke: share links, devices or sessions keep working | Former friend, thief of a phone | Continued access after removal | Medium | SEC-HIS-042, 050 |
| T-HIS-15 | Exposed servers stay unpatched for years, or are used as DDoS amplifiers | Mass scanners, DDoS operators | Compromise at scale; the server used as a weapon | High | SEC-HIS-052, 053, 054, 055, 065 |
| T-HIS-16 | Admin powers include running code, so any admin compromise becomes host compromise | Attacker who obtained an admin session through another bug | Host takeover and a path into the home network (the LastPass case) | Medium | SEC-HIS-021, 035, 056, 057 |
| T-HIS-17 | Supply-chain compromise through CI workflows, add-on repositories or tampered releases | Outside contributor, compromised maintainer token | Every installation compromised | Medium | SEC-HIS-057, 058, 059 |
| T-HIS-18 | Breach of a project-run account or community service | Attacker against the project's own infrastructure | Mass credential and personal-data exposure | Low (if SEC-HIS-061 holds) | SEC-HIS-061 |
| T-HIS-19 | The product itself leaks what people watch or listen to | Product design failure | Social harm, loss of trust (Plex 2023) | Medium | SEC-HIS-060 |
| T-HIS-20 | Compromise goes unnoticed: no audit trail, no alerts | Process failure | Long dwell time; admins cannot tell what was touched | Medium | SEC-HIS-055, 062, 063 |
| T-HIS-21 | The project gains, or is forced to use, the power to disable users' servers | Project or its update channel | Loss of user control; a single point of abuse | Low | SEC-HIS-054, 059 |
| T-HIS-22 | A fix closes one instance, and the class reappears or the fix regresses | Process failure | Repeat advisories (Jellyfin HLS 2021 and 2026, FFmpeg 2023 to 2026) | High | SEC-HIS-064, 066 |

## Requirements

| ID | Requirement | Standards | Release | Verified by |
|---|---|---|---|---|
| SEC-HIS-001 | The server must not base any authentication or authorization decision (sign-in without a credential, admin rights, permission to restart, relaxed limits) on the client's IP address, network range or "local network" status. | ASVS 5.0 8.4.2, 8.1.3; A07:2025, A01:2025; API2:2023; CWE-290, CWE-348 | R1 | Property test of the policy engine: for any request, changing only the source address (loopback, RFC 1918, link-local, ULA, public) never changes the decision. Integration test against a real SQLite database: every non-anonymous route called with no credential from `127.0.0.1`, `::1` and `192.168.1.10` returns 401. |
| SEC-HIS-002 | **Withdrawn 2026-10-02: merged into SEC-NET-016.** One forwarding-header rule. | RFC 7239 §8.1; ASVS 5.0 4.1.3, 15.3.4; A02:2025; CWE-348 | Withdrawn | Proved by the tests of SEC-NET-016 |
| SEC-HIS-003 | When trusted proxies are configured, the client address must be found by walking the forwarding chain from right to left and taking the first address that is not a trusted proxy; the leftmost entry must never be used on its own. | RFC 7239 §8.1; ASVS 5.0 15.3.4; CWE-348, CWE-290 | R1 | Property test with generated chains of trusted and untrusted hops and attacker-chosen leftmost entries; zero surviving mutants in the resolver. |
| SEC-HIS-004 | The server must have no passwordless sign-in for any network, including loopback, and no configuration key, environment variable or command-line flag that turns authentication off. | ASVS 5.0 6.1.3, 6.3.4; A07:2025; CWE-306, CWE-1188 | R1 | Snapshot test of the configuration schema, which fails if an authentication-disabling key appears. Integration test that every sign-in pathway rejects an empty or missing credential. |
| SEC-HIS-005 | Every HTTP and WebSocket route must declare an authorization policy when it is registered, and anonymous routes must match a reviewed allowlist kept in the repository. | ASVS 5.0 8.1.1, 8.2.1; A01:2025; API5:2023, API9:2023; CWE-862, CWE-306 | R1 | Compile-time: the router builder cannot register a route without a `Policy` value. Unit test that enumerates the router and compares its anonymous routes to the allowlist. Integration test that calls every other route with no credential and expects 401. |
| SEC-HIS-006 | Every endpoint that returns media-derived bytes (stream, download, artwork, lyrics, subtitles, segments, waveforms) must require a session credential or a signed stream token bound to that object; knowing an object ID alone must never be enough. | ASVS 5.0 8.2.2, 9.2.2; API1:2023; CWE-306, CWE-639 | R1 | Integration test for each byte route: a valid ID with no token returns 401, and a token for a different object returns 404. |
| SEC-HIS-007 | Each endpoint must exist at exactly one route; legacy aliases must not be kept after a replacement ships, and removed routes must return 404. | ASVS 5.0 15.2.3; API9:2023; CWE-306 | R1 | Route-inventory snapshot test, which fails when any handler is reachable at two paths. Test that every route listed as removed in the changelog returns 404. |
| SEC-HIS-008 | **Withdrawn 2026-10-02: merged into SEC-IAM-007, SEC-IAM-008.** The claim code is 128 bits and valid for 24 hours; setup stays gone after the claim (SEC-IAM-009). | ASVS 5.0 6.4.1, 6.6.3, 2.3.1; A02:2025, A07:2025; CWE-1188, CWE-306; NIST SP 800-218 PW.9.1 | Withdrawn | Proved by the tests of SEC-IAM-007, SEC-IAM-008 |
| SEC-HIS-009 | Handlers must take the acting user only from the authenticated session; request bodies and query strings must not carry user or owner IDs that affect authorization, and requests that include them must be rejected with 400. | ASVS 5.0 8.2.2, 8.2.3, 15.3.3; A01:2025; API1:2023, API3:2023; CWE-639, CWE-915 | R1 | Compile-time: handlers receive an `Actor` extractor, and request types reject unknown fields. Unit test that no request schema has a `userId`, `ownerId` or `uid` field. Integration test that sending one returns 400. |
| SEC-HIS-010 | Every read or write of a user-owned or library-scoped object must pass through one authorization function that checks the actor's right to that object and its library, and a denial must look the same as a missing object (404). | ASVS 5.0 8.2.2, 8.3.1; A01:2025; API1:2023; CWE-862, CWE-863 | R1 | Generated cross-user integration matrix against a real SQLite database: every object route is replayed as a second user and as a user without access to the object's library, expecting 404. Zero surviving mutants in the authorization module. |
| SEC-HIS-011 | No response may contain another account's credentials, tokens or secrets; secret values must use types that cannot be serialised into an API response. | ASVS 5.0 15.3.1, 14.2.6; API3:2023; CWE-669, CWE-200 | R1 | Compile-fail tests (trybuild) proving secret types do not implement serialisation. Snapshot tests of every response schema. |
| SEC-HIS-012 | Object, session, device, share and invitation identifiers exposed to clients must come from a CSPRNG with at least 128 bits of entropy, and must not be derived from file paths, names, client names or device IDs. | ASVS 5.0 7.2.3, 11.5.1; CWE-330, CWE-340 | R1 | Unit tests: the ID type can only be built from the OS random source. Property test: two fresh databases indexing identical paths produce different IDs. |
| SEC-HIS-013 | Administrative functions, including reading settings that contain command lines, paths, keys or tuner sources, must require an explicit admin permission checked on the server, and new users must start with no management rights. | ASVS 5.0 8.2.1, 8.2.3; API5:2023; CWE-862, CWE-285 | R1 | Generated integration matrix: every admin route, read or write, called with a non-admin session returns 404. Unit test of the default policy for new users. |
| SEC-HIS-014 | Remote control of playback sessions and queue hand-off must be limited to the actor's own sessions unless the target user has granted control, and the commands must be a closed set with no free-text or HTML payloads. | ASVS 5.0 8.2.2, 2.2.1; API1:2023, API5:2023; CWE-639, CWE-79 | R1 | Integration tests: controlling another user's session returns 404 without a grant. Unit test that the command type is a closed enum with no string fields displayed as markup. |
| SEC-HIS-015 | The server must never build a filesystem path from untrusted data (request values, headers, client names, uploaded file names, tag values, container attachment names, playlist entries, archive entry names); files it writes must be named by internally generated IDs or content hashes. | ASVS 5.0 5.3.2, 5.3.3, 5.4.1; A01:2025; CWE-22, CWE-23, CWE-36, CWE-73 | R1 | CI lint (clippy `disallowed-methods`) bans `Path::join`, `PathBuf::push` and `Path::new` on non-literal input outside the storage module. Property tests and a fuzz target for storage-name derivation. |
| SEC-HIS-016 | All file access must go through directory handles that confine resolution beneath the library or data root, so that `..`, absolute paths and symlinks cannot escape even if a path is built wrongly. | ASVS 5.0 5.3.2; CWE-22, CWE-59 | R1 | Integration tests on a real filesystem that plant `..`, absolute-path and symlink escapes, expecting a typed error. Property test of the confined-open function. |
| SEC-HIS-017 | The scanner must classify files by the content of the resolved target (magic bytes and a successful parse), not by entry name or extension, must not follow symlinks out of the library root, and must never serve bytes as a media type the content did not parse as. | ASVS 5.0 5.2.2; CWE-59, CWE-61, CWE-646 | R1 | Integration test with `passwd.flac` linked to a file outside the library, and a link into a library the user cannot read: neither is indexed or served. Fuzz target for classification. |
| SEC-HIS-018 | Playlist files (M3U, M3U8, PLS, XSPF), whether found in a library or imported, must resolve entries only to items in libraries the playlist owner can read; URLs, paths outside libraries and non-media files must be dropped, and artwork paths or URLs in playlists must be ignored. | ASVS 5.0 5.3.2, 1.3.6, 8.2.2; CWE-22, CWE-918, CWE-639 | R1.1 | Property tests of the playlist resolver with generated entries (absolute paths, `..`, `file:` and `http:` URLs, other users' libraries). Integration test that `#EXTALBUMARTURL` causes no fetch and no file read. |
| SEC-HIS-019 | The server must not extract archives (zip, rar, 7z and similar) received from outside until an architecture record allows it, and that record must require ignoring entry paths, refusing symlink entries, and limiting entry count and unpacked size. | ASVS 5.0 5.2.3, 5.2.5, 5.3.3; CWE-22, CWE-409 | R1 | `cargo-deny` ban on archive-extraction crates in the server and core, run in CI. Manual review of any record that lifts the ban. |
| SEC-HIS-020 | External programs must be started only by one typed command builder, which accepts enumerated codecs, containers, levels and filters, passes input and output as file descriptors rather than paths, never invokes a shell, and never places request, setting, tag or file-name text into the argument list. | ASVS 5.0 1.2.5, 1.3.3; A05:2025; CWE-78, CWE-88, CWE-77 | R1 | CI lint (clippy `disallowed-types`) allows `std::process::Command` and its async equivalents only in the builder module. Property test: for generated adversarial inputs, every argument is drawn from the enumerated vocabulary or is a descriptor reference. Zero surviving mutants. |
| SEC-HIS-021 | Paths to external programs must come only from the install or a read-only host configuration file; no API may read or write executable paths, command templates, script hooks or template directories. | ASVS 5.0 15.2.5, 1.3.7; A06:2025; CWE-15, CWE-94, CWE-1336 | R1 | Snapshot test of the configuration schema and of the route inventory: no field or route carries an executable path or template location. |
| SEC-HIS-022 | FFmpeg must run only in the sandboxed worker from record 1: an unprivileged user, no network, no filesystem access beyond the descriptors it is handed, a system-call filter (or the platform equivalent), and CPU, memory and wall-clock limits. | ADR 0001 decision 3; ASVS 5.0 15.2.5; CWE-250, CWE-269 | R2 | Integration test that runs hostile payloads in the worker (opening a socket, opening a file outside its descriptors, forking, allocating past the limit) and asserts each one is denied or killed. |
| SEC-HIS-023 | All outbound requests (metadata, artwork, scrobbling, advisory feed, OIDC, tuners, plugins) must use one egress client that allows only `http` and `https`, checks every resolved address at connect time against loopback, private, link-local, multicast, unspecified and IPv4-mapped ranges (except hosts an admin explicitly allowed for that integration), re-checks every redirect hop, and caps size and time. | ASVS 5.0 1.3.6, 13.2.4, 13.2.5, 15.3.2; A01:2025; API7:2023; CWE-918 | R1 | CI lint (`disallowed-types`) allows HTTP client types only in the egress module. Integration tests with a local DNS stub that resolves names to `127.0.0.1`, `169.254.169.254`, `::ffff:127.0.0.1` and private ranges, and with redirect chains into them. Property test of the address classifier. |
| SEC-HIS-024 | The server must never fetch a URL taken from an unauthenticated request, and must fetch URLs from media metadata, playlists or plugin responses only through a provider integration an admin has enabled. | ASVS 5.0 13.2.4, 13.1.1; API7:2023, API10:2023; CWE-918, CWE-306 | R1 | Route-inventory test: no anonymous route accepts a URL-typed parameter. Integration test: a file with an artwork URL tag causes no outbound request while providers are disabled. |
| SEC-HIS-025 | Live TV tuner and M3U sources must accept only `http` or `https` URLs (never file paths or `file:`), must be configurable only by admins, and every channel URL must go through the egress client with the admin's LAN allowlist each time it is fetched. | ASVS 5.0 1.3.6, 5.3.2; API7:2023; CWE-918, CWE-73 | R3 | Unit tests of the tuner URL parser with file paths and non-HTTP schemes. Integration tests showing a playlist that points a channel at loopback or a local file is refused. |
| SEC-HIS-026 | TLS clients (OIDC, providers, advisory feed) must verify certificates, and release builds must contain no setting or code path that turns verification off. | ASVS 5.0 12.3.2, 12.2.1; A04:2025; CWE-295 | R1 | CI lint bans certificate-bypass builder methods. Integration test against a self-signed endpoint expects a typed TLS error. |
| SEC-HIS-027 | The web client must render all text that comes from media files, tags, lyrics, file names, playlist and user names, device and client names, and server error messages as text, never as HTML; `dangerouslySetInnerHTML`, `innerHTML`, `outerHTML`, `document.write` and `eval` must fail the lint. | ASVS 5.0 3.2.2, 1.2.1, 1.3.2; A05:2025; CWE-79 | R1 | ESLint rules (`react/no-danger`, `no-restricted-properties`, `no-eval`) run in CI. Component tests render a corpus of XSS payloads in every metadata field and assert the literal text appears and no element or handler is created. |
| SEC-HIS-028 | Every HTML response must carry a Content-Security-Policy with no `unsafe-inline` or `unsafe-eval` for scripts, `object-src 'none'`, `base-uri 'none'` and `frame-ancestors 'none'`, plus `X-Content-Type-Options: nosniff`. | ASVS 5.0 3.4.3, 3.4.4, 3.4.6; CWE-79, CWE-1021 | R1 | Integration test asserting the headers on every HTML route. End-to-end test that an injected inline script is blocked and reported. |
| SEC-HIS-029 | **Withdrawn 2026-10-02: merged into SEC-API-032, SEC-API-036, SEC-TM-058.** The session cookie is SameSite=Lax, which is safe here because SEC-API-033, SEC-API-034 and SEC-API-036 close cross-site request forgery; Strict would open shared links signed out. | ASVS 5.0 3.3.1, 3.3.2, 3.3.3, 3.3.4, 3.5.3, 14.3.3; CWE-922, CWE-1004, CWE-352 | Withdrawn | Proved by the tests of SEC-API-032, SEC-API-036, SEC-TM-058 |
| SEC-HIS-030 | Images from files or users (embedded cover art, sidecar art, avatars, playlist covers) must be decoded by a memory-safe raster decoder with pixel and allocation limits and re-encoded to JPEG, PNG or WebP at one of a fixed set of sizes before they are served; SVG and other scriptable or vector formats must be rejected, never rasterised on the server, and never served from the application's origin. | ASVS 5.0 1.3.4, 5.2.1, 5.2.2, 5.2.6, 3.2.1; CWE-79, CWE-611, CWE-918, CWE-400 | R1 | Unit tests with SVG, HTML and polyglot pictures in FLAC `PICTURE` blocks and ID3 `APIC` frames, expecting a typed rejection. Fuzz target for the image pipeline. `cargo-deny` ban on ImageMagick, librsvg and other vector-rendering crates in the server. |
| SEC-HIS-031 | Every response carrying file-derived bytes must set a `Content-Type` chosen by the server from the indexed type (never from the file or the request), `X-Content-Type-Options: nosniff`, and `Content-Security-Policy: sandbox`; downloads must use `Content-Disposition: attachment` with an RFC 6266-encoded file name. | ASVS 5.0 3.2.1, 4.1.1, 3.4.4, 5.4.1, 5.4.2; RFC 6266; CWE-79, CWE-116 | R1 | Integration test of the headers on every byte route, including files whose content claims to be HTML. |
| SEC-HIS-032 | Any redirect target taken from a request (the post-sign-in "continue" value, OIDC return paths) must be a same-origin relative path that starts with exactly one `/` and is not followed by `/` or `\`; anything else must be replaced with the home route. | ASVS 5.0 3.7.2, 1.2.2; A01:2025; CWE-601 | R1 | Property test with generated `javascript:`, `data:`, `//host`, `/\host`, percent-encoded and mixed-case targets, all mapped to the home route. |
| SEC-HIS-033 | Client-reported metadata (device name, client name, client version, user agent) must be validated on the server to a bounded length and printable characters, stored as untrusted text, and shown only through text rendering. | ASVS 5.0 2.2.1, 2.2.2, 1.2.1; CWE-79, CWE-20 | R1 | Property tests of the validator with control characters, markup and over-long values. Component test of the admin devices page with payload names. |
| SEC-HIS-034 | Any XML parser in the server or core (XSPF, NFO, DLNA, podcast feeds) must reject DOCTYPE declarations and must not resolve external entities or expand entities. | ASVS 5.0 1.5.1; A02:2025; CWE-611, CWE-776 | R1 | Unit tests with XXE and entity-expansion ("billion laughs") documents, expecting a typed error. `cargo-deny` allowlist of XML crates. |
| SEC-HIS-035 | The server must not deserialise any format that can construct arbitrary types or run code (pickle, Java or .NET object serialisation, YAML with type tags) from files, uploads, plugins or the network; persisted and received data must use schema-bound formats that reject unknown fields. | ASVS 5.0 1.5.2; A08:2025; CWE-502 | R1 | `cargo-deny` ban list in CI. Unit test that each wire type rejects unknown fields. Manual review of every new dependency that parses data. |
| SEC-HIS-036 | Every parser of untrusted input (containers, tags, artwork, lyrics, playlists, subtitles, protocol messages) must live in the core crate under its no-panic and no-`unsafe` rules and must have a fuzz target, seeded with reproductions of the rival exploits in this file where they can be rebuilt. | ASVS 5.0 1.4.1, 1.4.2, 16.5.3; NIST SP 800-218 PW.8.2; CWE-20, CWE-787 | R1 | `cargo-fuzz` targets run in CI with a fixed time budget per parser. The existing gate (coverage, zero surviving mutants, no `unsafe` in core). |
| SEC-HIS-037 | Every request parameter that sizes work (image size, bitrate, segment length, page size, offset, count, string length) must be a bounded type with explicit limits, and image sizes must be one of a fixed set, so negative, zero, huge or overflowing values are rejected before anything is allocated. | ASVS 5.0 2.2.1, 15.2.2, 1.4.2; API4:2023; CWE-770, CWE-1284, CWE-400 | R1 | Property tests on each bounded type. Schema test that no request type contains a raw integer or unbounded string. |
| SEC-HIS-038 | Database access must use parameterised queries whose column names, sort keys and filter fields come from closed enums; request parameter names must never become SQL text, and username lookup must be an exact match after a documented normalisation. | ASVS 5.0 1.2.4, 15.3.7; A05:2025; CWE-89, CWE-943 | R1 | CI check that SQL is only written through compile-time-checked query macros. Property tests: sort and filter values outside the enum are rejected. Integration test: signing in as `%`, `_` or `admin%` fails. |
| SEC-HIS-039 | Subtitles from any source must be converted by the core into a text-only internal cue format, with a fixed allowlist of styles, before any client renders them, and clients must render cues through native text APIs. | ASVS 5.0 1.3.5, 3.2.2; CWE-79, CWE-787 | R2 | Fuzz targets for each subtitle format. Component tests rendering hostile cues in each client. |
| SEC-HIS-040 | Native players built on libmpv must turn off user scripts, user configuration files, the youtube-dl hook and unsafe playlist loading for server-supplied media. | ASVS 5.0 15.2.5; CWE-829, CWE-94 | R2 | Unit test of the option set passed to libmpv in each native module (option names to be confirmed against the shipped libmpv version; unverified). Manual review at each libmpv upgrade. |
| SEC-HIS-041 | The native API must not accept session tokens, API keys or passwords in URL query strings; media URLs must carry short-lived, single-object, signed stream tokens that cannot be used as session credentials. | ASVS 5.0 14.2.1, 7.2.2; A07:2025; CWE-598, CWE-532 | R1 | Integration test: a session token sent as a query parameter returns 401. Test that access logs written during the full suite contain no session token or password. |
| SEC-HIS-042 | Stream and share tokens must be checked on every request for signature, allowed algorithm, expiry, audience and purpose, and must also re-check the current state of the session or share they are bound to, so that a revocation takes effect on the next request. | ASVS 5.0 9.1.1, 9.1.2, 9.2.1, 9.2.2, 9.2.3, 7.4.1, 8.3.2; API2:2023; CWE-613, CWE-347 | R1 | Integration tests: after revoking a session or deleting a share, its outstanding stream URL returns 401; expired tokens return 401; a share token used on a non-share route returns 401. Compile-time: the token module exposes no decode-without-verify function. Zero surviving mutants. |
| SEC-HIS-043 | Every server secret (token and stream-URL signing keys, setup code) must be generated by a CSPRNG at first start, and the server must refuse to start if a secret cannot be loaded or created; there must be no default, fallback or compiled-in secret. | ASVS 5.0 11.5.1, 13.3.1, 16.5.3; A04:2025; CWE-321, CWE-798, CWE-636 | R1 | Unit tests: initialisation errors and out-of-order start-up return an error and the process exits non-zero before binding a port. CI check for string literals passed to key constructors. |
| SEC-HIS-044 | Session tokens, device credentials, API keys, app passwords and share secrets must be stored only as keyed hashes, with the key outside the database; signing keys must live in a mode-0600 file outside the database and outside every library and served directory. | ASVS 5.0 13.3.1, 14.1.1, 11.4.1; CWE-312, CWE-522 | R1 | Integration test that copies the database and tries every stored value as a credential, expecting all to fail. File-permission test on Unix. |
| SEC-HIS-045 | **Withdrawn 2026-10-02: merged into SEC-IAM-025.** There are no account passwords to store. | ASVS 5.0 11.4.2, 6.2.8; NIST SP 800-63B-4 §3.1.1.2; A04:2025; CWE-257, CWE-916 | Withdrawn | Proved by the tests of SEC-IAM-025 |
| SEC-HIS-046 | Every authentication pathway (passkey, device key, pairing code, claim code, recovery code, admin-issued recovery link, share-link password, app password, API key, any adapter) must go through one credential verifier that applies the same rate limiting, the same unknown-user handling and the same audit logging. | ASVS 5.0 6.1.3, 6.3.1, 6.3.4; A07:2025; API2:2023; CWE-307, CWE-287 | R1 | Unit test that enumerates the pathway enum and asserts each is wired to the verifier. Integration test per pathway: rapid failures are throttled and logged. |
| SEC-HIS-047 | Authentication must fail for a username that does not exist, is disabled, or has no credential of the type presented, and must never treat such an account as having an empty password, empty hash or default key; the failure must be indistinguishable in response and timing from a wrong credential. | ASVS 5.0 6.3.8; CWE-287, CWE-204 | R1 | Property tests with generated credentials (including an empty password and its MD5) for unknown and disabled users. Integration test comparing response bodies and timing envelopes. Zero surviving mutants. |
| SEC-HIS-048 | **Withdrawn 2026-10-02: merged into SEC-API-056.** Lockout applies only to guessable secrets; passkeys and device keys are throttled, never disabled. | ASVS 5.0 6.3.1, 2.4.1; NIST SP 800-63B-4 §3.2.2; CWE-307 | Withdrawn | Proved by the tests of SEC-API-056 |
| SEC-HIS-049 | **Withdrawn 2026-10-02: merged into SEC-IAM-025, SEC-IAM-091, SEC-IAM-092, SEC-IAM-106.** No anonymous recovery route; admin-issued links with a recovery hold; host-only owner recovery. | ASVS 5.0 6.4.1, 6.4.3, 6.4.6, 11.5.1; NIST SP 800-63B-4 §4.2, §4.2.3; CWE-640, CWE-338 | Withdrawn | Proved by the tests of SEC-IAM-025, SEC-IAM-091, SEC-IAM-092, SEC-IAM-106 |
| SEC-HIS-050 | **Withdrawn 2026-10-02: merged into SEC-IAM-042, SEC-TM-028.** Listing, revoking and immediate effect. | ASVS 5.0 7.4.1, 7.4.2, 7.4.3, 7.4.5, 7.5.2; CWE-613 | Withdrawn | Proved by the tests of SEC-IAM-042, SEC-TM-028 |
| SEC-HIS-051 | An OpenSubsonic adapter, if shipped, must authenticate with the OpenSubsonic `apiKey` extension by default; legacy `p`, `t` and `s` parameters may be accepted only with generated per-app passwords that are never the account password, are revocable, and go through the shared verifier and rate limiter. | OpenSubsonic API key extension; ASVS 5.0 6.3.4, 14.2.1; API2:2023; CWE-257, CWE-598, CWE-328 | R2 | The SEC-HIS-046 to 048 test suites run against the adapter. Integration test: the account password is refused in `p` and `t`. |
| SEC-HIS-052 | The server must not create router port mappings through UPnP-IGD, NAT-PMP or PCP. | ASVS 5.0 13.1.1; A02:2025; CWE-1188; NIST SP 800-218 PW.9.1 | R1 | `cargo-deny` ban on port-mapping crates. Manual review of network code at each release. |
| SEC-HIS-053 | LAN discovery must use mDNS, must ignore unicast queries whose source is not on the local subnet or prefix, must be rate-limited, and must advertise only a service name, port and server ID; SSDP and UPnP responders must not be shipped without a new architecture record. | RFC 6762 §5.5, §11; ASVS 5.0 2.4.1; CWE-406, CWE-940 | R1 | Integration test in a network namespace: a query from an off-subnet source gets no answer, and a burst is rate-limited. Unit test of the advertised record set. |
| SEC-HIS-054 | **Withdrawn 2026-10-02: merged into SEC-OPS-047, SEC-TM-067.** The advisory check is a required first-run question (SEC-OPS-047); no remote disable (SEC-TM-067). | NIST SP 800-218 RV.1.3, PS.2.1; ASVS 5.0 15.1.1, 15.2.1; A03:2025, A08:2025; CWE-494, CWE-1104 | Withdrawn | Proved by the tests of SEC-OPS-047, SEC-TM-067 |
| SEC-HIS-055 | On every start the server must log, and the dashboard must show, a security summary: listening addresses, whether HTTPS is active, trusted proxies, enabled adapters and providers, admin accounts with their credential types and last use, and the advisory status. | NIST SP 800-218 PW.9.2; ASVS 5.0 16.3.3; CWE-1188 | R1 | Snapshot test of the summary for fixed configurations. Component test of the dashboard panel. |
| SEC-HIS-056 | No API, admin or otherwise, may install, upload or load executable code (native libraries, scripts, templates containing code), and the server must not load native code dynamically. | ASVS 5.0 15.2.5, 1.3.2, 1.3.7; A08:2025; CWE-94, CWE-494, CWE-829 | R1 | `cargo-deny` ban on dynamic-loading crates. Route-inventory test: no route accepts a code or plugin upload. |
| SEC-HIS-057 | Plugins must be WebAssembly modules with no ambient authority, with network access only to hosts declared in a signed manifest and approved by an admin, all through the egress client; installing one must require fresh admin authentication. | ASVS 5.0 15.2.5, 13.2.4, 7.5.3; A03:2025, A08:2025; CWE-829, CWE-494; NIST SP 800-218 PW.4.4 | R2 | Integration tests with hostile plugins that try undeclared hosts, the filesystem, the clock and unbounded memory. Unit tests of manifest signature checks. |
| SEC-HIS-058 | CI workflows must never run untrusted pull-request code with secrets or write tokens (no `pull_request_target` or `workflow_run` that checks out pull-request code), must set least-privilege `permissions` per job, must pin third-party actions to full commit SHAs, and must expose release signing keys only to protected release jobs. | NIST SP 800-218 PO.3.2, PO.5.1, PS.1.1; A03:2025; CWE-269, CWE-829 | R1 | A workflow linter (for example zizmor or actionlint plus a custom rule) run by CI on `.github/workflows`. Manual review of every workflow change. |
| SEC-HIS-059 | Releases must be signed with build provenance and a software bill of materials, and every client and server download (advisory feed, updates, plugins) must verify a signature before use. | NIST SP 800-218 PS.2.1, PS.3.1, PS.3.2; ASVS 5.0 15.1.2; A08:2025; CWE-494 | R1 | Release CI job fails without signatures, provenance and SBOM. Unit tests of each verifier with tampered inputs. |
| SEC-HIS-060 | A user's play history, library contents or activity must not be shown or sent to any other non-admin user, or included in an email, notification or link preview, unless that user opted in for that audience; each user must be able to see what admins can see about them. | ASVS 5.0 14.2.3, 14.2.6, 8.2.3; A01:2025; CWE-359 | R1 | Integration tests for every route that returns activity, called as another user, before and after opt-in. Component test of the "what your admin can see" page. |
| SEC-HIS-061 | Project services may hold only the minimum routing data the per-server name service needs (random label and public key, plus a relay or edge endpoint ID from R2), with no accounts, user data or link to an owner identity, kept only while it is renewed (SEC-NET-010 to SEC-NET-012, SEC-NET-070); gunmetal.tv must serve only static content. | ADR 0001 decision 7; ASVS 5.0 14.1.1; CWE-359 | R1 | Manual review of the name-service schema at each release; deep-equality test of the registration request (SEC-NET-010); any other project-run service needs a new architecture record |
| SEC-HIS-062 | **Withdrawn 2026-10-02: merged into SEC-OPS-020.** One audit log. | ASVS 5.0 16.3.1, 16.3.2, 16.3.3, 16.4.1, 16.4.2, 16.2.5; A09:2025; CWE-778, CWE-117, CWE-532 | Withdrawn | Proved by the tests of SEC-OPS-020 |
| SEC-HIS-063 | The server must notify a user when a device, app password or API key is added to their account or a credential is reset, and notify every admin when an admin is created or a role is raised. | ASVS 5.0 6.3.5, 6.3.7; NIST SP 800-63B-4 §4.2.3; CWE-778 | R1 | Integration tests for each trigger. |
| SEC-HIS-064 | Every security fix must include a regression test that reproduces the exploit, and its advisory must record a class review: a search of the server, core, clients and adapters for the same root cause before publication. | NIST SP 800-218 RV.3.1, RV.3.3, RV.3.4 | R1 | Pull-request template and a CI check that security-labelled changes add a test. Manual review of the class-review note in each advisory. |
| SEC-HIS-065 | Every vulnerability fixed in a released version must be published as a GitHub security advisory with a CVE ID, affected and fixed versions, and credit, and the advisory feed (SEC-OPS-047) must be generated from those advisories. | NIST SP 800-218 RV.1.3, RV.2.2; A09:2025 | R1 | Release checklist reviewed manually. Unit test that the feed generator rejects advisories without version ranges. |
| SEC-HIS-066 | A "rival exploit replay" test suite must contain at least one test per incident in this file that applies to Gunmetal's features, named after the incident and citing its SEC-HIS requirement, and CI must fail if a SEC-HIS ID in this file has no test. | NIST SP 800-218 RV.3.3, PW.8.2 | R1 | CI check that maps each SEC-HIS ID to at least one test. The replay suite runs in the normal gate. |

**Bound by requirements in [standards-coverage.md](standards-coverage.md).** SEC-STD-004 (each regression test carries its requirement ID) and SEC-STD-035 (review of security-sensitive source and agent-written code).
The 2026-10-02 challenge review merged duplicated controls into one owner each; withdrawn rows above say where their content went, and [threat-model.md](threat-model.md#control-ownership) lists every owner.

Count: 57 live requirements: 50 R1, 1 R1.1, 0 R1.2, 0 R1.3, 5 R2, 1 R3 and 0 Later, plus 9 withdrawn rows kept so their IDs stay stable.

## Design guidance

This section is written for the agent that builds the server and clients.
It describes structures that make the requirements hold by construction.

### Request pipeline

1. **Client address.** One resolver turns the TCP peer address, plus the
   trusted-proxy list, into a `ClientAddr`. With an empty list it returns
   the peer address and nothing else. With a list it walks `Forwarded` or
   `X-Forwarded-For` from the right, skipping trusted hops, and stops at the
   first untrusted one (SEC-HIS-002, 003). `ClientAddr` is used for logging
   and rate limiting only. The authorization layer does not receive it, so
   a policy cannot depend on it (SEC-HIS-001).
2. **Usable proxy setup.** If forwarding headers arrive from a peer that is
   not trusted, log it once and show a dashboard banner: "Requests are
   arriving through a proxy at 172.18.0.2. Trust it?" One click, after
   admin re-authentication, adds that exact address. Never auto-trust.
3. **Routes.** Build the router with a builder whose `route()` takes a
   `Policy` (`Anonymous`, `Authenticated`, `StreamToken(Purpose)`,
   `Admin(Permission)`). There is no overload without a policy. A unit test
   walks the router, lists every `Anonymous` route and compares the list to
   a checked-in allowlist, so adding an anonymous route needs a reviewed
   change (SEC-HIS-005). Expect the anonymous list in R1 to hold the web
   client's static assets, a health check, sign-in start and finish, and
   the setup routes while unconfigured.
4. **Actor.** Authentication produces an `Actor` (user ID, profile, granted
   permissions, session ID). Handlers receive it as an argument. Request
   types derive deserialisation with unknown fields rejected and must not
   declare user or owner IDs (SEC-HIS-009).
5. **Objects.** Repository functions that read user-owned or
   library-scoped rows take an `Authorized<Id>`, which only
   `authorize(actor, action, id)` can produce. An unchecked fetch therefore
   does not compile. `authorize` returns `NotFound` for "exists but not
   yours" (SEC-HIS-010).
6. **Cross-user matrix.** A test helper creates two users and two libraries
   in a real SQLite database, creates one object of each kind as user A,
   and calls every route in the inventory as user B and as user C (who
   cannot see the library). Expected statuses come from a table written for
   the test, not from the code (CONTRIBUTING rule 4).

### Setup and recovery

- With no admin in the database, the server generates a setup code from the
  OS random source (at least 64 bits, shown as groups of base32 characters
  that are easy to read on a TV or terminal). It prints the code in a
  banner on standard output, writes it to `setup-code` (mode 0600) in the
  data directory, and mounts the setup router. Creating the admin and
  marking setup complete happen in one transaction. After that the setup
  router is never mounted again (SEC-HIS-008).
- The admin's first credential should be a passkey where a secure context
  exists. Otherwise it is a password that meets SP 800-63B-4 §3.1.1.2 (at
  least 15 characters when it is the only factor) plus a device key for the
  browser.
- Recovery: an admin issues a reset code to a user, and the user chooses
  their own new credential (ASVS 6.4.6). The last admin is recovered with a
  host command such as `gunmetal admin reset`, which needs filesystem
  access to the data directory. No email flow exists (SEC-HIS-049).

### Sign-in that stays usable on TVs and for families

- **TVs and consoles** show a pairing code and a QR code. A signed-in phone
  or browser approves it, and the TV receives a device key stored in the
  platform keystore. Nobody types a password with a remote. The pairing
  code goes through the shared verifier and rate limiter (SEC-HIS-046).
- **Household members** who will never manage passwords get profiles on
  paired household devices. This replaces the "no password on the LAN"
  convenience that Emby, Sonarr and Jellyfin offered, and that attackers
  used.
- **Rate limiting** slows guessing per account and per address, and only
  ever disables the password authenticator, so a child hammering the TV
  cannot lock a parent's passkey out (SEC-HIS-048). The unknown-user path
  runs a dummy Argon2id hash so timing matches (SEC-HIS-047).

### Tokens and secrets

- Keys live in `keys/` under the data directory (mode 0600), separate from
  the SQLite file. They are created atomically before any listener binds.
  If loading fails, the process exits (SEC-HIS-043, 044).
- Session tokens are random 256-bit values. The database stores
  `HMAC-SHA-256(key, token)`, so a stolen database (the Jellyfin
  CVE-2026-35031 and CVE-2026-35032 chains, the Tautulli config read) yields
  nothing usable.
- Stream tokens are a MAC over `(purpose, object ID, session or share ID,
  audience, expiry)`, with a lifetime of minutes. The verifier checks the
  MAC, then the claims, then looks up the session or share row on every
  request. If performance ever needs a cache, revocation must invalidate it
  synchronously (SEC-HIS-042). The module exports `verify()` only.
- The browser's `<audio>` element cannot set headers, which is the only
  reason stream tokens are in URLs. They are short-lived, bound to one
  object, and never accepted as session credentials (SEC-HIS-041). Strip
  them from access logs anyway.

### Files and paths

- Open each library root and the data directory once as a confined
  directory handle (the `cap-std` crate, or `openat2` with
  `RESOLVE_BENEATH` on Linux) and do all I/O relative to those handles
  (SEC-HIS-016).
- Rust's `Path::join` and `PathBuf::push` replace the base when the argument
  is absolute. This is the same trap as .NET's `Path.Combine`, which caused
  Jellyfin CVE-2026-49246 and the virtual-folder traversal. Ban both outside
  the storage module with clippy's `disallowed-methods` (SEC-HIS-015).
- Store derived files (resized artwork, extracted lyrics, attachments,
  uploaded images) under names built from a content hash and a size or
  kind enum: for example `art/ab/abcdef…-384.webp`. The original file name
  is kept only as display text.
- The scanner follows a symlink only if its fully resolved target stays
  inside the same library root, then classifies by content (SEC-HIS-017).
  A file that does not parse as a supported format is not indexed.

### External programs (R1 if any process runs; the sandbox lands in R2)

- `gunmetal-core` holds a pure function `fn ffmpeg_args(job: &TranscodeJob)
  -> Vec<OsString>`, where every field of `TranscodeJob` is an enum or a
  bounded number. It is unit-, property- and mutation-tested like any
  parser (SEC-HIS-020).
- Input and output are inherited file descriptors (`pipe:0`, `pipe:1`, or a
  descriptor number), so file names, including ones containing quotes
  (CVE-2026-48793), never reach the argument list. Clear the environment
  and set an explicit working directory.
- The FFmpeg path is resolved at start-up from the install layout or a
  root-owned configuration file. No API reads or writes it (SEC-HIS-021).

### Outbound requests

- One `Egress` type wraps the HTTP client with a custom resolver and
  connector. After DNS resolution and before connecting, each address is
  classified. Loopback, private, link-local, CGNAT, multicast, unspecified,
  documentation and IPv4-mapped forms of these are refused unless the
  integration's allowlist names that exact host. Redirects are followed
  manually, at most three hops, re-checking each. Responses have size,
  time and content-type caps (SEC-HIS-023).
- Integrations (Cover Art Archive, MusicBrainz, Last.fm, OIDC providers,
  and later tuners and plugins) each declare their host allowlist. A tuner
  on the LAN (an HDHomeRun, for example) is added by the admin as an
  explicit LAN host in R3 (SEC-HIS-025).
- Do not resolve URLs found in tags, NFO files or playlists unless a
  provider integration that uses them is enabled (SEC-HIS-024).

### Images, metadata and the web client

- Embedded pictures in FLAC and ID3 declare their own MIME type and can
  carry SVG or HTML. Decode with a pure-Rust raster decoder under explicit
  pixel and allocation limits. Re-encode to WebP or JPEG at a fixed ladder
  (for example 96, 192, 384, 768 and 1536 pixels). Cache by content hash
  and size. Anything else gets a placeholder (SEC-HIS-030, 037).
- Serve every byte route with a server-chosen `Content-Type`, `nosniff` and
  `Content-Security-Policy: sandbox` (SEC-HIS-031).
- In React Native Web, render metadata with `<Text>` only. Lyrics, comments
  and descriptions are plain text with line breaks. If rich text is ever
  needed, it goes through a small typed AST built in the core, never HTML.
  The lint bans in SEC-HIS-027 apply to every client package.
- Web sessions use cookies (SEC-HIS-029). Native apps keep their device
  keys in the iOS Keychain or Android Keystore.

### Exposure, discovery and updates

- Bind to all interfaces by default so phones on the LAN can connect, but
  never ask the router to open ports (SEC-HIS-052). Remote access is
  iroh-based (record 1) or the admin's own reverse proxy.
- mDNS only, following RFC 6762 §5.5 and §11 (SEC-HIS-053). DLNA and SSDP
  have no authentication by design and are left for a later architecture
  record.
- The advisory feed is a static JSON file on gunmetal.tv, signed with an
  offline Ed25519 key. The server downloads it daily without sending its
  version or any identifier, compares versions locally, and shows a banner
  (SEC-HIS-054). There is no command channel in the other direction.

### Admin powers and plugins

- Admins can manage users, libraries, providers and settings. They cannot
  run code: no executable paths, no templates, no script hooks, no plugin
  upload through the API (SEC-HIS-021, 056). This keeps an admin-session
  theft (the usual end of an XSS chain) from becoming host compromise, as
  it did in the Emby, Jellyfin 2023 and Tautulli cases.
- When plugins arrive, use a WebAssembly runtime with fuel or epoch limits,
  a memory cap and only the host functions the manifest declares
  (SEC-HIS-057). Network access goes through `Egress` with the manifest's
  hosts.

### Process and CI

- Add workflow linting, `cargo-deny` (bans, advisories, sources) and fuzz
  smoke runs to `scripts/gate.sh` when the first code that needs them
  lands. Workflows set `permissions: {}` at the top and grant per job
  (SEC-HIS-058).
- Keep the replay suite under a `rivals` test module, one test per ledger
  row that applies, named like
  `emby_cve_2023_33193_forwarded_header_is_not_local`. Each test cites its
  SEC-HIS ID in a doc comment, and a CI check maps IDs to tests
  (SEC-HIS-066).
- For every advisory, follow SSDF RV.3: record the root cause, grep the
  whole tree for the same pattern, and add a lint or a type if the pattern
  can be banned (SEC-HIS-064).

## Anti-patterns

Never do these:

1. **Treat the local network as a credential.** Emby CVE-2021-25827 and
   CVE-2023-33193 led to about 1,200 backdoored servers. Jellyfin
   CVE-2025-32012 allowed unauthenticated restarts. Sonarr CVE-2026-30975
   bypassed authentication.
2. **Believe forwarding headers by default, or take the leftmost entry.**
   Navidrome's rate-limit bypass (GHSA-f295-6wp9-qqfg) and Cleanuparr
   CVE-2026-44183 (CVSS 9.8) both came from this.
3. **Ship authentication off by default, or leave an open setup window.**
   Tautulli CVE-2026-41065, Jellyfin's anonymous virtual-folder traversal
   during setup, and Sonarr before v4 all did this.
4. **Rely on an unguessable ID instead of authentication.** Jellyfin
   CVE-2023-49096 was an unauthenticated injection whose only barrier was
   guessing a GUID. Issue #5415 lists more.
5. **Derive identifiers from names.** Jellyfin's session IDs were an MD5 of
   client name and device ID, so they could be computed
   (GHSA-4vx8-xhc9-qg6x).
6. **Accept `userId`, `ownerId` or `uid` from the client.** Navidrome
   CVE-2024-32963, GHSA-82gh-4ggp-gfg5 and GHSA-8jrh-w926-8rvw, and
   Jellyfin GHSA-9x85-gx46-6522, all did.
7. **Sanitise command-line strings with patterns or deny lists.** Jellyfin's
   FFmpeg fix was bypassed in 2025 (CVE-2025-31499), by a lower-case
   parameter in 2026 (CVE-2026-35033), and by a quote in a file name
   (CVE-2026-48793).
8. **Let admins set executable paths, command templates or template
   directories.** Jellyfin CVE-2023-48702, Navidrome CVE-2025-48948 and
   GHSA-4p3r-6362-833w, and Tautulli CVE-2026-41065 all exposed these.
9. **Join untrusted names onto paths, or check paths by string prefix.**
   Jellyfin CVE-2026-35031, CVE-2026-49246 and CVE-2023-30626, Tautulli
   CVE-2025-58761, and Kodi CVE-2017-8314 and CVE-2017-5982.
10. **Classify files by name.** Navidrome served `/etc/passwd` through a
    symlink named like a WAV file (GHSA-r5qr-m328-qcf4).
11. **Check SSRF targets by hostname string, or fetch URLs from metadata.**
    Navidrome GHSA-pr2j-mfc8-qjcc and GHSA-8hjf-6h34-82hr, Jellyfin
    CVE-2021-29490 and CVE-2026-35032, Emby CVE-2020-26948.
12. **Render metadata or client-reported names as HTML.** Navidrome
    CVE-2026-25578, Jellyfin CVE-2023-30627 and CVE-2026-49220, Emby
    CVE-2025-64325, and Popcorn Time's subtitle-to-code-execution chain.
13. **Keep tokens in `localStorage`.** This turned XSS into account theft in
    Navidrome CVE-2026-25578 and Jellyfin CVE-2024-43801.
14. **Put credentials in URLs.** Plex documents `X-Plex-Token` as a query
    parameter, Subsonic sends `p=` and `t=`, and Jellyfin issue #5415 flagged
    the same.
15. **Store passwords reversibly for protocol compatibility.** Subsonic's
    `md5(password + salt)` token forces it, which is why Navidrome encrypts
    passwords with a shared key.
16. **Fall back to a built-in secret, or use a non-cryptographic random
    generator.** Navidrome CVE-2023-51442 ("not so secret"), Airsonic
    CVE-2019-10907 (a fixed key) and CVE-2019-10908 (`java.util.Random`).
17. **Offer network-reachable password recovery.** Emby CVE-2025-64113 gave
    admin access with nothing but network access.
18. **Let request parameter names become SQL, or match usernames with
    `LIKE`.** Navidrome CVE-2024-47062 (CVSS 9.4).
19. **Expose a decode-without-verify token helper.** Navidrome's share
    streams ignored expiry because of one (GHSA-wp9c-pw66-c6j2).
20. **Give a compatibility API weaker controls than the native one.**
    Navidrome's Subsonic path had CVE-2023-51442, CVE-2025-27112 and
    GHSA-p994-r776-mw52.
21. **Let a parameter size an allocation without bounds.** Navidrome
    CVE-2026-25579 (CVSS 9.2) and the negative-size follow-up.
22. **Rasterise SVG or pass user images to ImageMagick on the server.**
    Immich GHSA-q89f-h332-8q2h and Jellyfin GHSA-cf3c-8m59-2vhx.
23. **Deserialise native object formats, or enable XML entities.** Plex
    CVE-2020-5741 (pickle; later in CISA KEV and linked to the LastPass
    breach) and Plex CVE-2018-13415 (XXE in SSDP).
24. **Extract archives using their entry paths.** Kodi CVE-2017-8314 let a
    subtitle zip overwrite Kodi's own add-on code.
25. **Open router ports automatically, or answer discovery from the
    internet.** Plex's discovery service became a DDoS amplifier.
26. **Run fork pull-request code in CI with secrets.** Jellyfin
    CVE-2026-31852, CVSS 10.0.
27. **Install third-party code without a sandbox or signatures.** Kodi
    add-on repositories spread a cryptominer in 2018. Emby's attackers
    installed a credential-stealing plugin through the admin API.
28. **Keep a vendor kill switch.** Emby stopped compromised servers through
    its update channel in 2023. It was well meant, but the same power can be
    misused or stolen, and record 1 promises users full control.
29. **Run a central account store.** Plex lost account data in 2015, 2022
    and 2025, and Kodi's forum lost data on about 400,000 users in 2023.
30. **Share activity by default.** Plex's "Week in Review" emails in 2023.
31. **Fix the instance and not the class.** Jellyfin's HLS traversal in 2021
    and 2026, its FFmpeg injection from 2023 to 2026, Immich's incomplete
    redirect fix, and Navidrome's symlink fix regressing in 0.63.0.
32. **Disable TLS verification in code, even "for development".** Immich
    GHSA-hfvf-5c8x-8rc4.
33. **Keep legacy route aliases.** Jellyfin's legacy HLS segment handlers
    were still unauthenticated in 2026 (GHSA-hw73-62vp-9gxf).
34. **Give new users powerful rights by default.** Jellyfin had Live TV
    management on for new users, which made CVE-2026-35032 reachable by
    anyone with an account.

## Open decisions for the project owner

> **Status, 2026-10-02.** The owner decisions from every file in
> `docs/security/` are consolidated and de-duplicated in
> [README.md](README.md#open-decisions-for-the-owner). Where the baseline
> has since chosen, or where a recommendation below disagrees with the
> README, the README and the requirement tables win.

1. **Plain HTTP on the LAN in R1.** `Secure` cookies, the `__Host-` prefix
   and passkeys all need a secure context, and most home users reach their
   server by LAN IP over plain HTTP. *Recommendation:* allow plain HTTP with
   a persistent "this connection is not encrypted" banner, `HttpOnly` and
   `SameSite=Strict` cookies, and passkeys disabled in that mode. Offer
   built-in ACME when the admin has a domain. Decide project-issued
   per-server names in a separate architecture record. *Trade-off:* forcing
   HTTPS would protect sessions on shared networks but would stop many
   households at setup. Nothing in this file depends on the LAN being
   trusted, so the risk is limited to eavesdropping on that network.
2. **FFmpeg in R1.** Record 2 suggests Opus transcoding for mobile data.
   *Recommendation:* R1 serves original bytes only (browsers play FLAC, MP3,
   AAC and Opus), and transcoding arrives in R2 with the sandbox
   (SEC-HIS-022). *Trade-off:* remote listening on mobile data uses more
   bandwidth until R2. The alternative is to pull the sandbox into R1.
3. **When the OpenSubsonic adapter ships, and whether legacy token auth is
   allowed.** *Recommendation:* not in R1. When it ships, use `apiKey` only
   by default, with legacy `t`/`s` behind an admin switch and per-app
   passwords (SEC-HIS-051). *Trade-off:* some older Subsonic apps will not
   connect without the switch.
4. **Advisory check on by default.** *Recommendation:* on, because the feed
   is static, signed and sends no identifiers. Offer an opt-out and say in
   the documentation what the request reveals: an IP address to Cloudflare.
   *Trade-off:* the privacy cost against patch latency, which the Plex
   numbers show is the bigger risk.
5. **Metadata providers before the plugin system.** If R1 needs cover art or
   MusicBrainz lookups, *recommendation:* build them as first-party
   providers that use the egress client with fixed host allowlists, and
   ship the WebAssembly plugin system later (SEC-HIS-057). *Trade-off:* this
   bends record 2's "lookups belong in plugins" for one release, in
   exchange for a much smaller R1 attack surface.
6. **Stable random IDs versus a rebuildable cache.** Random IDs
   (SEC-HIS-012) must survive a cache rebuild, or history and shares break.
   *Recommendation:* a new architecture record that puts the ID map (file
   fingerprint to ID) in the durable store with history and identity.
   *Trade-off:* more data that must be backed up.
7. **Public share links in R1.** *Recommendation:* ship them off by default
   per server, with a 7-day default expiry, revocation that takes effect on
   the next request (SEC-HIS-042) and no link previews. *Trade-off:* a
   popular feature, against an anonymous surface with a history of bugs at
   Navidrome.
8. **Disclosure policy.** *Recommendation:* GitHub private reporting (as
   SECURITY.md already says), CVE IDs through GitHub's CNA, a 90-day maximum
   embargo, credit for reporters, and a published supported-versions table
   from R1. *Trade-off:* the commitment of maintainer time to triage, which
   Navidrome's 19 advisories in ten days show can be heavy.
9. **DLNA and SSDP.** *Recommendation:* not before a dedicated architecture
   record. If ever added, make them read-only, per-library and off by
   default. *Trade-off:* some TVs only speak DLNA, but DLNA has no
   authentication at all.
10. **Automatic updates.** *Recommendation:* no self-updater in R1. Use the
    advisory banner, plus documentation for container auto-update tools.
    Never add a remote disable (SEC-HIS-054). *Trade-off:* slower patching
    than Emby's plugin-pushed hotfix, in exchange for no central control
    and no update channel to steal.

## Sources

CVE records (cveawg.mitre.org API):

- https://cveawg.mitre.org/api/cve/CVE-2023-33193
- https://cveawg.mitre.org/api/cve/CVE-2021-25827
- https://cveawg.mitre.org/api/cve/CVE-2025-34158
- https://cveawg.mitre.org/api/cve/CVE-2020-5741
- https://cveawg.mitre.org/api/cve/CVE-2023-49096
- https://cveawg.mitre.org/api/cve/CVE-2023-30627
- https://cveawg.mitre.org/api/cve/CVE-2026-31852
- https://cveawg.mitre.org/api/cve/CVE-2026-30975
- https://cveawg.mitre.org/api/cve/CVE-2026-44183
- https://cveawg.mitre.org/api/cve/CVE-2025-58761
- https://cveawg.mitre.org/api/cve/CVE-2025-64113
- https://cveawg.mitre.org/api/cve/CVE-2025-64325
- https://cveawg.mitre.org/api/cve/CVE-2020-26948
- https://cveawg.mitre.org/api/cve/CVE-2017-8314
- https://cveawg.mitre.org/api/cve/CVE-2017-5982
- https://cveawg.mitre.org/api/cve/CVE-2018-13415
- https://cveawg.mitre.org/api/cve/CVE-2017-9413
- https://cveawg.mitre.org/api/cve/CVE-2018-20228
- https://cveawg.mitre.org/api/cve/CVE-2019-10907
- https://cveawg.mitre.org/api/cve/CVE-2019-10908

Project and vendor advisories:

- https://github.com/jellyfin/jellyfin/security/advisories (pages 1 to 3)
- https://github.com/jellyfin/jellyfin/security/advisories/GHSA-j2hf-x4q5-47j3
- https://github.com/jellyfin/jellyfin/security/advisories/GHSA-8fw7-f233-ffr8
- https://github.com/jellyfin/jellyfin/security/advisories/GHSA-jh22-fw8w-2v9x
- https://github.com/jellyfin/jellyfin/security/advisories/GHSA-2c3c-r7gp-q32m
- https://github.com/jellyfin/jellyfin/security/advisories/GHSA-qcmf-gmhm-rfv9
- https://github.com/jellyfin/jellyfin/security/advisories/GHSA-6828-c7cx-hvqm
- https://github.com/jellyfin/jellyfin/security/advisories/GHSA-4vx8-xhc9-qg6x
- https://github.com/jellyfin/jellyfin/security/advisories/GHSA-9x85-gx46-6522
- https://github.com/jellyfin/jellyfin/security/advisories/GHSA-cf3c-8m59-2vhx
- https://github.com/jellyfin/jellyfin/security/advisories/GHSA-hw73-62vp-9gxf
- https://github.com/jellyfin/jellyfin/security/advisories/GHSA-f47c-m7gr-q92j
- https://github.com/jellyfin/jellyfin/security/advisories/GHSA-wwwm-px48-fpvq
- https://github.com/jellyfin/jellyfin/security/advisories/GHSA-fv79-gmhx-xh2v
- https://github.com/jellyfin/jellyfin/security/advisories/GHSA-9p5f-5x8v-x65m
- https://github.com/jellyfin/jellyfin/security/advisories/GHSA-vcmh-9wx9-rfqh
- https://github.com/jellyfin/jellyfin/security/advisories/GHSA-wg4c-c9g9-rxhx
- https://github.com/jellyfin/jellyfin/security/advisories/GHSA-rgjw-4fwc-9v96
- https://github.com/jellyfin/jellyfin/security/advisories/GHSA-rr9h-w522-cvmr
- https://github.com/jellyfin/jellyfin/issues/5415
- https://github.com/jellyfin/jellyfin/releases/tag/v12.0
- https://github.com/navidrome/navidrome/security/advisories (pages 1 to 3)
- https://github.com/navidrome/navidrome/security/advisories/GHSA-wq59-4q6r-635r
- https://github.com/navidrome/navidrome/security/advisories/GHSA-c3p4-vm8f-386p
- https://github.com/navidrome/navidrome/security/advisories/GHSA-rh3r-8pxm-hg4w
- https://github.com/navidrome/navidrome/security/advisories/GHSA-hrr4-3wgr-68x3
- https://github.com/navidrome/navidrome/security/advisories/GHSA-p994-r776-mw52
- https://github.com/navidrome/navidrome/security/advisories/GHSA-82gh-4ggp-gfg5
- https://github.com/navidrome/navidrome/security/advisories/GHSA-wp9c-pw66-c6j2
- https://github.com/navidrome/navidrome/security/advisories/GHSA-r5qr-m328-qcf4
- https://github.com/navidrome/navidrome/security/advisories/GHSA-8hjf-6h34-82hr
- https://github.com/navidrome/navidrome/security/advisories/GHSA-vwq6-xrw5-phpg
- https://github.com/navidrome/navidrome/security/advisories/GHSA-pr2j-mfc8-qjcc
- https://github.com/navidrome/navidrome/security/advisories/GHSA-f295-6wp9-qqfg
- https://github.com/navidrome/navidrome/security/advisories/GHSA-8jrh-w926-8rvw
- https://github.com/navidrome/navidrome/security/advisories/GHSA-4p3r-6362-833w
- https://github.com/navidrome/navidrome/security/advisories/GHSA-xwx7-p63r-2rj8
- https://github.com/navidrome/navidrome/security/advisories/GHSA-58vj-cv5w-v4v6
- https://github.com/navidrome/navidrome/security/advisories/GHSA-f238-rggp-82m3
- https://github.com/navidrome/navidrome/security/advisories/GHSA-4jrx-5w4h-3gpm
- https://www.navidrome.org/docs/usage/admin/security/
- https://github.com/EmbySupport/security/security/advisories/GHSA-fffj-6fr6-3fgf
- https://github.com/EmbySupport/Emby.Security/security/advisories/GHSA-95fv-5gfj-2r84
- https://emby.media/support/articles/advisory-23-05.html
- https://github.com/MediaBrowser/Emby/issues/3784
- https://github.com/Sonarr/Sonarr/security/advisories/GHSA-h5qx-5hjf-7c9r
- https://forums.sonarr.tv/t/authenication-required/31085
- https://github.com/Tautulli/Tautulli/security/advisories (pages 1 and 2)
- https://github.com/Tautulli/Tautulli/security/advisories/GHSA-68qx-mcf5-3jcp
- https://github.com/immich-app/immich/security/advisories
- https://github.com/immich-app/immich/security/advisories/GHSA-q89f-h332-8q2h
- https://github.com/immich-app/immich/security/advisories/GHSA-8244-8vpr-vp9c
- https://github.com/immich-app/immich/security/advisories/GHSA-hfvf-5c8x-8rc4
- https://github.com/lufinkey/vulnerability-research/tree/main/CVE-2025-34158
- https://www.tenable.com/security/research/tra-2020-32
- https://www.cisa.gov/known-exploited-vulnerabilities-catalog?field_cve=CVE-2020-5741
- https://support.plex.tv/articles/204059436-finding-an-authentication-token-x-plex-token/
- https://forums.plex.tv/t/860206.json
- https://www.netscout.com/blog/asert/plex-media-ssdp-pmssdp-reflectionamplification-ddos-attack
- https://research.checkpoint.com/2017/hacked-translation-directors-cut-full-technical-details/
- http://www.subsonic.org/pages/api.jsp
- https://opensubsonic.netlify.app/docs/extensions/apikeyauth/

Reporting (secondary sources, used where no primary source could be read):

- https://www.bleepingcomputer.com/news/security/emby-shuts-down-user-media-servers-hacked-in-recent-attack/
- https://www.helpnetsecurity.com/2025/08/27/plex-media-server-cve-2025-34158-attack/
- https://www.runzero.com/blog/plex/
- https://www.theregister.com/2025/09/09/plex_breach/
- https://securityaffairs.com/134814/data-breach/plex-data-breach.html (2022 Plex breach; Plex's own notice was not fetched)
- https://therecord.media/plex-unaware-of-vulnerabilities-lastpass-hacked/ and https://thehackernews.com/2023/03/lastpass-hack-engineers-failure-to.html (LastPass and Plex; LastPass's own post did not render for automated access)
- https://www.darkreading.com/cyberattacks-data-breaches/data-on-400k-kodi-forum-members-stolen-and-put-up-for-sale (Kodi's own announcement returned 403)
- https://blog.eset.ie/2018/09/13/kodi-add-ons-launch-cryptomining-campaign/
- https://www.bleepingcomputer.com/news/security/plex-media-servers-actively-abused-to-amplify-ddos-attacks/

Standards:

- OWASP ASVS 5.0.0: https://github.com/OWASP/ASVS/tree/v5.0.0/5.0/en and https://owasp.org/www-project-application-security-verification-standard/
- OWASP Top 10:2025: https://top10.owasp.org/2025
- OWASP API Security Top 10 2023: https://api-security.owasp.org/
- NIST SP 800-63B-4 (July 2025): https://csrc.nist.gov/pubs/sp/800/63/b/4/final and https://nvlpubs.nist.gov/nistpubs/SpecialPublications/NIST.SP.800-63B-4.pdf
- NIST SP 800-218 v1.1: https://nvlpubs.nist.gov/nistpubs/SpecialPublications/NIST.SP.800-218.pdf; Rev. 1 initial public draft: https://csrc.nist.gov/pubs/sp/800/218/r1/ipd
- RFC 6762 (Multicast DNS): https://www.rfc-editor.org/rfc/rfc6762.txt
- RFC 7239 (Forwarded HTTP Extension): https://www.rfc-editor.org/rfc/rfc7239.txt
- RFC 6266 (Content-Disposition in HTTP): cited by ASVS 5.0 5.4.2
