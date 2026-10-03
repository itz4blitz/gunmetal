# Standards coverage

Date: 2026-10-02. Status: proposed.

Web tools were available and used. Every standard version below was checked
against its primary source on 2026-10-02: the OWASP ASVS release feed and
its 5.0.0 CSV, the OWASP Top 10 and API Security sites, the OWASP MASVS
repository and MAS site, the MITRE CWE Top 25 pages, and the NIST CSRC SSDF
page and PDFs. Anything not confirmed is marked "(unverified)".

## Summary

This file checks the 835 requirements in the eleven other files of
`docs/security/` against the current version of each standard the owner
named, and adds requirements (SEC-STD-001 onwards) for what is missing.

The baseline is strong. Of the ASVS 5.0.0 requirements outside the WebRTC
chapter, 259 of 334 are already cited by at least one Gunmetal requirement.
Every category of the OWASP Top 10:2025 and the API Security Top 10 2023 is
covered by several requirements, and 20 of the 25 CWE Top 25 (2025)
weaknesses are cited by name. The other five (four memory-safety
weaknesses and CWE-77 command injection) are covered by design but not
cited. Supply chain, parsers, authorization and privacy are
well beyond Level 2 already.

What is missing falls into five groups:

1. **The documents contradict each other on how people sign in.** The
   identity file forbids passwords and TOTP outright (SEC-IAM-025). The
   threat model allows a password fallback (SEC-TM-013), and the accounts
   feature map ships passwords and TOTP in R1 (ACC-052, ACC-053). The threat
   model also lets a browser use the whole application over plain HTTP on
   the LAN, with a password (SEC-TM-007 and its session table), which the
   network file forbids (SEC-NET-001). It does this while citing ASVS 12.2.1,
   a Level 1 requirement that it breaks. Until one architecture record
   settles this, nobody can claim ASVS chapter 6 at any level. SEC-STD-006
   and SEC-STD-007 settle it. The recommendation is to keep SEC-IAM-025 and
   SEC-NET-001: passkeys, device keys and OIDC, with no password and no
   application served over cleartext.
2. **The development process has no proof that requirements become
   tests.** No requirement links the 835 SEC IDs to the tests that prove
   them (SSDF PW.1.2). No requirement asks for an independent review of the
   design (PW.2.1), for review rules covering security-sensitive source and
   code written by coding agents (PW.7), for dynamic testing of the running
   server (PW.8.2), or for compiler hardening (PW.6.2). SEC-STD-004, 005 and
   033 to 038 add them.
3. **Cryptography is specified by purpose but not by discipline.** Nothing
   yet names the approved implementations, forbids weak modes, sets nonce
   limits for AES-GCM (which SEC-EXT-050 mandates), says how crypto errors
   fail, or keeps the main server process from dumping secrets to disk.
   SEC-NET-065 cites the crypto-inventory requirement (ASVS 11.1.4) but only
   covers post-quantum key exchange. SEC-STD-018 to 024 fill the chapter to
   Level 3.
4. **A handful of untrusted-input paths have no owner.** Restoring a
   backup at setup, and the planned importers for rival databases, hand an
   attacker's SQLite file to SQLite's C code in the server process
   (SEC-STD-031). User-written patterns in smart playlists and live-TV
   filters reach a regex engine (SEC-STD-011). Tag keys from media files can
   pollute JavaScript prototypes in the client (SEC-STD-012). Several
   single-use secrets have no rule that makes them single-use under
   concurrent requests (SEC-STD-029).
5. **Not-applicable claims are not enforced.** ASVS chapters on SAML, LDAP,
   GraphQL, OpenID Providers and WebRTC are N/A only because Gunmetal does
   not use those technologies. SEC-STD-010 and SEC-STD-026 turn that into a
   CI check, so the N/A stays true.

Recommended ASVS targets: Level 3 for every chapter except business logic
(chapter 2, Level 2) and WebRTC (chapter 17, not applicable), with
nine recorded deviations. That is above the "Level 2 by default" starting
point because nearly every chapter either protects sign-in, sessions,
cryptography or the host, or already meets all of its Level 3 items, so
claiming Level 3 costs nothing and stops it from slipping. For the native
apps the target is the MAS-L2 and MAS-P profiles, with MAS-R excluded by an
architecture record.

## How this was checked

1. Every row of every requirement table in `docs/security/` was parsed
   (835 rows: threat-model 71, identity 105, web and API 98, media 80,
   plugins 76, operations 74, clients 72, network 67, supply chain 66,
   rival history 66, privacy 60). The "Standards" column was read for ASVS
   requirement numbers, Top 10 and API Top 10 IDs, MASVS control IDs, CWE
   IDs and SSDF practice and task IDs.
2. Each cited ASVS number was checked against the ASVS 5.0.0 CSV. All of
   them exist, so no file still uses ASVS 4.0.3 numbering. No file cites a
   2021 Top 10 ID.
3. A citation is a claim, not proof. For each ASVS requirement with no
   citation, the requirement text of all eleven files was searched for a
   control that meets it anyway. Those are marked "covered, uncited" below,
   with the IDs. Spot checks of cited mappings found one wrong citation
   (SEC-NET-065 for ASVS 11.1.4).
4. Gaps became SEC-STD requirements. Where a gap is deliberate and already
   argued in another file, it is listed as a deviation instead.

Limits: the coverage figures count citations, so they show breadth, not
depth. Whether a requirement fully meets the standard item it cites is for
the traceability check (SEC-STD-004) and the independent review
(SEC-STD-034) to confirm.

## Standard versions

| Standard | Version used | Status on 2026-10-02 | Notes |
|---|---|---|---|
| OWASP ASVS | 5.0.0 (May 2025) | Latest stable release | The project's rolling "bleeding edge" build (regenerated 2026-09-03) is not a release and is not used. |
| OWASP Top 10 | 2025 | Current edition on top10.owasp.org | The pages fetched do not state a final-release date (unverified). Categories A01 to A10 checked against the site. |
| OWASP API Security Top 10 | 2023 | Current; no newer edition or candidate announced | |
| OWASP MASVS | 2.1.0 (January 2024) | Latest release | The repository was still being edited in September 2026, reworking the MAS Testing Profiles; no new MASVS release. MASTG 2.0.0 (June 2026) is the matching test guide. |
| MAS Testing Profiles | MAS-L1, MAS-L2, MAS-R, MAS-P | Current on mas.owasp.org | Replaced the old L1, L2 and R levels. |
| CWE Top 25 | 2025 | Latest edition | The list page gives 2025-12-15 as its publication date; the CWE landing page shows a later date, probably a page revision (unverified). No 2026 edition yet. |
| NIST SP 800-218 (SSDF) | 1.1 (February 2022) | Final | SP 800-218 Rev. 1 (SSDF 1.2) is still an initial public draft (2025-12-17). Its two new practices, PO.6 and PS.4, are tracked below as forward-looking items. |

## Conflicts that block a coverage claim

These are not gaps against a standard. They are places where two Gunmetal
documents gave opposite answers, so a reviewer could not say which one the
product would meet. All four were resolved in the 2026-10-02 challenge
review as recommended here; [README.md](README.md) records the outcome and
the remaining owner decisions.

| Topic | One file says | Another file says | Resolution proposed |
|---|---|---|---|
| Passwords and TOTP | SEC-IAM-025 (R1): no account passwords, no TOTP | SEC-TM-013 (R1): passwords allowed where the owner enables them; threat-model asset AS-3 lists TOTP secrets; ACC-052 and ACC-053 ship password fallback and TOTP in R1 | SEC-STD-006. Recommendation: keep SEC-IAM-025. If the owner keeps passwords, SEC-STD-008 and SEC-STD-009 apply in full. |
| Cleartext on the LAN | SEC-NET-001 (R1): no credentials or API over plaintext from a non-loopback peer | SEC-TM-007 (R1) and the threat-model sign-in table: the application and password sign-in work over plain HTTP for local peers, with 24-hour sessions | SEC-STD-007. SEC-TM-007 cites ASVS 12.2.1 (Level 1) while permitting what that requirement forbids. |
| Native access-token lifetime | SEC-CLI-034: at most 15 minutes | Threat-model session table: at most one hour | Resolved 2026-10-02: 10 minutes with no refresh tokens (SEC-IAM-050); SEC-CLI-034 withdrawn and the threat-model table corrected. |
| ASVS 11.1.4 | SEC-NET-065 cites it | SEC-NET-065 only sets the TLS key-exchange group | SEC-STD-018 owns the inventory and migration plan; SEC-NET-065 should drop the 11.1.4 citation. |

## OWASP ASVS 5.0.0

### Target level per chapter

The starting point is Level 2. A chapter goes to Level 3 when it protects
sign-in, sessions, cryptography or the host, or when the baseline already
meets every Level 3 item in it, so that claiming Level 3 costs nothing and
a regression would be visible. "Uncited at target" counts the requirements
at or below the target level that no Gunmetal requirement cites, before
the SEC-STD additions.

| Chapter | Items (L1/L2/L3) | Target | Why | Uncited at target | After this file |
|---|---|---|---|---|---|
| V1 Encoding and Sanitization | 8/19/3 | **L3** | Untrusted strings from media files and providers reach paths, process arguments and SQL on the host, and V1.4 is memory safety. | 10 | Covered, with 1.3.8 N/A (Java-only) and four items N/A by SEC-STD-010 |
| V2 Validation and Business Logic | 4/7/2 | **L2** | A household media server has no high-value transactions. 2.4.2 (human timing) would break TV remotes, bulk imports and automation, which rate limits handle instead. 2.3.5 is met anyway by owner transfer. | 4 | Covered by SEC-STD-030 |
| V3 Web Frontend Security | 8/11/12 | **L3** with deviations | The web client renders attacker-controlled tags; every Level 3 item but four is already cited, and those four are cheap. | 6 | Covered; deviations 3.4.7 and 3.7.4 (owners' own domains) |
| V4 API and Web Service | 2/8/6 | **L3** | All Level 3 items (method allow-list, request smuggling, message signatures) are already cited. | 1 | Covered; 4.3.2 N/A (no GraphQL) |
| V5 File Handling | 4/5/4 | **L3** | Untrusted files are parsed with the server's privileges; this chapter protects the host. | 1 | Covered; deviation 5.4.3 (re-encoding instead of virus scanning) |
| V6 Authentication | 13/22/12 | **L3** | Owner-mandated. | 12 | Covered once SEC-STD-006 is decided; 6.6.1 and 6.8.3 N/A |
| V7 Session Management | 6/12/1 | **L3** | Owner-mandated. | 0 | Covered |
| V8 Authorization | 4/3/6 | **L3** | Administrator actions are host-equivalent (library roots, plugins, proxies), and cross-user leaks were the commonest rival advisories. | 1 | Covered by SEC-STD-028 |
| V9 Self-contained Tokens | 4/3/0 | **L3** (no Level 3 items) | Signed stream and image URLs are bearer tokens. | 0 | Covered |
| V10 OAuth and OIDC | 5/24/7 | **L3** for the OIDC client and for Gunmetal's own token issuance | It is sign-in. Gunmetal is an OIDC relying party and issues its own device tokens; it is not an OAuth authorization server for third parties or an OpenID Provider. | 18 | Covered or N/A, enforced by SEC-STD-025 and SEC-STD-026 |
| V11 Cryptography | 3/11/10 | **L3** with one deviation | Owner-mandated. | 11 | Covered; deviation 11.7.1 (memory encryption on the owner's hardware) |
| V12 Secure Communication | 3/6/3 | **L3** with two deviations | Transport carries passkey ceremonies, sessions and history. | 3 | Covered; deviations 12.1.4 (OCSP) and 12.1.5 (ECH, Later) |
| V13 Configuration | 1/12/8 | **L3** | Secrets management and information leakage protect the host. | 3 | Covered by SEC-STD-017 |
| V14 Data Protection | 2/7/4 | **L3** | Watch and listening history is sensitive; all four Level 3 items are already adopted by the privacy file. | 0 | Covered |
| V15 Secure Coding and Architecture | 3/10/8 | **L3** | Dependency risk, the sandbox and safe concurrency protect the host. | 3 | Covered by SEC-STD-012 and SEC-STD-029 |
| V16 Security Logging and Error Handling | 0/16/1 | **L3** | The single Level 3 item (a last-resort error handler) is already cited. | 0 | Covered |
| V17 WebRTC | 0/7/5 | **N/A** | No WebRTC anywhere in the design. | n/a | SEC-STD-010 keeps it N/A |

### Recorded deviations

Each is a deliberate choice argued elsewhere; SEC-STD-002 keeps them in one
register with a reason, a compensating control and a review date.

| ASVS | Level | Deviation | Compensating control | Argued in |
|---|---|---|---|---|
| 3.4.1 | L2 | `includeSubDomains` only on names the project issues or the owner vouches for | SEC-STD-016 preloads the project's own domains | web-and-api-security.md |
| 3.4.3 | L3 part | `'self'` allow-list with Trusted Types instead of per-response nonces | No inline script; bundle embedded in the binary | web-and-api-security.md |
| 3.4.7 | L3 | No CSP report endpoint | Every screen visited in CI with violations failing the build (SEC-API-044) | web-and-api-security.md |
| 3.7.4 | L3 | No preload for owners' own domains | Project-run domains preloaded (SEC-STD-016) | web-and-api-security.md |
| 5.4.3 | L2 | No virus scanning | Artwork decoded and re-encoded; file-derived bytes never served with a script-capable type (SEC-API-051, SEC-MED-059) | web-and-api-security.md |
| 11.7.1 | L3 | No full memory encryption requirement | Secrets zeroed and kept out of dumps (SEC-TM-049, SEC-STD-023) | this file |
| 12.1.4 | L3 | No OCSP stapling | Public CAs stopped OCSP in 2025; short certificate lifetimes (SEC-NET-004) | network-and-remote-access.md |
| 12.1.5 | L3 | No Encrypted Client Hello in R1 and R2 | Random per-server labels (SEC-NET-010); native clients use iroh | network-and-remote-access.md |
| 2.4.2 | L3 | Not adopted (chapter target is L2) | Rate limits and per-target lockouts (SEC-API-056, SEC-API-057) | this file |

### Section by section

"Cited" is the number of requirements in the section that at least one
Gunmetal requirement cites. The last column explains every uncited item.

| ASVS section | Cited | Gunmetal requirements citing it | Uncited items and their status |
|---|---|---|---|
| 1.1 Encoding and Sanitization Architecture | 1/2 | SEC-API-048, SEC-CLI-025, SEC-MED-013, SEC-TM-031 | **1.1.2** (L2): Covered, uncited: encoding happens at the sink through types (SEC-TM-031, SEC-API-046, SEC-API-047) |
| 1.2 Injection Prevention | 5/10 | SEC-API-046, SEC-API-047, SEC-API-063, SEC-API-066, SEC-API-071, SEC-CLI-001 and 17 more | **1.2.3** (L1): Gap: SEC-STD-013; **1.2.6** (L2): N/A, no LDAP: SEC-STD-010; **1.2.7** (L2): N/A unless XPath is adopted: SEC-STD-010; **1.2.8** (L2): N/A, no LaTeX: SEC-STD-010; **1.2.9** (L2): Gap: SEC-STD-011 |
| 1.3 Sanitization | 8/12 | SEC-API-044, SEC-API-045, SEC-API-046, SEC-API-048, SEC-API-077, SEC-API-079 and 36 more | **1.3.8** (L2): N/A, JNDI is Java-only; **1.3.9** (L2): N/A, no memcache: SEC-STD-010; **1.3.11** (L2): Gap once email alerts exist (SEC-OPS-035): SEC-STD-032; **1.3.12** (L3): Gap: SEC-STD-011 |
| 1.4 Memory, String, and Unmanaged Code | 3/3 | SEC-CLI-051, SEC-HIS-036, SEC-HIS-037, SEC-MED-002, SEC-MED-004, SEC-TM-032 and 1 more | All cited |
| 1.5 Safe Deserialization | 3/3 | SEC-API-024, SEC-API-067, SEC-API-081, SEC-EXT-025, SEC-EXT-034, SEC-HIS-034 and 6 more | All cited |
| 2.1 Validation and Business Logic Documentation | 0/3 | none | **2.1.1** (L1): Gap: SEC-STD-030; **2.1.2** (L2): Gap: SEC-STD-030; **2.1.3** (L2): Gap: SEC-STD-030 |
| 2.2 Input Validation | 2/3 | SEC-API-043, SEC-API-048, SEC-API-060, SEC-API-067, SEC-API-081, SEC-API-090 and 11 more | **2.2.3** (L2): Gap: SEC-STD-030 |
| 2.3 Business Logic Security | 4/5 | SEC-API-006, SEC-API-012, SEC-API-073, SEC-HIS-008, SEC-IAM-003, SEC-IAM-009 and 5 more | **2.3.5** (L3): Above target, but met by owner transfer needing both parties (SEC-IAM-003) |
| 2.4 Anti-automation | 1/2 | SEC-API-057, SEC-API-064, SEC-EXT-023, SEC-EXT-033, SEC-EXT-048, SEC-EXT-062 and 12 more | **2.4.2** (L3): Above target; not adopted (TV remotes, imports and automation are fast by nature; SEC-API-057 rate limits instead) |
| 3.1 Web Frontend Security Documentation | 1/1 | SEC-API-052 | All cited |
| 3.2 Unintended Content Interpretation | 3/3 | SEC-API-045, SEC-API-046, SEC-API-051, SEC-API-090, SEC-CLI-001, SEC-CLI-003 and 15 more | All cited |
| 3.3 Cookie Setup | 4/5 | SEC-API-032, SEC-CLI-006, SEC-HIS-029, SEC-IAM-011, SEC-IAM-039, SEC-NET-008 and 2 more | **3.3.5** (L3): Gap: SEC-STD-014 |
| 3.4 Browser Security Mechanism Headers | 7/8 | SEC-API-038, SEC-API-040, SEC-API-044, SEC-API-053, SEC-API-054, SEC-API-098 and 18 more | **3.4.7** (L3): Deviation recorded in web-and-api-security.md (no unauthenticated report endpoint) |
| 3.5 Browser Origin Separation | 6/8 | SEC-API-029, SEC-API-033, SEC-API-034, SEC-API-035, SEC-API-036, SEC-API-050 and 10 more | **3.5.6** (L3): Gap: SEC-STD-013; **3.5.7** (L3): Gap: SEC-STD-013 |
| 3.6 External Resource Integrity | 1/1 | SEC-API-049, SEC-CLI-012, SEC-PRV-018, SEC-SUP-037 | All cited |
| 3.7 Other Browser Security Considerations | 3/5 | SEC-API-047, SEC-API-052, SEC-API-070, SEC-CLI-002, SEC-CLI-069, SEC-HIS-032 and 4 more | **3.7.3** (L3): Gap: SEC-STD-015; **3.7.4** (L3): Deviation for owners' own domains; gap for project-run domains: SEC-STD-016 |
| 4.1 Generic Web Service Security | 5/5 | SEC-API-007, SEC-API-008, SEC-API-035, SEC-API-037, SEC-API-054, SEC-API-069 and 16 more | All cited |
| 4.2 HTTP Message Structure Validation | 5/5 | SEC-API-009, SEC-NET-018, SEC-NET-021, SEC-NET-048, SEC-TM-068 | All cited |
| 4.3 GraphQL | 1/2 | SEC-API-063 | **4.3.2** (L2): N/A, no GraphQL: SEC-STD-010 |
| 4.4 WebSocket | 4/4 | SEC-API-037, SEC-API-041, SEC-API-042, SEC-IAM-016, SEC-NET-020, SEC-TM-009 | All cited |
| 5.1 File Handling Documentation | 1/1 | SEC-API-085 | All cited |
| 5.2 File Upload and Content | 6/6 | SEC-API-060, SEC-API-065, SEC-API-085, SEC-API-086, SEC-API-088, SEC-API-089 and 19 more | All cited |
| 5.3 File Storage | 3/3 | SEC-API-018, SEC-API-022, SEC-API-051, SEC-API-082, SEC-API-087, SEC-API-089 and 20 more | All cited |
| 5.4 File Download | 2/3 | SEC-API-051, SEC-API-085, SEC-HIS-015, SEC-HIS-031, SEC-MED-059 | **5.4.3** (L2): Deviation recorded in web-and-api-security.md (artwork is re-encoded instead of virus-scanned) |
| 6.1 Authentication Documentation | 2/3 | SEC-API-056, SEC-CLI-029, SEC-HIS-004, SEC-HIS-046, SEC-IAM-002, SEC-IAM-025 and 3 more | **6.1.2** (L2): Gap: SEC-STD-008 |
| 6.2 Password Security | 6/12 | SEC-CLI-028, SEC-HIS-045, SEC-TM-013 | **6.2.2** (L1): Blocked by the password conflict (SEC-STD-006); SEC-STD-008 if any password exists; **6.2.3** (L1): Blocked by SEC-STD-006; SEC-STD-008; **6.2.5** (L1): Blocked by SEC-STD-006; SEC-STD-008; **6.2.9** (L2): Blocked by SEC-STD-006; SEC-STD-008; **6.2.10** (L2): Blocked by SEC-STD-006; SEC-STD-008; **6.2.11** (L2): Gap: SEC-STD-008 |
| 6.3 General Authentication Security | 8/8 | SEC-API-001, SEC-API-011, SEC-API-056, SEC-API-057, SEC-API-058, SEC-API-093 and 47 more | All cited |
| 6.4 Authentication Factor Lifecycle and Recovery | 6/6 | SEC-API-006, SEC-API-096, SEC-EXT-013, SEC-HIS-008, SEC-HIS-049, SEC-IAM-024 and 12 more | All cited |
| 6.5 General Multi-factor authentication requirements | 6/8 | SEC-API-059, SEC-CLI-022, SEC-CLI-038, SEC-EXT-014, SEC-EXT-071, SEC-IAM-007 and 9 more | **6.5.7** (L3): Covered, uncited: biometrics only unlock a hardware key (SEC-CLI-059, SEC-IAM-048); **6.5.8** (L3): N/A if SEC-IAM-025 stands; SEC-STD-009 if TOTP is kept |
| 6.6 Out-of-Band authentication mechanisms | 2/4 | SEC-API-006, SEC-API-056, SEC-API-059, SEC-CLI-038, SEC-EXT-071, SEC-HIS-008 and 4 more | **6.6.1** (L2): N/A, SMS and voice codes are forbidden (SEC-IAM-025, SEC-STD-010); **6.6.4** (L3): Gap: SEC-STD-027 (number matching already in SEC-CLI-038) |
| 6.7 Cryptographic authentication mechanism | 2/2 | SEC-CLI-043, SEC-IAM-019, SEC-IAM-048, SEC-TM-013, SEC-TM-063 | All cited |
| 6.8 Authentication with an Identity Provider | 3/4 | SEC-IAM-028, SEC-IAM-031, SEC-IAM-036, SEC-IAM-081, SEC-TM-022 | **6.8.3** (L2): N/A, no SAML: SEC-STD-010 |
| 7.1 Session Management Documentation | 3/3 | SEC-IAM-035, SEC-IAM-041, SEC-IAM-102, SEC-TM-015 | All cited |
| 7.2 Fundamental Session Management Security | 4/4 | SEC-API-003, SEC-API-004, SEC-API-032, SEC-API-042, SEC-CLI-034, SEC-CLI-071 and 9 more | All cited |
| 7.3 Session Timeout | 2/2 | SEC-API-027, SEC-CLI-010, SEC-IAM-041 | All cited |
| 7.4 Session Termination | 5/5 | SEC-API-003, SEC-API-017, SEC-API-028, SEC-API-039, SEC-CLI-009, SEC-CLI-022 and 23 more | All cited |
| 7.5 Defenses Against Session Abuse | 3/3 | SEC-API-021, SEC-CLI-014, SEC-CLI-022, SEC-CLI-059, SEC-EXT-012, SEC-EXT-014 and 19 more | All cited |
| 7.6 Federated Re-authentication | 2/2 | SEC-API-059, SEC-IAM-035, SEC-IAM-058 | All cited |
| 8.1 Authorization Documentation | 4/4 | SEC-API-001, SEC-API-014, SEC-API-019, SEC-API-068, SEC-API-075, SEC-CLI-024 and 10 more | All cited |
| 8.2 General Authorization Design | 3/4 | SEC-API-001, SEC-API-002, SEC-API-010, SEC-API-011, SEC-API-012, SEC-API-013 and 85 more | **8.2.4** (L3): Gap: SEC-STD-028 |
| 8.3 Operation Level Authorization | 3/3 | SEC-API-010, SEC-API-013, SEC-API-017, SEC-API-020, SEC-API-025, SEC-API-028 and 29 more | All cited |
| 8.4 Other Authorization Considerations | 2/2 | SEC-API-075, SEC-CLI-014, SEC-HIS-001, SEC-IAM-013, SEC-IAM-049, SEC-NET-044 and 5 more | All cited |
| 9.1 Token source and integrity | 3/3 | SEC-API-025, SEC-API-026, SEC-HIS-042, SEC-IAM-027, SEC-IAM-045, SEC-IAM-054 and 1 more | All cited |
| 9.2 Token content | 4/4 | SEC-API-026, SEC-API-027, SEC-HIS-006, SEC-HIS-042, SEC-IAM-027, SEC-IAM-045 and 2 more | All cited |
| 10.1 Generic OAuth and OIDC Security | 2/2 | SEC-CLI-026, SEC-EXT-031, SEC-TM-022 | All cited |
| 10.2 OAuth Client | 2/3 | SEC-CLI-026, SEC-CLI-040, SEC-EXT-031, SEC-IAM-026, SEC-IAM-034, SEC-TM-022 | **10.2.3** (L3): Gap: SEC-STD-025 |
| 10.3 OAuth Resource Server | 3/5 | SEC-CLI-034, SEC-IAM-028, SEC-IAM-036, SEC-IAM-050 | **10.3.1** (L2): Covered, uncited: opaque tokens checked against this server's state (SEC-TM-015) and bound to it (SEC-IAM-050); **10.3.2** (L2): Covered, uncited: tokens carry no claims; the policy layer decides (SEC-API-010, SEC-IAM-083) |
| 10.4 OAuth Authorization Server | 7/16 | SEC-CLI-026, SEC-CLI-034, SEC-CLI-040, SEC-CLI-070, SEC-IAM-026, SEC-IAM-033 and 2 more | **10.4.2** (L1): Covered by analogy: pairing codes single-use (SEC-API-059, SEC-TM-021); race-proof via SEC-STD-029; **10.4.3** (L1): Covered by analogy: pairing codes expire within 10 minutes (SEC-API-059); **10.4.7** (L2): N/A, no dynamic client registration: SEC-STD-026; **10.4.10** (L2): N/A, no confidential third-party clients: SEC-STD-026; **10.4.11** (L2): Covered, uncited: scoped keys (SEC-IAM-083); **10.4.12** (L3): N/A, no redirect-based authorization server: SEC-STD-026; **10.4.13** (L3): N/A: SEC-STD-026; **10.4.15** (L3): N/A: SEC-STD-026; **10.4.16** (L3): N/A: SEC-STD-026; native clients prove a device key instead (SEC-IAM-050) |
| 10.5 OIDC Client | 4/5 | SEC-CLI-026, SEC-CLI-040, SEC-IAM-026, SEC-IAM-027, SEC-IAM-028, SEC-TM-022 | **10.5.5** (L2): Gap: SEC-STD-025 |
| 10.6 OpenID Provider | 0/2 | none | **10.6.1** (L2): N/A, Gunmetal is not an OpenID Provider: SEC-STD-026; **10.6.2** (L2): N/A: SEC-STD-026 |
| 10.7 Consent Management | 0/3 | none | **10.7.1** (L2): Covered by analogy: the device-approval screen is the consent step (SEC-CLI-038, SEC-TM-021); SEC-STD-026 for third parties; **10.7.2** (L2): Covered by analogy (SEC-CLI-038); **10.7.3** (L2): Covered by analogy: list and revoke devices and keys (SEC-IAM-042) |
| 11.1 Cryptographic Inventory and Documentation | 3/4 | SEC-IAM-100, SEC-NET-057, SEC-NET-062, SEC-NET-065, SEC-OPS-011, SEC-OPS-015 and 2 more | **11.1.3** (L3): Gap: SEC-STD-018 |
| 11.2 Secure Cryptography Implementation | 3/5 | SEC-API-006, SEC-API-026, SEC-API-030, SEC-IAM-008, SEC-NET-057, SEC-NET-065 and 1 more | **11.2.1** (L2): Gap: SEC-STD-019; **11.2.5** (L3): Gap: SEC-STD-021 |
| 11.3 Encryption Algorithms | 2/5 | SEC-EXT-050, SEC-IAM-105, SEC-OPS-017, SEC-OPS-042, SEC-PRV-037, SEC-PRV-039 and 1 more | **11.3.1** (L1): Gap: SEC-STD-019; **11.3.4** (L3): Gap: SEC-STD-020; **11.3.5** (L3): Gap: SEC-STD-019 |
| 11.4 Hashing and Hash-based Functions | 2/4 | SEC-EXT-008, SEC-EXT-046, SEC-EXT-058, SEC-EXT-069, SEC-HIS-044, SEC-HIS-045 and 4 more | **11.4.3** (L2): Gap: SEC-STD-019; **11.4.4** (L2): Gap: SEC-STD-024 |
| 11.5 Random Values | 1/2 | SEC-API-006, SEC-API-023, SEC-API-030, SEC-API-096, SEC-EXT-008, SEC-EXT-046 and 21 more | **11.5.2** (L3): Gap: SEC-STD-022 |
| 11.6 Public Key Cryptography | 2/2 | SEC-NET-065, SEC-TM-059 | All cited |
| 11.7 In-Use Data Cryptography | 0/2 | none | **11.7.1** (L3): Deviation: Gunmetal runs on the owner's hardware and cannot require memory encryption; **11.7.2** (L3): Partly covered (SEC-TM-049 zeroes secrets); gap: SEC-STD-023 |
| 12.1 General TLS Security Guidance | 3/5 | SEC-CLI-008, SEC-CLI-042, SEC-IAM-011, SEC-NET-002, SEC-NET-033, SEC-TM-010 | **12.1.4** (L3): Deviation recorded in network-and-remote-access.md (public CAs dropped OCSP; short certificate lifetimes instead); **12.1.5** (L3): Deviation for R1 and R2 recorded in network-and-remote-access.md; revisit Later |
| 12.2 HTTPS Communication with External Facing Services | 2/2 | SEC-API-037, SEC-CLI-008, SEC-EXT-004, SEC-EXT-066, SEC-HIS-026, SEC-IAM-011 and 10 more | All cited |
| 12.3 General Service to Service Communication Security | 4/5 | SEC-API-078, SEC-CLI-042, SEC-EXT-004, SEC-EXT-066, SEC-HIS-026, SEC-IAM-032 and 12 more | **12.3.3** (L2): Gap: SEC-STD-040 |
| 13.1 Configuration Documentation | 4/4 | SEC-API-030, SEC-API-061, SEC-API-076, SEC-API-079, SEC-EXT-001, SEC-EXT-004 and 24 more | All cited |
| 13.2 Backend Communication Configuration | 6/6 | SEC-API-076, SEC-API-077, SEC-API-078, SEC-API-079, SEC-API-080, SEC-API-083 and 32 more | All cited |
| 13.3 Secret Management | 4/4 | SEC-API-030, SEC-CLI-016, SEC-CLI-031, SEC-EXT-008, SEC-EXT-013, SEC-EXT-050 and 21 more | All cited |
| 13.4 Unintended Information Leakage | 4/7 | SEC-API-002, SEC-API-005, SEC-API-008, SEC-API-072, SEC-API-091, SEC-CLI-019 and 14 more | **13.4.1** (L1): Gap: SEC-STD-017; **13.4.3** (L2): Gap: SEC-STD-017; **13.4.7** (L3): Gap: SEC-STD-017 |
| 14.1 Data Protection Documentation | 2/2 | SEC-HIS-044, SEC-HIS-061, SEC-IAM-004, SEC-NET-039, SEC-OPS-041, SEC-OPS-042 and 4 more | All cited |
| 14.2 General Data Protection | 8/8 | SEC-API-004, SEC-API-005, SEC-API-015, SEC-API-026, SEC-API-049, SEC-API-055 and 80 more | All cited |
| 14.3 Client-side Data Protection | 3/3 | SEC-API-032, SEC-API-039, SEC-API-055, SEC-CLI-004, SEC-CLI-006, SEC-CLI-009 and 9 more | All cited |
| 15.1 Secure Coding and Architecture Documentation | 5/5 | SEC-API-064, SEC-CLI-017, SEC-CLI-048, SEC-CLI-052, SEC-EXT-019, SEC-EXT-020 and 19 more | All cited |
| 15.2 Security Architecture and Dependencies | 5/5 | SEC-API-031, SEC-API-043, SEC-API-060, SEC-API-061, SEC-API-062, SEC-API-064 and 80 more | All cited |
| 15.3 Defensive Coding | 6/7 | SEC-API-004, SEC-API-013, SEC-API-024, SEC-API-057, SEC-API-067, SEC-API-068 and 26 more | **15.3.6** (L2): Gap: SEC-STD-012 |
| 15.4 Safe Concurrency | 2/4 | SEC-API-018, SEC-IAM-009, SEC-MED-036, SEC-MED-041, SEC-TM-043 | **15.4.1** (L3): Gap: SEC-STD-029; **15.4.3** (L3): Gap: SEC-STD-029 |
| 16.1 Security Logging Documentation | 1/1 | SEC-OPS-020, SEC-OPS-026, SEC-PRV-001, SEC-PRV-005, SEC-PRV-044, SEC-TM-056 | All cited |
| 16.2 General Logging | 5/5 | SEC-API-094, SEC-API-095, SEC-CLI-060, SEC-EXT-016, SEC-EXT-063, SEC-HIS-062 and 24 more | All cited |
| 16.3 Security Events | 4/4 | SEC-API-095, SEC-EXT-017, SEC-EXT-044, SEC-HIS-055, SEC-HIS-062, SEC-IAM-077 and 12 more | All cited |
| 16.4 Log Protection | 3/3 | SEC-API-095, SEC-HIS-062, SEC-IAM-094, SEC-IAM-096, SEC-IAM-097, SEC-MED-062 and 9 more | All cited |
| 16.5 Error Handling | 4/4 | SEC-API-072, SEC-API-073, SEC-API-078, SEC-CLI-051, SEC-EXT-016, SEC-EXT-024 and 25 more | All cited |
| 17.1 to 17.3 WebRTC (TURN, media, signalling) | 0/12 | none | N/A: Gunmetal uses no WebRTC; iroh in browsers uses relays over WebSocket. SEC-STD-010 keeps it that way |

## OWASP Top 10:2025

Every category is covered by many requirements. The counts are the number
of Gunmetal requirements citing the category; the IDs are a sample.

| Category | Cited by | Example requirements | Assessment and remaining gaps |
|---|---|---|---|
| A01 Broken Access Control | 62 | SEC-API-001, SEC-API-010, SEC-TM-008, SEC-IAM-002, SEC-HIS-005, SEC-API-076 | Strong: deny by default, one policy layer, a generated cross-user matrix, and SSRF (now part of A01) through one egress client. Gap: context signals must only ever tighten decisions (SEC-STD-028). |
| A02 Security Misconfiguration | 37 | SEC-API-005, SEC-API-008, SEC-TM-006, SEC-OPS-038, SEC-OPS-049, SEC-SUP-046 | Strong secure defaults that survive upgrades. Gaps: static asset serving (SEC-STD-017), HSTS preload for project domains (SEC-STD-016), binary hardening (SEC-STD-033). |
| A03 Software Supply Chain Failures | 35 | SEC-SUP-002, SEC-SUP-010, SEC-SUP-021, SEC-SUP-024, SEC-SUP-041, SEC-CLI-052 | The strongest area. Gap: release evidence that the gate actually ran (SEC-STD-005). |
| A04 Cryptographic Failures | 17 | SEC-NET-002, SEC-TM-052, SEC-HIS-043, SEC-HIS-045, SEC-IAM-011, SEC-API-030 | Purposes are well specified; discipline is not. Gaps: SEC-STD-018 to SEC-STD-024. |
| A05 Injection | 18 | SEC-API-044, SEC-API-066, SEC-TM-031, SEC-TM-039, SEC-HIS-020, SEC-CLI-001 | Strong type-driven design. Gaps: ReDoS (SEC-STD-011), prototype pollution (SEC-STD-012), JSON and script inclusion (SEC-STD-013), email headers (SEC-STD-032). |
| A06 Insecure Design | 19 | SEC-TM-001, SEC-IAM-001, SEC-API-085, SEC-EXT-039, SEC-EXT-076, SEC-HIS-021 | Threat modelling is thorough. Gaps: the documents contradict each other (SEC-STD-006, SEC-STD-007), no independent design review (SEC-STD-034), no business-limits register (SEC-STD-030). |
| A07 Authentication Failures | 33 | SEC-IAM-025, SEC-TM-013, SEC-API-003, SEC-CLI-006, SEC-HIS-001, SEC-EXT-072 | Blocked by the password conflict (SEC-STD-006). Gaps: user-chosen secrets that remain (SEC-STD-008), approval prompts an attacker can trigger (SEC-STD-027). |
| A08 Software or Data Integrity Failures | 33 | SEC-SUP-041, SEC-SUP-049, SEC-CLI-046, SEC-EXT-035, SEC-OPS-019, SEC-TM-066 | Strong for code and updates. Gap: untrusted SQLite files opened by the server (SEC-STD-031). |
| A09 Security Logging and Alerting Failures | 15 | SEC-API-095, SEC-IAM-093, SEC-HIS-062, SEC-OPS-032, SEC-NET-027, SEC-EXT-017 | Covered, including owner alerts and a verifiable audit log. No gap found. |
| A10 Mishandling of Exceptional Conditions | 17 | SEC-MED-001, SEC-MED-007, SEC-API-072, SEC-API-073, SEC-IAM-069, SEC-EXT-024 | Strong for parsers (typed errors, no panics, budgets). Gaps: crypto failures (SEC-STD-021), randomness failures (SEC-STD-022), races (SEC-STD-029). |

## OWASP API Security Top 10 2023

| Risk | Cited by | Example requirements | Assessment and remaining gaps |
|---|---|---|---|
| API1 Broken Object Level Authorization | 34 | SEC-API-010 to SEC-API-014, SEC-API-023, SEC-IAM-064 | Covered: random IDs, one visibility predicate, generated cross-user matrix including adapters and share links. |
| API2 Broken Authentication | 19 | SEC-API-002 to SEC-API-006, SEC-API-056, SEC-IAM-083 | Covered once SEC-STD-006 is decided. |
| API3 Broken Object Property Level Authorization | 15 | SEC-API-015, SEC-API-067, SEC-API-068, SEC-CLI-020, SEC-EXT-030 | Covered: response shaping per caller and no mass assignment. |
| API4 Unrestricted Resource Consumption | 34 | SEC-API-031, SEC-API-060 to SEC-API-064, SEC-EXT-023, SEC-MED-021 | Covered. User-supplied regex patterns are the one unbounded path (SEC-STD-011). |
| API5 Broken Function Level Authorization | 23 | SEC-API-001, SEC-API-019 to SEC-API-022, SEC-CLI-015 | Covered: every route declares a policy or the build fails. |
| API6 Unrestricted Access to Sensitive Business Flows | 8 | SEC-API-056, SEC-API-064, SEC-API-096, SEC-API-097, SEC-IAM-101, SEC-PRV-048 | Covered for invites, exports, pairing and share links. SEC-STD-030 makes the limits one register. |
| API7 Server Side Request Forgery | 23 | SEC-API-076 to SEC-API-084, SEC-EXT-001, SEC-EXT-002 | Covered: one egress client with resolved-address checks and no cross-host redirects. |
| API8 Security Misconfiguration | 7 | SEC-API-005, SEC-API-008, SEC-API-040, SEC-API-053, SEC-NET-024, SEC-OPS-037 | Covered; SEC-STD-017 and SEC-STD-033 add asset serving and binary hardening. |
| API9 Improper Inventory Management | 15 | SEC-API-001, SEC-API-091 to SEC-API-093, SEC-EXT-055 | Covered: the OpenAPI document is generated from the route table and diffed in CI. |
| API10 Unsafe Consumption of APIs | 15 | SEC-API-078, SEC-API-079, SEC-API-081, SEC-API-084, SEC-EXT-027, SEC-EXT-028 | Covered: provider responses are untrusted, size-limited and parsed into typed values. |

## OWASP MASVS 2.1.0 for the mobile and TV apps

**Scope.** MASVS applies to the Android and iOS phone and tablet apps, and
to the Android TV and tvOS apps, which run on the same platforms. The
Samsung (Tizen) and LG (webOS) builds are web applications in a TV
browser: they are assessed against ASVS chapter 3, and against
MASVS-STORAGE and MASVS-NETWORK by analogy, because those platforms have no
equivalent mobile test guide. Desktop builds (Later) are out of MASVS scope.

**Profile target.** MAS-L2 (Advanced Security) and MAS-P (Baseline
Privacy) for every in-scope app. The apps hold a device key that unlocks a
household's history and, for administrators, host-equivalent actions, so
MAS-L1 is too low. MAS-R (resilience against reverse engineering) is
excluded: the apps are open source, hold no secret worth hiding, and root
or jailbreak detection would lock out legitimate users of old TV boxes. The
client file already recommends this (its open decision 14); SEC-STD-039
makes it an architecture record.

| Control | Cited by | Assessment |
|---|---|---|
| MASVS-STORAGE-1 Store sensitive data securely | SEC-CLI-030, SEC-CLI-033, SEC-CLI-035, SEC-IAM-048, SEC-TM-059, SEC-PRV-057 and 6 more | Covered |
| MASVS-STORAGE-2 Prevent leakage of sensitive data | SEC-CLI-028, SEC-CLI-056, SEC-CLI-060, SEC-PRV-056, SEC-TM-061 and 4 more | Covered |
| MASVS-CRYPTO-1 Current strong cryptography | SEC-CLI-072 | Thin: one requirement. SEC-STD-019 applies the server's algorithm allow-list to native code too. |
| MASVS-CRYPTO-2 Key management | SEC-CLI-030 to SEC-CLI-032, SEC-IAM-048, SEC-TM-059 | Covered |
| MASVS-AUTH-1 Secure authentication and authorization protocols | SEC-CLI-032, SEC-CLI-036, SEC-IAM-048, SEC-TM-059, SEC-TM-060 | Covered |
| MASVS-AUTH-2 Local authentication | SEC-CLI-059 | Covered: OS user verification is bound to a keystore operation, not a boolean callback. |
| MASVS-AUTH-3 Additional authentication for sensitive operations | SEC-CLI-038, SEC-CLI-059, SEC-IAM-049, SEC-EXT-071 | Covered |
| MASVS-NETWORK-1 Secure network traffic | SEC-CLI-042, SEC-NET-009, SEC-NET-033, SEC-NET-061, SEC-TM-063 | Covered |
| MASVS-NETWORK-2 Identity pinning | SEC-CLI-043, SEC-IAM-051, SEC-IAM-057, SEC-NET-060 | Covered: native clients pin the server's iroh key. |
| MASVS-PLATFORM-1 Secure IPC | SEC-CLI-025, SEC-CLI-039, SEC-CLI-054, SEC-CLI-055 | Covered |
| MASVS-PLATFORM-2 Secure WebViews | SEC-CLI-041, SEC-TM-036 | Covered |
| MASVS-PLATFORM-3 Secure user interface | SEC-CLI-056 to SEC-CLI-058 | Covered |
| MASVS-CODE-1 Up-to-date platform | SEC-CLI-066 | Covered, with a tested fallback rule for old TV boxes. |
| MASVS-CODE-2 Enforced app updates | SEC-CLI-011, SEC-CLI-065, SEC-SUP-057, SEC-SUP-058 | Covered |
| MASVS-CODE-3 No vulnerable components | SEC-CLI-017, SEC-CLI-052, SEC-MED-071, SEC-TM-062 | Covered |
| MASVS-CODE-4 Validate untrusted input | SEC-CLI-001, SEC-CLI-050, SEC-MED-076, SEC-MED-077, SEC-TM-036 | Covered; SEC-STD-012 adds prototype pollution in the shared TypeScript UI. |
| MASVS-RESILIENCE-1 Platform integrity | none | N/A by design (MAS-R excluded, SEC-STD-039). Key attestation still caps software keystores below administrator (SEC-IAM-049). |
| MASVS-RESILIENCE-2 Anti-tampering | SEC-SUP-057, SEC-SUP-059 | Partly: platform code signing only. Otherwise N/A by design. |
| MASVS-RESILIENCE-3 Anti-static analysis | none | N/A by design: open source. |
| MASVS-RESILIENCE-4 Anti-dynamic analysis | none | N/A by design. |
| MASVS-PRIVACY-1 Minimize access | SEC-CLI-020, SEC-CLI-027, SEC-CLI-061 to SEC-CLI-064, SEC-PRV-009 and 5 more | Covered |
| MASVS-PRIVACY-2 Prevent identification | SEC-PRV-010, SEC-PRV-046, SEC-PRV-059 | Covered |
| MASVS-PRIVACY-3 Transparency | SEC-PRV-010, SEC-PRV-013, SEC-PRV-027, SEC-PRV-053, SEC-TM-053, SEC-TM-054 | Covered |
| MASVS-PRIVACY-4 User control | SEC-CLI-061, SEC-PRV-023, SEC-PRV-024, SEC-PRV-033, SEC-PRV-047 and 4 more | Covered |

What is missing is not a control but a verification plan: no requirement
says how the apps will be tested against MASTG before release. SEC-STD-039
adds it.

## CWE Top 25 (2025)

| Rank | Weakness | Cited by | Assessment |
|---|---|---|---|
| 1 | CWE-79 Cross-site scripting | 34, e.g. SEC-API-044 to SEC-API-047, SEC-API-051, SEC-CLI-001 | Covered: CSP, Trusted Types, text-only rendering of tags. |
| 2 | CWE-89 SQL injection | 5: SEC-API-063, SEC-API-066, SEC-HIS-038, SEC-TM-031, SEC-TM-039 | Covered: fixed statements, enumerated sorts, FTS5 terms quoted. |
| 3 | CWE-352 Cross-site request forgery | 13, e.g. SEC-API-029, SEC-API-033 to SEC-API-036, SEC-CLI-007 | Covered |
| 4 | CWE-862 Missing authorization | 11, e.g. SEC-API-001, SEC-API-019, SEC-API-093 | Covered |
| 5 | CWE-787 Out-of-bounds write | 4: SEC-CLI-051, SEC-HIS-036, SEC-HIS-039, SEC-TM-034 | Covered by design: safe Rust (`unsafe_code = forbid`), native decoders jailed or sanitizer-tested. |
| 6 | CWE-22 Path traversal | 24, e.g. SEC-API-018, SEC-API-022, SEC-MED-033 | Covered: directory handles with `RESOLVE_BENEATH`. |
| 7 | CWE-416 Use after free | 2: SEC-CLI-051, SEC-TM-034 | Covered by design, as rank 5. |
| 8 | CWE-125 Out-of-bounds read | 1: SEC-MED-008 | Covered by design (safe Rust, SEC-MED-002 denies indexing); add the CWE to SEC-TM-034. |
| 9 | CWE-78 OS command injection | 7, e.g. SEC-HIS-020, SEC-MED-063, SEC-TM-046, SEC-TM-047 | Covered: no shell, one typed command builder. |
| 10 | CWE-94 Code injection | 5, e.g. SEC-HIS-021, SEC-HIS-040, SEC-TM-065 | Covered |
| 11 | CWE-120 Classic buffer overflow | none | Covered by design (safe Rust; native code under SEC-TM-034 and SEC-CLI-051) but uncited. |
| 12 | CWE-434 Dangerous file upload | 10, e.g. SEC-API-085, SEC-API-086, SEC-MED-011 | Covered |
| 13 | CWE-476 NULL pointer dereference | none | Covered by design, as rank 11; uncited. |
| 14 | CWE-121 Stack-based buffer overflow | none | Covered by design, as rank 11; uncited. SEC-STD-033 adds stack protection for C components. |
| 15 | CWE-502 Deserialization of untrusted data | 3: SEC-CLI-021, SEC-HIS-035, SEC-MED-023 | Covered; SEC-STD-031 adds untrusted SQLite files. |
| 16 | CWE-122 Heap-based buffer overflow | none | Covered by design, as rank 11; uncited. |
| 17 | CWE-863 Incorrect authorization | 13, e.g. SEC-API-010, SEC-API-014, SEC-EXT-007 | Covered |
| 18 | CWE-20 Improper input validation | 17, e.g. SEC-API-067, SEC-API-081, SEC-CLI-021 | Covered |
| 19 | CWE-284 Improper access control | 3: SEC-CLI-071, SEC-NET-045, SEC-OPS-027 | Covered through its children (CWE-862, 863, 639). |
| 20 | CWE-200 Sensitive information exposure | 25, e.g. SEC-API-005, SEC-API-015, SEC-CLI-013 | Covered |
| 21 | CWE-306 Missing authentication for critical function | 24, e.g. SEC-API-001 to SEC-API-003, SEC-CLI-014 | Covered |
| 22 | CWE-918 Server-side request forgery | 37, e.g. SEC-API-076 to SEC-API-084 | Covered |
| 23 | CWE-77 Command injection | none | Covered through the CWE-78 requirements (SEC-HIS-020, SEC-MED-063, SEC-TM-046); uncited. |
| 24 | CWE-639 Authorization bypass through user-controlled key | 25, e.g. SEC-API-010 to SEC-API-013, SEC-API-024 | Covered |
| 25 | CWE-770 Allocation without limits | 25, e.g. SEC-API-031, SEC-API-043, SEC-EXT-023 | Covered; SEC-STD-011 closes the regex path. |

The five uncited weaknesses are covered in substance. They should still be
added to the Standards column of SEC-TM-034, SEC-CLI-051 and SEC-MED-063,
so that the coverage check in SEC-STD-002 can see them.

## NIST SP 800-218 SSDF 1.1

| Task | Status | Gunmetal requirements |
|---|---|---|
| PO.1.1 Infrastructure security requirements | Covered | SEC-SUP-001, SEC-SUP-002, SEC-SUP-012, SEC-SUP-055 |
| PO.1.2 Software security requirements | Covered | This directory; SEC-TM-001, SEC-TM-002; kept honest by SEC-STD-001, SEC-STD-002 |
| PO.1.3 Communicate requirements to third parties | Partial | Plugin rules (SEC-EXT-*), dependency policy (SEC-SUP-021, SEC-SUP-024, SEC-SUP-035); gap: SEC-STD-037 |
| PO.2.1 Roles and responsibilities | Gap | SEC-STD-036 |
| PO.2.2 Role-based training | Gap (adapted for volunteers) | SEC-STD-037 |
| PO.2.3 Leadership commitment | Partial | README engineering standards; SEC-STD-036 |
| PO.3.1 Specify toolchains | Covered | SEC-SUP-011, SEC-SUP-018, SEC-SUP-021, SEC-SUP-038 |
| PO.3.2 Secure the toolchain | Covered | SEC-SUP-010, SEC-SUP-011, SEC-SUP-038, SEC-HIS-058 |
| PO.3.3 Tools generate evidence | Gap | SEC-STD-005 |
| PO.4.1 Criteria for security checks | Partial | The gate in CONTRIBUTING.md, SEC-SUP-019, SEC-SUP-023; SEC-STD-002, SEC-STD-004 |
| PO.4.2 Gather and safeguard check results | Gap | SEC-STD-005 |
| PO.5.1 Separate and protect environments | Covered | SEC-SUP-012, SEC-SUP-013, SEC-SUP-015 to SEC-SUP-017, SEC-HIS-058 |
| PO.5.2 Harden development endpoints | Covered | SEC-SUP-001, SEC-SUP-055 |
| PS.1.1 Protect code from tampering | Covered | SEC-SUP-002 to SEC-SUP-006 |
| PS.2.1 Integrity verification for users | Covered | SEC-SUP-041, SEC-SUP-042, SEC-SUP-043, SEC-SUP-049 |
| PS.3.1 Archive releases | Covered | SEC-SUP-040, SEC-OPS-052 |
| PS.3.2 Provenance data | Covered | SEC-SUP-041, SEC-SUP-044 |
| PW.1.1 Risk modelling | Covered | SEC-TM-001, SEC-TM-002 and the threat model |
| PW.1.2 Track requirements and risks | Gap | SEC-STD-004 |
| PW.1.3 Standardised security features | Covered | One policy layer (SEC-API-010), one egress client (SEC-EXT-001), one command builder (SEC-HIS-020), untrusted types (SEC-TM-031) |
| PW.2.1 Independent design review | Gap | SEC-STD-034 |
| PW.4.1 Acquire well-secured components | Covered | SEC-SUP-020, SEC-SUP-021, SEC-SUP-024 to SEC-SUP-027 |
| PW.4.2 Create well-secured components | Covered | SEC-SUP-025, CONTRIBUTING.md |
| PW.4.4 Verify acquired components | Covered | SEC-SUP-021, SEC-SUP-024, SEC-SUP-036, SEC-SUP-060, SEC-HIS-057 |
| PW.5.1 Secure coding practices | Covered | SEC-MED-002, SEC-MED-004, SEC-TM-031, SEC-TM-039, SEC-HIS-020 |
| PW.6.1 Secure compilers and build tools | Covered | SEC-SUP-038, SEC-SUP-040 |
| PW.6.2 Compiler and linker hardening features | Partial | Lints (SEC-MED-002), `overflow-checks` (media file); gap: SEC-STD-033 |
| PW.7.1 Decide when to review code | Partial | SEC-SUP-002, SEC-SUP-005 (CI and config paths only); gap: SEC-STD-035 |
| PW.7.2 Perform code review and analysis | Partial | SEC-SUP-018 (CodeQL, zizmor); gap: SEC-STD-035 |
| PW.8.1 Decide when to test executables | Covered | The gate in CONTRIBUTING.md |
| PW.8.2 Design and perform the testing | Partial | Fuzzing (SEC-MED-027 to SEC-MED-032), sanitizers (SEC-CLI-051), SEC-HIS-036, SEC-HIS-066; gap: SEC-STD-038 |
| PW.9.1 Secure default settings | Covered | SEC-NET-022, SEC-OPS-038, SEC-OPS-049, SEC-HIS-008 |
| PW.9.2 Implement the defaults | Covered | SEC-HIS-055, SEC-OPS-049, SEC-OPS-052, SEC-OPS-061 |
| RV.1.1 Monitor for vulnerabilities | Covered | SEC-OPS-064, SEC-SUP-021, SEC-SUP-028, SEC-SUP-048 |
| RV.1.2 Review code for unknown vulnerabilities | Covered | SEC-SUP-018, continuous fuzzing (SEC-MED-029); SEC-STD-034 adds outside eyes |
| RV.1.3 Disclosure policy | Covered | SEC-SUP-007, SEC-SUP-008, SEC-OPS-064, SEC-OPS-065 |
| RV.2.1 Analyse each vulnerability | Covered | SEC-OPS-065 |
| RV.2.2 Plan the response | Covered | SEC-SUP-009, SEC-SUP-023, SEC-OPS-065 to SEC-OPS-070 |
| RV.3.1 to RV.3.4 Root causes and process change | Covered | SEC-TM-003, SEC-HIS-064, SEC-OPS-066 |

**SSDF 1.2 (draft) additions.** PO.6 (continuous improvement: update
tools, adopt features that remove whole classes of bugs, revisit decisions)
is partly met by SEC-HIS-064 and SEC-OPS-066; SEC-STD-003 adds a yearly
review. PS.4 (robust updates: test in realistic configurations, staged
rollout, automatic rollback, a fault-tolerant update engine) is met by
SEC-SUP-050, SEC-SUP-064, SEC-OPS-048, SEC-OPS-051 and SEC-OPS-052. When
1.2 is final, SEC-STD-003 re-baselines this table.

## Requirements

These close the gaps above. They follow the conventions of the other
files: "R1" is the first usable release (music), "R1.1", "R1.2" and
"R1.3" its point releases, "R2" the release with native apps, remote
access and the project name service, "R3" live TV, "Later" anything after, and
"Withdrawn" a row kept only for its ID. Where a requirement binds a later
surface as well, the Release column holds the first release and the text
says the rest.

| ID | Requirement | Standards | Release | Verified by |
|---|---|---|---|---|
| SEC-STD-001 | The repository must hold machine-readable copies of the pinned standard versions (the ASVS 5.0.0 CSV, the MASVS 2.1.0 control list, the CWE Top 25 2025 list, the Top 10:2025 and API Top 10 2023 IDs, and the SSDF 1.1 task list), and a docs lint must fail when any requirement in `docs/security/` cites an ID that does not exist in the pinned version or names a superseded edition. | SSDF PO.1.2, PO.4.1 | R1 | CI check (xtask docs lint) with a fixture document citing a non-existent ASVS number, which must fail |
| SEC-STD-002 | CI must regenerate the coverage tables in this file from the Standards columns and fail when an ASVS requirement at or below its chapter's target level has neither a citing requirement nor an entry in a deviation or not-applicable register that gives a reason, a compensating control, an owner and a review date; an expired review date must also fail. | ASVS (all chapters, as the levels table here sets them); SSDF PO.4.1 | R1 | CI check; unit tests of the generator against fixture tables |
| SEC-STD-003 | A scheduled job must check monthly for new versions of ASVS, the Top 10, the API Top 10, MASVS and the MAS profiles, the CWE Top 25 and SSDF (including SP 800-218 Rev. 1 becoming final), open an issue when one appears, and this file must be re-baselined within 90 days of a final release; once a year the maintainers must also review lints, fuzz targets and tools for new bug classes. | SSDF RV.1.1; SSDF 1.2 draft PO.6.1, PO.6.2 | R1 | Scheduled CI job with a test against recorded release feeds; the yearly review is a dated record in the repository (manual review) |
| SEC-STD-004 | Every SEC requirement whose release is at or before the release being built must be referenced by at least one test (a test name, attribute or tag carrying its ID) or by a named manual-review record; the release workflow must fail otherwise and must publish the resulting traceability report. | SSDF PW.1.2, PO.4.1 | R1 | CI check (xtask) that scans tests for IDs and compares them with the requirement tables; a fixture requirement with no test must fail the check |
| SEC-STD-005 | Each release must publish, with the same provenance as its binaries, a security evidence bundle: the gate's coverage and mutation reports (zero survivors), fuzzing statistics per target, cargo-deny and cargo-vet output, CodeQL and zizmor results, the Scorecard result, the SEC-STD-004 traceability report and the SEC-STD-002 coverage report. | SSDF PO.3.3, PO.4.2, PS.3.2 | R1 | Release workflow check that each part is present and attested; post-release job verifies the attestation |
| SEC-STD-006 | Before any server code stores a user, one architecture record must decide whether account passwords and TOTP exist in R1 (recommended and assumed throughout this baseline: they do not, SEC-IAM-025), must fix one session-lifetime table (the parameters table in threat-model.md: native access tokens of 10 minutes under SEC-IAM-050), and every file in `docs/security/` and `docs/features/` must agree with it; requirements it withdraws must be marked withdrawn and no live requirement may cite them. | ASVS 6.1.1, 6.1.3, 7.1.1; SSDF PW.1.2; A06:2025 | R1 | Manual review of the record; CI docs lint that fails when a withdrawn ID is cited, or when SEC-IAM-025 and SEC-TM-013 (or ACC-052 and ACC-053) are both live |
| SEC-STD-007 | **Withdrawn 2026-10-02: merged into SEC-NET-001.** The stricter wording (no PINs, share passwords or pairing codes over cleartext, whatever the settings) now lives in the owner. | ASVS 12.2.1, 12.3.1, 3.3.1; A04:2025; CWE-319, CWE-523 | Withdrawn | Proved by the tests of SEC-NET-001 |
| SEC-STD-008 | Any secret a person chooses (an account password if SEC-STD-006 keeps one, a share-link password, a backup passphrase) must accept any Unicode characters with no composition rules, allow at least 64 characters, never expire on a schedule, be changeable only with the current secret or a fresh user verification, and be rejected if it is on the breached list or contains an entry from a documented context list (the product name, the server name and label, the household name, user and profile names). | ASVS 6.1.2, 6.2.1 to 6.2.5, 6.2.9 to 6.2.12; SP 800-63B-4 §3.1.1.2; CWE-521 | R1.2 | Unit and property tests of the policy (any composition, 64 and more characters, context words); integration tests of change and of the breached list |
| SEC-STD-009 | **Withdrawn 2026-10-02.** TOTP is not offered (SEC-IAM-025, SEC-STD-006). If TOTP is ever proposed, this text returns with the identity architecture record. | ASVS 6.5.1, 6.5.5, 6.5.8; RFC 6238; CWE-294 | Withdrawn | Not applicable while withdrawn |
| SEC-STD-010 | Gunmetal must not include SAML, LDAP or other directory authentication, SMS or voice codes, email codes or links, GraphQL, JSONP, WebRTC or STUN/TURN, memcache, or XPath or XSLT built from runtime text; each must be banned in cargo-deny and the JavaScript dependency allow-list, absent from the route table and the client bundle, and adopting one requires an architecture record that brings the matching ASVS section to its chapter's target level. | ASVS 1.2.6 to 1.2.8, 1.3.9, 4.3, 6.6.1, 6.8, 3.5.6, V17; A06:2025 | R1 | CI check: cargo-deny bans, a bundle scan for `RTCPeerConnection` and `callback=` handlers, and a route-table test for GraphQL and SAML paths |
| SEC-STD-011 | Regular expressions applied to untrusted input, and patterns that users write (smart playlists, live-TV import filters, search), must run only in a linear-time engine (the Rust `regex` crate) on the server, with a pattern length cap, a compiled-size limit and a per-request time budget; literal text placed in a pattern must be escaped; backtracking engines (`fancy-regex`, PCRE2, Oniguruma) must be banned for these uses; the TypeScript client must not build a `RegExp` from runtime text and must pass `regexp/no-super-linear-backtracking` and `regexp/no-super-linear-move`. | ASVS 1.2.9, 1.3.12; A05:2025; CWE-1333, CWE-400 | R1 | Property test with known catastrophic patterns and 1 MiB inputs asserting completion within the budget; CI checks (cargo-deny bans, ESLint rules as errors) |
| SEC-STD-012 | The client must keep data keyed by untrusted strings (tag and Vorbis comment names, metadata keys from providers and plugins, user preferences) in `Map` or null-prototype objects, must never merge untrusted objects into existing ones with spread, `Object.assign` or deep-merge helpers, and must freeze `Object.prototype` in production builds once compatibility is proven. | ASVS 15.3.6; A08:2025; CWE-1321 | R1 | Property test feeding API payloads whose keys include `__proto__`, `constructor` and `prototype` through the client model and asserting `Object.prototype` is unchanged; ESLint rule against the banned merge patterns |
| SEC-STD-013 | API responses must be JSON with `Content-Type: application/json` and `nosniff`, no route may honour a callback parameter, no JavaScript response may contain per-user or authorised data, and bootstrap data must be fetched rather than inlined into HTML; if anything is ever inlined, `<`, `>`, `&`, U+2028 and U+2029 must be escaped. | ASVS 1.2.3, 3.5.6, 3.5.7; A05:2025; CWE-79, CWE-116 | R1 | Integration test over every route asserting type and headers; a test that a `callback` parameter changes nothing; a build check that emitted scripts are byte-identical for two different users |
| SEC-STD-014 | Every cookie the server sets must have a name and value of at most 4096 bytes combined, and the server must set no more than the cookies listed in its cookie inventory. | ASVS 3.3.5 | R1 | Integration test over all `Set-Cookie` headers against the inventory |
| SEC-STD-015 | Links to any origin outside the server (artist websites and other URLs from tags, provider pages, external help) must accept only `https` and `http`, must open only after a sheet showing the destination host with a cancel option, and must use `noopener` and `noreferrer`; on TVs they must be shown as a QR code instead of navigating. | ASVS 3.7.3, 3.7.2; CWE-601, CWE-1022 | R1 | Unit tests of the link classifier (schemes, hosts, look-alike Unicode); end-to-end test that clicking a tag URL shows the sheet |
| SEC-STD-016 | Every domain the project operates (gunmetal.tv and, if the per-server name service of SEC-NET-010 is built, its zone) must send `Strict-Transport-Security` with `max-age` of at least 63072000, `includeSubDomains` and `preload`, and must be on the browser HSTS preload list before it is announced to users. The name-service zone is covered from the day it is announced. | ASVS 3.7.4, 3.4.1; A02:2025; CWE-319 | R1 | Scheduled CI check of the headers and of the domains' presence in Chromium's preload list file; manual review of the submission |
| SEC-STD-017 | The server must serve web assets only from a build-time manifest of exact paths and allowed extensions embedded in the binary; it must never list a directory, never serve version-control metadata, environment or configuration files, and must answer anything not in the manifest with the same 404. | ASVS 13.4.1, 13.4.3, 13.4.7; A02:2025; CWE-548, CWE-527, CWE-538 | R1 | CI check that inspects the embedded manifest for disallowed names and extensions; integration test requesting `/.git/HEAD`, `/.env`, `/assets/` and a source map |
| SEC-STD-018 | The project must keep a cryptographic inventory (algorithm, key, purpose, library, location, rotation, post-quantum status) as CycloneDX cryptographic assets in the SBOM, with a migration plan for the classical signatures it relies on (passkeys, device keys, TUF and Sigstore); cryptographic crates may be used only from one `crypto` module, so discovery is mechanical, and CI must fail if the inventory and the code disagree. | ASVS 11.1.1 to 11.1.4, 11.2.2; A04:2025 | R1 | CI check (clippy `disallowed-methods` and `disallowed-types` outside the module, and an inventory diff); SBOM validation |
| SEC-STD-019 | Cryptography must come only from implementations on a reviewed allow-list chosen in an architecture record (for example rustls with the aws-lc-rs or ring provider and audited RustCrypto crates); encryption must use AEAD only (no ECB, no unauthenticated CBC or CTR, no MAC-then-encrypt, no RSA PKCS#1 v1.5 encryption); MD5 and SHA-1 may appear only behind a type that marks them as non-security (legacy protocol fields, fingerprints) and never for integrity or authentication; the same allow-list applies to native client code. | ASVS 11.2.1, 11.3.1, 11.3.2, 11.3.5, 11.4.1, 11.4.3; MASVS-CRYPTO-1; A04:2025; CWE-327, CWE-328, CWE-1240 | R1 | cargo-deny bans and clippy disallowed types; compile-fail tests (trybuild) showing the non-security digest type cannot feed a MAC or signature check |
| SEC-STD-020 | Every AEAD key must have a nonce strategy fixed in code: either 192-bit random nonces (XChaCha20-Poly1305), or 96-bit nonces from a counter persisted before use, or, for AES-GCM with random 96-bit nonces as SEC-EXT-050 specifies, rotation of the key well before 2^32 encryptions; nonces must be produced by one function per strategy. | ASVS 11.3.4; NIST SP 800-38D §8.3 (unverified section number); CWE-323 | R1 | Property test that a generator never repeats over a large sample; unit test that the per-key counter forces rotation at the threshold; mutation testing of the counter |
| SEC-STD-021 | Every decryption, MAC and signature check must return one opaque error that does not reveal which step failed, must release no plaintext before the tag is verified (streamed backups verified chunk by chunk), and must not log ciphertext, keys or the failing input. | ASVS 11.2.5, 16.5.1; A10:2025; CWE-209, CWE-208 | R1 | Unit tests per failure kind (bad tag, bad padding where applicable, truncation, wrong key) asserting the identical error value; a test that truncated streams emit no output |
| SEC-STD-022 | All security randomness must come from the operating system CSPRNG through one function, failure to obtain randomness must stop the operation (and the server at start-up) rather than fall back, and non-cryptographic generators (`fastrand`, seeded generators) must be banned outside tests. | ASVS 11.5.1, 11.5.2; A10:2025; CWE-338, CWE-330 | R1 | Clippy disallowed-methods; fault-injection unit test that a randomness error fails closed; concurrent test issuing 100,000 tokens across threads and asserting uniqueness |
| SEC-STD-023 | The main server process must disable core dumps (`RLIMIT_CORE` 0 and `PR_SET_DUMPABLE` 0, or the platform equivalent) at start, must keep decrypted third-party secrets in memory only for the call that uses them, and must hold secrets in zeroising types (extending SEC-TM-049 to every secret the server decrypts). | ASVS 11.7.2, 14.2.6; CWE-528, CWE-226 | R1 | Integration test reading the process's dumpable flag and core limit; unit tests that secret types zero on drop and cannot be formatted |
| SEC-STD-024 | Any key derived from a human secret (a backup passphrase, if that option is chosen, or an export password) must use Argon2id with at least the second recommended RFC 9106 parameters (64 MiB, 3 passes, 4 lanes) or age's scrypt recipient at its default work factor or higher, must store its parameters with the ciphertext so they can be raised later, and the passphrase must meet SEC-STD-008 with at least 15 characters. This binds only if a passphrase option exists. | ASVS 11.4.4, 11.4.2; RFC 9106 §4; CWE-916 | R1 | Unit tests of the parameter floor and the stored header; known-answer tests against RFC 9106 vectors |
| SEC-STD-025 | OIDC authorization requests must carry exactly the scopes in the provider's configuration (default `openid`; `profile` or `email` only when claim mapping is on); front-channel and back-channel logout must not be supported in R1, and if back-channel logout is added it must verify the logout token's signature, `iss`, `aud`, `iat`, `exp`, `events` and `sub` or `sid`, reject a token with a `nonce` or a reused `jti`, require the `logout+jwt` type, and end only the matching sessions. The scope rule is R1; back-channel logout is not built in R1. | ASVS 10.2.3, 10.5.5; OpenID Connect Back-Channel Logout 1.0 §2.6; CWE-863 | R1.2 | Integration tests against a test provider asserting the scope parameter; fixture tests for each logout-token defect if the feature exists |
| SEC-STD-026 | The server must not act as an OAuth authorization server for third-party clients or as an OpenID Provider: no discovery documents (`openid-configuration`, `oauth-authorization-server`), no authorize, token or dynamic-registration endpoints; Gunmetal's pairing and device tokens are a first-party protocol mapped to ASVS 10.4 here; delegated access for third-party apps needs an architecture record that adopts ASVS 10.4, 10.6 and 10.7 at Level 3 (pushed authorization requests, codes valid for at most 60 seconds, consent that can be reviewed and revoked). | ASVS 10.4, 10.6, 10.7; API9:2023 | R1 | Route-table CI check for those paths; manual review of any record that changes it |
| SEC-STD-027 | No flow may show an approval prompt on a person's device that someone without a session can trigger; device approvals must always start on the approving device (scanning the new device's QR code or typing its code), and if a pushed approval is ever added it must allow at most three pending prompts per account, rate-limit them, and require number matching. | ASVS 6.6.4, 6.6.3; RFC 8628 §5.4; CWE-308 | R1 | Integration test: an unauthenticated pairing request creates no notification, event or audit entry addressed to any user; integration test that 1,000 unauthenticated pairing requests produce zero notifications (shared with SEC-IAM-052) |
| SEC-STD-028 | **Withdrawn 2026-10-02: merged into SEC-IAM-013.** Context signals may only add friction; admin capabilities from a new network need fresh verification. | ASVS 8.2.4, 8.1.3, 8.1.4; A01:2025; CWE-290 | Withdrawn | Proved by the tests of SEC-IAM-013 |
| SEC-STD-029 | Every single-use or counted secret (setup code, invitation, recovery code, pairing code, share-link use limit, export download, and TOTP code if kept) must be consumed by one conditional update inside a single SQLite write transaction, so that concurrent redemptions cannot both succeed; async code must not hold a lock across an await, and custom synchronisation primitives must be model-checked. | ASVS 15.4.1, 15.4.2, 15.4.3; A10:2025; CWE-367, CWE-362, CWE-833 | R1 | Integration test racing 64 concurrent redemptions against a real SQLite file and asserting exactly one success; clippy `await_holding_lock` denied; loom tests for any custom primitive |
| SEC-STD-030 | The project must keep one machine-readable register of business limits, per user and server-wide (devices, sessions, invitations, share links, API keys, playlists and their length, queue length, uploads, exports, concurrent streams and transcodes), and of field validation rules generated from the core protocol types; the server must load limits from it, and cross-field rules (a share's expiry within the maximum, a child profile's ceiling within the household's) must be checked. | ASVS 2.1.1, 2.1.2, 2.1.3, 2.2.3; API6:2023; CWE-770, CWE-840 | R1 | CI check that every register entry has an enforcing test (through SEC-STD-004); property tests of the cross-field rules |
| SEC-STD-031 | Any SQLite file the server did not create itself (a restored backup, a rival server's database for import, a plugin-supplied database) must be opened read-only in a jailed worker with `trusted_schema` off, triggers and views disabled, `cell_size_check` on, memory mapping off and `quick_check` passed before any query, and its contents must reach the server only as typed rows. It binds the restore-at-setup path in R1 and the rival importers when they ship (Later). | SQLite security guidance (sqlite.org/security.html); ASVS 1.5.2, 5.2.2, 15.2.5; A08:2025; CWE-502, CWE-20 | R1 | Fuzz target over a corpus of corrupt and hostile database files (crafted schemas, triggers, views, oversized cells) asserting typed errors; integration test that a hostile restore at setup changes nothing |
| SEC-STD-032 | If email alerts exist (SEC-OPS-035), messages must be built with a typed builder that rejects carriage returns and line feeds in every header value, subjects must come from fixed templates, user-controlled text may appear only in a plain-text body, and recipients must be only those the owner configured. | ASVS 1.3.11; A05:2025; CWE-93 | Later | Unit tests with CRLF in user, device and server names; integration test against a local SMTP sink |
| SEC-STD-033 | Release binaries must be built position-independent with full RELRO and non-executable stacks, with `overflow-checks` on in the release profile, and every C or C++ component (bundled SQLite, the client media stack) must be compiled with stack protection and `_FORTIFY_SOURCE=3` where the toolchain supports it; CI must fail if a release binary loses a protection it had. It binds the server from R1 and the client media stack from R2. | SSDF PW.6.2; ASVS 15.2.5; CWE-121, CWE-122, CWE-787 | R1 | CI check running a binary hardening checker on every release artifact against a recorded baseline |
| SEC-STD-034 | Before the R1 tag, a reviewer who did not write them must review the threat model, the identity design and the authorization layer; before R2 (remote access, plugins, native apps), an external security assessment of the server and clients must be completed; critical and high findings must block the release, and the review records must be committed. The non-author review is due before the R1 tag and the external assessment before the R2 tag. | SSDF PW.2.1, PW.7.2, RV.1.2; A06:2025 | R1 | Release workflow check that a review record for the tag exists; manual review of the findings' closure |
| SEC-STD-035 | Source paths that implement authentication, the policy layer, cryptography, parsers, the jail, the egress client and the release process must be listed in CODEOWNERS; once two maintainers exist, a change to them needs approval from someone other than its author or the person who ran the coding agent that wrote it; until then it needs a completed security checklist and a 24-hour wait before merge; pull requests written by coding agents must be labelled and reviewed as outside contributions. | SSDF PW.7.1, PW.7.2; SLSA v1.2 Source L4 (two-party review); A06:2025 | R1 | Settings-drift job (as SEC-SUP-002) checking the ruleset and CODEOWNERS; CI check for the checklist and label |
| SEC-STD-036 | `SECURITY.md` or a governance file must name the security lead, the release managers, the incident lead and a deputy for each; the holders of the threshold TUF keys (SEC-SUP-049) must be different people; and a succession plan must say who takes over if a maintainer is unreachable for 30 days. It must be reviewed every year. | SSDF PO.2.1, PO.2.3 | R1 | CI docs lint that the roles are filled and the review date is current; manual review |
| SEC-STD-037 | CONTRIBUTING.md must link a short secure-coding guide drawn from these files (forbidden sinks, untrusted types, the test rules for security code), and the plugin SDK must publish the security requirements plugins are held to. The guide is due in R1 and the plugin SDK requirements with the SDK (R2). | SSDF PO.1.3, PO.2.2 | R1 | CI docs lint that the links resolve; manual review |
| SEC-STD-038 | CI must fuzz the running server through its generated OpenAPI description (property-based requests against a seeded server with a real SQLite file) on every pull request with a short budget and nightly with a long one, failing on any 5xx response, any response that breaks its schema, and any response that contains another user's data; a passive web scanner must also run nightly against the web client. | SSDF PW.8.2; ASVS 4.1.1, 15.3.1; API3:2023, API8:2023 | R1 | The CI jobs themselves, with a deliberately broken fixture route that must be caught |
| SEC-STD-039 | An architecture record must set MAS-L2 and MAS-P as the targets for the Android, iOS, Android TV and tvOS apps and exclude MAS-R with its reasons; each in-scope MASVS control must map to MASTG 2.x tests; CI must run static analysis of every built APK and IPA and fail on high findings; and a manual MASTG pass must precede each native release. Samsung and LG web builds are assessed against ASVS chapter 3. | MASVS 2.1.0 (all groups); MAS Testing Profiles; SSDF PW.8.2 | R2 | CI job on each app build; dated MASTG test record per release (manual review) |
| SEC-STD-040 | The server must talk to its scan worker, transcode jail, plugin host and command-line tools only over inherited socket pairs, pipes or Unix sockets that check the peer's credentials, never over TCP (loopback included). | ASVS 12.3.3, 12.3.5, 13.2.1; CWE-1327, CWE-923 | R1 | Listening-socket inventory test (as SEC-TM-006) asserting no TCP listener besides the published ones; integration test that a connection from another user's process is refused |

## Keeping these controls usable

Almost all of the controls above are invisible: build checks, typed
builders, test suites and headers. The ones people meet are designed so
that no household member, TV or child has to do anything new.

- **No password (SEC-STD-006, recommended).** Nothing to forget, reset or
  type with a TV remote. Passkeys on phones and laptops, QR pairing for TVs,
  and a hardware key or the native app for someone without a smartphone,
  as the identity file describes.
- **Cleartext gets a help page, not the app (SEC-STD-007).** The page
  explains in plain words how to open the secure address, and the
  per-server name service or a reverse-proxy recipe makes that one click.
  This is stricter than rivals, but it only takes away the one mode in
  which passkeys cannot work.
- **External links (SEC-STD-015)** show one sheet naming the site, which
  is what phones already do for unknown links; on TVs a QR code is easier
  than a TV browser anyway.
- **User-chosen secrets (SEC-STD-008)** have no composition rules and no
  expiry. The only rejection message is "this one is too easy to guess",
  with a suggestion.
- **Approvals (SEC-STD-027)** always start with the person holding the
  phone, so nobody gets surprise prompts at night.
- **Adaptive step-up (SEC-STD-028)** only touches administrators, and only
  when they change network, so family members never see it.
- **Regex patterns (SEC-STD-011)** still work for power users building
  smart playlists; only patterns that cannot finish in time are refused,
  with a message saying so.

## Open decisions for the project owner

> **Status, 2026-10-02.** The owner decisions from every file in
> `docs/security/` are consolidated and de-duplicated in
> [README.md](README.md#open-decisions-for-the-owner). Where the baseline
> has since chosen, or where a recommendation below disagrees with the
> README, the README and the requirement tables win.

1. **Passwords and TOTP (SEC-STD-006).** *Recommendation:* no account
   passwords and no TOTP, as SEC-IAM-025 says; withdraw SEC-TM-013,
   ACC-052 and ACC-053. *Trade-off:* a few old browsers and shared
   computers need a phone nearby for cross-device passkey sign-in. If
   passwords stay, SEC-STD-008 and SEC-STD-009 apply in full and chapter 6
   at Level 3 needs every password item.
2. **Cleartext on the LAN (SEC-STD-007).** *Recommendation:* follow
   SEC-NET-001. *Trade-off:* a household that never sets up HTTPS cannot
   use the browser client; the native apps (R2) and localhost still work.
3. **Target levels.** *Recommendation:* the table above (Level 3 except
   chapter 2). *Trade-off:* the extra Level 3 work is about ten
   requirements in this file, nearly all build checks and unit tests.
4. **External assessment before R2 (SEC-STD-034).** *Recommendation:* fund
   or seek a sponsored review (open-source security funds exist for this;
   which one is unverified). *Trade-off:* time and possibly money before
   remote access ships, which is when exposure jumps.
5. **MAS-R exclusion (SEC-STD-039).** *Recommendation:* exclude it.
   *Trade-off:* no defence against someone modifying their own copy of the
   app, which an open-source project cannot prevent anyway.

## Sources

Primary sources fetched on 2026-10-02:

- OWASP ASVS releases (5.0.0 is the latest stable; the "latest" tag is a
  rolling bleeding-edge build): https://github.com/OWASP/ASVS/releases
- ASVS 5.0.0 CSV used for every requirement number and level:
  https://github.com/OWASP/ASVS/releases/download/v5.0.0_release/OWASP_Application_Security_Verification_Standard_5.0.0_en.csv
- OWASP Top 10:2025: https://top10.owasp.org/2025
- OWASP API Security Top 10 2023: https://api-security.owasp.org/
- OWASP MASVS releases and controls: https://github.com/OWASP/masvs/releases,
  https://github.com/OWASP/masvs/blob/master/OWASP_MASVS.yaml,
  https://github.com/OWASP/masvs/blob/master/Document/03-Using_the_MASVS.md
- MAS Testing Profiles: https://mas.owasp.org/Profiles/
- MASTG 2.0.0 release: https://github.com/OWASP/mastg/releases/tag/v2.0.0
- CWE Top 25: https://cwe.mitre.org/top25/ and
  https://cwe.mitre.org/top25/archive/2025/2025_cwe_top25.html
- NIST SSDF publications: https://csrc.nist.gov/Projects/ssdf/publications
- NIST SP 800-218 (SSDF 1.1):
  https://nvlpubs.nist.gov/nistpubs/SpecialPublications/NIST.SP.800-218.pdf
- NIST SP 800-218 Rev. 1 initial public draft (SSDF 1.2), for PO.6 and PS.4:
  https://nvlpubs.nist.gov/nistpubs/SpecialPublications/NIST.SP.800-218r1.ipd.pdf
- SQLite guidance on untrusted database files:
  https://www.sqlite.org/security.html
- Rust `regex` crate (worst-case linear search time, `size_limit`; version
  1.13.1 at the time of writing): https://docs.rs/regex/latest/regex/
- eslint-plugin-regexp rules:
  https://ota-meshi.github.io/eslint-plugin-regexp/rules/no-super-linear-backtracking.html
- HSTS preload requirements: https://hstspreload.org/
- RFC 9106 (Argon2), section 4: https://www.rfc-editor.org/rfc/rfc9106.html
- OpenID Connect Back-Channel Logout 1.0 (final, errata set 1):
  https://openid.net/specs/openid-connect-backchannel-1_0.html

From memory, not re-fetched: the AES-GCM invocation limit for random
96-bit IVs in NIST SP 800-38D (cited above with the section number marked
unverified). Facts about OCSP and ECH come from
[network-and-remote-access.md](network-and-remote-access.md); facts about
rival incidents come from [rival-security-history.md](rival-security-history.md).
