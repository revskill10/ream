//! Capability-Based Security System for TLISP
//! 
//! Implements capability-based security for secure resource access with type-safe capabilities.
//! Capabilities are unforgeable tokens that grant specific permissions to resources.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use crate::tlisp::{Value, Expr};
use crate::tlisp::types::Type;
use crate::error::{TlispError, TlispResult};

/// Capability types
#[derive(Debug, Clone, PartialEq)]
pub enum CapabilityType {
    /// File system read capability
    FileRead(String), // path pattern
    /// File system write capability
    FileWrite(String), // path pattern
    /// Network access capability
    Network(String), // host pattern
    /// Process spawn capability
    Spawn,
    /// Memory allocation capability
    Memory(usize), // max bytes
    /// Database access capability
    Database(String), // database name
    /// Custom capability
    Custom(String, Type),
}

/// Capability token
#[derive(Debug, Clone)]
pub struct Capability {
    /// Capability ID
    id: u64,
    /// Capability type
    cap_type: CapabilityType,
    /// Whether capability is revoked
    revoked: bool,
    /// Capability metadata
    metadata: HashMap<String, String>,
}

/// Capability grant
#[derive(Debug, Clone)]
pub struct CapabilityGrant {
    /// Capability being granted
    capability: Capability,
    /// Grantee (process or actor)
    grantee: String,
    /// Grantor (who granted the capability)
    grantor: String,
    /// Grant timestamp
    timestamp: u64,
}

/// Capability manager
pub struct CapabilityManager {
    /// All capabilities
    capabilities: Arc<RwLock<HashMap<u64, Capability>>>,
    /// Capability grants
    grants: Arc<RwLock<HashMap<String, Vec<CapabilityGrant>>>>,
    /// Next capability ID
    next_cap_id: Arc<RwLock<u64>>,
    /// Capability policies
    policies: Arc<RwLock<HashMap<String, CapabilityPolicy>>>,
}

/// Capability policy
#[derive(Debug, Clone)]
pub struct CapabilityPolicy {
    /// Policy name
    name: String,
    /// Allowed capability types
    allowed_types: Vec<CapabilityType>,
    /// Maximum capabilities per grantee
    max_capabilities: Option<usize>,
    /// Policy rules
    rules: Vec<PolicyRule>,
}

/// Policy rule
#[derive(Debug, Clone)]
pub enum PolicyRule {
    /// Allow if condition is met
    Allow(PolicyCondition),
    /// Deny if condition is met
    Deny(PolicyCondition),
    /// Require specific capability
    Require(CapabilityType),
}

/// Policy condition
#[derive(Debug, Clone)]
pub enum PolicyCondition {
    /// Check grantee identity
    GranteeIs(String),
    /// Check grantor identity
    GrantorIs(String),
    /// Check capability type
    CapabilityType(CapabilityType),
    /// Check time constraint
    TimeConstraint(u64, u64), // start, end
    /// Custom condition
    Custom(String),
}

impl Capability {
    /// Create a new capability
    pub fn new(cap_type: CapabilityType) -> Self {
        Capability {
            id: 0, // Will be set by manager
            cap_type,
            revoked: false,
            metadata: HashMap::new(),
        }
    }

    /// Get capability ID
    pub fn id(&self) -> u64 {
        self.id
    }

    /// Get capability type
    pub fn cap_type(&self) -> &CapabilityType {
        &self.cap_type
    }

    /// Check if capability is revoked
    pub fn is_revoked(&self) -> bool {
        self.revoked
    }

    /// Revoke capability
    pub fn revoke(&mut self) {
        self.revoked = true;
    }

    /// Add metadata
    pub fn add_metadata(&mut self, key: String, value: String) {
        self.metadata.insert(key, value);
    }

    /// Get metadata
    pub fn get_metadata(&self, key: &str) -> Option<&String> {
        self.metadata.get(key)
    }
}

impl CapabilityManager {
    /// Create a new capability manager
    pub fn new() -> Self {
        CapabilityManager {
            capabilities: Arc::new(RwLock::new(HashMap::new())),
            grants: Arc::new(RwLock::new(HashMap::new())),
            next_cap_id: Arc::new(RwLock::new(1)),
            policies: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a new capability
    pub fn create_capability(&self, cap_type: CapabilityType) -> TlispResult<Capability> {
        let cap_id = {
            let mut next_id = self.next_cap_id.write().unwrap();
            let id = *next_id;
            *next_id += 1;
            id
        };

        let mut capability = Capability::new(cap_type);
        capability.id = cap_id;

        // Store capability
        {
            let mut capabilities = self.capabilities.write().unwrap();
            capabilities.insert(cap_id, capability.clone());
        }

        Ok(capability)
    }

    /// Grant capability to a grantee
    pub fn grant_capability(
        &self,
        capability: Capability,
        grantee: String,
        grantor: String,
    ) -> TlispResult<()> {
        // Check if grant is allowed by policies
        self.check_grant_policies(&capability, &grantee, &grantor)?;

        let grant = CapabilityGrant {
            capability,
            grantee: grantee.clone(),
            grantor,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };

        // Store grant
        {
            let mut grants = self.grants.write().unwrap();
            grants.entry(grantee).or_insert_with(Vec::new).push(grant);
        }

        Ok(())
    }

    /// Revoke capability
    pub fn revoke_capability(&self, cap_id: u64) -> TlispResult<()> {
        let mut capabilities = self.capabilities.write().unwrap();
        if let Some(capability) = capabilities.get_mut(&cap_id) {
            capability.revoke();
            Ok(())
        } else {
            Err(TlispError::Runtime(format!("Capability {} not found", cap_id)))
        }
    }

    /// Check if grantee has capability
    pub fn has_capability(&self, grantee: &str, cap_type: &CapabilityType) -> bool {
        let grants = self.grants.read().unwrap();
        if let Some(grantee_grants) = grants.get(grantee) {
            for grant in grantee_grants {
                if !grant.capability.is_revoked() && &grant.capability.cap_type == cap_type {
                    return true;
                }
            }
        }
        false
    }

    /// Get capabilities for grantee
    pub fn get_capabilities(&self, grantee: &str) -> Vec<Capability> {
        let grants = self.grants.read().unwrap();
        if let Some(grantee_grants) = grants.get(grantee) {
            grantee_grants
                .iter()
                .filter(|grant| !grant.capability.is_revoked())
                .map(|grant| grant.capability.clone())
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Add capability policy
    pub fn add_policy(&self, policy: CapabilityPolicy) {
        let mut policies = self.policies.write().unwrap();
        policies.insert(policy.name.clone(), policy);
    }

    /// Check grant against policies
    fn check_grant_policies(
        &self,
        capability: &Capability,
        grantee: &str,
        grantor: &str,
    ) -> TlispResult<()> {
        let policies = self.policies.read().unwrap();
        
        for policy in policies.values() {
            // Check if capability type is allowed
            if !policy.allowed_types.contains(&capability.cap_type) {
                continue; // Policy doesn't apply to this capability type
            }

            // Check policy rules
            for rule in &policy.rules {
                match rule {
                    PolicyRule::Allow(condition) => {
                        if self.evaluate_condition(condition, capability, grantee, grantor) {
                            return Ok(()); // Explicitly allowed
                        }
                    }
                    PolicyRule::Deny(condition) => {
                        if self.evaluate_condition(condition, capability, grantee, grantor) {
                            return Err(TlispError::Runtime(
                                format!("Capability grant denied by policy: {}", policy.name)
                            ));
                        }
                    }
                    PolicyRule::Require(required_cap) => {
                        if !self.has_capability(grantor, required_cap) {
                            return Err(TlispError::Runtime(
                                format!("Grantor lacks required capability: {:?}", required_cap)
                            ));
                        }
                    }
                }
            }

            // Check maximum capabilities limit
            if let Some(max_caps) = policy.max_capabilities {
                let current_caps = self.get_capabilities(grantee).len();
                if current_caps >= max_caps {
                    return Err(TlispError::Runtime(
                        format!("Maximum capabilities exceeded for {}", grantee)
                    ));
                }
            }
        }

        Ok(())
    }

    /// Evaluate policy condition
    fn evaluate_condition(
        &self,
        condition: &PolicyCondition,
        capability: &Capability,
        grantee: &str,
        grantor: &str,
    ) -> bool {
        match condition {
            PolicyCondition::GranteeIs(expected) => grantee == expected,
            PolicyCondition::GrantorIs(expected) => grantor == expected,
            PolicyCondition::CapabilityType(expected) => &capability.cap_type == expected,
            PolicyCondition::TimeConstraint(start, end) => {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                now >= *start && now <= *end
            }
            PolicyCondition::Custom(_) => {
                // Custom conditions would be evaluated by external logic
                false
            }
        }
    }

    /// List all capabilities
    pub fn list_capabilities(&self) -> Vec<Capability> {
        let capabilities = self.capabilities.read().unwrap();
        capabilities.values().cloned().collect()
    }

    /// Get capability statistics
    pub fn get_stats(&self) -> CapabilityStats {
        let capabilities = self.capabilities.read().unwrap();
        let grants = self.grants.read().unwrap();
        
        let total_capabilities = capabilities.len();
        let revoked_capabilities = capabilities.values()
            .filter(|cap| cap.is_revoked())
            .count();
        let total_grants = grants.values()
            .map(|g| g.len())
            .sum();

        CapabilityStats {
            total_capabilities,
            revoked_capabilities,
            total_grants,
            active_grantees: grants.len(),
        }
    }
}

/// Capability statistics
#[derive(Debug, Clone)]
pub struct CapabilityStats {
    /// Total number of capabilities
    pub total_capabilities: usize,
    /// Number of revoked capabilities
    pub revoked_capabilities: usize,
    /// Total number of grants
    pub total_grants: usize,
    /// Number of active grantees
    pub active_grantees: usize,
}

/// Capability primitive functions for TLISP
pub struct CapabilityPrimitives {
    /// Capability manager
    manager: Arc<CapabilityManager>,
}

impl CapabilityPrimitives {
    /// Create new capability primitives
    pub fn new(manager: Arc<CapabilityManager>) -> Self {
        CapabilityPrimitives { manager }
    }

    /// Create capability primitive
    pub fn create_capability(&self, cap_type: CapabilityType) -> TlispResult<Value> {
        let capability = self.manager.create_capability(cap_type)?;
        Ok(Value::String(format!("capability:{}", capability.id())))
    }

    /// Grant capability primitive
    pub fn grant_capability(
        &self,
        capability_id: u64,
        grantee: String,
        grantor: String,
    ) -> TlispResult<Value> {
        let capabilities = self.manager.capabilities.read().unwrap();
        if let Some(capability) = capabilities.get(&capability_id) {
            self.manager.grant_capability(capability.clone(), grantee, grantor)?;
            Ok(Value::Bool(true))
        } else {
            Err(TlispError::Runtime(format!("Capability {} not found", capability_id)))
        }
    }

    /// Check capability primitive
    pub fn has_capability(&self, grantee: String, cap_type: CapabilityType) -> TlispResult<Value> {
        let has_cap = self.manager.has_capability(&grantee, &cap_type);
        Ok(Value::Bool(has_cap))
    }

    /// With capability primitive (execute code with capability)
    pub fn with_capability(
        &self,
        grantee: String,
        cap_type: CapabilityType,
        code: Expr<Type>,
    ) -> TlispResult<Value> {
        // Check if grantee has the required capability
        if !self.manager.has_capability(&grantee, &cap_type) {
            return Err(TlispError::Runtime(
                format!("Missing required capability: {:?}", cap_type)
            ));
        }

        // TODO: Execute code in capability context
        // For now, just return success
        Ok(Value::Bool(true))
    }
}

impl Default for CapabilityManager {
    fn default() -> Self {
        Self::new()
    }
}
