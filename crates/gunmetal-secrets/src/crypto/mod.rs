//! The crypto module: with the core's SHA-256 file, the only place that
//! performs cryptography (SEC-STD-018, record 9).

pub mod aead;
pub mod kdf;
pub mod mac;
pub mod nonce;
pub mod passphrase;
