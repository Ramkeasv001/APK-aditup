//! Encrypted SQLite (SQLCipher) storage layer — Module 2 of the Phase 2
//! build (see docs/architecture/phase-1-blueprint.md §6).
//!
//! This crate owns: opening/keying/migrating the database, and one
//! repository per aggregate root. It owns no business logic — ranking,
//! scoring, and workflow rules live in later modules' crates and depend on
//! this one, never the other way around.

mod connection;
mod error;
mod migrations;
pub mod repositories;

pub use connection::Database;
pub use error::DbError;
