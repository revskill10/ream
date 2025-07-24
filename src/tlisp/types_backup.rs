//! Type system for TLISP with Hindley-Milner inference

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::tlisp::Expr;
use crate::error::{TypeError, TlispResult, TlispError};

/// Type representation
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Type {
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
    /// List type with element type
    List(Box<Type>),
    /// Function type with parameter and return types
    Function(Vec<Type>, Box<Type>),
    /// Process ID type
    Pid,
    /// Unit type
    Unit,
    /// Type variable for inference
    TypeVar(String),
}

impl Type {
    /// Check if this is a type variable
    pub fn is_var(&self) -> bool {
        matches!(self, Type::TypeVar(_))
    }
    
    /// Get type variable name if this is a type variable
    pub fn as_var(&self) -> Option<&str> {
        match self {
            Type::TypeVar(name) => Some(name),
            _ => None,
        }
    }
    
    /// Check if this type occurs in another type (occurs check)
    pub fn occurs_in(&self, other: &Type) -> bool {
        match (self, other) {
            (Type::TypeVar(a), Type::TypeVar(b)) => a == b,
            (Type::TypeVar(_), Type::List(inner)) => self.occurs_in(inner),
            (Type::TypeVar(_), Type::Function(params, ret)) => {
                params.iter().any(|p| self.occurs_in(p)) || self.occurs_in(ret)
            }
            _ => false,
        }
    }
    
    /// Get free type variables
    pub fn free_vars(&self) -> Vec<String> {
        match self {
            Type::TypeVar(name) => vec![name.clone()],
            Type::List(inner) => inner.free_vars(),
            Type::Function(params, ret) => {
                let mut vars = Vec::new();
                for param in params {
                    vars.extend(param.free_vars());
                }
                vars.extend(ret.free_vars());
                vars.sort();
                vars.dedup();
                vars
            }
            _ => Vec::new(),
        }
    }
}

impl std::fmt::Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Int => write!(f, "Int"),
            Type::Float => write!(f, "Float"),
            Type::Bool => write!(f, "Bool"),
            Type::String => write!(f, "String"),
            Type::Symbol => write!(f, "Symbol"),
            Type::List(inner) => write!(f, "[{}]", inner),
            Type::Function(params, ret) => {
                if params.is_empty() {
                    write!(f, "() -> {}", ret)
                } else {
                    let param_str = params.iter()
                        .map(|p| p.to_string())
                        .collect::<Vec<_>>()
                        .join(" -> ");
                    write!(f, "{} -> {}", param_str, ret)
                }
            }
            Type::Pid => write!(f, "Pid"),
            Type::Unit => write!(f, "()"),
            Type::TypeVar(name) => write!(f, "{}", name),
        }
    }
}

/// Type substitution for unification
#[derive(Debug, Clone, Default)]
pub struct Substitution {
    /// Variable mappings
    mappings: HashMap<String, Type>,
}

impl Substitution {
    /// Create a new empty substitution
    pub fn new() -> Self {
        Substitution {
            mappings: HashMap::new(),
        }
    }
    
    /// Create a substitution with a single mapping
    pub fn single(var: String, ty: Type) -> Self {
        let mut mappings = HashMap::new();
        mappings.insert(var, ty);
        Substitution { mappings }
    }
    
    /// Compose two substitutions
    pub fn compose(&self, other: &Substitution) -> Substitution {
        let mut result = self.mappings.clone();
        
        // Apply self to other's mappings
        for (var, ty) in &other.mappings {
            result.insert(var.clone(), self.apply(ty));
        }
        
        Substitution { mappings: result }
    }
    
    /// Apply substitution to a type
    pub fn apply(&self, ty: &Type) -> Type {
        match ty {
            Type::TypeVar(name) => {
                self.mappings.get(name).cloned().unwrap_or_else(|| ty.clone())
            }
            Type::List(inner) => Type::List(Box::new(self.apply(inner))),
            Type::Function(params, ret) => {
                let new_params = params.iter().map(|p| self.apply(p)).collect();
                let new_ret = Box::new(self.apply(ret));
                Type::Function(new_params, new_ret)
            }
            _ => ty.clone(),
        }
    }
    
    /// Apply substitution to an expression
    pub fn apply_expr(&self, expr: &Expr<Type>) -> Expr<Type> {
        expr.clone().map_type(|ty| self.apply(&ty))
    }
    
    /// Check if substitution is empty
    pub fn is_empty(&self) -> bool {
        self.mappings.is_empty()
    }
    
    /// Get the mappings
    pub fn mappings(&self) -> &HashMap<String, Type> {
        &self.mappings
    }
}

/// Type checker with Hindley-Milner inference
pub struct TypeChecker {
    /// Next type variable counter
    next_var: u32,
    /// Type environment
    env: HashMap<String, Type>,
}

impl TypeChecker {
    /// Create a new type checker
    pub fn new() -> Self {
        let mut checker = TypeChecker {
            next_var: 0,
            env: HashMap::new(),
        };
        
        // Add built-in types
        checker.add_builtins();
        checker
    }
    
    /// Generate a fresh type variable
    pub fn fresh_var(&mut self) -> Type {
        let var = format!("t{}", self.next_var);
        self.next_var += 1;
        Type::TypeVar(var)
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
                    let (first_typed, mut current_subst) = self.infer_with_subst(&items[0], subst)?;
                    let elem_ty = first_typed.get_type().clone();
                    
                    let mut typed_items = vec![first_typed];
                    
                    for item in &items[1..] {
                        let (typed_item, _new_subst) = self.infer_with_subst(item, &current_subst)?;
                        let unified_subst = self.unify(&elem_ty, typed_item.get_type())?;
                        current_subst = current_subst.compose(&unified_subst);
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
                
                // Extend environment with parameter types (preserve built-ins)
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
                
                // Create multi-argument function type: (Int, Int, ...) -> ReturnType
                let func_ty = Type::Function(final_param_types, Box::new(body_ty));
                
                Ok((Expr::Lambda(params.clone(), Box::new(typed_body), func_ty), body_subst))
            }
            Expr::Application(func, args, _) => {
                // Special handling for built-in functions and lambdas
                if let Expr::Symbol(name, _) = func.as_ref() {
                    let mut current_subst = subst.clone();

                    // Handle list specially (variadic)
                    if name == "list" {
                        // Handle list specially - all arguments should have the same type
                        let mut typed_args = Vec::new();

                        if args.is_empty() {
                            // Empty list - use a fresh type variable
                            let elem_ty = self.fresh_var();
                            let list_ty = Type::List(Box::new(elem_ty));
                            let typed_func = Expr::Symbol(name.clone(), Type::Function(vec![], Box::new(list_ty.clone())));
                            return Ok((Expr::Application(Box::new(typed_func), vec![], list_ty), current_subst));
                        }

                        // Infer type of first argument
                        let (first_typed, first_subst) = self.infer_with_subst(&args[0], &current_subst)?;
                        let elem_ty = first_typed.get_type().clone();
                        current_subst = current_subst.compose(&first_subst);
                        typed_args.push(first_typed);

                        // All other arguments should have the same type
                        for arg in &args[1..] {
                            let (typed_arg, arg_subst) = self.infer_with_subst(arg, &current_subst)?;
                            let unified_subst = self.unify(&elem_ty, typed_arg.get_type())?;
                            current_subst = current_subst.compose(&arg_subst).compose(&unified_subst);
                            typed_args.push(typed_arg);
                        }

                        let list_ty = Type::List(Box::new(current_subst.apply(&elem_ty)));
                        let func_ty = Type::Function(vec![elem_ty; args.len()], Box::new(list_ty.clone()));
                        let typed_func = Expr::Symbol(name.clone(), func_ty);

                        return Ok((Expr::Application(Box::new(typed_func), typed_args, list_ty), current_subst));
                    }

                    // Handle built-in arithmetic and comparison operators specially
                    if matches!(name.as_str(), "+" | "-" | "*" | "/" | "=" | "<" | "<=" | ">" | ">=") {
                        if args.len() != 2 {
                            return Err(TlispError::Type(TypeError::ArityMismatch {
                                expected: 2,
                                actual: args.len(),
                            }));
                        }

                        // Infer argument types
                        let mut typed_args = Vec::new();
                        for arg in args {
                            let (typed_arg, arg_subst) = self.infer_with_subst(arg, &current_subst)?;
                            current_subst = current_subst.compose(&arg_subst);
                            typed_args.push(typed_arg);
                        }

                        // Get the expected function type from environment
                        let func_ty = self.env.get(name).cloned().unwrap_or_else(|| {
                            // Default to Int -> Int -> Int for arithmetic operators
                            Type::Function(vec![Type::Int, Type::Int], Box::new(Type::Int))
                        });

                        let typed_func = Expr::Symbol(name.clone(), func_ty.clone());

                        // Extract return type from function type
                        let return_ty = match &func_ty {
                            Type::Function(_, ret) => (**ret).clone(),
                            _ => Type::Int, // fallback
                        };

                        return Ok((Expr::Application(Box::new(typed_func), typed_args, return_ty), current_subst));
                    }

                }

                // General function application
                let (typed_func, mut current_subst) = self.infer_with_subst(func, subst)?;
                let func_ty = typed_func.get_type().clone();

                // Debug output for function applications
                if let Expr::Symbol(name, _) = func.as_ref() {
                    if matches!(name.as_str(), "+" | "-" | "*" | "/" | "=" | "<" | "<=" | ">" | ">=") {
                        eprintln!("DEBUG: General application for arithmetic operator {}", name);
                    }
                }

                // Infer argument types
                let mut typed_args = Vec::new();
                for arg in args {
                    let (typed_arg, arg_subst) = self.infer_with_subst(arg, &current_subst)?;
                    current_subst = current_subst.compose(&arg_subst);
                    typed_args.push(typed_arg);
                }

                // Special handling for direct lambda application
                if let Expr::Lambda(_, _, _) = func.as_ref() {
                    // For lambda applications, always use multi-argument approach
                    let (typed_func, func_subst) = self.infer_with_subst(func, subst)?;
                    let mut current_subst = func_subst;

                    let mut typed_args = Vec::new();
                    let mut arg_types = Vec::new();

                    for arg in args {
                        let (typed_arg, arg_subst) = self.infer_with_subst(arg, &current_subst)?;
                        current_subst = current_subst.compose(&arg_subst);
                        arg_types.push(typed_arg.get_type().clone());
                        typed_args.push(typed_arg);
                    }

                    let result_ty = self.fresh_var();
                    let func_ty = typed_func.get_type().clone();
                    let expected_func_ty = Type::Function(arg_types, Box::new(result_ty.clone()));

                    let unified_subst = self.unify(&current_subst.apply(&func_ty), &expected_func_ty)?;
                    let final_subst = current_subst.compose(&unified_subst);
                    let final_result_ty = final_subst.apply(&result_ty);

                    return Ok((Expr::Application(Box::new(typed_func), typed_args, final_result_ty.clone()), final_subst));
                }

                // Try multi-argument application first, then fall back to curried
                let result_ty = self.fresh_var();
                let arg_types: Vec<Type> = typed_args.iter().map(|arg| arg.get_type().clone()).collect();
                let expected_func_ty = Type::Function(arg_types, Box::new(result_ty.clone()));

                // Special handling for arithmetic operators - force multi-argument application
                if let Expr::Symbol(name, _) = func.as_ref() {
                    if matches!(name.as_str(), "+" | "-" | "*" | "/" | "=" | "<" | "<=" | ">" | ">=") {
                        // For arithmetic operators, we must use multi-argument application
                        eprintln!("DEBUG: Unifying arithmetic operator {} with func_ty={:?}, expected_func_ty={:?}",
                                 name, current_subst.apply(&func_ty), expected_func_ty);
                        match self.unify(&current_subst.apply(&func_ty), &expected_func_ty) {
                            Ok(unified_subst) => {
                                let final_subst = current_subst.compose(&unified_subst);
                                let final_result_ty = final_subst.apply(&result_ty);
                                return Ok((Expr::Application(Box::new(typed_func), typed_args, final_result_ty), final_subst));
                            }
                            Err(e) => {
                                eprintln!("DEBUG: Unification failed for {}: {:?}", name, e);
                                return Err(TlispError::Type(TypeError::UnificationFailure {
                                    left: format!("{:?}", current_subst.apply(&func_ty)),
                                    right: format!("{:?}", expected_func_ty),
                                }));
                            }
                        }
                    }
                }

                let final_result_ty = match self.unify(&current_subst.apply(&func_ty), &expected_func_ty) {
                    Ok(unified_subst) => {
                        let final_subst = current_subst.compose(&unified_subst);
                        final_subst.apply(&result_ty)
                    }
                    Err(_) => {
                        // Fall back to curried application for non-arithmetic functions
                        let mut curr_func_ty = current_subst.apply(&func_ty);
                        for typed_arg in &typed_args {
                            let result_ty = self.fresh_var();
                            let arg_ty = typed_arg.get_type().clone();
                            let expected_func_ty = Type::Function(vec![arg_ty], Box::new(result_ty.clone()));

                            let unified_subst = self.unify(&curr_func_ty, &expected_func_ty)?;
                            current_subst = current_subst.compose(&unified_subst);
                            curr_func_ty = current_subst.apply(&result_ty);
                        }
                        curr_func_ty
                    }
                };

                Ok((Expr::Application(Box::new(typed_func), typed_args, final_result_ty), current_subst))
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
            (a, b) if a == b => Ok(Substitution::new()),
            (a, b) => Err(TypeError::UnificationFailure { 
                left: a.to_string(), 
                right: b.to_string() 
            }.into()),
        }
    }
    
    /// Instantiate a type scheme (for now, just clone)
    fn instantiate(&mut self, ty: &Type) -> Type {
        // In a full implementation, this would handle type schemes with quantified variables
        ty.clone()
    }
    
    /// Add built-in function types
    fn add_builtins(&mut self) {
        // Arithmetic functions (multi-argument: (Int, Int) -> Int)
        let int_binop = Type::Function(vec![Type::Int, Type::Int], Box::new(Type::Int));
        self.env.insert("+".to_string(), int_binop.clone());
        self.env.insert("-".to_string(), int_binop.clone());
        self.env.insert("*".to_string(), int_binop.clone());
        self.env.insert("/".to_string(), int_binop);
        
        // Comparison functions (multi-argument: (Int, Int) -> Bool)
        let int_cmp = Type::Function(vec![Type::Int, Type::Int], Box::new(Type::Bool));
        self.env.insert("=".to_string(), int_cmp.clone());
        self.env.insert("<".to_string(), int_cmp.clone());
        self.env.insert("<=".to_string(), int_cmp.clone());
        self.env.insert(">".to_string(), int_cmp.clone());
        self.env.insert(">=".to_string(), int_cmp);
        
        // List functions
        let list_var = Type::TypeVar("a".to_string());
        let list_ty = Type::List(Box::new(list_var.clone()));
        
        // Variadic list constructor - for now, handle in the type checker specially
        // We'll handle this as a special case in the type inference
        self.env.insert("car".to_string(), Type::Function(vec![list_ty.clone()], Box::new(list_var.clone())));
        self.env.insert("cdr".to_string(), Type::Function(vec![list_ty.clone()], Box::new(list_ty.clone())));
        self.env.insert("cons".to_string(), Type::Function(vec![list_var.clone(), list_ty.clone()], Box::new(list_ty.clone())));
        self.env.insert("length".to_string(), Type::Function(vec![list_ty], Box::new(Type::Int)));
        
        // I/O functions
        self.env.insert("print".to_string(), Type::Function(vec![list_var], Box::new(Type::Unit)));
        
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
    fn test_type_operations() {
        let int_ty = Type::Int;
        let var_ty = Type::TypeVar("a".to_string());
        
        assert!(!int_ty.is_var());
        assert!(var_ty.is_var());
        assert_eq!(var_ty.as_var(), Some("a"));
    }
    
    #[test]
    fn test_substitution() {
        let subst = Substitution::single("a".to_string(), Type::Int);
        let var_ty = Type::TypeVar("a".to_string());
        
        assert_eq!(subst.apply(&var_ty), Type::Int);
        assert_eq!(subst.apply(&Type::Bool), Type::Bool);
    }
    
    #[test]
    fn test_unification() {
        let mut checker = TypeChecker::new();
        
        let var_a = Type::TypeVar("a".to_string());
        let int_ty = Type::Int;
        
        let subst = checker.unify(&var_a, &int_ty).unwrap();
        assert_eq!(subst.apply(&var_a), int_ty);
    }
    
    #[test]
    fn test_occurs_check() {
        let mut checker = TypeChecker::new();
        
        let var_a = Type::TypeVar("a".to_string());
        let list_a = Type::List(Box::new(var_a.clone()));
        
        // This should fail the occurs check
        let result = checker.unify(&var_a, &list_a);
        assert!(result.is_err());
    }
}
