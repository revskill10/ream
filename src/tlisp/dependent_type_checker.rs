//! Dependent Type Checker for TLISP
//! 
//! Implements Phase 3 of the dependent types plan: type inference and checking
//! with constraint generation and verification capabilities.

use std::collections::HashMap;
use crate::tlisp::{Expr, Value};
use crate::tlisp::types::{Type, TypeTerm, Kind, Substitution};
use crate::tlisp::constraint_solver::{ConstraintSolver, Constraint};
use crate::tlisp::type_evaluator::TypeEvaluator;
use crate::error::{TlispResult, TypeError};

/// Type environment for dependent type checking
#[derive(Debug, Clone)]
pub struct TypeEnvironment {
    /// Variable type bindings
    vars: HashMap<String, Type>,
    
    /// Type constructor bindings
    type_constructors: HashMap<String, Type>,
    
    /// Value bindings for dependent types
    values: HashMap<String, Value>,
    
    /// Parent environment (for scoping)
    parent: Option<Box<TypeEnvironment>>,
}

impl TypeEnvironment {
    /// Create new empty environment
    pub fn new() -> Self {
        TypeEnvironment {
            vars: HashMap::new(),
            type_constructors: HashMap::new(),
            values: HashMap::new(),
            parent: None,
        }
    }
    
    /// Create child environment
    pub fn child(&self) -> Self {
        TypeEnvironment {
            vars: HashMap::new(),
            type_constructors: HashMap::new(),
            values: HashMap::new(),
            parent: Some(Box::new(self.clone())),
        }
    }
    
    /// Bind variable to type
    pub fn bind_var(&mut self, name: String, ty: Type) {
        self.vars.insert(name, ty);
    }
    
    /// Bind type constructor
    pub fn bind_type_constructor(&mut self, name: String, ty: Type) {
        self.type_constructors.insert(name, ty);
    }
    
    /// Bind value for dependent types
    pub fn bind_value(&mut self, name: String, value: Value) {
        self.values.insert(name, value);
    }
    
    /// Look up variable type
    pub fn lookup_var(&self, name: &str) -> Option<Type> {
        self.vars.get(name).cloned()
            .or_else(|| self.parent.as_ref().and_then(|p| p.lookup_var(name)))
    }
    
    /// Look up type constructor
    pub fn lookup_type_constructor(&self, name: &str) -> Option<Type> {
        self.type_constructors.get(name).cloned()
            .or_else(|| self.parent.as_ref().and_then(|p| p.lookup_type_constructor(name)))
    }
    
    /// Look up value
    pub fn lookup_value(&self, name: &str) -> Option<Value> {
        self.values.get(name).cloned()
            .or_else(|| self.parent.as_ref().and_then(|p| p.lookup_value(name)))
    }
}

impl Default for TypeEnvironment {
    fn default() -> Self {
        Self::new()
    }
}

/// Typing context for dependent type checking
#[derive(Debug, Clone)]
pub struct TypingContext {
    /// Current constraints being generated
    constraints: Vec<Constraint>,
    
    /// Fresh variable counter
    var_counter: usize,
    
    /// Current substitution
    substitution: Substitution,
}

impl TypingContext {
    /// Create new typing context
    pub fn new() -> Self {
        TypingContext {
            constraints: Vec::new(),
            var_counter: 0,
            substitution: Substitution::new(),
        }
    }
    
    /// Generate fresh type variable
    pub fn fresh_var(&mut self) -> Type {
        let name = format!("t{}", self.var_counter);
        self.var_counter += 1;
        Type::TypeVar(name)
    }
    
    /// Add constraint
    pub fn add_constraint(&mut self, constraint: Constraint) {
        self.constraints.push(constraint);
    }
    
    /// Get all constraints
    pub fn constraints(&self) -> &[Constraint] {
        &self.constraints
    }
    
    /// Clear constraints
    pub fn clear_constraints(&mut self) {
        self.constraints.clear();
    }
    
    /// Apply substitution to type
    pub fn apply_substitution(&self, ty: &Type) -> Type {
        self.substitution.apply(ty)
    }
}

impl Default for TypingContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Dependent Type Checker
pub struct DependentTypeChecker {
    /// Type environment
    env: TypeEnvironment,
    
    /// Constraint solver
    solver: ConstraintSolver,
    
    /// Type evaluator
    evaluator: TypeEvaluator,
    
    /// Current typing context
    context: TypingContext,
}

impl DependentTypeChecker {
    /// Create new dependent type checker
    pub fn new() -> Self {
        let mut checker = DependentTypeChecker {
            env: TypeEnvironment::new(),
            solver: ConstraintSolver::new(),
            evaluator: TypeEvaluator::new(),
            context: TypingContext::new(),
        };
        
        checker.add_builtins();
        checker
    }
    
    /// Add built-in types and functions
    fn add_builtins(&mut self) {
        // Basic types
        self.env.bind_type_constructor("Int".to_string(), Type::Int);
        self.env.bind_type_constructor("Float".to_string(), Type::Float);
        self.env.bind_type_constructor("Bool".to_string(), Type::Bool);
        self.env.bind_type_constructor("String".to_string(), Type::String);
        self.env.bind_type_constructor("Unit".to_string(), Type::Unit);
        
        // List type constructor
        self.env.bind_type_constructor("List".to_string(), Type::TypeLambda {
            param: "T".to_string(),
            param_kind: Kind::Type,
            body: Box::new(Type::List(Box::new(Type::TypeVar("T".to_string())))),
        });
        
        // Function type constructor
        self.env.bind_type_constructor("Function".to_string(), Type::TypeLambda {
            param: "A".to_string(),
            param_kind: Kind::Type,
            body: Box::new(Type::TypeLambda {
                param: "B".to_string(),
                param_kind: Kind::Type,
                body: Box::new(Type::Function(
                    vec![Type::TypeVar("A".to_string())],
                    Box::new(Type::TypeVar("B".to_string()))
                )),
            }),
        });
        
        // Built-in functions
        self.env.bind_var("+".to_string(), Type::Function(
            vec![Type::Int, Type::Int],
            Box::new(Type::Int)
        ));
        
        self.env.bind_var("-".to_string(), Type::Function(
            vec![Type::Int, Type::Int],
            Box::new(Type::Int)
        ));
        
        self.env.bind_var("*".to_string(), Type::Function(
            vec![Type::Int, Type::Int],
            Box::new(Type::Int)
        ));
        
        // For now, use a simple function type for equality - we'll improve polymorphism later
        self.env.bind_var("=".to_string(), Type::Function(
            vec![Type::TypeVar("T".to_string()), Type::TypeVar("T".to_string())],
            Box::new(Type::Bool)
        ));

        // Add eq as alias for =
        self.env.bind_var("eq".to_string(), Type::Function(
            vec![Type::TypeVar("T".to_string()), Type::TypeVar("T".to_string())],
            Box::new(Type::Bool)
        ));
        
        self.env.bind_var("<".to_string(), Type::Function(
            vec![Type::Int, Type::Int],
            Box::new(Type::Bool)
        ));

        self.env.bind_var("<=".to_string(), Type::Function(
            vec![Type::Int, Type::Int],
            Box::new(Type::Bool)
        ));

        self.env.bind_var(">".to_string(), Type::Function(
            vec![Type::Int, Type::Int],
            Box::new(Type::Bool)
        ));

        self.env.bind_var(">=".to_string(), Type::Function(
            vec![Type::Int, Type::Int],
            Box::new(Type::Bool)
        ));

        // List functions
        self.env.bind_var("list".to_string(), Type::TypeLambda {
            param: "T".to_string(),
            param_kind: Kind::Type,
            body: Box::new(Type::Function(
                vec![], // Variadic function - empty args for now
                Box::new(Type::List(Box::new(Type::TypeVar("T".to_string()))))
            )),
        });

        self.env.bind_var("car".to_string(), Type::TypeLambda {
            param: "T".to_string(),
            param_kind: Kind::Type,
            body: Box::new(Type::Function(
                vec![Type::List(Box::new(Type::TypeVar("T".to_string())))],
                Box::new(Type::TypeVar("T".to_string()))
            )),
        });

        self.env.bind_var("cdr".to_string(), Type::TypeLambda {
            param: "T".to_string(),
            param_kind: Kind::Type,
            body: Box::new(Type::Function(
                vec![Type::List(Box::new(Type::TypeVar("T".to_string())))],
                Box::new(Type::List(Box::new(Type::TypeVar("T".to_string()))))
            )),
        });

        self.env.bind_var("cons".to_string(), Type::TypeLambda {
            param: "T".to_string(),
            param_kind: Kind::Type,
            body: Box::new(Type::Function(
                vec![Type::TypeVar("T".to_string()), Type::List(Box::new(Type::TypeVar("T".to_string())))],
                Box::new(Type::List(Box::new(Type::TypeVar("T".to_string()))))
            )),
        });

        self.env.bind_var("length".to_string(), Type::TypeLambda {
            param: "T".to_string(),
            param_kind: Kind::Type,
            body: Box::new(Type::Function(
                vec![Type::List(Box::new(Type::TypeVar("T".to_string())))],
                Box::new(Type::Int)
            )),
        });

        // Predicates
        self.env.bind_var("null?".to_string(), Type::TypeLambda {
            param: "T".to_string(),
            param_kind: Kind::Type,
            body: Box::new(Type::Function(
                vec![Type::TypeVar("T".to_string())],
                Box::new(Type::Bool)
            )),
        });

        // Math functions
        self.env.bind_var("mod".to_string(), Type::Function(
            vec![Type::Int, Type::Int],
            Box::new(Type::Int)
        ));

        self.env.bind_var("modulo".to_string(), Type::Function(
            vec![Type::Int, Type::Int],
            Box::new(Type::Int)
        ));

        // String functions - string-append is variadic, so we'll handle it specially
        // For now, define it as a simple function that the type checker will handle specially
        self.env.bind_var("string-append".to_string(), Type::Function(
            vec![], // Empty args - will be handled specially in type inference
            Box::new(Type::String)
        ));
    }
    
    /// Define a variable in the environment
    pub fn define_var(&mut self, name: String, ty: Type) {
        self.env.bind_var(name, ty);
    }
    
    /// Define a value for dependent types
    pub fn define_value(&mut self, name: String, value: Value) {
        self.env.bind_value(name, value);
    }
    
    /// Infer type of expression
    pub fn infer_type(&mut self, expr: &Expr<()>) -> TlispResult<Type> {
        self.context.clear_constraints();
        let ty = self.infer_type_internal(expr)?;
        
        // Solve constraints
        for constraint in self.context.constraints() {
            self.solver.solve(constraint)?;
        }
        
        Ok(self.context.apply_substitution(&ty))
    }
    
    /// Internal type inference implementation
    fn infer_type_internal(&mut self, expr: &Expr<()>) -> TlispResult<Type> {
        match expr {
            Expr::Number(_, _) => Ok(Type::Int),
            Expr::Float(_, _) => Ok(Type::Float),
            Expr::Bool(_, _) => Ok(Type::Bool),
            Expr::String(_, _) => Ok(Type::String),

            Expr::Symbol(name, _) => {
                self.env.lookup_var(name)
                    .ok_or_else(|| TypeError::UndefinedVariable(name.clone()).into())
            }

            Expr::Application(func, args, _) => {
                self.infer_application_type(func, args)
            }

            Expr::Lambda(params, body, _) => {
                self.infer_lambda_type(params, body)
            }

            Expr::Let(bindings, body, _) => {
                self.infer_let_type(bindings, body)
            }

            Expr::If(cond, then_branch, else_branch, _) => {
                self.infer_if_type(cond, then_branch, else_branch)
            }

            Expr::List(elements, _) => {
                self.infer_list_type(elements)
            }

            Expr::Quote(quoted_expr, _) => {
                self.infer_quote_type(quoted_expr)
            }

            Expr::Define(name, value_expr, _) => {
                self.infer_define_type(name, value_expr)
            }

            Expr::Set(name, value_expr, _) => {
                self.infer_set_type(name, value_expr)
            }
        }
    }

    /// Infer type of function application
    fn infer_application_type(&mut self, func: &Expr<()>, args: &[Expr<()>]) -> TlispResult<Type> {
        // Special handling for functions with special arity requirements
        if let Expr::Symbol(name, _) = func {
            if name == "-" {
                return self.infer_minus_type(args);
            }
            if name == "string-append" {
                return self.infer_string_append_type(args);
            }
            if name == "list" {
                return self.infer_list_constructor_type(args);
            }
        }

        let func_type = self.infer_type_internal(func)?;
        let arg_types = args.iter()
            .map(|arg| self.infer_type_internal(arg))
            .collect::<Result<Vec<_>, _>>()?;

        match func_type {
            Type::Function(param_types, return_type) => {
                // Check parameter count
                if param_types.len() != arg_types.len() {
                    return Err(TypeError::ArityMismatch {
                        expected: param_types.len(),
                        actual: arg_types.len(),
                    }.into());
                }

                // Generate unification constraints
                for (param_type, arg_type) in param_types.iter().zip(arg_types.iter()) {
                    self.context.add_constraint(Constraint::TypeEquality(param_type.clone(), arg_type.clone()));
                }

                Ok(*return_type)
            }

            Type::DepFunction { param_name, param_type, return_type } => {
                // Check single parameter for dependent function
                if arg_types.len() != 1 {
                    return Err(TypeError::ArityMismatch {
                        expected: 1,
                        actual: arg_types.len(),
                    }.into());
                }

                let arg_type = &arg_types[0];

                // Unify parameter type
                self.context.add_constraint(Constraint::TypeEquality(*param_type.clone(), arg_type.clone()));

                // Substitute argument value in return type
                let substituted_return = self.substitute_in_type(&param_name, &args[0], &return_type)?;

                Ok(substituted_return)
            }

            Type::Refinement { var, base_type, predicate } => {
                // For refinement types, check that the argument satisfies the predicate
                if arg_types.len() != 1 {
                    return Err(TypeError::ArityMismatch {
                        expected: 1,
                        actual: arg_types.len(),
                    }.into());
                }

                let arg_type = &arg_types[0];

                // Check that argument has the base type
                self.context.add_constraint(Constraint::TypeEquality(*base_type.clone(), arg_type.clone()));

                // Check that the predicate holds for the argument
                self.context.add_constraint(Constraint::Refinement {
                    var: var.clone(),
                    var_type: *base_type.clone(),
                    predicate: *predicate.clone(),
                });

                // Return the base type (refinement types are transparent for computation)
                Ok(*base_type.clone())
            }

            Type::TypeLambda { param, param_kind: _, body } => {
                // Type application - apply type arguments
                if arg_types.len() != 1 {
                    return Err(TypeError::ArityMismatch {
                        expected: 1,
                        actual: arg_types.len(),
                    }.into());
                }

                // Substitute type parameter in body
                let substituted = self.substitute_type_var(&param, &arg_types[0], &body)?;
                Ok(substituted)
            }

            _ => {
                // Try to unify with a function type
                let return_type = self.context.fresh_var();
                let func_type = Type::Function(arg_types, Box::new(return_type.clone()));
                self.context.add_constraint(Constraint::TypeEquality(func_type.clone(), func_type));
                Ok(return_type)
            }
        }
    }

    /// Special type inference for minus function (handles both unary and binary cases)
    fn infer_minus_type(&mut self, args: &[Expr<()>]) -> TlispResult<Type> {
        let arg_types = args.iter()
            .map(|arg| self.infer_type_internal(arg))
            .collect::<Result<Vec<_>, _>>()?;

        match arg_types.len() {
            1 => {
                // Unary minus: Int -> Int or Float -> Float
                let arg_type = &arg_types[0];
                match arg_type {
                    Type::Int => Ok(Type::Int),
                    Type::Float => Ok(Type::Float),
                    Type::TypeVar(_) => {
                        // Add constraint that the argument must be numeric
                        self.context.add_constraint(Constraint::TypeEquality(arg_type.clone(), Type::Int));
                        Ok(Type::Int)
                    }
                    _ => Err(TypeError::TypeMismatch(
                        "Int".to_string(),
                        format!("{:?}", arg_type),
                    ).into()),
                }
            }
            2 => {
                // Binary minus: Int -> Int -> Int
                let arg1_type = &arg_types[0];
                let arg2_type = &arg_types[1];

                // Add constraints for numeric types
                self.context.add_constraint(Constraint::TypeEquality(arg1_type.clone(), Type::Int));
                self.context.add_constraint(Constraint::TypeEquality(arg2_type.clone(), Type::Int));

                Ok(Type::Int)
            }
            _ => Err(TypeError::ArityMismatch {
                expected: 2, // We'll say 2 for the error message, but we accept 1 or 2
                actual: arg_types.len(),
            }.into()),
        }
    }

    /// Special type inference for string-append function (variadic)
    fn infer_string_append_type(&mut self, args: &[Expr<()>]) -> TlispResult<Type> {
        if args.is_empty() {
            return Err(TypeError::ArityMismatch {
                expected: 1, // At least one argument
                actual: 0,
            }.into());
        }

        // All arguments must be strings
        for arg in args {
            let arg_type = self.infer_type_internal(arg)?;
            self.context.add_constraint(Constraint::TypeEquality(arg_type, Type::String));
        }

        Ok(Type::String)
    }

    /// Special type inference for list constructor function (variadic)
    fn infer_list_constructor_type(&mut self, args: &[Expr<()>]) -> TlispResult<Type> {
        if args.is_empty() {
            // Empty list
            return Ok(Type::List(Box::new(Type::TypeVar("T".to_string()))));
        }

        // Infer the type of the first element
        let first_type = self.infer_type_internal(&args[0])?;

        // All elements must have the same type
        for arg in &args[1..] {
            let arg_type = self.infer_type_internal(arg)?;
            self.context.add_constraint(Constraint::TypeEquality(arg_type, first_type.clone()));
        }

        Ok(Type::List(Box::new(first_type)))
    }

    /// Infer type of lambda expression
    fn infer_lambda_type(&mut self, params: &[String], body: &Expr<()>) -> TlispResult<Type> {
        // Create fresh type variables for parameters
        let param_types: Vec<Type> = params.iter()
            .map(|_| self.context.fresh_var())
            .collect();

        // Create child environment with parameter bindings
        let old_env = self.env.clone();
        for (param, param_type) in params.iter().zip(param_types.iter()) {
            self.env.bind_var(param.clone(), param_type.clone());
        }

        // Infer body type
        let body_type = self.infer_type_internal(body)?;

        // Restore environment
        self.env = old_env;

        Ok(Type::Function(param_types, Box::new(body_type)))
    }

    /// Infer type of let expression
    fn infer_let_type(&mut self, bindings: &[(String, Expr<()>)], body: &Expr<()>) -> TlispResult<Type> {
        let old_env = self.env.clone();

        // Process bindings sequentially
        for (name, value_expr) in bindings {
            let value_type = self.infer_type_internal(value_expr)?;
            self.env.bind_var(name.clone(), value_type);

            // For dependent types, also bind the value if it's a literal
            if let Some(value) = self.expr_to_value(value_expr) {
                self.env.bind_value(name.clone(), value);
            }
        }

        // Infer body type
        let body_type = self.infer_type_internal(body)?;

        // Restore environment
        self.env = old_env;

        Ok(body_type)
    }

    /// Infer type of if expression
    fn infer_if_type(&mut self, cond: &Expr<()>, then_branch: &Expr<()>, else_branch: &Expr<()>) -> TlispResult<Type> {
        let cond_type = self.infer_type_internal(cond)?;
        let then_type = self.infer_type_internal(then_branch)?;
        let else_type = self.infer_type_internal(else_branch)?;

        // Condition must be boolean
        self.context.add_constraint(Constraint::TypeEquality(cond_type, Type::Bool));

        // Both branches must have the same type
        self.context.add_constraint(Constraint::TypeEquality(then_type.clone(), else_type));

        Ok(then_type)
    }

    /// Infer type of list expression
    fn infer_list_type(&mut self, elements: &[Expr<()>]) -> TlispResult<Type> {
        if elements.is_empty() {
            // Empty list has polymorphic type
            let elem_type = self.context.fresh_var();
            return Ok(Type::List(Box::new(elem_type)));
        }

        // Infer type of first element
        let first_type = self.infer_type_internal(&elements[0])?;

        // All elements must have the same type
        for element in &elements[1..] {
            let elem_type = self.infer_type_internal(element)?;
            self.context.add_constraint(Constraint::TypeEquality(first_type.clone(), elem_type));
        }

        Ok(Type::List(Box::new(first_type)))
    }

    /// Infer type of quote expression
    fn infer_quote_type(&mut self, quoted_expr: &Expr<()>) -> TlispResult<Type> {
        // Quote expressions return the quoted expression as a literal value
        // The type depends on what's being quoted
        match quoted_expr {
            Expr::Symbol(_, _) => Ok(Type::String), // Quoted symbols become strings
            Expr::Number(_, _) => Ok(Type::Int),    // Quoted numbers stay numbers
            Expr::Float(_, _) => Ok(Type::Float),   // Quoted floats stay floats
            Expr::String(_, _) => Ok(Type::String), // Quoted strings stay strings
            Expr::Bool(_, _) => Ok(Type::Bool),     // Quoted bools stay bools
            Expr::List(_, _) => Ok(Type::List(Box::new(Type::String))), // Quoted lists become lists of strings
            _ => Ok(Type::String), // Default to string for other quoted expressions
        }
    }

    /// Infer type of define expression
    fn infer_define_type(&mut self, name: &str, value_expr: &Expr<()>) -> TlispResult<Type> {
        // Special handling for lambda expressions to support recursion
        if let Expr::Lambda(params, body, _) = value_expr {
            // Create a placeholder function type for recursion
            let param_types: Vec<Type> = params.iter().map(|_| Type::TypeVar("T".to_string())).collect();
            let return_type = Type::TypeVar("R".to_string());
            let function_type = Type::Function(param_types.clone(), Box::new(return_type.clone()));

            // Bind the function to itself for recursive calls
            self.env.bind_var(name.to_string(), function_type.clone());

            // Now infer the actual lambda type
            let lambda_type = self.infer_lambda_type(params, body)?;

            // Update the binding with the actual type
            self.env.bind_var(name.to_string(), lambda_type.clone());

            // For dependent types, also bind the value if it's a literal
            if let Some(value) = self.expr_to_value(value_expr) {
                self.env.bind_value(name.to_string(), value);
            }

            return Ok(Type::Unit);
        }

        // For non-lambda expressions, use the original logic
        let value_type = self.infer_type_internal(value_expr)?;

        // Bind the variable in the environment
        self.env.bind_var(name.to_string(), value_type.clone());

        // For dependent types, also bind the value if it's a literal
        if let Some(value) = self.expr_to_value(value_expr) {
            self.env.bind_value(name.to_string(), value);
        }

        // Define expressions return Unit type
        Ok(Type::Unit)
    }

    /// Infer type of set expression
    fn infer_set_type(&mut self, name: &str, value_expr: &Expr<()>) -> TlispResult<Type> {
        // Check that the variable exists
        let existing_type = self.env.lookup_var(name)
            .ok_or_else(|| TypeError::UndefinedVariable(name.to_string()))?;

        // Infer the type of the new value
        let value_type = self.infer_type_internal(value_expr)?;

        // Check that the new value has the same type as the existing variable
        self.context.add_constraint(Constraint::TypeEquality(existing_type, value_type));

        // Update the value binding if it's a literal
        if let Some(value) = self.expr_to_value(value_expr) {
            self.env.bind_value(name.to_string(), value);
        }

        // Set expressions return Unit type
        Ok(Type::Unit)
    }

    /// Create a dependent function type
    pub fn create_dependent_function(&mut self, param_name: String, param_type: Type, return_type: Type) -> Type {
        Type::DepFunction {
            param_name,
            param_type: Box::new(param_type),
            return_type: Box::new(return_type),
        }
    }

    /// Create a refinement type
    pub fn create_refinement_type(&mut self, var: String, base_type: Type, predicate: TypeTerm) -> Type {
        Type::Refinement {
            var,
            base_type: Box::new(base_type),
            predicate: Box::new(predicate),
        }
    }

    /// Infer type for dependent function application with multiple parameters
    pub fn infer_dependent_application(&mut self, func_type: &Type, args: &[Expr<()>]) -> TlispResult<Type> {
        match func_type {
            Type::DepFunction { param_name, param_type, return_type } => {
                if args.len() != 1 {
                    return Err(TypeError::ArityMismatch {
                        expected: 1,
                        actual: args.len(),
                    }.into());
                }

                // Infer argument type
                let arg_type = self.infer_type_internal(&args[0])?;

                // Check parameter type compatibility
                self.context.add_constraint(Constraint::TypeEquality(*param_type.clone(), arg_type));

                // Substitute the argument in the return type
                let substituted_return = self.substitute_in_type(param_name, &args[0], return_type)?;

                Ok(substituted_return)
            }

            Type::Function(param_types, return_type) => {
                // Regular function application
                if param_types.len() != args.len() {
                    return Err(TypeError::ArityMismatch {
                        expected: param_types.len(),
                        actual: args.len(),
                    }.into());
                }

                // Check each argument type
                for (param_type, arg) in param_types.iter().zip(args.iter()) {
                    let arg_type = self.infer_type_internal(arg)?;
                    self.context.add_constraint(Constraint::TypeEquality(param_type.clone(), arg_type));
                }

                Ok(*return_type.clone())
            }

            _ => Err(TypeError::Mismatch {
                expected: "function type".to_string(),
                actual: format!("{:?}", func_type),
            }.into()),
        }
    }

    /// Check if a type is a dependent type
    pub fn is_dependent_type(&self, ty: &Type) -> bool {
        match ty {
            Type::DepFunction { .. } => true,
            Type::Refinement { .. } => true,
            Type::TypeApp { .. } => true,
            Type::TypeLambda { .. } => true,
            Type::Function(param_types, return_type) => {
                param_types.iter().any(|pt| self.is_dependent_type(pt)) || self.is_dependent_type(return_type)
            }
            Type::List(elem_type) => self.is_dependent_type(elem_type),
            _ => false,
        }
    }

    /// Normalize a dependent type by applying substitutions
    pub fn normalize_type(&mut self, ty: &Type) -> TlispResult<Type> {
        match ty {
            Type::TypeApp { constructor, args } => {
                // Apply type constructor to arguments
                match constructor.as_ref() {
                    Type::TypeLambda { param, param_kind: _, body } => {
                        if args.len() != 1 {
                            return Err(TypeError::ArityMismatch {
                                expected: 1,
                                actual: args.len(),
                            }.into());
                        }

                        // Convert type term to type for substitution
                        let arg_type = self.type_term_to_type(args[0].clone())?;
                        let substituted = self.substitute_type_var(param, &arg_type, body)?;
                        self.normalize_type(&substituted)
                    }

                    _ => Ok(ty.clone()),
                }
            }

            Type::Function(param_types, return_type) => {
                let normalized_params = param_types.iter()
                    .map(|pt| self.normalize_type(pt))
                    .collect::<Result<Vec<_>, _>>()?;
                let normalized_return = self.normalize_type(return_type)?;

                Ok(Type::Function(normalized_params, Box::new(normalized_return)))
            }

            Type::List(elem_type) => {
                let normalized_elem = self.normalize_type(elem_type)?;
                Ok(Type::List(Box::new(normalized_elem)))
            }

            _ => Ok(ty.clone()),
        }
    }

    /// Advanced type substitution with capture avoidance
    pub fn substitute_with_capture_avoidance(&mut self, var: &str, replacement: &Type, ty: &Type) -> TlispResult<Type> {
        self.substitute_with_capture_avoidance_internal(var, replacement, ty, &mut std::collections::HashSet::new())
    }

    /// Internal implementation of capture-avoiding substitution
    fn substitute_with_capture_avoidance_internal(
        &mut self,
        var: &str,
        replacement: &Type,
        ty: &Type,
        bound_vars: &mut std::collections::HashSet<String>
    ) -> TlispResult<Type> {
        match ty {
            Type::TypeVar(name) if name == var && !bound_vars.contains(name) => {
                Ok(replacement.clone())
            }

            Type::TypeLambda { param, param_kind, body } => {
                if param == var {
                    // Variable is bound in this lambda, don't substitute
                    Ok(ty.clone())
                } else if self.occurs_free_in_type(param, replacement) {
                    // Need to rename to avoid capture
                    let fresh_param = self.generate_fresh_type_var(param);
                    let renamed_body = self.substitute_type_var(param, &Type::TypeVar(fresh_param.clone()), body)?;

                    bound_vars.insert(fresh_param.clone());
                    let substituted_body = self.substitute_with_capture_avoidance_internal(var, replacement, &renamed_body, bound_vars)?;
                    bound_vars.remove(&fresh_param);

                    Ok(Type::TypeLambda {
                        param: fresh_param,
                        param_kind: param_kind.clone(),
                        body: Box::new(substituted_body),
                    })
                } else {
                    bound_vars.insert(param.clone());
                    let substituted_body = self.substitute_with_capture_avoidance_internal(var, replacement, body, bound_vars)?;
                    bound_vars.remove(param);

                    Ok(Type::TypeLambda {
                        param: param.clone(),
                        param_kind: param_kind.clone(),
                        body: Box::new(substituted_body),
                    })
                }
            }

            Type::DepFunction { param_name, param_type, return_type } => {
                if param_name == var {
                    // Variable is bound in this dependent function
                    let substituted_param_type = self.substitute_with_capture_avoidance_internal(var, replacement, param_type, bound_vars)?;
                    Ok(Type::DepFunction {
                        param_name: param_name.clone(),
                        param_type: Box::new(substituted_param_type),
                        return_type: return_type.clone(),
                    })
                } else if self.occurs_free_in_type(param_name, replacement) {
                    // Need to rename to avoid capture
                    let fresh_param = self.generate_fresh_var(param_name);
                    let renamed_return = self.substitute_type_var(param_name, &Type::TypeVar(fresh_param.clone()), return_type)?;

                    let substituted_param_type = self.substitute_with_capture_avoidance_internal(var, replacement, param_type, bound_vars)?;
                    bound_vars.insert(fresh_param.clone());
                    let substituted_return_type = self.substitute_with_capture_avoidance_internal(var, replacement, &renamed_return, bound_vars)?;
                    bound_vars.remove(&fresh_param);

                    Ok(Type::DepFunction {
                        param_name: fresh_param,
                        param_type: Box::new(substituted_param_type),
                        return_type: Box::new(substituted_return_type),
                    })
                } else {
                    let substituted_param_type = self.substitute_with_capture_avoidance_internal(var, replacement, param_type, bound_vars)?;
                    bound_vars.insert(param_name.clone());
                    let substituted_return_type = self.substitute_with_capture_avoidance_internal(var, replacement, return_type, bound_vars)?;
                    bound_vars.remove(param_name);

                    Ok(Type::DepFunction {
                        param_name: param_name.clone(),
                        param_type: Box::new(substituted_param_type),
                        return_type: Box::new(substituted_return_type),
                    })
                }
            }

            Type::Function(param_types, return_type) => {
                let substituted_params = param_types.iter()
                    .map(|pt| self.substitute_with_capture_avoidance_internal(var, replacement, pt, bound_vars))
                    .collect::<Result<Vec<_>, _>>()?;
                let substituted_return = self.substitute_with_capture_avoidance_internal(var, replacement, return_type, bound_vars)?;

                Ok(Type::Function(substituted_params, Box::new(substituted_return)))
            }

            Type::List(elem_type) => {
                let substituted_elem = self.substitute_with_capture_avoidance_internal(var, replacement, elem_type, bound_vars)?;
                Ok(Type::List(Box::new(substituted_elem)))
            }

            _ => Ok(ty.clone()),
        }
    }

    /// Check if a variable occurs free in a type
    fn occurs_free_in_type(&self, var: &str, ty: &Type) -> bool {
        match ty {
            Type::TypeVar(name) => name == var,
            Type::Function(param_types, return_type) => {
                param_types.iter().any(|pt| self.occurs_free_in_type(var, pt)) ||
                self.occurs_free_in_type(var, return_type)
            }
            Type::List(elem_type) => self.occurs_free_in_type(var, elem_type),
            Type::TypeLambda { param, param_kind: _, body } => {
                param != var && self.occurs_free_in_type(var, body)
            }
            Type::DepFunction { param_name, param_type, return_type } => {
                self.occurs_free_in_type(var, param_type) ||
                (param_name != var && self.occurs_free_in_type(var, return_type))
            }
            _ => false,
        }
    }

    /// Generate a fresh type variable name
    fn generate_fresh_type_var(&mut self, base: &str) -> String {
        let mut counter = 0;
        loop {
            let candidate = if counter == 0 {
                format!("{}'", base)
            } else {
                format!("{}{}", base, counter)
            };

            // Check if this name is already in use (simplified check)
            if !self.env.lookup_type_constructor(&candidate).is_some() {
                return candidate;
            }
            counter += 1;
        }
    }

    /// Generate a fresh variable name
    fn generate_fresh_var(&mut self, base: &str) -> String {
        let mut counter = 0;
        loop {
            let candidate = if counter == 0 {
                format!("{}'", base)
            } else {
                format!("{}{}", base, counter)
            };

            // Check if this name is already in use (simplified check)
            if !self.env.lookup_var(&candidate).is_some() {
                return candidate;
            }
            counter += 1;
        }
    }

    /// Compose two substitutions
    pub fn compose_substitutions(&mut self, first: &Substitution, second: &Substitution) -> TlispResult<Substitution> {
        let mut result = Substitution::new();

        // Apply second substitution to the range of first substitution
        for (var, ty) in first.bindings() {
            let substituted_ty = second.apply(ty);
            result.bind(var.clone(), substituted_ty);
        }

        // Add bindings from second substitution that are not in first
        for (var, ty) in second.bindings() {
            if !first.bindings().contains_key(var) {
                result.bind(var.clone(), ty.clone());
            }
        }

        Ok(result)
    }

    /// Apply multiple substitutions in sequence
    pub fn apply_substitutions(&mut self, substitutions: &[Substitution], ty: &Type) -> TlispResult<Type> {
        let mut result = ty.clone();
        for subst in substitutions {
            result = subst.apply(&result);
        }
        Ok(result)
    }

    /// Parallel substitution (simultaneous substitution of multiple variables)
    pub fn parallel_substitute(&mut self, substitutions: &[(String, Type)], ty: &Type) -> TlispResult<Type> {
        // Create a substitution object and apply it
        let mut subst = Substitution::new();
        for (var, replacement) in substitutions {
            subst.bind(var.clone(), replacement.clone());
        }
        Ok(subst.apply(ty))
    }

    /// Advanced type term substitution with capture avoidance
    pub fn substitute_type_term_advanced(&mut self, var: &str, replacement: &TypeTerm, term: &TypeTerm) -> TlispResult<TypeTerm> {
        self.substitute_type_term_internal(var, replacement, term, &mut std::collections::HashSet::new())
    }

    /// Internal implementation of type term substitution
    fn substitute_type_term_internal(
        &mut self,
        var: &str,
        replacement: &TypeTerm,
        term: &TypeTerm,
        bound_vars: &mut std::collections::HashSet<String>
    ) -> TlispResult<TypeTerm> {
        match term {
            TypeTerm::Var(name) if name == var && !bound_vars.contains(name) => {
                Ok(replacement.clone())
            }

            TypeTerm::App(func, args) => {
                let substituted_func = self.substitute_type_term_internal(var, replacement, func, bound_vars)?;
                let substituted_args = args.iter()
                    .map(|arg| self.substitute_type_term_internal(var, replacement, arg, bound_vars))
                    .collect::<Result<Vec<_>, _>>()?;

                Ok(TypeTerm::App(Box::new(substituted_func), substituted_args))
            }

            TypeTerm::Lambda(param, param_type, body) => {
                if param == var {
                    // Variable is bound in this lambda
                    // For type terms, we need a different substitution approach
                    Ok(TypeTerm::Lambda(param.clone(), param_type.clone(), body.clone()))
                } else if self.occurs_free_in_type_term(param, replacement) {
                    // Need to rename to avoid capture
                    let fresh_param = self.generate_fresh_var(param);
                    let renamed_body = self.substitute_type_term_var(param, &TypeTerm::Var(fresh_param.clone()), body)?;

                    bound_vars.insert(fresh_param.clone());
                    let substituted_body = self.substitute_type_term_internal(var, replacement, &renamed_body, bound_vars)?;
                    bound_vars.remove(&fresh_param);

                    Ok(TypeTerm::Lambda(fresh_param, param_type.clone(), Box::new(substituted_body)))
                } else {
                    bound_vars.insert(param.clone());
                    let substituted_body = self.substitute_type_term_internal(var, replacement, body, bound_vars)?;
                    bound_vars.remove(param);

                    Ok(TypeTerm::Lambda(param.clone(), param_type.clone(), Box::new(substituted_body)))
                }
            }

            TypeTerm::Annotated(inner_term, ty) => {
                let substituted_term = self.substitute_type_term_internal(var, replacement, inner_term, bound_vars)?;
                // For now, don't substitute in the type annotation
                Ok(TypeTerm::Annotated(Box::new(substituted_term), ty.clone()))
            }

            _ => Ok(term.clone()),
        }
    }

    /// Check if a variable occurs free in a type term
    fn occurs_free_in_type_term(&self, var: &str, term: &TypeTerm) -> bool {
        match term {
            TypeTerm::Var(name) => name == var,
            TypeTerm::App(func, args) => {
                self.occurs_free_in_type_term(var, func) ||
                args.iter().any(|arg| self.occurs_free_in_type_term(var, arg))
            }
            TypeTerm::Lambda(param, param_type, body) => {
                self.occurs_free_in_type(var, param_type) ||
                (param != var && self.occurs_free_in_type_term(var, body))
            }
            TypeTerm::Annotated(inner_term, ty) => {
                self.occurs_free_in_type_term(var, inner_term) ||
                self.occurs_free_in_type(var, ty)
            }
            _ => false,
        }
    }

    /// Substitute type term variable
    fn substitute_type_term_var(&mut self, var: &str, replacement: &TypeTerm, term: &TypeTerm) -> TlispResult<TypeTerm> {
        match term {
            TypeTerm::Var(name) if name == var => Ok(replacement.clone()),

            TypeTerm::App(func, args) => {
                let substituted_func = self.substitute_type_term_var(var, replacement, func)?;
                let substituted_args = args.iter()
                    .map(|arg| self.substitute_type_term_var(var, replacement, arg))
                    .collect::<Result<Vec<_>, _>>()?;

                Ok(TypeTerm::App(Box::new(substituted_func), substituted_args))
            }

            TypeTerm::Lambda(param, param_type, body) => {
                if param == var {
                    // Variable is bound in this lambda
                    Ok(term.clone())
                } else {
                    let substituted_body = self.substitute_type_term_var(var, replacement, body)?;
                    Ok(TypeTerm::Lambda(param.clone(), param_type.clone(), Box::new(substituted_body)))
                }
            }

            TypeTerm::Annotated(inner_term, ty) => {
                let substituted_term = self.substitute_type_term_var(var, replacement, inner_term)?;
                Ok(TypeTerm::Annotated(Box::new(substituted_term), ty.clone()))
            }

            _ => Ok(term.clone()),
        }
    }

    /// Substitute term in type (for dependent types)
    fn substitute_in_type(&mut self, var: &str, term: &Expr<()>, ty: &Type) -> TlispResult<Type> {
        match ty {
            Type::TypeVar(name) if name == var => {
                // Convert term to type-term and then to type
                let type_term = self.expr_to_type_term(term)?;
                self.type_term_to_type(type_term)
            }

            Type::TypeApp { constructor, args } => {
                let new_args = args.iter()
                    .map(|arg| self.substitute_in_type_term(var, term, arg))
                    .collect::<Result<Vec<_>, _>>()?;

                Ok(Type::TypeApp {
                    constructor: constructor.clone(),
                    args: new_args,
                })
            }

            Type::Refinement { var: ref_var, base_type, predicate } => {
                if ref_var == var {
                    // Variable is bound in refinement
                    Ok(ty.clone())
                } else {
                    let new_base = self.substitute_in_type(var, term, base_type)?;
                    let new_predicate = self.substitute_in_type_term(var, term, predicate)?;

                    Ok(Type::Refinement {
                        var: ref_var.clone(),
                        base_type: Box::new(new_base),
                        predicate: Box::new(new_predicate),
                    })
                }
            }

            Type::DepFunction { param_name, param_type, return_type } => {
                if param_name == var {
                    // Variable is bound in dependent function
                    Ok(ty.clone())
                } else {
                    let new_param_type = self.substitute_in_type(var, term, param_type)?;
                    let new_return_type = self.substitute_in_type(var, term, return_type)?;

                    Ok(Type::DepFunction {
                        param_name: param_name.clone(),
                        param_type: Box::new(new_param_type),
                        return_type: Box::new(new_return_type),
                    })
                }
            }

            Type::Function(param_types, return_type) => {
                let new_param_types = param_types.iter()
                    .map(|pt| self.substitute_in_type(var, term, pt))
                    .collect::<Result<Vec<_>, _>>()?;
                let new_return_type = self.substitute_in_type(var, term, return_type)?;

                Ok(Type::Function(new_param_types, Box::new(new_return_type)))
            }

            Type::List(elem_type) => {
                let new_elem_type = self.substitute_in_type(var, term, elem_type)?;
                Ok(Type::List(Box::new(new_elem_type)))
            }

            Type::TypeLambda { param, param_kind, body } => {
                if param == var {
                    // Variable is bound in type lambda
                    Ok(ty.clone())
                } else {
                    let new_body = self.substitute_in_type(var, term, body)?;
                    Ok(Type::TypeLambda {
                        param: param.clone(),
                        param_kind: param_kind.clone(),
                        body: Box::new(new_body),
                    })
                }
            }

            _ => Ok(ty.clone()), // No substitution needed for other types
        }
    }

    /// Substitute type variable in type
    fn substitute_type_var(&mut self, var: &str, replacement: &Type, ty: &Type) -> TlispResult<Type> {
        match ty {
            Type::TypeVar(name) if name == var => Ok(replacement.clone()),

            Type::Function(param_types, return_type) => {
                let new_param_types = param_types.iter()
                    .map(|pt| self.substitute_type_var(var, replacement, pt))
                    .collect::<Result<Vec<_>, _>>()?;
                let new_return_type = self.substitute_type_var(var, replacement, return_type)?;

                Ok(Type::Function(new_param_types, Box::new(new_return_type)))
            }

            Type::List(elem_type) => {
                let new_elem_type = self.substitute_type_var(var, replacement, elem_type)?;
                Ok(Type::List(Box::new(new_elem_type)))
            }

            Type::TypeLambda { param, param_kind, body } => {
                if param == var {
                    // Variable is bound in type lambda
                    Ok(ty.clone())
                } else {
                    let new_body = self.substitute_type_var(var, replacement, body)?;
                    Ok(Type::TypeLambda {
                        param: param.clone(),
                        param_kind: param_kind.clone(),
                        body: Box::new(new_body),
                    })
                }
            }

            _ => Ok(ty.clone()), // No substitution needed for other types
        }
    }

    /// Substitute in type term
    fn substitute_in_type_term(&mut self, var: &str, term: &Expr<()>, type_term: &TypeTerm) -> TlispResult<TypeTerm> {
        match type_term {
            TypeTerm::Var(name) if name == var => {
                self.expr_to_type_term(term)
            }

            TypeTerm::App(func, args) => {
                let new_func = self.substitute_in_type_term(var, term, func)?;
                let new_args = args.iter()
                    .map(|arg| self.substitute_in_type_term(var, term, arg))
                    .collect::<Result<Vec<_>, _>>()?;

                Ok(TypeTerm::App(Box::new(new_func), new_args))
            }

            TypeTerm::Lambda(param, param_type, body) => {
                if param == var {
                    // Variable is bound in lambda
                    Ok(type_term.clone())
                } else {
                    let new_param_type = self.substitute_in_type(var, term, param_type)?;
                    let new_body = self.substitute_in_type_term(var, term, body)?;

                    Ok(TypeTerm::Lambda(param.clone(), Box::new(new_param_type), Box::new(new_body)))
                }
            }

            TypeTerm::Annotated(inner_term, ty) => {
                let new_term = self.substitute_in_type_term(var, term, inner_term)?;
                let new_type = self.substitute_in_type(var, term, ty)?;

                Ok(TypeTerm::Annotated(Box::new(new_term), Box::new(new_type)))
            }

            _ => Ok(type_term.clone()), // No substitution needed for literals and other terms
        }
    }

    /// Convert expression to type term
    fn expr_to_type_term(&mut self, expr: &Expr<()>) -> TlispResult<TypeTerm> {
        match expr {
            Expr::Symbol(name, _) => Ok(TypeTerm::Var(name.clone())),

            Expr::Number(n, _) => Ok(TypeTerm::Literal(Value::Int(*n))),
            Expr::Float(f, _) => Ok(TypeTerm::Literal(Value::Float(*f))),
            Expr::Bool(b, _) => Ok(TypeTerm::Literal(Value::Bool(*b))),
            Expr::String(s, _) => Ok(TypeTerm::Literal(Value::String(s.clone()))),

            Expr::Application(func, args, _) => {
                let func_term = self.expr_to_type_term(func)?;
                let arg_terms = args.iter()
                    .map(|arg| self.expr_to_type_term(arg))
                    .collect::<Result<Vec<_>, _>>()?;

                Ok(TypeTerm::App(Box::new(func_term), arg_terms))
            }

            Expr::Lambda(params, body, _) => {
                if params.len() != 1 {
                    return Err(TypeError::UnsupportedExpression(
                        "Multi-parameter lambdas not supported in type terms".to_string()
                    ).into());
                }

                let param = &params[0];
                let param_type = self.context.fresh_var(); // Infer parameter type
                let body_term = self.expr_to_type_term(body)?;

                Ok(TypeTerm::Lambda(param.clone(), Box::new(param_type), Box::new(body_term)))
            }

            _ => Err(TypeError::UnsupportedExpression(
                format!("Cannot convert {:?} to type term", expr)
            ).into()),
        }
    }

    /// Convert type term to type
    fn type_term_to_type(&mut self, type_term: TypeTerm) -> TlispResult<Type> {
        match type_term {
            TypeTerm::Var(name) => {
                // Look up type constructor or create type variable
                Ok(self.env.lookup_type_constructor(&name)
                    .unwrap_or_else(|| Type::TypeVar(name)))
            }

            TypeTerm::Literal(Value::Int(n)) => {
                // Integer literals in types represent type-level constants
                Ok(Type::TypeApp {
                    constructor: Box::new(Type::TypeVar("Const".to_string())),
                    args: vec![TypeTerm::Literal(Value::Int(n))],
                })
            }

            TypeTerm::App(func, args) => {
                let func_type = self.type_term_to_type(*func)?;
                Ok(Type::TypeApp {
                    constructor: Box::new(func_type),
                    args,
                })
            }

            _ => Err(TypeError::UnsupportedExpression(
                format!("Cannot convert type term {:?} to type", type_term)
            ).into()),
        }
    }

    /// Convert expression to value (for literals)
    fn expr_to_value(&self, expr: &Expr<()>) -> Option<Value> {
        match expr {
            Expr::Number(n, _) => Some(Value::Int(*n)),
            Expr::Float(f, _) => Some(Value::Float(*f)),
            Expr::Bool(b, _) => Some(Value::Bool(*b)),
            Expr::String(s, _) => Some(Value::String(s.clone())),
            _ => None,
        }
    }

    /// Check type against annotation
    pub fn check_type(&mut self, expr: &Expr<()>, expected: &Type) -> TlispResult<()> {
        let inferred = self.infer_type_internal(expr)?;

        // Add constraint for type equality
        self.context.add_constraint(Constraint::TypeEquality(inferred, expected.clone()));

        // Solve constraints
        for constraint in self.context.constraints() {
            self.solver.solve(constraint)?;
        }

        Ok(())
    }

    /// Check refinement type
    pub fn check_refinement(&mut self, expr: &Expr<()>, var: &str, base_type: &Type, predicate: &TypeTerm) -> TlispResult<()> {
        // Check that expression has base type
        self.check_type(expr, base_type)?;

        // Check that predicate holds for expression
        let substituted_predicate = self.substitute_in_type_term(var, expr, predicate)?;

        // Add constraint for predicate satisfaction
        self.context.add_constraint(Constraint::Refinement {
            var: var.to_string(),
            var_type: base_type.clone(),
            predicate: substituted_predicate,
        });

        // Solve constraints
        for constraint in self.context.constraints() {
            self.solver.solve(constraint)?;
        }

        Ok(())
    }

    /// Get current type environment (for testing)
    pub fn env(&self) -> &TypeEnvironment {
        &self.env
    }

    /// Get current constraint solver (for testing)
    pub fn solver(&mut self) -> &mut ConstraintSolver {
        &mut self.solver
    }
}

impl Default for DependentTypeChecker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dependent_type_checker_creation() {
        let checker = DependentTypeChecker::new();

        // Check that built-ins are available
        assert!(checker.env().lookup_var("+").is_some());
        assert!(checker.env().lookup_var("=").is_some());
        assert!(checker.env().lookup_type_constructor("Int").is_some());
        assert!(checker.env().lookup_type_constructor("List").is_some());
    }

    #[test]
    fn test_basic_type_inference() {
        let mut checker = DependentTypeChecker::new();

        // Test integer literal
        let expr = Expr::Number(42, ());
        let ty = checker.infer_type(&expr).unwrap();
        assert_eq!(ty, Type::Int);

        // Test boolean literal
        let expr = Expr::Bool(true, ());
        let ty = checker.infer_type(&expr).unwrap();
        assert_eq!(ty, Type::Bool);

        // Test string literal
        let expr = Expr::String("hello".to_string(), ());
        let ty = checker.infer_type(&expr).unwrap();
        assert_eq!(ty, Type::String);
    }

    #[test]
    fn test_variable_lookup() {
        let mut checker = DependentTypeChecker::new();

        // Define a variable
        checker.define_var("x".to_string(), Type::Int);

        // Test variable lookup
        let expr = Expr::Symbol("x".to_string(), ());
        let ty = checker.infer_type(&expr).unwrap();
        assert_eq!(ty, Type::Int);
    }

    #[test]
    fn test_function_application() {
        let mut checker = DependentTypeChecker::new();

        // Test built-in function application: (+ 1 2)
        let func = Expr::Symbol("+".to_string(), ());
        let args = vec![
            Expr::Number(1, ()),
            Expr::Number(2, ()),
        ];
        let expr = Expr::Application(Box::new(func), args, ());

        let ty = checker.infer_type(&expr).unwrap();
        assert_eq!(ty, Type::Int);
    }

    #[test]
    fn test_lambda_type_inference() {
        let mut checker = DependentTypeChecker::new();

        // Test lambda: (lambda (x) x)
        let params = vec!["x".to_string()];
        let body = Expr::Symbol("x".to_string(), ());
        let expr = Expr::Lambda(params, Box::new(body), ());

        let ty = checker.infer_type(&expr).unwrap();

        // Should be a function type
        match ty {
            Type::Function(param_types, _) => {
                assert_eq!(param_types.len(), 1);
            }
            _ => panic!("Expected function type, got {:?}", ty),
        }
    }

    #[test]
    fn test_dependent_function_creation() {
        let mut checker = DependentTypeChecker::new();

        // Create a dependent function: (n: Int) -> Vec(n, String)
        let dep_func = checker.create_dependent_function(
            "n".to_string(),
            Type::Int,
            Type::TypeApp {
                constructor: Box::new(Type::TypeVar("Vec".to_string())),
                args: vec![
                    TypeTerm::Var("n".to_string()),
                    TypeTerm::Var("String".to_string()),
                ],
            }
        );

        match dep_func {
            Type::DepFunction { param_name, param_type, return_type } => {
                assert_eq!(param_name, "n");
                assert_eq!(*param_type, Type::Int);
                match *return_type {
                    Type::TypeApp { .. } => {}, // Expected
                    _ => panic!("Expected TypeApp for return type"),
                }
            }
            _ => panic!("Expected DepFunction"),
        }
    }

    #[test]
    fn test_refinement_type_creation() {
        let mut checker = DependentTypeChecker::new();

        // Create a refinement type: {x: Int | x > 0}
        let refinement = checker.create_refinement_type(
            "x".to_string(),
            Type::Int,
            TypeTerm::App(
                Box::new(TypeTerm::Var(">".to_string())),
                vec![
                    TypeTerm::Var("x".to_string()),
                    TypeTerm::Literal(Value::Int(0)),
                ],
            )
        );

        match refinement {
            Type::Refinement { var, base_type, predicate } => {
                assert_eq!(var, "x");
                assert_eq!(*base_type, Type::Int);
                match *predicate {
                    TypeTerm::App(_, _) => {}, // Expected
                    _ => panic!("Expected App for predicate"),
                }
            }
            _ => panic!("Expected Refinement"),
        }
    }

    #[test]
    fn test_dependent_type_detection() {
        let checker = DependentTypeChecker::new();

        // Test basic types
        assert!(!checker.is_dependent_type(&Type::Int));
        assert!(!checker.is_dependent_type(&Type::Bool));

        // Test dependent function
        let dep_func = Type::DepFunction {
            param_name: "x".to_string(),
            param_type: Box::new(Type::Int),
            return_type: Box::new(Type::Int),
        };
        assert!(checker.is_dependent_type(&dep_func));

        // Test refinement type
        let refinement = Type::Refinement {
            var: "x".to_string(),
            base_type: Box::new(Type::Int),
            predicate: Box::new(TypeTerm::Literal(Value::Bool(true))),
        };
        assert!(checker.is_dependent_type(&refinement));

        // Test type application
        let type_app = Type::TypeApp {
            constructor: Box::new(Type::TypeVar("Vec".to_string())),
            args: vec![TypeTerm::Literal(Value::Int(5))],
        };
        assert!(checker.is_dependent_type(&type_app));
    }

    #[test]
    fn test_capture_avoiding_substitution() {
        let mut checker = DependentTypeChecker::new();

        // Test simple substitution without capture
        let ty = Type::Function(vec![Type::TypeVar("T".to_string())], Box::new(Type::Int));
        let replacement = Type::Bool;
        let result = checker.substitute_with_capture_avoidance("T", &replacement, &ty).unwrap();

        match result {
            Type::Function(params, _) => {
                assert_eq!(params[0], Type::Bool);
            }
            _ => panic!("Expected function type"),
        }
    }

    #[test]
    fn test_substitution_composition() {
        let mut checker = DependentTypeChecker::new();

        // Create two substitutions
        let mut subst1 = Substitution::new();
        subst1.bind("T".to_string(), Type::Int);

        let mut subst2 = Substitution::new();
        subst2.bind("U".to_string(), Type::Bool);

        // Compose them
        let composed = checker.compose_substitutions(&subst1, &subst2).unwrap();

        // Check that both bindings are present
        assert!(composed.contains("T"));
        assert!(composed.contains("U"));
        assert_eq!(composed.get("T"), Some(&Type::Int));
        assert_eq!(composed.get("U"), Some(&Type::Bool));
    }

    #[test]
    fn test_parallel_substitution() {
        let mut checker = DependentTypeChecker::new();

        // Create a type with multiple variables
        let ty = Type::Function(
            vec![Type::TypeVar("T".to_string()), Type::TypeVar("U".to_string())],
            Box::new(Type::TypeVar("T".to_string()))
        );

        // Apply parallel substitution
        let substitutions = vec![
            ("T".to_string(), Type::Int),
            ("U".to_string(), Type::Bool),
        ];

        let result = checker.parallel_substitute(&substitutions, &ty).unwrap();

        match result {
            Type::Function(params, return_type) => {
                assert_eq!(params[0], Type::Int);
                assert_eq!(params[1], Type::Bool);
                assert_eq!(return_type.as_ref(), &Type::Int);
            }
            _ => panic!("Expected function type"),
        }
    }

    #[test]
    fn test_type_normalization() {
        let mut checker = DependentTypeChecker::new();

        // Test normalization of a simple function type
        let func_ty = Type::Function(
            vec![Type::TypeVar("T".to_string())],
            Box::new(Type::TypeVar("U".to_string()))
        );

        // Normalize the function type (should remain unchanged for now)
        let result = checker.normalize_type(&func_ty).unwrap();

        match result {
            Type::Function(params, return_type) => {
                assert!(matches!(params[0], Type::TypeVar(_)));
                assert!(matches!(return_type.as_ref(), Type::TypeVar(_)));
            }
            _ => panic!("Expected function type after normalization"),
        }

        // Test normalization of a list type
        let list_ty = Type::List(Box::new(Type::TypeVar("T".to_string())));
        let normalized_list = checker.normalize_type(&list_ty).unwrap();

        match normalized_list {
            Type::List(elem_type) => {
                assert!(matches!(elem_type.as_ref(), Type::TypeVar(_)));
            }
            _ => panic!("Expected list type"),
        }
    }

    #[test]
    fn test_occurs_check() {
        let checker = DependentTypeChecker::new();

        // Test occurs check for simple case
        let ty = Type::TypeVar("T".to_string());
        assert!(checker.occurs_free_in_type("T", &ty));
        assert!(!checker.occurs_free_in_type("U", &ty));

        // Test occurs check in function type
        let func_ty = Type::Function(
            vec![Type::TypeVar("T".to_string())],
            Box::new(Type::TypeVar("U".to_string()))
        );
        assert!(checker.occurs_free_in_type("T", &func_ty));
        assert!(checker.occurs_free_in_type("U", &func_ty));
        assert!(!checker.occurs_free_in_type("V", &func_ty));

        // Test occurs check with binding (type lambda)
        let type_lambda = Type::TypeLambda {
            param: "T".to_string(),
            param_kind: Kind::Type,
            body: Box::new(Type::TypeVar("T".to_string())),
        };
        assert!(!checker.occurs_free_in_type("T", &type_lambda)); // T is bound

        let type_lambda_free = Type::TypeLambda {
            param: "T".to_string(),
            param_kind: Kind::Type,
            body: Box::new(Type::TypeVar("U".to_string())),
        };
        assert!(checker.occurs_free_in_type("U", &type_lambda_free)); // U is free
    }
}
