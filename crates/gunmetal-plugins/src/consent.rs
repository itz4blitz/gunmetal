//! Consent text and settings forms.
//!
//! The screen is plain text generated from the manifest. A television never
//! shows it (SEC-EXT-039). Settings are a typed form drawn by Gunmetal, never
//! plugin markup (SEC-EXT-032).

use crate::record::{Manifest, Mode, SecretDecl};

/// The fixed limit under "It cannot".
const CANNOT: &str = "- see what anyone else plays, read your files, or change your library";

/// Appended only when each person turns the plugin on for themselves.
const PRIVATE_LISTENING: &str = "Tracks you play in private listening are never sent.";

/// Where a person is asked for consent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Surface {
    /// A phone.
    Phone,
    /// The web client.
    Web,
    /// A desktop app.
    Desktop,
    /// A television. Consent is never shown here.
    Tv,
}

/// A television cannot show a consent screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConsentError;

/// Who published the plugin, and whether their signature verified.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Publisher<'a> {
    /// The publisher's name, copied as text.
    pub name: &'a str,
    /// Whether the package signature verified.
    pub verified: bool,
}

/// The most bytes a text setting may hold.
const TEXT_MAX_BYTES: usize = 256;

/// One setting Gunmetal's own client draws. Never plugin markup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormField {
    /// A short text value.
    Text {
        /// The field name.
        name: String,
        /// The label the person sees, copied as text.
        label: String,
        /// The most bytes the value may hold.
        max_bytes: usize,
    },
    /// A secret the person types.
    Secret {
        /// The field name.
        name: String,
        /// The label the person sees, copied as text.
        label: String,
    },
    /// A yes or no switch.
    Toggle {
        /// The field name.
        name: String,
        /// The label the person sees, copied as text.
        label: String,
    },
}

/// Why a settings triple is not a field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormError {
    /// The triple carries markup, or a kind that would deliver it.
    Markup,
    /// The kind is not one this host draws.
    UnknownKind,
    /// The name or the label is empty.
    Empty,
}

/// The plain-language consent screen for `manifest`.
///
/// Phone, web and desktop get the same text. A plugin's why and a shared
/// secret's label are copied as text. A per-user secret is named as the
/// plugin plus the secret's name.
///
/// # Errors
///
/// [`ConsentError`] when `surface` is a television. The check is first: a TV
/// never shows consent, whatever the manifest says.
#[must_use = "the consent screen must be shown before the plugin runs"]
pub fn text(
    manifest: &Manifest,
    publisher: Publisher<'_>,
    surface: Surface,
) -> Result<String, ConsentError> {
    match surface {
        Surface::Tv => Err(ConsentError),
        Surface::Phone | Surface::Web | Surface::Desktop => Ok(screen(manifest, publisher)),
    }
}

/// Turns setting triples into fields Gunmetal's own clients can draw.
///
/// Each triple is `(kind, name, label)`. Names and labels are copied as text.
///
/// # Errors
///
/// [`FormError::Markup`] when the kind is `html` or `script`, when the kind
/// contains a less-than character, or when the name or label contains a
/// less-than or greater-than character.
///
/// [`FormError::Empty`] when the name or the label is empty and the triple is
/// not markup.
///
/// [`FormError::UnknownKind`] for any other kind.
#[must_use = "a refused form must not be drawn"]
pub fn form(fields: &[(&str, &str, &str)]) -> Result<Vec<FormField>, FormError> {
    let mut parsed = Vec::with_capacity(fields.len());
    for triple in fields {
        let (kind, name, label) = *triple;
        parsed.push(classify(kind, name, label)?);
    }
    Ok(parsed)
}

/// The screen for a phone, the web client or a desktop app.
fn screen(manifest: &Manifest, publisher: Publisher<'_>) -> String {
    let Publisher { name, verified } = publisher;
    let status = if verified {
        "signature verified"
    } else {
        "signature not verified"
    };
    let plugin = manifest.name.as_str();
    let mut lines = vec![
        format!("{plugin}, published by {name} ({status})"),
        String::new(),
        "If you turn this on, it will:".to_owned(),
    ];
    lines.extend(will_lines(manifest));
    lines.push(String::new());
    lines.push("It cannot:".to_owned());
    lines.push(CANNOT.to_owned());
    match manifest.mode {
        Mode::PerUser => {
            lines.push(String::new());
            lines.push(PRIVATE_LISTENING.to_owned());
        }
        Mode::Server => {}
    }
    lines.join("\n")
}

/// The lines under "it will".
fn will_lines(manifest: &Manifest) -> Vec<String> {
    if manifest.network.is_empty() && manifest.secrets.is_empty() {
        return vec!["- run only the grants it lists, which are none".to_owned()];
    }
    let mut lines = Vec::new();
    for grant in &manifest.network {
        let host = grant.host.as_str();
        let why = grant.why.as_str();
        lines.push(format!(
            "- send the title, artist, album and time of each track you play to {host}, to {why}"
        ));
    }
    let plugin = manifest.name.as_str();
    for secret in &manifest.secrets {
        lines.push(secret_line(plugin, secret));
    }
    lines
}

/// One secret, copied as text. A per-user secret names the plugin and the
/// secret. A shared secret uses its label, with no "your".
fn secret_line(plugin: &str, secret: &SecretDecl) -> String {
    let kept = if secret.per_user {
        let name = secret.name.as_str();
        format!("your {plugin} {name}")
    } else {
        secret.label.clone()
    };
    format!("- keep {kept}, which only this plugin can read")
}

/// One triple, or why it is not a field.
fn classify(kind: &str, name: &str, label: &str) -> Result<FormField, FormError> {
    if markup_kind(kind) || markup_text(name) || markup_text(label) {
        return Err(FormError::Markup);
    }
    if name.is_empty() || label.is_empty() {
        return Err(FormError::Empty);
    }
    let name = name.to_owned();
    let label = label.to_owned();
    match kind {
        "text" => Ok(FormField::Text {
            name,
            label,
            max_bytes: TEXT_MAX_BYTES,
        }),
        "secret" => Ok(FormField::Secret { name, label }),
        "toggle" => Ok(FormField::Toggle { name, label }),
        _ => Err(FormError::UnknownKind),
    }
}

/// Whether `kind` would deliver markup.
fn markup_kind(kind: &str) -> bool {
    kind.contains('<') || kind == "html" || kind == "script"
}

/// Whether `text` contains a markup character.
fn markup_text(text: &str) -> bool {
    text.contains('<') || text.contains('>')
}

#[cfg(test)]
mod tests {
    use super::{ConsentError, FormError, FormField, Publisher, Surface, form, text};
    use crate::id::PluginId;
    use crate::record::{
        DEFAULT_DEADLINE_SECS, DEFAULT_KV_MIB, DEFAULT_MEMORY_MIB, DEFAULT_OUTBOUND_PER_MINUTE,
        ExactHost, Manifest, Mode, NetworkGrant, SecretDecl,
    };
    use crate::scope::Scope;
    use crate::version::PluginVersion;
    use crate::world::World;

    const LISTENBRAINZ: &str = "ListenBrainz, published by Gunmetal (signature verified)\n\nIf you turn this on, it will:\n- send the title, artist, album and time of each track you play to api.listenbrainz.org, to Send the tracks you play to your ListenBrainz account\n- keep your ListenBrainz token, which only this plugin can read\n\nIt cannot:\n- see what anyone else plays, read your files, or change your library\n\nTracks you play in private listening are never sent.";

    fn listenbrainz() -> Manifest {
        Manifest {
            id: PluginId::parse("org.listenbrainz.scrobbler").expect("id"),
            name: "ListenBrainz".to_owned(),
            version: PluginVersion::parse("1.2.0").expect("version"),
            world: World::Scrobbler,
            mode: Mode::PerUser,
            network: vec![NetworkGrant {
                host: ExactHost::parse_public("api.listenbrainz.org").expect("host"),
                why: "Send the tracks you play to your ListenBrainz account".to_owned(),
            }],
            scopes: vec![Scope::EventsSelf],
            secrets: vec![SecretDecl {
                name: "token".to_owned(),
                label: "Your ListenBrainz user token".to_owned(),
                per_user: true,
            }],
            memory_mib: DEFAULT_MEMORY_MIB,
            deadline_secs: DEFAULT_DEADLINE_SECS,
            kv_mib: DEFAULT_KV_MIB,
            outbound_per_minute: DEFAULT_OUTBOUND_PER_MINUTE,
        }
    }

    fn publisher(name: &str, verified: bool) -> Publisher<'_> {
        Publisher { name, verified }
    }

    /// Verifies: SEC-EXT-032, SEC-EXT-039
    #[test]
    fn a_tv_never_shows_consent() {
        let manifest = listenbrainz();
        assert_eq!(
            text(&manifest, publisher("Gunmetal", true), Surface::Tv),
            Err(ConsentError)
        );
        assert_eq!(
            text(&manifest, publisher("Gunmetal", false), Surface::Tv),
            Err(ConsentError)
        );
        let mut server = manifest;
        server.mode = Mode::Server;
        server.network.clear();
        server.secrets.clear();
        server.name = "<script>".to_owned();
        assert_eq!(
            text(&server, publisher("Gunmetal", true), Surface::Tv),
            Err(ConsentError)
        );
    }

    /// Verifies: SEC-EXT-039
    #[test]
    fn the_listenbrainz_screen_is_this_text_on_phone_web_and_desktop() {
        let manifest = listenbrainz();
        let published = publisher("Gunmetal", true);
        for surface in [Surface::Phone, Surface::Web, Surface::Desktop] {
            let screen = text(&manifest, published, surface);
            assert_eq!(screen.as_deref(), Ok(LISTENBRAINZ), "{surface:?}");
            let shown = screen.expect("screen");
            assert!(!shown.contains('<'), "{surface:?}");
            assert!(!shown.contains('>'), "{surface:?}");
        }
    }

    /// Verifies: SEC-EXT-039
    #[test]
    fn an_unverified_publisher_is_named_not_verified() {
        let screen = text(
            &listenbrainz(),
            publisher("Other Press", false),
            Surface::Phone,
        );
        assert_eq!(
            screen,
            Ok("ListenBrainz, published by Other Press (signature not verified)\n\nIf you turn this on, it will:\n- send the title, artist, album and time of each track you play to api.listenbrainz.org, to Send the tracks you play to your ListenBrainz account\n- keep your ListenBrainz token, which only this plugin can read\n\nIt cannot:\n- see what anyone else plays, read your files, or change your library\n\nTracks you play in private listening are never sent.".to_owned())
        );
    }

    /// Verifies: SEC-EXT-039
    #[test]
    fn no_grants_still_names_what_the_plugin_cannot_do() {
        let manifest = Manifest {
            id: PluginId::parse("org.example.shelf").expect("id"),
            name: "Shelf".to_owned(),
            version: PluginVersion::parse("1.2.0").expect("version"),
            world: World::MusicMetadataProvider,
            mode: Mode::PerUser,
            network: Vec::new(),
            scopes: vec![Scope::LibraryRead],
            secrets: Vec::new(),
            memory_mib: DEFAULT_MEMORY_MIB,
            deadline_secs: DEFAULT_DEADLINE_SECS,
            kv_mib: DEFAULT_KV_MIB,
            outbound_per_minute: DEFAULT_OUTBOUND_PER_MINUTE,
        };
        assert_eq!(
            text(&manifest, publisher("Ada", true), Surface::Web),
            Ok("Shelf, published by Ada (signature verified)\n\nIf you turn this on, it will:\n- run only the grants it lists, which are none\n\nIt cannot:\n- see what anyone else plays, read your files, or change your library\n\nTracks you play in private listening are never sent.".to_owned())
        );
    }

    /// Verifies: SEC-EXT-039
    #[test]
    fn a_server_plugin_omits_private_listening() {
        let mut manifest = listenbrainz();
        manifest.mode = Mode::Server;
        assert_eq!(
            text(&manifest, publisher("Gunmetal", true), Surface::Desktop),
            Ok("ListenBrainz, published by Gunmetal (signature verified)\n\nIf you turn this on, it will:\n- send the title, artist, album and time of each track you play to api.listenbrainz.org, to Send the tracks you play to your ListenBrainz account\n- keep your ListenBrainz token, which only this plugin can read\n\nIt cannot:\n- see what anyone else plays, read your files, or change your library".to_owned())
        );
    }

    /// Verifies: SEC-EXT-039
    #[test]
    fn a_shared_secret_is_kept_under_its_label() {
        let manifest = Manifest {
            id: PluginId::parse("org.example.archive").expect("id"),
            name: "Archive".to_owned(),
            version: PluginVersion::parse("1.0.0").expect("version"),
            world: World::Scrobbler,
            mode: Mode::PerUser,
            network: Vec::new(),
            scopes: Vec::new(),
            secrets: vec![SecretDecl {
                name: "api_key".to_owned(),
                label: "Server archive key".to_owned(),
                per_user: false,
            }],
            memory_mib: DEFAULT_MEMORY_MIB,
            deadline_secs: DEFAULT_DEADLINE_SECS,
            kv_mib: DEFAULT_KV_MIB,
            outbound_per_minute: DEFAULT_OUTBOUND_PER_MINUTE,
        };
        assert_eq!(
            text(&manifest, publisher("Gunmetal", true), Surface::Phone),
            Ok("Archive, published by Gunmetal (signature verified)\n\nIf you turn this on, it will:\n- keep Server archive key, which only this plugin can read\n\nIt cannot:\n- see what anyone else plays, read your files, or change your library\n\nTracks you play in private listening are never sent.".to_owned())
        );
    }

    /// Verifies: SEC-EXT-039
    #[test]
    fn a_why_containing_script_is_copied_as_text() {
        let manifest = Manifest {
            id: PluginId::parse("org.listenbrainz.scrobbler").expect("id"),
            name: "ListenBrainz".to_owned(),
            version: PluginVersion::parse("1.2.0").expect("version"),
            world: World::Scrobbler,
            mode: Mode::PerUser,
            network: vec![NetworkGrant {
                host: ExactHost::parse_public("api.listenbrainz.org").expect("host"),
                why: "before <script> after".to_owned(),
            }],
            scopes: Vec::new(),
            secrets: Vec::new(),
            memory_mib: DEFAULT_MEMORY_MIB,
            deadline_secs: DEFAULT_DEADLINE_SECS,
            kv_mib: DEFAULT_KV_MIB,
            outbound_per_minute: DEFAULT_OUTBOUND_PER_MINUTE,
        };
        let screen = text(&manifest, publisher("Gunmetal", true), Surface::Phone);
        assert_eq!(
            screen,
            Ok("ListenBrainz, published by Gunmetal (signature verified)\n\nIf you turn this on, it will:\n- send the title, artist, album and time of each track you play to api.listenbrainz.org, to before <script> after\n\nIt cannot:\n- see what anyone else plays, read your files, or change your library\n\nTracks you play in private listening are never sent.".to_owned())
        );
        let shown = screen.expect("screen");
        assert!(
            shown.contains("<script>"),
            "the why is copied as text, not stripped"
        );
        assert!(!shown.contains("&lt;"), "the why is not escaped as HTML");
        assert!(shown.contains("before <script> after"));
    }

    /// Verifies: SEC-EXT-039
    #[test]
    fn a_label_containing_script_is_copied_as_text() {
        let manifest = Manifest {
            id: PluginId::parse("org.example.archive").expect("id"),
            name: "Archive".to_owned(),
            version: PluginVersion::parse("1.0.0").expect("version"),
            world: World::Scrobbler,
            mode: Mode::Server,
            network: Vec::new(),
            scopes: Vec::new(),
            secrets: vec![SecretDecl {
                name: "api_key".to_owned(),
                label: "<script>".to_owned(),
                per_user: false,
            }],
            memory_mib: DEFAULT_MEMORY_MIB,
            deadline_secs: DEFAULT_DEADLINE_SECS,
            kv_mib: DEFAULT_KV_MIB,
            outbound_per_minute: DEFAULT_OUTBOUND_PER_MINUTE,
        };
        let screen = text(&manifest, publisher("Gunmetal", true), Surface::Web);
        assert_eq!(
            screen,
            Ok("Archive, published by Gunmetal (signature verified)\n\nIf you turn this on, it will:\n- keep <script>, which only this plugin can read\n\nIt cannot:\n- see what anyone else plays, read your files, or change your library".to_owned())
        );
        let shown = screen.expect("screen");
        assert!(shown.contains("<script>"));
        assert!(!shown.contains("&lt;"));
    }

    /// Verifies: SEC-EXT-039
    #[test]
    fn every_host_and_secret_is_listed_in_order() {
        let manifest = Manifest {
            id: PluginId::parse("org.example.radio").expect("id"),
            name: "Radio".to_owned(),
            version: PluginVersion::parse("0.1.0").expect("version"),
            world: World::Scrobbler,
            mode: Mode::PerUser,
            network: vec![
                NetworkGrant {
                    host: ExactHost::parse_public("one.example").expect("host"),
                    why: "First reason".to_owned(),
                },
                NetworkGrant {
                    host: ExactHost::parse_public("two.example").expect("host"),
                    why: "Second reason".to_owned(),
                },
            ],
            scopes: Vec::new(),
            secrets: vec![
                SecretDecl {
                    name: "session".to_owned(),
                    label: "Your Radio user token".to_owned(),
                    per_user: true,
                },
                SecretDecl {
                    name: "api_key".to_owned(),
                    label: "Station key".to_owned(),
                    per_user: false,
                },
            ],
            memory_mib: DEFAULT_MEMORY_MIB,
            deadline_secs: DEFAULT_DEADLINE_SECS,
            kv_mib: DEFAULT_KV_MIB,
            outbound_per_minute: DEFAULT_OUTBOUND_PER_MINUTE,
        };
        assert_eq!(
            text(&manifest, publisher("Ada", false), Surface::Desktop),
            Ok("Radio, published by Ada (signature not verified)\n\nIf you turn this on, it will:\n- send the title, artist, album and time of each track you play to one.example, to First reason\n- send the title, artist, album and time of each track you play to two.example, to Second reason\n- keep your Radio session, which only this plugin can read\n- keep Station key, which only this plugin can read\n\nIt cannot:\n- see what anyone else plays, read your files, or change your library\n\nTracks you play in private listening are never sent.".to_owned())
        );
    }

    /// Verifies: SEC-EXT-032
    #[test]
    fn a_three_field_form_is_text_secret_and_toggle() {
        assert_eq!(
            form(&[
                ("text", "username", "User name"),
                ("secret", "token", "Your token"),
                ("toggle", "share", "Share plays"),
            ]),
            Ok(vec![
                FormField::Text {
                    name: "username".to_owned(),
                    label: "User name".to_owned(),
                    max_bytes: 256,
                },
                FormField::Secret {
                    name: "token".to_owned(),
                    label: "Your token".to_owned(),
                },
                FormField::Toggle {
                    name: "share".to_owned(),
                    label: "Share plays".to_owned(),
                },
            ])
        );
    }

    /// Verifies: SEC-EXT-032
    #[test]
    fn names_and_labels_are_copied_without_trimming() {
        assert_eq!(
            form(&[("text", " user ", " Label ")]),
            Ok(vec![FormField::Text {
                name: " user ".to_owned(),
                label: " Label ".to_owned(),
                max_bytes: 256,
            }])
        );
        assert_eq!(
            form(&[("secret", "amp", "&amp;")]),
            Ok(vec![FormField::Secret {
                name: "amp".to_owned(),
                label: "&amp;".to_owned(),
            }])
        );
    }

    /// Verifies: SEC-EXT-032
    #[test]
    fn markup_empty_and_unknown_kinds_are_refused() {
        let cases = [
            ("html", "name", "label", FormError::Markup),
            ("script", "name", "label", FormError::Markup),
            ("<script", "name", "label", FormError::Markup),
            ("text<script", "name", "label", FormError::Markup),
            ("text", "<b>", "label", FormError::Markup),
            ("text", "name", "<b>", FormError::Markup),
            ("text", "na>me", "label", FormError::Markup),
            ("secret", "name", "lab>el", FormError::Markup),
            ("html", "", "label", FormError::Markup),
            ("text", "<", "", FormError::Markup),
            ("widget", "name", "label", FormError::UnknownKind),
            ("HTML", "name", "label", FormError::UnknownKind),
            ("javascript", "name", "label", FormError::UnknownKind),
            ("script>", "name", "label", FormError::UnknownKind),
            (">", "name", "label", FormError::UnknownKind),
            ("", "name", "label", FormError::UnknownKind),
            (" text", "name", "label", FormError::UnknownKind),
            ("text", "", "label", FormError::Empty),
            ("secret", "name", "", FormError::Empty),
            ("toggle", "", "", FormError::Empty),
            ("widget", "", "label", FormError::Empty),
            ("widget", "name", "", FormError::Empty),
        ];
        for (kind, name, label, expected) in cases {
            assert_eq!(
                form(&[(kind, name, label)]),
                Err(expected),
                "{kind:?} {name:?} {label:?}"
            );
        }
        assert_eq!(
            form(&[
                ("text", "ok", "Ok"),
                ("html", "x", "X"),
                ("widget", "y", "Y"),
            ]),
            Err(FormError::Markup)
        );
        assert_eq!(form(&[]), Ok(vec![]));
    }
}
