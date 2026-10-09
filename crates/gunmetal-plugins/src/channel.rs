//! The identity the server fixes on a plugin's IPC channel.
//!
//! A message is from the plugin the server recorded at spawn, whatever the
//! message claims (SEC-EXT-021).

use crate::id::PluginId;

/// Why a claimed identity is refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClaimError {
    /// The message named a plugin other than the one fixed at spawn.
    Mismatch {
        /// The plugin the server recorded.
        fixed: String,
        /// The plugin the message named.
        claimed: String,
    },
}

/// Accepts a claim only when it is the identity fixed at spawn.
///
/// # Errors
///
/// [`ClaimError::Mismatch`] when `claimed` is not `fixed`.
pub fn accept(fixed: &PluginId, claimed: &str) -> Result<(), ClaimError> {
    if claimed == fixed.as_str() {
        Ok(())
    } else {
        Err(ClaimError::Mismatch {
            fixed: fixed.as_str().to_owned(),
            claimed: claimed.to_owned(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{ClaimError, accept};
    use crate::id::PluginId;

    /// Verifies: SEC-EXT-021
    #[test]
    fn a_message_claiming_another_plugin_is_rejected() {
        let fixed = PluginId::parse("org.listenbrainz.scrobbler").expect("id");
        assert_eq!(accept(&fixed, "org.listenbrainz.scrobbler"), Ok(()));
        assert_eq!(
            accept(&fixed, "org.example.other"),
            Err(ClaimError::Mismatch {
                fixed: "org.listenbrainz.scrobbler".to_owned(),
                claimed: "org.example.other".to_owned(),
            })
        );
        assert_eq!(
            accept(&fixed, ""),
            Err(ClaimError::Mismatch {
                fixed: "org.listenbrainz.scrobbler".to_owned(),
                claimed: String::new(),
            })
        );
    }
}
