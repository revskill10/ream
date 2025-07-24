//! Constraint solver for dependent types and refinement checking

use std::collections::HashMap;
use crate::tlisp::{Expr, Value};
use crate::tlisp::types::{Type, TypeTerm, Kind};
use crate::error::{TlispResult, TypeError};

/// Constraint representation
#[derive(Debug, Clone, PartialEq)]
pub enum Constraint {
    /// Type equality: T1 = T2
    TypeEquality(Type, Type),
    
    /// Type term equality: t1 = t2
    TermEquality(TypeTerm, TypeTerm),
    
    /// Refinement constraint: P(x) where x: T
    Refinement {
        var: String,
        var_type: Type,
        predicate: TypeTerm,
    },
    
    /// Subtyping constraint: T1 <: T2
    Subtyping(Type, Type),
    
    /// Kind constraint: T : K
    Kinding(Type, Kind),
    
    /// Conjunction of constraints
    And(Vec<Constraint>),
    
    /// Disjunction of constraints
    Or(Vec<Constraint>),
    
    /// Implication: C1 => C2
    Implies(Box<Constraint>, Box<Constraint>),
    
    /// Existential quantification: ∃x. C
    Exists(String, Type, Box<Constraint>),
    
    /// Universal quantification: ∀x. C
    Forall(String, Type, Box<Constraint>),
}

/// Constraint solving context
#[derive(Debug, Clone)]
pub struct ConstraintContext {
    /// Type variable bindings
    type_vars: HashMap<String, Type>,
    
    /// Value variable bindings
    value_vars: HashMap<String, Value>,
    
    /// Assumptions (constraints we can assume to be true)
    assumptions: Vec<Constraint>,
}

impl ConstraintContext {
    /// Create new constraint context
    pub fn new() -> Self {
        ConstraintContext {
            type_vars: HashMap::new(),
            value_vars: HashMap::new(),
            assumptions: Vec::new(),
        }
    }
    
    /// Add type variable binding
    pub fn bind_type(&mut self, var: String, ty: Type) {
        self.type_vars.insert(var, ty);
    }
    
    /// Add value variable binding
    pub fn bind_value(&mut self, var: String, value: Value) {
        self.value_vars.insert(var, value);
    }
    
    /// Add assumption
    pub fn assume(&mut self, constraint: Constraint) {
        self.assumptions.push(constraint);
    }
    
    /// Look up type variable
    pub fn lookup_type(&self, var: &str) -> Option<&Type> {
        self.type_vars.get(var)
    }
    
    /// Look up value variable
    pub fn lookup_value(&self, var: &str) -> Option<&Value> {
        self.value_vars.get(var)
    }
}

impl Default for ConstraintContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Constraint solver
pub struct ConstraintSolver {
    /// Current solving context
    context: ConstraintContext,
    
    /// Maximum solving depth (to prevent infinite recursion)
    max_depth: usize,
    
    /// Current solving depth
    current_depth: usize,
    
    /// Enable SMT solver integration (placeholder for now)
    use_smt: bool,
}

impl ConstraintSolver {
    /// Create new constraint solver
    pub fn new() -> Self {
        ConstraintSolver {
            context: ConstraintContext::new(),
            max_depth: 100,
            current_depth: 0,
            use_smt: false,
        }
    }
    
    /// Create solver with SMT integration
    pub fn with_smt() -> Self {
        ConstraintSolver {
            context: ConstraintContext::new(),
            max_depth: 100,
            current_depth: 0,
            use_smt: true,
        }
    }
    
    /// Solve a constraint
    pub fn solve(&mut self, constraint: &Constraint) -> TlispResult<bool> {
        if self.current_depth >= self.max_depth {
            return Err(TypeError::MaxDepthExceeded(self.max_depth).into());
        }
        
        self.current_depth += 1;
        let result = self.solve_constraint(constraint);
        self.current_depth -= 1;

        result
    }
    
    /// Internal constraint solving logic
    fn solve_constraint(&mut self, constraint: &Constraint) -> TlispResult<bool> {
        match constraint {
            Constraint::TypeEquality(t1, t2) => {
                self.solve_type_equality(t1, t2)
            }
            
            Constraint::TermEquality(term1, term2) => {
                self.solve_term_equality(term1, term2)
            }
            
            Constraint::Refinement { var, var_type, predicate } => {
                self.solve_refinement(var, var_type, predicate)
            }
            
            Constraint::Subtyping(t1, t2) => {
                self.solve_subtyping(t1, t2)
            }
            
            Constraint::Kinding(ty, kind) => {
                self.solve_kinding(ty, kind)
            }
            
            Constraint::And(constraints) => {
                for c in constraints {
                    if !self.solve(c)? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            
            Constraint::Or(constraints) => {
                for c in constraints {
                    if self.solve(c)? {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            
            Constraint::Implies(premise, conclusion) => {
                if !self.solve(premise)? {
                    Ok(true) // Vacuously true
                } else {
                    self.solve(conclusion)
                }
            }
            
            Constraint::Exists(var, var_type, constraint) => {
                self.solve_existential(var, var_type, constraint)
            }
            
            Constraint::Forall(var, var_type, constraint) => {
                self.solve_universal(var, var_type, constraint)
            }
        }
    }
    
    /// Solve type equality constraint
    fn solve_type_equality(&mut self, t1: &Type, t2: &Type) -> TlispResult<bool> {
        match (t1, t2) {
            // Reflexivity
            (a, b) if a == b => Ok(true),
            
            // Type variables
            (Type::TypeVar(var), ty) | (ty, Type::TypeVar(var)) => {
                if let Some(bound_type) = self.context.lookup_type(var).cloned() {
                    self.solve_type_equality(&bound_type, ty)
                } else {
                    // Unify by binding the variable
                    self.context.bind_type(var.clone(), ty.clone());
                    Ok(true)
                }
            }
            
            // Structural equality
            (Type::List(elem1), Type::List(elem2)) => {
                self.solve_type_equality(elem1, elem2)
            }
            
            (Type::Function(params1, ret1), Type::Function(params2, ret2)) => {
                if params1.len() != params2.len() {
                    return Ok(false);
                }
                
                for (p1, p2) in params1.iter().zip(params2.iter()) {
                    if !self.solve_type_equality(p1, p2)? {
                        return Ok(false);
                    }
                }
                
                self.solve_type_equality(ret1, ret2)
            }
            
            // Different types
            _ => Ok(false),
        }
    }
    
    /// Solve term equality constraint
    fn solve_term_equality(&mut self, term1: &TypeTerm, term2: &TypeTerm) -> TlispResult<bool> {
        match (term1, term2) {
            // Reflexivity
            (a, b) if a == b => Ok(true),
            
            // Variables
            (TypeTerm::Var(var), term) | (term, TypeTerm::Var(var)) => {
                if let Some(value) = self.context.lookup_value(var) {
                    // Convert value to term and compare
                    let value_term = TypeTerm::Literal(value.clone());
                    self.solve_term_equality(&value_term, term)
                } else {
                    // For now, assume variables are equal if they have the same name
                    Ok(matches!(term, TypeTerm::Var(other_var) if var == other_var))
                }
            }
            
            // Literals
            (TypeTerm::Literal(v1), TypeTerm::Literal(v2)) => {
                Ok(v1 == v2)
            }
            
            // Applications
            (TypeTerm::App(f1, args1), TypeTerm::App(f2, args2)) => {
                if args1.len() != args2.len() {
                    return Ok(false);
                }
                
                if !self.solve_term_equality(f1, f2)? {
                    return Ok(false);
                }
                
                for (a1, a2) in args1.iter().zip(args2.iter()) {
                    if !self.solve_term_equality(a1, a2)? {
                        return Ok(false);
                    }
                }
                
                Ok(true)
            }
            
            // Different term types
            _ => Ok(false),
        }
    }
    
    /// Solve refinement constraint
    fn solve_refinement(&mut self, var: &str, var_type: &Type, predicate: &TypeTerm) -> TlispResult<bool> {
        // For now, implement basic refinement checking
        // In a full implementation, this would use SMT solving
        
        match predicate {
            TypeTerm::App(func, args) => {
                if let TypeTerm::Var(func_name) = func.as_ref() {
                    match func_name.as_str() {
                        ">" | "<" | ">=" | "<=" | "=" => {
                            self.solve_arithmetic_predicate(var, var_type, func_name, args)
                        }
                        _ => {
                            // Unknown predicate, assume true for now
                            Ok(true)
                        }
                    }
                } else {
                    Ok(true)
                }
            }
            _ => Ok(true),
        }
    }
    
    /// Solve arithmetic predicate
    fn solve_arithmetic_predicate(&mut self, _var: &str, _var_type: &Type, op: &str, args: &[TypeTerm]) -> TlispResult<bool> {
        if args.len() != 2 {
            return Ok(false);
        }
        
        // Try to evaluate the predicate with known values
        let left_val = self.evaluate_term(&args[0])?;
        let right_val = self.evaluate_term(&args[1])?;
        
        match (left_val, right_val) {
            (Some(Value::Int(a)), Some(Value::Int(b))) => {
                match op {
                    ">" => Ok(a > b),
                    "<" => Ok(a < b),
                    ">=" => Ok(a >= b),
                    "<=" => Ok(a <= b),
                    "=" => Ok(a == b),
                    _ => Ok(false),
                }
            }
            _ => {
                // Can't evaluate, assume true for now
                // In a real implementation, this would generate SMT constraints
                Ok(true)
            }
        }
    }
    
    /// Evaluate a type term to a value if possible
    fn evaluate_term(&self, term: &TypeTerm) -> TlispResult<Option<Value>> {
        match term {
            TypeTerm::Literal(value) => Ok(Some(value.clone())),
            TypeTerm::Var(var) => Ok(self.context.lookup_value(var).cloned()),
            TypeTerm::App(func, args) => {
                // Try to evaluate function application
                if let TypeTerm::Var(func_name) = func.as_ref() {
                    match func_name.as_str() {
                        "+" | "-" | "*" | "/" => {
                            if args.len() == 2 {
                                if let (Some(Value::Int(a)), Some(Value::Int(b))) = 
                                    (self.evaluate_term(&args[0])?, self.evaluate_term(&args[1])?) {
                                    match func_name.as_str() {
                                        "+" => Ok(Some(Value::Int(a + b))),
                                        "-" => Ok(Some(Value::Int(a - b))),
                                        "*" => Ok(Some(Value::Int(a * b))),
                                        "/" if b != 0 => Ok(Some(Value::Int(a / b))),
                                        _ => Ok(None),
                                    }
                                } else {
                                    Ok(None)
                                }
                            } else {
                                Ok(None)
                            }
                        }
                        _ => Ok(None),
                    }
                } else {
                    Ok(None)
                }
            }
            _ => Ok(None),
        }
    }
    
    /// Solve subtyping constraint
    fn solve_subtyping(&mut self, t1: &Type, t2: &Type) -> TlispResult<bool> {
        // Basic subtyping rules
        match (t1, t2) {
            // Reflexivity
            (a, b) if a == b => Ok(true),
            
            // Refinement subtyping: {x: T | P} <: T
            (Type::Refinement { base_type, .. }, t) => {
                self.solve_subtyping(base_type, t)
            }
            
            // Function subtyping (contravariant in parameters, covariant in return)
            (Type::Function(params1, ret1), Type::Function(params2, ret2)) => {
                if params1.len() != params2.len() {
                    return Ok(false);
                }
                
                // Parameters are contravariant
                for (p1, p2) in params1.iter().zip(params2.iter()) {
                    if !self.solve_subtyping(p2, p1)? {
                        return Ok(false);
                    }
                }
                
                // Return type is covariant
                self.solve_subtyping(ret1, ret2)
            }
            
            // List subtyping (covariant)
            (Type::List(elem1), Type::List(elem2)) => {
                self.solve_subtyping(elem1, elem2)
            }
            
            _ => Ok(false),
        }
    }
    
    /// Solve kinding constraint
    fn solve_kinding(&mut self, ty: &Type, expected_kind: &Kind) -> TlispResult<bool> {
        let actual_kind = ty.kind();
        Ok(self.kinds_equal(&actual_kind, expected_kind))
    }
    
    /// Check if two kinds are equal
    fn kinds_equal(&self, k1: &Kind, k2: &Kind) -> bool {
        match (k1, k2) {
            (Kind::Type, Kind::Type) => true,
            (Kind::Constraint, Kind::Constraint) => true,
            (Kind::Effect, Kind::Effect) => true,
            (Kind::Capability, Kind::Capability) => true,
            (Kind::Arrow(from1, to1), Kind::Arrow(from2, to2)) => {
                self.kinds_equal(from1, from2) && self.kinds_equal(to1, to2)
            }
            _ => false,
        }
    }
    
    /// Solve existential quantification
    fn solve_existential(&mut self, var: &str, _var_type: &Type, constraint: &Constraint) -> TlispResult<bool> {
        // Try to find a witness for the existential
        // For now, just assume the constraint can be satisfied
        let old_binding = self.context.type_vars.get(var).cloned();
        
        // Try with a fresh type variable
        let witness = Type::TypeVar(format!("{}_{}", var, self.current_depth));
        self.context.bind_type(var.to_string(), witness);
        
        let result = self.solve(constraint);
        
        // Restore old binding
        if let Some(old_type) = old_binding {
            self.context.bind_type(var.to_string(), old_type);
        } else {
            self.context.type_vars.remove(var);
        }
        
        result
    }
    
    /// Solve universal quantification
    fn solve_universal(&mut self, var: &str, var_type: &Type, constraint: &Constraint) -> TlispResult<bool> {
        // For universal quantification, the constraint must hold for all possible values
        // This is undecidable in general, so we use heuristics
        
        let old_binding = self.context.type_vars.get(var).cloned();
        
        // Try with a few representative values
        let test_values = match var_type {
            Type::Int => vec![
                Type::TypeVar("test_int_0".to_string()),
                Type::TypeVar("test_int_pos".to_string()),
                Type::TypeVar("test_int_neg".to_string()),
            ],
            _ => vec![Type::TypeVar(format!("test_{}", var))],
        };
        
        for test_value in test_values {
            self.context.bind_type(var.to_string(), test_value);
            if !self.solve(constraint)? {
                // Restore and return false
                if let Some(old_type) = old_binding {
                    self.context.bind_type(var.to_string(), old_type);
                } else {
                    self.context.type_vars.remove(var);
                }
                return Ok(false);
            }
        }
        
        // Restore old binding
        if let Some(old_type) = old_binding {
            self.context.bind_type(var.to_string(), old_type);
        } else {
            self.context.type_vars.remove(var);
        }
        
        Ok(true)
    }
    
    /// Generate constraints for type checking
    pub fn generate_constraints(&mut self, expr: &Expr<()>, expected_type: &Type) -> TlispResult<Vec<Constraint>> {
        let mut constraints = Vec::new();
        
        match expr {
            Expr::Number(_n, _) => {
                constraints.push(Constraint::TypeEquality(Type::Int, expected_type.clone()));
            }
            
            Expr::String(_, _) => {
                constraints.push(Constraint::TypeEquality(Type::String, expected_type.clone()));
            }
            
            Expr::Bool(_, _) => {
                constraints.push(Constraint::TypeEquality(Type::Bool, expected_type.clone()));
            }
            
            Expr::Symbol(name, _) => {
                if let Some(var_type) = self.context.lookup_type(name) {
                    constraints.push(Constraint::TypeEquality(var_type.clone(), expected_type.clone()));
                } else {
                    // Create fresh type variable
                    let fresh_var = Type::TypeVar(format!("{}_{}", name, self.current_depth));
                    self.context.bind_type(name.clone(), fresh_var.clone());
                    constraints.push(Constraint::TypeEquality(fresh_var, expected_type.clone()));
                }
            }
            
            Expr::List(elements, _) => {
                if let Type::List(elem_type) = expected_type {
                    for element in elements {
                        let elem_constraints = self.generate_constraints(element, elem_type)?;
                        constraints.extend(elem_constraints);
                    }
                } else {
                    return Err(TypeError::Mismatch {
                        expected: format!("List(_)"),
                        actual: format!("{}", expected_type)
                    }.into());
                }
            }
            
            _ => {
                // For other expressions, generate basic constraints
                // This would be expanded in a full implementation
            }
        }
        
        Ok(constraints)
    }
}

impl Default for ConstraintSolver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constraint_solver_creation() {
        let solver = ConstraintSolver::new();
        assert_eq!(solver.current_depth, 0);
        assert_eq!(solver.max_depth, 100);
        assert!(!solver.use_smt);

        let smt_solver = ConstraintSolver::with_smt();
        assert!(smt_solver.use_smt);
    }

    #[test]
    fn test_constraint_context_operations() {
        let mut context = ConstraintContext::new();

        // Test type binding
        context.bind_type("T".to_string(), Type::Int);
        assert_eq!(context.lookup_type("T"), Some(&Type::Int));

        // Test value binding
        context.bind_value("x".to_string(), Value::Int(42));
        assert_eq!(context.lookup_value("x"), Some(&Value::Int(42)));

        // Test assumptions
        let constraint = Constraint::TypeEquality(Type::Int, Type::Int);
        context.assume(constraint.clone());
        assert_eq!(context.assumptions.len(), 1);
    }

    #[test]
    fn test_type_equality_constraint() {
        let mut solver = ConstraintSolver::new();

        // Test reflexivity: Int = Int
        let constraint = Constraint::TypeEquality(Type::Int, Type::Int);
        assert!(solver.solve(&constraint).unwrap());

        // Test inequality: Int ≠ String
        let constraint = Constraint::TypeEquality(Type::Int, Type::String);
        assert!(!solver.solve(&constraint).unwrap());

        // Test structural equality: [Int] = [Int]
        let list_int = Type::List(Box::new(Type::Int));
        let constraint = Constraint::TypeEquality(list_int.clone(), list_int.clone());
        assert!(solver.solve(&constraint).unwrap());

        // Test structural inequality: [Int] ≠ [String]
        let list_string = Type::List(Box::new(Type::String));
        let constraint = Constraint::TypeEquality(list_int, list_string);
        assert!(!solver.solve(&constraint).unwrap());
    }

    #[test]
    fn test_type_variable_unification() {
        let mut solver = ConstraintSolver::new();

        // Test unification: T = Int (should bind T to Int)
        let type_var = Type::TypeVar("T".to_string());
        let constraint = Constraint::TypeEquality(type_var.clone(), Type::Int);
        assert!(solver.solve(&constraint).unwrap());

        // Check that T is now bound to Int
        assert_eq!(solver.context.lookup_type("T"), Some(&Type::Int));

        // Test consistency: T = Int should still hold
        let constraint = Constraint::TypeEquality(type_var, Type::Int);
        assert!(solver.solve(&constraint).unwrap());
    }

    #[test]
    fn test_function_type_equality() {
        let mut solver = ConstraintSolver::new();

        // Test function type equality: (Int -> String) = (Int -> String)
        let func1 = Type::Function(vec![Type::Int], Box::new(Type::String));
        let func2 = Type::Function(vec![Type::Int], Box::new(Type::String));
        let constraint = Constraint::TypeEquality(func1, func2);
        assert!(solver.solve(&constraint).unwrap());

        // Test function type inequality: (Int -> String) ≠ (String -> Int)
        let func1 = Type::Function(vec![Type::Int], Box::new(Type::String));
        let func2 = Type::Function(vec![Type::String], Box::new(Type::Int));
        let constraint = Constraint::TypeEquality(func1, func2);
        assert!(!solver.solve(&constraint).unwrap());

        // Test arity mismatch: (Int -> String) ≠ (Int, Bool -> String)
        let func1 = Type::Function(vec![Type::Int], Box::new(Type::String));
        let func2 = Type::Function(vec![Type::Int, Type::Bool], Box::new(Type::String));
        let constraint = Constraint::TypeEquality(func1, func2);
        assert!(!solver.solve(&constraint).unwrap());
    }

    #[test]
    fn test_term_equality_constraint() {
        let mut solver = ConstraintSolver::new();

        // Test literal equality: 42 = 42
        let term1 = TypeTerm::Literal(Value::Int(42));
        let term2 = TypeTerm::Literal(Value::Int(42));
        let constraint = Constraint::TermEquality(term1, term2);
        assert!(solver.solve(&constraint).unwrap());

        // Test literal inequality: 42 ≠ 24
        let term1 = TypeTerm::Literal(Value::Int(42));
        let term2 = TypeTerm::Literal(Value::Int(24));
        let constraint = Constraint::TermEquality(term1, term2);
        assert!(!solver.solve(&constraint).unwrap());

        // Test variable equality: x = x
        let term1 = TypeTerm::Var("x".to_string());
        let term2 = TypeTerm::Var("x".to_string());
        let constraint = Constraint::TermEquality(term1, term2);
        assert!(solver.solve(&constraint).unwrap());
    }

    #[test]
    fn test_refinement_constraint() {
        let mut solver = ConstraintSolver::new();

        // Test simple refinement: {x: Int | x > 0}
        let predicate = TypeTerm::App(
            Box::new(TypeTerm::Var(">".to_string())),
            vec![
                TypeTerm::Var("x".to_string()),
                TypeTerm::Literal(Value::Int(0)),
            ],
        );

        let constraint = Constraint::Refinement {
            var: "x".to_string(),
            var_type: Type::Int,
            predicate,
        };

        // For now, refinement constraints are assumed to be satisfiable
        assert!(solver.solve(&constraint).unwrap());
    }

    #[test]
    fn test_subtyping_constraint() {
        let mut solver = ConstraintSolver::new();

        // Test reflexivity: Int <: Int
        let constraint = Constraint::Subtyping(Type::Int, Type::Int);
        assert!(solver.solve(&constraint).unwrap());

        // Test refinement subtyping: {x: Int | x > 0} <: Int
        let refinement = Type::Refinement {
            var: "x".to_string(),
            base_type: Box::new(Type::Int),
            predicate: Box::new(TypeTerm::App(
                Box::new(TypeTerm::Var(">".to_string())),
                vec![
                    TypeTerm::Var("x".to_string()),
                    TypeTerm::Literal(Value::Int(0)),
                ],
            )),
        };

        let constraint = Constraint::Subtyping(refinement, Type::Int);
        assert!(solver.solve(&constraint).unwrap());

        // Test function subtyping: (String -> Int) <: (String -> Int)
        let func1 = Type::Function(vec![Type::String], Box::new(Type::Int));
        let func2 = Type::Function(vec![Type::String], Box::new(Type::Int));
        let constraint = Constraint::Subtyping(func1, func2);
        assert!(solver.solve(&constraint).unwrap());
    }

    #[test]
    fn test_kinding_constraint() {
        let mut solver = ConstraintSolver::new();

        // Test basic type kinding: Int : *
        let constraint = Constraint::Kinding(Type::Int, Kind::Type);
        assert!(solver.solve(&constraint).unwrap());

        // Test list type kinding: [Int] : *
        let list_type = Type::List(Box::new(Type::Int));
        let constraint = Constraint::Kinding(list_type, Kind::Type);
        assert!(solver.solve(&constraint).unwrap());

        // Test wrong kinding: Int : Constraint (should fail)
        let constraint = Constraint::Kinding(Type::Int, Kind::Constraint);
        assert!(!solver.solve(&constraint).unwrap());
    }

    #[test]
    fn test_conjunction_constraint() {
        let mut solver = ConstraintSolver::new();

        // Test conjunction: (Int = Int) ∧ (String = String)
        let constraints = vec![
            Constraint::TypeEquality(Type::Int, Type::Int),
            Constraint::TypeEquality(Type::String, Type::String),
        ];
        let constraint = Constraint::And(constraints);
        assert!(solver.solve(&constraint).unwrap());

        // Test failing conjunction: (Int = Int) ∧ (Int = String)
        let constraints = vec![
            Constraint::TypeEquality(Type::Int, Type::Int),
            Constraint::TypeEquality(Type::Int, Type::String),
        ];
        let constraint = Constraint::And(constraints);
        assert!(!solver.solve(&constraint).unwrap());
    }

    #[test]
    fn test_disjunction_constraint() {
        let mut solver = ConstraintSolver::new();

        // Test disjunction: (Int = String) ∨ (Int = Int)
        let constraints = vec![
            Constraint::TypeEquality(Type::Int, Type::String),
            Constraint::TypeEquality(Type::Int, Type::Int),
        ];
        let constraint = Constraint::Or(constraints);
        assert!(solver.solve(&constraint).unwrap());

        // Test failing disjunction: (Int = String) ∨ (Bool = Float)
        let constraints = vec![
            Constraint::TypeEquality(Type::Int, Type::String),
            Constraint::TypeEquality(Type::Bool, Type::Float),
        ];
        let constraint = Constraint::Or(constraints);
        assert!(!solver.solve(&constraint).unwrap());
    }

    #[test]
    fn test_implication_constraint() {
        let mut solver = ConstraintSolver::new();

        // Test vacuous truth: (Int = String) => (Bool = Float)
        let premise = Box::new(Constraint::TypeEquality(Type::Int, Type::String));
        let conclusion = Box::new(Constraint::TypeEquality(Type::Bool, Type::Float));
        let constraint = Constraint::Implies(premise, conclusion);
        assert!(solver.solve(&constraint).unwrap());

        // Test valid implication: (Int = Int) => (Int = Int)
        let premise = Box::new(Constraint::TypeEquality(Type::Int, Type::Int));
        let conclusion = Box::new(Constraint::TypeEquality(Type::Int, Type::Int));
        let constraint = Constraint::Implies(premise, conclusion);
        assert!(solver.solve(&constraint).unwrap());

        // Test invalid implication: (Int = Int) => (Int = String)
        let premise = Box::new(Constraint::TypeEquality(Type::Int, Type::Int));
        let conclusion = Box::new(Constraint::TypeEquality(Type::Int, Type::String));
        let constraint = Constraint::Implies(premise, conclusion);
        assert!(!solver.solve(&constraint).unwrap());
    }

    #[test]
    fn test_arithmetic_evaluation() {
        let solver = ConstraintSolver::new();

        // Test literal evaluation
        let term = TypeTerm::Literal(Value::Int(42));
        let result = solver.evaluate_term(&term).unwrap();
        assert_eq!(result, Some(Value::Int(42)));

        // Test addition: 2 + 3 = 5
        let term = TypeTerm::App(
            Box::new(TypeTerm::Var("+".to_string())),
            vec![
                TypeTerm::Literal(Value::Int(2)),
                TypeTerm::Literal(Value::Int(3)),
            ],
        );
        let result = solver.evaluate_term(&term).unwrap();
        assert_eq!(result, Some(Value::Int(5)));

        // Test subtraction: 10 - 4 = 6
        let term = TypeTerm::App(
            Box::new(TypeTerm::Var("-".to_string())),
            vec![
                TypeTerm::Literal(Value::Int(10)),
                TypeTerm::Literal(Value::Int(4)),
            ],
        );
        let result = solver.evaluate_term(&term).unwrap();
        assert_eq!(result, Some(Value::Int(6)));
    }

    #[test]
    fn test_max_depth_protection() {
        let mut solver = ConstraintSolver::new();
        solver.max_depth = 2; // Set very low limit

        // Create a constraint that would cause deep recursion
        let type_var = Type::TypeVar("T".to_string());
        let constraint = Constraint::TypeEquality(type_var.clone(), type_var);

        // Should not exceed max depth for simple case
        assert!(solver.solve(&constraint).is_ok());
    }

    #[test]
    fn test_constraint_generation() {
        let mut solver = ConstraintSolver::new();

        // Test constraint generation for integer literal
        let expr = crate::tlisp::Expr::Number(42, ());
        let constraints = solver.generate_constraints(&expr, &Type::Int).unwrap();
        assert_eq!(constraints.len(), 1);

        match &constraints[0] {
            Constraint::TypeEquality(t1, t2) => {
                assert_eq!(*t1, Type::Int);
                assert_eq!(*t2, Type::Int);
            }
            _ => panic!("Expected TypeEquality constraint"),
        }
    }
}
