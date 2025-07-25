//! Environment management for TLISP

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::rc::Rc;
use std::cell::RefCell;
use crate::tlisp::Value;

/// Environment for variable bindings
#[derive(Debug, Clone)]
pub struct Environment {
    /// Variable bindings
    bindings: HashMap<String, Value>,
    /// Parent environment
    parent: Option<Arc<Mutex<Environment>>>,
}

impl Environment {
    /// Create a new empty environment
    pub fn new() -> Self {
        Environment {
            bindings: HashMap::new(),
            parent: None,
        }
    }
    
    /// Create an environment with a parent
    pub fn with_parent(parent: Arc<Mutex<Environment>>) -> Self {
        Environment {
            bindings: HashMap::new(),
            parent: Some(parent),
        }
    }
    
    /// Define a variable in this environment
    pub fn define(&mut self, name: String, value: Value) {
        self.bindings.insert(name, value);
    }

    /// Remove a variable from this environment
    pub fn undefine(&mut self, name: &str) {
        self.bindings.remove(name);
    }
    
    /// Get a variable from this environment or parent environments
    pub fn get(&self, name: &str) -> Option<Value> {
        if let Some(value) = self.bindings.get(name) {
            Some(value.clone())
        } else if let Some(ref parent) = self.parent {
            parent.lock().unwrap().get(name)
        } else {
            None
        }
    }

    /// Set a variable in this environment or parent environments
    pub fn set(&mut self, name: &str, value: Value) -> bool {
        if self.bindings.contains_key(name) {
            self.bindings.insert(name.to_string(), value);
            true
        } else if let Some(ref parent) = self.parent {
            parent.lock().unwrap().set(name, value)
        } else {
            false
        }
    }

    /// Get parent environment
    pub fn parent(&self) -> Option<Arc<Mutex<Environment>>> {
        self.parent.as_ref().map(|p| Arc::clone(p))
    }
    
    /// Get all bindings (including parent bindings)
    pub fn all_bindings(&self) -> HashMap<String, Value> {
        let mut all = HashMap::new();

        // Add parent bindings first
        if let Some(ref parent) = self.parent {
            all.extend(parent.lock().unwrap().all_bindings());
        }

        // Add local bindings (overriding parent bindings)
        all.extend(self.bindings.clone());

        all
    }
    
    /// Get local bindings only
    pub fn local_bindings(&self) -> &HashMap<String, Value> {
        &self.bindings
    }
    
    /// Check if variable is defined locally
    pub fn has_local(&self, name: &str) -> bool {
        self.bindings.contains_key(name)
    }
    
    /// Check if variable is defined (including in parent environments)
    pub fn has(&self, name: &str) -> bool {
        self.bindings.contains_key(name) ||
        self.parent.as_ref().map_or(false, |p| p.lock().unwrap().has(name))
    }
    
    /// Remove a variable from this environment
    pub fn remove(&mut self, name: &str) -> Option<Value> {
        self.bindings.remove(name)
    }
    
    /// Clear all local bindings
    pub fn clear(&mut self) {
        self.bindings.clear();
    }
    
    /// Get the number of local bindings
    pub fn len(&self) -> usize {
        self.bindings.len()
    }
    
    /// Check if environment is empty
    pub fn is_empty(&self) -> bool {
        self.bindings.is_empty()
    }
    
    /// Get all variable names (including parent environments)
    pub fn all_names(&self) -> Vec<String> {
        let mut names = Vec::new();

        // Add parent names first
        if let Some(ref parent) = self.parent {
            names.extend(parent.lock().unwrap().all_names());
        }

        // Add local names
        names.extend(self.bindings.keys().cloned());

        // Remove duplicates
        names.sort();
        names.dedup();

        names
    }
    
    /// Create a string representation of the environment
    pub fn to_string(&self) -> String {
        let mut result = String::new();
        result.push_str("Environment {\n");
        
        for (name, value) in &self.bindings {
            result.push_str(&format!("  {} = {}\n", name, value));
        }
        
        if let Some(ref parent) = self.parent {
            result.push_str("  parent: ");
            result.push_str(&parent.lock().unwrap().to_string());
        }
        
        result.push_str("}");
        result
    }
}

impl Default for Environment {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for Environment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

/// Environment builder for creating pre-configured environments
pub struct EnvironmentBuilder {
    bindings: HashMap<String, Value>,
    parent: Option<Arc<Mutex<Environment>>>,
}

impl EnvironmentBuilder {
    /// Create a new environment builder
    pub fn new() -> Self {
        EnvironmentBuilder {
            bindings: HashMap::new(),
            parent: None,
        }
    }
    
    /// Set parent environment
    pub fn with_parent(mut self, parent: Arc<Mutex<Environment>>) -> Self {
        self.parent = Some(parent);
        self
    }
    
    /// Add a binding
    pub fn bind(mut self, name: String, value: Value) -> Self {
        self.bindings.insert(name, value);
        self
    }
    
    /// Add multiple bindings
    pub fn bind_all(mut self, bindings: HashMap<String, Value>) -> Self {
        self.bindings.extend(bindings);
        self
    }
    
    /// Build the environment
    pub fn build(self) -> Environment {
        let mut env = if let Some(parent) = self.parent {
            Environment::with_parent(parent)
        } else {
            Environment::new()
        };
        
        env.bindings = self.bindings;
        env
    }
}

impl Default for EnvironmentBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Standard library environment
pub struct StandardEnvironment;

impl StandardEnvironment {
    /// Create a standard environment with built-in functions
    pub fn new() -> Environment {
        let mut env = Environment::new();
        
        // Arithmetic functions
        env.define("+".to_string(), Value::Builtin("add".to_string()));
        env.define("-".to_string(), Value::Builtin("sub".to_string()));
        env.define("*".to_string(), Value::Builtin("mul".to_string()));
        env.define("/".to_string(), Value::Builtin("div".to_string()));
        
        // Comparison functions
        env.define("=".to_string(), Value::Builtin("eq".to_string()));
        env.define("<".to_string(), Value::Builtin("lt".to_string()));
        env.define("<=".to_string(), Value::Builtin("le".to_string()));
        env.define(">".to_string(), Value::Builtin("gt".to_string()));
        env.define(">=".to_string(), Value::Builtin("ge".to_string()));
        
        // List functions
        env.define("list".to_string(), Value::Builtin("list".to_string()));
        env.define("car".to_string(), Value::Builtin("car".to_string()));
        env.define("cdr".to_string(), Value::Builtin("cdr".to_string()));
        env.define("cons".to_string(), Value::Builtin("cons".to_string()));
        env.define("length".to_string(), Value::Builtin("length".to_string()));
        
        // I/O functions
        env.define("print".to_string(), Value::Builtin("print".to_string()));
        env.define("println".to_string(), Value::Builtin("println".to_string()));
        
        // REAM integration
        env.define("spawn".to_string(), Value::Builtin("spawn".to_string()));
        env.define("send".to_string(), Value::Builtin("send".to_string()));
        env.define("receive".to_string(), Value::Builtin("receive".to_string()));
        env.define("self".to_string(), Value::Builtin("self".to_string()));

        // Hypervisor functions
        env.define("hypervisor:start".to_string(), Value::Builtin("hypervisor:start".to_string()));
        env.define("hypervisor:stop".to_string(), Value::Builtin("hypervisor:stop".to_string()));
        env.define("hypervisor:register-actor".to_string(), Value::Builtin("hypervisor:register-actor".to_string()));
        env.define("hypervisor:unregister-actor".to_string(), Value::Builtin("hypervisor:unregister-actor".to_string()));
        env.define("hypervisor:get-actor-metrics".to_string(), Value::Builtin("hypervisor:get-actor-metrics".to_string()));
        env.define("hypervisor:get-system-metrics".to_string(), Value::Builtin("hypervisor:get-system-metrics".to_string()));
        env.define("hypervisor:list-actors".to_string(), Value::Builtin("hypervisor:list-actors".to_string()));
        env.define("hypervisor:health-check".to_string(), Value::Builtin("hypervisor:health-check".to_string()));
        env.define("hypervisor:set-alert-threshold".to_string(), Value::Builtin("hypervisor:set-alert-threshold".to_string()));
        env.define("hypervisor:get-alerts".to_string(), Value::Builtin("hypervisor:get-alerts".to_string()));
        env.define("hypervisor:restart-actor".to_string(), Value::Builtin("hypervisor:restart-actor".to_string()));
        env.define("hypervisor:suspend-actor".to_string(), Value::Builtin("hypervisor:suspend-actor".to_string()));
        env.define("hypervisor:resume-actor".to_string(), Value::Builtin("hypervisor:resume-actor".to_string()));
        env.define("hypervisor:kill-actor".to_string(), Value::Builtin("hypervisor:kill-actor".to_string()));
        env.define("hypervisor:get-supervision-tree".to_string(), Value::Builtin("hypervisor:get-supervision-tree".to_string()));

        // Constants
        env.define("true".to_string(), Value::Bool(true));
        env.define("false".to_string(), Value::Bool(false));
        env.define("null".to_string(), Value::Null);
        
        env
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_environment_basic() {
        let mut env = Environment::new();
        
        env.define("x".to_string(), Value::Int(42));
        assert_eq!(env.get("x"), Some(Value::Int(42)));
        assert_eq!(env.get("y"), None);
        
        assert!(env.has("x"));
        assert!(!env.has("y"));
    }
    
    #[test]
    fn test_environment_parent() {
        let parent = Arc::new(Mutex::new(Environment::new()));
        parent.lock().unwrap().define("x".to_string(), Value::Int(42));

        let child = Environment::with_parent(Arc::clone(&parent));

        assert_eq!(child.get("x"), Some(Value::Int(42)));
        assert!(child.has("x"));
        assert!(!child.has_local("x"));
    }

    #[test]
    fn test_environment_shadowing() {
        let parent = Arc::new(Mutex::new(Environment::new()));
        parent.lock().unwrap().define("x".to_string(), Value::Int(42));

        let mut child = Environment::with_parent(Arc::clone(&parent));
        child.define("x".to_string(), Value::Int(24));

        // Child should shadow parent
        assert_eq!(child.get("x"), Some(Value::Int(24)));
        assert_eq!(parent.lock().unwrap().get("x"), Some(Value::Int(42)));
    }
    
    #[test]
    fn test_environment_builder() {
        let env = EnvironmentBuilder::new()
            .bind("x".to_string(), Value::Int(42))
            .bind("y".to_string(), Value::String("hello".to_string()))
            .build();
        
        assert_eq!(env.get("x"), Some(Value::Int(42)));
        assert_eq!(env.get("y"), Some(Value::String("hello".to_string())));
    }
    
    #[test]
    fn test_standard_environment() {
        let env = StandardEnvironment::new();
        
        assert!(env.has("+"));
        assert!(env.has("list"));
        assert!(env.has("spawn"));
        assert_eq!(env.get("true"), Some(Value::Bool(true)));
    }
    
    #[test]
    fn test_all_bindings() {
        let parent = Arc::new(Mutex::new(Environment::new()));
        parent.lock().unwrap().define("x".to_string(), Value::Int(1));
        parent.lock().unwrap().define("y".to_string(), Value::Int(2));

        let mut child = Environment::with_parent(Arc::clone(&parent));
        child.define("y".to_string(), Value::Int(3)); // Shadow parent
        child.define("z".to_string(), Value::Int(4));

        let all = child.all_bindings();
        assert_eq!(all.get("x"), Some(&Value::Int(1)));
        assert_eq!(all.get("y"), Some(&Value::Int(3))); // Child value
        assert_eq!(all.get("z"), Some(&Value::Int(4)));
    }
}
