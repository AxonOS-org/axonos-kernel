//! HMAC-SHA256 Attestation
//!
//! DC6: Intent attestation via ATECC608B secure element.
//!
//! Each intent observation carries a 32-byte HMAC-SHA256 tag.
//! Keys derived via HKDF-SHA256 from device-unique secret.

pub mod hmac;

pub use hmac::{Attestation, HmacSha256};
