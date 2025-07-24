//! Fixed TLISP type system - simplified and working correctly

use std::collections::HashMap;
use crate::tlisp::Expr;
use crate::error::{TlispError, TlispResult, TypeError};

/// Type representation
#[derive(Debug, Clone, PartialEq)]
pub enum Type {
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
            _ => false,
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
            _ => ty.clone(),
        }
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
                let expected_func_ty = Type::Function(arg_types, Box::new(result_ty.clone()));

                // Unify function types
                let func_ty = typed_func.get_type().clone();
                let unified_subst = self.unify(&current_subst.apply(&func_ty), &expected_func_ty)?;
                let final_subst = current_subst.compose(&unified_subst);
                let final_result_ty = final_subst.apply(&result_ty);

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
    
    /// Instantiate a type scheme (simplified - just clone for now)
    fn instantiate(&mut self, ty: &Type) -> Type {
        ty.clone()
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
        
        self.env.insert("car".to_string(), Type::Function(vec![list_ty.clone()], Box::new(list_var.clone())));
        self.env.insert("cdr".to_string(), Type::Function(vec![list_ty.clone()], Box::new(list_ty.clone())));
        self.env.insert("cons".to_string(), Type::Function(vec![list_var.clone(), list_ty.clone()], Box::new(list_ty.clone())));
        self.env.insert("length".to_string(), Type::Function(vec![list_ty], Box::new(Type::Int)));
        
        // I/O functions
        self.env.insert("print".to_string(), Type::Function(vec![Type::TypeVar("a".to_string())], Box::new(Type::Unit)));
        
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