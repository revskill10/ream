//! TLisp Security Integration
//!
//! This module provides TLisp language bindings for the REAM security system,
//! allowing TLisp programs to interact with secrets, environment variables,
//! and access control mechanisms.

use crate::security::basic_security::{SecurityManager, SecretType, SecurityClassification, AccessType};
use crate::tlisp::{Value, TlispInterpreter};
use crate::error::{TlispError, TlispResult};
use std::sync::{Arc, RwLock};
use std::time::SystemTime;

/// TLisp security context that wraps the security manager
pub struct TlispSecurityContext {
    security_manager: Arc<SecurityManager>,
    current_actor: Arc<RwLock<String>>,
}

impl TlispSecurityContext {
    /// Create a new TLisp security context
    pub fn new(security_manager: Arc<SecurityManager>) -> Self {
        TlispSecurityContext {
            security_manager,
            current_actor: Arc::new(RwLock::new("system".to_string())),
        }
    }

    /// Set the current actor for security operations
    pub fn set_current_actor(&self, actor: String) {
        *self.current_actor.write().unwrap() = actor;
    }

    /// Get the current actor
    pub fn get_current_actor(&self) -> String {
        self.current_actor.read().unwrap().clone()
    }

    /// Store a secret from TLisp
    pub fn store_secret(&self, args: Vec<Value>) -> TlispResult<Value> {
        if args.len() != 3 {
            return Err(TlispError::Runtime(format!(
                "Arity mismatch: expected 3 arguments, got {}",
                args.len()
            )));
        }

        let name = match &args[0] {
            Value::String(s) => s.clone(),
            _ => return Err(TlispError::Runtime("Expected string for secret name".to_string())),
        };

        let value = match &args[1] {
            Value::String(s) => s.as_bytes().to_vec(),
            Value::List(bytes) => {
                let mut result = Vec::new();
                for byte_val in bytes {
                    match byte_val {
                        Value::Int(i) => {
                            if *i >= 0 && *i <= 255 {
                                result.push(*i as u8);
                            } else {
                                return Err(TlispError::Runtime("Byte values must be 0-255".to_string()));
                            }
                        }
                        _ => return Err(TlispError::Runtime("Expected integer for byte value".to_string())),
                    }
                }
                result
            }
            _ => return Err(TlispError::Runtime("Expected string or list of bytes for secret value".to_string())),
        };

        let secret_type = match &args[2] {
            Value::String(s) => match s.as_str() {
                "database-url" => SecretType::DatabaseUrl,
                "api-key" => SecretType::ApiKey,
                "certificate-key" => SecretType::CertificateKey,
                "session-secret" => SecretType::SessionSecret,
                "encryption-key" => SecretType::EncryptionKey,
                custom => SecretType::Custom(custom.to_string()),
            },
            _ => return Err(TlispError::Runtime("Expected string for secret type".to_string())),
        };

        let secret = crate::security::basic_security::Secret {
            name: name.clone(),
            value,
            secret_type,
            created_at: SystemTime::now(),
            expires_at: None,
        };

        match self.security_manager.store_secret(secret) {
            Ok(()) => Ok(Value::String(format!("Secret '{}' stored successfully", name))),
            Err(e) => Err(TlispError::Runtime(format!("Failed to store secret: {}", e))),
        }
    }

    /// Retrieve a secret from TLisp
    pub fn retrieve_secret(&self, args: Vec<Value>) -> TlispResult<Value> {
        if args.len() != 1 {
            return Err(TlispError::Runtime(format!(
                "Arity mismatch: expected 1 argument, got {}",
                args.len()
            )));
        }

        let name = match &args[0] {
            Value::String(s) => s.clone(),
            _ => return Err(TlispError::Runtime("Expected string for secret name".to_string())),
        };

        let actor = self.get_current_actor();
        match self.security_manager.retrieve_secret(&name, &actor) {
            Ok(secret) => {
                // Return secret as a list of key-value pairs
                let result = vec![
                    Value::List(vec![Value::String("name".to_string()), Value::String(secret.name)]),
                    Value::List(vec![Value::String("value".to_string()), Value::String(String::from_utf8_lossy(&secret.value).to_string())]),
                    Value::List(vec![Value::String("type".to_string()), Value::String(format!("{:?}", secret.secret_type))]),
                    Value::List(vec![Value::String("created-at".to_string()), Value::String(format!("{:?}", secret.created_at))]),
                ];
                Ok(Value::List(result))
            }
            Err(e) => Err(TlispError::Runtime(format!("Failed to retrieve secret: {}", e))),
        }
    }

    /// Set an environment variable from TLisp
    pub fn set_env_var(&self, args: Vec<Value>) -> TlispResult<Value> {
        if args.len() != 3 {
            return Err(TlispError::Runtime(format!(
                "Arity mismatch: expected 3 arguments, got {}",
                args.len()
            )));
        }

        let name = match &args[0] {
            Value::String(s) => s.clone(),
            _ => return Err(TlispError::Runtime("Expected string for env var name".to_string())),
        };

        let value = match &args[1] {
            Value::String(s) => s.clone(),
            _ => return Err(TlispError::Runtime("Expected string for env var value".to_string())),
        };

        let classification = match &args[2] {
            Value::String(s) => match s.as_str() {
                "public" => SecurityClassification::Public,
                "internal" => SecurityClassification::Internal,
                "confidential" => SecurityClassification::Confidential,
                "secret" => SecurityClassification::Secret,
                "top-secret" => SecurityClassification::TopSecret,
                _ => return Err(TlispError::Runtime("Invalid security classification".to_string())),
            },
            _ => return Err(TlispError::Runtime("Expected string for security classification".to_string())),
        };

        match self.security_manager.set_env_var(name.clone(), value, classification) {
            Ok(()) => Ok(Value::String(format!("Environment variable '{}' set successfully", name))),
            Err(e) => Err(TlispError::Runtime(format!("Failed to set env var: {}", e))),
        }
    }

    /// Get an environment variable from TLisp
    pub fn get_env_var(&self, args: Vec<Value>) -> TlispResult<Value> {
        if args.len() != 1 {
            return Err(TlispError::Runtime(format!(
                "Arity mismatch: expected 1 argument, got {}",
                args.len()
            )));
        }

        let name = match &args[0] {
            Value::String(s) => s.clone(),
            _ => return Err(TlispError::Runtime("Expected string for env var name".to_string())),
        };

        let actor = self.get_current_actor();
        match self.security_manager.get_env_var(&name, &actor) {
            Ok(value) => Ok(Value::String(value)),
            Err(e) => Err(TlispError::Runtime(format!("Failed to get env var: {}", e))),
        }
    }

    /// Grant access to a resource from TLisp
    pub fn grant_access(&self, args: Vec<Value>) -> TlispResult<Value> {
        if args.len() != 2 {
            return Err(TlispError::Runtime(format!(
                "Arity mismatch: expected 2 arguments, got {}",
                args.len()
            )));
        }

        let actor = match &args[0] {
            Value::String(s) => s.clone(),
            _ => return Err(TlispError::Runtime("Expected string for actor".to_string())),
        };

        let resource = match &args[1] {
            Value::String(s) => s.clone(),
            _ => return Err(TlispError::Runtime("Expected string for resource".to_string())),
        };

        self.security_manager.grant_access(&actor, &resource);
        Ok(Value::String(format!("Access granted to '{}' for resource '{}'", actor, resource)))
    }

    /// Check access to a resource from TLisp
    pub fn check_access(&self, args: Vec<Value>) -> TlispResult<Value> {
        if args.len() != 3 {
            return Err(TlispError::Runtime(format!(
                "Arity mismatch: expected 3 arguments, got {}",
                args.len()
            )));
        }

        let actor = match &args[0] {
            Value::String(s) => s.clone(),
            _ => return Err(TlispError::Runtime("Expected string for actor".to_string())),
        };

        let resource = match &args[1] {
            Value::String(s) => s.clone(),
            _ => return Err(TlispError::Runtime("Expected string for resource".to_string())),
        };

        let access_type = match &args[2] {
            Value::String(s) => match s.as_str() {
                "read" => AccessType::Read,
                "write" => AccessType::Write,
                "delete" => AccessType::Delete,
                "rotate" => AccessType::Rotate,
                _ => return Err(TlispError::Runtime("Invalid access type".to_string())),
            },
            _ => return Err(TlispError::Runtime("Expected string for access type".to_string())),
        };

        match self.security_manager.check_access(&actor, &resource, access_type) {
            Ok(()) => Ok(Value::Bool(true)),
            Err(_) => Ok(Value::Bool(false)),
        }
    }

    /// Get audit log from TLisp
    pub fn get_audit_log(&self, _args: Vec<Value>) -> TlispResult<Value> {
        let audit_log = self.security_manager.get_audit_log();
        let mut result = Vec::new();

        for event in audit_log {
            // Represent each event as a list of key-value pairs
            let event_list = vec![
                Value::List(vec![Value::String("event-type".to_string()), Value::String(format!("{:?}", event.event_type))]),
                Value::List(vec![Value::String("resource".to_string()), Value::String(event.resource)]),
                Value::List(vec![Value::String("actor".to_string()), Value::String(event.actor)]),
                Value::List(vec![Value::String("timestamp".to_string()), Value::String(format!("{:?}", event.timestamp))]),
            ];
            result.push(Value::List(event_list));
        }

        Ok(Value::List(result))
    }

    /// List all secrets from TLisp
    pub fn list_secrets(&self, _args: Vec<Value>) -> TlispResult<Value> {
        let secrets = self.security_manager.list_secrets();
        let result: Vec<Value> = secrets.into_iter().map(Value::String).collect();
        Ok(Value::List(result))
    }

    /// List all environment variables from TLisp
    pub fn list_env_vars(&self, _args: Vec<Value>) -> TlispResult<Value> {
        let env_vars = self.security_manager.list_env_vars();
        let result: Vec<Value> = env_vars.into_iter().map(Value::String).collect();
        Ok(Value::List(result))
    }
}

/// Register security functions with TLisp interpreter
pub fn register_security_functions(
    interpreter: &mut crate::tlisp::TlispInterpreter,
    _security_context: Arc<TlispSecurityContext>,
) -> TlispResult<()> {
    // Register security functions as built-ins
    // The actual implementation will be handled by the evaluator
    interpreter.define("security/store-secret".to_string(), Value::Builtin("security/store-secret".to_string()));
    interpreter.define("security/retrieve-secret".to_string(), Value::Builtin("security/retrieve-secret".to_string()));
    interpreter.define("security/set-env-var".to_string(), Value::Builtin("security/set-env-var".to_string()));
    interpreter.define("security/get-env-var".to_string(), Value::Builtin("security/get-env-var".to_string()));
    interpreter.define("security/grant-access".to_string(), Value::Builtin("security/grant-access".to_string()));
    interpreter.define("security/check-access".to_string(), Value::Builtin("security/check-access".to_string()));
    interpreter.define("security/get-audit-log".to_string(), Value::Builtin("security/get-audit-log".to_string()));
    interpreter.define("security/list-secrets".to_string(), Value::Builtin("security/list-secrets".to_string()));
    interpreter.define("security/list-env-vars".to_string(), Value::Builtin("security/list-env-vars".to_string()));

    Ok(())
}
