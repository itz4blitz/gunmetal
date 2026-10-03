# Network and remote access

Written 2026-10-02. Web tools were available and used. Standards, versions
and incidents were checked against primary sources (OWASP, RFC Editor, CVE
records, vendor documentation and advisories). The shared web-search budget
ran out part-way through, so the last checks were made by fetching primary
pages directly. Anything not confirmed from a source is marked
"(unverified)". In the tables, "ASVS" means OWASP ASVS 5.0.0 (May 2025),
"Top 10" means the OWASP Top 10:2025, "API" means the OWASP API Security
Top 10 2023, and "MASVS" means OWASP MASVS v2.1.

## Summary

Gunmetal will sit in people's homes, will often be run by someone who is
not a network engineer, and will be used by housemates, children and
friends who are not engineers at all. The network design therefore has to
be secure when the owner does nothing, and still secure when the owner does
the wrong thing, such as forwarding a port or putting a tunnel in front of
the server without telling it. The single principle behind every
requirement below is: **the network location of a request may take access
away, but it never grants access.** "On the LAN" is not an identity. Emby's
2023 compromise (CVE-2023-33193) and Jellyfin's 2025 restart flaw
(CVE-2025-32012) both came from treating "local" as "trusted", and both were
reached by forging the headers that decide what "local" means.

**Encryption everywhere, including the living room.** Gunmetal has two
doors, and both are encrypted end to end on the LAN and remotely:

1. **iroh** for native clients (from R2), device pairing and remote access.
   iroh gives Gunmetal, for free, QUIC with TLS 1.3 in which both ends are
   authenticated by Ed25519 keys (raw public keys, RFC 7250, the default
   since iroh 0.34), NAT traversal that connects directly in roughly nine
   cases out of ten, and an end-to-end encrypted relay fallback. iroh 1.0
   shipped on 2026-06-15 with a wire-stability promise; 1.3.0 is current.
   What iroh does not give is authorisation: deciding which keys may connect
   is Gunmetal's job, and the server must refuse unknown keys before any
   application stream opens.
2. **HTTPS with a publicly trusted certificate** for browsers and
   third-party apps. Browsers need a real certificate for a secure context,
   which passkeys require. The recommended answer is Plex's: each server
   gets its own wildcard certificate under a project-run name zone, whose
   names encode the LAN address, with the private key generated and kept on
   the server. Gunmetal tightens Plex's design: names resolve only to local
   addresses, labels are random and unlinkable, the zone goes on the Public
   Suffix List, and CAA records bind issuance to the server's own ACME
   account. Users with their own domain, Tailscale, or a browser on the
   server itself have tested alternatives. Plain HTTP survives only as a
   redirect page, and on loopback.

**Remote access must be easier than port forwarding, or people will port
forward.** In the selfh.st 2025 survey, 721 respondents forward ports and
2,716 use reverse proxies (see `docs/research/pain-points-and-demand.md`).
Gunmetal's answer is remote access over iroh that needs no router change,
switched on with one control, with relays the project or the user runs.
Relays and any project edge see metadata (IP addresses, endpoint IDs,
timing, and volume while relayed) but never content; the client tells users
which path and which operator carry their session. For browsers away from
home, the recommended design is a TLS-passthrough edge: TLS still ends on
the user's own server, and the edge forwards ciphertext. This is an owner
decision (OD-2), because it means running infrastructure.

**Exposure by mistake becomes a dead end, not a breach.** By default every
listener is in a "home posture": a request that arrives directly from a
non-local address (a forwarded port, a global IPv6 address, a DMZ host) gets
a static page with no sign-in form, no API, no name and no version, and the
admin gets an alert saying what happened and how to fix it. A request that
carries forwarding headers from a proxy the admin has not declared is
refused with a "configure your proxy" page, so a tunnel cannot silently make
the server public. A fresh, unclaimed server can be set up only from a local
address with a code printed on its console, because attackers watch
Certificate Transparency logs and claim fresh installs within minutes.

**The rest:** no router port mapping and no SSDP, UPnP or DLNA in the core;
mDNS only from R2, saying as little as possible; reverse-proxy recipes for
Caddy, nginx, Traefik and Cloudflare Tunnel that CI runs for real; DoS
budgets sized for a two-core, 1 GiB machine and proven by a load test that
keeps music playing during a flood; an admin surface with its own session,
fresh user verification, and no access over internet-facing paths unless the
admin turns it on; and inventories of listening sockets, outbound
destinations and keys that are enforced as tests. The file has 67 live requirements: 49 R1, 16 R2, 1 R3 and 1 Later, plus 5 withdrawn rows kept so their IDs stay stable.

## Threats

| ID | Threat | Who (the attacker or failure) | Impact | Likelihood | Mitigated by (requirement IDs) |
|---|---|---|---|---|---|
| T-NET-01 | Passive capture of credentials, session tokens or listening activity on the home network | Housemate, guest on the Wi-Fi, compromised smart device, neighbour on an open network | Account takeover; exposure of what people play | High | SEC-NET-001, 002, 007, 008, 061 |
| T-NET-02 | Active impersonation of the server on the LAN (ARP spoofing, rogue access point, hostile router) | LAN attacker, compromised router | Stolen credentials; malicious web client served to the browser | Medium | SEC-NET-003, 009, 010, 014, 060, 061 |
| T-NET-03 | The server is reached from the internet by mistake: manual port forward, DMZ host, UPnP opened by other software, global IPv6 address behind a permissive router, container published on a public interface | Internet scanners, opportunistic attackers | Brute force against sign-in; exploitation of any pre-authentication bug | High | SEC-NET-024, 025, 026, 027, 028, 030, 047, 048 |
| T-NET-04 | Takeover of an unclaimed server during first-run setup | Scanners; attackers watching Certificate Transparency logs | Full admin control and a backdoor | Medium | SEC-NET-029, 010, 043 |
| T-NET-05 | Forged forwarding headers make a remote request look local or dodge rate limits | Internet attacker | Admin bypass (Emby 2023), unauthenticated restart (Jellyfin 2025), brute force (Navidrome 2026) | High | SEC-NET-016, 017, 018, 026, 042, 052 |
| T-NET-06 | DNS rebinding: a web page a household member visits drives the LAN server's API | Any website | Cross-origin reads and writes against the server | Medium | SEC-NET-014, 020, 008 |
| T-NET-07 | Cross-site WebSocket hijacking or permissive CORS | Malicious website | Session riding; data exfiltration | Medium | SEC-NET-020, 007, 008 |
| T-NET-08 | Request smuggling through a reverse proxy | Internet attacker | Request hijacking, authentication bypass | Low | SEC-NET-021, 022 |
| T-NET-09 | Host-header poisoning of generated invite, share and stream links | Anyone who can send a request | Invitees sent to an attacker's site; capability URLs leaked | Medium | SEC-NET-015, 014 |
| T-NET-10 | Traffic analysis by a relay or edge operator | Project, n0 or self-hosted relay operator, anyone who compromises or compels them | Reveals which devices use which server, when and from where; traffic shape can identify content | Medium | SEC-NET-037, 038, 039, 041 |
| T-NET-11 | A relay, edge or other project infrastructure tries to read or alter traffic | Compromised project infrastructure | Interception of remote sessions | Low | SEC-NET-041, 042, 033, 060 |
| T-NET-12 | The name-service operator, or whoever compromises it, obtains a certificate for a server's names | Compromised name service | Impersonation of the server to browsers, if also on-path | Low | SEC-NET-012, 060, 061 |
| T-NET-13 | CA hierarchy changes or incomplete chains break clients, and users respond by turning TLS off (Plex and Plexamp, September 2026) | Operational failure | Outage, then weaker security | Medium | SEC-NET-003, 004, 060 |
| T-NET-14 | The server falls back to plaintext when its certificate fails | Operational failure | Credentials exposed on the LAN | Medium | SEC-NET-005, 001 |
| T-NET-15 | Resource exhaustion on modest hardware: slow connections, HTTP/2 reset and CONTINUATION floods, multi-range requests, expensive endpoints, unbounded pre-authentication state | Internet attacker, buggy client, abusive friend | Music stops for the household; crash | High | SEC-NET-048, 049, 050, 051, 052, 053, 054, 055 |
| T-NET-16 | Unknown peers flood the iroh endpoint through relays | Anyone who learns the server's endpoint ID | CPU and memory exhaustion | Medium | SEC-NET-033, 035, 054 |
| T-NET-17 | Gunmetal used as a reflector or amplifier (SSDP, mDNS), or UPnP callbacks abused (CallStranger) | Attacker targeting a third party | The server joins a DDoS; LAN data exfiltration | Low | SEC-NET-059, 063, 066, 031 |
| T-NET-18 | LAN discovery leaks household details | Anyone on the same network: guests, dorms, hotels | Names of people and libraries; device fingerprinting | Medium | SEC-NET-063 |
| T-NET-19 | The admin surface is reached from the internet, or an XSS in the media UI (fed by untrusted metadata) rides an admin session | Internet attacker; poisoned metadata | Server takeover | Medium | SEC-NET-044, 045, 026, 007 |
| T-NET-20 | Operational endpoints and version strings help attackers find unpatched servers | Scanners | Targeted exploitation | Medium | SEC-NET-046, 047 |
| T-NET-21 | The home IP address is published through address lookup or leaked through invites | Anyone who learns the endpoint ID or sees an invite | Location exposure; direct DoS | Medium | SEC-NET-038, 036 |
| T-NET-22 | Invite or pairing secrets leak through query strings, logs or Referer headers | Log readers, third-party sites | Unauthorised device enrolment | Medium | SEC-NET-036, 035, 007 |
| T-NET-23 | A revoked or stolen device keeps streaming over an open connection | Thief, former household member | Continued access | Medium | SEC-NET-034 |
| T-NET-24 | Tailscale or other proxy identity headers are spoofed | Anyone who can reach the backend directly | Impersonation of users | Medium | SEC-NET-023, 016 |
| T-NET-25 | A tunnel (Tailscale Funnel, Cloudflare Tunnel) makes the server public without the server knowing | The admin's own configuration | Internet exposure while in home posture | Medium | SEC-NET-017, 019 |
| T-NET-26 | Users give up on remote access and forward the HTTP port | Usability failure | Everything in T-NET-03 | High | SEC-NET-040, 043, 013, 022, 024 |
| T-NET-27 | Live TV source URLs make the server fetch LAN or loopback services | Authenticated user; poisoned playlist | Server-side request forgery into the LAN | Medium | SEC-NET-067 |
| T-NET-28 | The server's identity key or TLS key is stolen from the host or a backup | Malware, leaked backup | Impersonation of the server | Low | SEC-NET-006, 057, 062 |
| T-NET-29 | Recorded traffic is decrypted later by a quantum-capable adversary | Long-term passive adversary | Retroactive exposure of history | Low | SEC-NET-065 |
| T-NET-30 | IPv6-specific bypasses: rotating addresses within a /64, IPv4-mapped addresses slipping past checks | Internet attacker | Rate-limit and posture bypass | Medium | SEC-NET-052, 025 |
| T-NET-31 | Every Gunmetal server is enumerated from Certificate Transparency logs | CT log watchers | Targeted scanning | High, if names are logged | SEC-NET-010, 011, 029, 043, 047 |

## Requirements

Requirements apply to every listener and every protocol, including the
OpenSubsonic and Jellyfin adapters whenever they ship. Numbers given as
defaults (timeouts, limits, the reference hardware) are initial values the
owner can tune (OD-11); the requirement is that a limit exists, is tested,
and has a default.

| ID | Requirement | Standards | Release | Verified by |
|---|---|---|---|---|
| SEC-NET-001 | Over plaintext HTTP, the server must give every peer other than loopback only a static redirect or help page that sets no cookie: no web client, no API, no WebSocket upgrade, no adapter route, and no credential of any kind accepted (passkey ceremony, session or admin cookie, invite secret, PIN, share password, pairing or claim code, app key), whatever any setting says. Browser navigations may redirect to the configured HTTPS URL, API routes fail with 403, and the help page explains in plain words how to reach the secure address. This is the only cleartext rule in the baseline. | ASVS 3.3.1, 4.1.2, 4.4.1, 12.2.1, 12.3.1; Top 10 A04:2025; CWE-319, CWE-523; RFC 9325 | R1 | Integration test that replays every route in the route table over the plaintext listener from RFC 1918, link-local, unique-local and public addresses and asserts the exact redirect or help response, no Set-Cookie and an unchanged database; the same test from 127.0.0.1 and ::1 asserts loopback behaviour; Playwright test on an http:// LAN origin asserting no credential field is rendered |
| SEC-NET-002 | Every TLS listener must negotiate only TLS 1.3, or TLS 1.2 with ECDHE key exchange and an AEAD cipher suite. | ASVS 12.1.1, 12.1.2; RFC 8446; RFC 8996; RFC 9325; NIST SP 800-52 Rev. 2; CWE-327 | R1 | Integration test that sends recorded ClientHello fixtures for SSL 3.0 to TLS 1.1 and non-AEAD TLS 1.2 suites and asserts handshake failure; positive tests for TLS 1.3 and one permitted TLS 1.2 suite |
| SEC-NET-003 | The server must present the complete certificate chain, and must refuse to activate a newly obtained certificate whose chain does not validate, without fetching missing intermediates, against the bundled WebPKI root set. | ASVS 12.2.2; CWE-295; RFC 5280 | R1 | Unit tests over fixture chains (complete; missing intermediate; missing cross-signed root, as in Let's Encrypt's Generation Y rollout; wrong order) asserting exact typed errors; integration test with a strict validating client against the running listener |
| SEC-NET-004 | Certificate renewal must be automatic, must follow ACME Renewal Information when the CA offers it and otherwise renew no later than two-thirds of the way through the certificate's lifetime, must work for lifetimes from 6 to 398 days, and must keep serving the current certificate until its replacement has validated. | RFC 8555; RFC 9773; CA/Browser Forum ballot SC-081v3; ASVS 12.2.2; NIST SP 800-218 PW.9 | R1 | Property test of the renewal scheduler over random lifetimes and ARI windows with an injected clock; integration test against Pebble (Let's Encrypt's test CA), including a failed renewal that leaves the old certificate in service |
| SEC-NET-005 | When no valid certificate is available, the server must not serve the web client, sign-in or API in plaintext to non-loopback peers; it must report the problem on the console, in the log, on the plaintext help page and to signed-in admins over every other channel. | ASVS 12.2.1, 16.5.3; CWE-319; CWE-636 | R1 | Integration test with expired and missing certificate fixtures asserting the plaintext listener still serves only the help page and the admin notice is raised over loopback and iroh |
| SEC-NET-006 | TLS private keys, the ACME account key and the server's iroh secret key must be generated on the server from the operating system's CSPRNG, stored readable only by the service user (mode 0600 or the platform equivalent), never written to logs and never sent to any other party, including project services. | ASVS 11.5.1, 13.3.2, 16.2.5; CWE-312; CWE-532; CWE-732 | R1 | Integration test asserting file modes and ownership; unit test that formats every configuration and key type and asserts secrets render as a fixed placeholder; unit tests with deep equality on every outbound project-service request body |
| SEC-NET-007 | **Withdrawn 2026-10-02: merged into SEC-API-038, SEC-API-044, SEC-API-053.** Headers have one owner each. The CSP gains configured relay origins in connect-src only while browser remote access is on (R2), as SEC-API-044 records. | ASVS 3.4.1, 3.4.3, 3.4.4, 3.4.5, 3.4.6; RFC 6797; CWE-1021; CWE-319 | Withdrawn | Proved by the tests of SEC-API-038, SEC-API-044, SEC-API-053 |
| SEC-NET-008 | **Withdrawn 2026-10-02: merged into SEC-API-032.** One session-cookie definition, including "never on a plaintext response" and "Secure never depends on request headers". | ASVS 3.3.1, 3.3.2, 3.3.3, 3.3.4; CWE-614; CWE-1004 | Withdrawn | Proved by the tests of SEC-API-032 |
| SEC-NET-009 | All outbound TLS (ACME CA, name service, relays, update feed, plugins) must validate certificates against the WebPKI with hostname checks; the only custom verifier allowed is iroh's raw-public-key pinning, and code that disables verification must fail CI. | ASVS 12.3.2; MASVS-NETWORK-1; CWE-295; CWE-297 | R1 | CI check that rejects rustls "dangerous" configuration APIs and accept-invalid-certificate options outside one allowlisted module; integration test against a test server with a wrong-host certificate asserting a typed error |
| SEC-NET-010 | If the server uses the project's per-server name service (the recommended install-time default, owner decision OD-1), each server's name label must be 128-bit random, generated on the server, and not derived from its endpoint ID, owner or address; the server must obtain its own wildcard certificate by DNS-01 with a key that never leaves it, and must sign every DNS update with its identity key. | ASVS 12.2.2, 11.5.1; RFC 8555 section 8.4; CWE-359 | R1 | Unit tests with deep equality on the registration and TXT-update requests (label, public key, TXT value, signature and nothing else); unit test that the label generator's only input is the CSPRNG; manual review of the name service; CI job that runs the full first-run claim and passkey ceremony from a LAN peer over the name-service path (Pebble and a test DNS zone) |
| SEC-NET-011 | The name service must answer A and AAAA queries, as a pure function of the queried name, only for labels that encode an address in RFC 1918, RFC 6598 or RFC 4193 space (plus the edge names of OD-2), and must return no address for public addresses or malformed labels. | CWE-1188; RFC 1918; RFC 4193; RFC 6598 | R1 | Property tests: encode and decode round-trip for every local IPv4 address and random local IPv6 addresses; every public address and malformed label yields no answer; fuzz target for the label parser |
| SEC-NET-012 | The name service must publish a CAA record for each registered label that restricts issuance to the server's own ACME account and to dns-01, and the server must check that record at each renewal and alert the admin if it is missing or names another account. | RFC 8657; RFC 8659; ASVS 12.2.2 | R1 | Integration test with a local test DNS server: a missing or mismatched accounturi raises the admin alert; a correct record does not |
| SEC-NET-013 | Browser HTTPS must also work without the project name service: through the admin's own domain with ACME DNS-01 (and DNS-PERSIST-01 once the configured CA offers it), through Tailscale Serve, and on localhost, each with a documented setup. | ASVS 12.2.2; NIST SP 800-218 PW.9 | R1 | CI integration jobs for own-domain DNS-01 (Pebble with a test DNS provider) and for localhost (headless browser asserting a secure context and a passkey ceremony); Tailscale path on the manual release checklist |
| SEC-NET-014 | The server must answer 421 Misdirected Request to any request whose Host or :authority is not in its configured name set (per-server names, configured public hostnames, localhost names and, on the plaintext redirect listener only, its own interface address literals). | RFC 9110 sections 7.4 and 15.5.20; CWE-346; CWE-350 | R1 | Property test of the host matcher (case, trailing dot, ports, punycode, IPv6 literals); integration test replaying a DNS-rebinding request carrying an attacker's hostname |
| SEC-NET-015 | Every absolute URL the server emits (invites, share links, redirects, cast and stream URLs) must be built from the configured canonical origin for the path the request arrived on, never from Host, :authority or forwarding headers. | ASVS 3.7.2; CWE-601; CWE-346 | R1 | Unit tests of the URL builder with deep equality; integration test that sends forged Host and X-Forwarded-Host values and asserts the invite link is unchanged |
| SEC-NET-016 | The server must ignore Forwarded, X-Forwarded-*, X-Real-IP, True-Client-IP, CF-Connecting-IP and similar headers for every purpose unless the immediate peer is in the owner-configured trusted-proxy list, which must be empty by default. Through a trusted proxy, the client address is the right-most address in the chain that is not itself a trusted proxy, and that single derived address is used for rate limiting, logging and every location rule. | ASVS 4.1.3, 15.3.4; RFC 7239 section 8; A01:2025; CWE-348; CWE-290; CVE-2023-33193 | R1 | Property tests of chain parsing in core (spoofed left-most entries, malformed entries, IPv6, obfuscated forms); integration test matrix (each header, trusted and untrusted peer) asserting the resolved client address, scheme and host |
| SEC-NET-017 | When a peer that is not a trusted proxy sends any forwarding header, the server must ignore the headers, give the request internet posture, log a security event and offer the fix: the claim page (authorised by the claim code) or an admin page (authorised by a fresh-uv check) asks "A reverse proxy at <address> is in front of Gunmetal. Trust it?", and `gunmetal trust-proxy <address> --public` or `--overlay` does the same on the host. | ASVS 4.1.3, 16.3.3; Top 10 A02:2025; CWE-1188, CWE-348 | R1 | Integration tests of the claim-through-proxy flow behind the CI Caddy, nginx and Traefik containers (SEC-NET-022), of the header matrix, of the event record and of the prompt text |
| SEC-NET-018 | For requests from trusted proxies, the client address must be the first address that is not a trusted proxy when the forwarding chain is read from the right; a malformed or over-long chain must be refused with 400. | ASVS 15.3.4, 4.2.5; RFC 7239; CWE-348 | R1 | Property test: for any attacker-chosen prefix followed by any sequence of trusted proxies, the resolved address is the first untrusted address from the right; fuzz target for the header parsers |
| SEC-NET-019 | The owner must declare each trusted proxy as "private overlay" or "public" (the default), and every request through a public proxy gets internet posture. A private-overlay request must also carry an overlay-specific proof on every request or it gets internet posture: for Tailscale Serve, the presence of the Tailscale-User-Login header, which Tailscale omits on Funnel traffic (used only as a posture signal, never as identity, SEC-NET-023). A private overlay must reach the server on a dedicated listener (a Unix socket or a port used only by the overlay), and loopback on the main port is never accepted as a private proxy. | ASVS 4.1.3, 13.1.1; CWE-1188, CWE-290, CWE-348 | R1 | Unit test that an undeclared proxy parses as public; integration test that a Funnel-shaped request (no identity headers) from the declared proxy gets internet posture and setup and admin are refused; integration test that a loopback request with a forged X-Forwarded-For and no proof gets internet posture |
| SEC-NET-020 | WebSocket upgrades must be refused unless Origin is one of the server's own origins, and no response may carry Access-Control-Allow-Origin with a wildcard, a reflected Origin or "null". | ASVS 4.4.1, 4.4.2, 3.4.2; CWE-1385; CWE-942 | R1 | Integration tests with foreign, null and missing Origin values on every WebSocket route and on preflights for every API route |
| SEC-NET-021 | The HTTP/1.1 front end must reject ambiguous framing (Content-Length with Transfer-Encoding, conflicting Content-Length values, invalid chunk sizes, obsolete line folding, bare CR or LF in headers), both directly and behind each documented reverse proxy. | ASVS 4.2.1, 4.2.2, 4.2.4; RFC 9112 sections 6.3 and 11.2; CWE-444 | R1 | Integration tests with a request-smuggling corpus against the server alone and behind nginx, Caddy and Traefik containers; fuzzing of any request-head code the project owns |
| SEC-NET-022 | The project must ship reverse-proxy configurations for Caddy, nginx, Traefik and Cloudflare Tunnel plus a Tailscale Serve recipe, and CI must run each real proxy except Tailscale in front of the server and assert client address, scheme, Host, WebSocket and Range behaviour. | ASVS 13.1.1; NIST SP 800-218 PW.9.1; Top 10 A02:2025 | R1 | CI integration job per proxy; Tailscale recipe on the manual release checklist |
| SEC-NET-023 | The server must never treat Tailscale identity headers (Tailscale-User-Login, Tailscale-User-Name, Tailscale-User-Profile-Pic) or any other proxy-supplied identity header as authentication, unless a later architecture record adds proxy sign-in with its own requirements. | ASVS 6.3.4, 8.3.3; CWE-290; CWE-348 | R1 | Integration test sending each header from loopback and from a trusted proxy and asserting the authentication state does not change |
| SEC-NET-024 | In the default home posture, the HTTP and HTTPS listeners must serve only a static help page (no sign-in form, no API, no server name, no version) to any request whose resolved client address is outside loopback, RFC 1918, RFC 6598, RFC 4193 and IPv6 link-local space, on IPv4 and IPv6 alike. RFC 6598 space (100.64.0.0/10) counts as local only when one of the server's own interfaces holds an address in the same overlay prefix; otherwise it is non-local. | ASVS 13.4.5; Top 10 A02:2025; API8:2023; CWE-1327; CWE-1188 | R1 | Integration test that runs the whole route table from each address class (real loopback and ULA addresses where CI allows, otherwise through a trusted test proxy) and asserts the literal help response for every non-local class; property tests of the 100.64.0.0/10 rule with and without a matching interface |
| SEC-NET-025 | Address classification must convert IPv4-mapped IPv6 addresses to IPv4 before classifying them, and must classify NAT64, 6to4, Teredo, multicast, unspecified, documentation and every other non-local range as non-local. | CWE-697; CWE-1327; RFC 4291; RFC 6052; RFC 3056; RFC 4380 | R1 | Property tests over both families against an independent reference table written for the test |
| SEC-NET-026 | **Withdrawn 2026-10-02: merged into SEC-IAM-013.** Location and context signals may only add friction or remove access. | ASVS 6.3.4, 8.2.1, 8.3.1; Top 10 A01:2025, A07:2025; CWE-290; CWE-348 | Withdrawn | Proved by the tests of SEC-IAM-013 |
| SEC-NET-027 | When a request from a non-local address reaches a listener in home posture, the server must record a security event and, within one minute, show the admin an alert naming the listener, time and source with steps to fix it, raised at most once per listener per day with a running count. | ASVS 16.3.3; Top 10 A09:2025 | R1 | Integration test with an injected clock asserting the event, the alert text and the de-duplication |
| SEC-NET-028 | At startup and whenever interfaces change, the server must show the admin which listeners are bound to globally routable IPv4 or IPv6 addresses, and must say that inbound reachability through the router cannot be confirmed from the server. | ASVS 13.1.1; CWE-1327 | R1 | Unit test with fake interface lists; component test of the admin text |
| SEC-NET-029 | **Withdrawn 2026-10-02: merged into SEC-IAM-007, SEC-IAM-008.** Setup is protected by the 128-bit code and a secure context, not by the source address range, so container and NAT installs can be claimed; it is still never accepted through pairing or the edge. | ASVS 6.3.1, 6.3.2, 2.4.1; CWE-306; CWE-307; CWE-208; CWE-1188 | Withdrawn | Proved by the tests of SEC-IAM-007, SEC-IAM-008 |
| SEC-NET-030 | The server must never request router port mappings (UPnP IGD, NAT-PMP or PCP) for its HTTP listeners, and iroh's port mapper must stay disabled unless the admin enables it. | RFC 6886; RFC 6887; Top 10 A02:2025; CWE-1188 | R1 | Unit test asserting the iroh endpoint builder configuration; integration test in a network namespace asserting no SSDP M-SEARCH, NAT-PMP or PCP packets leave the host during startup or when remote access is turned on |
| SEC-NET-031 | In each documented configuration, the set of sockets the server listens on must equal the documented list exactly. | ASVS 13.1.1, 13.4.2; API9:2023; CWE-1327 | R1 | Integration test that reads the process's listening sockets and compares them with a literal list per configuration |
| SEC-NET-032 | In each configuration the server must connect out only to the destinations in the egress inventory (SEC-TM-075; by default in R1 only the naming purpose, when the install chose the project name service), and every other outbound connection must need an explicit owner or plugin grant. | ASVS 13.1.1, 13.2.4, 13.2.5, 14.2.3; ADR 0002; CWE-359 | R1 | Integration test in a network namespace recording DNS queries and connection attempts through startup, scan, playback and idle, compared with a literal allowlist |
| SEC-NET-033 | For every ALPN except the pairing protocol, the server's iroh endpoint must reject a connection whose client endpoint ID is not an enrolled, unrevoked device key before any application stream is accepted. | ASVS 12.1.3, 12.3.5, 8.2.1; MASVS-NETWORK-1; RFC 7250; RFC 9001; CWE-306 | R2 | In-process integration test with two real iroh endpoints on loopback (relays disabled) asserting rejection in the connection hook and zero calls into application handlers |
| SEC-NET-034 | Revoking a device or app credential must close that device's open iroh connections, HTTP sessions and in-flight streams within 5 seconds. | ASVS 8.3.2; CWE-613 | R2 | Integration test that revokes a device mid-stream and asserts closure within 5 seconds of an injected clock |
| SEC-NET-035 | The pairing protocol must accept only invite-redemption and device-approval messages of at most 4 KiB, one exchange per connection, with per-source and global rate limits, invite secrets of at least 128 bits compared in constant time, and a security event for each failure. | ASVS 2.4.1, 6.3.1, 16.3.1; CWE-307; CWE-770; CWE-208 | R2 | Integration tests for each rule; fuzz target for the pairing message decoder |
| SEC-NET-036 | Invite and pairing secrets must travel only in URL fragments, QR payloads or the pairing protocol, and must never appear in query strings, server logs, address-lookup records or Referer headers. | ASVS 14.2.1; CWE-598; CWE-532 | R1 | Unit test of the invite encoder (secret only after "#"); integration test that searches every log written during an invite flow for the secret |
| SEC-NET-037 | Servers and clients must use only relays from an explicit configured list (the project default from OD-3, or one the admin sets), reached over HTTPS with WebPKI validation, and the admin must be able to replace the list with a self-hosted relay in one setting. | ASVS 12.3.2, 13.2.5; CWE-295 | R2 | Integration test with a local iroh relay behind a test CA; unit tests of relay configuration parsing |
| SEC-NET-038 | The server must not publish its direct IP addresses to any public address-lookup service (DNS, pkarr or the Mainline DHT); if address lookup is on, it must publish only its relay URL. | ASVS 14.2.3; CWE-359 | R2 | Integration test against a local iroh-dns-server asserting deep equality of the published record with a relay-only record |
| SEC-NET-039 | Each client must show whether its connection is direct or relayed and who operates the relay or edge, and the privacy page must state what that operator can see: both public IP addresses, both endpoint IDs, connection times and, while relayed, traffic volume and timing. | ASVS 14.1.2; CWE-359 | R2 | Component test of the connection indicator; manual review of the privacy page against this list |
| SEC-NET-040 | Turning remote access on must not require router changes, open ports or software other than Gunmetal, and the relay-only path must work with all direct UDP blocked. | NIST SP 800-218 PW.9; Top 10 A06:2025 | R2 | Integration test in relay-only mode that plays a track end to end through a local relay; manual review of the enablement flow on the release checklist |
| SEC-NET-041 | Browser remote access must terminate TLS on the Gunmetal server itself; any project-run relay or edge must forward only ciphertext and must hold no key able to decrypt user traffic. | ASVS 12.2.1, 12.3.1; Top 10 A04:2025; CWE-300 | R2 | Integration test through the reference edge asserting the certificate the client sees is the server's and the edge holds no TLS key; manual review of the edge design |
| SEC-NET-042 | The server must accept a client address supplied by the edge only over an iroh connection authenticated as the configured edge endpoint ID, and must give every edge request the internet posture. | ASVS 4.1.3, 15.3.4; CWE-348; CWE-290 | R2 | Integration test: the same metadata sent from any other endpoint ID is rejected; internet-posture limits apply to edge requests |
| SEC-NET-043 | Browser remote access must stay off until an admin turns it on after setup; turning it on must take one control in setup or settings, and turning it off must end new and existing edge sessions within 5 seconds. | NIST SP 800-218 PW.9; CWE-1188 | R2 | Integration test of both transitions; manual review of the setup flow |
| SEC-NET-044 | **Withdrawn 2026-10-02: merged into SEC-IAM-041.** The separate admin session is now the elevation model of SEC-IAM-041. | ASVS 8.2.1, 8.4.2, 3.5.4; Top 10 A01:2025; CWE-306 | Withdrawn | Proved by the tests of SEC-IAM-041 |
| SEC-NET-045 | Admin operations must be refused on internet-posture paths (public proxy, edge) unless the admin has turned remote administration on, and must be allowed over iroh from enrolled admin devices. | ASVS 8.4.2, 8.2.1; Top 10 A01:2025; CWE-284 | R1 | Integration test matrix: every admin route from every path class, with remote administration on and off |
| SEC-NET-046 | Metrics, profiling and debug endpoints must be off by default and, when on, must listen only on loopback or require a dedicated scoped token; the health endpoint must return only liveness. | ASVS 13.4.2, 13.4.5; API9:2023; CWE-497 | R1 | Integration tests of each endpoint in both states; listening-socket inventory (SEC-NET-031) |
| SEC-NET-047 | Unauthenticated responses must not reveal the server version, build, operating system, library names or the server's display name. | ASVS 13.4.6, 16.5.1; CWE-497 | R1 | Integration test scanning headers and bodies of every unauthenticated response for the version string, component names and display name |
| SEC-NET-048 | The HTTP front end must enforce a global connection cap, a per-client-key connection cap, a header read timeout of at most 10 seconds, a header size limit of at most 16 KiB, a minimum request-body data rate, per-route body size caps and an idle keep-alive timeout. | ASVS 4.2.5, 13.1.2, 15.1.3, 15.2.2; API4:2023; CWE-400; CWE-770 | R1 | Integration tests for each limit, including slow-header and slow-body simulations that assert bounded memory |
| SEC-NET-049 | HTTP/2 must allow at most 100 concurrent streams per connection and must bound client-initiated and server-initiated stream resets, CONTINUATION frames and header list size per connection. | RFC 9113 section 10.5; CVE-2023-44487; CVE-2025-8671; CERT/CC VU#421644; ASVS 15.2.2; CWE-400 | R1 | Integration tests with scripted Rapid Reset, MadeYouReset-style and CONTINUATION-flood clients asserting connection closure and bounded CPU and memory |
| SEC-NET-050 | A byte-range request may name at most one range; a request with more than one range must be answered with 416 Range Not Satisfiable. | RFC 9110 sections 14.2 and 17.15; CVE-2011-3192; CWE-400 | R1 | Unit and property tests of the Range parser in the core; fuzz target; integration test |
| SEC-NET-051 | State created for unauthenticated clients (sign-in challenges, setup and pairing attempts, rate-limit counters) must live in fixed-capacity stores with eviction, so memory stays bounded at any request volume. | ASVS 2.4.1, 15.2.2; CWE-770 | R1 | Property test that the store never exceeds capacity under any operation sequence; load test asserting bounded memory |
| SEC-NET-052 | Rate-limit and connection-cap keys must be the IPv4 address or hierarchical IPv6 prefixes (/64, /56 and /48 counters, each with its own ceiling), or, for trusted-proxy traffic, the address resolved under SEC-NET-016; requests from an "unknown" peer (SEC-NET-068) use their own bucket, which can never delay loopback or local-direct sign-in. | ASVS 2.4.1, 6.3.1; CWE-799 | R1 | Property tests of key derivation over both families, including an attacker rotating through the /64s of one /48 |
| SEC-NET-053 | Expensive authenticated operations (search, artwork resizing, archive downloads, scans, audio transcodes, and from R2 video remuxing and transcoding) must run under per-user and global concurrency limits with queueing, so one user cannot stall another's playback. | ASVS 15.1.3, 15.2.2, 2.3.2; API4:2023; API6:2023; CWE-770 | R1 | Integration test that saturates one user's quota and asserts another user's stream keeps its throughput |
| SEC-NET-054 | The iroh endpoint must cap total connections, connections per endpoint ID and concurrent streams per connection, must rate-limit handshakes per source, and must require address validation (QUIC Retry) when under load. | RFC 9000 sections 8 and 21; ASVS 15.2.2; API4:2023; CWE-770; CWE-406 | R1 | Integration load tests with many unknown and known endpoints asserting each cap and the switch to Retry |
| SEC-NET-055 | On the reference low-end profile (initially 2 CPU cores and 1 GiB of memory, enforced with cgroups), an ongoing 2 Mbit/s audio stream must play without client buffer underrun while the server receives 2,000 idle slow connections and 200 failed sign-in attempts a second. | ASVS 15.2.2; API4:2023 | R1 | Nightly CI load test with the profile enforced by cgroups |
| SEC-NET-056 | The server must log, as structured security events without secrets and with injection-safe encoding: TLS handshake failures (aggregated), Host rejections, untrusted forwarding headers, non-local access attempts, setup and pairing failures, rate-limit trips, relay or edge changes, and certificate renewal failures. | ASVS 16.3.1, 16.3.3, 16.3.4, 16.4.1, 16.2.5; Top 10 A09:2025; CWE-117; CWE-532 | R1 | Integration tests asserting deep equality of each event record |
| SEC-NET-057 | The project must keep a cryptographic inventory listing every key, certificate and algorithm on the network paths (TLS keys, ACME account key, iroh identity key, name-service registration, edge and relay keys) with where it lives, its lifetime and how it is rotated. | ASVS 11.1.1, 11.1.2, 11.2.2 | R1 | CI check that every key wrapper type defined in the code base appears in the inventory file; manual review |
| SEC-NET-058 | The web client used on the LAN must be served by the Gunmetal server from the same origin as its API, not from a project-hosted origin. | Top 10 A08:2025; CWE-829 | R1 | Integration test that the server serves the bundle and that its Content-Security-Policy names no project origin |
| SEC-NET-059 | The core server must not implement or open SSDP, UPnP eventing or DLNA services. | CISA alert TA14-017A; CVE-2020-12695; CWE-406; CWE-441 | R1 | Listening-socket inventory (SEC-NET-031); CI dependency check rejecting UPnP and SSDP crates in the server's dependency tree |
| SEC-NET-060 | Native clients must pin the server endpoint ID received in the invite and must refuse a different key for that server unless it comes with a key-rotation statement signed by the pinned key. | MASVS-NETWORK-2; ASVS 12.3.2, 12.3.4; CWE-295 | R2 | Core unit and property tests of the pin store and statement verifier: any statement not signed by the pinned key is rejected with a typed error |
| SEC-NET-061 | Native clients must reach the server only over iroh or over HTTPS to the server's own names with certificate validation, with no plaintext LAN path. | MASVS-NETWORK-1; ASVS 12.3.1; CWE-319 | R2 | Integration test with packet capture on a LAN path asserting only QUIC or TLS traffic to the server; review gate on the client's transport module |
| SEC-NET-062 | The server must support rotating its iroh identity key by issuing a continuity statement signed by the old key, and must accept connections on both keys for a configurable grace period (default 30 days). | ASVS 11.1.1, 13.3.4 | R2 | Integration test: a pinned client follows a rotation; a statement signed by any other key is refused |
| SEC-NET-063 | When mDNS advertising is on, the server must advertise only on interfaces with local addresses, must ignore queries not received with IP TTL 255 or from off-link sources, and must advertise only the service type, a generic instance name and its endpoint ID, with no user, library or household names or version unless the admin opts in. | RFC 6762 sections 5.5 and 11; RFC 6763; CWE-200; CWE-406 | R2 | Unit test with deep equality on the advertised records; integration test with crafted queries (TTL below 255, off-link source) asserting silence |
| SEC-NET-064 | Cast and operating-system transfer URLs must be capability URLs scoped to one item and one device or cast session, must expire within the item's duration plus at most 1 hour (refreshable), must be revocable, and must never carry a session token. | ASVS 14.2.1; API1:2023; CWE-598 | R2 | Unit tests of the URL signer; integration tests for expiry, scope and revocation |
| SEC-NET-065 | TLS listeners must offer the X25519MLKEM768 hybrid key exchange and choose it whenever the client offers it. | ASVS 11.2.2, 11.6.2; FIPS 203 | R2 | Integration tests with a client offering only X25519MLKEM768, and with one offering it alongside X25519, asserting the hybrid group is chosen |
| SEC-NET-066 | Any DLNA or UPnP plugin must be off by default, serve only local addresses and only libraries the admin selects, be read-only, never send event callbacks to non-local addresses, and answer SSDP only on the local link. | CISA alert TA14-017A; CVE-2020-12695; CWE-406; CWE-441; ADR 0002 | Later | Plugin integration tests; listening-socket inventory with the plugin enabled |
| SEC-NET-067 | Live TV sources (M3U, XMLTV and tuner addresses) must be fetched through the server's egress policy, which must refuse loopback, link-local, the server's own addresses and cloud metadata addresses unless the admin explicitly allows a specific LAN tuner address. | API7:2023; ASVS 13.2.4, 13.2.5; CWE-918 | R3 | Integration tests with redirecting and DNS-rebinding source URLs; property tests on the destination classifier shared with SEC-NET-025 |
| SEC-NET-068 | A peer whose address equals the default gateway or bridge address of the server's own interface, or a configured container gateway, must be classified "unknown" and treated as non-local (internet posture, no "on your home network" label, its own rate-limit bucket). `doctor` and the first start must detect container or NAT front-ends (the share of requests from the gateway, container markers in cgroups) and ask the owner to declare the topology, naming the fixes (host networking, rootless Docker with its userland proxy disabled, or a trusted proxy). | ASVS 4.1.3, 13.1.1; CWE-348, CWE-1327 | R1 | Integration test in a network namespace that SNATs public clients to the bridge gateway, asserting internet posture, the exposure alert (SEC-NET-027), no home-network label, and that exhausting the unknown bucket does not delay loopback sign-in |
| SEC-NET-069 | When the server uses the project name service, it must by default monitor Certificate Transparency for its own label through at least two independent CT monitors reached through the egress client, and raise a critical owner alert for any certificate not issued to its own ACME account key. | RFC 9162; ASVS 12.2.2; CWE-295 | R1 | Integration test with a fake CT feed containing a foreign certificate, asserting the critical alert, and none for the server's own certificate |
| SEC-NET-070 | The project name service must launch only after its zone is on the Public Suffix List; must limit registrations per source and per key and require proof-of-work or a minimum key age; must garbage-collect labels that stop renewing; must keep its zone-signing and update keys offline under two-person control (SEC-STD-036), separate from any edge infrastructure; and must publish its issuance log. | ASVS 2.4.1; SSDF PO.5.1; CWE-770 | R1 | Load test of the registration endpoint; manual review before launch of the PSL entry, the key ceremony record and the published log |
| SEC-NET-071 | When the name service refuses, rate-limits or cannot be reached, the server must keep working on every other path (localhost, own domain, tailnet) and explain the failure and the alternatives in plain words on the console and the help page. | ASVS 16.5.2; CWE-636 | R1 | Integration test against a refusing test name service, asserting the console and help-page text and that localhost sign-in still works |
| SEC-NET-072 | The server must alert the owner 30 and 7 days before its certificate expires, with a one-click "renew now"; the claim page must explain that the chosen name is permanent for passkeys; and a documented, tested migration must move a household to a new origin by re-enrolling members through device-to-device approval. | ASVS 12.1.1, 16.3.3; CWE-298 | R1 | Injected-clock tests of both alerts; end-to-end origin-migration test |

**Bound by requirements in [standards-coverage.md](standards-coverage.md).** SEC-STD-016 (HSTS preload for project domains, including the name-service zone), SEC-STD-018 (crypto inventory and post-quantum plan, which now owns ASVS 11.1.4 instead of SEC-NET-065) and SEC-STD-040 (no TCP between the server and its own workers).
The 2026-10-02 challenge review merged duplicated controls into one owner each; withdrawn rows above say where their content went, and [threat-model.md](threat-model.md#control-ownership) lists every owner.

Count: 67 live requirements: 49 R1, 16 R2, 1 R3 and 1 Later, plus 5 withdrawn rows kept so their IDs stay stable.

## Design guidance

### The three doors and the two postures

The server opens exactly three kinds of socket in R1, and SEC-NET-031 makes
that list a test:

| Door | Default bind | Who uses it | Encryption |
|---|---|---|---|
| HTTPS listener | All interfaces, IPv4 and IPv6 | Browsers on the LAN, third-party apps, trusted reverse proxies | TLS 1.3 or 1.2 with a WebPKI certificate |
| Plaintext HTTP listener | All interfaces | People who type `http://192.168.x.x`; it only redirects (and serves the app on loopback) | None, so it carries nothing sensitive |
| iroh endpoint | UDP on `0.0.0.0` and `[::]` (iroh's default) | Native clients (R2), pairing, the edge, remote access | QUIC with TLS 1.3, raw Ed25519 keys on both sides |

Every request is classified once, at the edge of the server, into a
**path class**, and every surface consults one policy table. Build this as
a pure function in the core (no I/O) so it can be property-tested
exhaustively against a literal table written in the test:

| Path class | How it is recognised | Posture | Setup (unclaimed) | Web client and sign-in | Media API | Admin API |
|---|---|---|---|---|---|---|
| Loopback | Peer is 127.0.0.0/8 or ::1, no forwarding headers | Home | With code | Yes (plaintext allowed) | Yes | Yes, with admin session |
| Local direct | Peer in RFC 1918, RFC 6598, RFC 4193 or fe80::/10 | Home | With code | HTTPS only | Yes | Yes, with admin session |
| Private overlay proxy | Peer is a trusted proxy declared private | Home | With code | Yes | Yes | Yes, with admin session |
| Public proxy | Peer is a trusted proxy declared public (the default) | Internet | No | Yes | Yes | Only if remote administration is on |
| Edge | iroh connection from the configured edge key | Internet | No | Only if browser remote access is on | Yes | Only if remote administration is on |
| iroh device | iroh connection from an enrolled device key | Authenticated transport | No | n/a | Yes | Admin devices, with fresh user verification |
| iroh pairing | Unknown key on the pairing ALPN | Unauthenticated | No | Invite redemption only | No | No |
| Non-local direct | Anything else reaching an HTTP listener | Home posture blocks it | No | Help page only | No | No |
| Untrusted proxy | Forwarding headers from a peer not in the list | Refused | No | "Configure your proxy" page | No | No |

The table only ever *removes* access. Authentication is still required on
every row; a loopback request has no more rights than any other until it
signs in (SEC-NET-026). The "home" and "internet" postures differ only in
limits and in what unauthenticated surfaces exist: internet posture uses
stricter rate limits, and refuses admin and setup.

### TLS everywhere, including the LAN

**What iroh gives for free.** Every iroh connection is QUIC with TLS 1.3.
Since iroh 0.34 the handshake uses raw public keys (RFC 7250) instead of
self-signed X.509 certificates, and the endpoint ID *is* the Ed25519 public
key, so dialling an endpoint ID authenticates the server and the server
learns the client's key during the handshake. There is no CA, no
certificate expiry and no hostname. iroh 1.0 (2026-06-15) promised
wire compatibility across all 1.x versions and added QUIC multipath. For
native clients this solves the local-certificate problem completely: pin
the server's key from the invite (SEC-NET-060) and use iroh on the LAN and
remotely alike. A CA hierarchy change, like Let's Encrypt's move to its
Generation Y roots in May 2026, cannot break a native client, which is
exactly what broke Plexamp in September 2026 (its built-in trust store
lacked the new roots).

**The browser problem.** Browsers grant a secure context only to HTTPS
origins with a publicly trusted certificate, and to localhost. Passkeys,
service workers and `__Host-` cookies all need one. A LAN server at
`http://192.168.1.7` has none. The options:

| Option | Works for non-technical users | Central dependency | Notes |
|---|---|---|---|
| Self-signed or private CA | No: warnings, and TVs cannot install roots | None | Teaches people to click through certificate warnings. Rejected |
| Per-server names under a project zone (Plex's design) | Yes | Name service, CA | Recommended default (OD-1) |
| Admin's own domain with ACME DNS-01 | Technical users | Their DNS provider | Supported. DNS-PERSIST-01 will remove the need for DNS API credentials on the server once the CA offers it |
| Tailscale Serve (`*.ts.net` certificates) | Users already on Tailscale | Tailscale | Supported recipe; names appear in CT logs |
| Browser on the server itself (localhost) | Setup only | None | Always available |
| IP-address certificates | No: public IPs only | CA | Let's Encrypt issues these since January 2026, but only for public addresses and only as 6-day certificates, so they do not help on a LAN |

**Per-server names, done more strictly than Plex.** Plex's scheme (written
up by Filippo Valsorda in 2015) gives each server a wildcard certificate for
`*.<hash>.plex.direct`, and the plex.direct DNS answers
`1-2-3-4.<hash>.plex.direct` with 1.2.3.4. The private key stays on the
server. Plex now gets these certificates from Let's Encrypt. Gunmetal's
version:

1. At first start, if the admin has not turned it off, the server generates
   a 128-bit random label (base32, lower case) and an ACME account key, and
   registers the label with the name service by signing a request with its
   iroh identity key. The service stores label to public key, first come
   first served. Nothing else about the server is sent (SEC-NET-010).
2. The server asks the CA for `*.<label>.<zone>` using DNS-01. It sends the
   challenge TXT value to the name service in a request signed by the same
   key. The service publishes it, then removes it. The CSR key never leaves
   the server.
3. The service publishes a CAA record for `<label>.<zone>` with
   `accounturi` set to the server's ACME account and `validationmethods`
   set to `dns-01` (RFC 8657). The server checks it at each renewal
   (SEC-NET-012). This does not stop a malicious zone operator, who can
   change CAA too, but it stops casual or mistaken issuance and makes
   tampering visible.
4. The zone answers A and AAAA only for labels that encode *local*
   addresses (SEC-NET-011). Encode IPv4 as dashes (`192-168-1-7`) and IPv6
   as dashes with `--` for the zero run (`fd12-3456--7`). Decoding is a pure
   function: no database lookup is needed to answer, and a property test can
   cover it. Refusing public addresses means a per-server name can never
   help a server that someone has exposed directly to the internet.
5. Put `<zone>` on the Public Suffix List. Three reasons: Let's Encrypt
   counts its limit of 50 new certificates per 7 days per *registered
   domain*, which it finds through the PSL, so without an entry every
   Gunmetal server would share one limit; without an entry, all servers
   under the zone are the same "site", so SameSite cookies would not stop
   one server owner's page from making requests to another's; and passkeys
   use `<label>.<zone>` as their relying-party ID, so each server gets its
   own credential and it survives a change of LAN address.
6. Renew with ACME Renewal Information (RFC 9773, June 2025) and fall back
   to renewing at two-thirds of the lifetime (SEC-NET-004). Lifetimes are
   falling: the CA/Browser Forum's ballot SC-081v3 caps public certificates
   at 200 days from 2026-03-15, 100 days from 2027-03-15 and 47 days from
   2029-03-15. Let's Encrypt's default goes to 64 days on 2027-02-10 and to
   45 days on 2028-02-16, and its 6-day profile is generally available.
   Revocation is no longer OCSP: Let's Encrypt announced that it would drop
   OCSP URLs from new certificates on 2025-05-07 and turn its responders
   off on 2025-08-06, moving to CRLs, so ASVS 12.1.4 (OCSP stapling,
   level 3) does not apply; short lifetimes do that job.
7. Serve the full chain and check it with a strict validator before
   switching (SEC-NET-003). In September 2026, Plex servers served the leaf
   and the YR2 intermediate without the cross-signed ISRG Root YR, and
   strict clients (OpenSSL and GnuTLS without fetching missing
   intermediates) refused to connect.

The plaintext listener at `http://192.168.1.7:<port>/` serves one static
page that redirects to `https://192-168-1-7.<label>.<zone>:<port>/`. Many
routers block public names that resolve to private addresses (DNS rebinding
protection in dnsmasq, Unbound and similar), which Plex documents as a
common problem. If the HTTPS page has not loaded within a few seconds, the
redirect page shows plain instructions (allow `<zone>` in the router's
rebinding settings, or use the app on the server, or Tailscale). It never
falls back to serving sign-in over HTTP (SEC-NET-001, SEC-NET-005).

Because the zone operator controls DNS for every label, it could obtain a
certificate for any server's names. That is the real cost of this design
for browsers, and the docs must say so. Native clients are immune because
they pin iroh keys. Certificate Transparency monitoring for one's own label
(opt-in, Later) would detect it after the fact.

**TLS configuration.** Use rustls. TLS 1.3 preferred, TLS 1.2 kept for
older TVs and third-party apps, which rustls already limits to ECDHE with
AEAD suites. rustls 0.23 (0.23.45 at the time of writing) supports the
hybrid X25519MLKEM768 group with the aws-lc-rs provider and prefers it only
with its `prefer-post-quantum` feature; the provider choice is OD-8.
Encrypted Client Hello (ASVS 12.1.5, level 3) would hide the label in SNI
from on-path observers; leave it for Later.

### Remote access over iroh

**Endpoint configuration.** Set these explicitly in one place, and pin them
with a unit test (SEC-NET-030, 033, 037, 038, 054), because several iroh
defaults are not what Gunmetal wants:

| iroh setting | iroh default (1.3.0) | Gunmetal setting | Why |
|---|---|---|---|
| Relay mode | n0's public relays | Explicit list: project relays (OD-3) or the admin's own | n0 describes its public relays as for development and hobby use only, with rate limits and no uptime promise |
| Port mapper (UPnP, PCP, NAT-PMP) | Enabled | Disabled; admin can enable (OD-5) | Gunmetal does not change router configuration without being asked |
| Address lookup | DNS/pkarr to `dns.iroh.link` | Relay URL only, to the service chosen in OD-4, or none | Never publish the home IP address |
| mDNS address lookup | Off | Off in R1; R2 per SEC-NET-063 | |
| Bind | `0.0.0.0` and `[::]` | Same | Unauthenticated peers are refused at the handshake |
| Connection hooks | None | Allowlist of enrolled device keys per ALPN | Authorisation is Gunmetal's job |

**ALPNs.** Use one application protocol, `gunmetal/1`, for enrolled
devices, and one pairing protocol, `gunmetal-pair/1`, for unknown keys
(SEC-NET-033, 035). The connection hook looks up the client key before
accepting any stream; the pairing handler reads one length-prefixed message
of at most 4 KiB, answers, and closes. Invite tickets carry the server's
endpoint ID, its relay URLs and a 128-bit secret, in a URL fragment or QR
code (SEC-NET-036). Redeeming an invite enrols the new device's key.
Revocation removes the key from the allowlist and closes live connections
(SEC-NET-034); Jellyfin 12.1 had to fix device revocation that left open
sessions running.

**What a relay operator can and cannot see.** This must be in the privacy
page, in plain words (SEC-NET-039):

| A relay operator can see | A relay operator cannot see | A relay operator can do |
|---|---|---|
| Public IP addresses of both ends | Anything inside the connection: track, title, user, playlist | Drop, delay or throttle traffic |
| Endpoint IDs of both ends, so which devices use which server (a household and friendship graph) | Which account or person is using a device | Refuse service |
| When each device is online and connected | Traffic after the connection goes direct | Correlate IP addresses with time over many sessions |
| Traffic volume and timing while relayed | | Hand that metadata to anyone who compels it |

Traffic shape can reveal content. Schuster, Shmatikov and Tromer (USENIX
Security 2017) identified encrypted video streams from their burst patterns
alone. Whether the same works for single music tracks, whose sizes are
distinctive, is (unverified), but it should be assumed. Padding is too
expensive to require. The honest mitigations are direct connections (about
nine in ten), relays the user trusts, and telling users.

**Browsers away from home (OD-2).** iroh in a browser runs relay-only: the
WASM build cannot hole-punch, so every byte goes through a relay. Three
designs are possible:

1. **TLS-passthrough edge (recommended).** The project runs an edge that
   accepts TLS on port 443 for names `r-<secret>.<label>.<zone>`, reads only
   the SNI, and forwards the raw TLS bytes over iroh to the server that
   registered that name. TLS ends on the server, with the same wildcard
   certificate it uses on the LAN, so the edge sees ciphertext only
   (SEC-NET-041). The secret part of the name is covered by the wildcard
   certificate, so it never appears in CT logs, and scanners cannot find
   the server from those logs (it is not a security boundary, just one less
   invitation). The edge passes the client address to the server as a PROXY
   protocol v2 header on the iroh stream, and the server trusts it only from
   the edge's pinned key (SEC-NET-042). Passkeys work at home and away,
   because both names share the relying-party ID `<label>.<zone>`. The code
   the browser runs still comes from the user's own server.
2. **Hosted web app plus iroh in WASM.** A static app on a project domain
   connects to the server through relays. No edge is needed, but the
   project's hosting becomes the code-delivery point for every user's
   session, so a compromise of that hosting compromises everyone. It also
   meets Chrome's Local Network Access prompt (enforced since Chrome 142):
   a public page reaching a LAN address must ask permission. In November
   2025 Plex warned that this prompt pushed its web app, which loads from
   app.plex.tv, onto the paid remote path even inside the house.
   SEC-NET-058 keeps the LAN web app on the server's own origin whatever
   is chosen here.
3. **Tailscale only, for R1.** No project infrastructure, but every family
   member needs a Tailscale account and app, so many owners will port
   forward instead (T-NET-26).

Music through a relay or edge is affordable; video is not. Plex's relay is
capped at 2 Mbps for everyone (see `docs/research/users-sharing-and-security.md`),
which is why its users forward ports. Gunmetal's edge should not cap music
below lossless rates, and the R2 native apps will mostly connect directly.

When the user is at home but the app is open on the remote name, offer a
"switch to your home connection" link that *navigates* to the LAN name.
Top-level navigation is not gated by Local Network Access, while a
background `fetch` from the public name to the LAN name would raise the
prompt.

### Running behind a reverse proxy

Reverse proxies are how 2,716 selfh.st respondents reach their services,
so they must work well, but only on purpose:

- **Trusted proxies are a list of addresses or CIDR ranges, empty by
  default** (SEC-NET-016). Each entry is declared `private` (an overlay
  such as Tailscale Serve) or `public` (the default, for anything reachable
  from the internet, including Cloudflare Tunnel and Tailscale Funnel)
  (SEC-NET-019).
- **Untrusted forwarding headers fail closed** (SEC-NET-017). The page says,
  in effect: "A reverse proxy or tunnel is in front of this server. Add its
  address to Trusted proxies in Settings, then reload." This is the moment
  the server learns it is exposed and switches to internet posture. Navidrome
  ignored this case and its sign-in rate limit could be bypassed with forged
  X-Forwarded-For, X-Real-IP and True-Client-IP headers
  (GHSA-f295-6wp9-qqfg, September 2026).
- **Client address resolution** walks X-Forwarded-For (or `Forwarded`,
  RFC 7239) from the right, skipping trusted proxies, and stops at the first
  untrusted address (SEC-NET-018). Never take the leftmost value; the client
  writes it.
- **Scheme and host** come from configuration, not from X-Forwarded-Proto
  or X-Forwarded-Host. The admin sets a canonical public URL per proxy, and
  every generated link uses it (SEC-NET-015). A path prefix (`/gunmetal`) is
  also configuration, never derived from X-Forwarded-Prefix.
- **Host checking** accepts the configured public hostname for requests
  from that proxy and answers 421 for anything else (SEC-NET-014).
- **Recipes** for Caddy, nginx, Traefik and Cloudflare Tunnel must: pass
  Host unchanged; overwrite (not append to) the forwarding headers at the
  first proxy; pass WebSocket upgrades; disable response buffering for media
  routes; pass Range and Content-Range unchanged; and use long read timeouts
  for streams. CI runs each one (SEC-NET-022).
- **Authentication stays in Gunmetal.** Proxies that add identity headers
  (Authelia, authentik, Tailscale Serve) do not sign anyone in
  (SEC-NET-023). Proxy sign-in is a separate decision for the sign-in
  design, not for the network layer.

### Tailscale and similar overlays

Tailscale is the most common way technical owners reach home services. The
supported recipe is: bind Gunmetal's HTTPS listener to loopback, run
`tailscale serve` in front of it, add `127.0.0.1` as a `private` trusted
proxy, and set the `*.ts.net` name as the canonical URL. Points to document:

- Tailscale Serve adds Tailscale-User-Login and related identity headers
  and strips them from incoming requests, but Tailscale's own guidance is
  that the backend must listen only on localhost, or anyone who can reach
  it directly can supply their own values. Gunmetal does not use them for
  sign-in at all.
- Funnel publishes the service to the whole internet and carries no
  identity headers. Treat a Funnel proxy as `public`.
- `tailscale cert` obtains Let's Encrypt certificates, so machine names
  appear in public CT logs. Tailscale warns against sensitive machine
  names; the docs should repeat that, and suggest its randomised tailnet
  names.
- Tailscale's addresses (100.64.0.0/10, the RFC 6598 range, and
  `fd7a:115c:a1e0::/48` inside the ULA range) classify as local, so a
  direct tailnet connection to the server is in home posture without a
  proxy. Headscale and other WireGuard overlays work the same way.

### LAN discovery

- **R1 has no discovery protocol.** Browsers cannot use mDNS, and the R1
  client is a browser, so the server prints its LAN URL and a QR code on
  its console and in the admin UI instead.
- **No SSDP, UPnP or DLNA in the core, ever** (SEC-NET-059). CISA's alert
  TA14-017A lists SSDP with a bandwidth amplification factor of 30.8 and
  mDNS with 2 to 10, so a responder that answers off-link queries helps
  attack third parties. CallStranger (CVE-2020-12695, June 2020) showed
  that UPnP's SUBSCRIBE callback lets an attacker make devices send data to
  any address. If DLNA ever arrives, it is a plugin under the rules in
  SEC-NET-066.
- **mDNS from R2**, so native apps can find a server on first launch.
  Follow RFC 6762's source address check: drop queries that did not arrive
  with IP TTL 255, and unicast queries from off-link sources. Advertise a
  generic instance name such as "Gunmetal 7F3A" plus the endpoint ID; the
  endpoint ID is not secret, since only enrolled keys get past the
  handshake. Never advertise people's names, library names, the household
  name or the version unless the admin chooses to (SEC-NET-063). iroh's own
  mDNS address lookup announces the endpoint ID and local addresses, and
  iroh's docs warn that a connected peer learns your IP addresses.
- **Casting (R2)** discovers devices from the sender (the phone), not the
  server. Cast receivers cannot speak iroh, so they get capability URLs on
  the LAN name (SEC-NET-064).

### IPv6

- **IPv6 removes the accidental protection of NAT.** A server with a
  global IPv6 address is reachable from anywhere unless the router's
  firewall blocks inbound traffic. RFC 6092 recommends that home routers
  block it by default, but how many actually do is (unverified). So the
  owner may have exposed the server without doing anything. The home
  posture (SEC-NET-024) and the exposure report (SEC-NET-028) cover this.
- **Classify carefully.** Convert IPv4-mapped addresses (`::ffff:a.b.c.d`)
  before classifying; treat NAT64 (`64:ff9b::/96`), 6to4 (`2002::/16`) and
  Teredo (`2001::/32`) as non-local even though they embed IPv4 addresses
  (SEC-NET-025). Write the reference table in the test by hand from the
  RFCs, not by calling the classifier.
- **Rate-limit by /64.** One home or VPS usually gets at least a /64, so
  limits keyed on a single address are trivially bypassed by rotating
  through it (SEC-NET-052). Privacy addresses (RFC 8981) also change
  regularly, so per-address history means little.
- **Link-local addresses** need a zone index and are excluded from
  per-server names; LAN clients use IPv4 or ULA names instead.
- Both listeners and the iroh endpoint bind both families, and every test
  that exercises the posture runs on both.

### What happens when someone exposes the server by mistake

| What the owner did | What reaches Gunmetal | What Gunmetal does |
|---|---|---|
| Forwarded a port on the router | Requests from public addresses | Help page, security event, admin alert explaining that remote access needs no open port (SEC-NET-024, 027) |
| Has a global IPv6 address and a permissive router | Requests from public IPv6 addresses | The same; the admin also sees which listeners hold global addresses (SEC-NET-028) |
| Put a reverse proxy or tunnel in front without telling Gunmetal | Requests with forwarding headers from an untrusted peer | Refused with the "configure your proxy" page and an admin notice (SEC-NET-017) |
| Configured the proxy as trusted | Requests with a resolved client address | Internet posture: stricter limits, no setup, no admin unless remote administration is on (SEC-NET-019, 045) |
| Used a proxy that adds no headers, on the same host | Requests that look like loopback | Cannot be detected; the docs say so, and the opt-in external check (OD-10) catches it |
| Exposed a server before claiming it | Setup attempts from public addresses | Refused; setup needs a local address and the console code (SEC-NET-029) |

In every case, authentication and authorisation still stand on their own;
the posture only shrinks what an unauthenticated stranger can touch, from
"the whole sign-in surface and every pre-authentication bug" to "a TLS
handshake, an HTTP parser and a static page". The alert must not nag: one
per listener per day, with a count, and a single "turn on remote access
instead" button. Attackers do watch for fresh installs: Feisty Duck's
newsletter of 2022-05-31 described attackers reading CT logs to finish
WordPress installers before their owners did, within minutes of a
certificate appearing. A per-server name reveals the label, not the IP
address, but the console code makes the timing irrelevant anyway.

### Denial of service on modest hardware

Gunmetal's promise is that it runs on small machines, so its limits must be
sized for them, and the most important thing to protect is the music that
is already playing. Initial defaults (OD-11):

| Limit | Initial default | Notes |
|---|---|---|
| Concurrent HTTP connections | 1,024 total, 64 per client key | Client key is an IPv4 address or IPv6 /64 (SEC-NET-052) |
| Header read timeout | 10 s | Slowloris |
| Header block size | 16 KiB, 100 fields | ASVS 4.2.5 |
| Request body | Minimum 1 KiB/s after the first 10 s; per-route caps (small for JSON, larger for uploads) | Slow POST |
| Idle keep-alive | 60 s | |
| Stalled response | 5 min with no bytes read | A paused player stops reading; do not cut it off sooner |
| HTTP/2 | 100 concurrent streams; bounded pending and local resets; CONTINUATION and header-list caps | Rapid Reset (CVE-2023-44487), CONTINUATION flood (VU#421644, April 2024), MadeYouReset (CVE-2025-8671, August 2025) |
| Range | One range per request | Apache Killer (CVE-2011-3192) |
| Unauthenticated state | Fixed-capacity stores (for example 10,000 challenges) with eviction | SEC-NET-051 |
| iroh | 256 connections, 8 per endpoint ID, 64 streams per connection; Retry under load | Unknown keys never reach the application |

Implementation notes: h2, the HTTP/2 crate under hyper, added a limit on
reset streams waiting to be accepted in 0.3.17 (April 2023, the mechanism
Rapid Reset later abused at scale), limits on error resets in 0.4.2 and a
CONTINUATION limit in 0.4.4 (April 2024); keep it current and set the
limits explicitly rather than relying on defaults. Whether h2 has a separately named limit for server-initiated
resets (the MadeYouReset pattern) is (unverified); the integration test in
SEC-NET-049 is what proves it, whatever the knob is called. Put
concurrency limits for expensive work (SEC-NET-053) in a queue with
per-user fairness, so that a scan or a search storm waits behind playback.
Do authentication before any expensive work, and make sign-in endpoints
cheap: passkey verification is a signature check, not a password hash.
Prove all of it with the nightly load test (SEC-NET-055).

### Separating the admin surface from the media surface

The media surface renders untrusted text all day: tags, lyrics, artwork
names, metadata from the internet. An XSS there must not be able to act as
an admin. So:

- Admin rights live in a **separate admin session**, created by a fresh
  user-verifying sign-in and expiring after 15 idle minutes (SEC-NET-044).
  With passkeys this is one tap, so it does not get in the way.
- Recommended (OD-6): serve the admin UI on a **separate origin**, for
  example `a-192-168-1-7.<label>.<zone>`, which the existing wildcard
  certificate covers, with its own `__Host-` cookie. The same React Native
  web bundle can run in an admin mode. A script injected into the media
  origin cannot read or send the admin cookie.
- Admin routes are refused over internet-posture paths unless the admin
  turns remote administration on (SEC-NET-045). Over iroh, admin routes are
  open to enrolled admin devices. This is the only place network location
  matters for admin, and it only removes access.
- Operational endpoints (metrics, profiling) are off by default and
  loopback-only when on (SEC-NET-046). Unauthenticated responses carry no
  version (SEC-NET-047); the in-app advisory banner shows the version to
  signed-in admins only.
- Adapters (OpenSubsonic, Jellyfin) expose no admin operations.

### Testing the network layer

- The path-class function and the policy table are pure core code, held to
  the 100% coverage and zero-surviving-mutant rules, and tested against a
  literal table.
- Address classification, forwarding-chain resolution, host matching, the
  name-service label codec, the Range parser and the pairing decoder each
  get property tests and fuzz targets, like the media parsers.
- Socket and egress inventories (SEC-NET-031, 032) run on Linux CI in a
  network namespace with a recording DNS resolver; compare against literal
  lists in the test.
- ACME flows run against Pebble; iroh tests run two in-process endpoints
  with relays disabled, plus a local `iroh-relay` and `iroh-dns-server` for
  relay and address-lookup tests.
- Real proxies (nginx, Caddy, Traefik, cloudflared in a local mode) run as
  containers in a CI job.
- The low-end load test runs nightly under cgroup limits.
- Tailscale paths, which need a real tailnet, go on the manual release
  checklist.

## Anti-patterns

- **Trusting "local".** Passwordless or privileged access for LAN
  addresses, decided from headers an attacker controls. Emby's
  CVE-2023-33193 (CVSS 9.1, May 2023) let remote attackers pass as local;
  about 1,200 servers were backdoored. Jellyfin's CVE-2025-32012
  (GHSA-qcmf-gmhm-rfv9, fixed in 10.10.7) let anyone restart the server by
  forging a LAN address. (CWE-290, CWE-348.)
- **Believing X-Forwarded-For by default**, or reading its leftmost value.
  Navidrome's sign-in rate limit fell to forged forwarding headers in
  September 2026 (GHSA-f295-6wp9-qqfg).
- **An unauthenticated setup page on the network.** Attackers watching CT
  logs finished WordPress installs before their owners (Feisty Duck,
  2022-05-31).
- **Self-signed certificates with "just click Advanced".** It trains people
  to accept the warning a real attacker would trigger. (CWE-295.)
- **Hard-coded trust anchors in clients.** Plexamp shipped its own root
  store; when Let's Encrypt's new roots arrived, servers vanished from the
  app as their certificates renewed, forcing an emergency release in
  September 2026.
- **Serving an incomplete chain.** Plex Media Server left out the
  cross-signed Generation Y root in September 2026, and strict clients
  refused to connect.
- **Turning off certificate verification for outbound calls.** Immich's
  2026 OIDC code fetched with TLS verification off (see
  `docs/research/users-sharing-and-security.md`).
- **Loading the LAN web app from a vendor origin.** Chrome's Local Network
  Access prompt pushed Plex's web app onto the remote path inside the
  house (November 2025).
- **Opening router ports automatically for an HTTP API.** Plex uses UPnP
  and NAT-PMP to forward its port; Gunmetal never does for HTTP, and keeps
  iroh's port mapper off unless asked.
- **UPnP event callbacks to arbitrary addresses** (CallStranger,
  CVE-2020-12695) and **discovery responders that answer anyone** (SSDP and
  mDNS amplification, CISA TA14-017A).
- **Assembling multi-range responses.** Apache Killer (CVE-2011-3192,
  exploited in August 2011) exhausted servers with overlapping ranges.
- **Unbounded HTTP/2.** Rapid Reset (CVE-2023-44487, exploited from August
  to October 2023), CONTINUATION floods (2024) and MadeYouReset
  (CVE-2025-8671, 2025).
- **No Host check on a local web service.** Transmission's RPC was driven
  by DNS rebinding (CVE-2018-5702, January 2018); the fix was a host
  allowlist. (CWE-346.)
- **Building links from the Host header.** Every invite and share link
  becomes poisonable. (CWE-601.)
- **Secrets in query strings.** They end up in logs, browser history and
  Referer headers. (CWE-598.)
- **Publishing the home IP address** in a public discovery record keyed by
  a stable ID.
- **Treating overlay identity headers as sign-in** while the backend
  listens on all interfaces; Tailscale's own documentation warns against
  it.
- **Rate limits per IPv6 address.** A /64 holds 2^64 of them.
- **A plaintext "LAN fast path" for native apps.** Home networks hold
  guests, cheap cameras and other people's laptops.
- **A relay so slow that people port forward.** Plex's 2 Mbps relay cap is
  the example to avoid for music.
- **Infrastructure that can switch servers off.** In 2023 Emby shipped an
  update that stopped compromised servers from starting. Gunmetal's name
  service, relays and edge may refuse service, but must never be able to
  change or disable a server.

## Open decisions for the project owner

> **Status, 2026-10-02.** The owner decisions from every file in
> `docs/security/` are consolidated and de-duplicated in
> [README.md](README.md#open-decisions-for-the-owner). Where the baseline
> has since chosen, or where a recommendation below disagrees with the
> README, the README and the requirement tables win.

**OD-1. Run a per-server HTTPS name service?** Recommendation: yes, on by
default, with a clear "skip" in setup. Trade-offs: it is a central service
the project must keep running (if it disappears, certificates expire within
one lifetime and browsers fall back to the own-domain, Tailscale and
localhost paths); the zone operator can always obtain a certificate for any
label, which matters for browsers only; it needs a Public Suffix List entry
and possibly a raised CA rate limit; routers with DNS rebinding protection
block it until the user allows the zone; and the first-start registration
is outbound traffic before the owner has clicked anything. Without it,
non-technical users have no passkey-capable browser path at all in R1.
Also to decide: the zone name (it should not be the docs domain), and the
CA (Let's Encrypt, with a second ACME CA as fallback).

**OD-2. How do browsers reach the server from outside the home in R1?**
Recommendation: the TLS-passthrough edge over iroh, opt-in at setup. It
keeps TLS and the app code on the user's server, and makes passkeys work
everywhere. Trade-offs: the project runs and pays for an edge, every
listener-hour of lossless music costs roughly 0.3 to 0.5 GB of edge traffic
(unverified estimate), and an enabled server's sign-in page becomes
reachable from the internet (behind a secret name). The hosted-WASM
alternative needs no edge but makes the project's hosting a single point of
compromise and runs into Local Network Access prompts. Deferring remote
access to R2 (Tailscale-only in R1) costs nothing but invites port
forwarding. If remote access moves to R2, SEC-NET-033 to 043 move with it;
the posture, proxy and exposure requirements stay R1.

**OD-3. Who runs the relays and the edge, and what do they log?**
Recommendation: the project runs default relays and the edge, publishes a
policy of no per-connection logs beyond short-lived abuse counters, and lets
any admin replace them with a self-hosted relay in one setting. Trade-offs:
cost and an operational duty, against n0's managed relays (paid, and a
third party sees the metadata) or self-hosting only (hard for
non-technical users). n0's public relays are not for production.

**OD-4. Address lookup.** Recommendation: run an `iroh-dns-server` for the
project, publishing relay URLs only, so servers can change relay without
re-inviting everyone. Trade-off: another service; the alternatives are n0's
`dns.iroh.link`, the Mainline DHT (no operator, but public), or no lookup
with relay URLs carried in invites (simplest, but a relay change breaks
existing clients).

**OD-5. iroh port mapping.** Recommendation: off by default, offered as a
one-click option when the connection report shows mostly relayed sessions.
Trade-off: more direct connections (lower relay cost and less metadata for
the relay operator) against the principle that Gunmetal does not touch the
router unasked. The mapped port only reaches the iroh endpoint, which
refuses unknown keys.

**OD-6. A separate origin for the admin UI?** Recommendation: yes for
per-server names (no extra certificate needed) and documented for
own-domain setups. Trade-off: protection from XSS in the media UI, against
a second sign-in step (one passkey tap) and some build complexity.

**OD-7. Fail closed on untrusted forwarding headers?** Recommendation: yes
(SEC-NET-017), with an explicit "ignore forwarding headers" setting for
odd setups. Trade-off: a little first-time friction for reverse-proxy
users, against the server not knowing it is on the internet.

**OD-8. TLS crypto provider and post-quantum key exchange.**
Recommendation: aws-lc-rs with the hybrid X25519MLKEM768 group preferred,
if it builds cleanly for every R1 target (including 32-bit ARM and
FreeBSD); otherwise ring for R1 and the hybrid group in R2 (SEC-NET-065).
Trade-off: aws-lc-rs brings C and assembly into the server build; listening
history is low-value to a long-term decryptor, but the change is cheap.

**OD-9. Ports and the plaintext listener.** Recommendation: keep a
redirect-only plaintext listener, because people type IP addresses, and
choose default ports that do not clash with Plex, Jellyfin or Emby so the
servers can run side by side during migration.

**OD-10. An opt-in external exposure check?** Recommendation: offer one,
run from the project edge: on request it tries to reach the server's public
address on its HTTP ports and reports the result, with no logging.
Trade-off: it is the only way to see a port forward or a header-less proxy
from outside, but it is outbound contact the user must choose.

**OD-11. Limit defaults and the reference hardware.** Recommendation: adopt
the table under "Denial of service on modest hardware" and the 2-core,
1 GiB profile for SEC-NET-055, and revisit them after the scan benchmark
from ADR 0001 gives real numbers.

**OD-12. Revocation latency.** Recommendation: 5 seconds (SEC-NET-034,
043). Trade-off: a shorter target needs push-based revocation to every
connection; a longer one leaves a stolen phone playing.

## Sources

Standards and specifications
- OWASP ASVS 5.0.0, chapters V2, V3, V4, V6, V8, V11, V12, V13, V14, V15 and V16: https://github.com/OWASP/ASVS/tree/master/5.0/en
- OWASP ASVS 5.0 release at Global AppSec EU, 2025-05-30: https://owasp2025globalappseceu.sched.com/event/1whCc/introducing-the-50-release-of-the-asvs
- OWASP Top 10:2025: https://top10.owasp.org/2025
- OWASP API Security Top 10 2023: https://api-security.owasp.org/editions/2023/en/0x11-t10
- OWASP MASVS network controls: https://mas.owasp.org/MASVS/08-MASVS-NETWORK/ and https://mas.owasp.org/MASVS/controls/MASVS-NETWORK-2/
- NIST SP 800-52 Rev. 2: https://csrc.nist.gov/pubs/sp/800/52/r2/final
- NIST SP 800-218 (SSDF 1.1); SSDF 1.2 (SP 800-218r1) was an initial public draft dated 2025-12-17 when checked: https://csrc.nist.gov/pubs/sp/800/218/r1/ipd
- RFC 8446 (TLS 1.3), RFC 8996 (deprecating TLS 1.0 and 1.1), RFC 9325 (BCP 195), RFC 7250 (raw public keys), RFC 9000 and RFC 9001 (QUIC), RFC 8555 (ACME), RFC 9773 (ARI, June 2025), RFC 8657 and RFC 8659 (CAA), RFC 6797 (HSTS), RFC 7239 (Forwarded), RFC 9110, RFC 9112 and RFC 9113 (HTTP), RFC 6762 and RFC 6763 (mDNS, DNS-SD), RFC 6886 (NAT-PMP), RFC 6887 (PCP), RFC 1918, RFC 4193, RFC 4291, RFC 6598, RFC 6052, RFC 3056, RFC 4380, RFC 6092, RFC 8981: https://www.rfc-editor.org/
- RFC 9110 range guidance: https://httpwg.org/specs/rfc9110.html
- CA/Browser Forum ballot SC-081v3: https://cabforum.org/2025/04/11/ballot-sc081v3-introduce-schedule-of-reducing-validity-and-data-reuse-periods/

iroh
- Relays: https://docs.iroh.computer/concepts/relays
- Security and privacy: https://docs.iroh.computer/concepts/security-privacy
- Public relay terms: https://docs.iroh.computer/iroh-services/relays/public
- Browser (WASM) support: https://docs.iroh.computer/deployment/wasm-browser-support
- Address lookup: https://docs.iroh.computer/concepts/address-lookup
- Local (mDNS) discovery: https://docs.iroh.computer/connecting/local-discovery
- Crate docs, version 1.3.0, and endpoint builder defaults: https://docs.rs/iroh/latest/iroh/ and https://docs.rs/iroh/latest/iroh/endpoint/struct.Builder.html
- Raw public keys in 0.34: https://iroh.computer/blog/iroh-0-34-0-raw-public-keys
- 1.0.0-rc.1 (relay access control): https://iroh.computer/blog/iroh-1-0-0-rc-1
- 1.0 release coverage: https://www.techtimes.com/articles/318490/20260616/peer-peer-library-iroh-10-ships-dial-devices-key-not-ip-address.htm

Certificates and browsers
- Plex's per-server certificates: https://words.filippo.io/how-plex-is-doing-https-for-all-its-users/
- Plex incomplete chain, September 2026: https://forums.plex.tv/t/https-certs-are-missing-trust-roots/943238
- Plexamp trust store, September 2026: https://forums.plex.tv/t/plexamp-v4-50-3-ready-or-not/942338
- Plex and Chrome Local Network Access, November 2025 (via docs/research/pain-points-and-demand.md): https://forums.plex.tv/t/important-note-about-the-plex-web-app-local-network-access/933264
- Let's Encrypt lifetimes: https://letsencrypt.org/2025/12/02/from-90-to-45
- Let's Encrypt 6-day and IP certificates: https://letsencrypt.org/2026/01/15/6day-and-ip-general-availability
- Let's Encrypt DNS-PERSIST-01: https://letsencrypt.org/2026/02/18/dns-persist-01
- Let's Encrypt ending OCSP: https://letsencrypt.org/2024/12/05/ending-ocsp/
- Let's Encrypt Generation Y hierarchy: https://letsencrypt.org/2025/11/24/gen-y-hierarchy
- Let's Encrypt ARI as RFC: https://letsencrypt.org/2025/09/16/ari-rfc
- Let's Encrypt rate limits: https://letsencrypt.org/docs/rate-limits/
- rustls defaults (0.23.45): https://docs.rs/rustls/latest/rustls/manual/_05_defaults/index.html
- Chrome Local Network Access: https://developer.chrome.com/blog/local-network-access
- CT logs used against WordPress installs: https://www.feistyduck.com/newsletter/issue_89_certificate_transparency_data_is_used_to_compromise_wordpress_before_installation

Tailscale
- HTTPS certificates: https://tailscale.com/kb/1153/enabling-https
- Serve and identity headers: https://tailscale.com/kb/1312/serve
- Funnel: https://tailscale.com/kb/1223/funnel

Incidents and advisories
- CVE-2023-33193 (Emby): https://cveawg.mitre.org/api/cve/CVE-2023-33193
- CVE-2025-32012 (Jellyfin): https://osv.dev/vulnerability/CVE-2025-32012
- Navidrome GHSA-f295-6wp9-qqfg: https://github.com/navidrome/navidrome/security/advisories
- CVE-2018-5702 (Transmission DNS rebinding): https://openwall.com/lists/oss-security/2018/01/15/1
- CVE-2020-12695 (CallStranger): https://www.tenable.com/blog/cve-2020-12695-callstranger-vulnerability-in-universal-plug-and-play-upnp-puts-billions-of
- CISA TA14-017A: https://www.cisa.gov/news-events/alerts/2014/01/17/udp-based-amplification-attacks
- CVE-2011-3192: https://cveawg.mitre.org/api/cve/CVE-2011-3192
- CVE-2023-44487: https://cveawg.mitre.org/api/cve/CVE-2023-44487
- CVE-2025-8671 (MadeYouReset): https://access.redhat.com/security/cve/cve-2025-8671
- HTTP/2 CONTINUATION flood and hyper: https://seanmonstar.com/blog/hyper-http2-continuation-flood/
- h2 changelog: https://github.com/hyperium/h2/blob/master/CHANGELOG.md
- Beauty and the Burst (USENIX Security 2017): https://usenix.org/conference/usenixsecurity17/technical-sessions/presentation/schuster

Project documents
- docs/adr/0001-architecture.md and docs/adr/0002-music-is-first-class.md
- docs/research/users-sharing-and-security.md (Emby 2023 backdoors, Plex relay cap, Immich OIDC, Jellyfin 12.1 revocation fix)
- docs/research/setup-migration-and-operations.md (first-run setup code, passkeys need HTTPS)
- docs/research/clients-platforms-and-offline.md (casting, DLNA, background transfers)
- docs/research/pain-points-and-demand.md (selfh.st 2025 survey, Plex Local Network Access note, Plexamp)
