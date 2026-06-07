//! pam-codec: PAM binary format codec for PopCap/PvZ2 animations.
//!
//! Provides types and binary serialization/deserialization for the
//! PAM animation format used in Plants vs. Zombies 2.

pub mod types;
pub mod error;
mod decoder;
mod encoder;

pub use decoder::decode_pam;
pub use encoder::encode_pam;
pub use types::*;
pub use error::{Error, Result};
