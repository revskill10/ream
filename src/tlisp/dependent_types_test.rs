//! Unit tests for dependent type system

use super::types::*;
use super::{Value, Expr};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_type_creation() {
        // Test basic type creation
        let int_type = Type::Int;
        assert_eq!(int_type.kind(), Kind::Type);
        assert!(!int_type.is_dependent());
        assert!(!int_type.is_var());
        
        let string_type = Type::String;
        assert_eq!(string_type.kind(), Kind::Type);
        
        let type_var = Type::TypeVar("a".to_string());
        assert!(type_var.is_var());
        assert_eq!(type_var.as_var(), Some("a"));
    }

    #[test]
    fn test_dependent_function_type() {
        // Test dependent function type: (n: Int) -> Vec(n, String)
        let vec_type = Type::TypeApp {
            constructor: Box::new(Type::TypeVar("Vec".to_string())),
            args: vec![
                TypeTerm::Var("n".to_string()),
                TypeTerm::Literal(Value::String("String".to_string())),
            ],
        };
        
        let dep_func = Type::DepFunction {
            param_name: "n".to_string(),
            param_type: Box::new(Type::Int),
            return_type: Box::new(vec_type),
        };
        
        assert!(dep_func.is_dependent());
        assert_eq!(dep_func.kind(), Kind::Type);
        
        // Test display
        let display_str = format!("{}", dep_func);
        assert!(display_str.contains("n: Int"));
    }

    #[test]
    fn test_type_lambda() {
        // Test type lambda: λ(T: *) -> List(T)
        let type_lambda = Type::TypeLambda {
            param: "T".to_string(),
            param_kind: Kind::Type,
            body: Box::new(Type::List(Box::new(Type::TypeVar("T".to_string())))),
        };
        
        assert!(type_lambda.is_dependent());
        
        // Test kind
        let expected_kind = Kind::Arrow(Box::new(Kind::Type), Box::new(Kind::Type));
        assert_eq!(type_lambda.kind(), expected_kind);
    }

    #[test]
    fn test_refinement_type() {
        // Test refinement type: {x: Int | x > 0}
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
        
        // Test display
        let display_str = format!("{}", refinement);
        assert!(display_str.contains("x: Int"));
    }

    #[test]
    fn test_type_term_operations() {
        // Test type term creation and operations
        let var_term = TypeTerm::Var("x".to_string());
        let literal_term = TypeTerm::Literal(Value::Int(42));
        
        // Test free variables
        assert_eq!(var_term.free_vars(), vec!["x".to_string()]);
        assert_eq!(literal_term.free_vars(), Vec::<String>::new());
        
        // Test substitution
        let replacement = TypeTerm::Literal(Value::Int(10));
        let substituted = var_term.substitute("x", &replacement);
        assert_eq!(substituted, replacement);
        
        // Test that substitution doesn't affect other variables
        let other_var = TypeTerm::Var("y".to_string());
        let not_substituted = other_var.substitute("x", &replacement);
        assert_eq!(not_substituted, other_var);
    }

    #[test]
    fn test_type_application() {
        // Test type application: Vec(3, Int)
        let type_app = Type::TypeApp {
            constructor: Box::new(Type::TypeVar("Vec".to_string())),
            args: vec![
                TypeTerm::Literal(Value::Int(3)),
                TypeTerm::Literal(Value::String("Int".to_string())),
            ],
        };
        
        assert!(type_app.is_dependent());
        
        // Test display
        let display_str = format!("{}", type_app);
        assert!(display_str.contains("Vec"));
    }

    #[test]
    fn test_session_types() {
        // Test session type: !Int.?String.end
        let session = SessionType::Send(
            Box::new(Type::Int),
            Box::new(SessionType::Receive(
                Box::new(Type::String),
                Box::new(SessionType::End),
            )),
        );
        
        let session_type = Type::Session(Box::new(session));
        assert!(!session_type.is_dependent());
        assert!(session_type.is_session());
        
        // Test session operations
        if let Type::Session(ref s) = session_type {
            assert!(!s.is_complete());
            assert_eq!(s.next_message_type(), Some(&Type::Int));
        }
    }

    #[test]
    fn test_capability_types() {
        // Test capability types
        let read_cap = CapabilityType::Read("file.txt".to_string());
        let write_cap = CapabilityType::Write("file.txt".to_string());
        let send_cap = CapabilityType::Send(Box::new(Type::String));
        
        let union_cap = CapabilityType::Union(vec![read_cap, write_cap, send_cap]);
        let cap_type = Type::Capability(Box::new(union_cap));
        
        assert!(cap_type.is_capability());
        assert_eq!(cap_type.kind(), Kind::Capability);
    }

    #[test]
    fn test_effect_types() {
        // Test effect types
        let io_effect = EffectType::IO;
        let state_effect = EffectType::State;
        let union_effect = EffectType::Union(vec![io_effect, state_effect]);
        
        let effect_type = Type::Effect(Box::new(union_effect));
        
        assert!(effect_type.is_effect());
        assert_eq!(effect_type.kind(), Kind::Effect);
    }

    #[test]
    fn test_equality_types() {
        // Test equality type: x = y
        let equality = Type::Equality(
            Box::new(TypeTerm::Var("x".to_string())),
            Box::new(TypeTerm::Var("y".to_string())),
        );
        
        assert!(equality.is_dependent());
        assert_eq!(equality.kind(), Kind::Constraint);
    }

    #[test]
    fn test_occurs_check_with_dependent_types() {
        // Test occurs check with dependent types
        let type_var = Type::TypeVar("a".to_string());
        
        let dep_func = Type::DepFunction {
            param_name: "x".to_string(),
            param_type: Box::new(Type::TypeVar("a".to_string())),
            return_type: Box::new(Type::Int),
        };
        
        assert!(dep_func.occurs_in(&type_var));
        
        let type_lambda = Type::TypeLambda {
            param: "T".to_string(),
            param_kind: Kind::Type,
            body: Box::new(Type::TypeVar("a".to_string())),
        };
        
        assert!(type_lambda.occurs_in(&type_var));
    }

    #[test]
    fn test_free_variables_in_dependent_types() {
        // Test free variable collection in dependent types
        let dep_func = Type::DepFunction {
            param_name: "x".to_string(),
            param_type: Box::new(Type::TypeVar("A".to_string())),
            return_type: Box::new(Type::TypeVar("B".to_string())),
        };
        
        let free_vars = dep_func.free_vars();
        assert!(free_vars.contains(&"A".to_string()));
        assert!(free_vars.contains(&"B".to_string()));
        
        let refinement = Type::Refinement {
            var: "x".to_string(),
            base_type: Box::new(Type::TypeVar("T".to_string())),
            predicate: Box::new(TypeTerm::Var("P".to_string())),
        };
        
        let refinement_vars = refinement.free_vars();
        assert!(refinement_vars.contains(&"T".to_string()));
    }

    #[test]
    fn test_kind_system() {
        // Test kind system
        let type_kind = Kind::Type;
        let arrow_kind = Kind::Arrow(Box::new(Kind::Type), Box::new(Kind::Type));
        let dep_arrow_kind = Kind::DepArrow(
            "x".to_string(),
            Box::new(Type::Int),
            Box::new(Kind::Type),
        );
        
        // Test display
        assert_eq!(format!("{}", type_kind), "*");
        assert_eq!(format!("{}", arrow_kind), "* -> *");
        assert!(format!("{}", dep_arrow_kind).contains("x: Int"));
    }

    #[test]
    fn test_type_term_lambda() {
        // Test type term lambda: λ(x: Int) x + 1
        let lambda_term = TypeTerm::Lambda(
            "x".to_string(),
            Box::new(Type::Int),
            Box::new(TypeTerm::App(
                Box::new(TypeTerm::Var("+".to_string())),
                vec![
                    TypeTerm::Var("x".to_string()),
                    TypeTerm::Literal(Value::Int(1)),
                ],
            )),
        );
        
        // Test free variables (should not include bound variable)
        let free_vars = lambda_term.free_vars();
        assert!(!free_vars.contains(&"x".to_string()));
        assert!(free_vars.contains(&"+".to_string()));
        
        // Test substitution (should not substitute bound variable)
        let replacement = TypeTerm::Literal(Value::Int(42));
        let substituted = lambda_term.substitute("x", &replacement);
        assert_eq!(substituted, lambda_term); // Should be unchanged
    }

    #[test]
    fn test_complex_dependent_type() {
        // Test complex dependent type: (n: Nat) -> (m: Nat) -> Matrix(n, m, Float)
        let matrix_type = Type::TypeApp {
            constructor: Box::new(Type::TypeVar("Matrix".to_string())),
            args: vec![
                TypeTerm::Var("n".to_string()),
                TypeTerm::Var("m".to_string()),
                TypeTerm::Literal(Value::String("Float".to_string())),
            ],
        };
        
        let inner_dep_func = Type::DepFunction {
            param_name: "m".to_string(),
            param_type: Box::new(Type::TypeVar("Nat".to_string())),
            return_type: Box::new(matrix_type),
        };
        
        let outer_dep_func = Type::DepFunction {
            param_name: "n".to_string(),
            param_type: Box::new(Type::TypeVar("Nat".to_string())),
            return_type: Box::new(inner_dep_func),
        };
        
        assert!(outer_dep_func.is_dependent());
        
        // Test that it contains the expected variables
        let free_vars = outer_dep_func.free_vars();
        assert!(free_vars.contains(&"Nat".to_string()));
        assert!(free_vars.contains(&"Matrix".to_string()));
    }
}
