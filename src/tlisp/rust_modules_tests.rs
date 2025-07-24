//! Unit tests for Rust modules integration

#[cfg(test)]
mod tests {
    use super::super::rust_modules::*;
    use crate::tlisp::Value;

    #[test]
    fn test_http_server_module_creation() {
        let module = create_http_server_module();
        assert_eq!(module.name, "http-server");
        assert_eq!(module.version, "1.0.0");
        assert!(module.exports.contains_key("start"));
        assert!(module.exports.contains_key("stop"));
        assert!(module.exports.contains_key("get"));
        assert!(module.exports.contains_key("post"));
    }

    #[test]
    fn test_json_module_creation() {
        let module = create_json_module();
        assert_eq!(module.name, "json");
        assert_eq!(module.version, "1.0.0");
        assert!(module.exports.contains_key("parse"));
        assert!(module.exports.contains_key("stringify"));
        assert!(module.exports.contains_key("get"));
        assert!(module.exports.contains_key("object"));
    }

    #[test]
    fn test_async_utils_module_creation() {
        let module = create_async_utils_module();
        assert_eq!(module.name, "async-utils");
        assert_eq!(module.version, "1.0.0");
        assert!(module.exports.contains_key("now"));
        assert!(module.exports.contains_key("timestamp-ms"));
        assert!(module.exports.contains_key("format-time"));
        assert!(module.exports.contains_key("sleep"));
    }

    #[test]
    fn test_http_server_start() {
        let args = vec![Value::Int(8080)];
        let result = http_server::start(&args);
        assert!(result.is_ok());
        
        if let Ok(Value::List(response)) = result {
            assert_eq!(response.len(), 2);
            assert_eq!(response[0], Value::Symbol("server-started".to_string()));
            assert_eq!(response[1], Value::Int(8080));
        }
    }

    #[test]
    fn test_http_server_start_invalid_args() {
        let args = vec![Value::String("invalid".to_string())];
        let result = http_server::start(&args);
        assert!(result.is_err());
    }

    #[test]
    fn test_http_server_get() {
        let args = vec![Value::String("http://example.com".to_string())];
        let result = http_server::get(&args);
        assert!(result.is_ok());
        
        if let Ok(Value::List(response)) = result {
            assert_eq!(response.len(), 3);
            assert_eq!(response[0], Value::Symbol("http-response".to_string()));
            assert_eq!(response[1], Value::Int(200));
        }
    }

    #[test]
    fn test_http_server_post() {
        let args = vec![
            Value::String("http://example.com".to_string()),
            Value::String("{\"test\": \"data\"}".to_string()),
        ];
        let result = http_server::post(&args);
        assert!(result.is_ok());
        
        if let Ok(Value::List(response)) = result {
            assert_eq!(response.len(), 3);
            assert_eq!(response[0], Value::Symbol("http-response".to_string()));
            assert_eq!(response[1], Value::Int(201));
        }
    }

    #[test]
    fn test_json_parse() {
        let args = vec![Value::String("{\"name\": \"John\", \"age\": 30}".to_string())];
        let result = json::parse(&args);
        assert!(result.is_ok());
    }

    #[test]
    fn test_json_parse_invalid() {
        let args = vec![Value::String("{ invalid json }".to_string())];
        let result = json::parse(&args);
        assert!(result.is_err());
    }

    #[test]
    fn test_json_stringify() {
        let args = vec![Value::String("test".to_string())];
        let result = json::stringify(&args);
        assert!(result.is_ok());
        
        if let Ok(Value::String(json_str)) = result {
            assert_eq!(json_str, "\"test\"");
        }
    }

    #[test]
    fn test_json_object() {
        let args = vec![
            Value::String("name".to_string()),
            Value::String("John".to_string()),
            Value::String("age".to_string()),
            Value::Int(30),
        ];
        let result = json::object(&args);
        assert!(result.is_ok());
        
        if let Ok(Value::String(json_str)) = result {
            assert!(json_str.contains("name"));
            assert!(json_str.contains("John"));
            assert!(json_str.contains("age"));
            assert!(json_str.contains("30"));
        }
    }

    #[test]
    fn test_json_object_odd_args() {
        let args = vec![Value::String("name".to_string())]; // Odd number of args
        let result = json::object(&args);
        assert!(result.is_err());
    }

    #[test]
    fn test_json_get() {
        let args = vec![
            Value::String("{\"name\": \"John\", \"age\": 30}".to_string()),
            Value::String("name".to_string()),
        ];
        let result = json::get(&args);
        assert!(result.is_ok());
        
        if let Ok(Value::String(value)) = result {
            assert_eq!(value, "John");
        }
    }

    #[test]
    fn test_json_get_missing_key() {
        let args = vec![
            Value::String("{\"name\": \"John\"}".to_string()),
            Value::String("missing".to_string()),
        ];
        let result = json::get(&args);
        assert!(result.is_ok());
        
        if let Ok(value) = result {
            assert_eq!(value, Value::Unit);
        }
    }

    #[test]
    fn test_async_utils_now() {
        let args = vec![];
        let result = async_utils::now(&args);
        assert!(result.is_ok());
        
        if let Ok(Value::Int(timestamp)) = result {
            assert!(timestamp > 0);
        }
    }

    #[test]
    fn test_async_utils_timestamp_ms() {
        let args = vec![];
        let result = async_utils::timestamp_ms(&args);
        assert!(result.is_ok());
        
        if let Ok(Value::Int(timestamp)) = result {
            assert!(timestamp > 0);
        }
    }

    #[test]
    fn test_async_utils_format_time() {
        let args = vec![
            Value::Int(1640995200), // 2022-01-01 00:00:00 UTC
            Value::String("iso8601".to_string()),
        ];
        let result = async_utils::format_time(&args);
        assert!(result.is_ok());
        
        if let Ok(Value::String(formatted)) = result {
            assert!(formatted.contains("timestamp:"));
        }
    }

    #[test]
    fn test_async_utils_sleep() {
        let args = vec![Value::Int(100)];
        let result = async_utils::sleep(&args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Value::Unit);
    }

    #[test]
    fn test_async_utils_spawn_task() {
        let args = vec![Value::String("test-task".to_string())];
        let result = async_utils::spawn_task(&args);
        assert!(result.is_ok());
        
        if let Ok(Value::Symbol(status)) = result {
            assert_eq!(status, "task-spawned");
        }
    }

    #[test]
    fn test_json_value_conversion() {
        // Test TLisp to JSON conversion - simplified test since the function is private
        let test_value = Value::String("test".to_string());
        let args = vec![test_value];
        let result = json::stringify(&args);
        assert!(result.is_ok(), "Failed to stringify TLisp value");
    }

    #[test]
    fn test_json_round_trip() {
        // Test round trip through stringify and parse
        let original = Value::String("hello".to_string());

        // Convert to JSON string
        let stringify_result = json::stringify(&vec![original.clone()]);
        assert!(stringify_result.is_ok());

        if let Ok(Value::String(json_str)) = stringify_result {
            // Parse it back
            let parse_result = json::parse(&vec![Value::String(json_str)]);
            assert!(parse_result.is_ok());
        }
    }

    #[test]
    fn test_error_handling() {
        // Test various error conditions
        
        // HTTP with wrong argument types
        let result = http_server::start(&vec![Value::String("not-a-number".to_string())]);
        assert!(result.is_err());
        
        // JSON with invalid data
        let result = json::parse(&vec![Value::Int(42)]); // Should be string
        assert!(result.is_err());
        
        // Async utils with wrong argument types
        let result = async_utils::format_time(&vec![Value::String("not-a-number".to_string())]);
        assert!(result.is_err());
    }

    #[test]
    fn test_module_exports() {
        // Test that all modules export the expected functions
        let http_module = create_http_server_module();
        let expected_http_functions = vec!["start", "stop", "get", "post", "send-response"];
        for func in expected_http_functions {
            assert!(http_module.exports.contains_key(func), "HTTP module missing function: {}", func);
        }

        let json_module = create_json_module();
        let expected_json_functions = vec!["parse", "stringify", "get", "set!", "object"];
        for func in expected_json_functions {
            assert!(json_module.exports.contains_key(func), "JSON module missing function: {}", func);
        }

        let async_module = create_async_utils_module();
        let expected_async_functions = vec!["now", "timestamp-ms", "format-time", "sleep", "spawn-task"];
        for func in expected_async_functions {
            assert!(async_module.exports.contains_key(func), "Async module missing function: {}", func);
        }
    }
}
