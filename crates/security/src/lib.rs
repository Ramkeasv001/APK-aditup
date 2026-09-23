//! Authentication, session, Admin-elevation, audit logging, and encryption layer.
//! Module 3 (core auth) + Module 13 (hardening: audit log hash-chain + project keys).
//!
//! `AuthService` is the single entry point for authentication; `audit_log` and
//! `encryption` are independently exposed for Module 13 security hardening.

mod attempt_limiter;
mod audit_log;
mod auth_service;
mod encryption;
mod error;
mod password;
mod recovery;
mod session;

pub use attempt_limiter::AttemptLimiter;
pub use audit_log::{AuditEventType, AuditLogEntry, AuditResult};
pub use auth_service::AuthService;
pub use encryption::{AES256GCMProvider, EncryptionError, EncryptionProvider, ProjectEncryptionKey};
pub use error::SecurityError;
pub use password::{hash_password, verify_password};
pub use recovery::{generate_recovery_key, verify_recovery_key};
pub use session::SessionVault;
