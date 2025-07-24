//! Integration tests for TLisp with Rust modules and actor system

#[cfg(test)]
mod tests {
    use crate::tlisp::{Evaluator, Value, Expr, types::Type, environment::Environment};
    use std::rc::Rc;
    use std::cell::RefCell;

    fn create_test_evaluator() -> Evaluator {
        let env = Rc::new(RefCell::new(Environment::new()));

        // Add all module functions
        env.borrow_mut().define("import".to_string(), Value::Builtin("import".to_string()));
        env.borrow_mut().define("json:parse".to_string(), Value::Builtin("json:parse".to_string()));
        env.borrow_mut().define("json:stringify".to_string(), Value::Builtin("json:stringify".to_string()));
        env.borrow_mut().define("json:object".to_string(), Value::Builtin("json:object".to_string()));
        env.borrow_mut().define("http-server:start".to_string(), Value::Builtin("http-server:start".to_string()));
        env.borrow_mut().define("http-server:get".to_string(), Value::Builtin("http-server:get".to_string()));
        env.borrow_mut().define("async-utils:now".to_string(), Value::Builtin("async-utils:now".to_string()));
        env.borrow_mut().define("async-utils:timestamp-ms".to_string(), Value::Builtin("async-utils:timestamp-ms".to_string()));
        env.borrow_mut().define("async-utils:format-time".to_string(), Value::Builtin("async-utils:format-time".to_string()));
        env.borrow_mut().define("spawn".to_string(), Value::Builtin("spawn".to_string()));
        env.borrow_mut().define("self".to_string(), Value::Builtin("self".to_string()));
        env.borrow_mut().define("string-length".to_string(), Value::Builtin("string-length".to_string()));
        env.borrow_mut().define("string-append".to_string(), Value::Builtin("string-append".to_string()));
        env.borrow_mut().define("list-ref".to_string(), Value::Builtin("list-ref".to_string()));

        Evaluator::new(env)
    }

    #[test]
    fn test_http_json_integration() {
        let mut evaluator = create_test_evaluator();
        
        // Test importing modules
        let import_http = Expr::Application(
            Box::new(Expr::Symbol("import".to_string(), Type::Function(vec![Type::Symbol], Box::new(Type::Symbol)))),
            vec![Expr::Symbol("http-server".to_string(), Type::Symbol)],
            Type::Symbol,
        );
        
        let result = evaluator.eval(&import_http);
        assert!(result.is_ok());
        
        let import_json = Expr::Application(
            Box::new(Expr::Symbol("import".to_string(), Type::Function(vec![Type::Symbol], Box::new(Type::Symbol)))),
            vec![Expr::Symbol("json".to_string(), Type::Symbol)],
            Type::Symbol,
        );
        
        let result = evaluator.eval(&import_json);
        assert!(result.is_ok());
    }

    #[test]
    fn test_json_module_functions() {
        let mut evaluator = create_test_evaluator();
        
        // Test JSON parsing
        let json_parse = Expr::Application(
            Box::new(Expr::Symbol("json:parse".to_string(), Type::Function(vec![Type::String], Box::new(Type::TypeVar("JsonValue".to_string()))))),
            vec![Expr::String("{\"name\": \"test\", \"value\": 42}".to_string(), Type::String)],
            Type::TypeVar("JsonValue".to_string()),
        );
        
        let result = evaluator.eval(&json_parse);
        assert!(result.is_ok());
        
        // Test JSON object creation
        let json_object = Expr::Application(
            Box::new(Expr::Symbol("json:object".to_string(), Type::Function(vec![], Box::new(Type::String)))),
            vec![
                Expr::String("name".to_string(), Type::String),
                Expr::String("John".to_string(), Type::String),
                Expr::String("age".to_string(), Type::String),
                Expr::Number(30, Type::Int),
            ],
            Type::String,
        );
        
        let result = evaluator.eval(&json_object);
        assert!(result.is_ok());
        
        if let Ok(Value::String(json_str)) = result {
            assert!(json_str.contains("name"));
            assert!(json_str.contains("John"));
        }
    }

    #[test]
    fn test_http_module_functions() {
        let mut evaluator = create_test_evaluator();
        
        // Test HTTP server start
        let http_start = Expr::Application(
            Box::new(Expr::Symbol("http-server:start".to_string(), Type::Function(vec![Type::Int], Box::new(Type::List(Box::new(Type::TypeVar("ServerResponse".to_string()))))))),
            vec![Expr::Number(8080, Type::Int)],
            Type::List(Box::new(Type::TypeVar("ServerResponse".to_string()))),
        );
        
        let result = evaluator.eval(&http_start);
        assert!(result.is_ok());
        
        if let Ok(Value::List(response)) = result {
            assert_eq!(response.len(), 2);
            assert_eq!(response[0], Value::Symbol("server-started".to_string()));
            assert_eq!(response[1], Value::Int(8080));
        }
        
        // Test HTTP GET request
        let http_get = Expr::Application(
            Box::new(Expr::Symbol("http-server:get".to_string(), Type::Function(vec![Type::String], Box::new(Type::List(Box::new(Type::TypeVar("HttpResponse".to_string()))))))),
            vec![Expr::String("http://example.com".to_string(), Type::String)],
            Type::List(Box::new(Type::TypeVar("HttpResponse".to_string()))),
        );
        
        let result = evaluator.eval(&http_get);
        assert!(result.is_ok());
        
        if let Ok(Value::List(response)) = result {
            assert_eq!(response.len(), 3);
            assert_eq!(response[0], Value::Symbol("http-response".to_string()));
            assert_eq!(response[1], Value::Int(200));
        }
    }

    #[test]
    fn test_async_utils_module_functions() {
        let mut evaluator = create_test_evaluator();
        
        // Test timestamp functions
        let now_call = Expr::Application(
            Box::new(Expr::Symbol("async-utils:now".to_string(), Type::Function(vec![], Box::new(Type::Int)))),
            vec![],
            Type::Int,
        );
        
        let result = evaluator.eval(&now_call);
        assert!(result.is_ok());
        
        if let Ok(Value::Int(timestamp)) = result {
            assert!(timestamp > 0);
        }
        
        // Test timestamp-ms
        let timestamp_ms_call = Expr::Application(
            Box::new(Expr::Symbol("async-utils:timestamp-ms".to_string(), Type::Function(vec![], Box::new(Type::Int)))),
            vec![],
            Type::Int,
        );
        
        let result = evaluator.eval(&timestamp_ms_call);
        assert!(result.is_ok());
        
        if let Ok(Value::Int(timestamp)) = result {
            assert!(timestamp > 0);
        }
        
        // Test format-time
        let format_time_call = Expr::Application(
            Box::new(Expr::Symbol("async-utils:format-time".to_string(), Type::Function(vec![Type::Int, Type::String], Box::new(Type::String)))),
            vec![
                Expr::Number(1640995200, Type::Int),
                Expr::String("iso8601".to_string(), Type::String),
            ],
            Type::String,
        );
        
        let result = evaluator.eval(&format_time_call);
        assert!(result.is_ok());
        
        if let Ok(Value::String(formatted)) = result {
            assert!(!formatted.is_empty());
        }
    }

    #[test]
    fn test_actor_system_integration() {
        let mut evaluator = create_test_evaluator();
        
        // Test spawn function
        let spawn_call = Expr::Application(
            Box::new(Expr::Symbol("spawn".to_string(), Type::Function(vec![Type::Symbol], Box::new(Type::Pid)))),
            vec![Expr::Symbol("test-actor".to_string(), Type::Symbol)],
            Type::Pid,
        );
        
        let result = evaluator.eval(&spawn_call);
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), Value::Pid(_)));
        
        // Test self function
        let self_call = Expr::Application(
            Box::new(Expr::Symbol("self".to_string(), Type::Function(vec![], Box::new(Type::Pid)))),
            vec![],
            Type::Pid,
        );
        
        let result = evaluator.eval(&self_call);
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), Value::Pid(_)));
    }

    #[test]
    fn test_complex_integration_scenario() {
        let mut evaluator = create_test_evaluator();

        // Simulate a complex scenario: HTTP request -> JSON processing -> Actor communication

        // 1. Make HTTP request
        let http_get = Expr::Application(
            Box::new(Expr::Symbol("http-server:get".to_string(), Type::Function(vec![Type::String], Box::new(Type::List(Box::new(Type::TypeVar("HttpResponse".to_string()))))))),
            vec![Expr::String("http://api.example.com/data".to_string(), Type::String)],
            Type::List(Box::new(Type::TypeVar("HttpResponse".to_string()))),
        );

        let http_result = evaluator.eval(&http_get);
        assert!(http_result.is_ok());
        
        // 2. Process JSON response (simulated)
        let json_parse = Expr::Application(
            Box::new(Expr::Symbol("json:parse".to_string(), Type::Function(vec![Type::String], Box::new(Type::TypeVar("JsonValue".to_string()))))),
            vec![Expr::String("{\"status\": \"success\", \"data\": [1, 2, 3]}".to_string(), Type::String)],
            Type::TypeVar("JsonValue".to_string()),
        );
        
        let json_result = evaluator.eval(&json_parse);
        assert!(json_result.is_ok());
        
        // 3. Spawn actor to process data
        let spawn_processor = Expr::Application(
            Box::new(Expr::Symbol("spawn".to_string(), Type::Function(vec![Type::Symbol], Box::new(Type::Pid)))),
            vec![Expr::Symbol("data-processor".to_string(), Type::Symbol)],
            Type::Pid,
        );
        
        let spawn_result = evaluator.eval(&spawn_processor);
        assert!(spawn_result.is_ok());
        assert!(matches!(spawn_result.unwrap(), Value::Pid(_)));
    }

    #[test]
    fn test_error_handling_integration() {
        let mut evaluator = create_test_evaluator();
        
        // Test error handling in HTTP requests
        let bad_http_call = Expr::Application(
            Box::new(Expr::Symbol("http-server:get".to_string(), Type::Function(vec![Type::String], Box::new(Type::List(Box::new(Type::TypeVar("HttpResponse".to_string()))))))),
            vec![Expr::Number(123, Type::Int)], // Wrong type - should be string
            Type::List(Box::new(Type::TypeVar("HttpResponse".to_string()))),
        );
        
        let result = evaluator.eval(&bad_http_call);
        assert!(result.is_err());
        
        // Test error handling in JSON parsing
        let bad_json_call = Expr::Application(
            Box::new(Expr::Symbol("json:parse".to_string(), Type::Function(vec![Type::String], Box::new(Type::TypeVar("JsonValue".to_string()))))),
            vec![Expr::Number(123, Type::Int)], // Wrong type - should be string
            Type::TypeVar("JsonValue".to_string()),
        );
        
        let result = evaluator.eval(&bad_json_call);
        assert!(result.is_err());
    }

    #[test]
    fn test_module_function_chaining() {
        let mut evaluator = create_test_evaluator();
        
        // Test chaining module functions: HTTP -> JSON -> Async
        
        // 1. Get current timestamp
        let timestamp_call = Expr::Application(
            Box::new(Expr::Symbol("async-utils:timestamp-ms".to_string(), Type::Function(vec![], Box::new(Type::Int)))),
            vec![],
            Type::Int,
        );
        
        let timestamp_result = evaluator.eval(&timestamp_call);
        assert!(timestamp_result.is_ok());
        
        // 2. Create JSON object with timestamp
        let json_with_timestamp = Expr::Application(
            Box::new(Expr::Symbol("json:object".to_string(), Type::Function(vec![], Box::new(Type::String)))),
            vec![
                Expr::String("timestamp".to_string(), Type::String),
                Expr::Number(1640995200000, Type::Int), // Mock timestamp
                Expr::String("status".to_string(), Type::String),
                Expr::String("active".to_string(), Type::String),
            ],
            Type::String,
        );
        
        let json_result = evaluator.eval(&json_with_timestamp);
        assert!(json_result.is_ok());
        
        if let Ok(Value::String(json_str)) = json_result {
            assert!(json_str.contains("timestamp"));
            assert!(json_str.contains("status"));
        }
    }

    #[test]
    fn test_all_builtin_functions() {
        let mut evaluator = create_test_evaluator();
        
        // Test all the new builtin functions we added
        let test_cases = vec![
            // List access functions
            ("cadr", vec![Expr::List(vec![
                Expr::Number(1, Type::Int),
                Expr::Number(2, Type::Int),
                Expr::Number(3, Type::Int),
            ], Type::List(Box::new(Type::Int)))], Some(Value::Int(2))),
            
            // String functions
            ("string-length", vec![Expr::String("hello".to_string(), Type::String)], Some(Value::Int(5))),
            ("string-append", vec![
                Expr::String("hello".to_string(), Type::String),
                Expr::String(" world".to_string(), Type::String),
            ], Some(Value::String("hello world".to_string()))),
            
            // List functions
            ("list-ref", vec![
                Expr::List(vec![
                    Expr::String("a".to_string(), Type::String),
                    Expr::String("b".to_string(), Type::String),
                    Expr::String("c".to_string(), Type::String),
                ], Type::List(Box::new(Type::String))),
                Expr::Number(1, Type::Int),
            ], Some(Value::String("b".to_string()))),
        ];
        
        for (func_name, args, expected) in test_cases {
            let expr = Expr::Application(
                Box::new(Expr::Symbol(func_name.to_string(), Type::Function(vec![], Box::new(Type::TypeVar("a".to_string()))))),
                args,
                Type::TypeVar("a".to_string()),
            );
            
            let result = evaluator.eval(&expr);
            
            if let Some(expected_value) = expected {
                assert!(result.is_ok(), "Function {} failed", func_name);
                assert_eq!(result.unwrap(), expected_value, "Function {} returned wrong value", func_name);
            } else {
                // Just check that it doesn't crash
                let _ = result;
            }
        }
    }

    #[test]
    fn test_performance_with_large_data() {
        let mut evaluator = create_test_evaluator();
        
        // Test with large JSON data
        let large_json = format!("{{\"data\": [{}]}}", 
            (0..1000).map(|i| format!("{{\"id\": {}, \"value\": \"item_{}\"}}", i, i))
                     .collect::<Vec<_>>()
                     .join(", "));
        
        let json_parse = Expr::Application(
            Box::new(Expr::Symbol("json:parse".to_string(), Type::Function(vec![Type::String], Box::new(Type::TypeVar("JsonValue".to_string()))))),
            vec![Expr::String(large_json, Type::String)],
            Type::TypeVar("JsonValue".to_string()),
        );
        
        let start = std::time::Instant::now();
        let result = evaluator.eval(&json_parse);
        let duration = start.elapsed();
        
        assert!(result.is_ok());
        assert!(duration.as_millis() < 1000, "JSON parsing took too long: {:?}", duration);
    }
}
