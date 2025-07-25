//! Extended TLISP type system with dependent types

use std::collections::HashMap;
use crate::tlisp::{Expr, Value};
use crate::error::{TlispResult, TypeError, TlispError};

/// Trait for converting Rust types to TLISP values
pub trait ToTlisp {
    fn to_tlisp(&self) -> Value;
}

/// Trait for converting TLISP values to Rust types
pub trait FromTlisp: Sized {
    fn from_tlisp(value: &Value) -> Result<Self, TypeError>;
}

// Basic implementations for primitive types
impl ToTlisp for i32 {
    fn to_tlisp(&self) -> Value {
        Value::Int(*self as i64)
    }
}

impl FromTlisp for i32 {
    fn from_tlisp(value: &Value) -> Result<Self, TypeError> {
        match value {
            Value::Int(i) => Ok(*i as i32),
            _ => Err(TypeError::TypeMismatch(
                "i32".to_string(),
                format!("{:?}", value)
            ))
        }
    }
}

impl ToTlisp for String {
    fn to_tlisp(&self) -> Value {
        Value::String(self.clone())
    }
}

impl FromTlisp for String {
    fn from_tlisp(value: &Value) -> Result<Self, TypeError> {
        match value {
            Value::String(s) => Ok(s.clone()),
            _ => Err(TypeError::TypeMismatch(
                "String".to_string(),
                format!("{:?}", value)
            ))
        }
    }
}

/// Type representation with dependent types
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Type {
    // Existing types
    /// Type variable (for inference)
    TypeVar(String),
    /// Integer type
    Int,
    /// Float type
    Float,
    /// Boolean type
    Bool,
    /// String type
    String,
    /// Symbol type
    Symbol,
    /// Unit type
    Unit,
    /// Process ID type
    Pid,
    /// List type
    List(Box<Type>),
    /// Function type: (param_types) -> return_type
    Function(Vec<Type>, Box<Type>),
    /// Unknown type (for inference)
    Unknown,
    /// Macro type
    Macro,

    // NEW: Dependent types
    /// Dependent function: (x: A) -> B(x)
    DepFunction {
        param_name: String,
        param_type: Box<Type>,
        return_type: Box<Type>, // May reference param_name
    },

    /// Type application: F(args...)
    TypeApp {
        constructor: Box<Type>,
        args: Vec<TypeTerm>,
    },

    /// Type lambda: λ(x: T) -> U
    TypeLambda {
        param: String,
        param_kind: Kind,
        body: Box<Type>,
    },

    /// Refinement type: {x: T | P(x)}
    Refinement {
        var: String,
        base_type: Box<Type>,
        predicate: Box<TypeTerm>,
    },

    /// Equality type: a = b
    Equality(Box<TypeTerm>, Box<TypeTerm>),

    /// Session type for actor protocols
    Session(Box<SessionType>),

    /// Capability type for security
    Capability(Box<CapabilityType>),

    /// Effect type for side effects
    Effect(Box<EffectType>),
}

impl std::hash::Hash for Type {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        // Only hash the basic types that we use in the cross-language bridge
        match self {
            Type::TypeVar(s) => {
                0u8.hash(state);
                s.hash(state);
            }
            Type::Int => 1u8.hash(state),
            Type::Float => 2u8.hash(state),
            Type::Bool => 3u8.hash(state),
            Type::String => 4u8.hash(state),
            Type::Unit => 5u8.hash(state),
            Type::List(inner) => {
                6u8.hash(state);
                inner.hash(state);
            }
            Type::Function(params, ret) => {
                7u8.hash(state);
                params.hash(state);
                ret.hash(state);
            }
            // Note: Tuple, Record, and Variant types don't exist in current Type enum
            Type::Pid => 11u8.hash(state),
            // For complex dependent types, just hash a discriminant
            _ => {
                99u8.hash(state);
                std::mem::discriminant(self).hash(state);
            }
        }
    }
}

impl Eq for Type {}

/// Terms that can appear in types
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum TypeTerm {
    /// Variable reference
    Var(String),

    /// Literal value
    Literal(Value),

    /// Function application
    App(Box<TypeTerm>, Vec<TypeTerm>),

    /// Lambda abstraction
    Lambda(String, Box<Type>, Box<TypeTerm>),

    /// Type annotation
    Annotated(Box<TypeTerm>, Box<Type>),
}

/// Kind system for type-level expressions
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Kind {
    /// Kind of types: *
    Type,

    /// Kind of type constructors: * -> *
    Arrow(Box<Kind>, Box<Kind>),

    /// Kind of dependent types: (x: T) -> *
    DepArrow(String, Box<Type>, Box<Kind>),

    /// Kind of constraints
    Constraint,

    /// Kind of effects
    Effect,

    /// Kind of capabilities
    Capability,
}

/// Session types for actor protocols
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum SessionType {
    /// Send message of type T, then continue with S
    Send(Box<Type>, Box<SessionType>),

    /// Receive message of type T, then continue with S
    Receive(Box<Type>, Box<SessionType>),

    /// Choice between multiple protocols
    Choose(Vec<SessionType>),

    /// Offer multiple protocols
    Offer(Vec<SessionType>),

    /// Protocol complete
    End,
}

/// Capability types for security
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum CapabilityType {
    /// Read capability for resource
    Read(String),

    /// Write capability for resource
    Write(String),

    /// Execute capability for resource
    Execute(String),

    /// Send capability for message type
    Send(Box<Type>),

    /// Receive capability for message type
    Receive(Box<Type>),

    /// Spawn capability for actor type
    Spawn(Box<Type>),

    /// Combination of capabilities
    Union(Vec<CapabilityType>),

    /// Intersection of capabilities
    Intersection(Vec<CapabilityType>),
}

/// Effect types for side effects
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum EffectType {
    /// Pure computation (no effects)
    Pure,

    /// I/O effects
    IO,

    /// State mutation effects
    State,

    /// Exception effects
    Exception,

    /// Actor communication effects
    Actor,

    /// Combination of effects
    Union(Vec<EffectType>),
}

/// Effect grades for tracking effect levels
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum EffectGrade {
    /// No effects
    Pure,

    /// Low-level effects (local state)
    Low,

    /// Medium-level effects (I/O)
    Medium,

    /// High-level effects (network, actors)
    High,
}

impl Type {
    /// Check if this is a type variable
    pub fn is_var(&self) -> bool {
        matches!(self, Type::TypeVar(_))
    }

    /// Extract type variable name
    pub fn as_var(&self) -> Option<&str> {
        match self {
            Type::TypeVar(name) => Some(name),
            _ => None,
        }
    }

    /// Check if a type occurs in this type (occurs check)
    pub fn occurs_in(&self, other: &Type) -> bool {
        match (self, other) {
            (Type::TypeVar(a), Type::TypeVar(b)) => a == b,
            (Type::List(elem), _) => elem.occurs_in(other),
            (Type::Function(params, ret), _) => {
                params.iter().any(|p| p.occurs_in(other)) || ret.occurs_in(other)
            }
            (Type::DepFunction { param_type, return_type, .. }, _) => {
                param_type.occurs_in(other) || return_type.occurs_in(other)
            }
            (Type::TypeApp { constructor, .. }, _) => {
                constructor.occurs_in(other)
            }
            (Type::TypeLambda { body, .. }, _) => {
                body.occurs_in(other)
            }
            (Type::Refinement { base_type, .. }, _) => {
                base_type.occurs_in(other)
            }
            _ => false,
        }
    }

    /// Check if this is a dependent type
    pub fn is_dependent(&self) -> bool {
        matches!(self,
            Type::DepFunction { .. } |
            Type::TypeApp { .. } |
            Type::TypeLambda { .. } |
            Type::Refinement { .. } |
            Type::Equality(_, _)
        )
    }

    /// Check if this is a session type
    pub fn is_session(&self) -> bool {
        matches!(self, Type::Session(_))
    }

    /// Check if this is a capability type
    pub fn is_capability(&self) -> bool {
        matches!(self, Type::Capability(_))
    }

    /// Check if this is an effect type
    pub fn is_effect(&self) -> bool {
        matches!(self, Type::Effect(_))
    }

    /// Get the kind of this type
    pub fn kind(&self) -> Kind {
        match self {
            Type::TypeVar(_) | Type::Int | Type::Float | Type::Bool |
            Type::String | Type::Symbol | Type::Unit | Type::Pid |
            Type::Unknown | Type::Macro => Kind::Type,

            Type::List(_) | Type::Function(_, _) | Type::DepFunction { .. } => Kind::Type,

            Type::TypeApp { constructor, .. } => {
                // Kind depends on constructor's kind and arguments
                constructor.kind()
            }

            Type::TypeLambda { param_kind, body, .. } => {
                Kind::Arrow(Box::new(param_kind.clone()), Box::new(body.kind()))
            }

            Type::Refinement { .. } => Kind::Type,
            Type::Equality(_, _) => Kind::Constraint,
            Type::Session(_) => Kind::Type,
            Type::Capability(_) => Kind::Capability,
            Type::Effect(_) => Kind::Effect,
        }
    }

    /// Get free type variables
    pub fn free_vars(&self) -> Vec<String> {
        match self {
            Type::TypeVar(name) => vec![name.clone()],
            Type::List(elem) => elem.free_vars(),
            Type::Function(params, ret) => {
                let mut vars = Vec::new();
                for param in params {
                    vars.extend(param.free_vars());
                }
                vars.extend(ret.free_vars());
                vars
            }
            Type::DepFunction { param_type, return_type, .. } => {
                let mut vars = param_type.free_vars();
                vars.extend(return_type.free_vars());
                vars
            }
            Type::TypeLambda { body, .. } => body.free_vars(),
            Type::Refinement { base_type, .. } => base_type.free_vars(),
            _ => Vec::new(),
        }
    }
}

impl std::fmt::Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::TypeVar(name) => write!(f, "{}", name),
            Type::Int => write!(f, "Int"),
            Type::Float => write!(f, "Float"),
            Type::Bool => write!(f, "Bool"),
            Type::String => write!(f, "String"),
            Type::Symbol => write!(f, "Symbol"),
            Type::Unit => write!(f, "Unit"),
            Type::Pid => write!(f, "Pid"),
            Type::List(elem) => write!(f, "[{}]", elem),
            Type::Function(params, ret) => {
                if params.is_empty() {
                    write!(f, "() -> {}", ret)
                } else {
                    write!(f, "({}) -> {}",
                           params.iter().map(|p| p.to_string()).collect::<Vec<_>>().join(", "),
                           ret)
                }
            }
            Type::DepFunction { param_name, param_type, return_type } => {
                write!(f, "({}: {}) -> {}", param_name, param_type, return_type)
            }
            Type::TypeApp { constructor, args } => {
                if args.is_empty() {
                    write!(f, "{}", constructor)
                } else {
                    write!(f, "{}({})", constructor,
                           args.iter().map(|a| format!("{:?}", a)).collect::<Vec<_>>().join(", "))
                }
            }
            Type::TypeLambda { param, body, .. } => {
                write!(f, "λ{}.{}", param, body)
            }
            Type::Refinement { var, base_type, predicate } => {
                write!(f, "{{{}: {} | {:?}}}", var, base_type, predicate)
            }
            Type::Equality(left, right) => {
                write!(f, "{:?} = {:?}", left, right)
            }
            Type::Session(session) => {
                write!(f, "Session({:?})", session)
            }
            Type::Capability(cap) => {
                write!(f, "Cap({:?})", cap)
            }
            Type::Effect(eff) => {
                write!(f, "Effect({:?})", eff)
            }
            Type::Unknown => write!(f, "Unknown"),
            Type::Macro => write!(f, "Macro"),
        }
    }
}

impl TypeTerm {
    /// Get free variables in type term
    pub fn free_vars(&self) -> Vec<String> {
        match self {
            TypeTerm::Var(name) => vec![name.clone()],
            TypeTerm::Literal(_) => Vec::new(),
            TypeTerm::App(func, args) => {
                let mut vars = func.free_vars();
                for arg in args {
                    vars.extend(arg.free_vars());
                }
                vars
            }
            TypeTerm::Lambda(param, _, body) => {
                let mut vars = body.free_vars();
                vars.retain(|v| v != param);
                vars
            }
            TypeTerm::Annotated(term, _) => term.free_vars(),
        }
    }

    /// Substitute variable in type term
    pub fn substitute(&self, var: &str, replacement: &TypeTerm) -> TypeTerm {
        match self {
            TypeTerm::Var(name) if name == var => replacement.clone(),
            TypeTerm::Var(_) => self.clone(),
            TypeTerm::Literal(_) => self.clone(),
            TypeTerm::App(func, args) => {
                let new_func = Box::new(func.substitute(var, replacement));
                let new_args = args.iter().map(|arg| arg.substitute(var, replacement)).collect();
                TypeTerm::App(new_func, new_args)
            }
            TypeTerm::Lambda(param, param_type, body) if param != var => {
                let new_body = Box::new(body.substitute(var, replacement));
                TypeTerm::Lambda(param.clone(), param_type.clone(), new_body)
            }
            TypeTerm::Lambda(_, _, _) => self.clone(), // Variable is bound
            TypeTerm::Annotated(term, ty) => {
                let new_term = Box::new(term.substitute(var, replacement));
                TypeTerm::Annotated(new_term, ty.clone())
            }
        }
    }
}

impl std::fmt::Display for TypeTerm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TypeTerm::Var(name) => write!(f, "{}", name),
            TypeTerm::Literal(value) => write!(f, "{}", value.to_string()),
            TypeTerm::App(func, args) => {
                if args.is_empty() {
                    write!(f, "{}", func)
                } else {
                    write!(f, "({} {})", func,
                           args.iter().map(|a| a.to_string()).collect::<Vec<_>>().join(" "))
                }
            }
            TypeTerm::Lambda(param, param_type, body) => {
                write!(f, "(λ({}: {}) {})", param, param_type, body)
            }
            TypeTerm::Annotated(term, ty) => {
                write!(f, "({} : {})", term, ty)
            }
        }
    }
}

impl std::fmt::Display for Kind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Kind::Type => write!(f, "*"),
            Kind::Arrow(from, to) => write!(f, "{} -> {}", from, to),
            Kind::DepArrow(param, param_type, result) => {
                write!(f, "({}: {}) -> {}", param, param_type, result)
            }
            Kind::Constraint => write!(f, "Constraint"),
            Kind::Effect => write!(f, "Effect"),
            Kind::Capability => write!(f, "Capability"),
        }
    }
}

impl SessionType {
    /// Check if session is complete
    pub fn is_complete(&self) -> bool {
        matches!(self, SessionType::End)
    }

    /// Get the next message type to send/receive
    pub fn next_message_type(&self) -> Option<&Type> {
        match self {
            SessionType::Send(ty, _) | SessionType::Receive(ty, _) => Some(ty),
            _ => None,
        }
    }
}

impl std::fmt::Display for SessionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SessionType::Send(ty, cont) => write!(f, "!{}.{}", ty, cont),
            SessionType::Receive(ty, cont) => write!(f, "?{}.{}", ty, cont),
            SessionType::Choose(choices) => {
                write!(f, "⊕{{{}}}",
                       choices.iter().map(|c| c.to_string()).collect::<Vec<_>>().join(", "))
            }
            SessionType::Offer(offers) => {
                write!(f, "&{{{}}}",
                       offers.iter().map(|o| o.to_string()).collect::<Vec<_>>().join(", "))
            }
            SessionType::End => write!(f, "end"),
        }
    }
}

/// Type substitution
#[derive(Debug, Clone)]
pub struct Substitution {
    mappings: HashMap<String, Type>,
}

impl Substitution {
    /// Create empty substitution
    pub fn new() -> Self {
        Substitution {
            mappings: HashMap::new(),
        }
    }
    
    /// Create single substitution
    pub fn single(var: String, ty: Type) -> Self {
        let mut mappings = HashMap::new();
        mappings.insert(var, ty);
        Substitution { mappings }
    }
    
    /// Apply substitution to a type
    pub fn apply(&self, ty: &Type) -> Type {
        match ty {
            Type::TypeVar(name) => {
                if let Some(replacement) = self.mappings.get(name) {
                    self.apply(replacement)
                } else {
                    ty.clone()
                }
            }
            Type::List(elem) => Type::List(Box::new(self.apply(elem))),
            Type::Function(params, ret) => {
                let new_params = params.iter().map(|p| self.apply(p)).collect();
                let new_ret = Box::new(self.apply(ret));
                Type::Function(new_params, new_ret)
            }

            // Handle dependent types
            Type::DepFunction { param_name, param_type, return_type } => {
                Type::DepFunction {
                    param_name: param_name.clone(),
                    param_type: Box::new(self.apply(param_type)),
                    return_type: Box::new(self.apply(return_type)),
                }
            }
            Type::TypeApp { constructor, args } => {
                Type::TypeApp {
                    constructor: Box::new(self.apply(constructor)),
                    args: args.iter().map(|arg| self.apply_type_term(arg)).collect(),
                }
            }
            Type::TypeLambda { param, param_kind, body } => {
                Type::TypeLambda {
                    param: param.clone(),
                    param_kind: param_kind.clone(),
                    body: Box::new(self.apply(body)),
                }
            }
            Type::Refinement { var, base_type, predicate } => {
                Type::Refinement {
                    var: var.clone(),
                    base_type: Box::new(self.apply(base_type)),
                    predicate: Box::new(self.apply_type_term(predicate)),
                }
            }
            Type::Equality(left, right) => {
                Type::Equality(
                    Box::new(self.apply_type_term(left)),
                    Box::new(self.apply_type_term(right)),
                )
            }

            // For now, don't substitute into session, capability, and effect types
            _ => ty.clone(),
        }
    }

    /// Apply substitution to a type term
    pub fn apply_type_term(&self, term: &TypeTerm) -> TypeTerm {
        match term {
            TypeTerm::Var(_name) => {
                // For now, don't substitute type terms
                term.clone()
            }
            TypeTerm::App(func, args) => {
                TypeTerm::App(
                    Box::new(self.apply_type_term(func)),
                    args.iter().map(|arg| self.apply_type_term(arg)).collect(),
                )
            }
            TypeTerm::Lambda(param, param_type, body) => {
                TypeTerm::Lambda(
                    param.clone(),
                    Box::new(self.apply(param_type)),
                    Box::new(self.apply_type_term(body)),
                )
            }
            TypeTerm::Annotated(term, ty) => {
                TypeTerm::Annotated(
                    Box::new(self.apply_type_term(term)),
                    Box::new(self.apply(ty)),
                )
            }
            _ => term.clone(),
        }
    }
    
    /// Bind a variable to a type
    pub fn bind(&mut self, var: String, ty: Type) {
        self.mappings.insert(var, ty);
    }

    /// Get the bindings as a reference to the HashMap
    pub fn bindings(&self) -> &HashMap<String, Type> {
        &self.mappings
    }

    /// Get the bindings as a mutable reference to the HashMap
    pub fn bindings_mut(&mut self) -> &mut HashMap<String, Type> {
        &mut self.mappings
    }

    /// Check if a variable is bound
    pub fn contains(&self, var: &str) -> bool {
        self.mappings.contains_key(var)
    }

    /// Get the type bound to a variable
    pub fn get(&self, var: &str) -> Option<&Type> {
        self.mappings.get(var)
    }

    /// Remove a binding
    pub fn remove(&mut self, var: &str) -> Option<Type> {
        self.mappings.remove(var)
    }

    /// Check if the substitution is empty
    pub fn is_empty(&self) -> bool {
        self.mappings.is_empty()
    }

    /// Get the number of bindings
    pub fn len(&self) -> usize {
        self.mappings.len()
    }

    /// Compose two substitutions
    pub fn compose(&self, other: &Substitution) -> Substitution {
        let mut mappings = self.mappings.clone();

        // Apply self to other's mappings
        for (var, ty) in &other.mappings {
            mappings.insert(var.clone(), self.apply(ty));
        }

        Substitution { mappings }
    }
    
    /// Apply substitution to expression
    pub fn apply_expr(&self, expr: &Expr<Type>) -> Expr<Type> {
        match expr {
            Expr::Number(n, ty) => Expr::Number(*n, self.apply(ty)),
            Expr::Float(f, ty) => Expr::Float(*f, self.apply(ty)),
            Expr::Bool(b, ty) => Expr::Bool(*b, self.apply(ty)),
            Expr::String(s, ty) => Expr::String(s.clone(), self.apply(ty)),
            Expr::Symbol(name, ty) => Expr::Symbol(name.clone(), self.apply(ty)),
            Expr::List(items, ty) => {
                let new_items = items.iter().map(|item| self.apply_expr(item)).collect();
                Expr::List(new_items, self.apply(ty))
            }
            Expr::Lambda(params, body, ty) => {
                let new_body = Box::new(self.apply_expr(body));
                Expr::Lambda(params.clone(), new_body, self.apply(ty))
            }
            Expr::Application(func, args, ty) => {
                let new_func = Box::new(self.apply_expr(func));
                let new_args = args.iter().map(|arg| self.apply_expr(arg)).collect();
                Expr::Application(new_func, new_args, self.apply(ty))
            }
            Expr::Define(name, value, ty) => {
                let new_value = Box::new(self.apply_expr(value));
                Expr::Define(name.clone(), new_value, self.apply(ty))
            }
            Expr::Set(name, value, ty) => {
                let new_value = Box::new(self.apply_expr(value));
                Expr::Set(name.clone(), new_value, self.apply(ty))
            }
            _ => expr.clone(),
        }
    }
}

/// Type checker
pub struct TypeChecker {
    /// Type environment
    env: HashMap<String, Type>,
    /// Counter for fresh variables
    var_counter: usize,
}

impl TypeChecker {
    /// Create new type checker
    pub fn new() -> Self {
        let mut checker = TypeChecker {
            env: HashMap::new(),
            var_counter: 0,
        };
        checker.add_builtins();
        checker
    }
    
    /// Define a variable in the type environment
    pub fn define(&mut self, name: String, ty: Type) {
        self.env.insert(name, ty);
    }

    /// Generate fresh type variable
    fn fresh_var(&mut self) -> Type {
        let name = format!("t{}", self.var_counter);
        self.var_counter += 1;
        Type::TypeVar(name)
    }
    
    /// Infer the type of an expression
    pub fn infer(&mut self, expr: &Expr<()>) -> TlispResult<Expr<Type>> {
        let (typed_expr, _) = self.infer_with_subst(expr, &Substitution::new())?;
        Ok(typed_expr)
    }
    
    /// Infer type with substitution
    fn infer_with_subst(&mut self, expr: &Expr<()>, subst: &Substitution) -> TlispResult<(Expr<Type>, Substitution)> {
        match expr {
            Expr::Number(n, _) => {
                Ok((Expr::Number(*n, Type::Int), subst.clone()))
            }
            Expr::Float(f, _) => {
                Ok((Expr::Float(*f, Type::Float), subst.clone()))
            }
            Expr::Bool(b, _) => {
                Ok((Expr::Bool(*b, Type::Bool), subst.clone()))
            }
            Expr::String(s, _) => {
                Ok((Expr::String(s.clone(), Type::String), subst.clone()))
            }
            Expr::Symbol(name, _) => {
                let ty = self.env.get(name).cloned();
                if let Some(ty) = ty {
                    let instantiated = self.instantiate(&ty);
                    Ok((Expr::Symbol(name.clone(), instantiated), subst.clone()))
                } else {
                    Err(TypeError::UndefinedVariable(name.clone()).into())
                }
            }
            Expr::List(items, _) => {
                if items.is_empty() {
                    let elem_ty = self.fresh_var();
                    let list_ty = Type::List(Box::new(elem_ty));
                    Ok((Expr::List(Vec::new(), list_ty), subst.clone()))
                } else {
                    // Check if this list should be treated as a function application
                    // If the first element is a symbol that refers to a function, treat as application
                    if let Expr::Symbol(name, _) = &items[0] {
                        if let Some(func_ty) = self.env.get(name).cloned() {
                            // If it's a function type, treat this list as an application
                            if matches!(func_ty, Type::Function(_, _)) {
                                // Convert List to Application and type-check as such
                                let func = Box::new(items[0].clone());
                                let args = items[1..].to_vec();
                                let app_expr = Expr::Application(func, args, ());
                                return self.infer_with_subst(&app_expr, subst);
                            }
                        }
                    }

                    // Otherwise, type-check as a regular list
                    let (first_typed, mut current_subst) = self.infer_with_subst(&items[0], subst)?;
                    let elem_ty = first_typed.get_type().clone();

                    let mut typed_items = vec![first_typed];

                    for item in &items[1..] {
                        let (typed_item, new_subst) = self.infer_with_subst(item, &current_subst)?;
                        let unified_subst = self.unify(&elem_ty, typed_item.get_type())?;
                        current_subst = current_subst.compose(&new_subst).compose(&unified_subst);
                        typed_items.push(typed_item);
                    }

                    let final_elem_ty = current_subst.apply(&elem_ty);
                    let list_ty = Type::List(Box::new(final_elem_ty));

                    Ok((Expr::List(typed_items, list_ty), current_subst))
                }
            }
            Expr::Lambda(params, body, _) => {
                // Create fresh type variables for parameters
                let param_types: Vec<Type> = params.iter().map(|_| self.fresh_var()).collect();
                
                // Extend environment with parameter types
                for (param, ty) in params.iter().zip(param_types.iter()) {
                    self.env.insert(param.clone(), ty.clone());
                }

                // Infer body type
                let (typed_body, body_subst) = self.infer_with_subst(body, subst)?;
                let body_ty = typed_body.get_type().clone();

                // Remove parameter bindings from environment
                for param in params {
                    self.env.remove(param);
                }
                
                // Apply substitution to parameter types
                let final_param_types: Vec<Type> = param_types.iter()
                    .map(|ty| body_subst.apply(ty))
                    .collect();
                
                let func_ty = Type::Function(final_param_types, Box::new(body_ty));
                
                Ok((Expr::Lambda(params.clone(), Box::new(typed_body), func_ty), body_subst))
            }
            Expr::Application(func, args, _) => {
                // Special handling for built-in functions
                if let Expr::Symbol(name, _) = func.as_ref() {
                    // Special handling for begin (sequential evaluation)
                    if name == "begin" {
                        if args.is_empty() {
                            // Empty begin returns unit
                            let typed_func = Expr::Symbol(name.clone(), Type::Function(vec![], Box::new(Type::Unit)));
                            return Ok((Expr::Application(Box::new(typed_func), vec![], Type::Unit), subst.clone()));
                        }

                        let mut current_subst = subst.clone();
                        let mut typed_args = Vec::new();
                        let mut last_ty = Type::Unit;

                        // Type check all arguments, return type of last one
                        for arg in args {
                            let (typed_arg, arg_subst) = self.infer_with_subst(arg, &current_subst)?;
                            current_subst = current_subst.compose(&arg_subst);
                            last_ty = typed_arg.get_type().clone();
                            typed_args.push(typed_arg);
                        }

                        let typed_func = Expr::Symbol(name.clone(), Type::Function(vec![], Box::new(last_ty.clone())));
                        return Ok((Expr::Application(Box::new(typed_func), typed_args, last_ty), current_subst));
                    }

                    // Special handling for string-append (variadic)
                    if name == "string-append" {
                        let mut current_subst = subst.clone();
                        let mut typed_args = Vec::new();

                        if args.is_empty() {
                            // Empty string-append returns empty string
                            let string_ty = Type::String;
                            let typed_func = Expr::Symbol(name.clone(), Type::Function(vec![], Box::new(string_ty.clone())));
                            return Ok((Expr::Application(Box::new(typed_func), vec![], string_ty), current_subst));
                        }

                        // All arguments should be strings
                        for arg in args {
                            let (typed_arg, arg_subst) = self.infer_with_subst(arg, &current_subst)?;
                            current_subst = current_subst.compose(&arg_subst);
                            let unified_subst = self.unify(&Type::String, typed_arg.get_type())?;
                            current_subst = current_subst.compose(&unified_subst);
                            typed_args.push(typed_arg);
                        }

                        let func_ty = Type::Function(vec![Type::String; args.len()], Box::new(Type::String));
                        let typed_func = Expr::Symbol(name.clone(), func_ty);

                        return Ok((Expr::Application(Box::new(typed_func), typed_args, Type::String), current_subst));
                    }

                    // Special handling for list constructor (variadic)
                    if name == "list" {
                        let mut current_subst = subst.clone();
                        let mut typed_args = Vec::new();
                        
                        if args.is_empty() {
                            // Empty list
                            let elem_ty = self.fresh_var();
                            let list_ty = Type::List(Box::new(elem_ty));
                            let typed_func = Expr::Symbol(name.clone(), Type::Function(vec![], Box::new(list_ty.clone())));
                            return Ok((Expr::Application(Box::new(typed_func), vec![], list_ty), current_subst));
                        }
                        
                        // Infer first argument type
                        let (first_typed, first_subst) = self.infer_with_subst(&args[0], &current_subst)?;
                        current_subst = current_subst.compose(&first_subst);
                        let elem_ty = first_typed.get_type().clone();
                        typed_args.push(first_typed);
                        
                        // All other arguments should have the same type
                        for arg in &args[1..] {
                            let (typed_arg, arg_subst) = self.infer_with_subst(arg, &current_subst)?;
                            current_subst = current_subst.compose(&arg_subst);
                            let unified_subst = self.unify(&elem_ty, typed_arg.get_type())?;
                            current_subst = current_subst.compose(&unified_subst);
                            typed_args.push(typed_arg);
                        }
                        
                        let final_elem_ty = current_subst.apply(&elem_ty);
                        let list_ty = Type::List(Box::new(final_elem_ty.clone()));
                        let func_ty = Type::Function(vec![final_elem_ty; args.len()], Box::new(list_ty.clone()));
                        let typed_func = Expr::Symbol(name.clone(), func_ty);
                        
                        return Ok((Expr::Application(Box::new(typed_func), typed_args, list_ty), current_subst));
                    }
                }
                
                // Infer function type
                let (typed_func, mut current_subst) = self.infer_with_subst(func, subst)?;
                
                // Infer argument types
                let mut typed_args = Vec::new();
                for arg in args {
                    let (typed_arg, arg_subst) = self.infer_with_subst(arg, &current_subst)?;
                    current_subst = current_subst.compose(&arg_subst);
                    typed_args.push(typed_arg);
                }

                // Create expected function type
                let result_ty = self.fresh_var();
                let arg_types: Vec<Type> = typed_args.iter().map(|arg| arg.get_type().clone()).collect();

                // Special handling for variadic functions like +
                let expected_func_ty = if let Expr::Symbol(name, _) = &typed_func {
                    if name == "+" && arg_types.len() > 2 {
                        // For +, allow any number of arguments, all should be numbers
                        Type::Function(vec![Type::Int, Type::Int], Box::new(Type::Int))
                    } else {
                        Type::Function(arg_types, Box::new(result_ty.clone()))
                    }
                } else {
                    Type::Function(arg_types, Box::new(result_ty.clone()))
                };

                // Unify function types
                let func_ty = typed_func.get_type().clone();
                let unified_subst = if let Expr::Symbol(name, _) = &typed_func {
                    if name == "+" && typed_args.len() > 2 {
                        // For variadic +, just check that all args are numbers and return Int
                        for arg in &typed_args {
                            let arg_ty = arg.get_type();
                            if !matches!(arg_ty, Type::Int | Type::Float) {
                                return Err(TlispError::Type(TypeError::Mismatch {
                                    expected: "number".to_string(),
                                    actual: format!("{:?}", arg_ty),
                                }));
                            }
                        }
                        Substitution::new()
                    } else {
                        self.unify(&current_subst.apply(&func_ty), &expected_func_ty)?
                    }
                } else {
                    self.unify(&current_subst.apply(&func_ty), &expected_func_ty)?
                };
                let final_subst = current_subst.compose(&unified_subst);
                let final_result_ty = if let Expr::Symbol(name, _) = &typed_func {
                    if name == "+" && typed_args.len() > 2 {
                        Type::Int // + always returns Int for now
                    } else {
                        final_subst.apply(&result_ty)
                    }
                } else {
                    final_subst.apply(&result_ty)
                };

                Ok((Expr::Application(Box::new(typed_func), typed_args, final_result_ty), final_subst))
            }
            Expr::Define(name, value_expr, _) => {
                // Infer the type of the value expression
                let (typed_value, value_subst) = self.infer_with_subst(value_expr, subst)?;
                let value_ty = typed_value.get_type().clone();

                // Add the binding to the environment
                self.env.insert(name.clone(), value_ty.clone());

                // Return the define expression with the value type
                Ok((Expr::Define(name.clone(), Box::new(typed_value), value_ty), value_subst))
            }
            Expr::Set(name, value_expr, _) => {
                // Check that the variable exists
                let var_ty = self.env.get(name).cloned()
                    .ok_or_else(|| TypeError::UndefinedVariable(name.clone()))?;

                // Infer the type of the value expression
                let (typed_value, value_subst) = self.infer_with_subst(value_expr, subst)?;
                let value_ty = typed_value.get_type().clone();

                // Unify the variable type with the value type
                let unified_subst = self.unify(&var_ty, &value_ty)?;
                let final_subst = value_subst.compose(&unified_subst);

                // Return the set expression with the variable type
                Ok((Expr::Set(name.clone(), Box::new(typed_value), var_ty), final_subst))
            }
            _ => {
                // For other expression types, use a placeholder
                let ty = self.fresh_var();
                Ok((expr.clone().map_type(|_| ty.clone()), subst.clone()))
            }
        }
    }
    
    /// Unify two types
    pub fn unify(&mut self, t1: &Type, t2: &Type) -> TlispResult<Substitution> {
        match (t1, t2) {
            (Type::TypeVar(a), Type::TypeVar(b)) if a == b => Ok(Substitution::new()),
            (Type::TypeVar(a), ty) | (ty, Type::TypeVar(a)) => {
                if ty.occurs_in(&Type::TypeVar(a.clone())) {
                    Err(TypeError::OccursCheck {
                        var: a.clone(),
                        ty: ty.to_string()
                    }.into())
                } else {
                    Ok(Substitution::single(a.clone(), ty.clone()))
                }
            }
            (Type::List(a), Type::List(b)) => self.unify(a, b),
            (Type::Function(params1, ret1), Type::Function(params2, ret2)) => {
                if params1.len() != params2.len() {
                    return Err(TypeError::ArityMismatch {
                        expected: params1.len(),
                        actual: params2.len()
                    }.into());
                }

                let mut subst = Substitution::new();

                // Unify parameters
                for (p1, p2) in params1.iter().zip(params2.iter()) {
                    let param_subst = self.unify(&subst.apply(p1), &subst.apply(p2))?;
                    subst = subst.compose(&param_subst);
                }

                // Unify return types
                let ret_subst = self.unify(&subst.apply(ret1), &subst.apply(ret2))?;
                subst = subst.compose(&ret_subst);

                Ok(subst)
            }

            // Handle dependent function types
            (Type::DepFunction { param_name: n1, param_type: p1, return_type: r1 },
             Type::DepFunction { param_name: n2, param_type: p2, return_type: r2 }) => {
                if n1 != n2 {
                    // For now, require parameter names to match
                    return Err(TypeError::UnificationFailure {
                        left: t1.to_string(),
                        right: t2.to_string()
                    }.into());
                }

                let mut subst = Substitution::new();

                // Unify parameter types
                let param_subst = self.unify(&subst.apply(p1), &subst.apply(p2))?;
                subst = subst.compose(&param_subst);

                // Unify return types
                let ret_subst = self.unify(&subst.apply(r1), &subst.apply(r2))?;
                subst = subst.compose(&ret_subst);

                Ok(subst)
            }

            // Handle type applications
            (Type::TypeApp { constructor: c1, args: a1 },
             Type::TypeApp { constructor: c2, args: a2 }) => {
                if a1.len() != a2.len() {
                    return Err(TypeError::ArityMismatch {
                        expected: a1.len(),
                        actual: a2.len()
                    }.into());
                }

                let mut subst = Substitution::new();

                // Unify constructors
                let constr_subst = self.unify(&subst.apply(c1), &subst.apply(c2))?;
                subst = subst.compose(&constr_subst);

                // For now, don't unify type term arguments (would need type term unification)

                Ok(subst)
            }

            (a, b) if a == b => Ok(Substitution::new()),
            (a, b) => Err(TypeError::UnificationFailure {
                left: a.to_string(),
                right: b.to_string()
            }.into()),
        }
    }
    
    /// Instantiate a type scheme (creates fresh type variables)
    fn instantiate(&mut self, ty: &Type) -> Type {
        match ty {
            Type::TypeVar(_) => self.fresh_var(),
            Type::List(elem) => Type::List(Box::new(self.instantiate(elem))),
            Type::Function(params, ret) => {
                let new_params = params.iter().map(|p| self.instantiate(p)).collect();
                let new_ret = Box::new(self.instantiate(ret));
                Type::Function(new_params, new_ret)
            }
            _ => ty.clone(),
        }
    }
    
    /// Add built-in function types
    fn add_builtins(&mut self) {
        // Arithmetic functions: (Int, Int) -> Int
        let int_binop = Type::Function(vec![Type::Int, Type::Int], Box::new(Type::Int));
        self.env.insert("+".to_string(), int_binop.clone());
        self.env.insert("-".to_string(), int_binop.clone());
        self.env.insert("*".to_string(), int_binop.clone());
        self.env.insert("/".to_string(), int_binop);
        
        // Comparison functions: (Int, Int) -> Bool
        let int_cmp = Type::Function(vec![Type::Int, Type::Int], Box::new(Type::Bool));
        self.env.insert("=".to_string(), int_cmp.clone());
        self.env.insert("<".to_string(), int_cmp.clone());
        self.env.insert("<=".to_string(), int_cmp.clone());
        self.env.insert(">".to_string(), int_cmp.clone());
        self.env.insert(">=".to_string(), int_cmp);
        
        // List functions
        let list_var = Type::TypeVar("a".to_string());
        let list_ty = Type::List(Box::new(list_var.clone()));
        
        // Special handling for variadic list constructor
        // We'll handle this in the type inference logic
        self.env.insert("list".to_string(), Type::Function(vec![list_var.clone()], Box::new(list_ty.clone())));
        self.env.insert("car".to_string(), Type::Function(vec![list_ty.clone()], Box::new(list_var.clone())));
        self.env.insert("cdr".to_string(), Type::Function(vec![list_ty.clone()], Box::new(list_ty.clone())));
        self.env.insert("cons".to_string(), Type::Function(vec![list_var.clone(), list_ty.clone()], Box::new(list_ty.clone())));
        self.env.insert("append".to_string(), Type::Function(vec![list_ty.clone(), list_ty.clone()], Box::new(list_ty.clone())));
        self.env.insert("length".to_string(), Type::Function(vec![list_ty], Box::new(Type::Int)));
        
        // I/O functions
        self.env.insert("print".to_string(), Type::Function(vec![Type::TypeVar("a".to_string())], Box::new(Type::Unit)));
        self.env.insert("println".to_string(), Type::Function(vec![Type::TypeVar("a".to_string())], Box::new(Type::Unit)));
        self.env.insert("newline".to_string(), Type::Function(vec![], Box::new(Type::Unit)));

        // String functions - using variadic approach with multiple overloads
        // For now, we'll support up to 5 arguments by creating multiple type signatures
        // This is a workaround until we implement proper variadic types
        self.env.insert("string-append".to_string(), Type::Function(vec![Type::String], Box::new(Type::String)));
        self.env.insert("number->string".to_string(), Type::Function(vec![Type::Int], Box::new(Type::String)));
        self.env.insert("symbol->string".to_string(), Type::Function(vec![Type::Symbol], Box::new(Type::String)));
        self.env.insert("list->string".to_string(), Type::Function(vec![Type::List(Box::new(Type::TypeVar("a".to_string())))], Box::new(Type::String)));

        // Type predicates
        self.env.insert("null?".to_string(), Type::Function(vec![Type::TypeVar("a".to_string())], Box::new(Type::Bool)));
        self.env.insert("number?".to_string(), Type::Function(vec![Type::TypeVar("a".to_string())], Box::new(Type::Bool)));
        self.env.insert("string?".to_string(), Type::Function(vec![Type::TypeVar("a".to_string())], Box::new(Type::Bool)));
        self.env.insert("symbol?".to_string(), Type::Function(vec![Type::TypeVar("a".to_string())], Box::new(Type::Bool)));
        self.env.insert("boolean?".to_string(), Type::Function(vec![Type::TypeVar("a".to_string())], Box::new(Type::Bool)));
        self.env.insert("list?".to_string(), Type::Function(vec![Type::TypeVar("a".to_string())], Box::new(Type::Bool)));
        self.env.insert("equal?".to_string(), Type::Function(vec![Type::TypeVar("a".to_string()), Type::TypeVar("a".to_string())], Box::new(Type::Bool)));

        // Math functions
        self.env.insert("modulo".to_string(), Type::Function(vec![Type::Int, Type::Int], Box::new(Type::Int)));
        self.env.insert("floor".to_string(), Type::Function(vec![Type::TypeVar("a".to_string())], Box::new(Type::Int)));
        self.env.insert("sqrt".to_string(), Type::Function(vec![Type::TypeVar("a".to_string())], Box::new(Type::Float)));
        self.env.insert("abs".to_string(), Type::Function(vec![Type::TypeVar("a".to_string())], Box::new(Type::TypeVar("a".to_string()))));

        // System functions
        self.env.insert("error".to_string(), Type::Function(vec![Type::String], Box::new(Type::TypeVar("a".to_string()))));
        self.env.insert("current-time".to_string(), Type::Function(vec![], Box::new(Type::Int)));
        self.env.insert("random".to_string(), Type::Function(vec![Type::Int], Box::new(Type::Int)));

        // Control flow - begin is special, we'll handle it in type inference
        // For now, give it a polymorphic type that can accept any arguments
        self.env.insert("begin".to_string(), Type::Function(vec![], Box::new(Type::TypeVar("a".to_string()))));

        // REAM functions
        let func_ty = Type::Function(vec![], Box::new(Type::TypeVar("a".to_string())));
        self.env.insert("spawn".to_string(), Type::Function(vec![func_ty], Box::new(Type::Pid)));
        self.env.insert("send".to_string(), Type::Function(vec![Type::Pid, Type::TypeVar("a".to_string())], Box::new(Type::Unit)));
        self.env.insert("receive".to_string(), Type::Function(vec![], Box::new(Type::TypeVar("a".to_string()))));
        self.env.insert("self".to_string(), Type::Function(vec![], Box::new(Type::Pid)));
    }
}

impl Default for TypeChecker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_type_operations() {
        let int_type = Type::Int;
        assert_eq!(int_type.kind(), Kind::Type);
        assert!(!int_type.is_dependent());

        let type_var = Type::TypeVar("a".to_string());
        assert!(type_var.is_var());
        assert_eq!(type_var.as_var(), Some("a"));
    }

    #[test]
    fn test_dependent_function_creation() {
        let dep_func = Type::DepFunction {
            param_name: "n".to_string(),
            param_type: Box::new(Type::Int),
            return_type: Box::new(Type::List(Box::new(Type::String))),
        };

        assert!(dep_func.is_dependent());
        assert_eq!(dep_func.kind(), Kind::Type);
    }

    #[test]
    fn test_type_lambda_creation() {
        let type_lambda = Type::TypeLambda {
            param: "T".to_string(),
            param_kind: Kind::Type,
            body: Box::new(Type::List(Box::new(Type::TypeVar("T".to_string())))),
        };

        assert!(type_lambda.is_dependent());
        let expected_kind = Kind::Arrow(Box::new(Kind::Type), Box::new(Kind::Type));
        assert_eq!(type_lambda.kind(), expected_kind);
    }

    #[test]
    fn test_refinement_type_creation() {
        let predicate = TypeTerm::App(
            Box::new(TypeTerm::Var(">".to_string())),
            vec![
                TypeTerm::Var("x".to_string()),
                TypeTerm::Literal(Value::Int(0)),
            ],
        );

        let refinement = Type::Refinement {
            var: "x".to_string(),
            base_type: Box::new(Type::Int),
            predicate: Box::new(predicate),
        };

        assert!(refinement.is_dependent());
        assert_eq!(refinement.kind(), Kind::Type);
    }

    #[test]
    fn test_session_type_creation() {
        let session = SessionType::Send(
            Box::new(Type::Int),
            Box::new(SessionType::End),
        );

        let session_type = Type::Session(Box::new(session));
        assert!(session_type.is_session());
        assert!(!session_type.is_dependent());
    }

    #[test]
    fn test_capability_type_creation() {
        let cap = CapabilityType::Read("file.txt".to_string());
        let cap_type = Type::Capability(Box::new(cap));

        assert!(cap_type.is_capability());
        assert_eq!(cap_type.kind(), Kind::Capability);
    }

    #[test]
    fn test_effect_type_creation() {
        let effect = EffectType::IO;
        let effect_type = Type::Effect(Box::new(effect));

        assert!(effect_type.is_effect());
        assert_eq!(effect_type.kind(), Kind::Effect);
    }
}