//! REAM Security Architecture
//!
//! Basic security system providing:
//! - Environment variables and secrets management
//! - Access control and audit logging
//!
//! This is a foundational implementation that can be extended with
//! consensus storage and advanced cryptographic features.

pub mod basic_security;
pub mod tlisp_integration;

// Re-export main types
pub use basic_security::{
    SecurityManager, SecretType, SecurityClassification,
    SecurityEvent, SecurityEventType, AccessType
};
pub use tlisp_integration::{
    TlispSecurityContext, register_security_functions
};


