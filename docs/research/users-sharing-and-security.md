# Users, sharing and security

Research date: 2026-10-02. Web tools were available and used. The general
web-search budget for the session ran out part-way through, so the later
findings come from fetching primary sources directly: vendor documentation,
GitHub security advisory pages, the Jellyfin feature board and the Plex
forum. Anything not confirmed from a source is marked "(unverified)".

## Scope

This document covers everything about *who* can use a Gunmetal server and
*how safely*:

- accounts, households and profiles, including managed (child) profiles;
- parental controls and content ratings;
- per-library, per-device and per-item permissions;
- sharing with friends and family, invitations and public share links;
- sign-in methods: passwords, two-factor, passkeys, single sign-on, LDAP,
  reverse-proxy authentication and TV code sign-in;
- device and session management;
- remote access, both built into rivals and general-purpose tools such as
  Tailscale, Cloudflare Tunnel and iroh;
- bandwidth and stream limits per user;
- privacy and telemetry;
- the security track record of each rival.

The main rivals are Plex, Jellyfin and Emby. Navidrome is the reference for a
self-hosted music server, and Immich for a modern self-hosted app that got
sign-in and sharing right. Tailscale, Cloudflare Tunnel and iroh are the
references for remote access, since iroh is the transport that
[record 1](../adr/0001-architecture.md) has already chosen.

Two architecture records constrain the design ideas below. Record 1 commits
to random IDs, authorization on every object, short-lived signed stream URLs,
passkeys and OIDC, device-bound keys, no central account, remote access over
iroh and a sandboxed FFmpeg. Record 2 commits to an OpenSubsonic adapter and
to scrobbling only through plugins that hold explicit network grants.

## Feature inventory

In the tables, "Yes" means the feature is built in and free. A rival
appearing under "Best in class" is honest credit; it does not mean the
feature is good enough.

### Accounts and identity

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Server usable with no vendor account | Install, sign in and play without registering with a company | No. The server is claimed with a plex.tv account and sign-in goes through plex.tv. Users report local play failing when plex.tv or the home internet is down (forum request open since 2015) | Yes, all accounts are local to the server | Yes. Emby Connect is optional | Jellyfin and Emby, because nothing outside the house can lock you out | Record 1 already commits Gunmetal to no central account |
| One identity across many servers | A friend signs in once and sees every server shared with them | Yes. The plex.tv account is the identity everywhere | No. Every server has its own accounts and address | Partly. Emby Connect links local users on several servers to one Emby account by email | Plex. Invites land in the app the friend already has, with no address to type | Hard to match without a central account; see Risks |
| Administrator role | Someone can manage the server | One owner per server, plus the Home admin (unverified) | Any user can be made an administrator with a single switch, and admins have every power | Any user can be granted admin access to the dashboard | Jellyfin and Emby allow several admins | None of the three offers a scoped admin, such as one who manages users but not plugins (unverified) |
| Delegated management rights | Trusted users can curate without being full admins | Not found (unverified) | Separate switches for collection, subtitle, lyric and Live TV recording management | Not found (unverified) | Jellyfin, for its fine-grained switches | Jellyfin's subtitle-upload right was one route into a critical 2026 remote code execution flaw |
| Disable an account without deleting it | Pause someone's access and keep their history | Remove the share (unverified) | Yes, with "Disable this user" | Not checked (unverified) | Jellyfin | |
| Hide users from the sign-in screen | Usernames are not shown to anyone who reaches the page | Not applicable; sign-in is at plex.tv | Yes, and new users are hidden by default in the current user policy | User picker with a hide option (unverified) | Jellyfin, for hiding by default | A visible user list helps attackers guess usernames |
| Self-service sign-up with approval | A friend requests an account and the admin approves it | Friends create their own plex.tv account | No. Feature request #494 has 227 votes | No (unverified) | Plex, by way of its central account | Wizarr and jfa-go fill this gap for Jellyfin |
| Defaults for new users | New accounts start with the household's normal settings | Not applicable (unverified) | No. Request #363 has 130 votes | Settings can be imported from another user | Emby, for copying one user's settings to another | |
| Account deletion with a grace period | A mistaken deletion can be undone | No (unverified) | No (unverified) | No (unverified) | Immich, which keeps a deleted user for 7 days by default before purging them | |
| Export of a user's own data | Users can take their history and playlists away | Through a privacy request (unverified) | No built-in export (unverified) | No (unverified) | None verified | Record 1 makes watch history an exportable append-only log |
| Multiple accounts on one device | Two people share a tablet without signing out | Yes, through Home user switching | No. Request #2353 has 154 votes and is marked as started | Some apps (unverified) | Plex | |

### Households, profiles and kids

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Household group | People who live together are grouped, separate from friends | Plex Home, up to 15 members including the admin | No. Request #493 for groups or master accounts with sub-users has 202 votes | No (unverified) | Plex | Plex says Home is meant for people who live together |
| Managed child profiles | A child gets their own profile with no email or password | Yes. Managed accounts have no username, email or password and are reached by switching from an adult's sign-in | No. Every user is a full account with a password | No (unverified) | Plex | |
| Profile PIN | A child cannot switch into a parent's profile | Yes, Home users can have a PIN (unverified) | No. Request #2549 has 62 votes | Not checked (unverified) | Plex | |
| Restriction presets | One click gives an age-appropriate profile | Yes: Younger Kid, Older Kid, Teen and None. The support article implies presets are free, but the plans page lists parental controls as a Plex Pass feature, so current status is unclear | No presets. Request #3513 for reusable profiles has 8 votes | No (unverified) | Plex | |
| Maximum content rating | Content above a rating is hidden | Presets are free; custom rating rules need Plex Pass | Yes, free, with a separate sub-rating ceiling in the current user policy | Yes, free | Jellyfin and Emby, for being free and granular | |
| Block unrated items | Items with no rating are hidden from kids | Not documented (unverified) | Yes, chosen per media type | Yes | Jellyfin, for choosing per media type | Unrated home videos and music are common, so this matters |
| Allow-list and block-list tags | "Only show what I tagged for the kids", or "hide these" | Labels, but custom label rules need Plex Pass | Both. Allow-tags shipped after request #235 (86 votes) | Both | Jellyfin and Emby, for being free | Jellyfin request #4136 asks for tag autocomplete because a typo silently breaks the filter |
| Per-item exception to a rating limit | Let a child watch one PG-13 film without raising the whole limit | No | No. Requests #4035 (7 votes) and #2737 (26 votes) | No (unverified) | None | Clear gap |
| Access schedule | A user can only sign in during set hours | No (unverified) | Yes, by day and hour | Yes, by day and hour, but not on admin accounts | Jellyfin and Emby | Emby's error message for a schedule block was confusing enough to start a forum thread |
| Daily screen-time allowance | "Two hours a day", not only "between 4 and 7" | No | No. Request #699 has 29 votes | No (unverified) | None | Gap |
| Controls on the vendor's own online content | Kids cannot reach the free streaming catalogue | Yes. Plex online content can be on for all, off for all, or off for managed users | Live TV access switch per user | Not checked (unverified) | Plex | Not relevant to Gunmetal, which has no catalogue of its own |
| Restrictions also hide folder view | Folder browsing cannot be used to get round the restrictions | Folder view is removed while restrictions are on, because folder names cannot be filtered | Not checked (unverified) | Not checked (unverified) | Plex, for closing the hole explicitly | A reminder that every browse path must apply the same filter |
| Rating systems from many countries | Ratings make sense wherever you live | Ratings by country (unverified) | Many country systems; 12.0 fixed the Swedish, Italian, Norwegian and Greek ones | Not checked (unverified) | Jellyfin, for breadth | Mapping between systems is a constant source of bugs |
| Explicit-lyrics filter for music | Kids' profiles skip explicit tracks | Not found (unverified) | Only by tags or ratings (unverified) | Not found (unverified) | None verified | A music-first gap; see the ideas section |
| Parent can see what a child played | Viewing history per profile | Yes, through the dashboard (unverified) | Activity log and playback history (unverified) | Yes (unverified) | Not compared | |

### Permissions

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Access per library | Each person sees only the libraries chosen for them | Yes, per share | Yes | Yes | All three | Navidrome added per-library access in 0.58.0, then fixed many places that ignored it in 2026 |
| Allow-list of devices per user | An account works only on approved devices | No (unverified) | Yes. 12.1 fixed revoking a device without ending its open sessions | Not checked (unverified) | Jellyfin | The 12.1 fix shows how easy revocation is to get wrong |
| Remote access on or off per user | Grandparents only on the home network, friends remote | Home or friend status decides this (unverified) | Yes, a switch per user | Not checked (unverified) | Jellyfin | |
| Playback-mode rights | Choose whether a user may trigger transcodes or remuxes | No per-user control (unverified) | Separate switches for audio transcoding, video transcoding, remuxing and forced remote transcoding | Similar (unverified) | Jellyfin | Maps directly onto Gunmetal's direct, remux and transcode ladder |
| Download for offline use | Allow or deny offline copies per person | Downloads need Plex Pass; per-share download switch (unverified) | Yes, free, per user | Offline downloads need Emby Premiere | Jellyfin | |
| Delete from library | Trusted users can remove media | Server-wide setting (unverified) | Per user, and per library folder | Not checked (unverified) | Jellyfin | |
| Remote control of other people's sessions | Cast to, or control, someone else's player | Not checked (unverified) | Per user; off by default for other users, on for shared devices | Not checked (unverified) | Jellyfin, for defaulting to off | Jellyfin published a broken-access-control advisory for this API in September 2026 |
| Watch-together rights | Who may create or join group playback | Watch Together (current status unverified) | SyncPlay access level per user | Not checked (unverified) | Jellyfin | |
| Live TV and DVR rights | Who may watch and who may schedule recordings | Plex Pass DVR (unverified detail) | Separate switches for watching and managing recordings | DVR needs Premiere | Jellyfin | Relevant once Gunmetal's live TV module lands |
| Scoped API keys | A script or third-party app gets only the rights it needs | Tokens carry the whole account (unverified) | Admin-created keys with no scopes (unverified) | No scopes (unverified) | Immich, which gives each key a list of permissions | Immich's January 2026 flaw let a key raise its own permissions (CVE-2026-23896) |
| Authorization on every object | User A cannot fetch user B's items by guessing IDs | Not checked (unverified) | No. Issue #5415 (2021) found unauthenticated stream, image and subtitle endpoints; item IDs are said to be derived from file paths (unverified) | Not checked (unverified) | None proven | Record 1 requires random IDs and authorization on every object |

### Sharing with friends and family (video servers)

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Share libraries outside the home | Friends get chosen libraries on their own devices | Yes, as friends through plex.tv, with per-library choice | Make them an account and send them your address | Make an account and link it to Emby Connect | Plex | Plex's best feature for families spread across houses |
| Invite by email or link | The admin sends one message and the friend is in | Invite by email or username (details unverified) | No. Wizarr (3.2k stars) and jfa-go are add-ons | Emby Connect email linking | Plex for ease; Wizarr among self-hosted tools | |
| Invite expiry and time-limited membership | Links stop working and access can end on a date | No (unverified) | Wizarr only | No (unverified) | Wizarr | |
| Onboarding for the invitee | "Install this app, then tap here" | The friend's existing Plex app | Wizarr only | Not checked (unverified) | Wizarr, for built-in onboarding guides | Most of the friction in sharing is the invitee's first five minutes |
| Who pays for a friend's remote viewing | Friends can watch from their own homes | Since 2025-04-29 the owner needs Plex Pass, or each friend needs Plex Pass or a Remote Watch Pass. Music and photos are exempt | Free | Free | Jellyfin and Emby | The Remote Watch Pass is listed at $2.99 a month or $29.99 a year after an introductory price that ran to June 2026 |
| Owner sees friends' activity | The admin sees who is watching what | Yes, and Tautulli is widely used for history (unverified) | Dashboard sessions and activity | Yes | Not compared | Friends are rarely told how much the owner can see |
| Watch-together invite link | Send a link and watch in sync | Watch Together (unverified) | SyncPlay groups exist, but a join link is request #971 with 267 votes | Not checked (unverified) | None | |
| Temporary file or item link | Share one film or episode with someone who has no account | No (unverified) | No. Request #72 has 580 votes | No (unverified) | None among the video servers | Navidrome and Immich do this; see the next table |

### Sharing references from music and photo servers

| Feature | What the user gets | Navidrome | Immich | Jellyfin | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Public link without an account | Send an album or playlist to anyone | Yes, for tracks, albums, artists and playlists; on by default since 0.63.0 | Yes, as random unguessable URLs | No | Immich | |
| Link password | Only people with the password can open it | Not documented | Yes | No | Immich | |
| Link expiry | Links die on their own | Yes, one year by default and configurable | Yes | No | Both | |
| Download switch on a link | Listen only, or allow download | Downloads allowed (whether a switch exists is unverified) | Yes, chosen per link | No | Immich | |
| Revoking a link stops playback at once | A deleted share cannot keep streaming | Fixed in September 2026; shared stream URLs had kept working after expiry or deletion | Not checked (unverified) | Not applicable | None proven | Signed URLs must re-check the share on every request |
| Contribute to a shared collection | Friends add to a shared album or playlist | Public playlists (collaboration unverified) | Shared albums with editor or viewer roles, and upload through a public link | Playlist sharing (unverified) | Immich | For music, the equivalent is a collaborative playlist |
| Share a whole library with a partner | A spouse sees everything without per-album shares | No | Yes, partner sharing | Not applicable | Immich | The music equivalent is a household library |
| Preview cards in chat apps | A link shows artwork and a title in Discord | Yes, through page meta-tags | Not checked (unverified) | No | Navidrome | Previews leak titles to the chat platform, so this should be a choice |
| Only owners can manage their shares | Users cannot read or delete each other's links | Missing ownership checks were fixed in September 2026 | Not checked (unverified) | Not applicable | None proven | Two Navidrome advisories (one High, one Moderate) came from share creation trusting client input |

### Sign-in methods

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Username and password | The basic sign-in | Yes, held at plex.tv, which was breached in 2022 and again in 2025 | Yes, held on the server | Yes, held on the server | Not ranked | Passwords held centrally are a single target for every Plex user |
| Two-factor codes (TOTP) | A code from an authenticator app | Yes | Not built in. Request #26 has 1,103 votes and is marked Planned; third-party plugins exist | No. Requested since January 2018; in May 2025 staff said it is still on the list | Plex | Emby staff cite the cost of making every app aware of it |
| Passkeys (WebAuthn) | Phishing-resistant sign-in with no password | No. Forum request from June 2023 has 27 votes and no staff reply | No. Request #4174 has 5 votes; a third-party plugin claims passkeys (unverified) | No | None among media servers | Record 1 makes passkeys the default for Gunmetal |
| Single sign-on (OIDC) | Use the household identity provider | Not for the server; Google or Apple sign-in to plex.tv | Plugin only, web UI only. The main SSO plugin (1.5k stars) was archived on 2026-05-12. Request #230 has 1,191 votes and is marked Planned | Not found (unverified) | Immich: built in, with auto-registration, role and quota claims, and mobile redirects | |
| LDAP | Use an existing directory | No | Plugin (details unverified) | Plugin (unverified) | Jellyfin | Navidrome LDAP issue #141 has 148 reactions and has been open since 2020 |
| Reverse-proxy header sign-in | Authelia or Authentik in front of the server | No | No (unverified) | No (unverified) | Navidrome, with an explicit list of trusted sources | Dangerous if the proxy lets clients set the header |
| Social sign-in | "Continue with Google or Apple" | Yes | No | No | Plex, for convenience | Depends on a central account |
| TV sign-in with a code | Type a short code on your phone instead of a password on the TV | Device linking code (unverified) | Quick Connect: a 6-digit code since 10.7, enabled by the admin. A QR version is request #2642, 225 votes, Planned | PIN process through Emby Connect | Jellyfin, for being local and free | Navidrome added Quick Connect for its Jellyfin-compatible API in 0.64.1 |
| Passwordless sign-in on the local network | No password at home | Networks can be set as allowed without auth (unverified) | Not found (unverified) | Yes, and this was the root of the 2023 compromise | None. This is an anti-feature | Header spoofing made remote attackers look local |
| Brute-force protection | Guessing passwords is slow or impossible | Handled at plex.tv (unverified) | Lockout per user after a set number of failures, but unlimited by default | Not checked (unverified) | Navidrome's sliding-window limiter, though its Subsonic path was only covered in September 2026 | Every sign-in surface, including adapters, needs the same limit |
| Password reset | Recover a forgotten password | Email reset at plex.tv | Pluggable reset provider; the default asks for a file on the server (unverified) | Similar (unverified) | Plex for convenience; physical access to the host is the self-hosted norm | Immich lets an admin issue a temporary password that must be changed |
| Sign out everywhere on password change | A leaked password stops working on every device | Yes, as an option during reset | Not checked (unverified) | Not checked (unverified) | Plex | |
| Credentials for third-party apps | Music apps sign in without the main password | Not applicable | Admin API keys (unverified) | Not checked (unverified) | OpenSubsonic's API-key extension, which recommends dropping token and salt sign-in | Subsonic token and salt sign-in forces the server to keep a reversible copy of the password; Navidrome encrypts it with a shared key |
| Removing legacy sign-in schemes | Old, weaker sign-in paths are switched off | Not checked (unverified) | 12.0 (September 2026) disables legacy authorization by default, with a switch to keep it | Not checked (unverified) | Jellyfin | |

### Devices and sessions

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| List of signed-in devices | See every phone, TV and browser on the account | Yes, at plex.tv (unverified detail) | Admin devices page (unverified detail) | Yes (unverified) | Immich lets each user see their own authorized devices (unverified) | Users, not only admins, should see their own devices |
| Revoke one device | Remove a lost phone | Yes | Yes, with the 12.1 fix for open sessions | Yes (unverified) | Plex | |
| Sign out of all sessions | One button after a scare | Yes, at plex.tv/security, as recommended after the 2025 breach | Not checked (unverified) | Not checked (unverified) | Plex | |
| Live sessions dashboard | See who is playing what, and how | Yes | Yes | Yes | All three | |
| Stop a stream with a message | End a stream and tell the viewer why | Plex Pass (unverified) | Playback can be stopped through remote control; a dedicated kill-stream and idle-stream option is request #301 with 402 votes | Not checked (unverified) | Plex | |
| Simultaneous streams per user | Stop one account being shared across a whole street | Not built in (unverified) | Counts every session, including browsing, so the limit misfires | Per user; enforcement needs Emby Premiere (unverified) | Emby, for counting streams | Infuse users report Jellyfin treating each open device as a stream |
| Vendor limit on devices | No cap on how many devices a household uses | No device cap found (unverified) | None | Emby Premiere allows 30 devices per household | Jellyfin | Not something Gunmetal should copy |
| Sign-in and admin audit log | Who signed in, from where, and what admins changed | Not checked (unverified) | Activity log (unverified detail) | Not checked (unverified) | None verified as complete | |
| New-device alert | "A new TV signed in to your account" | Not checked (unverified) | No (unverified) | No (unverified) | None verified | Cheap to build and useful against shared credentials |

### Remote access built into the video servers

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Automatic port forwarding | Remote access with no router setup | UPnP or NAT-PMP, with manual port 32400 as a fallback | Removed (unverified); a reverse proxy or port forward is expected | UPnP (unverified) | Plex | Opening ports is itself the risk that sank Emby servers in 2023 |
| Vendor relay when ports cannot open | Works behind carrier-grade NAT | Plex Relay, end-to-end encrypted and tunnelled, capped at 2 Mbps for every subscriber | None. Request #3384 for a broker has 2 votes | None. Emby Connect only looks up the address | Plex, though 2 Mbps is too little for good video | |
| Automatic HTTPS certificates | No certificate work | Yes, through Plex-issued names (unverified detail) | Bring your own, usually through a reverse proxy | Not checked (unverified) | Plex | A central service that issues names is the price |
| Server discovery | No address to type | Through the plex.tv account | Local auto-discovery only | Emby Connect | Plex | |
| Cost of remote viewing | | Plex Pass or Remote Watch Pass for video since 2025-04-29; TV apps were enforced later (Roku first, others through 2026) | Free | Free | Jellyfin and Emby | |
| VPN and container quirks | Local devices are treated as local | VPNs, containers and privacy settings can make local devices count as remote, which then triggers the paywall | Not applicable | Not applicable | Not ranked | Tailscale users of Plex hit this |

### General-purpose remote-access tools

| Feature | What the user gets | Tailscale | Cloudflare Tunnel | iroh | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| No open ports | Nothing on the router changes | Yes | Yes | Yes | All three | |
| Who can read the traffic | Only you and the viewer | WireGuard end to end; Funnel ends TLS on your own device | Cloudflare ends TLS at its edge (unverified) | End to end; relays cannot read it | Tailscale and iroh | |
| What a friend must install | Ease for non-technical family | The Tailscale client and an account. To accept a shared machine they must be an owner or admin of their own tailnet | Nothing, if a public hostname is used | Nothing extra, if the media app embeds iroh | iroh inside the app, which is invisible to the friend | This is why record 1 chose iroh |
| Terms for streaming media | Allowed to push video through it | Funnel has bandwidth limits that cannot be configured and are not published | The CDN terms reserve the right to limit video served without paid products; Tunnel is not named in those terms (unverified how they apply) | Free public relays are rate-limited and meant for testing only | Self-hosted iroh relay | |
| Direct versus relayed | Speed and cost | Peer to peer with relay fallback (share relayed unverified) | Always through Cloudflare | Direct in about 90% of cases; browsers are relay-only | iroh for native apps | Relay-only browsers matter a lot for a web client |
| Price | | Free Personal plan for up to 6 users with unlimited user devices; Funnel and node sharing on all plans | Free tier (unverified) | Open source; paid managed relays exist | Tailscale or a self-hosted iroh relay | |
| Self-hostable | No dependency on the vendor | Control plane through Headscale (unverified) | No | The relay is open source | iroh | |

### Bandwidth and stream limits

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Remote bitrate cap per user | A friend on a slow line cannot starve the house | Global only, and the setting needs Plex Pass. A per-user request from 2017 has 188 votes | Yes, per user | Per user (unverified) | Jellyfin | With direct play, a cap below the file's bitrate forces a transcode; see Risks |
| Total upload budget | Stay within the home connection's upload | An "Internet upload speed" setting (Plex Pass) | No. Requests #1504 (fair split) and #1012 | Not checked (unverified) | Plex | |
| Downloads give way to streams | Offline syncs do not stutter live playback | Yes, downloads scale back when streams need the upload | Not checked (unverified) | Not checked (unverified) | Plex | |
| Transcodes per user | One user cannot take the whole GPU | No; requested on the forum (count unverified) | Only through the transcode switches | Not checked (unverified) | None | |
| Smoothing traffic bursts | Remote streams do not cause lag spikes at home | Not checked (unverified) | No. Request #1185 (15 votes) | Not checked (unverified) | None | |
| Remote music quality | Lossless music away from home | Plexamp remote music is free of the paywall | A request (#3129) says a 2 Mbps music cap causes buffering (which client is unverified) | Not checked (unverified) | Plex (Plexamp) | Record 2 makes cheap Opus transcoding the fallback |

### Privacy and telemetry

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Telemetry by default | Nothing leaves the house unless the user chooses | Collects usage data with opt-outs (detail unverified; the privacy page refused automated access) | None. The site says it has no tracking or phone-home | Not checked (unverified) | Jellyfin | Navidrome sends anonymous statistics by default, with an opt-out, and keeps them for 30 days |
| Watch history shared with friends | Friends never see what you watched unless you choose | November 2023 "Week in Review" emails and Discover Together exposed viewing to friends and caused a large backlash; Plex says history is private by default | No social features | No (unverified) | Jellyfin, by having none | |
| Watch history held by the vendor | History stays on your server | Watch state can sync to plex.tv, depending on account settings | Local only | Local (unverified) | Jellyfin | Record 1 keeps history in a local, exportable log |
| Email preferences respected | Opt-outs stay opted out | Users reported unsubscribed preferences being switched back on | Not applicable | Not checked (unverified) | Not ranked | |
| Users can see what the admin can see | "Your server admin can see your history and devices" | No (unverified) | No (unverified) | No (unverified) | None | A gap Gunmetal can fill cheaply |
| Private listening session | Play something without it entering history | No (unverified) | No (unverified) | No (unverified) | None among servers (streaming apps have it, unverified) | A music-first feature |
| Third-party scrobbling | Last.fm and similar, chosen per user | Not checked (unverified) | Plugins | Not checked (unverified) | Navidrome links Last.fm per user, but its link callback had an unauthenticated flaw until September 2026 | Record 2 puts this behind plugins with network grants |

### Security posture

| Feature | What the user gets | Plex | Jellyfin | Emby | Best in class (who, and why) | Notes |
|---|---|---|---|---|---|---|
| Every endpoint authenticated by default | No anonymous access to streams, images or user lists | Not checked (unverified) | No. Issue #5415 listed unauthenticated stream, subtitle, image, user and system-info endpoints; many were fixed later, and 12.0 turned legacy authorization off | Not checked (unverified) | None proven | |
| Published advisories | Users can see what was fixed and when | Email to owners and press coverage | Public GitHub advisories (15 published in 2026 by late September) | Not checked (unverified) | Jellyfin, Navidrome and Immich, for transparency | Volume shows scrutiny as well as weakness |
| Transcoder isolation | A malicious file cannot take over the host | Not checked (unverified) | FFmpeg argument injection advisories in 2023, 2025 and 2026 | Not checked (unverified) | None | Record 1 sandboxes FFmpeg |
| Trusted-proxy handling | Forwarded-for headers cannot be forged to look local | Not checked (unverified) | IP-spoofing denial-of-service advisory, April 2025 | Header spoofing gave remote attackers admin access in 2023 | None | Navidrome's rate limiter was bypassed the same way in September 2026 |
| Handling uploaded images | Uploaded artwork cannot attack the server | Not checked (unverified) | SVG upload to admin escalation (2024); SVG memory exhaustion and SSRF (2026) | Not checked (unverified) | None | Immich had an SVG upload to remote code execution flaw in September 2026 |
| Vendor kill switch | The vendor can stop your server | The server depends on plex.tv for sign-in | None | Used in 2023 to stop compromised servers starting | Not ranked | Emby's move stopped credential theft, but it also shows the vendor can disable servers |

### Security track record (incident log)

This table lists the notable incidents found, newest first. It is a log, not
a list of features.

| Date | Product | What happened | Lesson for Gunmetal |
|---|---|---|---|
| 2026-09 | Navidrome | 19 advisories published between 11 and 21 September. They include unauthenticated brute force through the Subsonic API, share creation that trusted a client-supplied user ID, SQL injection, share endpoints with no ownership checks, shared stream URLs that kept working after deletion, a rate-limit bypass through forged forwarding headers, symlink file reads, and an unauthenticated Last.fm callback flaw. Fixed in releases from July to September 2026 (0.63.0 to 0.64.2) | Adapters and share links are where authorization goes wrong. Test every route against every other user |
| 2026-09 | Jellyfin | Advisories for path traversal in library-folder management (Critical), broken access control in the session remote-control API, an IDOR in playlist management, HLS segment path traversal, and SVG memory exhaustion and SSRF | Library management and session control need object-level checks |
| 2026-09 | Immich | Authenticated SVG upload reached ImageMagick and allowed remote code execution (High) | Never hand user-supplied images to a large C toolkit outside a sandbox |
| 2026-06/07 | Immich | One-click account takeover through XSS in the sign-in redirect (Critical), then an incomplete fix; OIDC fetches with TLS verification switched off; SSRF through the OAuth profile picture URL | OIDC and redirect handling need their own test suite |
| 2026-06 | Jellyfin | FFmpeg argument injection through a subtitle path (High); path traversal in client log upload; MKV attachment path traversal; XSS in user management | Never build file paths or command lines from metadata |
| 2026-04 | Jellyfin | Subtitle upload path traversal chained to remote code execution as root (CVE-2026-35031, CVSS 9.9). It needed admin rights or the upload-subtitles right. Also SSRF and file read through the Live TV M3U tuner and through stream arguments. Fixed in 10.11.7 | Delegated rights such as "manage subtitles" must not reach the filesystem by name |
| 2026-03 | Jellyfin | A GitHub Actions workflow in the iOS repository could run code from fork pull requests (CVE-2026-31852, CVSS 9.8). Users needed to do nothing | Supply chain: CI permissions are part of the attack surface |
| 2026-01 | Immich | An API key could raise its own permissions to full access (CVE-2026-23896). Fixed in 2.5.0 | Scoped keys need a no-escalation rule on update as well as on create |
| 2025-08/09 | Plex | CVE-2025-34158, rated at the top of the CVSS scale, affected server versions 1.41.7.x to 1.42.0.x and was fixed in 1.42.1. Censys still saw about 314,000 vulnerable instances on 2025-08-25. Separately, plex.tv was breached on 2025-08-23, exposing emails, usernames and hashed passwords, and Plex told users to reset their passwords | Self-hosted servers stay unpatched for a long time; central account stores are breach magnets |
| 2025-05 | Navidrome | SQL injection through a role parameter and a transcoding permission bypass (both Critical) | |
| 2025-04 | Jellyfin | FFmpeg argument injection (High) and IP-spoofing denial of service | |
| 2025-02 | Navidrome | Subsonic sign-in bypass with a non-existent username | Adapters need the same tests as the native API |
| 2024-12 | Navidrome | JWT secret stored in plain text in the database (High) | |
| 2024-09 | Navidrome, Jellyfin | Navidrome: several SQL injections and an ORM leak (Critical). Jellyfin: privilege escalation to admin through SVG upload | |
| 2024-04 | Navidrome | Parameter tampering (High) | |
| 2023-12 | Jellyfin | FFmpeg codec-parameter argument injection reachable through an unauthenticated stream endpoint (CVE-2023-49096), fixed in 10.8.13; a custom-FFmpeg-path remote code execution flaw | |
| 2023-12 | Navidrome | Sign-in bypass in the Subsonic endpoint (High) | |
| 2023-11 | Plex | "Week in Review" emails told friends what users had watched. The main forum thread reached 635 posts, 2,704 likes and 28,689 views | Privacy failures cost as much trust as breaches |
| 2023-05 | Emby | Header spoofing (CVE-2023-33193, CVSS 9.1) let remote attackers pass as local on servers that allowed passwordless local admin sign-in. About 1,200 servers were backdoored with a credential-stealing plugin. Emby shipped an update that stopped affected servers from starting. Fixed in 4.7.12 and beta 4.8.31 | Never offer passwordless local sign-in; never trust forwarding headers by default |
| 2023-03 | Plex | CVE-2020-5741 (authenticated remote code execution through Python deserialization in Windows servers before 1.19.3) was added to CISA's Known Exploited Vulnerabilities catalogue on 2023-03-10. It was widely reported as the way into a LastPass engineer's home computer (unverified; the reports could not be opened) | An admin-only flaw still matters when the admin's home network is the target |
| 2022-08 | Plex | plex.tv breach exposing emails, usernames and hashed passwords; a forced password reset | |
| 2021-03 | Jellyfin | Issue #5415 catalogued unauthenticated endpoints and the separation between authentication and user IDs | Authorization must be designed in, not added later |

## Pain points and unmet demand

1. **Plex now charges for remote video, and the price is rising.** Since
   2025-04-29, remote video playback from a personal server needs the owner
   or the viewer to pay
   ([Plex support](https://support.plex.tv/articles/requirements-for-remote-playback-of-personal-media/)).
   Enforcement reached Roku first, with other TV platforms and third-party
   clients following through 2026
   ([How-To Geek](https://www.howtogeek.com/plex-is-now-enforcing-remote-play-restrictions-on-tvs/)).
   The Remote Watch Pass is now listed at $2.99 a month or $29.99 a year
   after an introductory price that ran to June 2026
   ([Plex plans](https://www.plex.tv/plans/)). How-To Geek describes reader
   reaction as overwhelmingly negative, with many people saying they are
   moving to Jellyfin or Emby. One reader wrote that they felt "trapped in an
   abusive relationship".
2. **Plex sign-in depends on plex.tv, even at home.** The forum request for
   a local authentication service that survives plex.tv outages dates from
   2015. It has 372 votes, 95 posts and 6,077 views, the last post was in
   September 2026, and no staff member has replied
   ([forum topic 111339](https://forums.plex.tv/t/111339.json)).
3. **Two Plex breaches, and still no passkeys.** plex.tv was breached in
   2022 and again in August 2025, and both times users had to reset their
   passwords
   ([Betanews](https://betanews.com/2025/09/09/plex-suffers-data-breach-warns-customers-to-change-passwords/),
   [Android Authority](https://www.androidauthority.com/plex-data-breach-3595999/)).
   A passkey request from June 2023 has 27 votes, a post in December 2025
   citing the breach, and no staff reply
   ([forum topic 843216](https://forums.plex.tv/t/passkey-authentication-for-plex-accounts/843216)).
4. **Plex leaked viewing habits to friends.** In November 2023 the "Week in
   Review" emails and Discover Together showed friends what users had
   watched. The main thread drew 635 posts, 2,704 likes and 28,689 views,
   and its opening post had 115 heart reactions. Users also reported that
   email preferences they had turned off were switched back on
   ([forum topic 860206](https://forums.plex.tv/t/860206.json)). Related threads
   include "Discover together breached the privacy policy" (24 replies) and
   "Do not share my watch history" (14 replies).
5. **There are no per-user bandwidth limits on Plex.** The request dates
   from May 2017 and has 188 votes, 68 posts and 4,711 views. Admins with 5
   to 12 Mbps of upload want to cap friends without capping themselves
   ([forum topic 192859](https://forums.plex.tv/t/192859.json)).
6. **Jellyfin has no built-in two-factor or single sign-on, and the main SSO
   plugin is gone.** OIDC/SSO (#230, 1,191 votes, 45 comments) and two-factor
   (#26, 1,103 votes, 83 comments) are the most-voted security requests, and
   both are marked Planned
   ([Jellyfin feature board, most wanted](https://features.jellyfin.org/api/v1/posts?view=most-wanted&limit=100)).
   The community SSO plugin
   only ever worked in the web UI, and its author archived it on 2026-05-12
   ([jellyfin-plugin-sso](https://github.com/9p4/jellyfin-plugin-sso)).
7. **Jellyfin's authorization model keeps producing advisories.** Issue
   #5415 (March 2021) listed unauthenticated stream, subtitle, image, user
   and system-info endpoints. It also noted that authentication and user IDs
   are decoupled, so one signed-in user could reach another's data
   ([issue 5415](https://github.com/jellyfin/jellyfin/issues/5415)). In 2026
   alone Jellyfin has published at least 15 advisories, including a CVSS 9.9
   remote code execution flaw and a critical path traversal
   ([advisories](https://github.com/jellyfin/jellyfin/security/advisories)).
   Version 12.0 (September 2026) turned legacy authorization off by default
   ([release notes](https://github.com/jellyfin/jellyfin/releases/tag/v12.0)).
   That is progress, and it is honest to say Jellyfin is fixing these
   problems in the open.
8. **Jellyfin households have no profiles, PINs or quick switching.** The
   requests are user groups or master accounts (#493, 202 votes), multiple
   account switching (#2353, 154 votes, started), a profile PIN so kids
   cannot use a parent's profile (#2549, 62 votes), and reusable parental
   profiles (#3513, 8 votes)
   ([feature board search](https://features.jellyfin.org/api/v1/posts?query=parental&limit=30)).
9. **Parental controls are too blunt.** The requests are screen-time
   allowances (#699, 29 votes), allow-tags that work alongside the rating
   cap (#2737, 26 votes), per-library and genre filters (#2585, 25 votes;
   #1775, 14 votes), and per-item exceptions (#4035, 7 votes). Nobody offers
   an explicit-lyrics filter that a music-first household would expect.
10. **Sharing outside the server is missing on the video servers.** Jellyfin
    requests: temporary links to a file (#72, 580 votes), a SyncPlay invite
    link (#971, 267 votes), a sign-up page with admin approval (#494, 227
    votes), and QR-code Quick Connect (#2642, 225 votes, Planned). Wizarr,
    with 3.2k stars, exists only to fill the invitation gap across Plex,
    Jellyfin and Emby ([Wizarr](https://github.com/wizarrrr/wizarr)).
11. **Admins cannot stop streams cleanly or count them properly.** "Kill
    stream option" for idle or paused streams has 402 votes on Jellyfin
    (#301). Jellyfin's session limit counts every connected device, browsing
    included, which Infuse users report as a misfire
    ([Firecore forum](https://community.firecore.com/t/jellyfin-treats-each-device-as-a-simultaneous-stream-instead-of-just-looking-at-actual-streaming/48066/5)).
12. **Emby has no two-factor after eight years.** The thread opened in
    January 2018, now runs to 19 pages with 32 followers, and in May 2025
    staff said it was still on their list
    ([Emby forum](https://emby.media/community/topic/55542-2-factor-authentication-2fa/page/14)).
13. **Emby's 2023 compromise came from a convenience feature.**
    Passwordless local admin sign-in combined with forged proxy headers let
    attackers in, and about 1,200 servers were backdoored
    ([BleepingComputer](https://www.bleepingcomputer.com/news/security/emby-shuts-down-user-media-servers-hacked-in-recent-attack/),
    [CVE-2023-33193](https://cveawg.mitre.org/api/cve/CVE-2023-33193)).
14. **Navidrome users want directory and SSO sign-in.** LDAP (#141, 148
    reactions, open since 2020) and "Improve SSO support (Round #2)" (#2394,
    65 reactions) are both open; an earlier OIDC request (#858, 103
    reactions) is closed
    ([GitHub search](https://api.github.com/search/issues?q=repo:navidrome/navidrome+OIDC+OR+OAuth+OR+SSO&sort=reactions&per_page=8)).
    Its September 2026 batch of 19 advisories shows how much authorization
    surface a Subsonic-compatible API adds
    ([advisories](https://github.com/navidrome/navidrome/security/advisories)).
15. **Self-hosted servers stay unpatched.** Plex shipped the fix for
    CVE-2025-34158 in early August 2025, yet on 25 August Censys still saw
    about 314,000 exposed servers on vulnerable versions
    ([Help Net Security](https://www.helpnetsecurity.com/2025/08/27/plex-media-server-cve-2025-34158-attack/)).
16. **General-purpose remote access is hard for family.** Tailscale's free
    plan is generous: up to 6 users, unlimited user devices, and sharing on
    every plan ([pricing](https://tailscale.com/pricing)). But a friend must
    install a client, create an account and be an admin of their own tailnet
    to accept a shared machine
    ([sharing docs](https://tailscale.com/kb/1084/sharing)). Cloudflare's CDN
    terms reserve the right to limit video served without paid products
    ([terms](https://www.cloudflare.com/service-specific-terms-application-services/)).

## Where Gunmetal can be clearly better

Each idea names the pain point it answers and what would have to be true
technically. All of them fit records 1 and 2.

1. **Free remote access with no vendor in the path** (pain points 1, 2 and
   16). Native clients reach the server over iroh with no open ports, no
   account and no fee for viewers. For this to be true, every native client
   must embed iroh, and invite links must carry the server's node address.
   Relays must be self-hostable, and the docs must be honest that about 10%
   of connections fall back to a relay. The server must keep working with no
   internet connection at all, because sign-in is local.
2. **Passkeys first, with a code or QR fallback for TVs** (pain points 3, 6
   and 12). Phones and computers sign in with a passkey or a device-bound
   key. TVs show a QR code that the user approves from an already signed-in
   phone, which is Quick Connect done properly (#2642). For this to be true,
   native apps must hold a per-device key pair in the platform's secure
   storage, so they do not need WebAuthn or a domain. Browsers need HTTPS on
   a real domain, because WebAuthn will not accept an IP address as the
   relying-party ID (see Risks). TOTP can be an optional second factor for
   password accounts.
3. **Built-in OIDC that works in every client** (pain point 6). OIDC should
   cover role and library-group claims, as Immich does, and work in the
   native apps, not only the web UI. For this to be true, native apps must
   use a system browser and a redirect back to the app, and the server must
   verify TLS and refuse to fetch avatar URLs. Immich's 2026 advisories are
   the test checklist.
4. **App passwords and scoped keys for adapters** (pain points 7 and 14).
   The OpenSubsonic and Jellyfin adapters never accept the account password.
   Each third-party app gets its own revocable credential, scoped to that
   user and to read and play only, listed with the user's devices. For this
   to be true, the OpenSubsonic adapter must support the `apiKey` extension
   and refuse token-and-salt sign-in for the main credential. Older clients
   that need token-and-salt get a random per-app password; it must be stored
   reversibly, so its leak exposes only that one app. Key updates must never
   grant more than the caller holds, which is the Immich CVE-2026-23896
   lesson.
5. **Deny by default, proven by tests** (pain points 7, 13 and 14). Every
   route declares an authorization policy, and a test fails the build if any
   route lacks one. Every object fetch passes through one checking layer.
   Cross-user tests replay user A's IDs as user B on every endpoint,
   including adapters and share links. For this to be true, the route table
   must be enumerable in tests, IDs must be random (record 1), and the 100%
   coverage and zero-surviving-mutant gate must cover the authorization
   layer. There is no passwordless local sign-in option at all. Forwarding
   headers are ignored unless the admin lists trusted proxies.
6. **Households with real profiles** (pain point 8). A household account
   holds profiles, each with its own history, queue and taste, and switching
   between them is instant. Managed child profiles have no credentials and
   an optional PIN. For this to be true, the data model must separate the
   credential (account or device key) from the profile (history and
   restrictions), and the synced library on each device must be filtered per
   profile. Because record 1 syncs the library to the device, the filter must
   also be applied when building the sync payload, not only in the UI.
7. **Parental controls that fit real families** (pain point 9). Reusable
   policies with presets, rating ceilings per country system, unrated
   handling per type, allow-list exceptions per item, time windows plus a
   daily allowance, and an explicit-lyrics filter for music. For this to be
   true, the music parsers must read advisory or explicit flags from tags at
   scan time (which tags carry them is unverified), and the play-time
   counter must be enforced by the server, not the client.
8. **Invitations built in** (pain point 10). An invitation is a link or QR
   code with an expiry, a use count, a policy preset and an optional end
   date for membership. Its landing page tells the invitee which app to
   install. For this to be true, the invite secret should travel in the URL
   fragment of a static page on gunmetal.tv, so it never reaches any server
   log before the app takes it. Redeeming the invite creates the device key
   at that moment. No central account is involved.
9. **Share links for music, then for video** (pain point 10). Share an
   album, playlist or track with a password, an expiry and a download
   switch. Revocation takes effect on the next request. For this to be true,
   the signed stream URLs from record 1 must be short-lived and must re-check
   the share's state, which avoids Navidrome's stale-share bug. Preview cards
   should be off by default for privacy.
10. **Sessions users can see and control** (pain point 11). Every user sees
    their own devices and app passwords, with last-seen times, and can revoke
    any of them or sign out everywhere. Admins can stop a stream with a
    message. Concurrency limits count playback, not browsing. New-device
    alerts are built in. For this to be true, stream URLs must be bound to
    the session, so revocation stops playback within one URL lifetime.
11. **Bandwidth limits that respect direct play** (pain point 5). Set a
    total upload budget, shared fairly, and per-user caps. For music, a cap
    switches to Opus, which is cheap (record 2). For video, a cap only
    chooses among existing versions or suggests an offline download; it
    never quietly triggers a GPU transcode unless the admin allows that for
    the user. For this to be true, the playback decision engine in the core
    crate must take the user's policy and the live budget as inputs.
12. **Privacy that is visible** (pain point 4). No telemetry, no phone-home
    and no social features by default. Each user gets a "what your admin can
    see" page and a private listening mode that keeps a session out of
    history and out of recommendations. For this to be true, the
    append-only history log needs a "not recorded" path, and scrobbling
    stays in plugins with network grants (record 2). We must be honest that
    a server admin controls the machine and could read anything; the promise
    is about what the product surfaces.
13. **Security operations as a feature** (pain point 15). Publish GitHub
    advisories for every fix. Offer an update check that fetches a signed,
    static manifest with no identifiers attached, and can be switched off.
    Show an in-app banner when the running version has a known advisory. For
    this to be true, releases must be signed and the manifest must hold
    advisory ranges. The server must never be remotely disabled by the
    project, unlike Emby in 2023.
14. **Uploaded files cannot reach the filesystem by name.** Uploaded
    subtitles, lyrics and artwork are stored under content-hash names. SVG is
    never rasterised by a C library in the server process. This answers
    Jellyfin's CVE-2026-35031 and the SVG advisories at both Jellyfin and
    Immich. For this to be true, upload handlers must live in the core with
    the same no-panic, typed-error rules as the parsers.

## Risks and hard parts

- **Passkeys need a domain.** WebAuthn only works in a secure context, and
  the relying-party ID must be a domain, not an IP address
  ([MDN](https://developer.mozilla.org/en-US/docs/Web/API/PublicKeyCredentialCreationOptions)).
  A server reached by LAN IP or by iroh node ID has no such domain. Native
  apps can avoid the problem with their own device keys. However, platform
  passkey sync on iOS and Android also expects an associated web domain
  (unverified). The web client will need either the user's own domain or
  per-server names issued by the project, as plex.direct does. Issuing names
  is a central service, even if it holds no accounts.
- **Browsers on iroh are relay-only.** The WASM build cannot hole-punch, so
  all of a browser's traffic flows through a relay
  ([iroh docs](https://docs.iroh.computer/deployment/wasm-browser-support)).
  Remote video in a browser would therefore cost relay bandwidth on every
  byte. Music in a browser is affordable; video in a browser probably needs
  the user's own domain and reverse proxy, or a self-hosted relay.
- **Someone has to run relays.** n0's public relays are rate-limited and
  meant for testing only
  ([iroh relays](https://docs.iroh.computer/concepts/relays)). If the
  project runs default relays, it takes on cost and a soft central
  dependency, and it can see connection metadata such as IP addresses, but
  not content.
- **Identity data is not a cache.** Record 1 treats SQLite as a rebuildable
  cache, with watch history as the only irreplaceable data. User records,
  passkey public keys, device keys, app passwords, policies and invitations
  cannot be rebuilt by rescanning files. They need the same backup and
  export treatment as the history log, which probably needs a new record.
- **One identity across servers without a central account.** Plex's best
  sharing feature comes from plex.tv. Gunmetal can get close by letting the
  client hold many server memberships, each with its own device key, and add
  them through invite links. Moving to a new phone then needs a
  client-to-client transfer or a fresh invite from each server.
- **Adapters bring in weaker sign-in.** Subsonic token-and-salt and the
  Jellyfin API were designed around passwords. Navidrome's advisory history
  shows that a compatibility API doubles the authorization surface. Adapters
  must sit behind the same policy layer and the same tests, and should be
  off by default.
- **Per-user caps conflict with direct play.** A cap below a file's bitrate
  means transcoding, which is what Gunmetal is designed to avoid. The honest
  options are to refuse the stream, offer a download, use a lower version
  that already exists, or let the admin allow transcoding for that user.
- **Rating systems are a long tail.** Jellyfin is still fixing country
  rating systems in its 12.0 release. Gunmetal needs a data source for
  ratings and their equivalences, whose licence has not been checked
  (unverified).
- **Account recovery with no central account.** If the only admin loses
  every device and passkey, recovery has to rely on access to the host, such
  as a command-line reset that needs filesystem access. That must not become
  a remote path.
- **OIDC is easy to get subtly wrong.** Immich, a strong project, shipped
  TLS verification switched off, an open redirect and an SSRF in its OIDC
  path in 2026.
- **Volume of scrutiny.** Even small projects received 10 to 20 advisories
  a year in 2026, apparently because security researchers and automated
  tools now look harder. Gunmetal must plan for steady advisory triage from
  the first release.
- **Shared credentials and abuse.** Free remote access with no per-viewer
  fee will invite account sharing well beyond friends and family. Limits on
  concurrency and devices need to work from the first release.

## Open questions

1. Will the project run default iroh relays, and if so, who pays and what
   metadata do they log?
2. Should gunmetal.tv issue per-server HTTPS names (like plex.direct) so
   that browsers can use passkeys and connect directly, and does that break
   the spirit of "no central account"?
3. Does identity data need a new architecture record that sets it apart
   from the rebuildable SQLite cache?
4. Is client-side aggregation of server memberships enough to replace
   Plex's single account, or do people expect one sign-in across servers?
5. What can an admin see by default: other users' full history, only live
   sessions, or only totals? Is private listening on by default for music?
6. No telemetry at all, or opt-in anonymous counts like Navidrome's? The
   benchmark claims in the README might benefit from real-world numbers.
7. Will Gunmetal support LDAP and reverse-proxy header sign-in, both of
   which users request and both of which carry risk, or only OIDC?
8. On a shared living-room TV, is the TV a household device with a profile
   picker, or a personal device? Who approves its sign-in?
9. Should public share links cover video, given the bandwidth and the
   copyright exposure of sharing films with people outside the household
   (unverified legal analysis)?
10. How should the Jellyfin adapter map Gunmetal policies onto the
    Jellyfin-shaped user policy that existing clients expect?
11. Which tags in FLAC, MP3, MP4 and Ogg files reliably mark explicit
    content, and how do we treat files with no flag?
12. Should the server deny remote access to new accounts until an admin
    approves it, which would be a stricter default than any rival?

## Sources

- https://support.plex.tv/articles/requirements-for-remote-playback-of-personal-media/
- https://www.plex.tv/plans/
- https://www.howtogeek.com/plex-is-now-enforcing-remote-play-restrictions-on-tvs/
- https://www.androidauthority.com/plex-remote-viewing-restriction-roku-tv-3619768/
- https://www.aftvnews.com/plex-begins-enforcing-subscription-requirements-to-stream-remotely/
- https://support.plex.tv/articles/parental-controls/
- https://support.plex.tv/articles/203948776-managed-users/
- https://support.plex.tv/articles/203815766-what-is-plex-home/ (search snippet only; the page refused automated access)
- https://forums.plex.tv/t/15-user-is-the-limits/227027
- https://support.plex.tv/articles/200289506-remote-access/
- https://support.plex.tv/articles/216766168-accessing-a-server-through-relay/
- https://support.plex.tv/articles/two-factor-authentication/
- https://support.plex.tv/articles/201862428-plex-accounts/
- https://forums.plex.tv/t/passkey-authentication-for-plex-accounts/843216
- https://forums.plex.tv/t/843216.json
- https://forums.plex.tv/t/111339.json
- https://forums.plex.tv/t/192859.json
- https://forums.plex.tv/t/860206.json
- https://forums.plex.tv/search.json?q=week%20in%20review%20privacy
- https://forums.plex.tv/search.json?q=per%20user%20bandwidth%20limit%20order:votes
- https://forums.plex.tv/search.json?q=sign%20in%20offline%20local%20server%20plex.tv%20down
- https://betanews.com/2025/09/09/plex-suffers-data-breach-warns-customers-to-change-passwords/
- https://www.androidauthority.com/plex-data-breach-3595999/
- https://www.bitdefender.com/en-us/blog/hotforsecurity/plex-reset-passwords-data-breach-2025
- https://www.helpnetsecurity.com/2025/08/27/plex-media-server-cve-2025-34158-attack/
- https://www.tenable.com/cve/CVE-2025-34158
- https://cveawg.mitre.org/api/cve/CVE-2020-5741
- https://jellyfin.org/
- https://jellyfin.org/posts/
- https://jellyfin.org/docs/general/server/users/
- https://jellyfin.org/docs/general/server/quick-connect
- https://raw.githubusercontent.com/jellyfin/jellyfin/master/MediaBrowser.Model/Users/UserPolicy.cs
- https://raw.githubusercontent.com/jellyfin/jellyfin-web/master/src/strings/en-us.json
- https://github.com/jellyfin/jellyfin/issues/5415
- https://github.com/jellyfin/jellyfin/security/advisories
- https://github.com/jellyfin/jellyfin/security/advisories?page=2
- https://github.com/jellyfin/jellyfin/releases/tag/v12.0
- https://github.com/jellyfin/jellyfin/releases/latest
- https://www.sentinelone.com/vulnerability-database/cve-2026-35031/
- https://www.sentinelone.com/vulnerability-database/cve-2026-49246/
- https://www.strix.ai/cve/CVE-2026-31852
- https://www.strix.ai/cve/CVE-2026-35032
- https://nvd.nist.gov/vuln/detail/CVE-2023-49096
- https://features.jellyfin.org/api/v1/posts?view=most-wanted&limit=100
- https://features.jellyfin.org/api/v1/posts?query=parental&limit=30
- https://features.jellyfin.org/api/v1/posts?query=invite&limit=20
- https://features.jellyfin.org/api/v1/posts?query=passkey&limit=20
- https://features.jellyfin.org/api/v1/posts?query=bandwidth&limit=20
- https://features.jellyfin.org/api/v1/posts?query=remote%20access&limit=20
- https://github.com/9p4/jellyfin-plugin-sso
- https://newreleases.io/project/github/ZL154/JellyfinSecurity/release/v2.0.0
- https://community.firecore.com/t/jellyfin-treats-each-device-as-a-simultaneous-stream-instead-of-just-looking-at-actual-streaming/48066/5
- https://github.com/wizarrrr/wizarr
- https://emby.media/support/articles/Users.html
- https://emby.media/support/articles/Parental-Controls.html
- https://emby.media/support/articles/Emby-Connect.html
- https://emby.media/premiere.html
- https://emby.media/community/topic/55542-2-factor-authentication-2fa/page/14
- https://emby.media/community/topic/109483-maximum-streaming-limit
- https://cveawg.mitre.org/api/cve/CVE-2023-33193
- https://www.bleepingcomputer.com/news/security/emby-shuts-down-user-media-servers-hacked-in-recent-attack/
- https://www.navidrome.org/docs/usage/admin/security/
- https://www.navidrome.org/docs/usage/admin/insights/
- https://www.navidrome.org/docs/usage/features/sharing/
- https://www.navidrome.org/docs/usage/features/multi-library/
- https://www.navidrome.org/docs/usage/integration/authentication/
- https://github.com/navidrome/navidrome/security/advisories
- https://github.com/navidrome/navidrome/security/advisories?page=2
- https://github.com/navidrome/navidrome/security/advisories?page=3
- https://github.com/navidrome/navidrome/releases
- https://api.github.com/search/issues?q=repo:navidrome/navidrome+OIDC+OR+OAuth+OR+SSO&sort=reactions&per_page=8
- https://opensubsonic.netlify.app/docs/extensions/apikeyauth/
- https://docs.immich.app/administration/oauth/
- https://docs.immich.app/features/sharing/
- https://docs.immich.app/administration/user-management/
- https://github.com/immich-app/immich/releases/tag/v1.133.0
- https://github.com/immich-app/immich/security/advisories
- https://github.com/immich-app/immich/security/advisories/GHSA-237r-x578-h5mv
- https://api.github.com/search/issues?q=repo:immich-app/immich+2FA+OR+TOTP+OR+%22two+factor%22+in:title&sort=reactions&per_page=5
- https://tailscale.com/pricing
- https://tailscale.com/kb/1223/funnel
- https://tailscale.com/kb/1084/sharing
- https://www.cloudflare.com/service-specific-terms-application-services/
- https://docs.iroh.computer/concepts/relays
- https://docs.iroh.computer/deployment/wasm-browser-support
- https://developer.mozilla.org/en-US/docs/Web/API/PublicKeyCredentialCreationOptions
