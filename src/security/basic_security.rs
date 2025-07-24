//! Basic Security Implementation for REAM
//!
//! This module provides a foundational security system with:
//! - Environment variables and secrets management
//! - Basic access control
//! - Audit logging
//!
//! This implementation focuses on core functionality and can be extended
//! with advanced cryptographic features and consensus storage later.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::SystemTime;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Types of secrets that can be stored
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecretType {
    DatabaseUrl,
    ApiKey,
    CertificateKey,
    SessionSecret,
    EncryptionKey,
    Custom(String),
}

/// Security classification levels for environment variables
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecurityClassification {
    Public,
    Internal,
    Confidential,
    Secret,
    TopSecret,
}

/// Access types for security policies
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AccessType {
    Read,
    Write,
    Delete,
    Rotate,
}

/// Security event types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecurityEventType {
    SecretStored,
    SecretAccessed,
    SecretRotated,
    SecretExpired,
    EnvVarSet,
    EnvVarAccessed,
    AccessDenied,
    SecurityViolation,
}

/// A secret with metadata (simplified version without zeroize)
#[derive(Debug, Clone)]
pub struct Secret {
    pub name: String,
    pub value: Vec<u8>,
    pub secret_type: SecretType,
    pub created_at: SystemTime,
    pub expires_at: Option<SystemTime>,
}

/// Classified environment variable
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassifiedEnvVar {
    pub name: String,
    pub value: String,
    pub classification: SecurityClassification,
    pub audit_required: bool,
}

/// Security event for audit logging
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityEvent {
    pub event_type: SecurityEventType,
    pub resource: String,
    pub actor: String,
    pub timestamp: SystemTime,
    pub metadata: HashMap<String, String>,
}

/// Access request
#[derive(Debug, Clone)]
pub struct AccessRequest {
    pub actor: String,
    pub resource: String,
    pub action: String,
    pub context: HashMap<String, String>,
}

/// Access decision
#[derive(Debug, Clone)]
pub struct AccessDecision {
    pub allowed: bool,
    pub reason: String,
    pub evaluated_at: SystemTime,
}

/// Security errors
#[derive(Debug, Error)]
pub enum SecurityError {
    #[error("Secret not found: {0}")]
    SecretNotFound(String),
    #[error("Secret expired: {0}")]
    SecretExpired(String),
    #[error("Environment variable not found: {0}")]
    EnvVarNotFound(String),
    #[error("Access denied for {actor} to {resource} with {access_type:?}")]
    AccessDenied {
        resource: String,
        actor: String,
        access_type: AccessType,
    },
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

/// Basic security manager
pub struct SecurityManager {
    /// Stored secrets (in-memory for now)
    secrets: Arc<RwLock<HashMap<String, Secret>>>,
    /// Environment variables with classification
    env_vars: Arc<RwLock<HashMap<String, ClassifiedEnvVar>>>,
    /// Security audit log
    audit_log: Arc<RwLock<Vec<SecurityEvent>>>,
    /// Simple access control (actor -> allowed resources)
    access_control: Arc<RwLock<HashMap<String, Vec<String>>>>,
}

impl SecurityManager {
    /// Create a new security manager
    pub fn new() -> Self {
        SecurityManager {
            secrets: Arc::new(RwLock::new(HashMap::new())),
            env_vars: Arc::new(RwLock::new(HashMap::new())),
            audit_log: Arc::new(RwLock::new(Vec::new())),
            access_control: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Store a secret
    pub fn store_secret(&self, secret: Secret) -> Result<(), SecurityError> {
        let name = secret.name.clone();
        self.secrets.write().unwrap().insert(name.clone(), secret);
        
        // Log security event
        self.log_security_event(SecurityEvent {
            event_type: SecurityEventType::SecretStored,
            resource: name,
            actor: "system".to_string(),
            timestamp: SystemTime::now(),
            metadata: HashMap::new(),
        });
        
        Ok(())
    }

    /// Retrieve a secret
    pub fn retrieve_secret(&self, name: &str, actor: &str) -> Result<Secret, SecurityError> {
        // Check access
        self.check_access(actor, name, AccessType::Read)?;
        
        let secrets = self.secrets.read().unwrap();
        let secret = secrets.get(name)
            .ok_or_else(|| SecurityError::SecretNotFound(name.to_string()))?;
        
        // Check expiration
        if let Some(expires_at) = secret.expires_at {
            if SystemTime::now() > expires_at {
                return Err(SecurityError::SecretExpired(name.to_string()));
            }
        }
        
        // Log access event
        self.log_security_event(SecurityEvent {
            event_type: SecurityEventType::SecretAccessed,
            resource: name.to_string(),
            actor: actor.to_string(),
            timestamp: SystemTime::now(),
            metadata: HashMap::new(),
        });
        
        Ok(secret.clone())
    }

    /// Set a classified environment variable
    pub fn set_env_var(
        &self,
        name: String,
        value: String,
        classification: SecurityClassification,
    ) -> Result<(), SecurityError> {
        let audit_required = matches!(classification, SecurityClassification::Secret | SecurityClassification::TopSecret);
        let env_var = ClassifiedEnvVar {
            name: name.clone(),
            value,
            classification,
            audit_required,
        };
        
        self.env_vars.write().unwrap().insert(name.clone(), env_var);
        
        // Log environment variable creation
        self.log_security_event(SecurityEvent {
            event_type: SecurityEventType::EnvVarSet,
            resource: name,
            actor: "system".to_string(),
            timestamp: SystemTime::now(),
            metadata: HashMap::new(),
        });
        
        Ok(())
    }

    /// Get a classified environment variable
    pub fn get_env_var(&self, name: &str, actor: &str) -> Result<String, SecurityError> {
        // Check access
        self.check_access(actor, name, AccessType::Read)?;
        
        let env_vars = self.env_vars.read().unwrap();
        let env_var = env_vars.get(name)
            .ok_or_else(|| SecurityError::EnvVarNotFound(name.to_string()))?;
        
        // Log access if audit required
        if env_var.audit_required {
            self.log_security_event(SecurityEvent {
                event_type: SecurityEventType::EnvVarAccessed,
                resource: name.to_string(),
                actor: actor.to_string(),
                timestamp: SystemTime::now(),
                metadata: HashMap::new(),
            });
        }
        
        Ok(env_var.value.clone())
    }

    /// Grant access to a resource for an actor
    pub fn grant_access(&self, actor: &str, resource: &str) {
        let mut access_control = self.access_control.write().unwrap();
        access_control.entry(actor.to_string())
            .or_insert_with(Vec::new)
            .push(resource.to_string());
    }

    /// Check if an actor has access to a resource
    pub fn check_access(&self, actor: &str, resource: &str, access_type: AccessType) -> Result<(), SecurityError> {
        let access_control = self.access_control.read().unwrap();
        
        // System actor has access to everything
        if actor == "system" {
            return Ok(());
        }
        
        // Check if actor has access to this resource
        if let Some(allowed_resources) = access_control.get(actor) {
            if allowed_resources.contains(&resource.to_string()) || allowed_resources.contains(&"*".to_string()) {
                return Ok(());
            }
        }
        
        Err(SecurityError::AccessDenied {
            resource: resource.to_string(),
            actor: actor.to_string(),
            access_type,
        })
    }

    /// Log a security event
    fn log_security_event(&self, event: SecurityEvent) {
        self.audit_log.write().unwrap().push(event);
    }

    /// Get audit log
    pub fn get_audit_log(&self) -> Vec<SecurityEvent> {
        self.audit_log.read().unwrap().clone()
    }

    /// Get all stored secrets (for debugging/admin purposes)
    pub fn list_secrets(&self) -> Vec<String> {
        self.secrets.read().unwrap().keys().cloned().collect()
    }

    /// Get all environment variables (for debugging/admin purposes)
    pub fn list_env_vars(&self) -> Vec<String> {
        self.env_vars.read().unwrap().keys().cloned().collect()
    }
}

impl Default for SecurityManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secret_storage_and_retrieval() {
        let security_manager = SecurityManager::new();
        
        // Grant access
        security_manager.grant_access("test_user", "test_secret");
        
        // Store a secret
        let secret = Secret {
            name: "test_secret".to_string(),
            value: b"secret_value".to_vec(),
            secret_type: SecretType::ApiKey,
            created_at: SystemTime::now(),
            expires_at: None,
        };
        
        security_manager.store_secret(secret).unwrap();
        
        // Retrieve the secret
        let retrieved = security_manager.retrieve_secret("test_secret", "test_user").unwrap();
        assert_eq!(retrieved.value, b"secret_value");
        assert_eq!(retrieved.name, "test_secret");
    }

    #[test]
    fn test_env_var_management() {
        let security_manager = SecurityManager::new();
        
        // Grant access
        security_manager.grant_access("test_user", "test_env");
        
        // Set environment variable
        security_manager.set_env_var(
            "test_env".to_string(),
            "test_value".to_string(),
            SecurityClassification::Internal,
        ).unwrap();
        
        // Get environment variable
        let value = security_manager.get_env_var("test_env", "test_user").unwrap();
        assert_eq!(value, "test_value");
    }

    #[test]
    fn test_access_control() {
        let security_manager = SecurityManager::new();
        
        // Try to access without permission
        let result = security_manager.check_access("test_user", "secret_resource", AccessType::Read);
        assert!(result.is_err());
        
        // Grant access
        security_manager.grant_access("test_user", "secret_resource");
        
        // Try again with permission
        let result = security_manager.check_access("test_user", "secret_resource", AccessType::Read);
        assert!(result.is_ok());
    }
}
