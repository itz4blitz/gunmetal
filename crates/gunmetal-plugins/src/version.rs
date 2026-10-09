//! A plugin version: three numbers, nothing else.
//!
//! Pre-release and build text are refused, so a comparison is total and a
//! lower version cannot be disguised as a higher one (SEC-EXT-040).

/// `major.minor.patch`, each from 0 to 999, with no leading zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PluginVersion {
    major: u16,
    minor: u16,
    patch: u16,
}

/// Why a text is not a version.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VersionError {
    /// It is not three dot-separated components.
    Shape,
    /// A component is not a decimal number, or is above 999.
    Component,
    /// A component has a leading zero.
    LeadingZero,
}

impl PluginVersion {
    /// Reads `major.minor.patch`.
    ///
    /// # Errors
    ///
    /// A [`VersionError`] for any other spelling.
    pub fn parse(text: &str) -> Result<Self, VersionError> {
        let parts: Vec<&str> = text.split('.').collect();
        if parts.len() != 3 {
            return Err(VersionError::Shape);
        }
        Ok(Self {
            major: component(parts[0])?,
            minor: component(parts[1])?,
            patch: component(parts[2])?,
        })
    }

    /// The major component.
    #[must_use]
    pub const fn major(self) -> u16 {
        self.major
    }

    /// The minor component.
    #[must_use]
    pub const fn minor(self) -> u16 {
        self.minor
    }

    /// The patch component.
    #[must_use]
    pub const fn patch(self) -> u16 {
        self.patch
    }
}

/// One component, or why it is not one.
fn component(text: &str) -> Result<u16, VersionError> {
    if text.is_empty() || text.len() > 3 {
        return Err(VersionError::Shape);
    }
    if text.len() > 1 && text.starts_with('0') {
        return Err(VersionError::LeadingZero);
    }
    if !text.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(VersionError::Component);
    }
    let mut value: u16 = 0;
    for byte in text.bytes() {
        let digit = u16::from(byte.saturating_sub(b'0'));
        value = value.saturating_mul(10).saturating_add(digit);
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::{PluginVersion, VersionError};

    /// Verifies: SEC-EXT-040
    #[test]
    fn a_three_part_version_compares_by_its_numbers() {
        let older = PluginVersion::parse("1.2.0").expect("1.2.0");
        let patch = PluginVersion::parse("1.2.1").expect("1.2.1");
        let minor = PluginVersion::parse("1.3.0").expect("1.3.0");
        let major = PluginVersion::parse("2.0.0").expect("2.0.0");
        assert_eq!((older.major(), older.minor(), older.patch()), (1, 2, 0));
        assert!(older < patch);
        assert!(patch < minor);
        assert!(minor < major);
        assert_eq!(older, PluginVersion::parse("1.2.0").expect("again"));
    }

    /// Verifies: SEC-EXT-040
    #[test]
    fn a_version_is_refused_when_it_is_not_three_plain_numbers() {
        let cases = [
            ("", VersionError::Shape),
            ("1", VersionError::Shape),
            ("1.2", VersionError::Shape),
            ("1.2.0.1", VersionError::Shape),
            ("1.2.", VersionError::Shape),
            (".2.0", VersionError::Shape),
            ("01.2.0", VersionError::LeadingZero),
            ("1.02.0", VersionError::LeadingZero),
            ("1.2.03", VersionError::LeadingZero),
            ("1.2.a", VersionError::Component),
            ("v1.2.0", VersionError::Component),
            ("1.2.0-rc.1", VersionError::Shape),
            ("1.2.0+build", VersionError::Shape),
        ];
        for (text, expected) in cases {
            assert_eq!(PluginVersion::parse(text), Err(expected), "{text}");
        }
        assert_eq!(
            PluginVersion::parse("0.0.0").expect("zero"),
            PluginVersion::parse("0.0.0").expect("zero again")
        );
        assert_eq!(
            PluginVersion::parse("999.999.999").expect("max").major(),
            999
        );
    }
}
