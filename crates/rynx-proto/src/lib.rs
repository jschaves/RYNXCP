//! RYNXCP control-plane protocol.
//!
//! The API never touches the operating system directly. It speaks this
//! versioned, length-prefixed CBOR protocol to `rynxd` over a local transport.
//! Every frame is authenticated with the daemon token and, on Unix, the daemon
//! additionally verifies peer credentials of the caller.

pub mod model;
pub mod wire;

pub use model::*;
pub use wire::{decode, encode, recv_frame, send_frame, ProtocolError};
