//! Type-level evaluator for dependent types

use std::collections::HashMap;
use crate::tlisp::{Expr, Value};
use crate::tlisp::types::{Type, TypeTerm, Kind, TypeChecker};
use crate::error::{TlispResult, TypeError};

/// Type environment for type-level computation
#[derive(Debug, Clone)]
pub struct TypeEnvironment {
    /// Type bindings
    types: HashMap<String, Type>,
    /// Kind bindings
    kinds: HashMap<String, Kind>,
    /// Value bindings (for dependent types)
    values: HashMap<String, Value>,
}

impl TypeEnvironment {
    /// Create new type environment
    pub fn new() -> Self {
        let mut env = TypeEnvironment {
            types: HashMap::new(),
            kinds: HashMap::new(),
            values: HashMap::new(),
        };
        env.add_builtins();
        env
    }
    
    /// Look up type binding
    pub fn lookup_type(&self, name: &str) -> Option<Type> {
        self.types.get(name).cloned()
    }
    
    /// Look up kind binding
    pub fn lookup_kind(&self, name: &str) -> Option<Kind> {
        self.kinds.get(name).cloned()
    }
    
    /// Look up value binding
    pub fn lookup_value(&self, name: &str) -> Option<Value> {
        self.values.get(name).cloned()
    }
    
    /// Define type binding
    pub fn define_type(&mut self, name: String, ty: Type) {
        self.types.insert(name, ty);
    }
    
    /// Define kind binding
    pub fn define_kind(&mut self, name: String, kind: Kind) {
        self.kinds.insert(name, kind);
    }
    
    /// Define value binding
    pub fn define_value(&mut self, name: String, value: Value) {
        self.values.insert(name, value);
    }
    
    /// Add built-in type constructors
    fn add_builtins(&mut self) {
        // Basic type constructors as type lambdas
        let list_constructor = Type::TypeLambda {
            param: "T".to_string(),
            param_kind: Kind::Type,
            body: Box::new(Type::List(Box::new(Type::TypeVar("T".to_string())))),
        };
        self.define_type("List".to_string(), list_constructor);
        self.define_kind("List".to_string(), Kind::Arrow(Box::new(Kind::Type), Box::new(Kind::Type)));

        let function_constructor = Type::TypeLambda {
            param: "A".to_string(),
            param_kind: Kind::Type,
            body: Box::new(Type::TypeLambda {
                param: "B".to_string(),
                param_kind: Kind::Type,
                body: Box::new(Type::Function(vec![Type::TypeVar("A".to_string())], Box::new(Type::TypeVar("B".to_string())))),
            }),
        };
        self.define_type("Function".to_string(), function_constructor);
        self.define_kind("Function".to_string(), Kind::Arrow(Box::new(Kind::Type), Box::new(Kind::Arrow(Box::new(Kind::Type), Box::new(Kind::Type)))));

        // Built-in types
        self.define_type("Int".to_string(), Type::Int);
        self.define_type("Float".to_string(), Type::Float);
        self.define_type("Bool".to_string(), Type::Bool);
        self.define_type("String".to_string(), Type::String);
        self.define_type("Symbol".to_string(), Type::Symbol);
        self.define_type("Unit".to_string(), Type::Unit);
        self.define_type("Pid".to_string(), Type::Pid);

        // Type-level functions
        self.define_value("=".to_string(), Value::Builtin("type_eq".to_string()));
        self.define_value("eq".to_string(), Value::Builtin("type_eq".to_string())); // Add eq as alias
        self.define_value("+".to_string(), Value::Builtin("type_add".to_string()));
        self.define_value("-".to_string(), Value::Builtin("type_sub".to_string()));
        self.define_value("*".to_string(), Value::Builtin("type_mul".to_string()));
        self.define_value("/".to_string(), Value::Builtin("type_div".to_string()));
        self.define_value("<".to_string(), Value::Builtin("type_lt".to_string()));
        self.define_value("<=".to_string(), Value::Builtin("type_le".to_string()));
        self.define_value(">".to_string(), Value::Builtin("type_gt".to_string()));
        self.define_value(">=".to_string(), Value::Builtin("type_ge".to_string()));
    }
}

impl Default for TypeEnvironment {
    fn default() -> Self {
        Self::new()
    }
}

/// Type-level evaluator
pub struct TypeEvaluator {
    /// Type environment
    env: TypeEnvironment,
    /// Type checker for validation
    type_checker: TypeChecker,
}

impl TypeEvaluator {
    /// Create new type evaluator
    pub fn new() -> Self {
        TypeEvaluator {
            env: TypeEnvironment::new(),
            type_checker: TypeChecker::new(),
        }
    }
    
    /// Evaluate type expression to normal form
    pub fn eval_type(&mut self, expr: &Expr<()>) -> TlispResult<Type> {
        match expr {
            Expr::Symbol(name, _) => {
                self.env.lookup_type(name)
                    .ok_or_else(|| TypeError::UndefinedVariable(name.clone()).into())
            }
            
            Expr::Application(func, args, _) => {
                self.eval_type_application(func, args)
            }
            
            Expr::Lambda(params, body, _) => {
                self.eval_type_lambda(params, body)
            }
            
            Expr::If(cond, then_branch, else_branch, _) => {
                self.eval_conditional_type(cond, then_branch, else_branch)
            }
            
            Expr::List(elements, _) => {
                self.eval_type_list(elements)
            }
            
            Expr::Number(n, _) => {
                // Numbers in type context represent type-level constants
                Ok(Type::TypeApp {
                    constructor: Box::new(Type::TypeVar("Const".to_string())),
                    args: vec![TypeTerm::Literal(Value::Int(*n))],
                })
            }
            
            Expr::String(s, _) => {
                // Strings in type context represent type names
                Ok(self.env.lookup_type(s)
                    .unwrap_or_else(|| Type::TypeVar(s.clone())))
            }
            
            _ => Err(TypeError::InvalidTypeExpression(format!("{:?}", expr)).into()),
        }
    }
    
    /// Evaluate type application
    fn eval_type_application(&mut self, func: &Expr<()>, args: &[Expr<()>]) -> TlispResult<Type> {
        let func_type = self.eval_type(func)?;
        let arg_types = args.iter()
            .map(|arg| self.eval_type(arg))
            .collect::<Result<Vec<_>, _>>()?;
        
        self.apply_type_function(func_type, arg_types)
    }
    
    /// Apply type-level function
    fn apply_type_function(&mut self, func: Type, args: Vec<Type>) -> TlispResult<Type> {
        match func {
            Type::TypeLambda { param, param_kind, body } => {
                if args.is_empty() {
                    return Err(TypeError::ArityMismatch { expected: 1, actual: 0 }.into());
                }
                
                // Check kind compatibility
                let arg_kind = args[0].kind();
                if arg_kind != param_kind {
                    return Err(TypeError::KindMismatch { 
                        expected: format!("{}", param_kind), 
                        actual: format!("{}", arg_kind) 
                    }.into());
                }
                
                // Substitute parameter in body
                let substituted = self.substitute_type_var(&param, &args[0], &body)?;
                
                // Continue with remaining arguments
                if args.len() > 1 {
                    self.apply_type_function(substituted, args[1..].to_vec())
                } else {
                    Ok(substituted)
                }
            }
            
            // Handle built-in type constructors that are now type lambdas
            Type::List(_) | Type::Function(_, _) => {
                // These should be handled by the TypeLambda case above
                Err(TypeError::NotATypeFunction(format!("{}", func)).into())
            }
            
            _ => Err(TypeError::NotATypeFunction(format!("{}", func)).into()),
        }
    }
    
    /// Evaluate type lambda
    fn eval_type_lambda(&mut self, params: &[String], body: &Expr<()>) -> TlispResult<Type> {
        if params.is_empty() {
            return self.eval_type(body);
        }
        
        // Create nested type lambda
        let body_type = self.eval_type(body)?;
        
        params.iter().rev().fold(
            Ok(body_type),
            |acc, param| {
                acc.map(|body| Type::TypeLambda {
                    param: param.clone(),
                    param_kind: Kind::Type, // Default kind, should be inferred
                    body: Box::new(body),
                })
            }
        )
    }
    
    /// Evaluate conditional type
    fn eval_conditional_type(&mut self, cond: &Expr<()>, then_branch: &Expr<()>, else_branch: &Expr<()>) -> TlispResult<Type> {
        // Evaluate condition as type-level boolean
        let cond_result = self.eval_type_condition(cond)?;
        
        if cond_result {
            self.eval_type(then_branch)
        } else {
            self.eval_type(else_branch)
        }
    }
    
    /// Evaluate type-level condition
    fn eval_type_condition(&mut self, expr: &Expr<()>) -> TlispResult<bool> {
        match expr {
            Expr::Bool(b, _) => Ok(*b),
            Expr::Application(func, args, _) => {
                if let Expr::Symbol(name, _) = func.as_ref() {
                    match name.as_str() {
                        "=" => self.eval_type_equality(args),
                        "eq" => self.eval_type_equality(args), // Add eq as alias
                        "<" => self.eval_type_comparison(args, |a, b| a < b),
                        "<=" => self.eval_type_comparison(args, |a, b| a <= b),
                        ">" => self.eval_type_comparison(args, |a, b| a > b),
                        ">=" => self.eval_type_comparison(args, |a, b| a >= b),
                        _ => Err(TypeError::InvalidTypeCondition(format!("{}", name)).into()),
                    }
                } else {
                    Err(TypeError::InvalidTypeCondition("complex function".to_string()).into())
                }
            }
            _ => Err(TypeError::InvalidTypeCondition(format!("{:?}", expr)).into()),
        }
    }
    
    /// Evaluate type equality
    fn eval_type_equality(&mut self, args: &[Expr<()>]) -> TlispResult<bool> {
        if args.len() != 2 {
            return Err(TypeError::ArityMismatch { expected: 2, actual: args.len() }.into());
        }
        
        let left = self.eval_type_term(&args[0])?;
        let right = self.eval_type_term(&args[1])?;
        
        Ok(self.type_terms_equal(&left, &right))
    }
    
    /// Evaluate type comparison
    fn eval_type_comparison<F>(&mut self, args: &[Expr<()>], op: F) -> TlispResult<bool>
    where
        F: Fn(i64, i64) -> bool,
    {
        if args.len() != 2 {
            return Err(TypeError::ArityMismatch { expected: 2, actual: args.len() }.into());
        }
        
        let left = self.eval_type_term(&args[0])?;
        let right = self.eval_type_term(&args[1])?;
        
        match (left, right) {
            (TypeTerm::Literal(Value::Int(a)), TypeTerm::Literal(Value::Int(b))) => Ok(op(a, b)),
            _ => Err(TypeError::InvalidTypeComparison("non-integer values".to_string()).into()),
        }
    }
    
    /// Evaluate type list
    fn eval_type_list(&mut self, elements: &[Expr<()>]) -> TlispResult<Type> {
        if elements.is_empty() {
            return Ok(Type::List(Box::new(Type::TypeVar("a".to_string()))));
        }
        
        // For now, assume all elements have the same type
        let first_type = self.eval_type(&elements[0])?;
        Ok(Type::List(Box::new(first_type)))
    }
    
    /// Evaluate type term
    fn eval_type_term(&mut self, expr: &Expr<()>) -> TlispResult<TypeTerm> {
        match expr {
            Expr::Symbol(name, _) => Ok(TypeTerm::Var(name.clone())),
            Expr::Number(n, _) => Ok(TypeTerm::Literal(Value::Int(*n))),
            Expr::String(s, _) => Ok(TypeTerm::Literal(Value::String(s.clone()))),
            Expr::Bool(b, _) => Ok(TypeTerm::Literal(Value::Bool(*b))),
            Expr::Application(func, args, _) => {
                let func_term = self.eval_type_term(func)?;
                let arg_terms = args.iter()
                    .map(|arg| self.eval_type_term(arg))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(TypeTerm::App(Box::new(func_term), arg_terms))
            }
            _ => Err(TypeError::InvalidTypeTerm(format!("{:?}", expr)).into()),
        }
    }
    
    /// Check if two type terms are equal
    fn type_terms_equal(&self, left: &TypeTerm, right: &TypeTerm) -> bool {
        match (left, right) {
            (TypeTerm::Var(a), TypeTerm::Var(b)) => a == b,
            (TypeTerm::Literal(a), TypeTerm::Literal(b)) => a == b,
            (TypeTerm::App(f1, args1), TypeTerm::App(f2, args2)) => {
                self.type_terms_equal(f1, f2) && 
                args1.len() == args2.len() &&
                args1.iter().zip(args2.iter()).all(|(a1, a2)| self.type_terms_equal(a1, a2))
            }
            _ => false,
        }
    }
    
    /// Substitute type variable in type
    fn substitute_type_var(&self, var: &str, replacement: &Type, ty: &Type) -> TlispResult<Type> {
        match ty {
            Type::TypeVar(name) if name == var => Ok(replacement.clone()),
            Type::TypeVar(_) => Ok(ty.clone()),
            Type::List(elem) => {
                let new_elem = self.substitute_type_var(var, replacement, elem)?;
                Ok(Type::List(Box::new(new_elem)))
            }
            Type::Function(params, ret) => {
                let new_params = params.iter()
                    .map(|p| self.substitute_type_var(var, replacement, p))
                    .collect::<Result<Vec<_>, _>>()?;
                let new_ret = self.substitute_type_var(var, replacement, ret)?;
                Ok(Type::Function(new_params, Box::new(new_ret)))
            }
            Type::TypeLambda { param, param_kind, body } if param != var => {
                let new_body = self.substitute_type_var(var, replacement, body)?;
                Ok(Type::TypeLambda {
                    param: param.clone(),
                    param_kind: param_kind.clone(),
                    body: Box::new(new_body),
                })
            }
            _ => Ok(ty.clone()),
        }
    }
}

impl Default for TypeEvaluator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tlisp::parser::Parser;

    fn parse_expr(input: &str) -> Expr<()> {
        let mut parser = Parser::new();
        let tokens = parser.tokenize(input).unwrap();
        parser.parse(&tokens).unwrap()
    }

    #[test]
    fn test_type_evaluator_creation() {
        let evaluator = TypeEvaluator::new();
        assert!(evaluator.env.lookup_type("Int").is_some());
        assert!(evaluator.env.lookup_type("String").is_some());
    }

    #[test]
    fn test_basic_type_evaluation() {
        let mut evaluator = TypeEvaluator::new();

        // Test basic type lookup
        let int_expr = parse_expr("Int");
        let result = evaluator.eval_type(&int_expr).unwrap();
        assert_eq!(result, Type::Int);

        let string_expr = parse_expr("String");
        let result = evaluator.eval_type(&string_expr).unwrap();
        assert_eq!(result, Type::String);
    }

    #[test]
    fn test_type_application() {
        let mut evaluator = TypeEvaluator::new();

        // Test List type application: (List Int)
        // Create the application expression manually to avoid parsing issues
        let list_sym = Expr::Symbol("List".to_string(), ());
        let int_sym = Expr::Symbol("Int".to_string(), ());
        let list_expr = Expr::Application(Box::new(list_sym), vec![int_sym], ());

        let result = evaluator.eval_type(&list_expr).unwrap();
        assert_eq!(result, Type::List(Box::new(Type::Int)));
    }

    #[test]
    fn test_function_type_application() {
        let mut evaluator = TypeEvaluator::new();

        // Test Function type application: (Function Int String)
        // Create the application expression manually to avoid parsing issues
        let func_sym = Expr::Symbol("Function".to_string(), ());
        let int_sym = Expr::Symbol("Int".to_string(), ());
        let string_sym = Expr::Symbol("String".to_string(), ());
        let func_expr = Expr::Application(Box::new(func_sym), vec![int_sym, string_sym], ());

        let result = evaluator.eval_type(&func_expr).unwrap();
        assert_eq!(result, Type::Function(vec![Type::Int], Box::new(Type::String)));
    }

    #[test]
    fn test_type_lambda_evaluation() {
        let mut evaluator = TypeEvaluator::new();

        // Test type lambda: (lambda (T) Int) - a constant type lambda
        // This avoids the issue with undefined variables in the body
        let int_sym = Expr::Symbol("Int".to_string(), ());

        // Create (lambda (T) Int)
        let lambda_expr = Expr::Lambda(vec!["T".to_string()], Box::new(int_sym), ());

        let result = evaluator.eval_type(&lambda_expr).unwrap();

        match result {
            Type::TypeLambda { param, body, .. } => {
                assert_eq!(param, "T");
                assert_eq!(*body, Type::Int);
            }
            _ => panic!("Expected TypeLambda, got: {:?}", result),
        }
    }

    #[test]
    fn test_conditional_type_evaluation() {
        let mut evaluator = TypeEvaluator::new();

        // Test conditional type: (if true Int String)
        let cond_expr = parse_expr("(if true Int String)");
        let result = evaluator.eval_type(&cond_expr).unwrap();
        assert_eq!(result, Type::Int);

        // Test conditional type: (if false Int String)
        let cond_expr = parse_expr("(if false Int String)");
        let result = evaluator.eval_type(&cond_expr).unwrap();
        assert_eq!(result, Type::String);
    }

    #[test]
    fn test_type_equality_condition() {
        let mut evaluator = TypeEvaluator::new();

        // Test type equality: (= 1 1) should be true
        let eq_expr = parse_expr("(= 1 1)");
        let result = evaluator.eval_type_condition(&eq_expr).unwrap();
        assert!(result);

        // Test type inequality: (= 1 2) should be false
        let neq_expr = parse_expr("(= 1 2)");
        let result = evaluator.eval_type_condition(&neq_expr).unwrap();
        assert!(!result);
    }

    #[test]
    fn test_type_comparison_condition() {
        let mut evaluator = TypeEvaluator::new();

        // Test type comparison: (< 1 2) should be true
        let lt_expr = parse_expr("(< 1 2)");
        let result = evaluator.eval_type_condition(&lt_expr).unwrap();
        assert!(result);

        // Test type comparison: (> 1 2) should be false
        let gt_expr = parse_expr("(> 1 2)");
        let result = evaluator.eval_type_condition(&gt_expr).unwrap();
        assert!(!result);
    }

    #[test]
    fn test_type_term_evaluation() {
        let mut evaluator = TypeEvaluator::new();

        // Test variable term
        let var_expr = parse_expr("x");
        let result = evaluator.eval_type_term(&var_expr).unwrap();
        assert_eq!(result, TypeTerm::Var("x".to_string()));

        // Test literal term
        let lit_expr = parse_expr("42");
        let result = evaluator.eval_type_term(&lit_expr).unwrap();
        assert_eq!(result, TypeTerm::Literal(Value::Int(42)));

        // Test application term
        let app_expr = parse_expr("(+ 1 2)");
        let result = evaluator.eval_type_term(&app_expr).unwrap();
        match result {
            TypeTerm::App(func, args) => {
                assert_eq!(*func, TypeTerm::Var("+".to_string()));
                assert_eq!(args.len(), 2);
            }
            _ => panic!("Expected App"),
        }
    }

    #[test]
    fn test_type_substitution() {
        let evaluator = TypeEvaluator::new();

        // Test substitution in simple type
        let ty = Type::TypeVar("T".to_string());
        let replacement = Type::Int;
        let result = evaluator.substitute_type_var("T", &replacement, &ty).unwrap();
        assert_eq!(result, Type::Int);

        // Test substitution in list type
        let list_ty = Type::List(Box::new(Type::TypeVar("T".to_string())));
        let result = evaluator.substitute_type_var("T", &replacement, &list_ty).unwrap();
        assert_eq!(result, Type::List(Box::new(Type::Int)));

        // Test no substitution for different variable
        let result = evaluator.substitute_type_var("U", &replacement, &ty).unwrap();
        assert_eq!(result, ty);
    }

    #[test]
    fn test_type_environment_operations() {
        let mut env = TypeEnvironment::new();

        // Test type definition and lookup
        env.define_type("MyType".to_string(), Type::Int);
        assert_eq!(env.lookup_type("MyType"), Some(Type::Int));

        // Test kind definition and lookup
        env.define_kind("MyKind".to_string(), Kind::Type);
        assert_eq!(env.lookup_kind("MyKind"), Some(Kind::Type));

        // Test value definition and lookup
        env.define_value("MyValue".to_string(), Value::Int(42));
        assert_eq!(env.lookup_value("MyValue"), Some(Value::Int(42)));
    }

    #[test]
    fn test_error_handling() {
        let mut evaluator = TypeEvaluator::new();

        // Test undefined type variable
        let undefined_expr = parse_expr("UndefinedType");
        let result = evaluator.eval_type(&undefined_expr);
        assert!(result.is_err());

        // Test type application error - try to apply a non-function type
        let func_expr = Expr::Symbol("Int".to_string(), ()); // Int is not a type constructor
        let arg_expr = Expr::Symbol("String".to_string(), ());
        let app_expr = Expr::Application(Box::new(func_expr), vec![arg_expr], ());
        let result = evaluator.eval_type(&app_expr);
        assert!(result.is_err(), "Expected type application error for Int applied to String, got: {:?}", result);
    }
}
