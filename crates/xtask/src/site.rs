//! `xtask site`: the project site, gunmetal.tv, is the static files under
//! `site/`, and this check holds them to the security baseline's rules for
//! a service the project operates (work package WP-139). It runs against
//! the real repository in this module's tests, so the gate enforces it on
//! every change.
//!
//! - `site/.well-known/security.txt` has `Contact`, `Expires`, `Policy`,
//!   `Canonical` and `Preferred-Languages` (RFC 9116), its addresses use
//!   HTTPS, its canonical address is on gunmetal.tv, and it expires in the
//!   future and less than a year ahead (SEC-SUP-008).
//! - `site/_headers`, which Cloudflare Pages and Netlify both read, sends
//!   every page `Strict-Transport-Security` with a `max-age` of at least
//!   two years, `includeSubDomains` and `preload` (SEC-STD-016), and a
//!   `Content-Security-Policy` of `default-src 'none'` whose sources are
//!   only `'none'` and `'self'`, so a browser loads nothing from another
//!   origin. No path may send a weaker value of either.
//! - Every page links to the privacy notice at `/privacy/`, and no page
//!   loads anything from another origin: only an `<a href>` may name one
//!   (SEC-PRV-054).
//! - No page runs script: no `<script>` or `<style>` element, no event
//!   handler attribute and no `javascript:` address. So no landing page
//!   can read the invitation secret from the URL fragment, and the
//!   fragment never reaches the project's servers (SEC-PRV-055).
//! - The site holds only pages, text files and `_headers`: no script,
//!   function, server configuration or other dynamic endpoint
//!   (SEC-HIS-061).
//!
//! The page scanner is small and fails closed: an unterminated tag, quoted
//! value or comment is a finding, an attribute value with a character
//! reference (which it does not decode) is one, and an attribute value that
//! only looks like another origin's address is one too. It reads addresses
//! as a browser does, ignoring tabs and line breaks and taking `\` as `/`.

use crate::tree::Tree;

/// The site's directory, from the repository root.
const SITE: &str = "site";

/// The site's `security.txt`.
const SECURITY_TXT: &str = "site/.well-known/security.txt";

/// The site's response headers.
const HEADERS: &str = "site/_headers";

/// The privacy notice (SEC-PRV-054).
const PRIVACY_NOTICE: &str = "site/privacy/index.html";

/// The invitation landing page (ACC-080, SEC-PRV-055).
const INVITE: &str = "site/invite/index.html";

/// Files the site must have.
const REQUIRED: [&str; 4] = [SECURITY_TXT, HEADERS, PRIVACY_NOTICE, INVITE];

/// The address every page links to for the privacy notice.
const PRIVACY_LINK: &str = "/privacy/";

/// The site's own origin.
const ORIGIN: &str = "https://gunmetal.tv";

/// Where `security.txt` says it lives (RFC 9116, section 2.5.2).
const CANONICAL: &str = "https://gunmetal.tv/.well-known/security.txt";

/// The shortest HSTS `max-age` the project's domains may send, two years
/// in seconds (SEC-STD-016).
const HSTS_MAX_AGE: u64 = 63_072_000;

/// How far ahead `security.txt` may expire: less than a year, counted as
/// 365 days in seconds (SEC-SUP-008).
const YEAR: u64 = 365 * 86_400;

/// The path pattern whose headers apply to every page.
const EVERY_PAGE: &str = "/*";

/// Elements whose content is code that the page scanner does not read.
const CODE: [&str; 2] = ["script", "style"];

/// Something `site` found wrong. Paths are from the repository root.
#[derive(Debug, PartialEq, Eq)]
pub enum Finding {
    /// A file the site must have is missing.
    Missing {
        /// The file.
        path: String,
    },
    /// A file that is not a page, a text file or `site/_headers`, or that
    /// is not UTF-8 text.
    Unexpected {
        /// The file.
        path: String,
    },
    /// A line of `security.txt` or `_headers` that is not a name, a colon
    /// and a value, or a header before any path.
    Malformed {
        /// The file.
        path: String,
        /// The line, from 1.
        line: usize,
    },
    /// A required `security.txt` field is absent.
    Absent {
        /// The field.
        field: &'static str,
    },
    /// A `security.txt` field that may appear only once appears again.
    Repeated {
        /// The field.
        field: &'static str,
    },
    /// A `security.txt` field's value is not what the rules allow.
    Invalid {
        /// The field.
        field: &'static str,
        /// The value.
        value: String,
    },
    /// `security.txt` has expired.
    Expired {
        /// Its `Expires`, in seconds since the epoch.
        expires: u64,
        /// The time of the check, in seconds since the epoch.
        now: u64,
    },
    /// `security.txt` expires a year or more ahead.
    TooFar {
        /// Its `Expires`, in seconds since the epoch.
        expires: u64,
        /// The time of the check, in seconds since the epoch.
        now: u64,
    },
    /// The headers for every page (`/*`) do not include this one.
    NoHeader {
        /// The header.
        header: &'static str,
    },
    /// A path's value for a checked header is weaker than the rules allow.
    WeakHeader {
        /// The header.
        header: &'static str,
        /// The path pattern it is sent for.
        path: String,
        /// The value.
        value: String,
    },
    /// A page with an unterminated tag, quoted value or comment.
    Unparsable {
        /// The page.
        path: String,
    },
    /// A page with an element whose content is code.
    Code {
        /// The page.
        path: String,
        /// The element.
        element: String,
    },
    /// A page with an event handler or a `javascript:` address.
    Script {
        /// The page.
        path: String,
        /// The element.
        element: String,
        /// The attribute.
        attribute: String,
    },
    /// A page that loads something from another origin.
    ThirdParty {
        /// The page.
        path: String,
        /// The element.
        element: String,
        /// The attribute.
        attribute: String,
        /// The attribute's value.
        value: String,
    },
    /// A page with a character reference in an attribute value, which the
    /// page scanner does not decode, so it cannot tell what the value names.
    Reference {
        /// The page.
        path: String,
        /// The element.
        element: String,
        /// The attribute.
        attribute: String,
    },
    /// A page with no link to the privacy notice.
    NoPrivacyLink {
        /// The page.
        path: String,
    },
}

/// Checks the site in `tree` at time `now`, in seconds since the epoch.
pub fn check(tree: &dyn Tree, now: u64) -> Vec<Finding> {
    let files: Vec<String> = tree
        .files(SITE)
        .iter()
        .map(|file| format!("{SITE}/{file}"))
        .collect();
    let mut findings: Vec<Finding> = REQUIRED
        .iter()
        .filter(|&&required| !files.iter().any(|file| file == required))
        .map(|&path| Finding::Missing {
            path: path.to_owned(),
        })
        .collect();
    for path in &files {
        findings.extend(file(path, tree.read(path), now));
    }
    findings
}

/// The findings for the site's file `path`, whose text is `text`. The
/// extension, everything after the path's last `.`, is compared exactly:
/// the site's pages and text files are named in lowercase.
fn file(path: &str, text: Option<String>, now: u64) -> Vec<Finding> {
    let extension = path.rsplit_once('.').map(|(_, extension)| extension);
    match (text, extension) {
        (Some(text), _) if path == SECURITY_TXT => security_txt(&text, now),
        (Some(text), _) if path == HEADERS => headers(&text),
        (Some(text), Some("html")) => page(path, &text),
        (Some(_), Some("txt")) => Vec::new(),
        _ => vec![Finding::Unexpected {
            path: path.to_owned(),
        }],
    }
}

/// A `security.txt` field the site must have.
struct Field {
    /// Its name.
    name: &'static str,
    /// Whether it may appear only once.
    once: bool,
    /// Whether a value is allowed.
    valid: fn(&str) -> bool,
}

/// The fields SEC-SUP-008 requires.
const FIELDS: [Field; 5] = [
    Field {
        name: "Contact",
        once: false,
        valid: is_https,
    },
    Field {
        name: "Expires",
        once: true,
        valid: |value| epoch_seconds(value).is_some(),
    },
    Field {
        name: "Policy",
        once: false,
        valid: is_https,
    },
    Field {
        name: "Canonical",
        once: false,
        valid: |value| value == CANONICAL,
    },
    Field {
        name: "Preferred-Languages",
        once: true,
        valid: is_language_list,
    },
];

/// The findings for `security.txt`, whose text is `text`, at `now`.
fn security_txt(text: &str, now: u64) -> Vec<Finding> {
    let mut findings = Vec::new();
    let mut fields = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        match line.split_once(':') {
            Some((name, value)) => fields.push((name.trim(), value.trim())),
            None => findings.push(Finding::Malformed {
                path: SECURITY_TXT.to_owned(),
                line: index + 1,
            }),
        }
    }
    for field in FIELDS {
        let values: Vec<&str> = fields
            .iter()
            .filter(|(name, _)| name.eq_ignore_ascii_case(field.name))
            .map(|&(_, value)| value)
            .collect();
        if values.is_empty() {
            findings.push(Finding::Absent { field: field.name });
        } else if field.once && values.len() > 1 {
            findings.push(Finding::Repeated { field: field.name });
        }
        findings.extend(
            values
                .iter()
                .filter(|value| !(field.valid)(value))
                .map(|&value| Finding::Invalid {
                    field: field.name,
                    value: value.to_owned(),
                }),
        );
    }
    let expires = fields
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("Expires"))
        .and_then(|&(_, value)| epoch_seconds(value));
    match expires {
        Some(expires) if expires <= now => findings.push(Finding::Expired { expires, now }),
        Some(expires) if expires - now >= YEAR => findings.push(Finding::TooFar { expires, now }),
        _ => {}
    }
    findings
}

/// Whether `value` is an HTTPS address.
fn is_https(value: &str) -> bool {
    value.starts_with("https://")
}

/// Whether `value` is a comma-separated list of language tags.
fn is_language_list(value: &str) -> bool {
    value.split(',').all(|tag| {
        let tag = tag.trim();
        !tag.is_empty() && tag.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
    })
}

/// The time `value`, written `YYYY-MM-DDTHH:MM:SSZ` (RFC 3339 in UTC, the
/// form `security.txt` uses), in seconds since the epoch; `None` for any
/// other text or a time before 1970.
fn epoch_seconds(value: &str) -> Option<u64> {
    const PATTERN: &[u8; 20] = b"0000-00-00T00:00:00Z";
    let bytes = value.as_bytes();
    let fits = bytes.len() == PATTERN.len()
        && bytes.iter().zip(PATTERN).all(|(&byte, &expected)| {
            if expected == b'0' {
                byte.is_ascii_digit()
            } else {
                byte == expected
            }
        });
    if !fits {
        return None;
    }
    let number = |start: usize, len: usize| {
        bytes
            .iter()
            .skip(start)
            .take(len)
            .fold(0, |total, &digit| total * 10 + u64::from(digit - b'0'))
    };
    let (year, month, day) = (number(0, 4), number(5, 2), number(8, 2));
    let (hour, minute, second) = (number(11, 2), number(14, 2), number(17, 2));
    let valid = year >= 1970
        && (1..=12).contains(&month)
        && (1..=days_in_month(year, month)).contains(&day)
        && hour < 24
        && minute < 60
        && second < 60;
    valid.then(|| ((days_since_epoch(year, month, day) * 24 + hour) * 60 + minute) * 60 + second)
}

/// The number of days in `month` (1 to 12) of `year`.
fn days_in_month(year: u64, month: u64) -> u64 {
    const DAYS: [u64; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    DAYS.iter()
        .zip(1..)
        .find(|&(_, number)| number == month)
        .map_or(0, |(&days, _)| days)
        + u64::from(leap && month == 2)
}

/// The days from 1970-01-01 to the valid date `year`-`month`-`day`, from
/// 1970, by Howard Hinnant's `days_from_civil` with March as the first
/// month of the year.
fn days_since_epoch(year: u64, month: u64, day: u64) -> u64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year / 400;
    let year_of_era = year - era * 400;
    let day_of_year = (153 * ((month + 9) % 12) + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

/// A header the check holds to a rule wherever `_headers` sends it.
struct Header {
    /// Its name.
    name: &'static str,
    /// Whether a value meets the rule.
    strong: fn(&str) -> bool,
}

/// The headers every page must get, and their rules.
const RULES: [Header; 2] = [
    Header {
        name: "Strict-Transport-Security",
        strong: is_strong_hsts,
    },
    Header {
        name: "Content-Security-Policy",
        strong: is_same_origin_policy,
    },
];

/// The findings for `_headers`, whose text is `text`. A line that does
/// not start with whitespace is a path pattern; an indented line is a
/// header for the pattern above it.
fn headers(text: &str) -> Vec<Finding> {
    let mut findings = Vec::new();
    let mut path = None;
    let mut everywhere = [false; RULES.len()];
    for (index, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if !line.starts_with(char::is_whitespace) {
            path = Some(trimmed);
            continue;
        }
        let Some((path, (name, value))) = path.zip(trimmed.split_once(':')) else {
            findings.push(Finding::Malformed {
                path: HEADERS.to_owned(),
                line: index + 1,
            });
            continue;
        };
        let value = value.trim();
        for (rule, sent) in RULES.iter().zip(&mut everywhere) {
            if name.trim().eq_ignore_ascii_case(rule.name) {
                *sent |= path == EVERY_PAGE;
                if !(rule.strong)(value) {
                    findings.push(Finding::WeakHeader {
                        header: rule.name,
                        path: path.to_owned(),
                        value: value.to_owned(),
                    });
                }
            }
        }
    }
    findings.extend(
        RULES
            .iter()
            .zip(everywhere)
            .filter(|&(_, sent)| !sent)
            .map(|(rule, _)| Finding::NoHeader { header: rule.name }),
    );
    findings
}

/// Whether the `Strict-Transport-Security` value `value` lasts at least two
/// years and covers subdomains and the preload list.
fn is_strong_hsts(value: &str) -> bool {
    let directives: Vec<String> = value
        .split(';')
        .map(|directive| directive.trim().to_ascii_lowercase())
        .collect();
    let has = |wanted: &str| directives.iter().any(|directive| directive == wanted);
    let long = directives.iter().any(|directive| {
        directive
            .strip_prefix("max-age=")
            .and_then(|age| age.parse::<u64>().ok())
            .is_some_and(|age| age >= HSTS_MAX_AGE)
    });
    long && has("includesubdomains") && has("preload")
}

/// Whether the `Content-Security-Policy` value `value` starts from
/// `default-src 'none'` and names no source but `'none'` and `'self'`.
fn is_same_origin_policy(value: &str) -> bool {
    let directives: Vec<Vec<&str>> = value
        .split(';')
        .map(|directive| directive.split_whitespace().collect())
        .collect();
    directives
        .iter()
        .any(|directive| *directive == ["default-src", "'none'"])
        && directives
            .iter()
            .flat_map(|directive| directive.iter().skip(1))
            .all(|source| *source == "'none'" || *source == "'self'")
}

/// The findings for the page `path`, whose text is `html`.
fn page(path: &str, html: &str) -> Vec<Finding> {
    let Some(tags) = start_tags(html) else {
        return vec![Finding::Unparsable {
            path: path.to_owned(),
        }];
    };
    let mut findings = Vec::new();
    for tag in &tags {
        if CODE.contains(&tag.name.as_str()) {
            findings.push(Finding::Code {
                path: path.to_owned(),
                element: tag.name.clone(),
            });
        }
        for (attribute, value) in &tag.attributes {
            let address = as_address(value);
            if attribute.starts_with("on") || address.starts_with("javascript:") {
                findings.push(Finding::Script {
                    path: path.to_owned(),
                    element: tag.name.clone(),
                    attribute: attribute.clone(),
                });
            } else if value.contains('&') {
                findings.push(Finding::Reference {
                    path: path.to_owned(),
                    element: tag.name.clone(),
                    attribute: attribute.clone(),
                });
            } else if !tag.is_link(attribute) && names_another_origin(&address) {
                findings.push(Finding::ThirdParty {
                    path: path.to_owned(),
                    element: tag.name.clone(),
                    attribute: attribute.clone(),
                    value: value.clone(),
                });
            }
        }
    }
    let links_notice = tags.iter().any(|tag| {
        tag.attributes
            .iter()
            .any(|(attribute, value)| tag.is_link(attribute) && value == PRIVACY_LINK)
    });
    if !links_notice {
        findings.push(Finding::NoPrivacyLink {
            path: path.to_owned(),
        });
    }
    findings
}

/// The attribute value `value` as a browser reads an address: without
/// surrounding whitespace or any tab or line break, with `\` read as `/`,
/// and lowercased, so a scheme or `//` split by those characters is seen.
fn as_address(value: &str) -> String {
    value
        .trim()
        .chars()
        .filter(|&c| !matches!(c, '\t' | '\n' | '\r'))
        .map(|c| {
            if c == '\\' {
                '/'
            } else {
                c.to_ascii_lowercase()
            }
        })
        .collect()
}

/// Whether `address`, read by [`as_address`], may name anywhere but the
/// site's own origin: it holds `//` or starts with an HTTP scheme.
fn names_another_origin(address: &str) -> bool {
    let own = address == ORIGIN || address.starts_with(&format!("{ORIGIN}/"));
    let absolute =
        address.contains("//") || address.starts_with("http:") || address.starts_with("https:");
    absolute && !own
}

/// A start tag: its lowercase name and its attributes, each a lowercase
/// name and the value as written.
#[derive(Debug, PartialEq, Eq)]
struct Tag {
    /// The element's name.
    name: String,
    /// The attributes, in order.
    attributes: Vec<(String, String)>,
}

impl Tag {
    /// Whether `attribute` of this tag is a link a person follows, which
    /// loads nothing until they do.
    fn is_link(&self, attribute: &str) -> bool {
        self.name == "a" && attribute == "href"
    }
}

/// The start tags in `html`, skipping comments, end tags and declarations;
/// `None` when a tag, quoted value or comment is not terminated.
fn start_tags(html: &str) -> Option<Vec<Tag>> {
    let mut tags = Vec::new();
    let mut rest = html;
    while let Some((_, after)) = rest.split_once('<') {
        rest = if let Some(comment) = after.strip_prefix("!--") {
            comment.split_once("-->")?.1
        } else {
            let (tag, after) = tag(after)?;
            tags.extend(tag);
            after
        };
    }
    Some(tags)
}

/// The tag that `text`, which follows a `<`, starts, and the text after
/// its `>`. An end tag or declaration has no name, so it gives no tag.
fn tag(text: &str) -> Option<(Option<Tag>, &str)> {
    let (name, mut rest) = split_at_first(text, |c| !(c.is_ascii_alphanumeric() || c == '-'));
    let mut attributes = Vec::new();
    loop {
        rest = rest.trim_start_matches(|c: char| c.is_ascii_whitespace() || c == '/');
        if let Some(after) = rest.strip_prefix('>') {
            let tag = (!name.is_empty()).then(|| Tag {
                name: name.to_ascii_lowercase(),
                attributes,
            });
            return Some((tag, after));
        }
        if rest.is_empty() {
            return None;
        }
        let (attribute, after) = split_at_first(rest, |c| {
            c.is_ascii_whitespace() || c == '=' || c == '>' || c == '/'
        });
        let (value, after) = attribute_value(after)?;
        attributes.push((attribute.to_ascii_lowercase(), value.to_owned()));
        rest = after;
    }
}

/// The value of the attribute whose name `text` follows, and the text
/// after it: empty when there is no `=`, else a double- or single-quoted
/// or unquoted value. `None` when a quote is not closed.
fn attribute_value(text: &str) -> Option<(&str, &str)> {
    let Some(after) = text.trim_start().strip_prefix('=') else {
        return Some(("", text));
    };
    let after = after.trim_start();
    match after.chars().next() {
        Some(quote @ ('"' | '\'')) => after[1..].split_once(quote),
        _ => Some(split_at_first(after, |c| {
            c.is_ascii_whitespace() || c == '>'
        })),
    }
}

/// `text` split before the first character for which `stop` holds, or not
/// split when there is none.
fn split_at_first(text: &str, stop: impl Fn(char) -> bool) -> (&str, &str) {
    text.split_at(text.find(stop).unwrap_or(text.len()))
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::{Finding, Tag, check, epoch_seconds, start_tags};
    use crate::ROOT;
    use crate::tree::memory::Memory;
    use crate::tree::{Disk, Tree};

    /// 2026-10-03T00:00:00Z, in seconds since the epoch.
    const NOW: u64 = 1_790_985_600;

    /// 2027-09-30T00:00:00Z, the `Expires` of [`SECURITY_TXT`].
    const EXPIRES: u64 = 1_822_262_400;

    /// 365 days, in seconds.
    const YEAR: u64 = 31_536_000;

    /// A `security.txt` with every field, two contacts, comments, a blank
    /// line, mixed-case names and an extra field the check ignores.
    const SECURITY_TXT: &str = "\
# A comment line.

Contact: https://github.com/PremierStudio/gunmetal/security/advisories/new
contact: https://github.com/itz4blitz
Expires: 2027-09-30T00:00:00Z
Policy: https://github.com/PremierStudio/gunmetal/blob/main/SECURITY.md
Canonical: https://gunmetal.tv/.well-known/security.txt
PREFERRED-LANGUAGES: en, en-GB
Hiring: anything else is ignored
";

    /// Headers for every page, and one more path with a header of its own.
    const HEADERS: &str = "\
# Every page.
/*
  Strict-Transport-Security: max-age=63072000; includeSubDomains; preload
  Content-Security-Policy: default-src 'none'; img-src 'self'; frame-ancestors 'none'
  Referrer-Policy: no-referrer

/invite/*
  Cache-Control: no-store
";

    /// A page that links to the privacy notice and to another site.
    const PAGE: &str = "<!doctype html>\n<html lang=\"en\"><!-- <script> in a comment never runs -->\n<a href=\"/privacy/\">Privacy</a> <a href='https://github.com/PremierStudio/gunmetal'>Source</a></html>\n";

    /// A site with every required file, each passing every check, with the
    /// files in `overrides` added or replaced.
    fn site(overrides: &[(&str, &str)]) -> Memory {
        let base = Memory::default()
            .with("site/.well-known/security.txt", SECURITY_TXT)
            .with("site/_headers", HEADERS)
            .with("site/invite/index.html", PAGE)
            .with("site/privacy/index.html", PAGE);
        overrides
            .iter()
            .fold(base, |tree, &(path, text)| tree.with(path, text))
    }

    /// The findings for the complete site with `security.txt` replaced.
    fn security(text: &str) -> Vec<Finding> {
        check(&site(&[("site/.well-known/security.txt", text)]), NOW)
    }

    /// The findings for the complete site with `_headers` replaced.
    fn headers(text: &str) -> Vec<Finding> {
        check(&site(&[("site/_headers", text)]), NOW)
    }

    /// The findings for the complete site with the invitation landing page
    /// replaced.
    fn invite(html: &str) -> Vec<Finding> {
        check(&site(&[("site/invite/index.html", html)]), NOW)
    }

    fn missing(path: &str) -> Finding {
        Finding::Missing {
            path: path.to_owned(),
        }
    }

    fn unexpected(path: &str) -> Finding {
        Finding::Unexpected {
            path: path.to_owned(),
        }
    }

    fn invalid(field: &'static str, value: &str) -> Finding {
        Finding::Invalid {
            field,
            value: value.to_owned(),
        }
    }

    fn weak(header: &'static str, path: &str, value: &str) -> Finding {
        Finding::WeakHeader {
            header,
            path: path.to_owned(),
            value: value.to_owned(),
        }
    }

    fn malformed(path: &str, line: usize) -> Finding {
        Finding::Malformed {
            path: path.to_owned(),
            line,
        }
    }

    fn third_party(element: &str, attribute: &str, value: &str) -> Finding {
        Finding::ThirdParty {
            path: "site/invite/index.html".to_owned(),
            element: element.to_owned(),
            attribute: attribute.to_owned(),
            value: value.to_owned(),
        }
    }

    fn tag(name: &str, attributes: &[(&str, &str)]) -> Tag {
        Tag {
            name: name.to_owned(),
            attributes: attributes
                .iter()
                .map(|&(attribute, value)| (attribute.to_owned(), value.to_owned()))
                .collect(),
        }
    }

    /// The real site, checked at the time the test runs: the gate enforces
    /// every rule below on every change, and fails once `security.txt`
    /// expires or is renewed for a year or more.
    ///
    /// Verifies: SEC-SUP-008, SEC-STD-016, SEC-PRV-054, SEC-PRV-055, SEC-HIS-061
    #[test]
    fn the_project_site_passes_every_check() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |since| since.as_secs());
        assert_eq!(check(&Disk::new(ROOT), now), []);
    }

    #[test]
    fn the_site_subcommand_checks_the_site_beneath_the_root() {
        let fixtures = concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures");
        let mut out = Vec::new();
        assert_eq!(
            crate::dispatch(&["site".to_owned()], fixtures, NOW, &[], &mut out),
            Err(crate::Failure::Findings(vec![
                r#"Missing { path: "site/.well-known/security.txt" }"#.to_owned(),
                r#"Missing { path: "site/_headers" }"#.to_owned(),
                r#"Missing { path: "site/privacy/index.html" }"#.to_owned(),
                r#"Missing { path: "site/invite/index.html" }"#.to_owned(),
            ]))
        );
        assert_eq!(out, b"");
    }

    #[test]
    fn a_complete_site_has_no_findings() {
        assert_eq!(check(&site(&[]), NOW), []);
    }

    /// Verifies: SEC-SUP-008, SEC-PRV-054
    #[test]
    fn the_site_needs_its_security_files_privacy_notice_and_landing_page() {
        assert_eq!(
            check(&Memory::default(), NOW),
            [
                missing("site/.well-known/security.txt"),
                missing("site/_headers"),
                missing("site/privacy/index.html"),
                missing("site/invite/index.html"),
            ]
        );
    }

    /// Verifies: SEC-HIS-061
    #[test]
    fn the_site_holds_only_static_pages_and_text() {
        let tree = site(&[
            ("site/.htaccess", "RewriteEngine On\n"),
            ("site/about/index.html", PAGE),
            ("site/app.js", "fetch('/api')\n"),
            ("site/functions/api.ts", "export default {}\n"),
            ("site/invite/_headers", HEADERS),
            ("site/robots.txt", "User-agent: *\n"),
        ]);
        assert_eq!(
            check(&tree, NOW),
            [
                unexpected("site/.htaccess"),
                unexpected("site/app.js"),
                unexpected("site/functions/api.ts"),
                unexpected("site/invite/_headers"),
            ]
        );
    }

    /// A tree with one page that is not UTF-8 text.
    struct Binary;

    impl Tree for Binary {
        fn read(&self, _: &str) -> Option<String> {
            None
        }

        fn files(&self, dir: &str) -> Vec<String> {
            assert_eq!(dir, "site");
            vec!["privacy/index.html".to_owned()]
        }
    }

    #[test]
    fn a_file_that_is_not_text_is_unexpected() {
        assert_eq!(
            check(&Binary, NOW),
            [
                missing("site/.well-known/security.txt"),
                missing("site/_headers"),
                missing("site/invite/index.html"),
                unexpected("site/privacy/index.html"),
            ]
        );
    }

    /// Verifies: SEC-SUP-008
    #[test]
    fn security_txt_needs_every_required_field() {
        assert_eq!(
            security("# Nothing but a comment.\n"),
            [
                Finding::Absent { field: "Contact" },
                Finding::Absent { field: "Expires" },
                Finding::Absent { field: "Policy" },
                Finding::Absent { field: "Canonical" },
                Finding::Absent {
                    field: "Preferred-Languages"
                },
            ]
        );
    }

    /// Verifies: SEC-SUP-008
    #[test]
    fn security_txt_addresses_use_https_and_name_the_canonical_location() {
        let text = "\
Contact: http://example.com/report
Contact: https://github.com/itz4blitz
expires: 2027-09-30T00:00:00Z
POLICY: mailto:security@example.com
Canonical: https://gunmetal.tv/.well-known/security.txt
Canonical: https://example.com/.well-known/security.txt
Preferred-Languages: en,
";
        assert_eq!(
            security(text),
            [
                invalid("Contact", "http://example.com/report"),
                invalid("Policy", "mailto:security@example.com"),
                invalid("Canonical", "https://example.com/.well-known/security.txt"),
                invalid("Preferred-Languages", "en,"),
            ]
        );
    }

    #[test]
    fn security_txt_languages_are_a_list_of_tags() {
        for (languages, findings) in [
            ("en", vec![]),
            ("en,de-AT, fr", vec![]),
            ("", vec![invalid("Preferred-Languages", "")]),
            ("en, ,de", vec![invalid("Preferred-Languages", "en, ,de")]),
            ("en_GB", vec![invalid("Preferred-Languages", "en_GB")]),
            ("en de", vec![invalid("Preferred-Languages", "en de")]),
        ] {
            let text = SECURITY_TXT.replace("en, en-GB", languages);
            assert_eq!(security(&text), findings);
        }
    }

    /// Verifies: SEC-SUP-008
    #[test]
    fn security_txt_names_its_expiry_and_languages_once() {
        let text =
            format!("{SECURITY_TXT}Expires: 2027-01-15T08:09:10Z\nPreferred-Languages: de\n");
        assert_eq!(
            security(&text),
            [
                Finding::Repeated { field: "Expires" },
                Finding::Repeated {
                    field: "Preferred-Languages"
                },
            ]
        );
    }

    #[test]
    fn security_txt_lines_need_a_name_and_a_value() {
        let text = format!("{SECURITY_TXT}   \n  # An indented comment.\nno colon here\n");
        assert_eq!(
            security(&text),
            [malformed("site/.well-known/security.txt", 12)]
        );
    }

    /// Verifies: SEC-SUP-008
    #[test]
    fn security_txt_expires_in_the_future_and_less_than_a_year_ahead() {
        let at = |now| check(&site(&[]), now);
        assert_eq!(at(EXPIRES - 1), []);
        assert_eq!(
            at(EXPIRES),
            [Finding::Expired {
                expires: EXPIRES,
                now: EXPIRES
            }]
        );
        assert_eq!(
            at(EXPIRES + 1),
            [Finding::Expired {
                expires: EXPIRES,
                now: EXPIRES + 1
            }]
        );
        assert_eq!(at(EXPIRES - YEAR + 1), []);
        assert_eq!(
            at(EXPIRES - YEAR),
            [Finding::TooFar {
                expires: EXPIRES,
                now: EXPIRES - YEAR
            }]
        );
    }

    #[test]
    fn security_txt_expires_is_an_rfc_3339_time_in_utc() {
        let text = SECURITY_TXT.replace("2027-09-30T00:00:00Z", "30 September 2027");
        assert_eq!(security(&text), [invalid("Expires", "30 September 2027")]);
    }

    #[test]
    fn reads_utc_times_as_seconds_since_the_epoch() {
        for (time, seconds) in [
            ("1970-01-01T00:00:00Z", 0),
            ("1970-01-01T00:00:01Z", 1),
            ("2000-02-29T12:34:56Z", 951_827_696),
            ("2026-10-03T00:00:00Z", 1_790_985_600),
            ("2027-01-15T08:09:10Z", 1_800_000_550),
            ("2027-01-31T00:00:00Z", 1_801_353_600),
            ("2027-04-30T00:00:00Z", 1_809_043_200),
            ("2027-09-30T00:00:00Z", 1_822_262_400),
            ("2027-12-31T00:00:00Z", 1_830_211_200),
            ("2028-02-29T23:59:59Z", 1_835_481_599),
            ("2100-03-01T00:00:00Z", 4_107_542_400),
            ("2399-02-28T00:00:00Z", 13_542_940_800),
            ("2400-12-31T23:59:59Z", 13_601_087_999),
        ] {
            assert_eq!((time, epoch_seconds(time)), (time, Some(seconds)));
        }
    }

    #[test]
    fn refuses_anything_but_a_valid_utc_time_from_1970() {
        for time in [
            "",
            "2027-09-30T00:00:00",
            "2027-09-30T00:00:00Z ",
            "2027-09-30 00:00:00Z",
            "2027/09/30T00:00:00Z",
            "2027-09/30T00:00:00Z",
            "2027-09-30T00-00:00Z",
            "2027-09-30T00:00-00Z",
            "2027-09-30T00:00:00z",
            "2O27-09-30T00:00:00Z",
            "+027-09-30T00:00:00Z",
            "2027-09-30T00:00:0\u{e9}",
            "1969-12-31T23:59:59Z",
            "2027-00-10T00:00:00Z",
            "2027-13-10T00:00:00Z",
            "2027-09-00T00:00:00Z",
            "2027-01-32T00:00:00Z",
            "2027-04-31T00:00:00Z",
            "2027-09-31T00:00:00Z",
            "2027-02-29T00:00:00Z",
            "2100-02-29T00:00:00Z",
            "2028-02-30T00:00:00Z",
            "2027-09-30T24:00:00Z",
            "2027-09-30T23:60:00Z",
            "2027-09-30T23:59:60Z",
        ] {
            assert_eq!((time, epoch_seconds(time)), (time, None));
        }
    }

    /// Verifies: SEC-STD-016
    #[test]
    fn every_page_gets_strict_transport_security_and_a_same_origin_policy() {
        let both = [
            Finding::NoHeader {
                header: "Strict-Transport-Security",
            },
            Finding::NoHeader {
                header: "Content-Security-Policy",
            },
        ];
        assert_eq!(headers("# No headers at all.\n"), both);
        let one_path = "\
/invite/*
  Strict-Transport-Security: max-age=63072000; includeSubDomains; preload
  Content-Security-Policy: default-src 'none'
";
        assert_eq!(headers(one_path), both);
    }

    /// Verifies: SEC-STD-016
    #[test]
    fn strict_transport_security_lasts_two_years_with_subdomains_and_preload() {
        for value in [
            "max-age=63072000; includeSubDomains; preload",
            "MAX-AGE=94608000;INCLUDESUBDOMAINS;Preload",
            " preload ; includeSubDomains ; max-age=63072001 ",
        ] {
            let text = format!(
                "/*\n  strict-transport-security: {value}\n  Content-Security-Policy: default-src 'none'\n"
            );
            assert_eq!(headers(&text), []);
        }
        for value in [
            "max-age=63071999; includeSubDomains; preload",
            "max-age=63072000; preload",
            "max-age=63072000; includeSubDomains",
            "includeSubDomains; preload",
            "max-age=two-years; includeSubDomains; preload",
            "max-age=63072000 includeSubDomains preload",
        ] {
            let text = format!(
                "/*\n  Strict-Transport-Security: {value}\n  Content-Security-Policy: default-src 'none'\n"
            );
            assert_eq!(
                headers(&text),
                [weak("Strict-Transport-Security", "/*", value)]
            );
        }
    }

    /// Verifies: SEC-PRV-054
    #[test]
    fn the_content_security_policy_allows_no_other_origin() {
        let sts = "Strict-Transport-Security: max-age=63072000; includeSubDomains; preload";
        for value in [
            "default-src 'none'",
            "default-src 'none'; img-src 'self'; frame-ancestors 'none'; upgrade-insecure-requests",
        ] {
            let text = format!("/*\n  {sts}\n  content-security-policy: {value}\n");
            assert_eq!(headers(&text), []);
        }
        for value in [
            "default-src 'self'",
            "img-src 'self'",
            "default-src 'none' 'self'",
            "default-src 'none'; img-src https://cdn.example",
            "default-src 'none'; script-src 'unsafe-inline'",
            "default-src 'none'; style-src *",
        ] {
            let text = format!("/*\n  {sts}\n  Content-Security-Policy: {value}\n");
            assert_eq!(
                headers(&text),
                [weak("Content-Security-Policy", "/*", value)]
            );
        }
    }

    /// Verifies: SEC-STD-016
    #[test]
    fn a_weaker_header_for_one_path_is_refused_too() {
        let text = format!("{HEADERS}/old/*\n  Strict-Transport-Security: max-age=0\n");
        assert_eq!(
            headers(&text),
            [weak("Strict-Transport-Security", "/old/*", "max-age=0")]
        );
    }

    #[test]
    fn headers_follow_a_path_and_have_a_name_and_a_value() {
        let text = format!("  Referrer-Policy: no-referrer\n{HEADERS}  no colon here\n");
        assert_eq!(
            headers(&text),
            [
                malformed("site/_headers", 1),
                malformed("site/_headers", 10)
            ]
        );
    }

    /// Verifies: SEC-PRV-055
    #[test]
    fn no_page_runs_script() {
        let html = "\
<a href=/privacy/>Privacy</a>
<script>window.secret = location.hash</script>
<SCRIPT src=/app.js></SCRIPT>
<style>body { background: url(/a.png) }</style>
<style-guide>a custom element</style-guide>
<details open>an attribute that starts with o</details>
<button onclick=\"send(location.hash)\">Go</button>
<a href=\" JavaScript:send(location.hash)\">Go</a>
<a href=\"java\tscript:send(location.hash)\">Go</a>
<a href=\"\njava\nscript\r:send(location.hash)\">Go</a>
";
        let script = |element: &str, attribute: &str| Finding::Script {
            path: "site/invite/index.html".to_owned(),
            element: element.to_owned(),
            attribute: attribute.to_owned(),
        };
        let code = |element: &str| Finding::Code {
            path: "site/invite/index.html".to_owned(),
            element: element.to_owned(),
        };
        assert_eq!(
            invite(html),
            [
                code("script"),
                code("script"),
                code("style"),
                script("button", "onclick"),
                script("a", "href"),
                script("a", "href"),
                script("a", "href"),
            ]
        );
    }

    /// Verifies: SEC-PRV-054
    #[test]
    fn pages_load_nothing_from_another_origin() {
        let html = "\
<a href=/privacy/>Privacy</a>
<a href=\"https://github.com/PremierStudio/gunmetal\">Links may leave the site</a>
<link rel=canonical href=https://gunmetal.tv>
<img src=\"https://gunmetal.tv/icon.png\" alt=\"\">
<img src=\"https://analytics.example/pixel.gif\">
<img src=//cdn.example/a.png>
<link rel=stylesheet href='HTTPS://fonts.example/css'>
<img srcset=\"https://gunmetal.tv.example/a.png 2x\">
<meta http-equiv=refresh content=\"0; url=http://example.com/\">
<area href=\"https://tracker.example/\">
<img src=\"/\\evil.example/a.png\">
<img src=\"\\\\evil.example/b.png\">
<img src=\"/\n/evil.example/c.png\">
<img src=\"/\t/evil.example/d.png\">
<img src=\"/\r/evil.example/e.png\">
<img src=\"Http:evil.example/f.png\">
<img src=\"https:evil.example/g.png\">
<img src=\"https://gunmetal.tv@evil.example/h.png\">
<img src=\"/images/a.png\" alt=\"A picture: the logo\">
";
        assert_eq!(
            invite(html),
            [
                third_party("img", "src", "https://analytics.example/pixel.gif"),
                third_party("img", "src", "//cdn.example/a.png"),
                third_party("link", "href", "HTTPS://fonts.example/css"),
                third_party("img", "srcset", "https://gunmetal.tv.example/a.png 2x"),
                third_party("meta", "content", "0; url=http://example.com/"),
                third_party("area", "href", "https://tracker.example/"),
                third_party("img", "src", "/\\evil.example/a.png"),
                third_party("img", "src", "\\\\evil.example/b.png"),
                third_party("img", "src", "/\n/evil.example/c.png"),
                third_party("img", "src", "/\t/evil.example/d.png"),
                third_party("img", "src", "/\r/evil.example/e.png"),
                third_party("img", "src", "Http:evil.example/f.png"),
                third_party("img", "src", "https:evil.example/g.png"),
                third_party("img", "src", "https://gunmetal.tv@evil.example/h.png"),
            ]
        );
    }

    #[test]
    fn an_attribute_value_with_a_character_reference_is_refused() {
        let html = "\
<a href=/privacy/ title=\"Privacy\">Privacy &amp; data</a>
<img src=\"&#47;&#47;evil.example/a.png\">
<a href=\"&#106;avascript:send(location.hash)\">Go</a>
<img alt=\"Fish &amp; chips\" src=/a.png>
";
        let reference = |element: &str, attribute: &str| Finding::Reference {
            path: "site/invite/index.html".to_owned(),
            element: element.to_owned(),
            attribute: attribute.to_owned(),
        };
        assert_eq!(
            invite(html),
            [
                reference("img", "src"),
                reference("a", "href"),
                reference("img", "alt"),
            ]
        );
    }

    /// Verifies: SEC-PRV-054
    #[test]
    fn every_page_links_to_the_privacy_notice() {
        let html = "<a href=\"/privacy\">No slash</a><link href=\"/privacy/\"><a title=\"/privacy/\">Title</a>";
        assert_eq!(
            invite(html),
            [Finding::NoPrivacyLink {
                path: "site/invite/index.html".to_owned()
            }]
        );
    }

    #[test]
    fn a_page_with_an_unterminated_tag_or_comment_is_refused() {
        for tail in [
            "<a href=\"x",
            "<!-- open",
            "<img src='x",
            "<img src=x",
            "<img",
        ] {
            assert_eq!(
                invite(&format!("<a href=/privacy/>Privacy</a>{tail}")),
                [Finding::Unparsable {
                    path: "site/invite/index.html".to_owned()
                }]
            );
        }
    }

    #[test]
    fn start_tags_reads_names_and_attributes_in_every_form() {
        let html = "<!DOCTYPE html><P>text</p><!-- <b> --><x-card Data-A=\"1 > 2\" b='single' c=plain d = \"spaced\" e/><br/><>< img><a =x><a\nhref=/privacy/>";
        assert_eq!(
            start_tags(html),
            Some(vec![
                tag("p", &[]),
                tag(
                    "x-card",
                    &[
                        ("data-a", "1 > 2"),
                        ("b", "single"),
                        ("c", "plain"),
                        ("d", "spaced"),
                        ("e", ""),
                    ]
                ),
                tag("br", &[]),
                tag("a", &[("", "x")]),
                tag("a", &[("href", "/privacy/")]),
            ])
        );
        assert_eq!(start_tags("no tags at all"), Some(vec![]));
    }
}
