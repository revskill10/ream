//! Tests for TLisp examples to ensure they work correctly

#[cfg(test)]
mod tests {
    use crate::tlisp::{Evaluator, Value, Expr, types::Type, environment::Environment, standard_library::StandardLibrary};
    use std::rc::Rc;
    use std::cell::RefCell;

    fn create_full_test_evaluator() -> Evaluator {
        let env = Rc::new(RefCell::new(Environment::new()));

        // Add standard library functions
        let stdlib = StandardLibrary::new();
        for (name, function) in stdlib.functions() {
            env.borrow_mut().define(name.clone(), function.clone());
        }

        // Add additional module functions
        let module_functions = vec![
            // Module functions
            "import",
            "http-server:start", "http-server:stop", "http-server:get", "http-server:post", "http-server:send-response",
            "json:parse", "json:stringify", "json:get", "json:set!", "json:object",
            "async-utils:now", "async-utils:timestamp-ms", "async-utils:format-time", "async-utils:sleep", "async-utils:spawn-task",

            // Actor functions
            "spawn", "send", "receive", "self",
        ];

        for builtin in module_functions {
            env.borrow_mut().define(builtin.to_string(), Value::Builtin(builtin.to_string()));
        }

        Evaluator::new(env)
    }

    #[test]
    fn test_basic_tlisp_functionality() {
        let mut evaluator = create_full_test_evaluator();
        
        // Test basic arithmetic
        let expr = Expr::Application(
            Box::new(Expr::Symbol("+".to_string(), Type::Function(vec![Type::Int, Type::Int], Box::new(Type::Int)))),
            vec![
                Expr::Number(2, Type::Int),
                Expr::Number(3, Type::Int),
            ],
            Type::Int,
        );
        
        let result = evaluator.eval(&expr).unwrap();
        assert_eq!(result, Value::Int(5));
    }

    #[test]
    fn test_module_imports() {
        let mut evaluator = create_full_test_evaluator();
        
        // Test importing modules
        let import_expr = Expr::Application(
            Box::new(Expr::Symbol("import".to_string(), Type::Function(vec![Type::Symbol], Box::new(Type::Symbol)))),
            vec![Expr::Symbol("http-server".to_string(), Type::Symbol)],
            Type::Symbol,
        );
        
        let result = evaluator.eval(&import_expr);
        assert!(result.is_ok());
    }

    #[test]
    fn test_json_functionality() {
        let mut evaluator = create_full_test_evaluator();
        
        // Test JSON object creation
        let json_object_expr = Expr::Application(
            Box::new(Expr::Symbol("json:object".to_string(), Type::Function(vec![], Box::new(Type::String)))),
            vec![
                Expr::String("name".to_string(), Type::String),
                Expr::String("test".to_string(), Type::String),
                Expr::String("value".to_string(), Type::String),
                Expr::Number(42, Type::Int),
            ],
            Type::String,
        );
        
        let result = evaluator.eval(&json_object_expr);
        assert!(result.is_ok());
        
        if let Ok(Value::String(json_str)) = result {
            assert!(json_str.contains("name"));
            assert!(json_str.contains("test"));
            assert!(json_str.contains("value"));
            assert!(json_str.contains("42"));
        }
    }

    #[test]
    fn test_http_functionality() {
        let mut evaluator = create_full_test_evaluator();
        
        // Test HTTP server start
        let http_start_expr = Expr::Application(
            Box::new(Expr::Symbol("http-server:start".to_string(), Type::Function(vec![Type::Int], Box::new(Type::List(Box::new(Type::TypeVar("ServerResponse".to_string()))))))),
            vec![Expr::Number(8080, Type::Int)],
            Type::List(Box::new(Type::TypeVar("ServerResponse".to_string()))),
        );
        
        let result = evaluator.eval(&http_start_expr);
        assert!(result.is_ok());
        
        if let Ok(Value::List(response)) = result {
            assert_eq!(response.len(), 2);
            assert_eq!(response[0], Value::Symbol("server-started".to_string()));
            assert_eq!(response[1], Value::Int(8080));
        }
    }

    #[test]
    fn test_async_utils_functionality() {
        let mut evaluator = create_full_test_evaluator();
        
        // Test timestamp function
        let timestamp_expr = Expr::Application(
            Box::new(Expr::Symbol("async-utils:now".to_string(), Type::Function(vec![], Box::new(Type::Int)))),
            vec![],
            Type::Int,
        );
        
        let result = evaluator.eval(&timestamp_expr);
        assert!(result.is_ok());
        
        if let Ok(Value::Int(timestamp)) = result {
            assert!(timestamp > 0);
        }
    }

    #[test]
    fn test_actor_functionality() {
        let mut evaluator = create_full_test_evaluator();
        
        // Test spawn function
        let spawn_expr = Expr::Application(
            Box::new(Expr::Symbol("spawn".to_string(), Type::Function(vec![Type::Symbol], Box::new(Type::Pid)))),
            vec![Expr::Symbol("test-actor".to_string(), Type::Symbol)],
            Type::Pid,
        );
        
        let result = evaluator.eval(&spawn_expr);
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), Value::Pid(_)));
    }

    #[test]
    fn test_string_functions() {
        let mut evaluator = create_full_test_evaluator();
        
        // Test string-append
        let string_append_expr = Expr::Application(
            Box::new(Expr::Symbol("string-append".to_string(), Type::Function(vec![Type::String, Type::String], Box::new(Type::String)))),
            vec![
                Expr::String("hello".to_string(), Type::String),
                Expr::String(" world".to_string(), Type::String),
            ],
            Type::String,
        );
        
        let result = evaluator.eval(&string_append_expr);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Value::String("hello world".to_string()));
    }

    #[test]
    fn test_list_functions() {
        let mut evaluator = create_full_test_evaluator();
        
        // Test list-ref
        let list_ref_expr = Expr::Application(
            Box::new(Expr::Symbol("list-ref".to_string(), Type::Function(vec![Type::List(Box::new(Type::String)), Type::Int], Box::new(Type::String)))),
            vec![
                Expr::List(vec![
                    Expr::String("first".to_string(), Type::String),
                    Expr::String("second".to_string(), Type::String),
                    Expr::String("third".to_string(), Type::String),
                ], Type::List(Box::new(Type::String))),
                Expr::Number(1, Type::Int),
            ],
            Type::String,
        );
        
        let result = evaluator.eval(&list_ref_expr);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Value::String("second".to_string()));
    }

    #[test]
    fn test_complex_expression() {
        let mut evaluator = create_full_test_evaluator();
        
        // Test a more complex expression that combines multiple features
        // (+ (string-length "hello") (length (list 1 2 3)))
        let complex_expr = Expr::Application(
            Box::new(Expr::Symbol("+".to_string(), Type::Function(vec![Type::Int, Type::Int], Box::new(Type::Int)))),
            vec![
                Expr::Application(
                    Box::new(Expr::Symbol("string-length".to_string(), Type::Function(vec![Type::String], Box::new(Type::Int)))),
                    vec![Expr::String("hello".to_string(), Type::String)],
                    Type::Int,
                ),
                Expr::Application(
                    Box::new(Expr::Symbol("length".to_string(), Type::Function(vec![Type::List(Box::new(Type::Int))], Box::new(Type::Int)))),
                    vec![
                        Expr::List(vec![
                            Expr::Number(1, Type::Int),
                            Expr::Number(2, Type::Int),
                            Expr::Number(3, Type::Int),
                        ], Type::List(Box::new(Type::Int))),
                    ],
                    Type::Int,
                ),
            ],
            Type::Int,
        );
        
        let result = evaluator.eval(&complex_expr);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Value::Int(8)); // 5 + 3 = 8
    }

    #[test]
    fn test_error_handling() {
        let mut evaluator = create_full_test_evaluator();
        
        // Test error handling with invalid function call
        let invalid_expr = Expr::Application(
            Box::new(Expr::Symbol("non-existent-function".to_string(), Type::Function(vec![], Box::new(Type::Int)))),
            vec![],
            Type::Int,
        );
        
        let result = evaluator.eval(&invalid_expr);
        assert!(result.is_err());
    }

    #[test]
    fn test_integration_scenario() {
        let mut evaluator = create_full_test_evaluator();
        
        // Test a realistic integration scenario:
        // 1. Create JSON data
        // 2. Start HTTP server
        // 3. Get timestamp
        
        // Step 1: Create JSON data
        let json_expr = Expr::Application(
            Box::new(Expr::Symbol("json:object".to_string(), Type::Function(vec![], Box::new(Type::String)))),
            vec![
                Expr::String("message".to_string(), Type::String),
                Expr::String("Hello from TLisp!".to_string(), Type::String),
            ],
            Type::String,
        );
        
        let json_result = evaluator.eval(&json_expr);
        assert!(json_result.is_ok());
        
        // Step 2: Start HTTP server
        let http_expr = Expr::Application(
            Box::new(Expr::Symbol("http-server:start".to_string(), Type::Function(vec![Type::Int], Box::new(Type::List(Box::new(Type::TypeVar("ServerResponse".to_string()))))))),
            vec![Expr::Number(3000, Type::Int)],
            Type::List(Box::new(Type::TypeVar("ServerResponse".to_string()))),
        );
        
        let http_result = evaluator.eval(&http_expr);
        assert!(http_result.is_ok());
        
        // Step 3: Get timestamp
        let timestamp_expr = Expr::Application(
            Box::new(Expr::Symbol("async-utils:timestamp-ms".to_string(), Type::Function(vec![], Box::new(Type::Int)))),
            vec![],
            Type::Int,
        );
        
        let timestamp_result = evaluator.eval(&timestamp_expr);
        assert!(timestamp_result.is_ok());
        
        // All steps should succeed
        println!("Integration test completed successfully!");
    }
}
