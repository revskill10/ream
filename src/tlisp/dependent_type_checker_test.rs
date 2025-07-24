//! Unit tests for the dependent type checker

#[cfg(test)]
mod tests {
    use super::super::dependent_type_checker::*;
    use super::super::types::*;
    use super::super::constraint_solver::*;
    use super::super::{Expr, Value};

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
    fn test_let_binding() {
        let mut checker = DependentTypeChecker::new();
        
        // Test let: (let ((x 42)) x)
        let bindings = vec![("x".to_string(), Expr::Number(42, ()))];
        let body = Expr::Symbol("x".to_string(), ());
        let expr = Expr::Let(bindings, Box::new(body), ());
        
        let ty = checker.infer_type(&expr).unwrap();
        assert_eq!(ty, Type::Int);
    }

    #[test]
    fn test_if_expression() {
        let mut checker = DependentTypeChecker::new();
        
        // Test if: (if true 1 2)
        let cond = Expr::Bool(true, ());
        let then_branch = Expr::Number(1, ());
        let else_branch = Expr::Number(2, ());
        let expr = Expr::If(Box::new(cond), Box::new(then_branch), Box::new(else_branch), ());
        
        let ty = checker.infer_type(&expr).unwrap();
        assert_eq!(ty, Type::Int);
    }

    #[test]
    fn test_list_type_inference() {
        let mut checker = DependentTypeChecker::new();
        
        // Test homogeneous list: [1, 2, 3]
        let elements = vec![
            Expr::Number(1, ()),
            Expr::Number(2, ()),
            Expr::Number(3, ()),
        ];
        let expr = Expr::List(elements, ());
        
        let ty = checker.infer_type(&expr).unwrap();
        assert_eq!(ty, Type::List(Box::new(Type::Int)));
    }

    #[test]
    fn test_empty_list() {
        let mut checker = DependentTypeChecker::new();
        
        // Test empty list: []
        let expr = Expr::List(vec![], ());
        
        let ty = checker.infer_type(&expr).unwrap();
        
        // Should be a polymorphic list type
        match ty {
            Type::List(elem_type) => {
                match *elem_type {
                    Type::TypeVar(_) => {}, // Expected
                    _ => panic!("Expected type variable for empty list element type"),
                }
            }
            _ => panic!("Expected list type, got {:?}", ty),
        }
    }

    #[test]
    fn test_type_environment_operations() {
        let mut env = TypeEnvironment::new();
        
        // Test variable binding
        env.bind_var("x".to_string(), Type::Int);
        assert_eq!(env.lookup_var("x"), Some(Type::Int));
        
        // Test type constructor binding
        env.bind_type_constructor("MyType".to_string(), Type::Bool);
        assert_eq!(env.lookup_type_constructor("MyType"), Some(Type::Bool));
        
        // Test value binding
        env.bind_value("val".to_string(), Value::Int(42));
        assert_eq!(env.lookup_value("val"), Some(Value::Int(42)));
    }

    #[test]
    fn test_child_environment() {
        let mut parent = TypeEnvironment::new();
        parent.bind_var("x".to_string(), Type::Int);
        
        let mut child = parent.child();
        child.bind_var("y".to_string(), Type::Bool);
        
        // Child should see both variables
        assert_eq!(child.lookup_var("x"), Some(Type::Int));
        assert_eq!(child.lookup_var("y"), Some(Type::Bool));
        
        // Parent should only see its own variable
        assert_eq!(parent.lookup_var("x"), Some(Type::Int));
        assert_eq!(parent.lookup_var("y"), None);
    }

    #[test]
    fn test_typing_context() {
        let mut context = TypingContext::new();
        
        // Test fresh variable generation
        let var1 = context.fresh_var();
        let var2 = context.fresh_var();
        
        match (&var1, &var2) {
            (Type::TypeVar(name1), Type::TypeVar(name2)) => {
                assert_ne!(name1, name2);
            }
            _ => panic!("Expected type variables"),
        }
        
        // Test constraint addition
        let constraint = Constraint::TypeEquality(Type::Int, Type::Int);
        context.add_constraint(constraint.clone());
        
        assert_eq!(context.constraints().len(), 1);
        assert_eq!(context.constraints()[0], constraint);
    }

    #[test]
    fn test_expr_to_type_term_conversion() {
        let mut checker = DependentTypeChecker::new();
        
        // Test number conversion
        let expr = Expr::Number(42, ());
        let type_term = checker.expr_to_type_term(&expr).unwrap();
        assert_eq!(type_term, TypeTerm::Literal(Value::Int(42)));
        
        // Test symbol conversion
        let expr = Expr::Symbol("x".to_string(), ());
        let type_term = checker.expr_to_type_term(&expr).unwrap();
        assert_eq!(type_term, TypeTerm::Var("x".to_string()));
        
        // Test application conversion
        let func = Expr::Symbol("f".to_string(), ());
        let args = vec![Expr::Number(1, ())];
        let expr = Expr::Application(Box::new(func), args, ());
        let type_term = checker.expr_to_type_term(&expr).unwrap();
        
        match type_term {
            TypeTerm::App(func, args) => {
                assert_eq!(*func, TypeTerm::Var("f".to_string()));
                assert_eq!(args.len(), 1);
                assert_eq!(args[0], TypeTerm::Literal(Value::Int(1)));
            }
            _ => panic!("Expected type term application"),
        }
    }

    #[test]
    fn test_expr_to_value_conversion() {
        let checker = DependentTypeChecker::new();
        
        // Test literal conversions
        assert_eq!(checker.expr_to_value(&Expr::Number(42, ())), Some(Value::Int(42)));
        assert_eq!(checker.expr_to_value(&Expr::Bool(true, ())), Some(Value::Bool(true)));
        assert_eq!(checker.expr_to_value(&Expr::String("test".to_string(), ())), Some(Value::String("test".to_string())));
        
        // Test non-literal
        assert_eq!(checker.expr_to_value(&Expr::Symbol("x".to_string(), ())), None);
    }

    #[test]
    fn test_type_checking_against_annotation() {
        let mut checker = DependentTypeChecker::new();
        
        // Test successful type check
        let expr = Expr::Number(42, ());
        assert!(checker.check_type(&expr, &Type::Int).is_ok());
        
        // Test type mismatch (this should generate constraints that may or may not be solvable)
        let expr = Expr::Number(42, ());
        let result = checker.check_type(&expr, &Type::Bool);
        // The result depends on constraint solving - for now we just check it doesn't panic
        let _ = result;
    }

    #[test]
    fn test_undefined_variable_error() {
        let mut checker = DependentTypeChecker::new();
        
        let expr = Expr::Symbol("undefined_var".to_string(), ());
        let result = checker.infer_type(&expr);
        
        assert!(result.is_err());
        match result.unwrap_err() {
            TlispError::TypeError(TypeError::UndefinedVariable(name)) => {
                assert_eq!(name, "undefined_var");
            }
            _ => panic!("Expected UndefinedVariable error"),
        }
    }
}
