//! Access Control Manager
//!
//! This module provides role-based access control (RBAC) and attribute-based access control (ABAC)
//! for the REAM security system.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};
use serde::{Deserialize, Serialize};
use std::time::SystemTime;

/// Role definition with permissions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    pub name: String,
    pub description: String,
    pub permissions: HashSet<Permission>,
    pub created_at: SystemTime,
    pub updated_at: SystemTime,
}

/// Permission definition
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Permission {
    pub resource: String,
    pub action: String,
    pub conditions: Vec<AccessCondition>,
}

/// Access condition for fine-grained control
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct AccessCondition {
    pub attribute: String,
    pub operator: ConditionOperator,
    pub value: String,
}

/// Condition operators
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ConditionOperator {
    Equals,
    NotEquals,
    Contains,
    StartsWith,
    EndsWith,
    GreaterThan,
    LessThan,
    In,
    NotIn,
}

/// Access request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessRequest {
    pub actor: String,
    pub resource: String,
    pub action: String,
    pub context: HashMap<String, String>,
}

/// Access decision
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessDecision {
    pub allowed: bool,
    pub reason: String,
    pub matched_permissions: Vec<Permission>,
    pub evaluated_at: SystemTime,
}

/// User with roles and attributes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub name: String,
    pub email: String,
    pub roles: HashSet<String>,
    pub attributes: HashMap<String, String>,
    pub created_at: SystemTime,
    pub last_login: Option<SystemTime>,
    pub active: bool,
}

/// Access control policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessPolicy {
    pub name: String,
    pub description: String,
    pub rules: Vec<PolicyRule>,
    pub priority: u32,
    pub active: bool,
}

/// Policy rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyRule {
    pub effect: PolicyEffect,
    pub subjects: Vec<String>, // Users or roles
    pub resources: Vec<String>,
    pub actions: Vec<String>,
    pub conditions: Vec<AccessCondition>,
}

/// Policy effect
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PolicyEffect {
    Allow,
    Deny,
}

/// Access control manager
pub struct AccessControlManager {
    /// Users in the system
    users: Arc<RwLock<HashMap<String, User>>>,
    /// Roles in the system
    roles: Arc<RwLock<HashMap<String, Role>>>,
    /// Access policies
    policies: Arc<RwLock<HashMap<String, AccessPolicy>>>,
    /// Access log for auditing
    access_log: Arc<RwLock<Vec<AccessLogEntry>>>,
}

/// Access log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessLogEntry {
    pub request: AccessRequest,
    pub decision: AccessDecision,
    pub timestamp: SystemTime,
}

impl AccessControlManager {
    /// Create a new access control manager
    pub fn new() -> Self {
        AccessControlManager {
            users: Arc::new(RwLock::new(HashMap::new())),
            roles: Arc::new(RwLock::new(HashMap::new())),
            policies: Arc::new(RwLock::new(HashMap::new())),
            access_log: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Add a user to the system
    pub fn add_user(&self, user: User) {
        self.users.write().unwrap().insert(user.id.clone(), user);
    }

    /// Add a role to the system
    pub fn add_role(&self, role: Role) {
        self.roles.write().unwrap().insert(role.name.clone(), role);
    }

    /// Add a policy to the system
    pub fn add_policy(&self, policy: AccessPolicy) {
        self.policies.write().unwrap().insert(policy.name.clone(), policy);
    }

    /// Assign a role to a user
    pub fn assign_role(&self, user_id: &str, role_name: &str) -> Result<(), AccessControlError> {
        let mut users = self.users.write().unwrap();
        let user = users.get_mut(user_id)
            .ok_or_else(|| AccessControlError::UserNotFound(user_id.to_string()))?;

        if !self.roles.read().unwrap().contains_key(role_name) {
            return Err(AccessControlError::RoleNotFound(role_name.to_string()));
        }

        user.roles.insert(role_name.to_string());
        Ok(())
    }

    /// Remove a role from a user
    pub fn remove_role(&self, user_id: &str, role_name: &str) -> Result<(), AccessControlError> {
        let mut users = self.users.write().unwrap();
        let user = users.get_mut(user_id)
            .ok_or_else(|| AccessControlError::UserNotFound(user_id.to_string()))?;

        user.roles.remove(role_name);
        Ok(())
    }

    /// Check access for a request
    pub fn check_access(&self, request: &AccessRequest) -> AccessDecision {
        let start_time = SystemTime::now();

        // Get user
        let users = self.users.read().unwrap();
        let user = match users.get(&request.actor) {
            Some(user) => user.clone(),
            None => {
                let decision = AccessDecision {
                    allowed: false,
                    reason: format!("User not found: {}", request.actor),
                    matched_permissions: Vec::new(),
                    evaluated_at: start_time,
                };
                self.log_access(request.clone(), decision.clone());
                return decision;
            }
        };
        drop(users);

        // Check if user is active
        if !user.active {
            let decision = AccessDecision {
                allowed: false,
                reason: format!("User is inactive: {}", request.actor),
                matched_permissions: Vec::new(),
                evaluated_at: start_time,
            };
            self.log_access(request.clone(), decision.clone());
            return decision;
        }

        // Collect all permissions from user's roles
        let mut all_permissions = Vec::new();
        let roles = self.roles.read().unwrap();
        for role_name in &user.roles {
            if let Some(role) = roles.get(role_name) {
                all_permissions.extend(role.permissions.iter().cloned());
            }
        }
        drop(roles);

        // Check permissions
        let mut matched_permissions = Vec::new();
        for permission in &all_permissions {
            if self.permission_matches(permission, request, user) {
                matched_permissions.push(permission.clone());
            }
        }

        // Check policies
        let policy_decision = self.evaluate_policies(request, user);

        // Make final decision
        let allowed = !matched_permissions.is_empty() && policy_decision;
        let reason = if allowed {
            "Access granted".to_string()
        } else if matched_permissions.is_empty() {
            "No matching permissions".to_string()
        } else {
            "Denied by policy".to_string()
        };

        let decision = AccessDecision {
            allowed,
            reason,
            matched_permissions,
            evaluated_at: start_time,
        };

        self.log_access(request.clone(), decision.clone());
        decision
    }

    /// Check if a permission matches a request
    fn permission_matches(&self, permission: &Permission, request: &AccessRequest, user: &User) -> bool {
        // Check resource match
        if !self.resource_matches(&permission.resource, &request.resource) {
            return false;
        }

        // Check action match
        if !self.action_matches(&permission.action, &request.action) {
            return false;
        }

        // Check conditions
        for condition in &permission.conditions {
            if !self.condition_matches(condition, request, user) {
                return false;
            }
        }

        true
    }

    /// Check if resource pattern matches
    fn resource_matches(&self, pattern: &str, resource: &str) -> bool {
        if pattern == "*" {
            return true;
        }
        
        if pattern.ends_with('*') {
            let prefix = &pattern[..pattern.len() - 1];
            return resource.starts_with(prefix);
        }
        
        pattern == resource
    }

    /// Check if action pattern matches
    fn action_matches(&self, pattern: &str, action: &str) -> bool {
        if pattern == "*" {
            return true;
        }
        
        pattern == action
    }

    /// Check if a condition matches
    fn condition_matches(&self, condition: &AccessCondition, request: &AccessRequest, user: &User) -> bool {
        let empty_string = String::new();
        let actual_value = match condition.attribute.as_str() {
            "user.id" => &user.id,
            "user.name" => &user.name,
            "user.email" => &user.email,
            attr if attr.starts_with("user.") => {
                let attr_name = &attr[5..];
                user.attributes.get(attr_name).unwrap_or(&empty_string)
            }
            attr if attr.starts_with("context.") => {
                let attr_name = &attr[8..];
                request.context.get(attr_name).unwrap_or(&empty_string)
            }
            _ => return false,
        };

        match condition.operator {
            ConditionOperator::Equals => actual_value == &condition.value,
            ConditionOperator::NotEquals => actual_value != &condition.value,
            ConditionOperator::Contains => actual_value.contains(&condition.value),
            ConditionOperator::StartsWith => actual_value.starts_with(&condition.value),
            ConditionOperator::EndsWith => actual_value.ends_with(&condition.value),
            ConditionOperator::GreaterThan => {
                actual_value.parse::<f64>().unwrap_or(0.0) > condition.value.parse::<f64>().unwrap_or(0.0)
            }
            ConditionOperator::LessThan => {
                actual_value.parse::<f64>().unwrap_or(0.0) < condition.value.parse::<f64>().unwrap_or(0.0)
            }
            ConditionOperator::In => {
                let values: Vec<&str> = condition.value.split(',').collect();
                values.contains(&actual_value.as_str())
            }
            ConditionOperator::NotIn => {
                let values: Vec<&str> = condition.value.split(',').collect();
                !values.contains(&actual_value.as_str())
            }
        }
    }

    /// Evaluate policies for a request
    fn evaluate_policies(&self, request: &AccessRequest, user: &User) -> bool {
        let mut allow_policies = Vec::new();
        let mut deny_policies = Vec::new();

        let policies = self.policies.read().unwrap();
        for policy in policies.values() {
            if !policy.active {
                continue;
            }

            for rule in &policy.rules {
                if self.rule_matches(rule, request, user) {
                    match rule.effect {
                        PolicyEffect::Allow => allow_policies.push((policy.priority, policy)),
                        PolicyEffect::Deny => deny_policies.push((policy.priority, policy)),
                    }
                }
            }
        }
        drop(policies);

        // Sort by priority (higher priority first)
        allow_policies.sort_by(|a, b| b.0.cmp(&a.0));
        deny_policies.sort_by(|a, b| b.0.cmp(&a.0));

        // Deny takes precedence if there are any deny policies
        if !deny_policies.is_empty() {
            return false;
        }

        // Allow if there are allow policies
        !allow_policies.is_empty()
    }

    /// Check if a policy rule matches
    fn rule_matches(&self, rule: &PolicyRule, request: &AccessRequest, user: &User) -> bool {
        // Check subjects (users or roles)
        let mut subject_match = false;
        for subject in &rule.subjects {
            if subject == &user.id || user.roles.contains(subject) {
                subject_match = true;
                break;
            }
        }
        if !subject_match {
            return false;
        }

        // Check resources
        if !rule.resources.iter().any(|r| self.resource_matches(r, &request.resource)) {
            return false;
        }

        // Check actions
        if !rule.actions.iter().any(|a| self.action_matches(a, &request.action)) {
            return false;
        }

        // Check conditions
        for condition in &rule.conditions {
            if !self.condition_matches(condition, request, user) {
                return false;
            }
        }

        true
    }

    /// Log an access request and decision
    fn log_access(&self, request: AccessRequest, decision: AccessDecision) {
        let entry = AccessLogEntry {
            request,
            decision,
            timestamp: SystemTime::now(),
        };
        self.access_log.write().unwrap().push(entry);
    }

    /// Get access log
    pub fn get_access_log(&self) -> Vec<AccessLogEntry> {
        self.access_log.read().unwrap().clone()
    }

    /// Get user by ID
    pub fn get_user(&self, user_id: &str) -> Option<User> {
        self.users.read().unwrap().get(user_id).cloned()
    }

    /// Get role by name
    pub fn get_role(&self, role_name: &str) -> Option<Role> {
        self.roles.read().unwrap().get(role_name).cloned()
    }

    /// Get policy by name
    pub fn get_policy(&self, policy_name: &str) -> Option<AccessPolicy> {
        self.policies.read().unwrap().get(policy_name).cloned()
    }
}

/// Access control errors
#[derive(Debug, thiserror::Error)]
pub enum AccessControlError {
    #[error("User not found: {0}")]
    UserNotFound(String),
    #[error("Role not found: {0}")]
    RoleNotFound(String),
    #[error("Policy not found: {0}")]
    PolicyNotFound(String),
    #[error("Permission denied")]
    PermissionDenied,
    #[error("Invalid condition: {0}")]
    InvalidCondition(String),
}
