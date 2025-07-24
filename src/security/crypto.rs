//! Cryptographic Operations Manager
//!
//! This module provides cryptographic primitives and operations for the REAM security system.

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce, KeyInit, AeadCore, AeadInPlace};
use aes_gcm::{Aes256Gcm, Key as AesKey, Nonce as AesNonce};
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier, password_hash::{rand_core::OsRng, SaltString}};
use ring::{
    digest,
    rand::{SystemRandom, SecureRandom},
    signature::{self, KeyPair, RsaKeyPair, UnparsedPublicKey, RSA_PKCS1_SHA256},
    hmac,
};
use zeroize::{Zeroize, ZeroizeOnDrop};
use thiserror::Error;

/// Supported encryption algorithms
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EncryptionAlgorithm {
    ChaCha20Poly1305,
    Aes256Gcm,
}

/// Key derivation parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyDerivationParams {
    pub algorithm: String,
    pub iterations: u32,
    pub memory_cost: Option<u32>,
    pub parallelism: Option<u32>,
    pub salt: Vec<u8>,
}

impl Default for KeyDerivationParams {
    fn default() -> Self {
        let salt = SaltString::generate(&mut OsRng);
        KeyDerivationParams {
            algorithm: "Argon2id".to_string(),
            iterations: 100_000,
            memory_cost: Some(65536), // 64 MB
            parallelism: Some(4),
            salt: salt.as_bytes().to_vec(),
        }
    }
}

/// Cryptographic key with metadata
#[derive(Debug, Clone, Zeroize, ZeroizeOnDrop)]
pub struct CryptoKey {
    pub id: String,
    pub key_data: Vec<u8>,
    pub algorithm: EncryptionAlgorithm,
    pub created_at: std::time::SystemTime,
    pub expires_at: Option<std::time::SystemTime>,
    pub usage: KeyUsage,
}

/// Key usage types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KeyUsage {
    Encryption,
    Signing,
    KeyDerivation,
    Authentication,
}

/// Encrypted data with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedData {
    pub ciphertext: Vec<u8>,
    pub nonce: Vec<u8>,
    pub algorithm: EncryptionAlgorithm,
    pub key_id: String,
    pub timestamp: std::time::SystemTime,
}

/// Digital signature with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DigitalSignature {
    pub signature: Vec<u8>,
    pub algorithm: String,
    pub key_id: String,
    pub timestamp: std::time::SystemTime,
}

/// Hash result with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HashResult {
    pub hash: Vec<u8>,
    pub algorithm: String,
    pub timestamp: std::time::SystemTime,
}

/// Cryptographic operations manager
pub struct CryptoManager {
    /// Stored cryptographic keys
    keys: HashMap<String, CryptoKey>,
    /// Random number generator
    rng: SystemRandom,
    /// Argon2 instance for key derivation
    argon2: Argon2<'static>,
}

impl CryptoManager {
    /// Create a new crypto manager
    pub fn new() -> Result<Self, CryptoError> {
        Ok(CryptoManager {
            keys: HashMap::new(),
            rng: SystemRandom::new(),
            argon2: Argon2::default(),
        })
    }

    /// Generate a new cryptographic key
    pub fn generate_key(
        &mut self,
        id: String,
        algorithm: EncryptionAlgorithm,
        usage: KeyUsage,
    ) -> Result<String, CryptoError> {
        let key_data = match algorithm {
            EncryptionAlgorithm::ChaCha20Poly1305 => {
                let mut key_bytes = [0u8; 32];
                self.rng.fill(&mut key_bytes)?;
                key_bytes.to_vec()
            }
            EncryptionAlgorithm::Aes256Gcm => {
                let mut key_bytes = [0u8; 32];
                self.rng.fill(&mut key_bytes)?;
                key_bytes.to_vec()
            }
        };

        let crypto_key = CryptoKey {
            id: id.clone(),
            key_data,
            algorithm,
            created_at: std::time::SystemTime::now(),
            expires_at: None,
            usage,
        };

        self.keys.insert(id.clone(), crypto_key);
        Ok(id)
    }

    /// Encrypt data using a specified key
    pub fn encrypt(
        &self,
        key_id: &str,
        plaintext: &[u8],
    ) -> Result<EncryptedData, CryptoError> {
        let key = self.keys.get(key_id)
            .ok_or_else(|| CryptoError::KeyNotFound(key_id.to_string()))?;

        match key.algorithm {
            EncryptionAlgorithm::ChaCha20Poly1305 => {
                let cipher_key = Key::from_slice(&key.key_data);
                let cipher = ChaCha20Poly1305::new(cipher_key);
                let nonce = ChaCha20Poly1305::generate_nonce(&mut OsRng);
                
                let mut buffer = plaintext.to_vec();
                let tag = cipher.encrypt_in_place_detached(&nonce, b"", &mut buffer)
                    .map_err(|_| CryptoError::EncryptionFailed)?;
                
                // Combine encrypted data and tag
                buffer.extend_from_slice(&tag);
                
                Ok(EncryptedData {
                    ciphertext: buffer,
                    nonce: nonce.to_vec(),
                    algorithm: key.algorithm.clone(),
                    key_id: key_id.to_string(),
                    timestamp: std::time::SystemTime::now(),
                })
            }
            EncryptionAlgorithm::Aes256Gcm => {
                let cipher_key = AesKey::from_slice(&key.key_data);
                let cipher = Aes256Gcm::new(cipher_key);
                let mut nonce_bytes = [0u8; 12];
                self.rng.fill(&mut nonce_bytes)?;
                let nonce = AesNonce::from_slice(&nonce_bytes);
                
                let mut buffer = plaintext.to_vec();
                let tag = cipher.encrypt_in_place_detached(nonce, b"", &mut buffer)
                    .map_err(|_| CryptoError::EncryptionFailed)?;
                
                // Combine encrypted data and tag
                buffer.extend_from_slice(&tag);
                
                Ok(EncryptedData {
                    ciphertext: buffer,
                    nonce: nonce_bytes.to_vec(),
                    algorithm: key.algorithm.clone(),
                    key_id: key_id.to_string(),
                    timestamp: std::time::SystemTime::now(),
                })
            }
        }
    }

    /// Decrypt data using a specified key
    pub fn decrypt(
        &self,
        encrypted_data: &EncryptedData,
    ) -> Result<Vec<u8>, CryptoError> {
        let key = self.keys.get(&encrypted_data.key_id)
            .ok_or_else(|| CryptoError::KeyNotFound(encrypted_data.key_id.clone()))?;

        match encrypted_data.algorithm {
            EncryptionAlgorithm::ChaCha20Poly1305 => {
                let cipher_key = Key::from_slice(&key.key_data);
                let cipher = ChaCha20Poly1305::new(cipher_key);
                let nonce = Nonce::from_slice(&encrypted_data.nonce);
                
                // Split encrypted data and tag
                let ciphertext_len = encrypted_data.ciphertext.len();
                if ciphertext_len < 16 {
                    return Err(CryptoError::DecryptionFailed);
                }
                
                let (ciphertext, tag) = encrypted_data.ciphertext.split_at(ciphertext_len - 16);
                let mut buffer = ciphertext.to_vec();
                
                cipher.decrypt_in_place_detached(nonce, b"", &mut buffer, tag.into())
                    .map_err(|_| CryptoError::DecryptionFailed)?;
                
                Ok(buffer)
            }
            EncryptionAlgorithm::Aes256Gcm => {
                let cipher_key = AesKey::from_slice(&key.key_data);
                let cipher = Aes256Gcm::new(cipher_key);
                let nonce = AesNonce::from_slice(&encrypted_data.nonce);
                
                // Split encrypted data and tag
                let ciphertext_len = encrypted_data.ciphertext.len();
                if ciphertext_len < 16 {
                    return Err(CryptoError::DecryptionFailed);
                }
                
                let (ciphertext, tag) = encrypted_data.ciphertext.split_at(ciphertext_len - 16);
                let mut buffer = ciphertext.to_vec();
                
                cipher.decrypt_in_place_detached(nonce, b"", &mut buffer, tag.into())
                    .map_err(|_| CryptoError::DecryptionFailed)?;
                
                Ok(buffer)
            }
        }
    }

    /// Derive a key from a password
    pub fn derive_key_from_password(
        &self,
        password: &str,
        params: &KeyDerivationParams,
    ) -> Result<Vec<u8>, CryptoError> {
        let salt = SaltString::from_b64(&String::from_utf8_lossy(&params.salt))
            .map_err(|_| CryptoError::KeyDerivationFailed)?;
        
        let password_hash = self.argon2.hash_password(password.as_bytes(), &salt)
            .map_err(|_| CryptoError::KeyDerivationFailed)?;
        
        Ok(password_hash.hash.unwrap().as_bytes().to_vec())
    }

    /// Verify a password against a hash
    pub fn verify_password(
        &self,
        password: &str,
        hash: &str,
    ) -> Result<bool, CryptoError> {
        let parsed_hash = PasswordHash::new(hash)
            .map_err(|_| CryptoError::KeyDerivationFailed)?;
        
        match self.argon2.verify_password(password.as_bytes(), &parsed_hash) {
            Ok(()) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    /// Compute a cryptographic hash
    pub fn hash(&self, data: &[u8], algorithm: &str) -> Result<HashResult, CryptoError> {
        let hash = match algorithm {
            "SHA256" => digest::digest(&digest::SHA256, data).as_ref().to_vec(),
            "SHA512" => digest::digest(&digest::SHA512, data).as_ref().to_vec(),
            _ => return Err(CryptoError::UnsupportedAlgorithm(algorithm.to_string())),
        };

        Ok(HashResult {
            hash,
            algorithm: algorithm.to_string(),
            timestamp: std::time::SystemTime::now(),
        })
    }

    /// Generate HMAC
    pub fn hmac(&self, key: &[u8], data: &[u8]) -> Result<Vec<u8>, CryptoError> {
        let hmac_key = hmac::Key::new(hmac::HMAC_SHA256, key);
        let signature = hmac::sign(&hmac_key, data);
        Ok(signature.as_ref().to_vec())
    }

    /// Verify HMAC
    pub fn verify_hmac(&self, key: &[u8], data: &[u8], signature: &[u8]) -> Result<bool, CryptoError> {
        let hmac_key = hmac::Key::new(hmac::HMAC_SHA256, key);
        match hmac::verify(&hmac_key, data, signature) {
            Ok(()) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    /// Generate random bytes
    pub fn random_bytes(&self, length: usize) -> Result<Vec<u8>, CryptoError> {
        let mut bytes = vec![0u8; length];
        self.rng.fill(&mut bytes)?;
        Ok(bytes)
    }

    /// Get key information
    pub fn get_key_info(&self, key_id: &str) -> Option<&CryptoKey> {
        self.keys.get(key_id)
    }

    /// List all keys
    pub fn list_keys(&self) -> Vec<&str> {
        self.keys.keys().map(|s| s.as_str()).collect()
    }

    /// Remove a key
    pub fn remove_key(&mut self, key_id: &str) -> Result<(), CryptoError> {
        self.keys.remove(key_id)
            .ok_or_else(|| CryptoError::KeyNotFound(key_id.to_string()))?;
        Ok(())
    }

    /// Rotate a key (generate new key with same ID)
    pub fn rotate_key(
        &mut self,
        key_id: &str,
    ) -> Result<(), CryptoError> {
        let old_key = self.keys.get(key_id)
            .ok_or_else(|| CryptoError::KeyNotFound(key_id.to_string()))?;
        
        let algorithm = old_key.algorithm.clone();
        let usage = old_key.usage.clone();
        
        // Remove old key and generate new one
        self.keys.remove(key_id);
        self.generate_key(key_id.to_string(), algorithm, usage)?;
        
        Ok(())
    }
}

/// Cryptographic errors
#[derive(Debug, Error)]
pub enum CryptoError {
    #[error("Encryption failed")]
    EncryptionFailed,
    #[error("Decryption failed")]
    DecryptionFailed,
    #[error("Key not found: {0}")]
    KeyNotFound(String),
    #[error("Key derivation failed")]
    KeyDerivationFailed,
    #[error("Unsupported algorithm: {0}")]
    UnsupportedAlgorithm(String),
    #[error("Random number generation failed")]
    RandomGenerationFailed,
    #[error("Invalid signature")]
    InvalidSignature,
    #[error("Key generation failed")]
    KeyGenerationFailed,
}

impl From<ring::error::Unspecified> for CryptoError {
    fn from(_: ring::error::Unspecified) -> Self {
        CryptoError::RandomGenerationFailed
    }
}
