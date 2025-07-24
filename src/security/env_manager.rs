//! Secure Environment Variables and Secrets Management
//!
//! This module provides comprehensive management of environment variables and secrets
//! with encryption, access control, and audit logging capabilities.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime};
use serde::{Deserialize, Serialize};
use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce, KeyInit, AeadCore, AeadInPlace};
use ring::rand::{SystemRandom, SecureRandom};
use zeroize::{Zeroize, ZeroizeOnDrop};
use crate::security::SecurityError;

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

/// A secret with metadata
#[derive(Debug, Clone, Zeroize, ZeroizeOnDrop)]
pub struct Secret {
    pub name: String,
    pub value: Vec<u8>,
    pub secret_type: SecretType,
    pub created_at: SystemTime,
    pub expires_at: Option<SystemTime>,
    pub rotation_interval: Option<Duration>,
}

/// Encrypted secret storage format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedSecret {
    pub name: String,
    pub encrypted_data: Vec<u8>,
    pub nonce: Vec<u8>,
    pub secret_type: SecretType,
    pub metadata: SecretMetadata,
    pub access_count: u64,
    pub last_accessed: SystemTime,
}

/// Metadata for secrets
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretMetadata {
    pub created_at: SystemTime,
    pub expires_at: Option<SystemTime>,
    pub rotation_interval: Option<Duration>,
    pub access_policy: String,
    pub encryption_algorithm: String,
    pub key_derivation_params: KeyDerivationParams,
}

/// Key derivation parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyDerivationParams {
    pub algorithm: String,
    pub iterations: u32,
    pub salt: Vec<u8>,
}

impl Default for KeyDerivationParams {
    fn default() -> Self {
        let mut salt = [0u8; 32];
        SystemRandom::new().fill(&mut salt).unwrap();
        
        KeyDerivationParams {
            algorithm: "Argon2id".to_string(),
            iterations: 100_000,
            salt: salt.to_vec(),
        }
    }
}

/// Classified environment variable
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassifiedEnvVar {
    pub name: String,
    pub value: String,
    pub classification: SecurityClassification,
    pub access_policy: String,
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

/// Types of security events
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

/// Access policy for resources
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessPolicy {
    pub resource: String,
    pub allowed_actors: Vec<String>,
    pub allowed_access_types: Vec<AccessType>,
    pub time_restrictions: Option<TimeRestrictions>,
    pub ip_restrictions: Option<Vec<String>>,
}

/// Time-based access restrictions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeRestrictions {
    pub start_time: String, // HH:MM format
    pub end_time: String,   // HH:MM format
    pub allowed_days: Vec<String>, // ["Monday", "Tuesday", ...]
}

impl AccessPolicy {
    pub fn allows(&self, actor: &str, access_type: AccessType) -> bool {
        self.allowed_actors.contains(&actor.to_string()) &&
        self.allowed_access_types.contains(&access_type)
    }
}

/// Secure environment manager
pub struct SecureEnvironmentManager {
    /// Encrypted secrets storage
    secrets: Arc<RwLock<HashMap<String, EncryptedSecret>>>,
    /// Environment variables with security classification
    env_vars: Arc<RwLock<HashMap<String, ClassifiedEnvVar>>>,
    /// Master encryption key
    master_key: Arc<RwLock<Key>>,
    /// Security audit log
    audit_log: Arc<RwLock<Vec<SecurityEvent>>>,
    /// Access control policies
    access_policies: Arc<RwLock<HashMap<String, AccessPolicy>>>,
}

impl SecureEnvironmentManager {
    /// Create a new secure environment manager
    pub fn new() -> Result<Self, SecurityError> {
        // Initialize with secure random master key
        let rng = SystemRandom::new();
        let mut master_key_bytes = [0u8; 32];
        rng.fill(&mut master_key_bytes)?;
        let master_key = Key::from_slice(&master_key_bytes);
        
        Ok(SecureEnvironmentManager {
            secrets: Arc::new(RwLock::new(HashMap::new())),
            env_vars: Arc::new(RwLock::new(HashMap::new())),
            master_key: Arc::new(RwLock::new(*master_key)),
            audit_log: Arc::new(RwLock::new(Vec::new())),
            access_policies: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Store a secret with encryption
    pub fn store_secret(&self, secret: Secret) -> Result<(), SecurityError> {
        // Encrypt secret with master key
        let cipher = ChaCha20Poly1305::new(&self.master_key.read().unwrap());
        let nonce = ChaCha20Poly1305::generate_nonce(&mut ring::rand::SystemRandom::new());
        
        let mut buffer = secret.value.clone();
        let tag = cipher.encrypt_in_place_detached(&nonce, b"", &mut buffer)
            .map_err(|_| SecurityError::EncryptionFailed)?;
        
        // Combine encrypted data and tag
        let mut encrypted_data = buffer;
        encrypted_data.extend_from_slice(&tag);
        
        // Create encrypted secret record
        let encrypted_secret = EncryptedSecret {
            name: secret.name.clone(),
            encrypted_data,
            nonce: nonce.to_vec(),
            secret_type: secret.secret_type.clone(),
            metadata: SecretMetadata {
                created_at: secret.created_at,
                expires_at: secret.expires_at,
                rotation_interval: secret.rotation_interval,
                access_policy: "default".to_string(),
                encryption_algorithm: "ChaCha20Poly1305".to_string(),
                key_derivation_params: KeyDerivationParams::default(),
            },
            access_count: 0,
            last_accessed: SystemTime::now(),
        };
        
        // Store encrypted secret
        self.secrets.write().unwrap().insert(secret.name.clone(), encrypted_secret);
        
        // Log security event
        self.log_security_event(SecurityEvent {
            event_type: SecurityEventType::SecretStored,
            resource: secret.name.clone(),
            actor: "system".to_string(),
            timestamp: SystemTime::now(),
            metadata: HashMap::new(),
        });
        
        Ok(())
    }

    /// Retrieve and decrypt a secret
    pub fn retrieve_secret(&self, name: &str, actor: &str) -> Result<Secret, SecurityError> {
        // Check access policy
        self.check_access_policy(name, actor, AccessType::Read)?;
        
        let mut secrets = self.secrets.write().unwrap();
        let encrypted_secret = secrets.get_mut(name)
            .ok_or_else(|| SecurityError::SecretNotFound(name.to_string()))?;
        
        // Check expiration
        if let Some(expires_at) = encrypted_secret.metadata.expires_at {
            if SystemTime::now() > expires_at {
                return Err(SecurityError::SecretExpired(name.to_string()));
            }
        }
        
        // Decrypt secret
        let cipher = ChaCha20Poly1305::new(&self.master_key.read().unwrap());
        let nonce = Nonce::from_slice(&encrypted_secret.nonce);
        
        // Split encrypted data and tag
        let encrypted_len = encrypted_secret.encrypted_data.len();
        if encrypted_len < 16 {
            return Err(SecurityError::DecryptionFailed);
        }
        
        let (encrypted_data, tag) = encrypted_secret.encrypted_data.split_at(encrypted_len - 16);
        let mut buffer = encrypted_data.to_vec();
        
        cipher.decrypt_in_place_detached(nonce, b"", &mut buffer, tag.into())
            .map_err(|_| SecurityError::DecryptionFailed)?;
        
        // Update access statistics
        encrypted_secret.access_count += 1;
        encrypted_secret.last_accessed = SystemTime::now();
        
        // Create decrypted secret
        let secret = Secret {
            name: name.to_string(),
            value: buffer,
            secret_type: encrypted_secret.secret_type.clone(),
            created_at: encrypted_secret.metadata.created_at,
            expires_at: encrypted_secret.metadata.expires_at,
            rotation_interval: encrypted_secret.metadata.rotation_interval,
        };
        
        // Log access event
        self.log_security_event(SecurityEvent {
            event_type: SecurityEventType::SecretAccessed,
            resource: name.to_string(),
            actor: actor.to_string(),
            timestamp: SystemTime::now(),
            metadata: HashMap::new(),
        });
        
        Ok(secret)
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
            access_policy: "default".to_string(),
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
        let env_vars = self.env_vars.read().unwrap();
        let env_var = env_vars.get(name)
            .ok_or_else(|| SecurityError::EnvVarNotFound(name.to_string()))?;
        
        // Check access policy
        self.check_access_policy(name, actor, AccessType::Read)?;
        
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

    /// Check access policy for a resource
    fn check_access_policy(&self, resource: &str, actor: &str, access_type: AccessType) -> Result<(), SecurityError> {
        let policies = self.access_policies.read().unwrap();
        
        if let Some(policy) = policies.get(resource) {
            if !policy.allows(actor, access_type.clone()) {
                return Err(SecurityError::AccessDenied {
                    resource: resource.to_string(),
                    actor: actor.to_string(),
                    access_type,
                });
            }
        }
        
        Ok(())
    }

    /// Log a security event
    fn log_security_event(&self, event: SecurityEvent) {
        self.audit_log.write().unwrap().push(event);
    }

    /// Get audit log
    pub fn get_audit_log(&self) -> Vec<SecurityEvent> {
        self.audit_log.read().unwrap().clone()
    }

    /// Set access policy for a resource
    pub fn set_access_policy(&self, resource: String, policy: AccessPolicy) {
        self.access_policies.write().unwrap().insert(resource, policy);
    }
}
