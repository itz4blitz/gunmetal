# Client and device security

Written 2026-10-02. Web tools were available and used. The shared web-search
budget ran out part way through, so later facts were checked by fetching
primary pages directly (OWASP source files, NIST pages, RFCs, platform
documentation, CVE records). Anything that could not be confirmed from a
source is marked "(unverified)". Facts that only appeared in search-result
summaries, without the page being opened, are marked as such in Sources.

Scope: the React Native apps on phones, tablets and TVs, the web client, the
desktop shell, the native libmpv player module, and what happens to data on
a device that is lost, stolen or shared by a household. Server-side
authentication design, the transport, server-side media parsing and the
release pipeline have their own documents in `docs/security/`; this file
covers only the parts of them that a client must get right.

Standard versions used throughout: OWASP MASVS v2.1.0 (the latest MASVS
release; MASTG v2.0.0 and MASWE v1.0.0 were reported as released in 2026),
OWASP ASVS 5.0.0 (references below are written "ASVS 3.4.3" for requirement
3.4.3 of 5.0.0), OWASP Top 10:2025, OWASP Mobile Top 10 2024, OWASP API
Security Top 10 2023, NIST SP 800-63B-4 (final, 2025), NIST SP 800-218 SSDF
v1.1 (the v1.2 revision was still an initial public draft when checked), and
the RFCs named in each row.

## Summary

Gunmetal's clients sit between two kinds of untrusted input. Everything the
server sends them started life as an untrusted file: tags, lyrics, artwork,
subtitles and embedded fonts written by whoever made the file, plus playlist
names, profile names and device names typed by other people. And the server
itself must treat every client as untrusted, because the clients are open
source, run on devices the user owns, and can be modified. Every decision
below follows from holding both of those lines at once.

**The web client (R1) is the first exposed surface, so it gets the strictest
rules first.** All library and user data is rendered as text, never as HTML;
the page ships with a strict Content Security Policy and Trusted Types
enforcement; the session lives only in a `__Host-` HttpOnly cookie, never in
script-readable storage; and security-changing admin actions need a fresh,
user-verified passkey assertion. That combination breaks the exact chain that
turned a Jellyfin device-name XSS into remote code execution in 2023
(CVE-2023-30627). The web client's code is embedded in the signed server
release and makes no request to any other origin, which also keeps the
household's library and IP address away from CDNs and metadata hosts. On
sign-out, account switch or remote revocation the browser is wiped with
`Clear-Site-Data` plus explicit client clean-up, and a "personal or shared
computer?" choice at sign-in decides whether anything is stored at all.

**Native apps (R2) hold a hardware-backed, non-exportable device key and
nothing reusable in plain storage.** Each install generates a P-256 key in
the Secure Enclave, StrongBox or TEE, reports its security level at
enrolment, and proves possession of it on every token refresh, so a copied
token or backup is useless on another device. The iroh transport key, which
must exist in memory as raw Ed25519 bytes, is stored wrapped by a keystore
key and is bound to the device record by a signature from the hardware key.
Devices fall into two server-enforced classes. Personal devices (phones and
computers with a lock screen) can approve other devices and perform admin
actions after local user verification. Limited devices (TVs, consoles,
shared browsers) can browse, play and download, but cannot do anything that
changes security. TV sign-in is by QR code approved on a phone that shows a
matching code, the requester's network and an expiry, because device-code
phishing is a proven attack (Storm-2372, 2025). Offline downloads play under
a device-bound grant that expires after 30 days without contact. Remote
revocation from any personal device wipes credentials, the library copy and
downloads on the lost device's next contact. We are honest that a grant
cannot stop someone with root access from reading files they already hold;
the real protections are OS encryption, the lock screen and server-side
revocation.

**The native player is the most dangerous code we ship, and it is
contained in layers.** libmpv, FFmpeg, libass and FreeType are large C code
bases that parse hostile media. Crafted subtitles took over VLC, Kodi and
others in 2017; an exploited FreeType font bug appeared in 2025; a Stagefright
MP4 bug in 2015 put Android's media parser on every attacker's list. Our
player never receives a path or URL, only bytes through an application
callback for one item. It is built from an allow-list of demuxers and
decoders with no network protocols, scripting, youtube-dl hook or
reference-following. By default it is only handed containers that our
memory-safe Rust parser already accepted on the server. It runs in a
separate, permission-less process wherever the platform allows (desktop now,
Android after a feasibility spike). A CI job plays a malformed-media corpus
through the shipped build under AddressSanitizer. Fixes to the media stack
ship within 14 days, or 7 if the bug is known to be exploited.

**Several things we deliberately do not do.** No web views anywhere in the
native apps; sign-in through an identity provider uses the system browser
with PKCE. No over-the-air JavaScript updates: the code that runs is the
code that was signed. No custom URL scheme carries a secret; invites arrive
through verified App Links and Universal Links with the secret in the URL
fragment. No certificate click-through: a self-signed server is trusted only
through a fingerprint delivered with the invite, and the server's identity
key is pinned at enrolment and re-proven on every connection. No root or
jailbreak detection (MASVS-RESILIENCE is out of scope by design): the user
owns the device, and every restriction is enforced by the server. No
analytics, crash-reporting or advertising SDKs. On a shared family TV, each
profile's data is partitioned, the profile picker shows no viewing data,
profile PINs are checked only by the server with rate limits, and
publishing to OS-wide surfaces (Watch Next, Top Shelf, Spotlight) is off by
default.

The file has 63 live requirements: 22 R1, 35 R2, 1 R3 and 5 Later, plus 9 withdrawn rows kept so their IDs stay stable. The R1 ones are R1 because they apply to the web
client, the server's session and device model, and shared rules that the R1
server must be built around.

## Threats

Likelihood is for a typical self-hosted install exposed to family, friends
and the internet, assuming no mitigation.

| ID | Threat | Who (the attacker or failure) | Impact | Likelihood | Mitigated by (requirement IDs) |
|---|---|---|---|---|---|
| T-CLI-01 | Stored XSS in the web client through library metadata: tags, lyrics, comments, artwork file names, playlist and profile names, synced from files or typed by other users | Anyone who can put a file into a watched folder (a downloaded album with crafted tags), another user on the server, a metadata provider | Account takeover of every user who views the item; the page acts as the victim | High | SEC-CLI-001, 002, 003, 006, 014, 021 |
| T-CLI-02 | Active content (SVG, HTML disguised as an image, polyglots) served as artwork or other user content from the client's own origin, then opened directly | Same as T-CLI-01 | Script runs with the web client's origin, defeating the CSP on the main page | Medium | SEC-CLI-003, 004, 005 |
| T-CLI-03 | XSS in admin views chaining to server takeover, as in Jellyfin CVE-2023-30627 (device ID in a request header, rendered unescaped on the admin devices page, then token theft, path traversal and code execution) | Any client that can register a session, including an unauthenticated one if device names are accepted before sign-in | Full server compromise, then every household device | Medium | SEC-CLI-001, 003, 006, 014, 024 |
| T-CLI-04 | Theft of session or refresh tokens from script-readable browser storage | XSS (T-CLI-01), a malicious browser extension, a shared computer | Persistent account access from elsewhere | High | SEC-CLI-006, 026, 003 |
| T-CLI-05 | Cross-site request forgery and clickjacking against the signed-in web client | Any web page the user visits | Unwanted actions: shares, invites, deletions | Medium | SEC-CLI-003, 004, 007 |
| T-CLI-06 | Credentials or session cookies sent in cleartext on a home or public network | Anyone on the same Wi-Fi, a compromised router | Account takeover | Medium | SEC-CLI-008, 042 |
| T-CLI-07 | Data left behind on a shared browser or device after sign-out or a user switch: library copy, artwork cache, service-worker caches, history | The next person to use the computer or tablet | Exposure of listening and viewing history | High | SEC-CLI-009, 010, 063, 064 |
| T-CLI-08 | Out-of-date client code keeps running after a security fix: long-lived tabs, service-worker caches, un-updated mobile apps, TV apps that never update | Failure, exploited by anyone who knows the fixed bug | Known vulnerabilities stay exploitable | High | SEC-CLI-011, 065, 066 |
| T-CLI-09 | Privacy leak to third parties: the client fetches artwork, fonts or scripts from CDNs or metadata hosts, or bundles analytics and crash SDKs | CDN and metadata operators, SDK vendors, anyone they share with | IP address, library contents and habits leave the house | High | SEC-CLI-012, 027 |
| T-CLI-10 | Secrets leaked through URLs: invite codes in query strings, tokens in server logs, browser history or `Referer` headers | Log readers, other users of the browser, linked sites | Invite or session hijack | Medium | SEC-CLI-004, 013, 060 |
| T-CLI-11 | Supply-chain compromise of client dependencies or developer tooling (the September 2025 npm `chalk`/`debug` compromise and the Shai-Hulud worm; the Metro dev server command injection CVE-2025-11953) | Attackers who phish package maintainers or scan for exposed dev servers | Malicious code in a release; compromised developer machines and signing keys | Medium | SEC-CLI-016, 017, 018, 019, 046 |
| T-CLI-12 | Malicious code pushed through an over-the-air update channel, or an update service that disappears (App Center and its hosted CodePush were retired on 2025-03-31) | Whoever controls or compromises the update channel | Code execution in every installed client at once | Medium if OTA is used | SEC-CLI-045, 046, 068 |
| T-CLI-13 | Extraction of credentials from native storage, cloud backups or device-to-device transfer | Malware with root, a person with the unlocked phone and a cable, someone with the user's cloud account | Account access from another device | Medium | SEC-CLI-030, 031, 032, 033 |
| T-CLI-14 | Replay of a stolen token, or a cloned app install, from another device | Same as T-CLI-13 | Silent, persistent access | Medium | SEC-CLI-022, 031, 032, 034 |
| T-CLI-15 | A lost or stolen phone, tablet or laptop is used to browse, stream, read history or play downloads | Thief, finder | Privacy loss; misuse of the account | High | SEC-CLI-022, 024, 035, 036, 037, 059 |
| T-CLI-16 | Device-pairing phishing: an attacker shows a TV code or QR and persuades a household member to approve the attacker's "TV" | Remote attacker by message or email, as in Storm-2372 | A persistent device on the victim's account | Medium | SEC-CLI-023, 024, 038 |
| T-CLI-17 | Deep-link abuse: another app claims the same custom scheme to receive invites or sign-in codes, or a crafted link triggers an action (join a hostile server, sign out, approve) | Malicious app on the same phone; anyone who can send a link | Credential capture, unwanted state changes | Medium | SEC-CLI-013, 025, 039, 040 |
| T-CLI-18 | Embedded web view abuse: identity-provider sign-in inside a web view (phishable, the app can read the password), or a JavaScript bridge exposed to remote content | Malicious or compromised page; a hostile identity provider page | Credential theft; native code reached from web content | Medium if web views are used | SEC-CLI-040, 041 |
| T-CLI-19 | Machine-in-the-middle through a user-installed CA, a "trust this certificate" click-through, an impostor server or a rogue reverse proxy | Hostile network operator, corporate proxy, attacker with a mis-issued certificate | Token theft, content tampering | Medium | SEC-CLI-042, 043 |
| T-CLI-20 | A malicious or compromised server (a friend's server the user joined by invite) attacks the client: hostile payloads, references that make the client fetch other hosts, attempts to read data from the user's other servers | Operator of any server the client is enrolled with | Cross-server data theft, client crash, SSRF from the client | Medium | SEC-CLI-001, 021, 044 |
| T-CLI-21 | Memory-corruption exploit in libmpv, FFmpeg, libass, FreeType or bundled image decoders through a crafted media file, subtitle or embedded font (Stagefright CVE-2015-1538, FreeType CVE-2025-27363, libwebp CVE-2023-4863, Check Point's 2017 subtitle attacks) | Whoever made a file that ends up in the library; subtitle sites | Code execution inside the app with access to credentials and the library | Medium | SEC-CLI-047 to 053 |
| T-CLI-22 | The player is steered by the file into opening local files or the network: HLS `concat`/`subfile` protocols (CVE-2016-1897, CVE-2016-1898), mpv's youtube-dl hook (CVE-2018-6360), MOV references, Matroska ordered chapters | Same as T-CLI-21 | Local file disclosure, SSRF from the user's device, arbitrary library loading | Medium | SEC-CLI-047, 048, 049 |
| T-CLI-23 | Other apps on the phone enumerate the library through exported components or a media browser service, or hijack implicit intents | Any installed app | Library and habit disclosure | Medium | SEC-CLI-054, 055 |
| T-CLI-24 | Leaks through the clipboard, screenshots and the app switcher, screen overlays, notifications, keyboard caches and OS-wide surfaces | Other apps, people nearby, the OS launcher's vendor | Invite and pairing codes leaked; history shown to others | Medium | SEC-CLI-028, 056, 057, 058, 061, 062 |
| T-CLI-25 | Household privacy on a shared TV or tablet: one member sees another's history, queue or search; a child switches into a parent's profile; PIN guessing; a previous profile's data lingers | Other household members, including children and teenagers | Privacy loss, unsuitable content reaching children | High | SEC-CLI-020, 024, 029, 061, 063, 064 |
| T-CLI-26 | A modified client ignores restrictions it was told to apply: kids limits, download permission, device-class limits | Any user who rebuilds or patches the open-source client | Policy bypass | High | SEC-CLI-015, 020, 024 |
| T-CLI-27 | Live TV sources attack clients: M3U names, group titles, logo URLs and stream URLs are provider-controlled (R3) | IPTV providers, anyone who edits a shared playlist | XSS, SSRF, IP leaks to providers | Medium | SEC-CLI-001, 067 |
| T-CLI-28 | XSS in a desktop shell becomes native code execution through Node integration or an over-broad IPC bridge (Signal Desktop CVE-2018-10994 was an XSS in an Electron app) | Same as T-CLI-01 | Full compromise of the user's computer | Medium (Later) | SEC-CLI-003, 069 |
| T-CLI-29 | Tokens given to a cast receiver or another device for playback are reused for other content or after the session | Owner of the receiving device, network observers | Unauthorised playback | Low | SEC-CLI-071 |
| T-CLI-30 | Downloads written to removable or shared storage are read by other apps, or by whoever finds the SD card | Other apps; finder of the card | Library and history disclosure | Low | SEC-CLI-035, 072 |

## Requirements

"Integration test" below always means a test against a real SQLite database
and a real server process, never a mocked database. "Instrumented test"
means an on-device test run on an emulator in CI. Every requirement is a
test or a CI check in the gate unless it says manual review.

| ID | Requirement | Standards | Release | Verified by |
|---|---|---|---|---|
| SEC-CLI-001 | The web and native clients must render every string that originates from media files, metadata providers, other users, other devices or a server (titles, artists, tags, lyrics, comments, overviews, playlist, profile and device names, error messages) as text, and the web client must contain no use of `dangerouslySetInnerHTML`, `innerHTML`, `outerHTML`, `insertAdjacentHTML`, `document.write`, `eval`, `new Function` or string arguments to timers. | ASVS 1.2.1, 1.3.2, 3.2.2; Top 10:2025 A05; MASVS-CODE-4; MASWE-0050; CWE-79, CWE-95 | R1 | CI lint (`react/no-danger`, `no-eval`, `no-implied-eval`, `no-restricted-properties`) as errors; component tests that render the shared "hostile metadata" fixture (every text field holds an XSS payload corpus) and assert the literal text appears and no element or attribute was created; Playwright test over the same fixture asserting a sentinel global is never set |
| SEC-CLI-002 | Any URL taken from metadata or user input must be rendered as a link only if it parses as an absolute `https:` URL (or matches a fixed internal route), with `rel="noopener noreferrer"`; anything else must be shown as plain text. | ASVS 1.2.2, 3.7.2; CWE-79, CWE-601 | R1 | Property test of the URL filter in the core (random strings, mixed-case `javascript:`, `data:`, `vbscript:`, embedded whitespace and control characters, IDN) asserting only `https:` passes; component test |
| SEC-CLI-003 | **Withdrawn 2026-10-02: merged into SEC-API-044.** The golden-policy and Trusted Types checks run under SEC-API-044. | ASVS 3.2.2, 3.4.3, 3.4.6; Top 10:2025 A05, A02; CWE-79, CWE-1021 | Withdrawn | Proved by the tests of SEC-API-044 |
| SEC-CLI-004 | Every server response to clients must carry `X-Content-Type-Options: nosniff`; HTML responses must carry `Referrer-Policy: no-referrer` and `Cross-Origin-Opener-Policy: same-origin`; media, artwork and API responses must carry `Cross-Origin-Resource-Policy: same-origin`; authenticated API responses must carry `Cache-Control: no-store`; and responses served over HTTPS on a domain name must carry HSTS with a `max-age` of at least one year. | ASVS 3.4.1, 3.4.4, 3.4.5, 3.4.8, 3.5.8, 14.3.2; CWE-524, CWE-693 | R1 | Integration test per response class against a golden header table |
| SEC-CLI-005 | Artwork and other user-derived content must reach clients only as an allow-listed raster type (JPEG, PNG, WebP) decided by the server from the content, never from a file extension or tag; SVG, HTML and every other active type must never be served from the client's origin; and every such response must carry `Content-Security-Policy: sandbox; default-src 'none'`. | ASVS 1.3.4, 3.2.1, 5.2.2; CWE-79, CWE-434 | R1 | Integration test serving a corpus (SVG with script, HTML named `.jpg`, JPEG/HTML polyglot, GIFAR-style polyglot) asserting each is refused or served with the safe type and headers; Playwright test that opening an artwork URL directly runs no script |
| SEC-CLI-006 | **Withdrawn 2026-10-02: merged into SEC-API-032, SEC-TM-058, SEC-IAM-017.** Cookie attributes (SEC-API-032); no credential in web storage (SEC-TM-058, SEC-IAM-017). | ASVS 3.3.1, 3.3.2, 3.3.3, 3.3.4, 14.3.3; Top 10:2025 A07; CWE-1004, CWE-614, CWE-1275, CWE-922 | Withdrawn | Proved by the tests of SEC-API-032, SEC-TM-058, SEC-IAM-017 |
| SEC-CLI-007 | The server must reject any state-changing request from a browser whose `Sec-Fetch-Site` is not `same-origin` (or, when that header is absent, whose `Origin` does not match), and no `GET` route may change state. | ASVS 3.5.1, 3.5.3; Top 10:2025 A01; CWE-352 | R1 | Integration test that enumerates the route table and replays every state-changing route with cross-site headers, asserting rejection; route-table test that no `GET` route is marked state-changing |
| SEC-CLI-008 | **Withdrawn 2026-10-02: merged into SEC-NET-001.** Including the help page that explains how to reach the secure address. | ASVS 12.1.1, 12.2.1; Top 10:2025 A04; CWE-319, CWE-523 | Withdrawn | Proved by the tests of SEC-NET-001 |
| SEC-CLI-009 | On sign-out, account or profile switch, and on receiving a "session revoked" response, the web client must delete that account's data from IndexedDB, OPFS, Cache Storage, `localStorage`, `sessionStorage` and memory and unregister any service worker it registered, and the sign-out response must carry `Clear-Site-Data: "cache", "cookies", "storage"`. | ASVS 14.3.1, 7.4.1; MASWE-0024; CWE-226, CWE-613 | R1 | Playwright test: fill the library cache, sign out (and separately, revoke from a second session), then assert every store is empty, no service worker is registered and the back button shows no data; integration test of the header |
| SEC-CLI-010 | At sign-in the web client must ask, as one plain question, whether the browser is personal or shared; in shared mode it must keep library data and artwork only in memory, use a cookie with no `Max-Age` or `Expires`, and end the session after 30 minutes of inactivity; in personal mode persistent storage is allowed subject to SEC-CLI-009. | NIST SP 800-63B-4 §2.1.3, §5.1; ASVS 7.3.1, 7.3.2, 14.3.3; CWE-524 | R1 | Playwright tests for both modes asserting storage contents and cookie attributes; integration test of the idle timeout with a controllable clock |
| SEC-CLI-011 | The server must reject API calls from a web client bundle whose build identifier differs from its own with a typed "reload required" error, the client must reload on that error, and any service worker must never serve a bundle older than the running server's. | ASVS 15.2.1; MASVS-CODE-2; MASWE-0043; Top 10:2025 A08; CWE-1104 | R1 | Integration test with a mismatched build header; Playwright test that simulates a server upgrade under an open tab and asserts a reload to the new build |
| SEC-CLI-012 | The web client's scripts, styles, fonts and WASM must ship inside the signed server release and be served only by that server, and the client must not load code or fonts from any other origin, including gunmetal.tv and CDNs. | ASVS 3.6.1, 15.2.4; Top 10:2025 A03, A08; CWE-829, CWE-830 | R1 | CSP golden test (SEC-API-044); Playwright harness that records every request across the whole UI suite and fails on any origin other than the server under test |
| SEC-CLI-013 | Any secret carried by a link into a client (invite, pairing, one-time sign-in) must travel in the URL fragment, must be removed from the address bar and history (`history.replaceState`) before any network request, and must not be redeemed without a confirmation screen that names the server and says what will happen. | ASVS 14.2.1, 3.4.5; CWE-598, CWE-200 | R1 | Playwright test asserting `location.href` and the history entries hold no secret after load, the server access log holds no secret, and nothing is redeemed without the click |
| SEC-CLI-014 | **Withdrawn 2026-10-02: merged into SEC-IAM-041.** Route tags and the fresh-uv list. | ASVS 7.5.1, 7.5.3, 8.4.2; NIST SP 800-63B-4 §2.2.3; Top 10:2025 A07; CWE-306 | Withdrawn | Proved by the tests of SEC-IAM-041 |
| SEC-CLI-015 | Every restriction a client applies or displays (profile content limits, download permission, device-class limits, admin visibility) must also be enforced by the server on every request, so that removing the client-side check grants nothing. | ASVS 8.2.1, 8.2.2, 8.3.1; Top 10:2025 A01; API1:2023, API5:2023; CWE-602, CWE-862 | R1 | Integration tests that call each restricted route directly, without the client, as each profile and device class and assert refusal |
| SEC-CLI-016 | No client build (web bundle, APK, IPA, desktop package) may contain an API key, signing key, server secret or shared credential. | ASVS 13.3.1; Mobile Top 10 2024 M1; MASWE-0004; CWE-798, CWE-321 | R1 | CI secret scanner run over every built artefact (unpacked), failing on any finding |
| SEC-CLI-017 | Every client build must produce a CycloneDX SBOM that includes native libraries, and CI must fail when a shipped dependency has a known high or critical vulnerability unless a written, dated exception exists. | ASVS 15.1.1, 15.1.2, 15.2.1; MASVS-CODE-3; MASWE-0044; Top 10:2025 A03; SSDF PW.4, RV.1; CWE-1395 | R1 | CI job: SBOM generation plus an OSV lookup on every build and daily on `main` |
| SEC-CLI-018 | CI must install JavaScript dependencies only from the committed lockfile with install-time lifecycle scripts disabled except for a reviewed allow-list, and any change to the lockfile must name each added or upgraded package in review. | ASVS 15.2.4; Top 10:2025 A03; Mobile Top 10 2024 M2; SSDF PS.1, PW.4; CWE-829 | R1 | CI check that the install command uses the lockfile and disables scripts; CI check that fails when a package outside the allow-list declares an install script |
| SEC-CLI-019 | Development servers (Metro, Expo, Vite or similar) must bind to loopback in the committed configuration, the React Native Community CLI, if used, must be 20.0.0 or later, and release builds must exclude dev menus, bundler connections, remote debugging and inspector hooks. | ASVS 13.4.2, 15.2.3; CWE-489, CWE-78 | R1 | CI check of the committed config and lockfile versions; release-build test asserting `__DEV__` is false and no dev-support classes or bundler URLs are present (for example with `apkanalyzer` on Android builds) |
| SEC-CLI-020 | The library data the server sends to a device must contain only what the signed-in profile may access: never other users' or profiles' history, queues or search history, and nothing outside a restricted profile's limits, including artwork and search-index entries. | ASVS 8.2.2, 8.2.3, 14.2.6, 15.3.1; API3:2023; MASVS-PRIVACY-1; CWE-359, CWE-200 | R1 | Integration test with two users and a child profile asserting the exact sync payload for each, including that a restricted title's artwork and search terms are absent |
| SEC-CLI-021 | Clients must decode every server response with schema-validating decoders that bound total size, string length, nesting depth and collection length, and must turn any decoding failure into a typed error without crashing. | Top 10:2025 A10; MASWE-0050; CWE-20, CWE-400, CWE-502 | R1 | Property tests and a `cargo-fuzz` target for the core's protocol decoder (no panic, bounded memory); web test that feeds malformed responses through the network layer and asserts a typed error screen |
| SEC-CLI-022 | **Withdrawn 2026-10-02: merged into SEC-IAM-042, SEC-TM-028.** Device and session lists (SEC-IAM-042); revocation takes effect on the next request (SEC-TM-028). | ASVS 7.4.1, 7.4.3, 7.4.5, 7.5.2, 6.5.6; NIST SP 800-63B-4 §5; CWE-613 | Withdrawn | Proved by the tests of SEC-IAM-042, SEC-TM-028 |
| SEC-CLI-023 | **Withdrawn 2026-10-02: merged into SEC-IAM-098, SEC-OPS-032.** Notices to the account's devices and owner alerts. | ASVS 6.3.5, 6.3.7; CWE-778 | Withdrawn | Proved by the tests of SEC-IAM-098, SEC-OPS-032 |
| SEC-CLI-024 | The server must assign every enrolled device a class, personal (a device that can perform local user verification bound to its key, or a browser in personal mode with a user-verifying passkey) or limited (TVs, consoles, web-build TVs, browsers in shared mode, devices without a secure lock screen), and must refuse device approval, account-security changes and admin operations from limited devices whatever the client shows. | ASVS 8.1.3, 8.2.1, 8.3.1; NIST SP 800-63B-4 §2.1, §2.2; CWE-602, CWE-285 | R1 | Integration test that enumerates security-changing routes and calls each as a limited device, asserting refusal |
| SEC-CLI-025 | All inbound links and codes (deep links, verified web links, QR payloads, notification and launcher intents, URL fragments) must be parsed by one pure function in the core into a closed set of typed routes; unrecognised input must yield "not recognised", and no route may change state without a confirmation screen. | ASVS 1.1.1; MASVS-PLATFORM-1; MASWE-0029, MASWE-0050; CWE-20, CWE-939 | R1 | Unit tests per route; property tests and a `cargo-fuzz` target asserting no panic and that no output is state-changing without its confirm flag |
| SEC-CLI-026 | When the web client signs in through an OIDC provider, the server must act as a confidential client (backend for frontend) so that no OAuth access, refresh or ID token ever reaches browser JavaScript. | ASVS 10.1.1, 10.2.1, 10.4.6, 10.5.1; RFC 9700 §2.1.1; CWE-922 | R1 | Integration test of the full code flow with a test identity provider; Playwright test asserting no token in any script-readable place or URL |
| SEC-CLI-027 | No client may include analytics, advertising, attribution, crash-reporting or tracking SDKs, or contact any host other than the user's servers, configured relays and the project's identifier-free advisory manifest; any crash report must be opt-in and sent only to the user's own server. | ASVS 14.2.3; MASVS-PRIVACY-1, PRIVACY-2, PRIVACY-3; Mobile Top 10 2024 M6; CWE-359 | R1 | CI dependency deny-list (SDK package names and known endpoints); network capture during the web and Android end-to-end suites asserting only allowed hosts |
| SEC-CLI-028 | PIN, pairing-code and password fields must mask input and turn off autocorrect, suggestions and keyboard learning, and password fields must allow paste and password managers. | ASVS 6.2.6, 6.2.7; MASVS-STORAGE-2; CWE-549 | R1 | Component tests asserting `type`, `autocomplete`, `autocorrect`, `spellcheck` (web) and `secureTextEntry`, `autoCorrect`, `textContentType` (native) on every such field |
| SEC-CLI-029 | **Withdrawn 2026-10-02: merged into SEC-IAM-062.** One PIN rule. | ASVS 6.1.1, 6.3.1; Top 10:2025 A07; CWE-603, CWE-307 | Withdrawn | Proved by the tests of SEC-IAM-062 |
| SEC-CLI-030 | Native clients must keep every credential (device private key, tokens, transport key, server identity pins, grant keys) only in the platform credential store (Keychain on Apple platforms, Android Keystore-protected storage, or the desktop OS equivalent), never in AsyncStorage, unencrypted MMKV, SharedPreferences, SQLite or plain files. | MASVS-STORAGE-1, MASVS-CRYPTO-2; MASWE-0001, MASWE-0003; Mobile Top 10 2024 M1, M9; CWE-922, CWE-312 | R2 | Lint rule that only the credential module may use the secure-storage wrapper and that AsyncStorage keys come from an allow-list; instrumented test that signs in with a canary token and scans the app data directory and a debug backup for it |
| SEC-CLI-031 | Each native install must generate its device authentication key as a non-exportable P-256 key in secure hardware where present (Secure Enclave, StrongBox or TEE), must report the key's security level (with Android key attestation where available) at enrolment, and the server must record and display that level. | MASVS-CRYPTO-2; MASWE-0003; NIST SP 800-63B-4 §2.3 (non-exportable keys); ASVS 13.3.3; CWE-522 | R2 | Instrumented tests on reference emulator images asserting `KeyInfo` security level and non-exportability; property tests of attestation parsing in the core; integration test of storage and display |
| SEC-CLI-032 | The iroh transport secret key must be stored only encrypted under a non-exportable keystore key (or as a Keychain item of a "this device only" class), and the server must accept an iroh node ID only when it is bound to an active device by a signature from that device's hardware key. | MASVS-CRYPTO-2, MASVS-AUTH-1; CWE-311, CWE-287 | R2 | Unit tests of the wrap and unwrap round trip and of tamper detection; integration tests that an unbound node ID and a revoked device's node ID are refused |
| SEC-CLI-033 | Keychain items must use `kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly` or a stricter "this device only" class, and on Android the credential store, the library database and downloads must be excluded from both `cloud-backup` and `device-transfer` in `dataExtractionRules` (and from `fullBackupContent` on older versions). | MASVS-STORAGE-1, MASVS-STORAGE-2; MASWE-0006; CWE-312, CWE-530 | R2 | CI check that parses the backup-rules XML; unit test of the Keychain wrapper's attributes; instrumented test that a debug backup contains none of these files |
| SEC-CLI-034 | **Withdrawn 2026-10-02: merged into SEC-IAM-050.** Native access tokens last 10 minutes and are renewed by signing a fresh server challenge; there are no refresh tokens. | RFC 9700 §2.2.2; RFC 9449; ASVS 10.4.5, 10.3.5, 7.2.4; NIST SP 800-63B-4 §5.1; CWE-294 | Withdrawn | Proved by the tests of SEC-IAM-050 |
| SEC-CLI-035 | Native clients must keep the library database, artwork cache and downloads in app-private internal storage under OS data protection (on iOS the default "until first user authentication" class or stricter), and must not write them to shared storage, media collections or removable storage. | MASVS-STORAGE-1; MASWE-0002; CWE-922, CWE-276 | R2 | Instrumented test asserting every written path is under app-private directories (and the iOS file protection attribute, by manual review until Apple builds ship); static check banning MediaStore and shared-directory writes |
| SEC-CLI-036 | Downloads must play offline only under a server-signed grant bound to the device key with an expiry (default 30 days, admin range 1 to 90) that is renewed silently on contact. Grant age must be measured with the device's monotonic elapsed time since the last server contact, with wall-clock checks that tolerate the clock going back by up to 24 hours. Expiry must never interrupt a track or queue already playing (it applies at the next play), must not delete files, and its screen must say "Connect to your server once to keep listening offline" without looking like an error. | MASVS-AUTH-1; NIST SP 800-63B-4 §2.1.3; CWE-613 | R2 | Core property tests over random skew, monotonic and wall-clock combinations and DST and time-zone jumps, with literal expected outcomes; integration test of silent renewal |
| SEC-CLI-037 | Native clients must offer two separate actions: "Switch account", which keeps that account's partition encrypted under a key that is unusable until the account signs in again, and "Remove account from this device", which, like a revocation reported by the server, deletes the credentials and keystore entries, the library database, the artwork cache and all downloads before any other screen is shown. | ASVS 7.4.1, 14.3.1; MASWE-0024; CWE-226 | R2 | Instrumented end-to-end tests of both paths: after switching, the partition is ciphertext and the key is unusable; after removal or server revocation, the data directory is empty and the keystore aliases are gone |
| SEC-CLI-038 | **Withdrawn 2026-10-02: merged into SEC-IAM-056, SEC-IAM-058, SEC-IAM-060.** One pairing specification; the matching code is required whenever the approval is not proven local. | RFC 8628 §5.4, §5.5; ASVS 6.5.5, 6.6.2, 6.6.3; MASVS-AUTH-3; CWE-451, CWE-306 | Withdrawn | Proved by the tests of SEC-IAM-056, SEC-IAM-058, SEC-IAM-060 |
| SEC-CLI-039 | Native apps must receive external links only through verified Android App Links and iOS Universal Links on project-controlled domains (plus the OIDC redirect), and must not register any custom URL scheme that can carry a secret or trigger an action. | RFC 8252 §7.1, §7.2, §8.1; MASVS-PLATFORM-1; MASWE-0029; CWE-939 | R2 | CI check of the Android manifest (`autoVerify="true"`, `https` only) and iOS associated-domains entitlement; CI check that `assetlinks.json` and the Apple association file match the release signing identities |
| SEC-CLI-040 | Native apps must perform identity-provider sign-in only in the system browser (ASWebAuthenticationSession or Android Custom Tabs) with PKCE S256, `state` and `nonce`. | RFC 8252 §6, §8.12; RFC 7636; RFC 9700 §2.1.1; ASVS 10.2.1, 10.4.6, 10.5.1; CWE-352 | R2 | Integration test of the native flow against a test provider; unit tests that a response with a wrong `state` or `nonce` is refused |
| SEC-CLI-041 | Native release builds must contain no WebView component (react-native-webview, WKWebView, `android.webkit.WebView`) in project code; external pages must open in the system browser. | MASVS-PLATFORM-2; MASWE-0033, MASWE-0034, MASWE-0035; RFC 8252 §8.12; CWE-749, CWE-829 | R2 | CI dependency deny-list; Android Lint and a class scan of the APK for WebView use from project packages; symbol scan of the iOS build |
| SEC-CLI-042 | Native clients must not permit cleartext traffic, must use the platform trust store without user-added CAs and with hostname verification for HTTPS, must contain no code path that disables certificate validation, and must accept a self-signed or private-CA server only through an SPKI pin delivered out of band with the invite or QR, never through a click-through. | MASVS-NETWORK-1; MASWE-0026, MASWE-0027; ASVS 12.1.1, 12.3.2; Mobile Top 10 2024 M5; CWE-295, CWE-319 | R2 | CI checks of the Android network security config and iOS ATS keys; static rule forbidding custom trust managers, hostname verifiers and trust-all delegates; integration tests against servers with self-signed, wrong-host and expired certificates and an emulator with a user CA intercepting traffic, all refused |
| SEC-CLI-043 | At enrolment a client must pin the server's identity public key received out of band, and on every connection, over iroh or HTTPS, the server must sign a fresh client nonce of at least 128 bits with that key; on mismatch the client must fail closed with no "continue anyway", and a key change is accepted only through a continuity statement signed by the pinned key (SEC-NET-060, SEC-NET-062). | MASVS-NETWORK-2; MASWE-0028; ASVS 6.7.2; CWE-295, CWE-322 | R2 | Property tests of the handshake in the core; integration test with an impostor server holding a valid TLS certificate but a different identity key; the rotation-follow test of SEC-NET-062 |
| SEC-CLI-044 | A client enrolled with several servers must keep each server's credentials, library copy, caches and downloads in a separate partition, and must fetch any resource only from the server whose data referenced it, ignoring absolute URLs in server data. | MASVS-PRIVACY-1; ASVS 1.3.6; Top 10:2025 A01; CWE-918, CWE-668 | R2 | Integration test with two servers where one returns references to the other's resources and to arbitrary hosts, asserting no such request is made and no data crosses partitions |
| SEC-CLI-045 | Native clients must execute only JavaScript, bytecode and native code shipped inside the signed release package, and must include no mechanism that downloads executable code or bundles at runtime (CodePush, `expo-updates` or similar). | MASWE-0049; Top 10:2025 A08; App Review Guideline 2.5.2; Google Play Device and Network Abuse policy; CWE-494 | R2 | CI dependency deny-list; release-build check for update URLs; Android Lint for dynamic class loading; manual review |
| SEC-CLI-046 | Every client release must be signed with keys held in hardware or a key-management service rather than on CI disks or laptops (APK Signature Scheme v2 or later, with v3 key rotation available), and the release signing certificate fingerprints must be published in `SECURITY.md` and on gunmetal.tv. | SSDF PS.2, PS.3; Top 10:2025 A03, A08; Mobile Top 10 2024 M2; CWE-345 | R2 | CI check that `apksigner verify --print-certs` matches the published fingerprint; manual review of key custody at each release |
| SEC-CLI-047 | Native players must receive media only through an application stream callback (libmpv `stream_cb` or equivalent) for one opaque item, and libmpv must never be given a file path, a network URL or a playlist. | ASVS 15.2.5, 1.3.6; MASWE-0050; CWE-918, CWE-73, CWE-829 | R2 | Unit test that the player bridge rejects any target other than its own scheme; corpus test (HLS `concat`/`subfile` playlist, EDL, MOV reference, Matroska linked segment, HTML page with a `<video>` element) asserting no file open and no socket, observed by syscall tracing on Linux desktop and Android emulator |
| SEC-CLI-048 | The bundled FFmpeg and libmpv must be built from a reviewed allow-list (only demuxers, parsers and decoders needed for supported formats; no network protocols; no `concat`, `subfile`, `data`, HLS or DASH; no Lua or JavaScript scripting, youtube-dl hook, archive or disc support, or IPC server), and the player must start with `config=no`, `load-scripts=no`, `ytdl=no`, `access-references=no`, `ordered-chapters=no`, `sub-auto=no`, `audio-file-auto=no`, `cover-art-auto=no` and no IPC server. | ASVS 15.1.5, 15.2.5; Top 10:2025 A02; CWE-1188, CWE-250 | R2 | CI check that compares the FFmpeg configure line and the built library's protocol and demuxer lists with a golden allow-list; unit test reading every listed mpv option back after initialisation |
| SEC-CLI-049 | By default a native client must hand libmpv only containers that the server's core parser accepted at scan time; other containers must be played through the server's remux or sandboxed transcode path unless an admin allows direct play for that library. The admin problem list must show the affected files per library with their count and cost (for example "These 312 files will be converted when played") and offer the per-library choice there. | ASVS 15.2.5; Top 10:2025 A06; CWE-653 | R2 | Property tests of the core's playback decision engine (an unparsed container never yields direct play unless the library flag is set); integration test; component test of the notice |
| SEC-CLI-050 | Where the platform offers a lower-privilege process (desktop; Android `isolatedProcess` once the feasibility spike passes), libmpv must run in its own process with no network access and no access to credentials or the library database, receiving only a rendering surface and a byte pipe; on platforms without such isolation (iOS, tvOS) SEC-CLI-047 and 048 remain mandatory and hardware decoding must be preferred. | ASVS 15.2.5; MASVS-CODE-4; CWE-653, CWE-250 | R2 | Instrumented Android test that the player process runs under a different UID without network permission and that a socket attempt from a test hook fails; desktop integration test that the sandboxed player cannot open files or sockets; manual review on Apple platforms |
| SEC-CLI-051 | CI must play the malformed-media corpus (the core's fixtures, public FFmpeg and libass fuzzing regressions, crafted subtitles and fonts) through the exact shipped player build compiled with AddressSanitizer and UndefinedBehaviorSanitizer on Android and Linux desktop, and fail on any sanitizer report, hang beyond a fixed limit, or crash of the app process. | ASVS 1.4.1, 1.4.2, 1.4.3, 16.5.3; SSDF PW.8; CWE-787, CWE-416, CWE-190, CWE-120, CWE-121, CWE-122, CWE-476 | R2 | The CI job itself, which is part of `scripts/gate.sh` once the player exists |
| SEC-CLI-052 | Media-stack components shipped in clients (FFmpeg, libmpv, libass, FreeType, HarfBuzz, FriBidi, libplacebo, dav1d and any bundled image codec) must be pinned in the SBOM, and a release fixing a memory-safety vulnerability in any of them must ship within 14 days of the upstream fix, or 7 days if it is in CISA's Known Exploited Vulnerabilities catalogue. | ASVS 15.1.1, 15.2.1; MASVS-CODE-3; Top 10:2025 A03; CWE-1395 | R2 | Daily CI job that checks the SBOM against OSV and opens a release-blocking issue with the deadline; manual review of release dates against advisories |
| SEC-CLI-053 | The web client must render subtitles only through the browser's text-track pipeline (WebVTT) or a WASM renderer running in a worker, never by inserting subtitle text into the DOM. | ASVS 3.2.2; CWE-79 | R2 | Component and Playwright tests feeding a subtitle corpus (HTML tags, script, malformed ASS) and asserting no DOM nodes are created from cue text |
| SEC-CLI-054 | Every Android component must declare `exported` explicitly; only the launcher activity, the verified-link activity, the media browser or library service and TV-provider integrations may be exported; every PendingIntent must be immutable; and no internal communication may use an implicit intent. | MASVS-PLATFORM-1; MASWE-0018, MASWE-0032; CWE-926, CWE-927 | R2 | Android Lint security checks as errors; CI comparison of the merged manifest's exported components with a golden list |
| SEC-CLI-055 | The media browser or library service must return an empty root to any caller not on an allow-list verified by package signature (system UI, Android Auto and named wearables), so other apps cannot enumerate the library. | MASVS-PLATFORM-1; MASWE-0018; CWE-285, CWE-200 | R2 | Instrumented test binding from a test app signed with an unknown key and asserting an empty root; test that an allow-listed signature gets the real root |
| SEC-CLI-056 | Native apps must never read the clipboard except on an explicit paste action and must never copy credentials. A copied pairing code or recovery code must be marked sensitive (Android 13+ `EXTRA_IS_SENSITIVE`; iOS local-only with an expiry of at most 2 minutes); invite links are shared through the OS share sheet as the main action and, when copied, are marked sensitive with a 10-minute expiry. | MASVS-STORAGE-2, MASVS-PLATFORM-3; MASWE-0030; CWE-200 | R2 | Unit tests of the single copy helper per platform; instrumented test inspecting the `ClipDescription` extras; lint rule banning clipboard APIs outside the helper |
| SEC-CLI-057 | Screens that display a recovery code or a device-approval code must set `FLAG_SECURE` on Android and be replaced by a placeholder in Apple app-switcher snapshots; invite screens, the player and the library screens must not set it, so that people can screenshot and share invitations. | MASVS-PLATFORM-3; MASWE-0038; CWE-200 | R2 | Instrumented test that walks every screen and asserts the window flag matches a golden table; manual review of the iOS snapshot until Apple builds ship |
| SEC-CLI-058 | Controls that approve devices, grant roles, create invites or revoke devices must ignore touches while obscured, and their Android activities must declare `HIDE_OVERLAY_WINDOWS`. | MASVS-PLATFORM-3; MASWE-0039; CWE-1021 | R2 | Instrumented test with a helper app drawing an overlay, asserting the tap is ignored |
| SEC-CLI-059 | On personal devices, approving a device, creating an admin invite, changing roles, revoking all devices and viewing recovery codes must require OS user verification bound to a keystore key operation (a `CryptoObject` or a Keychain access-control flag), not a boolean callback. | MASVS-AUTH-2, MASVS-AUTH-3; MASWE-0020, MASWE-0023; ASVS 7.5.3; NIST SP 800-63B-4 §2.2 | R2 | Instrumented test that the protected key cannot sign without verification; unit tests of the gate; manual review against the platform guidance |
| SEC-CLI-060 | Release builds must not write credentials, tokens, signed URLs, invite secrets, item titles or user and profile names to OS logs, crash reports or the console, and JavaScript console calls must be stripped from release bundles. | ASVS 16.2.5; MASVS-STORAGE-2; MASWE-0005; CWE-532 | R2 | CI check of the release bundle for console calls; instrumented test that captures logcat during the scripted UI suite and fails on the canary token or a fixture title |
| SEC-CLI-061 | Publishing to OS-wide surfaces (Android TV Watch Next and channels, tvOS Top Shelf, Spotlight or app-search indexing, assistant donations, media resumption) must be asked once during TV pairing ("Show continue-watching on the TV home screen?"), defaulting to on for a single-profile TV and off when several profiles exist, must be opt-in per profile elsewhere, and must never include restricted or PIN-protected profiles. | MASVS-PRIVACY-1, MASVS-PRIVACY-4; MASWE-0036; CWE-359 | R2 | Unit tests of the publication policy function over profile counts and profile kinds; instrumented test on an Android TV image of both defaults |
| SEC-CLI-062 | Notifications other than active media controls must not show item titles, other users' names or device details on the lock screen; security notices must say only that something happened. | MASVS-PRIVACY-1; MASWE-0037; CWE-359 | R2 | Instrumented test asserting notification visibility and the public version's text |
| SEC-CLI-063 | Each profile's local data (library subset, queue, history, search history, continue-watching, artwork cache) must be stored in a partition keyed by profile, and switching profiles must clear the previous profile's data from memory and the UI tree. | MASVS-PRIVACY-1; CWE-226, CWE-359 | R2 | End-to-end test that switches profiles and asserts no string from the previous profile is in the rendered or accessibility tree; unit tests of partition keys |
| SEC-CLI-064 | On a device that more than one profile has used, a cold start must open the profile picker, which shows only profile names and avatars; a PIN-protected profile's data at rest follows SEC-IAM-065, and an adult profile's Activity data follows SEC-IAM-110. | MASVS-PRIVACY-1; CWE-359 | R2 | End-to-end tests on the TV build; instrumented test that the data directory holds no partition for a PIN-protected profile after switching away |
| SEC-CLI-065 | The server must refuse native client versions listed as insecure in the signed advisory manifest, or below an admin-set minimum, with a typed "update required" response that the client displays, and clients must check the manifest at most daily without sending identifiers. | MASVS-CODE-2; MASWE-0043; ASVS 15.2.1; CWE-1104 | R2 | Integration test with manifest fixtures; property tests of version comparison; network capture asserting the manifest request carries no identifiers |
| SEC-CLI-066 | Native clients must declare their minimum OS versions in one place, and CI must fail if any control in this document relies on an API above that minimum without a tested fallback that still meets the requirement. | MASVS-CODE-1; MASWE-0041, MASWE-0042; CWE-477 | R2 | Android Lint `NewApi` as error; CI check that each guarded API has a test covering its fallback branch |
| SEC-CLI-067 | Live-TV stream URLs, logos and guide images from M3U and XMLTV sources must reach clients only through the server (proxied streams, re-encoded images), clients must never fetch provider URLs or hand them to libmpv, and channel names, group titles and programme text must be rendered as text. | ASVS 1.3.6, 1.2.1; API10:2023; CWE-918, CWE-79, CWE-359 | R3 | Integration test with a hostile M3U fixture (`javascript:` logos, `file://` streams, HTML in names) asserting proxied, sanitised output; web and Android end-to-end tests with network capture |
| SEC-CLI-068 | Any future over-the-air update mechanism must verify, in native code and before the bundle is written to disk, an Ed25519 signature from an offline-held key over the bundle and its version, and must reject any bundle whose version is not higher than the installed one. | SSDF PS.2; Top 10:2025 A08; MASWE-0049; CWE-347, CWE-494 | Later | Property tests of the verifier; integration tests with tampered, re-signed-by-wrong-key and replayed older bundles |
| SEC-CLI-069 | A desktop shell must load only bundled content through an app-specific protocol, keep context isolation and process sandboxing on and Node integration off in every renderer, block navigation and new windows, open external links only for `https:` after confirmation, validate the sender of every IPC message, and (for Electron) turn off the `RunAsNode` and Node inspector fuses and turn on embedded archive integrity. | Electron security checklist items 1 to 20; Tauri 2 capabilities; ASVS 3.2.2, 3.7.1; CWE-749, CWE-79 | Later | Test that launches the shell and reads back the effective web preferences or capabilities; fuse read-back in CI; Playwright-for-desktop test that navigation and new windows are blocked |
| SEC-CLI-070 | Samsung and LG web-build TVs (secure-context support unverified per platform) must hold a non-extractable Web Crypto key, be limited-class devices capped as software keys (SEC-IAM-049), authenticate under SEC-IAM-050 with no refresh tokens, and offer no downloads; they ship only after the secure-context test passes on each platform. | ASVS 10.4.5, 8.2.1; RFC 9449; CWE-613 | Later | The SEC-IAM-050 proof tests run on the web build; per-platform secure-context test |
| SEC-CLI-071 | Credentials handed to a cast receiver or another device for playback must be scoped to one item and one session, expire under the SEC-API-027 lifetime table unless refreshed by the sender, and never be account or device tokens. | ASVS 7.2.2, 8.2.2; CWE-284, CWE-639 | Later | Integration tests that a cast credential cannot fetch another item, the library or the API, and fails after expiry |
| SEC-CLI-072 | If downloads to removable storage are ever allowed, they must be encrypted in seekable authenticated chunks under a per-device key held in the platform keystore. | MASVS-STORAGE-1, MASVS-CRYPTO-1; CWE-311, CWE-922 | Later | Property tests of the chunk format (round trip, any bit flip detected, random seeks return correct bytes); instrumented test on an emulator with a removable volume |

**Bound by requirements in [standards-coverage.md](standards-coverage.md).** SEC-STD-012 (prototype pollution), SEC-STD-019 (the crypto allow-list applies to native code), SEC-STD-027 (no pushed approvals an outsider can trigger), SEC-STD-033 (hardened C components in the client media stack, R2) and SEC-STD-039 (MAS-L2 and MAS-P targets, MASTG verification).
The 2026-10-02 challenge review merged duplicated controls into one owner each; withdrawn rows above say where their content went, and [threat-model.md](threat-model.md#control-ownership) lists every owner.

Count: 63 live requirements: 22 R1, 35 R2, 1 R3 and 5 Later, plus 9 withdrawn rows kept so their IDs stay stable.

## Design guidance

### 1. Trust model in one paragraph

The server never trusts a client: every rule is enforced on the server
(SEC-CLI-015, 024). A client never trusts content: everything that came from
a media file, the internet, another user, another device or a server is
hostile input to the renderer and the player (SEC-CLI-001, 021, 047). A
client trusts a server only to the extent it can prove the identity pinned
at enrolment (SEC-CLI-043), and even then keeps it apart from other servers
(SEC-CLI-044). The core crate keeps its rule of no `unsafe` and no panics; the
libmpv bridge, which must call C, lives in its own small native module or
crate with its own fuzz target so that the core stays clean.

### 2. Platform capability matrix

What each platform can actually give us. Where a cell says "(unverified)" a
spike must confirm it before the requirement that depends on it is
implemented.

| Platform | Release | Credential store | Hardware key | Local user verification | Process isolation for the player | Screenshot control | Device class |
|---|---|---|---|---|---|---|---|
| Web (any browser) | R1 | `__Host-` HttpOnly cookie only | No (a non-exportable WebCrypto key is possible later; Chrome ships Device Bound Session Credentials, a W3C draft) | Passkey with user verification | Browser's own media sandbox | None | Personal or limited, chosen at sign-in |
| Android phone and tablet | R2 | Keystore-wrapped storage | TEE; StrongBox on some devices; the CDD requires a hardware-backed keystore on handhelds with a secure lock screen | Biometric or device credential bound to a key | `isolatedProcess` (feasibility of rendering and hardware decode from it unverified) | `FLAG_SECURE`, `HIDE_OVERLAY_WINDOWS` | Personal |
| Android TV, Google TV, Fire TV (Fire OS) | R2 | Keystore-wrapped storage | Not confirmed by the CDD for TVs (unverified); check `KeyInfo` at run time | Usually none: TVs often have no secure lock screen, and Keystore cannot enforce user authentication without one | As Android | `FLAG_SECURE` | Limited |
| iPhone and iPad | R2 or later (Apple builds) | Keychain | Secure Enclave (P-256 only) | Face ID, Touch ID or passcode via access control | None for third-party apps | No blocking API; app-switcher snapshot can be obscured | Personal |
| Apple TV | R2 or later | Keychain | Secure Enclave availability to third-party keys on tvOS unverified | None | None | n/a | Limited |
| Desktop (Windows, macOS, Linux) | Later | OS keychain or credential manager; isolation between apps of the same user is weak on Linux and Windows (unverified detail) | TPM or Secure Enclave where present (unverified per OS API) | OS prompt where available | Separate sandboxed player process (seccomp/Landlock or Flatpak, App Sandbox, AppContainer) | No | Personal |
| Samsung Tizen, LG webOS (web build) | Later | No secure store for web apps found (unverified) | No | No | Platform video element | No | Limited |

Two consequences shape the rest of the design. First, TVs cannot do
step-up, so anything security-changing has to happen on a phone or computer;
we make that a feature (approve the TV from your phone) rather than a
missing capability. Second, some platforms cannot isolate the player, so
attack-surface reduction (SEC-CLI-047, 048, 049) is mandatory everywhere and
isolation is an extra layer, not the only one.

### 3. Keys, sessions and assurance levels

Each native install has three secrets, all created at enrolment.

1. **Device key** (P-256, non-exportable, in secure hardware where
   available). It signs enrolment, refresh challenges and the binding of the
   transport key. On phones it is created with a user-verification
   requirement for a second, separate "step-up" key used only for
   SEC-CLI-059 actions; the everyday key has no user-verification
   requirement so background playback and downloads keep working with the
   screen locked.
2. **Transport key** (Ed25519 for iroh, which needs the raw 32 bytes in
   memory). It is generated by the app, wrapped with an AES-GCM key that
   lives in the Keystore (Android) or stored as a "this device only"
   Keychain item, and bound to the device by a signature from the device
   key. The server stores the node ID against the device record and refuses
   any other node ID (SEC-CLI-032).
3. **Server pin** (the server's identity public key, received from the
   invite or QR). Stored with the credentials; checked on every connection
   (SEC-CLI-043).

Sessions: short access tokens (15 minutes) plus a refresh step that requires
a fresh device-key signature over a server nonce, in the spirit of DPoP
(RFC 9449) and the RFC 9700 rule that public-client refresh tokens must be
sender-constrained or rotated. Under NIST SP 800-63B-4 an ordinary playback
session from a device key is an AAL1-style session (the guideline suggests
an overall reauthentication limit of no more than 30 days at AAL1); signing
a refresh challenge with the device key counts as reauthentication. Security
changes need AAL2-style proof at the moment of the action (device key plus
local user verification, or a user-verifying passkey on the web) rather
than a long-lived AAL2 session, which would force TVs and phones to keep
re-prompting. That split keeps playback frictionless and makes the
dangerous operations rare and explicit.

Usability: nobody types a password on a TV, nobody is asked to re-verify to
play music, and the only prompts a household member sees are for actions
that deserve them (adding a device, inviting someone, changing roles).

### 4. The web client (R1)

**Rendering.** All text goes through React's normal text rendering. Lyrics
use `white-space: pre-wrap` rather than inserted `<br>` elements. Synced
lyric timestamps are parsed by the core and rendered as text spans. Links
come only from the URL filter (SEC-CLI-002). The lint rules in SEC-CLI-001
make the dangerous APIs unwritable, and Trusted Types make them throw at run
time even if a dependency uses them. Trusted Types reached Baseline in
February 2026 according to MDN. React 19 also errors on `javascript:` URLs
in `href` and `src`, which is a useful backstop but not a control we rely on.

**Golden CSP.** Start from this and only loosen with a reviewed reason:

```
default-src 'none';
script-src 'self' 'wasm-unsafe-eval';
style-src 'self';
img-src 'self' blob:;
media-src 'self' blob:;
font-src 'self';
connect-src 'self';
worker-src 'self';
manifest-src 'self';
object-src 'none';
base-uri 'none';
form-action 'self';
frame-ancestors 'none';
require-trusted-types-for 'script';
trusted-types 'none'
```

Notes: drop `'wasm-unsafe-eval'` if the core is not compiled to WASM for the
web build (keyword support across all target browsers is unverified).
Whether the chosen React Native for Web styling path needs `'unsafe-inline'`
for styles is unverified; if it does, that is acceptable for `style-src`
only, never for `script-src`. If WASM threads are used, the page also needs
cross-origin isolation (COOP plus COEP), which tightens things further.

**Artwork and other user content.** The server decides the image type from
the bytes and serves only JPEG, PNG or WebP (SEC-CLI-005). Each image
response carries a sandbox CSP so that if someone opens an image URL in its
own tab, nothing in it can run with the client's origin. SVG artwork is
never served; if a library contains SVG, the server-side media document
decides whether to rasterise it in the sandbox.

**Session.** A `__Host-` cookie scoped to the server's origin, `HttpOnly`,
`Secure`, `SameSite=Lax`. Lax rather than Strict so that someone arriving
from a shared link in a chat app is still signed in; cross-site forgery is
handled by the `Sec-Fetch-Site` and `Origin` checks (SEC-CLI-007). Signed
stream URLs from record 1 still apply, but for the web client the cookie is
also required, so revocation is instant and a leaked stream URL alone is
not enough within its lifetime.

**Personal or shared computer.** Ask once at sign-in: "Is this your own
device?" with "Yes, keep me signed in" preselected and "No, this is a shared
or public computer" one tap away. Shared mode keeps everything in memory,
uses a browser-session cookie and times out after 30 minutes idle. This is
the honest version of "remember me", and it is the control that protects a
family computer.

**Sign-out and revocation.** One client function, `wipeAccount(accountId)`,
is called on sign-out, user or profile switch, and on the typed "session
revoked" error. It deletes the account's IndexedDB databases, OPFS
directory, Cache Storage entries and web storage keys, clears in-memory
stores, unregisters the service worker and navigates to the sign-in page.
The sign-out response also sends `Clear-Site-Data`. Both are needed:
`Clear-Site-Data` wipes the whole origin but only runs when the response
arrives, and remote revocation is detected by the client, not by a sign-out
response.

**Stale code.** Every API request carries the bundle's build ID; the server
answers a mismatch with a typed error and the client reloads (SEC-CLI-011).
This stops a tab left open for weeks from running yesterday's vulnerable
code, and stops a service worker from pinning an old bundle.

**Admin views.** These get no special rendering rules, because the general
rules already cover them, but they are the reason SEC-CLI-014 exists.
Device names, user names and anything a client supplies are shown as text
and truncated. A script injected into an admin page cannot read the session
(HttpOnly) and cannot silently perform an admin action, because the server
requires a fresh user-verifying passkey assertion for each security change,
which the browser shows as its own prompt.

**Secure context.** Passkeys need HTTPS on a domain, and the R1 web client
must not send credentials in cleartext (SEC-CLI-008). How a household gets
HTTPS on the LAN is a setup decision (see the open decisions and the setup
research); the client's job is to refuse cleanly and explain.

### 5. Native data at rest

- **Where.** App-private internal storage only (SEC-CLI-035). On iOS the
  default file protection class ("protected until first user
  authentication") is the right one for the library database and downloads:
  the stricter "complete" class drops the class key about ten seconds after
  locking, which would stop the next track from opening during background
  playback. Keychain items use the "after first unlock, this device only"
  class for the same reason, and because "this device only" items are not
  backed up and do not migrate to a new phone.
- **Backups and device transfer.** On Android 12 and later, `allowBackup`
  set to false does not reliably stop device-to-device transfer on every
  manufacturer's devices, so the exclusions must be written in
  `dataExtractionRules` for both `cloud-backup` and `device-transfer`
  (SEC-CLI-033). Keystore keys themselves never leave the device, but
  wrapped blobs and preference files would.
- **Encryption of media at the app level.** Not in R2. On internal storage
  the OS already encrypts at rest, and app-level encryption of multi-GB video
  costs battery and complicates seeking for libmpv. Removable storage is
  refused (SEC-CLI-035) until an encrypted, seekable chunk format exists
  (SEC-CLI-072).
- **Offline grants.** The grant is a small signed record: device ID, item
  IDs or a rule ID, issue time, expiry, and the server time at issue. The
  core verifies it as a pure function. To resist the clock being set back,
  the client stores the latest server time it has seen and treats a device
  clock earlier than that as expired (SEC-CLI-036). Be honest in the UI and
  the docs: grants stop casual misuse of a found phone and enforce the
  admin's policy for well-behaved clients; they do not stop someone with
  root from reading files already on the device. The protections that
  matter for a stolen phone are the lock screen, OS encryption and
  revocation of the device's credentials.

### 6. Lost or stolen devices

The flow, which must be testable end to end:

1. From any personal device (or the web client in personal mode), the user
   opens "Your devices", picks the lost one (name, type, last seen, network)
   and chooses "Sign out this device". This needs local user verification,
   but no admin.
2. The server marks the device revoked, drops its node-ID binding and fails
   all its tokens and stream URLs on the next request (SEC-CLI-022).
3. If the lost device comes online, its next request returns "revoked" and
   the client runs the native wipe before drawing any other screen
   (SEC-CLI-037).
4. If it never comes online, its downloads stop playing when the grant
   expires (default 30 days, SEC-CLI-036), and its keys are useless for the
   account.
5. Every other device shows that the device was signed out (SEC-CLI-023
   uses the same notice channel).

Usability: the device list uses the names devices give themselves plus
platform icons, and shows "this device" clearly so nobody revokes the phone
they are holding. "Sign out everywhere except this device" is one button.

### 7. Pairing a TV, and the shared family TV

**Pairing.** The TV shows a QR code and a 6-character code. Scanning the QR
with the Gunmetal phone app opens the approval screen (SEC-CLI-038): device
name as given, platform, "on your home network" or "remote", time, and the
same 6 characters the TV shows. The approver checks the characters match
and confirms with Face ID, fingerprint or device PIN. The request lives for
10 minutes and works once. QR scanning favours someone standing in front of
the TV, which is the cheapest mitigation for the remote-phishing pattern
that RFC 8628 §5.4 describes and Storm-2372 exploited. A typed-code fallback
exists for people without the phone app, through the web client's device
page.

**Device class.** The TV is enrolled as a limited device (SEC-CLI-024). It
can browse, play and download (if the household allows downloads on TVs),
and switch between the profiles that the approver allowed on that TV. It
cannot approve devices, invite people, change roles or reach admin
settings, whatever its UI shows.

**Profiles on a shared TV.**

- The picker shows names and avatars only. No "continue watching", no
  "recently played", no previews on the picker (SEC-CLI-064).
- Each profile's local data lives in its own partition (separate SQLite
  file or a profile-keyed schema), and switching profiles tears down the
  previous profile's in-memory stores and UI tree (SEC-CLI-063).
- PIN-protected profiles are verified by the server only (SEC-CLI-029).
  Their data is never written to the TV's disk; it is fetched after the PIN
  is accepted and dropped on switch. A four-digit PIN checked on the device
  against a stored hash would fall to an offline guess in seconds, and a
  curious teenager with developer mode could read anything stored.
- Children's profiles get a library copy filtered by the server
  (SEC-CLI-020): if a title is above the profile's limit, its artwork,
  name and search terms never reach the TV. Hiding things in the UI alone
  is not a control.
- OS-wide surfaces (Watch Next row, Top Shelf, assistant history) are off by
  default on TVs (SEC-CLI-061). They are shared with every person in the
  room and with the launcher's vendor; a profile owner can opt in.
- Cold start on a TV used by several profiles opens the picker, the way the
  big streaming apps do, which is both expected and private.

Honest limits: a household member with the TV in their hands and developer
access can still read whatever an unprotected profile cached. Profile
partitions protect against the everyday cases (a sibling scrolling through
your history, a child switching profiles), and the server protects the
restricted and PIN-protected cases.

### 8. Links, invites and sign-in from apps

- **Routes.** One core function, `parse_link(input) -> Route`, handles every
  inbound link or code on every platform, including the web client through
  the WASM build (SEC-CLI-025). Routes are a closed enum: open item, open
  album, open playlist, join server (needs confirmation), approve device
  (needs confirmation and user verification), unknown. Nothing else.
- **Invites.** An invite is a link to a static page on gunmetal.tv with the
  secret, server address and server identity fingerprint in the fragment,
  so they never reach any server log. On a phone with the app installed,
  the verified link opens the app directly (App Links with `autoVerify` and
  Universal Links, SEC-CLI-039); otherwise the page tells the person which
  app to install. The app shows "Join *server name*? It will be able to see
  what you play there" with the fingerprint available under "details", and
  only then redeems the secret.
- **Custom schemes.** Android does not verify custom schemes, and any app
  can claim one, so no secret or action ever travels in a custom scheme.
- **Identity providers.** Native apps use the system browser with PKCE,
  `state` and `nonce`, and a claimed `https` redirect where available
  (RFC 8252 §7.2). RFC 8252 says native apps "MUST NOT use embedded
  user-agents" for authorisation; we go further and ship no web views at
  all (SEC-CLI-041). The web client never sees OAuth tokens (SEC-CLI-026).

### 9. Network and certificates

- **iroh.** The server's iroh identity is its pinned key; iroh already
  authenticates the peer by that key, so a native client on iroh needs no
  certificate authority at all. SEC-CLI-043's signed nonce makes the same
  check explicit and transport-independent.
- **HTTPS to the user's own domain.** System trust store, hostname
  verification, no user-added CAs (Android apps targeting API 24 and later
  ignore user CAs by default, unverified against the current page; we set
  it explicitly either way), no cleartext (the
  default for API 28 and later; ATS on Apple). Do not pin public CA
  certificates: home servers use Let's Encrypt, whose chains and lifetimes
  change (it planned six-day certificates for general availability by the
  end of 2025), and a pinned intermediate would break every client on a
  routine renewal. Pinning is done at the application layer against the
  server's own identity key instead, which survives certificate renewals
  and reverse proxies.
- **Self-signed servers.** Supported, because many households run without a
  domain, but only through an SPKI hash carried in the invite or shown as a
  QR at setup. Android's static network security config cannot hold
  per-user pins, so this is implemented in the native networking module at
  run time. There is no "trust anyway" button anywhere.

### 10. Code delivery and signing

- **No over-the-air code** (SEC-CLI-045). Apple's guideline 2.5.2 forbids
  downloaded code that changes features; Google Play's policy exempts code
  run by an interpreter with indirect access to Android APIs, which is how
  JavaScript bundle updates are usually justified (Apple's current
  enforcement for JS bundles unverified). Our reasons are different: an
  update channel is a single point that can push code to every household at
  once; Expo's end-to-end code signing for updates is offered only on paid
  plans; Microsoft retired App Center (and its hosted CodePush) on
  2025-03-31; and our release cadence through stores, F-Droid and GitHub is
  enough. If a hot-fix channel is ever needed, SEC-CLI-068 defines the
  minimum.
- **Signing** (SEC-CLI-046). Keep release keys in hardware or a key
  management service; CI requests signatures and never holds the key on
  disk. Use APK Signature Scheme v3 so the key can be rotated. Publish the
  fingerprints. From 30 September 2026, certified Android devices in Brazil,
  Indonesia, Singapore and Thailand require apps from registered developers
  (package name and signing key), with a global rollout from 2027 and a
  special path for open-source apps; F-Droid and sideloaded builds from
  unregistered developers need an extra "advanced flow" step for users. See
  the open decisions.
- **The web client** ships inside the server binary (SEC-CLI-012), so its
  integrity is the server release's integrity, covered by the release
  document.

### 11. Containing the player

The player is designed in layers, so that losing any one layer does not
lose the device.

| Layer | What it does | Where |
|---|---|---|
| 0. What reaches the client | Only containers the core's Rust parser accepted at scan time go to libmpv; others go through the server's remux or sandboxed transcode (SEC-CLI-049). The server, not the file, chooses which tracks and segments are sent. | All native clients |
| 1. Bytes only | libmpv gets one opaque `gunmetal://` stream through `mpv_stream_cb_add_ro`; the read and seek callbacks serve ranges over the authenticated transport. libmpv never sees a path or URL, so references inside a file have nothing to resolve against (SEC-CLI-047). | All native clients |
| 2. Reduced build and safe options | FFmpeg configured from `--disable-everything` plus an allow-list; no network protocols; mpv built without Lua, JavaScript, libarchive, disc or youtube-dl support; runtime options as SEC-CLI-048. libmpv already skips config files and the terminal by default. `access-references=no` stops MOV references, ordered chapters and archive opening; `ytdl=no` closes the CVE-2018-6360 class; with no network protocols the CVE-2016-1897 class has nowhere to send data. | All native clients |
| 3. Process isolation | Desktop: mpv in a child process with no network and no file access beyond fonts, receiving bytes over a pipe and drawing into a window handle. Android: a bound service with `android:isolatedProcess="true"`, which runs under its own UID with no permissions; the app passes a `Surface` and a pipe over Binder. Whether hardware decoding and GPU rendering work from an isolated process must be proven by a spike (unverified); if not, keep the player in-process on Android and rely on layers 0 to 2 and 4. iOS and tvOS allow no extra processes for this purpose, so they rely on layers 0 to 2 and 4, with hardware decoding through VideoToolbox preferred. | Desktop now; Android after the spike |
| 4. Keep it patched and prove it | SBOM with exact versions; 14-day and 7-day patch windows (SEC-CLI-052); ASan and UBSan corpus job (SEC-CLI-051). | All native clients |

Subtitles and fonts need extra care because they are where the 2017 player
takeovers happened and where FreeType's 2025 exploited bug lives (variable
TrueType fonts). Embedded fonts in Matroska attachments go through libass
into FreeType. Keeping embedded fonts on (`embeddedfonts=yes`, mpv's
default) matters for anime typesetting; the open decisions cover the
trade-off. External subtitles are fetched by the server, never by the
client, and reach the player as bytes like everything else.

On the web, the browser's own media pipeline and process sandbox do this
job; ASS subtitles, when video arrives, are rendered by libass compiled to
WASM inside a worker, so a memory bug stays inside the WASM sandbox
(SEC-CLI-053).

### 12. Platform interaction and UI privacy (native)

- **Exported components** (SEC-CLI-054): only what the OS needs to reach.
  Use Android Lint's security checks as errors.
- **Media browser service** (SEC-CLI-055): Android's documentation shows
  returning an empty root to deny browsing and verifying callers by package
  name and signature (the UAMP sample's package validator). Allow-list
  system UI, Android Auto and named wearables.
- **Clipboard** (SEC-CLI-056): one helper. Android 13 hides the preview of
  content marked sensitive; Android 12 already shows a toast when an app
  reads another app's clipboard. On iOS use local-only items with an expiry
  so Universal Clipboard does not carry an invite to a Mac (API names
  `localOnly` and `expirationDate` from memory, unverified). Never read the
  clipboard to "helpfully" detect invite codes.
- **Screens** (SEC-CLI-057, 058): `FLAG_SECURE` only where a secret is on
  screen. Never on the player: users screenshot their own media, and the
  flag also blocks showing the window on non-secure displays such as screen
  mirroring. Android 12 already blocks touches through most untrusted
  overlays; `HIDE_OVERLAY_WINDOWS` and touch filtering
  (`filterTouchesWhenObscured`, unverified against current docs) cover the
  approval buttons.
- **Logs** (SEC-CLI-060): strip `console.*` in release with the bundler,
  route native logs through one logger with a redaction pass, and use a
  canary token in tests to prove nothing leaks.

### 13. Desktop shell (Later)

Whichever shell is chosen (see the open decisions), the rules are the same:
the UI is the web client's code loaded from the package, never a remote
page; no Node or native API is reachable from the renderer except a small,
typed IPC surface whose every message is validated and whose sender is
checked; and libmpv runs as a separate sandboxed process. For Electron, the
project's own security checklist (20 items, with context isolation on by
default since version 12, sandboxing since 20, and Node integration off
since 5) and its fuses are the test list. For Tauri 2, use the capability
system to grant only the commands the UI needs, the isolation pattern, and a
CSP. Bundled Chromium in Electron means we must ship Chromium security
updates quickly (the libwebp bug in 2023 reached Electron apps such as
Signal and 1Password); Tauri uses the OS web view, which the OS patches,
but WebKitGTK on Linux varies by distribution.

### 14. How these become tests

Shared fixtures make most of this cheap:

- **Hostile metadata corpus.** One fixture library, generated by the core's
  test helpers, in which every text field of every format (FLAC, MP3, MP4,
  Ogg, later Matroska) holds payloads: script tags, event-handler
  attributes, `javascript:` URLs, bidi overrides, very long strings,
  invalid UTF-8. Used by SEC-CLI-001, 002, 021, 053 and 067.
- **Canary credentials.** Test sign-ins use a recognisable token so that
  storage scans, backups, logs and network captures can search for it
  (SEC-CLI-006, 030, 033, 060).
- **Network-capture harness.** Playwright and the Android emulator suite
  record every request and fail on unexpected hosts (SEC-CLI-012, 027,
  044, 065, 067).
- **Route table enumeration.** The server exposes its route table to tests
  so that CSRF, step-up, device-class and server-side enforcement tests
  cover every route automatically, and a new route without a policy fails
  the build (SEC-CLI-007, 014, 015, 024).
- **Core property tests and fuzz targets** for the link parser, URL filter,
  protocol decoder, grant verifier, server-identity handshake and version
  comparison (SEC-CLI-002, 021, 025, 036, 043, 065).
- **Player corpus job** under sanitizers (SEC-CLI-051).

Each of these is ordinary test code under the project's rules: test first,
deep assertions, 100% coverage, zero surviving mutants.

## Anti-patterns

1. **Rendering any field a client or file supplied as HTML, anywhere,
   including the admin console.** Jellyfin's web client put a device ID from
   a request header into an attribute without escaping; an admin viewing the
   devices page ran the attacker's script, which read the admin token from
   browser storage, uploaded a payload through a log endpoint with path
   traversal, and reached code execution on the server (CVE-2023-30627 with
   CVE-2023-30626, fixed in 10.8.10).
2. **Keeping session or refresh tokens where scripts can read them.** The
   same Jellyfin chain depended on the token being readable from browser
   storage.
3. **Treating a desktop shell's renderer as safe because the content is
   "ours".** Signal Desktop had an XSS through URL handling in its message
   view (CVE-2018-10994, fixed in 1.10.1); in a shell with Node or broad IPC
   access, that class of bug becomes control of the computer.
4. **Letting the player open whatever the file points to.** mpv's
   youtube-dl hook accepted arbitrary URLs from a web page, and an `av://`
   URL could make mpv load an arbitrary local shared library
   (CVE-2018-6360). FFmpeg's HLS handling could be steered by a crafted
   playlist to put lines of local files into outbound HTTP requests
   (CVE-2016-1897 and CVE-2016-1898, fixed in 2.8.5).
5. **Parsing hostile media inside a privileged process.** Stagefright's MP4
   integer overflow (CVE-2015-1538) gave remote code execution through a
   media file on Android versions before 5.1.1 LMY48I.
6. **Trusting subtitles and fonts because they are "just text".** Check
   Point showed in May 2017 that crafted subtitles could take over VLC,
   Kodi, Popcorn Time and Stremio, and that attackers could push them to the
   top of a public subtitle site's rankings so players fetched them
   automatically. FreeType's out-of-bounds write in variable-font parsing
   (CVE-2025-27363, versions up to 2.13.0) was exploited in the wild.
7. **Shipping a bundled image or media decoder and updating it slowly.**
   libwebp's heap overflow (CVE-2023-4863) was exploited in the wild and
   reached many Electron and Chromium-based apps that had to ship their own
   fixes.
8. **Approving a device from a code without showing what is being
   approved.** Storm-2372 sent fake meeting invitations that persuaded
   people to enter attacker-generated device codes, giving the attacker
   access (Microsoft, February 2025). RFC 8628 §5.4 describes exactly this
   remote-phishing risk.
9. **Embedding sign-in pages in web views.** The app can read what the user
   types, and the user cannot see the real address; RFC 8252 forbids it for
   native apps.
10. **Passing secrets through custom URL schemes.** Any app can register
    the same scheme; Android verifies only `http` and `https` links.
11. **Offering "trust this certificate anyway", or relaxing checks because
    the server is "local".** Emby's passwordless local sign-in combined with
    forged forwarding headers let remote attackers pass as local and
    backdoor about 1,200 servers in 2023 (CVE-2023-33193, from the users
    and security research file). A client that relaxes TLS or sign-in on
    the LAN invites the same failure.
12. **Pinning public CA certificates for self-hosted servers.** Chains and
    lifetimes change on the CA's schedule, not ours; pin the server's own
    identity key instead.
13. **Over-the-air code channels.** They concentrate risk in one service and
    can vanish: App Center's hosted CodePush was retired on 2025-03-31.
14. **Running development servers on all interfaces, or shipping dev hooks.**
    The React Native Community CLI's Metro server bound to external
    interfaces and exposed an endpoint that could launch executables
    (CVE-2025-11953, CVSS 9.8, in CISA's exploited list), affecting versions
    before 20.0.0.
15. **Letting install scripts run unreviewed.** In September 2025 a phished
    maintainer's npm account was used to publish malicious versions of
    `chalk`, `debug` and others, and the Shai-Hulud worm then republished
    hundreds of packages and harvested developer credentials.
16. **Checking PINs on the device, or hiding restricted titles only in the
    UI.** A stored PIN hash falls to an offline guess; anything synced to a
    device can be read by someone with developer access. Enforce on the
    server (CWE-602, CWE-603).
17. **Revocation that waits for URLs to expire.** Navidrome's shared stream
    URLs kept working after the share was deleted until a fix in September
    2026 (from the users and security research file). Every stream request
    must re-check the session or share.
18. **Telling other people what someone watched.** Plex's 2023 "Week in
    Review" emails exposed viewing to friends and cost a great deal of trust
    (from the users and security research file). On shared screens and OS
    surfaces the same failure happens quietly; default to off.
19. **Root and jailbreak detection as "security".** It blocks legitimate
    users on de-Googled and custom-ROM devices and protects nothing,
    because the client is open source and the server enforces every rule.

## Open decisions for the project owner

> **Status, 2026-10-02.** The owner decisions from every file in
> `docs/security/` are consolidated and de-duplicated in
> [README.md](README.md#open-decisions-for-the-owner). Where the baseline
> has since chosen, or where a recommendation below disagrees with the
> README, the README and the requirement tables win.

1. **How the R1 web client gets HTTPS on a home network.** SEC-CLI-008
   refuses credentials over cleartext, and passkeys need HTTPS on a domain.
   Options: a user-supplied domain and certificate; a server-generated
   private CA whose fingerprint is shown at setup (browser warnings until
   installed); or per-server names issued through gunmetal.tv, as plex.direct
   does. Recommendation: support a user domain from R1 and design the
   gunmetal.tv naming service as an optional, account-free service, decided
   in its own ADR before R1. Trade-off: a naming service is a soft central
   dependency; without it, non-technical households struggle to get HTTPS.
2. **Default offline grant lifetime.** Recommendation: 30 days, renewed
   silently on any contact, admin range 1 to 90 days. Trade-off: longer
   suits long trips with no connection; shorter limits how long a lost,
   never-reconnected device can play downloads.
3. **App-level encryption of downloads, and removable storage.**
   Recommendation: no app-level encryption on internal storage; no
   removable storage in R2; add an encrypted, seekable format later only if
   users ask for SD-card downloads. Trade-off: some phones and TV boxes have
   little internal space.
4. **Default answer to "personal or shared computer?"** Recommendation:
   preselect "personal". Trade-off: preselecting "shared" is safer on
   library and family computers, but makes every person on their own laptop
   sign in again daily, and people then pick "remember me" without reading.
5. **Container policy for native direct play** (SEC-CLI-049).
   Recommendation: only core-parsed containers go to libmpv by default;
   others (AVI, MPEG-TS, WMV and so on) are remuxed or transcoded on the
   server, with a per-library admin switch to allow direct play. Trade-off:
   more server work for old formats until the core parses them, against a
   much smaller container attack surface on every client.
6. **Fund a spike on Android `isolatedProcess` for libmpv before R2.**
   Recommendation: yes, with a pass criterion of hardware decoding and HDR
   output from the isolated process at no more than a small, measured cost
   in start-up time. Trade-off: engineering time before video ships.
7. **Embedded subtitle fonts.** Recommendation: keep them on by default,
   because typeset subtitles need them, and rely on the patch windows and
   isolation; offer a per-library switch. Trade-off: on iOS and tvOS, with
   no isolation, a font-parser bug runs in the app process.
8. **Android distribution and signing.** Recommendation: Google Play, F-Droid
   and signed APKs on GitHub; register package names and signing keys under
   Google's developer verification (using the open-source path) before the
   2027 global rollout; aim for reproducible builds so F-Droid can publish
   with the project's signature. Trade-off: registration ties the project to
   a Google account and identity check; not registering adds friction for
   sideloading users.
9. **Biometric only, or biometric or device PIN, for step-up.**
   Recommendation: allow the device PIN as well (MASWE-0021 flags this for
   sensitive transactions), because many household members never enrol
   biometrics, and the server-side step-up and device-class limits still
   apply. Trade-off: a shoulder-surfed phone PIN is enough to approve a
   device.
10. **Profiles and PINs in R1.** If the R1 web client ships household
    profiles, SEC-CLI-029 is already R1 and SEC-CLI-063 should move to R1
    for browsers. Recommendation: ship accounts in R1 and profiles with the
    TV apps in R2, but build the R1 data model to separate credentials from
    profiles. Trade-off: families sharing one computer in R1 must use
    separate accounts.
11. **Minimum OS versions,** especially Android TV, where many boxes are
    stuck on old releases. Recommendation: set the floor at the oldest
    version that supports every R2 control here with a tested fallback, and
    publish the list. Trade-off: some older TV boxes will not be supported.
12. **Desktop shell technology.** Recommendation: Tauri 2 running the web
    client, with mpv as a separate sandboxed process. It matches the Rust
    core, has a capability-based IPC model and uses OS-patched web views.
    Trade-off: rendering differences between web views, and WebKitGTK
    quality on Linux; Electron is more uniform but means shipping Chromium
    updates ourselves.
13. **Over-the-air updates, permanently.** Recommendation: no OTA in any
    release; revisit only with SEC-CLI-068 and an ADR. Trade-off: urgent
    client fixes wait for store review (usually a day or two).
14. **MASVS-RESILIENCE out of scope.** Recommendation: record in an ADR that
    Gunmetal does no root, jailbreak, tamper or debugger detection and no
    obfuscation for security, and that every control is server-enforced.
    Trade-off: none for security; some app-store reviewers and checklists
    expect these controls.
15. **Adopt this document's trust model as ADR 3 (client security).**
    Recommendation: yes, so that later feature work cannot quietly add web
    views, OTA updates, client-side enforcement or analytics SDKs.

## Sources

OWASP

- MASVS releases (v2.1.0 is the latest, 2024-01-18): https://github.com/OWASP/owasp-masvs/releases
- MASVS v2.1.0 release note: https://mas.owasp.org/news/2024/01/18/masvs-v210-release--masvs-privacy/
- MASVS control index: https://mas.owasp.org/MASVS/ ; MASVS-NETWORK-2 statement: https://mas.owasp.org/MASVS/controls/MASVS-NETWORK-2/ (statement seen in search results)
- MASWE index and entries: https://mas.owasp.org/MASWE/ , https://mas.owasp.org/MASWE/MASVS-PLATFORM/MASWE-0029/ , https://mas.owasp.org/MASWE/MASVS-CODE/MASWE-0049/ , https://mas.owasp.org/MASWE/MASVS-STORAGE/MASWE-0006/
- MASTG v2.0.0 (June 2026) and MASWE v1.0.0 (2026-08-17) release dates: search results only (https://github.com/OWASP/mastg/releases , https://www.nowsecure.com/?p=34693); pages not opened
- ASVS 5.0.0 chapters: https://raw.githubusercontent.com/OWASP/ASVS/v5.0.0/5.0/en/0x10-V1-Encoding-and-Sanitization.md , .../0x12-V3-Web-Frontend-Security.md , .../0x14-V5-File-Handling.md , .../0x15-V6-Authentication.md , .../0x16-V7-Session-Management.md , .../0x17-V8-Authorization.md , .../0x19-V10-OAuth-and-OIDC.md , .../0x21-V12-Secure-Communication.md , .../0x22-V13-Configuration.md , .../0x23-V14-Data-Protection.md , .../0x24-V15-Secure-Coding-and-Architecture.md , .../0x25-V16-Security-Logging-and-Error-Handling.md (chapters V2, V4, V9, V11 and V17 were not opened, so none of their requirement numbers are cited)
- OWASP Top 10:2025: https://top10.owasp.org/2025/0x00_2025-Introduction
- OWASP Mobile Top 10 2024: https://owasp.org/www-project-mobile-top-10/ (list seen in search results)

NIST and IETF

- SP 800-63B-4 session management: https://pages.nist.gov/800-63-4/sp800-63b/session/
- SP 800-63B-4 assurance levels: https://pages.nist.gov/800-63-4/sp800-63b/aal/
- SP 800-63-4 final (2025): https://csrc.nist.gov/pubs/sp/800/63/4/final (search result)
- SP 800-218 Rev. 1 (SSDF 1.2) initial public draft, 2025-12-17: https://csrc.nist.gov/pubs/sp/800/218/r1/ipd ; SSDF v1.1 practice IDs cited from memory (unverified against the PDF)
- RFC 8252 OAuth for native apps: https://www.rfc-editor.org/rfc/rfc8252
- RFC 8628 device authorisation grant: https://www.rfc-editor.org/rfc/rfc8628
- RFC 9700 OAuth 2.0 security BCP: https://www.rfc-editor.org/rfc/rfc9700
- RFC 7636 (PKCE) and RFC 9449 (DPoP) are cited by number; not opened

Platforms and frameworks

- Android `service` element (`isolatedProcess`, `exported`): https://developer.android.com/guide/topics/manifest/service-element
- Android Keystore: https://developer.android.com/privacy-and-security/keystore
- Android 16 CDD (§9.11 keystore requirements for handhelds): https://source.android.com/docs/compatibility/16/android-16-cdd
- Android App Links verification: https://developer.android.com/training/app-links/verify-android-applinks
- Android network security configuration: https://developer.android.com/privacy-and-security/security-config
- Android Auto Backup and `dataExtractionRules`: https://developer.android.com/identity/data/autobackup
- Android copy and paste (sensitive clipboard content): https://developer.android.com/develop/ui/views/touch-and-input/copy-paste
- Android 12 behaviour changes (untrusted touches): https://developer.android.com/about/versions/12/behavior-changes-all
- Android activity protections (`FLAG_SECURE`, `HIDE_OVERLAY_WINDOWS`): https://developer.android.com/security/fraud-prevention/activities
- Android media surfaces (MediaBrowserService caller checks): https://developer.android.com/media/implement/surfaces/mobile
- Android developer verification: https://developer.android.com/developer-verification
- Google Play Device and Network Abuse policy: https://support.google.com/googleplay/android-developer/answer/16273414
- Apple App Review Guidelines (2.5.2, 4.7): https://developer.apple.com/app-store/review/guidelines/
- Apple Keychain accessibility class: https://developer.apple.com/documentation/security/ksecattraccessibleafterfirstunlockthisdeviceonly
- Apple Platform Security, data protection classes: https://support.apple.com/guide/security/data-protection-classes-secb010e978a/web
- CryptoKit SecureEnclave (page content not retrievable; tvOS availability unverified): https://developer.apple.com/documentation/cryptokit/secureenclave
- MDN Trusted Types: https://developer.mozilla.org/en-US/docs/Web/API/Trusted_Types_API
- MDN Clear-Site-Data: https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/Clear-Site-Data
- Chrome Device Bound Session Credentials: https://developer.chrome.com/docs/web-platform/device-bound-session-credentials
- React 19 upgrade guide (`javascript:` URLs): https://react.dev/blog/2024/04/25/react-19-upgrade-guide
- Electron security checklist: https://www.electronjs.org/docs/latest/tutorial/security
- Tauri 2 security: https://v2.tauri.app/security/
- Expo EAS Update code signing: https://docs.expo.dev/eas-update/code-signing/
- Visual Studio App Center retirement: https://learn.microsoft.com/en-us/appcenter/retirement
- iroh `SecretKey` (iroh 1.3.0, Ed25519): https://docs.rs/iroh/latest/iroh/struct.SecretKey.html
- mpv manual: https://mpv.io/manual/stable/
- libmpv `client.h`: https://raw.githubusercontent.com/mpv-player/mpv/master/include/mpv/client.h
- libmpv `stream_cb.h`: https://raw.githubusercontent.com/mpv-player/mpv/master/include/mpv/stream_cb.h
- Let's Encrypt six-day and IP certificates: https://letsencrypt.org/2025/01/16/6-day-and-ip-certs/

Incidents and advisories

- CVE-2023-30627 record: https://cveawg.mitre.org/api/cve/CVE-2023-30627 ; advisory: https://github.com/jellyfin/jellyfin-web/security/advisories/GHSA-89hp-h43h-r5pq ; write-up: https://gebir.ge/blog/peanut-butter-jellyfin-time/
- CVE-2018-10994 (Signal Desktop): https://cveawg.mitre.org/api/cve/CVE-2018-10994
- CVE-2015-1538 (Stagefright): https://cveawg.mitre.org/api/cve/CVE-2015-1538
- CVE-2018-6360 (mpv): https://security.archlinux.org/CVE-2018-6360 (search result)
- CVE-2016-1897 and CVE-2016-1898 (FFmpeg): https://mailman.archlinux.org/pipermail/arch-security/2016-January/000522.html (search result)
- CVE-2025-27363 (FreeType): https://access.redhat.com/security/cve/cve-2025-27363 (search result)
- CVE-2023-4863 (libwebp): https://www.cert.europa.eu/publications/security-advisories/2023-063/ (search result)
- CVE-2025-11953 (React Native Community CLI): https://security-tracker.debian.org/tracker/CVE-2025-11953 (search result)
- Storm-2372 device code phishing: https://www.microsoft.com/en-us/security/blog/2025/02/13/storm-2372-conducts-device-code-phishing-campaign (search result)
- September 2025 npm compromises (chalk/debug, Shai-Hulud): https://blog.pulsedive.com/npm-compromise-the-wrath-of-the-shai-hulud-supply-chain-attack/ (search result)
- Check Point, "Hacked in Translation" (2017): https://research.checkpoint.com/2017/hacked-in-translation/
- CVE-2023-33193 (Emby), Navidrome share-URL revocation fix, Plex "Week in Review": cited from `docs/research/users-sharing-and-security.md` and its sources

Project documents

- `README.md`, `CONTRIBUTING.md`, `AGENTS.md`, `SECURITY.md`
- `docs/adr/0001-architecture.md`, `docs/adr/0002-music-is-first-class.md`
- `docs/research/clients-platforms-and-offline.md`, `docs/research/users-sharing-and-security.md`, `docs/research/setup-migration-and-operations.md`, `docs/research/video-playback.md`, `docs/research/discovery-home-and-search.md`
