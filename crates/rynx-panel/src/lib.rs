//! Panel-side identity store: accounts, Argon2id hashes, sessions and RBAC.
//!
//! This database is separate from the daemon state on purpose: a compromise of
//! the web-facing API must not grant write access to the hosting state.

pub mod db;
pub mod packages;
pub mod role;

pub use db::{NewSession, PanelDb, PanelError, SessionContext, UserView};
pub use packages::{AccountPackageView, PackageInput, PackageView, WEB_BACKENDS};
pub use role::Role;
