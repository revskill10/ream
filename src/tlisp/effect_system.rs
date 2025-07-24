//! Effect System for TLISP
//! 
//! Implements effect tracking and control system for side effects with effect handlers.
//! Effects allow tracking and controlling side effects in a type-safe manner.

use std::collections::{HashMap, HashSet};
use crate::tlisp::{Value, Expr, Function};
use crate::tlisp::types::Type;
use crate::error::{TlispError, TlispResult};

/// Effect types
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Effect {
    /// Pure computation (no effects)
    Pure,
    /// I/O operations
    IO,
    /// State mutation
    State,
    /// Memory allocation
    Memory,
    /// Network operations
    Network,
    /// File system operations
    FileSystem,
    /// Database operations
    Database,
    /// Actor operations (spawn, send, receive)
    Actor,
    /// STM operations
    STM,
    /// Exception handling
    Exception,
    /// Non-determinism
    NonDet,
    /// Custom effect
    Custom(String),
}

/// Effect set
#[derive(Debug, Clone, PartialEq)]
pub struct EffectSet {
    /// Set of effects
    effects: HashSet<Effect>,
}

/// Effect handler
#[derive(Debug, Clone)]
pub struct EffectHandler {
    /// Effect being handled
    effect: Effect,
    /// Handler function
    handler: Function,
    /// Handler metadata
    metadata: HashMap<String, String>,
}

/// Effect operation
#[derive(Debug, Clone)]
pub enum EffectOperation {
    /// Perform an effect
    Perform(Effect, Value),
    /// Handle an effect
    Handle(Effect, EffectHandler),
    /// Resume computation
    Resume(Value),
    /// Abort computation
    Abort(String),
}

/// Effect computation
#[derive(Debug, Clone)]
pub struct EffectComputation {
    /// Computation expression
    expr: Expr<Type>,
    /// Required effects
    effects: EffectSet,
    /// Effect handlers
    handlers: Vec<EffectHandler>,
}

/// Effect type checker
pub struct EffectTypeChecker {
    /// Effect annotations for functions
    function_effects: HashMap<String, EffectSet>,
    /// Current effect context
    current_effects: EffectSet,
}

impl EffectSet {
    /// Create an empty effect set
    pub fn empty() -> Self {
        EffectSet {
            effects: HashSet::new(),
        }
    }

    /// Create a pure effect set
    pub fn pure() -> Self {
        let mut effects = HashSet::new();
        effects.insert(Effect::Pure);
        EffectSet { effects }
    }

    /// Create effect set with single effect
    pub fn single(effect: Effect) -> Self {
        let mut effects = HashSet::new();
        effects.insert(effect);
        EffectSet { effects }
    }

    /// Add effect to set
    pub fn add(&mut self, effect: Effect) {
        // Remove Pure if adding any other effect
        if effect != Effect::Pure {
            self.effects.remove(&Effect::Pure);
        }
        self.effects.insert(effect);
    }

    /// Remove effect from set
    pub fn remove(&mut self, effect: &Effect) {
        self.effects.remove(effect);
        // If no effects remain, add Pure
        if self.effects.is_empty() {
            self.effects.insert(Effect::Pure);
        }
    }

    /// Check if effect set contains effect
    pub fn contains(&self, effect: &Effect) -> bool {
        self.effects.contains(effect)
    }

    /// Check if effect set is pure
    pub fn is_pure(&self) -> bool {
        self.effects.len() == 1 && self.effects.contains(&Effect::Pure)
    }

    /// Union with another effect set
    pub fn union(&self, other: &EffectSet) -> EffectSet {
        let mut effects = self.effects.clone();
        for effect in &other.effects {
            if *effect != Effect::Pure {
                effects.remove(&Effect::Pure);
            }
            effects.insert(effect.clone());
        }
        EffectSet { effects }
    }

    /// Intersection with another effect set
    pub fn intersection(&self, other: &EffectSet) -> EffectSet {
        let effects = self.effects.intersection(&other.effects).cloned().collect();
        EffectSet { effects }
    }

    /// Check if this effect set is a subset of another
    pub fn is_subset_of(&self, other: &EffectSet) -> bool {
        self.effects.is_subset(&other.effects)
    }

    /// Get all effects
    pub fn effects(&self) -> &HashSet<Effect> {
        &self.effects
    }
}

impl EffectHandler {
    /// Create a new effect handler
    pub fn new(effect: Effect, handler: Function) -> Self {
        EffectHandler {
            effect,
            handler,
            metadata: HashMap::new(),
        }
    }

    /// Get handled effect
    pub fn effect(&self) -> &Effect {
        &self.effect
    }

    /// Get handler function
    pub fn handler(&self) -> &Function {
        &self.handler
    }

    /// Add metadata
    pub fn add_metadata(&mut self, key: String, value: String) {
        self.metadata.insert(key, value);
    }
}

impl EffectComputation {
    /// Create a new effect computation
    pub fn new(expr: Expr<Type>, effects: EffectSet) -> Self {
        EffectComputation {
            expr,
            effects,
            handlers: Vec::new(),
        }
    }

    /// Add effect handler
    pub fn add_handler(&mut self, handler: EffectHandler) {
        self.handlers.push(handler);
    }

    /// Get expression
    pub fn expr(&self) -> &Expr<Type> {
        &self.expr
    }

    /// Get effects
    pub fn effects(&self) -> &EffectSet {
        &self.effects
    }

    /// Get handlers
    pub fn handlers(&self) -> &[EffectHandler] {
        &self.handlers
    }

    /// Check if computation is pure
    pub fn is_pure(&self) -> bool {
        self.effects.is_pure()
    }
}

impl EffectTypeChecker {
    /// Create a new effect type checker
    pub fn new() -> Self {
        EffectTypeChecker {
            function_effects: HashMap::new(),
            current_effects: EffectSet::pure(),
        }
    }

    /// Annotate function with effects
    pub fn annotate_function(&mut self, name: String, effects: EffectSet) {
        self.function_effects.insert(name, effects);
    }

    /// Infer effects for expression
    pub fn infer_effects(&mut self, expr: &Expr<Type>) -> TlispResult<EffectSet> {
        match expr {
            Expr::Symbol(name, _) => {
                // Look up function effects
                if let Some(effects) = self.function_effects.get(name) {
                    Ok(effects.clone())
                } else {
                    Ok(EffectSet::pure())
                }
            }
            
            Expr::Number(_, _) | Expr::Float(_, _) | Expr::Bool(_, _) | Expr::String(_, _) => {
                Ok(EffectSet::pure())
            }
            
            Expr::List(exprs, _) => {
                let mut combined_effects = EffectSet::pure();
                for expr in exprs {
                    let expr_effects = self.infer_effects(expr)?;
                    combined_effects = combined_effects.union(&expr_effects);
                }
                Ok(combined_effects)
            }
            
            Expr::Lambda(_, body, _) => {
                // Lambda itself is pure, but its body may have effects
                self.infer_effects(body)
            }
            
            Expr::Application(func, args, _) => {
                let mut combined_effects = self.infer_effects(func)?;
                for arg in args {
                    let arg_effects = self.infer_effects(arg)?;
                    combined_effects = combined_effects.union(&arg_effects);
                }
                Ok(combined_effects)
            }
            
            Expr::Let(bindings, body, _) => {
                let mut combined_effects = EffectSet::pure();
                for (_, binding_expr) in bindings {
                    let binding_effects = self.infer_effects(binding_expr)?;
                    combined_effects = combined_effects.union(&binding_effects);
                }
                let body_effects = self.infer_effects(body)?;
                combined_effects = combined_effects.union(&body_effects);
                Ok(combined_effects)
            }
            
            Expr::If(cond, then_expr, else_expr, _) => {
                let cond_effects = self.infer_effects(cond)?;
                let then_effects = self.infer_effects(then_expr)?;
                let else_effects = self.infer_effects(else_expr)?;
                
                let combined = cond_effects.union(&then_effects).union(&else_effects);
                Ok(combined)
            }
            
            Expr::Quote(_, _) => Ok(EffectSet::pure()),
            
            Expr::Define(_, expr, _) => {
                self.infer_effects(expr)
            }
            
            Expr::Set(_, expr, _) => {
                let expr_effects = self.infer_effects(expr)?;
                let mut state_effects = EffectSet::single(Effect::State);
                Ok(state_effects.union(&expr_effects))
            }
        }
    }

    /// Check if expression effects are compatible with context
    pub fn check_effects(&mut self, expr: &Expr<Type>, allowed_effects: &EffectSet) -> TlispResult<()> {
        let expr_effects = self.infer_effects(expr)?;
        
        if expr_effects.is_subset_of(allowed_effects) {
            Ok(())
        } else {
            Err(TlispError::Runtime(
                format!("Effect mismatch: expression has effects {:?}, but only {:?} are allowed",
                    expr_effects.effects(), allowed_effects.effects())
            ))
        }
    }

    /// Get function effects
    pub fn get_function_effects(&self, name: &str) -> Option<&EffectSet> {
        self.function_effects.get(name)
    }

    /// Set current effect context
    pub fn set_current_effects(&mut self, effects: EffectSet) {
        self.current_effects = effects;
    }

    /// Get current effect context
    pub fn current_effects(&self) -> &EffectSet {
        &self.current_effects
    }
}

/// Effect system utilities
pub struct EffectUtils;

impl EffectUtils {
    /// Create IO effect set
    pub fn io_effects() -> EffectSet {
        let mut effects = EffectSet::empty();
        effects.add(Effect::IO);
        effects
    }

    /// Create state effect set
    pub fn state_effects() -> EffectSet {
        let mut effects = EffectSet::empty();
        effects.add(Effect::State);
        effects
    }

    /// Create actor effect set
    pub fn actor_effects() -> EffectSet {
        let mut effects = EffectSet::empty();
        effects.add(Effect::Actor);
        effects
    }

    /// Create STM effect set
    pub fn stm_effects() -> EffectSet {
        let mut effects = EffectSet::empty();
        effects.add(Effect::STM);
        effects
    }

    /// Create combined effect set
    pub fn combined_effects(effect_list: Vec<Effect>) -> EffectSet {
        let mut effects = EffectSet::empty();
        for effect in effect_list {
            effects.add(effect);
        }
        effects
    }
}

/// Effect primitive functions for TLISP
pub struct EffectPrimitives {
    /// Effect type checker
    checker: EffectTypeChecker,
}

impl EffectPrimitives {
    /// Create new effect primitives
    pub fn new() -> Self {
        EffectPrimitives {
            checker: EffectTypeChecker::new(),
        }
    }

    /// Perform effect primitive
    pub fn perform_effect(&mut self, effect: Effect, value: Value) -> TlispResult<Value> {
        // Check if effect is allowed in current context
        let current_effects = self.checker.current_effects();
        if !current_effects.contains(&effect) {
            return Err(TlispError::Runtime(
                format!("Effect {:?} not allowed in current context", effect)
            ));
        }

        // TODO: Actually perform the effect
        // For now, just return the value
        Ok(value)
    }

    /// Handle effect primitive
    pub fn handle_effect(
        &mut self,
        effect: Effect,
        handler: Function,
        computation: EffectComputation,
    ) -> TlispResult<Value> {
        // Add handler to computation
        let mut comp = computation;
        comp.add_handler(EffectHandler::new(effect, handler));

        // TODO: Execute computation with handler
        // For now, just return success
        Ok(Value::Bool(true))
    }

    /// Check effects primitive
    pub fn check_effects(&mut self, expr: Expr<Type>, allowed_effects: EffectSet) -> TlispResult<Value> {
        match self.checker.check_effects(&expr, &allowed_effects) {
            Ok(()) => Ok(Value::Bool(true)),
            Err(_) => Ok(Value::Bool(false)),
        }
    }

    /// Get effect type checker
    pub fn checker_mut(&mut self) -> &mut EffectTypeChecker {
        &mut self.checker
    }
}

impl Default for EffectTypeChecker {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for EffectPrimitives {
    fn default() -> Self {
        Self::new()
    }
}
