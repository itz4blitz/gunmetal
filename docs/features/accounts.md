# Accounts, sharing and security

This map covers who can use a Gunmetal server and how safely: owning the
server from the first minute, user accounts, households and profiles,
parental controls, permissions, sign-in, devices and sessions, invitations,
sharing, remote access, bandwidth limits, privacy, and the security
behaviour a user can see. The bar is set by three facts from the research.
Plex is the easiest product for sharing and household profiles, but it
depends on plex.tv, which has been breached twice and now charges for remote
video. Jellyfin keeps accounts local and has the finest permissions, but it
has no built-in two-factor, single sign-on or profiles, and its 2026
advisories are mostly authorization mistakes. Navidrome and Immich show how
share links and modern sign-in should work, and also how easily they go
wrong. Gunmetal should match Plex on ease, match Jellyfin on control, and
beat all of them on sign-in, revocation and privacy, with nothing outside
the house able to lock a user out or read their history.

## Features

Releases (R1, R2, R3, Later, No), the Demand scale, row ownership and the
terms "the user log" and "the identity store" are defined once in the
[feature map README](README.md). A row whose Release cell would differ
between maps names one owning row; the other maps point at it. In short: R1 ships only the server and the web client, so anything
that needs a native app, a device key in secure storage or iroh waits for
R2, and in R1 remote use means the owner's own reverse proxy, VPN or
domain. Rival claims come from the files in
`docs/research/`; "not covered in the research" means no evidence was
gathered either way.

### Server ownership and first run

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| ACC-001 | Claim a new server with a one-time setup code | Nobody else on the network can take over a fresh install before the owner does | Plex yes (claim through a plex.tv account); Jellyfin partial (wizard open on the LAN, 12.0 closed a re-run hole); Emby not checked | Medium: Jellyfin 12.0 and Navidrome 0.64.2 both fixed setup-window flaws in 2026 | R1 | The server prints a single-use code to its console and to a file only the service user can read, and refuses every other route until it is claimed. No vendor account is involved (ADR 1, decision 7) Owns the setup code; ADM-018 is its setup screen. | Setup-code generation and expiry; a "claimed" state in the identity store; a guard that blocks all routes while unclaimed | First-run setup screen (code entry); console and log message |
| ACC-002 | Owner account with no vendor account | The owner holds the server outright, and no company can lock them out | Plex no (every server is claimed by plex.tv); Jellyfin yes; Emby yes (Emby Connect optional) | High: Plex request for local authentication has 372 votes, open since 2015 | R1 | Parity with Jellyfin and Emby. Ahead of Plex because ADR 1 rules out any central account. Owns first-admin creation; ADM-019 is its setup screen. When the setup page is not a secure context (a browser reaching http://<LAN IP>), the admin may set a password, with ACC-052 enforcing strength and the ACC-063 limiter, and is prompted to add a passkey once HTTPS exists. | Owner role; first credential enrolment, passkey preferred | First-run setup; Account settings |
| ACC-003 | Sign-in and playback with no internet | Music keeps playing at home during an ISP or vendor outage | Plex no (plex.tv sign-in was down about two hours on 2026-07-14); Jellyfin yes; Emby yes (unverified) | High: the same 372-vote request and the July 2026 outage thread | R1 | Parity with Jellyfin. Every sign-in and authorization check runs on the server itself, and a test runs the whole sign-in suite with outbound networking off | No outbound call in any sign-in or authorization path | None (behaviour); connection indicator in the client |
| ACC-004 | Owner recovery from the host | An owner who has lost every device and passkey can still get back in | Plex email reset at plex.tv; Jellyfin asks for a file on the server (unverified); Immich lets an admin issue a temporary password | Medium: named as a hard part in the research | R1 | A command run on the host prints a single-use, short-lived recovery link. It needs filesystem access, so it can never become a remote path Owns host recovery; ADM-034 points here. | Command-line recovery; recovery-token store with expiry; audit event | Command line; recovery sign-in screen |
| ACC-005 | Hand ownership to another person | A server can change hands without rebuilding shares and history | Plex: unclaim and reclaim, and shares and watch state do not follow; Jellyfin and Emby: not applicable (local admin) | Low: one row in the operations research | Later | Ownership is a role on a local account, so a transfer is a role change and every profile keeps its history and shares What this adds beyond ADM-052 (R1, add a second admin and remove yourself) is a single owner-role transfer that both people confirm; until it ships, ADM-052 is the way. | Owner-role transfer confirmed by both people | Admin > Users; confirmation dialog |

### Accounts and identity

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| ACC-006 | Local user accounts | The owner adds people who sign in to this server, not to a company | Plex no (plex.tv accounts); Jellyfin yes; Emby yes | High: baseline for every self-hosted server | R1 | Parity with Jellyfin | User records in the identity store; create, edit and remove with per-object authorization | Admin > Users |
| ACC-007 | No user list before sign-in | Strangers who reach the sign-in page cannot see or guess usernames | Plex not applicable; Jellyfin yes, new users hidden by default; Emby picker with a hide option (unverified) | Low: listed as an attacker aid in the research | R1 | The sign-in page never lists users, and unknown and wrong usernames get the same answer in the same time | Uniform sign-in errors and timing | Sign-in screen |
| ACC-008 | Disable an account without deleting it | Pause someone's access and keep their history | Plex: remove the share (unverified); Jellyfin yes; Emby not checked | Low | R1 | Disabling bumps the account's session epoch, so open sessions and stream URLs stop: new requests fail at once, and an in-flight response is cut by the ACC-122 mechanism. Jellyfin needed a 12.1 fix to end open sessions on revocation | Account state; per-user session epoch checked by signed URLs | Admin > Users > user, Disable switch |
| ACC-009 | Deletion with a grace period | A mistaken deletion can be undone for a while | Plex, Jellyfin and Emby no (unverified); Immich keeps a deleted user for 7 days by default | Low | Later | Deletion writes a tombstone to the user log and a purge job removes the data after the grace period | Tombstone event; scheduled purge; restore | Admin > Users > Recently deleted |
| ACC-010 | Export your own data | A user takes their listening and watch history, playlists, ratings and settings with them | None verified; WatchState and similar third-party tools exist to move history | Medium: Jellyfin "Watched History" 830 votes and an export request; third-party movers with over 1,000 stars each | R1 | History is an append-only, exportable log (ADR 1), so export is a slice of that log in a documented format. It is self-service and needs no admin | Per-user export job over the log and the durable user-data store | Account > Your data |
| ACC-011 | Name and picture for each profile | People recognise their own profile at a glance | Plex, Jellyfin and Emby yes (detail unverified) | Low | R1 | Pictures are re-encoded to a raster format and stored under content-hash names. SVG is refused and OIDC avatar URLs are never fetched, which answers the SVG and SSRF advisories at Jellyfin and Immich | Image upload parser in the core with typed errors; content-addressed storage | Profile editor; profile picker |
| ACC-012 | Preferences that follow you | Language, theme and playback settings are the same on every device a person uses | Plex account-level (unverified); Jellyfin stores settings per client, synced only by plugins | Medium: Jellyfin per-user language request has 219 votes; Moonfin needs two server plugins to sync settings | R1 | Preferences are user data on the server, delivered with the synced library, so a new device picks them up with no plugin | Per-profile preference documents in the user log, included in sync | Settings (per profile) |
| ACC-013 | Identity data that survives a cache rebuild | Users, passkeys, devices, policies, invitations and shares are never lost to an upgrade or a rescan | Plex: shares may not survive a move; Jellyfin: users travel inside a backup | Medium: flagged as a risk in three research files | R1 | Identity and sharing data live in the durable, exportable store alongside history, not in the rebuildable SQLite cache. This needs a new architecture record | Durable identity store with an additive, versioned format; backup and restore | Admin > Backup (shared with the operations map) |
| ACC-014 | Several servers in one app | See CLI-018, which owns this feature. Accounts specifics: the app pins each server's identity key, and enrolment is per server, so no central service learns which servers a person uses. | Plex yes (one account); Jellyfin requested (177 votes); Emby yes, through Emby Connect | Medium: Jellyfin #47, 177 votes | R2 | See CLI-018. | None beyond CLI-018. | Server switcher; Add server |
| ACC-015 | Move to a new phone in one step | Server memberships come along without asking each owner for a new invite | Plex yes (sign in to plex.tv); others not covered in the research | Low: named as a hard part in the research | Later | The old phone approves the new one for every membership it holds, device to device, by QR code, and each server records the new device key | Device-to-device approval protocol; per-server device enrolment | Settings > Move to a new device |
| ACC-133 | One sign-in across several Gunmetal servers | A friend signs in once and sees every server shared with them, as plex.tv does for Plex | Plex yes (one plex.tv identity across servers); Jellyfin and Emby no | Low: no vote data | Later | Open decision: without a central account, the likely design is a device key that several servers trust, vouched for by the friend's home server or by an invite chain; nothing is built until that design is reviewed | Cross-server trust of device keys (to be designed) | Server switcher |

### Households and profiles

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| ACC-016 | Household | People who live together are grouped apart from friends, with shared devices and defaults | Plex Home, up to 15 members; Jellyfin no (#493, 202 votes); Emby no (unverified) | Medium: Jellyfin #493, 202 votes | R2 | A household is a first-class object with no member cap. It owns shared devices and default policies | Household entity and membership; household-owned devices and defaults | Admin > Household; profile picker |
| ACC-017 | Profiles separate from sign-in | Each person has their own history, queue, ratings and taste, even on a shared device | Plex yes (Home users); Jellyfin: every user is a full account; Emby not checked | Medium: Jellyfin account switching #2353, 154 votes, started | R1 | The model separates the credential (passkey or device key) from the profile, so one signed-in device can open several profiles, each with its own part of the history log R1 ships the data model (profiles separate from sign-in) so household features in R2 need no migration; the household, picker and parental rows (ACC-016, ACC-018 to ACC-034) are R2. | Profile entity linked to credentials; per-profile partition of the log and user data | Profile picker; Account settings |
| ACC-018 | Managed child profiles | A child gets a profile with no email or password | Plex yes; Jellyfin no; Emby no (unverified) | Medium: Jellyfin "child mode" 77 votes, profile PIN 62 | R2 | A managed profile has no credentials at all and opens only from an adult's device or a household device. Its restrictions are applied when its synced library is built; on a shared device the protection is weaker (see ACC-030) | Managed-profile type; policy link; sync filter | Admin > Household > Add a child; profile picker |
| ACC-019 | Profile picker and instant switching | Switch person on the sofa without signing out | Plex yes (Home switching); Jellyfin started (#2353, 154 votes); Emby in some apps (unverified) | Medium: 154 votes | R2 | Switching opens another profile's synced data on the device, so it does not wait on the server for anything except the PIN check | Profile list per credential or household device; switch endpoint | Profile picker at launch; avatar menu |
| ACC-020 | Profile PIN | Children cannot switch into a parent's profile, and the digits are hidden as they are typed | Plex yes (a request to hide PIN entry has 111 votes); Jellyfin no (62 votes); Emby not checked | Medium: 62 and 111 votes | R2 | The server checks the PIN and rate-limits attempts with the shared limiter (ACC-063). The client opens a profile's local data only after the switch is approved | PIN hash per profile; attempt limiter; switch approval | PIN pad; Profile settings |
| ACC-021 | Household devices | The living-room TV or speaker belongs to the household, shows its profiles and needs nobody's password | Plex Home (detail unverified); others not covered in the research | Medium: open question in the research about shared TVs | R2 | The TV enrols with a household device key approved by an adult. It can open only the profiles the household allows, with PINs on adult profiles | Household device enrolment; allowed-profile list per device | TV pairing screen; Admin > Household > Devices |
| ACC-022 | Guest mode on a shared device | A visitor plays music on the household speaker without touching anyone's history or taste | Not covered for servers; the Apple TV app has profiles without accounts (tvOS 26.2) | Low | Later | A guest session is an unrecorded session (ACC-117) on a household device, limited by a guest policy | Guest policy; unrecorded session type | Profile picker > Guest |

### Parental controls

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| ACC-023 | Restriction presets | One choice gives an age-appropriate profile: younger child, older child or teen | Plex yes (whether presets need Plex Pass is unclear); Jellyfin no (#3513, 8 votes); Emby no (unverified) | Medium | R2 | Presets are ordinary named policies built from the same rules as everything else, so a family can copy and adjust one. In R1 they cover music and libraries; R2 adds film and TV ratings | Built-in policy templates; policy copy | Add a child > preset chooser |
| ACC-024 | Explicit-content filter for music | Children's profiles skip explicit tracks and play the clean edition when the library has one | None verified at Plex, Jellyfin or Emby; Audiobookshelf denies explicit content to new users by default | Medium: named as a music-first gap; no vote count found | R2 | The core parsers read advisory flags from tags at scan time (which tags carry them is unverified), owners can mark items by hand, and the filter is applied when the profile's synced library is built | Explicit flag in the music model from tags and overrides; clean-edition link in release grouping; sync filter | Explicit badge on albums and tracks; Mark as explicit; child-profile settings |
| ACC-025 | Untagged and unrated items, per media type | The owner decides whether untagged music, home videos or unrated films reach children | Plex not documented; Jellyfin yes, per media type; Emby yes | Medium: unrated home videos and music are common | R2 | The choice is per media type in the policy, enforced at sync, and the settings screen shows how many items each choice hides | Policy field per media type; hidden-item counts | Child-profile settings |
| ACC-026 | Maximum film and TV rating | Content above a rating is hidden | Plex presets free, custom rules need Plex Pass; Jellyfin yes, free, with sub-ratings; Emby yes, free | Medium: all three have it | R2 | Parity on the rule. The edge is enforcement: the ceiling is applied in the shared core when the synced library and signed URLs are built, so restricted items never reach the device | Rating per item and country system; ceiling in policy | Child-profile settings |
| ACC-027 | Rating systems from many countries | Ratings make sense wherever the family lives | Plex by country (unverified); Jellyfin many systems, four fixed in 12.0; Emby not checked | Low: Jellyfin still fixing systems in 2026 | R2 | Parity. The equivalence table is data with its own tests; the licence of a source for it is unverified | Rating-system tables and equivalences | Settings > Region; child-profile settings |
| ACC-028 | Allow and block by tag, genre, library or rule | "Only what I labelled for the kids", or "never this genre" | Plex labels, custom rules need Plex Pass; Jellyfin allow and block tags (#235, 86 votes); Emby both | Medium: #235 (86 votes); genre and library filters #2585 (25) and #1775 (14) | R2 | Rules match a genre, a file tag or a LIB-180 label (household-wide), never a MUS-023 personal tag. Values are picked from existing ones, so a typo cannot silently break the filter (Jellyfin #4136), and the rules use the same language as smart playlists | Policy rules in the shared rule language; tag value index | Child-profile settings (rule editor with autocomplete) |
| ACC-029 | Exceptions for single items | A child may have one album or film without loosening the whole profile | None; Jellyfin #2737 (26 votes) and #4035 (7) | Low: 33 votes across two requests | R2 | A per-profile allow list of item IDs overrides the ceilings and is enforced by the same sync filter | Exception list in the policy | Item menu > Allow for a child; child-profile settings |
| ACC-030 | Every path obeys the restrictions | Search, artwork, folder view, recommendations, screensavers and offline copies never show a blocked item | Plex removes folder view while restrictions are on; Jellyfin filters home art but declined a screensaver filter; Emby not checked | Medium: the research calls kids mode "security, not styling" | R1 | The server filters when it builds a profile's synced library and on every object fetch. On a device only the child uses, restricted items never arrive. On a shared device they are hidden by the client: the adult profile's cache is on that device, and a child with file access to the device could read it. Encrypting each profile's local store is ACC-132 (Later). | Sync filter; per-object authorization; cross-profile tests on every route | None (behaviour), tested on every browse screen |
| ACC-031 | Restricted profiles still get discovery | Children get home rows and recommendations within what they may see | Plex: Discover was missing for managed users (104-post thread); Jellyfin yes | Low: 104-post thread | R2 | Rows are computed from the already-filtered synced library, so there is no separate code path for children | Nothing extra; the shared row engine | Home |
| ACC-032 | Listening and viewing schedules | A child's profile works only during set hours, with a clear message outside them | Plex no (unverified); Jellyfin yes; Emby yes, with a block message confusing enough to start a forum thread | Low | R2 | The server refuses new stream URLs outside the window and the client says why. Downloads on the child's device carry the schedule in their offline grant | Schedule in policy, enforced at URL issue and in offline grants | Child-profile settings; blocked-time message |
| ACC-033 | Daily time allowance | "Two hours a day", not only "between four and seven" | None; Jellyfin #699 (29 votes) | Low: 29 votes | Later | The server counts from playback events in the log. Offline play is counted when the device reconnects, which is an honest gap | Per-profile daily counter from the log; enforcement at URL issue | Child-profile settings; time-left indicator |
| ACC-034 | Parents see what children played | History per child profile, visible to the household's adults | Plex yes (unverified); Jellyfin activity log (unverified); Emby yes (unverified) | Low | R2 | The child's profile states that its history is visible to parents, so the rule is not hidden from the child | Per-profile history from the log; read right for household adults | Household > child > History |
| ACC-035 | Ask a parent | A child requests a blocked item and a parent approves it from their own device | Not covered in the research | Low: no evidence found | Later | Approval creates a per-item exception (ACC-029) | Request queue; notice to household adults | Blocked-item screen; approval prompt |
| ACC-036 | Live TV channel restrictions | Children see only some channels | Plex through sharing restrictions; Jellyfin user policy; Emby tag exclusions reported failing for live TV | Low: per-channel access requests have 34 and 5 votes | R3 | Channels are objects with their own permissions, so the same policy layer covers them | Channel objects in the policy model | Child-profile settings > Live TV |
| ACC-132 | Per-profile local stores encrypted on shared devices | A child on the family tablet cannot read the adult profile's cached library, even with file access | No rival found that encrypts per-profile caches (unverified) | Low: no vote data | Later | Each profile's local store is encrypted with a key the server releases only after the profile switch is approved, which turns ACC-030 from a UI gate into real protection on shared devices | Per-profile key release on approved switch; device key (ACC-051) | Profile picker |

### Permissions and roles

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| ACC-037 | Library access per person | Each person sees only the libraries chosen for them | Plex, Jellyfin and Emby yes; Navidrome added it in 0.58, then fixed many leaks in 2026 | High: baseline; leak fixes at Navidrome and Jellyfin (#17215) | R1 | Enforced when the synced library is built and on every fetch, with tests that replay one user's IDs as another user on every route | Library grants in policy; sync filter | Admin > Users > Libraries |
| ACC-038 | Named access policies shared by many users | Set up "household" or "friends" once and change it in one place | Plex no (unverified); Jellyfin no: groups #493 (202 votes), global defaults (258); Emby copies settings from another user, templates requested (174 replies) | Medium: over 500 Jellyfin votes across the requests | R2 | Users link to a policy rather than copying it, so changing the policy changes everyone on it, and per-user overrides stay possible | Policy entity; user-to-policy links; override layer | Admin > Policies |
| ACC-039 | Default preferences for new profiles | New people start with the household's language, quality and privacy settings | Plex not applicable (unverified); Jellyfin no (#363, 130 votes); Emby imports another user's settings | Medium: 130 votes | R2 | A default preference document on the household or policy is copied once into each new profile | Default preference document | Admin > Policies > Defaults |
| ACC-040 | Several administrators | More than one person can run the server | Plex one owner; Jellyfin yes; Emby yes | Low | R1 | Parity | Admin role | Admin > Users |
| ACC-041 | Scoped admin roles | A trusted person manages users, or curates the library, without power over plugins or storage | None found (unverified) | Low | Later | Roles are sets of named capabilities from the same scope list as tokens (ACC-049), so a role cannot hold a power that has no scope | Capability catalogue; role definitions | Admin > Roles |
| ACC-042 | Delegated curation rights | Trusted users edit metadata, lyrics or artwork, or manage collections, without being admins | Plex not found; Jellyfin separate switches for collections, subtitles and lyrics; Emby not found | Low: delegated editing request has 7 votes | Later | Uploads are stored under content-hash names, so a curation right can never name a file path. Jellyfin's CVSS 9.9 flaw started from the subtitle-upload right | Capability per curation action; content-addressed uploads | Item edit screens; Admin > Users > Rights |
| ACC-043 | Per-person transcode rights | The owner decides who may cause remuxes or video transcodes | Plex no per-user control (unverified); Jellyfin separate switches; Emby similar (unverified) | Medium: Emby "force direct play" 165 replies; Plex "disable 4K transcoding" 249 votes | R2 | The playback decision engine in the core takes the policy as an input and explains a refusal in plain words. Cheap Opus conversion of music stays allowed by default | Policy fields read by the decision engine | Admin > Users > Playback; player message |
| ACC-044 | Offline download rights | Allow or deny offline copies per person | Plex needs Plex Pass; Jellyfin free, per user; Emby needs Premiere | Medium: offline is a top-three demand theme | R2 | Free. Each download carries a signed offline grant tied to the device key | Download capability; grant issue | Admin > Users > Downloads |
| ACC-045 | See and revoke offline copies | The owner sees which devices hold which items and can revoke a lost device's downloads | Emby best (admin controls all synced media); Jellyfin no; Plex not checked | Low | R2 | The server records each grant per device. Revocation takes effect on next contact, and grants expire on a schedule the policy sets | Grant registry; expiry; revocation on contact | Admin > Devices > Downloads; Account > Devices |
| ACC-046 | Delete-from-library right | Trusted users remove media, with an undo | Plex server-wide (unverified), "allow managed users to delete" 155 votes; Jellyfin per user and per folder; Emby not checked | Low: 155 votes | Later | Deletion goes to the trash with a grace period and is logged with who did it Needs ADM-139 (opt-in writable root); without it there is nothing to delegate. | Delete capability; trash; audit event | Item menu > Delete; Admin > Trash |
| ACC-047 | Control another person's player | Pause or cast to someone else's session only when they allow it | Plex not checked; Jellyfin per user, off by default, with a broken-access-control advisory in Sept 2026; Emby not checked | Low | Later | Control is a capability held by the session's owner; another user needs a grant from them. A person's own devices are covered by queue hand-off in the music map | Session-control capability and grant | Device picker |
| ACC-048 | Live TV and recording rights | Separate permission to watch live TV and to schedule recordings | Plex: Home members only, who cannot schedule; Jellyfin separate switches; Emby DVR needs Premiere | Low | R3 | The same policy layer, with channels and recordings as objects | Live TV capabilities in policy | Admin > Users > Live TV |
| ACC-049 | Scoped tokens for integrations | A script, Lidarr or a stats tool gets only the access it needs | Plex tokens carry the whole account (unverified); Jellyfin keys have no scopes (#13992, 23 +1); Emby no scopes (unverified); Immich has scopes | Medium: Huntarr and Tautulli flaws in 2026 leaked all-powerful keys | R1 | Tokens carry named scopes, travel only in headers, show last-used time and can expire. An update can never grant more than the caller holds (Immich CVE-2026-23896) Owns scoped tokens; INT-017 points here. | Capability model with scopes; token store; no-escalation check | Admin > Integrations; Account > Tokens |

### Sign-in

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| ACC-050 | Passkeys | Phishing-resistant sign-in with nothing to remember or leak | None among media servers; Plex request 27 votes, Jellyfin 5 | Medium: low votes, but two plex.tv breaches and ADR 1 commits to it | R1 | Passkeys are the default where the server has a domain name with HTTPS (ADM-022) or is opened on localhost; a passkey-only account has no password stored anywhere. Elsewhere in R1, sign-in falls back to password plus two-factor (ACC-052, ACC-053), which is parity with Plex and the Jellyfin plugins. ADM-023 is the decision that sets how many R1 users get passkeys. | WebAuthn registration and sign-in; credential store; relying-party setting | Sign-in; first-run setup; Account > Sign-in methods |
| ACC-051 | Device keys for native apps | Phones, TVs and desktops stay signed in with a key that never leaves the device | None | Medium: follows from ADR 1 | R2 | Each app keeps its own key pair in the platform's secure storage, so it needs no domain and each device can be revoked alone | Device-key enrolment; challenge-response sign-in; per-device revocation | Pairing screens; Account > Devices |
| ACC-052 | Password sign-in as a fallback | Sign in from a browser that cannot use passkeys | Plex holds passwords at plex.tv, breached in 2022 and 2025; Jellyfin and Emby hold them on the server | Medium | R1 | Passwords live only on the owner's server, can be turned off per user or for the whole server once a passkey exists, and share the limiter used by every surface | Password hashing; policy switch; limiter | Sign-in; Account > Sign-in methods |
| ACC-053 | Two-factor codes with recovery codes | A code from an authenticator app protects password sign-in | Plex yes; Jellyfin not built in (#26, 1,103 votes, planned); Emby no (requested since 2018) | High: 1,103 votes; Emby thread with 459 replies | R1 | Parity with Plex; ahead of Jellyfin and Emby, which lack it natively. In R1 this is the default protection for most LAN installs, because passkeys need a domain (ACC-050). Recovery codes are single-use. Adapters cannot bypass it because they never accept the account password (ACC-129) | TOTP secrets; recovery codes; verification | Second sign-in step; Account > Sign-in methods |
| ACC-054 | Require strong sign-in | The owner insists that admins, or everyone, use a passkey or a second factor | Not covered in the research | Low | R2 | A policy field checked at sign-in, recommended on by default for admin accounts Deferred from R1 to R2 to keep the first release small (feature map README, "The R1 cut"). | Policy field; enforcement at sign-in | Admin > Policies > Sign-in |
| ACC-055 | Manage your own sign-in methods | Add, name and remove passkeys, a password, two-factor and linked identity providers | Plex security page at plex.tv; others not covered in the research | Low | R1 | The server refuses to remove a person's last working method, and each change needs a fresh confirmation (ACC-056) | Credential list per user; last-method guard | Account > Sign-in methods |
| ACC-056 | Confirm sensitive changes | Adding an admin, changing sign-in methods or creating a token asks for a fresh passkey touch | Not covered in the research | Low | R1 | A short-lived step-up grant on the session, required by sensitive routes in their declared policy | Step-up grant; route policy flag | Confirmation prompt |
| ACC-057 | Single sign-on with your own identity provider | Sign in through Authelia, authentik, Pocket ID or similar | Plex no; Jellyfin plugin only, web only, archived 2026-05-12 (#230, 1,191 votes, planned); Emby requested (216 replies, status unverified); built into Immich, Audiobookshelf and Kavita | High: 1,191 votes; selfh.st 2025 counts Authelia 390, Pocket ID 310 and authentik 268 respondents | R1 | Built in, not a plugin. Web client in R1; native apps in R2 through the system browser and a redirect back to the app. TLS is always verified, avatar URLs are never fetched, and Immich's 2026 OIDC advisories become test cases | OIDC client with discovery, state and nonce checks; redirect allow-list; account linking | Sign-in > Continue with provider; Admin > Sign-in > OIDC |
| ACC-058 | Provider claims map to policies | People from the identity provider land in the right policy and libraries automatically | Immich: auto-registration with role and quota claims; others not covered in the research | Medium: the same OIDC demand | R2 | Claims map to named policies (ACC-038), so the provider can never grant a capability directly | Claim-to-policy mapping; auto-provision switch | Admin > Sign-in > OIDC mapping |
| ACC-059 | LDAP directory sign-in | Use an existing household or company directory | Plex no; Jellyfin official plugin; Emby plugin (unverified) | Medium: Navidrome LDAP issue has 148 reactions, open since 2020 | Later | Recommended route is an identity provider in front of the directory, through OIDC, first (see open decisions) | LDAP bind and search; group-to-policy mapping | Admin > Sign-in > LDAP |
| ACC-060 | Sign-in from a trusted reverse-proxy header | Authelia or authentik in front of the server signs the person in | Plex no; Jellyfin no (unverified); Navidrome yes, with a list of trusted sources | Low | Later | Off by default and accepted only from listed proxy addresses. The Emby 2023 compromise shows what a forged header costs | Trusted-source list; header mapping | Admin > Sign-in > Proxy |
| ACC-061 | Sign in a TV with a QR code | Point a phone at the TV, see which TV and profile it is, approve | Plex link code (unverified); Jellyfin Quick Connect 6-digit code, QR planned (#2642, 225 votes); Emby PIN through Emby Connect | Medium: 225 votes | R2 | The approving phone shows the TV's name and the requested profile before approval. The TV gets its own device key, never a password. A short typed code is the fallback | Pairing request; approval; device-key enrolment; expiry; rate limit | TV sign-in screen; phone approval sheet |
| ACC-062 | Approve a new browser from a signed-in one | Sign in a borrowed laptop or a TV browser without typing a password | Jellyfin Quick Connect (admin-enabled); others not covered in the research | Low | R2 | The same pairing flow as ACC-061, so the web client has it before native TV apps exist R2, with ACC-061: one pairing protocol for both. | Pairing request and approval | Sign-in > Use another device; approval prompt |
| ACC-063 | Protection against guessing | Guessing passwords, PINs, pairing codes or share-link passwords is slow or impossible | Plex handles it at plex.tv (unverified); Jellyfin lockout unlimited by default; Navidrome sliding window, Subsonic path only covered in Sept 2026 (CVSS 7.4) | Medium: Navidrome's 2026 advisory | R1 | One limiter covers every surface, adapters included, keyed on account and on the connection's real source address unless a trusted proxy is listed | Shared limiter; failure counters; lockout notices | Sign-in messages; Admin > Security |
| ACC-064 | Help a locked-out user | The admin issues a one-time sign-in link instead of reading out a password | Plex email reset; Jellyfin reset provider (unverified); Immich temporary password that must be changed | Low | R1 | The link is single-use and short-lived and leads straight to enrolling a new passkey | One-time link store | Admin > Users > Help sign in (with QR) |
| ACC-065 | Sign out everywhere when a credential changes | A leaked password or lost phone stops working on every device at once | Plex yes, offered during reset; Jellyfin and Emby not checked | Medium: Plex told every user to do this after its 2025 breach | R1 | A credential change bumps the session epoch that every session and signed URL carries | Per-user session epoch | Password change dialog; Account > Devices |
| ACC-066 | Passwordless sign-in on the local network | Being on the home network counts as proof of identity | Plex allowed networks without auth (unverified); Emby offered it, and it led to the 2023 compromise | Low: a convenience; its cost at Emby was about 1,200 backdoored servers | No | Not offered. Forged headers made remote attackers look local at Emby. Household devices (ACC-021) give the same convenience safely | None | None |
| ACC-067 | "Continue with Google or Apple" buttons | One-tap social sign-in | Plex yes; Jellyfin no; Emby no | Low | No | Not offered as built-in buttons: they would need a central Gunmetal app registered with those companies. An owner can add a large provider as an ordinary OIDC provider through ACC-057 (per-provider support unverified) | None | None |

### Devices and sessions

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| ACC-068 | Your devices, in one list | Every phone, TV, browser, app password and token on the account, with last-seen time | Plex at plex.tv; Jellyfin admin devices page (detail unverified); Emby yes (unverified); Immich per user (unverified) | Medium | R1 | Each user, not only the admin, sees their own list, and it includes app passwords and tokens as well as devices | Device and credential registry with last-seen | Account > Devices |
| ACC-069 | Revoke one device | Remove a lost phone | Plex yes; Jellyfin yes, after a 12.1 fix; Emby yes (unverified) | Medium | R1 | New requests from the device fail at once, and an in-flight response is cut by the ACC-122 mechanism. Offline grants end on next contact. | Revocation; session binding | Account > Devices > Remove |
| ACC-070 | Sign out of all sessions | One button after a scare | Plex yes; Jellyfin requested (9 votes); Audiobookshelf has it since 2.36 | Medium: Plex's advice after its 2025 breach | R1 | Uses the same session epoch as ACC-065 | Per-user session epoch | Account > Devices |
| ACC-071 | New-device alerts | "A new TV signed in to your account" appears on the person's other devices | None verified | Low | R2 | Device keys make a new device a precise event rather than a guess from IP addresses. Alerts appear in the app, and by email or webhook only if the owner configures one | Enrolment event; in-app notice feed | Notice centre; Account > Devices |
| ACC-072 | Live sessions for admins | See ADM-099, which owns this feature. | Plex, Jellyfin and Emby yes | Medium: baseline | R1 | See ADM-099. | None beyond ADM-099. | Admin > Now playing |
| ACC-073 | Stop a stream with a message | See ADM-102, which owns this feature. Accounts specifics: stopping revokes the session, third-party apps included. | Plex Pass (unverified); Jellyfin requested (#301, 402 votes); Emby not checked; Tracearr automations | High: 402 votes | R1 | See ADM-102. | None beyond ADM-102. | Admin > Now playing > Stop; client banner |
| ACC-074 | Message one person or everyone | See ADM-103, which owns this feature. | Plex requested (1,206 votes); Jellyfin per session only, broadcast requested (89 votes); Emby requested (119 replies) | High: 1,206 votes | R2 | See ADM-103. | None beyond ADM-103. | Admin > Messages; client banner |
| ACC-075 | Stream limits that count playing, not browsing | Cap simultaneous streams per person without misfires | Plex not built in (unverified); Jellyfin counts every connected session; Emby per user, enforcement may need Premiere (unverified) | Medium: Jellyfin #1444 (85 votes), Emby 161 replies, Infuse users' reports | R2 | A stream holds a playback lease and browsing never does. A paused stream gives its lease back after a set time | Lease registry per user; idle release | Admin > Policies > Limits; "too many streams" message |
| ACC-076 | Device limits and allow-lists | An account works only on approved devices, or on at most a set number | Jellyfin device allow-list; Emby caps households at 30 devices; Plex no cap found | Low | R2 | Device keys make the count exact. Any cap is the owner's choice, never a vendor limit | Device count and allow-list in policy | Admin > Users > Devices |
| ACC-077 | Spot shared passwords | See when one account plays from many places at once | Tracearr (impossible travel, trust scores) works with all three | Low: Tracearr has 2,680 stars | Later | Device keys turn account sharing into "many devices on one account", which is visible without guessing from IP addresses | Per-account device and location summary | Admin > Security > Sharing report |
| ACC-078 | Sign-in and admin audit log | See ADM-110, which owns this feature. Accounts specifics: retention and IP truncation settings, and each user can read their own sign-in entries. | None verified complete; Jellyfin activity log; a request to anonymise it has 33 votes | Medium | R1 | See ADM-110. | None beyond ADM-110. | Admin > Activity; Account > Sign-in history |
| ACC-079 | Session lifetimes | Remembered devices stay signed in; a browser on a shared computer does not | Plex moved to 7-day tokens; others not covered in the research | Low | R1 | Lifetimes are a policy field. Device keys can stay valid for long because each is revocable on its own | Session expiry policy | Admin > Policies > Sessions; "Remember this browser" |

### Invitations

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| ACC-080 | Invite by link or QR code | The owner sends one link and the person is in, with the right libraries | Plex by email through plex.tv; Jellyfin no (sign-up page #494, 227 votes), with Wizarr and jfa-go filling the gap; Emby Connect linking | High: Wizarr exists only for this (3.2k stars); 227 votes | R1 | An invite is a capability with an expiry, a use count and a policy. Redeeming it creates the account and enrols a passkey on the spot, with no central account | Invite store; redemption; policy link; revocation; redemption log | Admin > Invitations; invite landing in the web client |
| ACC-081 | Membership that ends on a date | A friend's access stops on its own | Wizarr only | Low | R2 | An end date on the membership disables the account (ACC-008) automatically | Expiry job | Invite options; Admin > Users |
| ACC-082 | Invitee onboarding page | "Install this app, then tap here" | Wizarr has onboarding guides; Plex uses the app the friend already has | Medium: the research names the invitee's first minutes as the main friction | R2 | A static page on gunmetal.tv reads the invite secret from the URL fragment, so it never reaches any server log before the app takes it. There is no account at gunmetal.tv | Static page on the project site; app link handling | Invite landing page; app first launch |
| ACC-083 | Invites carry the server's address | The friend never types an address or port | Plex through the account; Jellyfin no; Emby through Emby Connect | Medium | R2 | The invite holds the server's iroh node address and identity key, so the app connects and pins the server without a domain | Node address and key in the invite | Invite QR; Add server |
| ACC-084 | Re-invite people from your old server | Moving from Plex, Jellyfin, Emby or Navidrome keeps the server's people, as invitations | None built in; Wizarr and third-party importers exist | Medium: switching cost keeps lifetime Plex users where they are | R2 | Imported users arrive as pending invitations with their old library grants, because credentials cannot move | Importer output to invitations (operations map) | Import report; Admin > Invitations |
| ACC-085 | Ask to join, with approval | A friend requests an account and the owner approves | Plex through plex.tv; Jellyfin no (#494, 227 votes); Emby no (unverified) | Medium: 227 votes | Later | Requests arrive only through a link the owner published, are rate-limited, and stay pending until approved | Request queue; approval | Request page; Admin > Requests |

### Sharing

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| ACC-086 | Share links for music | Send a track, album or playlist to anyone, with no account needed | Navidrome yes (on by default since 0.63); Immich yes; Jellyfin no (#72, 580 votes); Plex requested (294 votes) | High: 580 and 294 votes | R2 | A link is a scoped, expiring capability, and every stream request re-checks it, so deleting a share stops playback at once. Navidrome fixed stale shared stream URLs in Sept 2026 R2: share links are an unauthenticated surface of the kind where Navidrome published several 2026 advisories, and a personal music server does not need them first. Owns music share links; MUS-151 points here. | Share store; signed URLs that consult the share; default expiry | Share sheet on tracks, albums and playlists; public listening page |
| ACC-087 | Password on a share link | Only people with the password can open it | Immich yes; Navidrome not documented | Low | R2 | Password attempts go through the shared limiter (ACC-063) | Share password hash | Share sheet |
| ACC-088 | Download switch on a share link | Listen only, or allow download | Immich yes, per link; Navidrome allows downloads (switch unverified) | Low | R2 | The download right is part of the capability, so a listen-only link cannot fetch the original file | Capability field | Share sheet |
| ACC-089 | Your shares, and everyone's for admins | See, change and revoke every link you made | Navidrome fixed missing ownership checks in Sept 2026 | Medium: two Navidrome advisories | R2 | A share's owner comes from the session, never from client input, which was the Navidrome flaw | Share list per owner; admin view | Account > Shares; Admin > Shares |
| ACC-090 | Link previews in chat apps | A link shows artwork and title in Discord or a messenger | Navidrome yes, through page meta tags; Jellyfin no | Low | Later | Off by default, because previews leak titles to the chat platform, and switchable per link | Preview metadata page | Share sheet > Show preview |
| ACC-091 | Shared and collaborative playlists | Family members view, or add to, one playlist | Spotify yes; Plex shares to Home users; Navidrome public playlists, collaborative not planned; Jellyfin request to protect playlists from non-owners (19 votes) | Medium: friends' and shared playlists are what people leaving Spotify say they miss | R2 | Playlists are objects with viewer and editor roles under the same authorization layer. Edits are log events, so offline edits by two people merge Owns shared and collaborative playlists; MUS-150 points here. | Playlist roles; edit events; merge rules | Playlist > Share with people; collaborator avatars |
| ACC-092 | Share links for films and episodes | Send one film or episode to someone with no account | None among video servers; Jellyfin #72 (580 votes); Plex 294 votes | High: 580 votes | R2 | The same capability as ACC-086, off by default behind an owner switch because of bandwidth and copyright exposure (unverified legal analysis) | Video share capability; bandwidth cap per link | Share sheet on films and episodes |
| ACC-093 | Watch together with guest links | Send a link and watch in sync; guests need no account | Plex dropped it from the new apps (2,878-vote restore request); Jellyfin SyncPlay, invite link requested (#971, 267 votes); Emby requested | High: 2,878 votes | Later | A guest capability scoped to one title and a time window, over iroh. Guests see nothing else in the library Guest links use ACC-135; the synchronised session is VID-153, which owns group sessions. VID-154 points here. | Guest capability; control channel (video map) | Invite to watch; guest join screen |
| ACC-094 | Listen together | See VID-153, which owns this feature. Accounts specifics: the guest capability (ACC-135) is scoped to a queue rather than a title. | Spotify Jam; Plex requested (76 votes, closed); Navidrome requested (30 reactions) | Low | Later | See VID-153. | None beyond VID-153. | Queue > Invite to listen |
| ACC-135 | Single-item guest capability | A link that lets someone with no account play one item or join one queue for a set time, revocable at any moment | Plex Watch Together needs plex.tv accounts; Jellyfin SyncPlay needs local accounts (research) | High: Watch Together is the second most-voted Plex suggestion (2,878 votes) | R2 | The primitive behind ACC-093, ACC-094, LAT-109 and share links: a signed capability scoped to one item or queue and a time window, re-checked on every request, built on ADR 1's per-object authorisation and signed URLs. Shipping it in R2 makes sure the capability model allows Watch Together later; whether Watch Together itself moves to R2 is an open decision | Capability issue, scope, expiry and revocation; minimal guest player route | Share sheet; Settings > Shares |
| ACC-095 | Send to someone on this server | Recommend an album to a household member without a public link | Not covered for servers; Spotify shares inside its own app | Low | Later | The item reaches the recipient only if they may already see it, checked by the normal authorization layer | Inbox per profile | Share sheet > People on this server; inbox |

### Remote access

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| ACC-096 | Remote access with no open ports, no account and no fee | The apps reach home from anywhere, including behind carrier-grade NAT | Plex relay capped at 2 Mbps, and remote video needs Plex Pass or a Remote Watch Pass since 2025-04-29; Jellyfin needs your own VPN or proxy; Emby none | High: Plex paywall backlash; selfh.st 2025: VPN 2,902 and reverse proxy 2,716 respondents | R2 | Native clients embed iroh; the project's own connection-success figures will replace iroh's quoted rate of about 90% direct (unverified for mobile carriers and TVs). No open port; whether there is no fee behind CGNAT depends on the relay decision (ACC-101). Browsers still need the owner's proxy or VPN until ACC-102. Lands in R2 with the native clients; in R1, remote use means the owner's reverse proxy, VPN or domain (ACC-097). | iroh endpoint and node identity in the server; relay configuration | Admin > Remote access; connection badge in clients |
| ACC-097 | Works behind your own reverse proxy or VPN | Use Caddy, Tailscale or your own domain from the first release | Jellyfin expects this; Plex: VPNs can make local devices count as remote and trigger the paywall | High: the same survey | R1 | Forwarding headers are ignored unless the owner lists trusted proxies, so nobody can forge "local" (Emby 2023, Jellyfin 2025, Navidrome 2026) | Trusted-proxy list; source-address resolution | Admin > Network |
| ACC-134 | Serve under a path prefix behind a reverse proxy | Run Gunmetal at example.com/gunmetal next to other services | Jellyfin yes ("Base URL"); others not covered in the research | Medium: common reverse-proxy setup (no vote count in the research) | R1 | The prefix is one setting. WebAuthn checks the origin (scheme, host and port), which a path does not change, so passkeys work unchanged; signed stream URLs are signed over the path after the configured prefix is removed, so the proxy can add it back | Configured base path; prefix-aware router, asset URLs and URL signer | Admin > Network |
| ACC-098 | HTTPS served by the server | The web client works over HTTPS, which passkeys need | Plex automatic through Plex-issued names (detail unverified); Jellyfin bring your own | Medium | R1 | The server serves HTTPS itself with a certificate the owner supplies, or a self-signed one whose fingerprint setup shows Owns R1 HTTPS; ADM-022 is its admin screen. A self-signed certificate does not enable passkeys, because WebAuthn needs a domain name, not an IP address. | TLS listener; certificate load and reload | Admin > Network > HTTPS |
| ACC-099 | Automatic certificates for your own domain | No certificate work at all | Plex automatic (detail unverified); Jellyfin and Emby bring your own | Medium | Later | The server obtains and renews certificates for a domain the owner holds (ACME), with no Gunmetal service in the path Later in both maps (ADM-022 points here); depends on ACC-098. | ACME client; renewal job | Admin > Network > HTTPS |
| ACC-100 | Self-hosted relay | Remote access that depends on nobody but the owner | The iroh relay is open source; Tailscale's control plane can be self-hosted through Headscale (unverified) | Medium: n0's public relays are rate-limited and meant for testing | R2 | The owner runs a relay and points the server and its invites at it | Relay address in config and invites | Admin > Remote access > Relay |
| ACC-101 | Default relay for owners who cannot run one | Remote access works out of the box for non-technical owners | Plex Relay (2 Mbps cap); Jellyfin and Emby none | Medium | R2 | Depends on an owner decision about who runs and pays for it. A relay sees connection metadata such as IP addresses, never content | Relay list | Admin > Remote access |
| ACC-102 | Remote access in the browser over iroh | Listen from a browser away from home with no domain of your own | Plex through app.plex.tv; Jellyfin needs your own proxy | Medium | Later | iroh in the browser is relay-only, so every byte costs relay bandwidth: affordable for music, not for video | WASM iroh client; relay capacity | Web client connection settings |
| ACC-103 | Remote access on or off per person | Grandparents only at home, friends anywhere | Plex: Home or friend status decides (unverified); Jellyfin per-user switch; Emby not checked | Low | R2 | Local or remote is decided by the transport (LAN or iroh), not by headers, so the switch cannot be tricked. The default is set in the invite's policy | Policy field checked per connection | Admin > Users > Remote access; invite options |
| ACC-104 | Find the server on the home network | Apps find the server with no address to type | Plex through the account; Jellyfin local discovery; Emby through Emby Connect | Medium | R2 | Discovery only finds the server; joining still needs an invite or a pairing approval | LAN discovery responder | App first launch; Add server |
| ACC-105 | Connection status that tells the truth | See whether the app is local, direct or relayed, and why it is slow | Plex threads "cannot connect securely" (635 posts) and "Remote Access (again)" (263 posts) | Medium | R2 | The client shows the path and measured throughput, and the project publishes connection success rates as it will publish scan benchmarks | Connection diagnostics | Connection badge; Settings > Connection |
| ACC-106 | Automatic port forwarding | Remote access by opening a port on the router through UPnP | Plex UPnP or NAT-PMP; Jellyfin docs advise against automatic UPnP; Emby UPnP (unverified) | Low | No | Not offered. Opening ports is itself the risk that sank Emby servers in 2023, and iroh needs no open port | None | None |

### Bandwidth and stream limits

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| ACC-107 | Per-person music quality cap | A friend on a slow line cannot starve the house; they get Opus instead | Plex global cap only, needs Plex Pass, per-user request 188 votes since 2017; Jellyfin per user; Emby per user (unverified) | Medium: 188 votes | R2 | A cap switches music to Opus, which is cheap on any hardware (ADR 2), and the player says so R2, with the sandboxed Opus encoder (MUS-106); R1 has no transcoder. Until R2, a capped listener gets the original and the admin is told why. | Policy field; decision-engine input; Opus conversion | Admin > Users > Quality; player quality badge |
| ACC-108 | Per-person video cap that never transcodes silently | A cap picks an existing version or suggests a download, unless the owner allows transcoding for that person | Plex global only (Plex Pass); Jellyfin per user; Emby per user (unverified) | Medium: 188 votes | R2 | The decision engine treats the cap as an input and refuses a GPU transcode unless the policy allows one (ACC-043) | Decision-engine inputs; version list | Admin > Users > Quality; player message |
| ACC-109 | Total upload budget, shared fairly | Stay within the home connection's upload | Plex "Internet upload speed" (Plex Pass); Jellyfin requests #1504 and #1012; Emby not checked | Low | R2 | The decision engine receives the live budget and shares it across playback leases | Upload accounting; budget input | Admin > Network > Upload budget |
| ACC-110 | Downloads give way to streams | Offline syncs never make live playback stutter | Plex yes; Jellyfin and Emby not checked | Low | R2 | Download transfers run at lower priority on the same upload accounting | Transfer priority | None (behaviour) |
| ACC-111 | Transcodes per person | One user cannot take the whole CPU or GPU | None among the three; Navidrome has per-user transcoding caps (0.62) | Low | R2 | Per-policy transcode slots enforced by the sandbox supervisor | Slot counting | Admin > Policies > Limits |
| ACC-112 | Smoothing traffic bursts | Remote streams do not cause lag spikes at home | None; Jellyfin #1185 (15 votes) | Low: 15 votes | Later | Pacing on the same upload accounting as ACC-109 | Send pacing | None (behaviour) |

### Privacy

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| ACC-113 | Nothing leaves the house by default | No telemetry, no phone-home and no vendor copy of anyone's history | Plex collects usage data with opt-outs (detail unverified), and Plexamp's download telemetry cannot be turned off; Jellyfin none; Navidrome sends anonymous statistics by default | High: privacy is the second reason to self-host (3,520 of 4,081 selfh.st respondents) | R1 | The build has no telemetry. Any diagnostic report or update check is opt-in and shows exactly what it would send | None (absence); opt-in diagnostics preview (operations map) | First-run privacy step; Settings > Privacy |
| ACC-114 | No social feed | Nobody learns what a person played unless that person chooses to share it | Plex: "Week in Review" emails and Discover Together exposed viewing in 2023 (2,704 likes on the main thread), and reviews and Discussions followed; Jellyfin has no social features | High: the 2023 leak thread and its spin-offs | R1 | Any household-visible activity, such as "popular in this household", is opt-in per person and stays on the server | Per-profile visibility setting read by every feature that shows activity | Settings > Privacy |
| ACC-115 | What your admin can see | A page that tells each person what the owner can see about them | None | Medium: a gap the research names; the Plex leak shows the trust cost | R2 | Generated from the live authorization policy, so it cannot drift from what admins can really see. It says plainly that whoever runs the machine could read anything | Policy introspection | Account > Privacy > What your admin can see |
| ACC-116 | Choose what admins see | Full history, live sessions only, or totals only | None | Medium | R2 | One server setting read by every admin view, with the default chosen by the owner | Admin-visibility setting | Admin > Privacy |
| ACC-117 | Private listening | Play something without it entering history, recommendations or scrobbles | None among servers | Medium: Spotify's request to remove items from history has 5,458 votes | R1 | The history log has a "not recorded" path, and scrobbling plugins never see a private session Owns private sessions for all media; DIS-053, MUS-185 and VID-130 point here. | Unrecorded session flag | Player > Private session; indicator |
| ACC-118 | Remove plays from history | Keep a guilty pleasure out of the record | None; Spotify request 5,458 votes | Medium | R1 | Removal is an event in the log, so it reaches every device and recommendation. Plays already sent to Last.fm cannot be recalled, and the screen says so | Removal event | History > Remove |
| ACC-119 | Your own scrobbling accounts | Each person links their own Last.fm or ListenBrainz, and the admin never holds their tokens | Navidrome per user; the Jellyfin ListenBrainz plugin needs the admin to enter every user's settings; Plex not verified | Medium: Jellyfin Last.fm and ListenBrainz requests (27 and 28 votes); per-user plugin settings requested | R2 | Plugins keep per-user secrets the admin cannot read, and a profile can switch scrobbling off, for example for a child | Per-user secret storage in the plugin host | Account > Connected services |

### Security you can see

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| ACC-120 | Every endpoint needs sign-in | No anonymous access to streams, images, lyrics, user lists or system information | Jellyfin: #5415 listed unauthenticated endpoints and #13991 is still open; Plex: CVEs published in January 2026 let tokens be escalated or harvested | High: #5415 (114 +1); Jellyfin's 15 advisories in 2026 | R1 | Every route declares a policy and the build fails if one does not. In R1 the only anonymous routes are sign-in and pairing; share pages join them in R2 (ACC-086) | Route table that tests can enumerate | None (behaviour) |
| ACC-121 | Authorization on every object | One user cannot fetch another's items by guessing IDs | None proven; Jellyfin IDs said to derive from file paths (unverified); Navidrome fixed IDORs in 2026 | High: most 2026 advisories at the rivals are authorization mistakes | R1 | Random IDs (ADR 1), one checking layer, and tests that replay user A's IDs as user B on every route, adapters and share links included, all held to 100% coverage and zero surviving mutants | Authorization layer; cross-user test matrix | None (behaviour) |
| ACC-122 | Short-lived, session-bound stream URLs | A leaked stream link stops working within minutes | None proven; Navidrome's shared stream URLs kept working after deletion until Sept 2026 | Medium | R1 | URLs carry a signature bound to the session, the item and an expiry, and re-check revocation A response already in flight is not stopped by a signature checked at its start, and players often stream a whole file from one open-ended range request. So the server caps each range response (size to be set by measurement) and tracks open responses per session, aborting them when the session is revoked; a test proves that revocation ends an in-flight download. | URL signer and verifier | None |
| ACC-123 | Secrets never in URLs or logs | Tokens do not leak into proxy logs, browser history or shared log files | Plex tokens in URL parameters (unverified); Jellyfin `api_key` parameter; Navidrome logged admin passwords on failed setup until 0.64.2 | Medium | R1 | Tokens travel only in headers or cookies. Secret values are wrapped in types that cannot be printed, so logging one does not compile Owns secrets hygiene; ADM-121 points here. | Secret wrapper types; header and cookie auth | None |
| ACC-124 | Browser sessions that page scripts cannot steal | A cross-site scripting bug cannot lift the session | Jellyfin #5415 flagged tokens in browser local storage; Immich had a one-click takeover through XSS in 2026 | Medium | R1 | The web client's session lives in an HttpOnly, same-site cookie behind a strict content security policy | Cookie sessions; security headers | None |
| ACC-125 | Uploads cannot reach the filesystem by name | Uploaded artwork, lyrics and subtitles cannot attack the server | Jellyfin CVE-2026-35031 (CVSS 9.9) and SVG advisories; Immich SVG upload to remote code execution (Sept 2026) | Medium | R1 | Uploads are stored by content hash and parsed in the core with typed errors, and SVG is never rasterised in the server process | Content-addressed store; upload parsers | Upload dialogs |
| ACC-126 | An advisory for every fix | Owners know what to patch and why | Jellyfin, Navidrome and Immich publish GitHub advisories; some Plex CVEs reached users through the press first | Medium | R1 | Parity with Jellyfin and Navidrome, plus a security.txt and a written threat model before the first release | Advisory process | Project site; release notes |
| ACC-127 | Known-advisory banner | See ADM-054, which owns this feature. Accounts specifics: the check is opt-in and sends no identifiers. | Plex emailed owners about CVE-2025-34158; Jellyfin update-notice requests have 51 and 50 votes | Medium: Censys saw about 314,000 vulnerable Plex servers weeks after the fix | R1 | See ADM-054. | None beyond ADM-054. | Admin dashboard banner |
| ACC-128 | Nobody can switch your server off remotely | The project has no kill switch | Emby used one in 2023; Plex depends on plex.tv for sign-in; Jellyfin none | Low | R1 | No control channel from the project to servers exists, and the update check is read-only and optional | None | None (published promise) |

### Compatibility adapters

| ID | Feature | What the user gets | Rivals today | Demand | Release | How Gunmetal does it better | Server needs | UI surfaces |
|---|---|---|---|---|---|---|---|---|
| ACC-129 | App passwords for third-party apps | Symfonium or Finamp signs in without the main password | OpenSubsonic API-key extension; Navidrome stores reversible passwords under a shared key | Medium: Navidrome's Subsonic sign-in bypasses in 2023 and 2025 | R2 | Each app gets its own revocable credential scoped to reading and playing. Older Subsonic clients get a random per-app password, so a leak exposes one app Owns per-app credentials; INT-024 points here. Needed when the OpenSubsonic adapter ships (R2). | App-password store; scope mapping | Account > Apps; app setup QR |
| ACC-130 | Adapters off by default, under the same rules | Compatibility never reopens holes the native API closed | Navidrome published 19 advisories in Sept 2026; Jellyfin 12.0 turned legacy authorization off by default | Medium | R2 | Adapters are enabled separately and sit behind the same policy layer, limiter and cross-user tests, and their tokens expire (Navidrome's Jellyfin-API tokens never do) | Adapter routes in the route table; scope mapping | Admin > Compatibility |
| ACC-131 | Quick Connect through the Jellyfin adapter | Existing Jellyfin TV apps, such as the Roku app, sign in with a code | Navidrome added it to its Jellyfin API in 0.64.1 | Low | Later | Backed by the same pairing flow as ACC-061 | Pairing mapped onto the Jellyfin API | Approval prompt |

## Differentiators

1. **Remote access that is free, private and needs no setup (ACC-096,
   ACC-080, ACC-083).** Plex now charges for remote video and caps its relay
   at 2 Mbps; Jellyfin leaves families to VPNs and reverse proxies. An invite
   that carries the server's address and connects over iroh, with no open
   port, no account and no fee, removes the main reason people pay Plex and
   the main reason Jellyfin is hard to share. It arrives in R2 with the
   native clients, and its reach depends on the relay decision below.
2. **Modern sign-in on every client, built in (ACC-050, ACC-053, ACC-057,
   ACC-061).** Single sign-on (1,191 votes) and two-factor (1,103) are
   Jellyfin's most-voted security requests and neither is native; Emby has
   had a two-factor request open since 2018; no media server offers
   passkeys; and Jellyfin's SSO plugin was archived after only ever working
   on the web. Passkeys by default, two-factor codes, OIDC in the web client
   and native apps, and QR pairing for TVs is a combination nobody ships.
3. **Revocation that works and authorization that is proven (ACC-069,
   ACC-073, ACC-120 to ACC-122).** The rivals' 2026 advisories are mostly
   authorization mistakes: stale share URLs at Navidrome, unauthenticated
   endpoints and IDORs at Jellyfin. Session-bound, short-lived URLs and a
   cross-user test matrix that fails the build let Gunmetal promise
   something the others cannot: when an owner stops or revokes something,
   it stops. The claim is only worth making with the advisory process in
   ACC-126 behind it.
4. **Households with real profiles and children's controls enforced at sync,
   music included (ACC-016 to ACC-020, ACC-024, ACC-030).** Jellyfin has no
   household groups or profile PINs and has only started on account
   switching (202, 62 and 154 votes), and
   Plex gates parts of its controls behind Plex Pass. Filtering when the
   device's library is built means a restricted item never reaches a device
   only the child uses; on a shared device it is hidden rather than absent
   (ACC-030). The profile data model ships in R1 (ACC-017); the household,
   the picker and the parental rows, including the explicit-music filter,
   arrive in R2.
5. **Sharing music the way streaming apps do (ACC-086, ACC-091; R2).** Temporary
   share links are a 580-vote Jellyfin request, Navidrome shows the idea
   works for music, and shared playlists are what people leaving Spotify say
   they miss. Gunmetal's links expire, can carry a password and a download
   switch, and stop working the moment they are revoked.
6. **Privacy a user can see (ACC-113 to ACC-118).** Plex's 2023 viewing leak
   and Plexamp's telemetry with no off switch cost real trust, and privacy is
   the second reason people self-host. No telemetry and no feed are cheap;
   private listening (R1) and a "what your admin can see" page (R2) are
   things no media server we checked documents (unverified).

## Deliberately not doing

- **Passwordless sign-in on the local network (ACC-066).** It was the root of
  the 2023 Emby compromise. Household devices give the same convenience with
  a device key.
- **Social sign-in buttons and any central Gunmetal account (ACC-067).**
  They would make the project an identity provider and a breach target, as
  plex.tv has been twice. Owners who want a large provider can configure it
  as an ordinary OIDC provider.
- **Automatic port forwarding (ACC-106).** Opening ports is the exposure
  that made the Emby attack possible, and iroh does not need it.
- **A vendor kill switch (ACC-128).** Emby's 2023 shutdown of compromised
  servers stopped credential theft, but it also showed that the vendor could
  disable servers. Gunmetal will not build that channel.
- **Paid tiers for any feature in this area.** Remote playback, downloads,
  parental controls, two-factor, single sign-on, stream control and webhooks
  are all part of the AGPL build. Plex's paywalls are the top reason people
  leave it.
- **Vendor caps on users or devices.** Emby limits a household to 30
  devices and Plex Home to 15 members. Any limit in Gunmetal is the owner's
  choice (ACC-076).
- **Social features: public profiles, reviews, discussions or friend
  activity.** Plex's experience with these was poor, and nothing about a
  person's listening leaves the server unless they opt in (ACC-114).
- **Telemetry on by default (ACC-113).** Diagnostics are opt-in and show
  what they send.
- **Accepting the account password in compatibility adapters.** Subsonic's
  token-and-salt scheme forces the server to keep a recoverable password.
  Adapters get app passwords and keys only (ACC-129).
- **Fetching avatar or profile-picture URLs from identity providers.**
  Immich's 2026 SSRF came through exactly that path.
- **Controls for a vendor content catalogue.** Gunmetal has no catalogue of
  its own, so Plex's online-content switches have no equivalent here.

## Dependencies and risks

- **Blocking: ADR 3, durable user state.** ADR 1 (decision 5) names watch
  history as the only irreplaceable data, but most rows in this map that
  write user data (playlists, loves, ratings, corrections, layouts, shares,
  identities, IDs, settings) need it to survive a cache rebuild. ADR 3 must
  be accepted before any server work that stores user data. It defines the
  two durable stores every map now names the same way: **the user log** and
  **the identity store** (see the [feature map README](README.md)).
- **Identity is not a cache.** ADR 1 calls SQLite a rebuildable cache and
  names watch history as the only irreplaceable data. Users, passkey public
  keys, device keys, policies, invitations, shares, playlists and
  preferences cannot be rebuilt from files. ACC-013 needs a new architecture
  record before the server's data model is written, and the operations and
  music maps raise the same point.
- **Passkeys need a secure context.** WebAuthn only accepts a domain as the
  relying party and only runs over HTTPS or on localhost. A web client
  reached at `http://192.168.x.x` in R1 cannot register a passkey, and
  browser cryptography APIs are also limited to secure contexts
  (unverified). R1 therefore needs ACC-098 and a fallback (ACC-052), and the
  first-run design depends on this. Platform passkey sync on iOS and Android
  may also expect an associated web domain (unverified).
- **R1 is web-only.** Device keys, TV pairing, invite landing pages, iroh
  and offline grants all wait for the native clients in R2. Until then,
  remote use means the owner's own proxy or VPN, which is exactly what
  Jellyfin asks of people today. The R1 list in this map is also long; see
  the scope decision below.
- **iroh in practice.** Browsers on iroh are relay-only, n0's public relays
  are for testing, and behaviour on mobile networks, in mobile background
  modes and on TV platforms is unproven for media at video bitrates
  (unverified). Someone has to run relays, and a project relay would see
  connection metadata.
- **Shared rule language.** ACC-028 and the presets reuse the rule language
  the music and discovery maps define for smart playlists and rows. If that
  language slips, restrictions fall back to fixed fields.
- **Decision engine inputs.** ACC-043, ACC-107 to ACC-109 and ACC-111 need
  the playback decision engine in the core to accept the user's policy and
  a live upload budget, and to report reasons in plain words.
- **Explicit-content tags.** Which tags in FLAC, MP3, MP4 and Ogg reliably
  mark explicit content is unverified. ACC-024 must work with manual marking
  if the tags turn out to be sparse.
- **Rating data.** ACC-026 and ACC-027 need a source of rating systems and
  equivalences whose licence has not been checked (unverified).
- **Adapters double the surface.** Navidrome's 19 advisories in a month came
  largely from its Subsonic-compatible API and share endpoints. ACC-129 and
  ACC-130 must exist before any adapter ships.
- **OIDC is easy to get subtly wrong.** Immich shipped TLS verification
  switched off, an open redirect and an SSRF in its OIDC path in 2026. ACC-057
  needs its own test suite built from those advisories.
- **Offline grants weaken revocation.** A stolen phone keeps its downloads
  until its grant expires or the device next contacts the server (ACC-045).
  The default lifetime is a policy choice.
- **Children's controls are security.** PINs are guessable and local caches
  can be read. ACC-030 only holds if the sync payload, artwork and signed
  URLs are filtered on the server, and if cross-profile tests cover every
  route.
- **Free remote access invites account sharing.** With no per-viewer fee,
  credentials will travel beyond friends and family. ACC-075 and ACC-068
  need to work from the first release that has remote access.
- **Advisory volume.** Even small projects received 10 to 20 advisories each
  in 2026. Marketing Gunmetal as the secure option draws scrutiny, so the
  advisory process (ACC-126) needs people and a response time before the
  claim is made.
- **No central push.** Push notifications on iOS need Apple's service, which
  sits awkwardly with having no central account. New-device alerts and
  messages (ACC-071, ACC-074) are therefore in-app first.
- **Sibling maps.** Importers (ACC-084), the update manifest (ACC-127), the
  diagnostics preview (ACC-113) and backups (ACC-013) come from the
  operations map; per-user plugin secrets (ACC-119) come from the plugin host
  in the ecosystem map; guest watch-together (ACC-093) needs the video map's
  control channel.

## Open decisions for the project owner

1. **Do passwords exist at all?** Passkey-only is cleaner, but a browser on a
   plain-HTTP LAN address cannot use passkeys. *Recommendation:* allow
   passwords as a fallback in R1, with optional two-factor, and let an owner
   turn them off per user or server-wide once passkeys are enrolled.
2. **Should gunmetal.tv issue per-server HTTPS names, as plex.direct does?**
   That would make passkeys and direct browser connections easy, but it is a
   central service even if it holds no accounts. *Recommendation:* not in
   R1. Ship supplied certificates and reverse-proxy support (ACC-097,
   ACC-098), and decide on name issuance in its own record once the native
   clients exist.
3. **Will the project run default iroh relays, and who pays?**
   *Recommendation:* make self-hosted relays first-class in R2 (ACC-100),
   and run one modest project relay only if it is funded, with a published
   policy on what metadata it logs and for how long.
4. **A new architecture record for identity and user data.**
   *Recommendation:* yes, before any server code that stores users. It
   should cover identities, credentials, policies, invitations, shares,
   playlists and preferences, alongside history.
5. **What can admins see by default (ACC-116)?** *Recommendation:* live
   sessions and totals for adult profiles; full history only for managed
   child profiles, or for adults who opt in. Show the result on every
   person's "what your admin can see" page.
6. **Is private listening on by default?** *Recommendation:* off by default,
   because history feeds statistics and recommendations, but one tap away
   from the player and clearly indicated while on.
7. **No telemetry, or opt-in anonymous counts?** *Recommendation:* none in
   the server. Offer opt-in crash and diagnostic reports that preview their
   contents, and get performance numbers from the project's own benchmarks.
8. **LDAP and reverse-proxy header sign-in.** *Recommendation:* OIDC only
   in R1. Add header sign-in later, off by default, with a trusted-source
   list. Add LDAP only if demand persists once OIDC works, since most
   household identity providers can sit in front of a directory
   (unverified per product).
9. **Share links for films and episodes (ACC-092).** These carry bandwidth
   cost and copyright exposure (unverified legal analysis).
   *Recommendation:* ship in R2 behind an owner switch that is off by
   default, with a short maximum expiry and a per-link bandwidth cap. Get
   legal advice before marketing the feature.
10. **Remote access for new accounts.** The research asks whether new
    accounts should be denied remote access until approved.
    *Recommendation:* make remote access part of each invite's policy,
    chosen when the invite is made, so there is no separate approval step and
    household members do not get a worse default than friends.
11. **Is the living-room TV a household device or a personal one?**
    *Recommendation:* a household device with a profile picker, enrolled
    with approval from any adult in the household (ACC-021).
12. **When do compatibility adapters ship?** R1 has no native mobile app, so
    the OpenSubsonic adapter would be the only way to use a phone app such
    as Symfonium, but adapters double the authorization surface.
    *Decided in the feature map README:* the OpenSubsonic adapter ships in
    R2, off by default, with per-app keys (ACC-129) and the same cross-user
    tests; legacy password sign-in (INT-088) is Later.
13. **Default treatment of untagged music in children's profiles
    (ACC-024, ACC-025).** Hiding everything without an explicit or clean
    flag would empty most children's libraries. *Recommendation:* allow
    untagged music by default, show how many items that covers, and make
    the stricter choice one click away.
14. **Default lifetime of offline grants (ACC-045).** *Recommendation:*
    renew on every contact and expire after a few weeks without contact,
    with the owner able to shorten it.
15. **Scope of R1 for this area.** Resolved in the feature map README: the
    household and parental rows, share links, collaborative playlists and
    ACC-054, ACC-058, ACC-071, ACC-074, ACC-075, ACC-081, ACC-115 and
    ACC-116 moved to R2. ACC-086 left the floor proposed here, because share
    links are an unauthenticated surface. ACC-053 stays in R1, because
    password plus two-factor is the R1 default wherever passkeys cannot work.
16. **One sign-in across several Gunmetal servers (ACC-133).** Plex's
    plex.tv identity lets a friend see every shared server after one sign-in.
    *Recommendation:* design it as a device key trusted by several servers
    (for example, vouched for by the friend's home server), review it in its
    own record, and keep it Later.
