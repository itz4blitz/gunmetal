//! Gunmetal's egress door: the rules every outbound request passes.
//!
//! The server reaches out for a short, closed list of reasons, one per row
//! of the egress inventory in the threat model
//! (docs/security/threat-model.md). Each request names its
//! [`purpose::Purpose`], and the [`gate::Gate`] decides, before a name is
//! resolved or a socket opened, whether that purpose may reach that
//! destination (SEC-TM-048, SEC-API-079):
//!
//! - the [`grant::Configuration`] holds what the owner granted; by default
//!   that is nothing, and offline mode refuses everything (SEC-PRV-012,
//!   SEC-PRV-013);
//! - a request goes only to a [`destination::Destination`] its purpose's
//!   grant names exactly, as scheme, host and port;
//! - [`address::pin`] checks every address a name resolved to, and the
//!   request may connect only to the addresses it returns (SEC-EXT-002,
//!   SEC-API-077);
//! - [`redirect::follow`] decides whether a redirect is followed
//!   (SEC-EXT-003), and [`limits::Limits`] caps time and size (SEC-EXT-004);
//! - every refusal sends one security event to the audit log's sink, and
//!   every attempt is kept for the network activity page (SEC-PRV-008).
//!
//! This crate holds the decisions only. The client that resolves names,
//! connects, speaks TLS and HTTP and applies these decisions joins it when
//! its crates are accepted; until then nothing here opens a socket.

pub mod address;
pub mod denial;
pub mod destination;
pub mod gate;
pub mod grant;
pub mod limits;
pub mod purpose;
pub mod redirect;
