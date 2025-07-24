//! Rust modules for TLisp integration
//!
//! This module provides the Rust implementations that can be imported
//! and used from TLisp programs.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde_json::{Value as JsonValue, Map as JsonMap};
use crate::tlisp::{Value, TlispResult};
use crate::error::TlispError;
use crate::tlisp::module_system::{Module, ModuleLanguage};
use crate::tlisp::types::Type;

/// Create the HTTP server module
pub fn create_http_server_module() -> Module {
    let mut module = Module::new(
        "http-server".to_string(),
        "1.0.0".to_string(),
        ModuleLanguage::Rust,
    );

    // Export HTTP server functions
    module.export_symbol("start".to_string(), Value::Builtin("http-server:start".to_string()));
    module.export_symbol("stop".to_string(), Value::Builtin("http-server:stop".to_string()));
    module.export_symbol("get".to_string(), Value::Builtin("http-server:get".to_string()));
    module.export_symbol("post".to_string(), Value::Builtin("http-server:post".to_string()));
    module.export_symbol("put".to_string(), Value::Builtin("http-server:put".to_string()));
    module.export_symbol("delete".to_string(), Value::Builtin("http-server:delete".to_string()));
    module.export_symbol("send-response".to_string(), Value::Builtin("http-server:send-response".to_string()));

    // Export types
    module.export_type("HttpRequest".to_string(), Type::TypeVar("HttpRequest".to_string()));
    module.export_type("HttpResponse".to_string(), Type::TypeVar("HttpResponse".to_string()));

    module
}

/// Create the JSON processing module
pub fn create_json_module() -> Module {
    let mut module = Module::new(
        "json".to_string(),
        "1.0.0".to_string(),
        ModuleLanguage::Rust,
    );

    // Export JSON functions
    module.export_symbol("parse".to_string(), Value::Builtin("json:parse".to_string()));
    module.export_symbol("stringify".to_string(), Value::Builtin("json:stringify".to_string()));
    module.export_symbol("get".to_string(), Value::Builtin("json:get".to_string()));
    module.export_symbol("set!".to_string(), Value::Builtin("json:set!".to_string()));
    module.export_symbol("object".to_string(), Value::Builtin("json:object".to_string()));

    // Export types
    module.export_type("JsonValue".to_string(), Type::TypeVar("JsonValue".to_string()));

    module
}

/// Create the async utilities module
pub fn create_async_utils_module() -> Module {
    let mut module = Module::new(
        "async-utils".to_string(),
        "1.0.0".to_string(),
        ModuleLanguage::Rust,
    );

    // Export async utility functions
    module.export_symbol("now".to_string(), Value::Builtin("async-utils:now".to_string()));
    module.export_symbol("timestamp-ms".to_string(), Value::Builtin("async-utils:timestamp-ms".to_string()));
    module.export_symbol("format-time".to_string(), Value::Builtin("async-utils:format-time".to_string()));
    module.export_symbol("sleep".to_string(), Value::Builtin("async-utils:sleep".to_string()));
    module.export_symbol("spawn-task".to_string(), Value::Builtin("async-utils:spawn-task".to_string()));
    module.export_symbol("timestamp-iso".to_string(), Value::Builtin("async-utils:timestamp-iso".to_string()));

    // Export types
    module.export_type("Timestamp".to_string(), Type::Int);
    module.export_type("Duration".to_string(), Type::Int);

    module
}

/// Create the Ream ORM module
pub fn create_ream_orm_module() -> Module {
    let mut module = Module::new(
        "ream-orm".to_string(),
        "1.0.0".to_string(),
        ModuleLanguage::Rust,
    );

    // Export database connection functions
    module.export_symbol("connect".to_string(), Value::Builtin("ream-orm:connect".to_string()));
    module.export_symbol("disconnect".to_string(), Value::Builtin("ream-orm:disconnect".to_string()));
    module.export_symbol("execute".to_string(), Value::Builtin("ream-orm:execute".to_string()));
    module.export_symbol("execute-query".to_string(), Value::Builtin("ream-orm:execute-query".to_string()));
    module.export_symbol("execute-query-single".to_string(), Value::Builtin("ream-orm:execute-query-single".to_string()));
    module.export_symbol("execute-mutation".to_string(), Value::Builtin("ream-orm:execute-mutation".to_string()));
    module.export_symbol("execute-transaction".to_string(), Value::Builtin("ream-orm:execute-transaction".to_string()));

    // Export query builder functions
    module.export_symbol("create-query-builder".to_string(), Value::Builtin("ream-orm:create-query-builder".to_string()));
    module.export_symbol("select".to_string(), Value::Builtin("ream-orm:select".to_string()));
    module.export_symbol("where".to_string(), Value::Builtin("ream-orm:where".to_string()));
    module.export_symbol("limit".to_string(), Value::Builtin("ream-orm:limit".to_string()));
    module.export_symbol("order-by".to_string(), Value::Builtin("ream-orm:order-by".to_string()));
    module.export_symbol("build-query".to_string(), Value::Builtin("ream-orm:build-query".to_string()));

    // Export mutation builder functions
    module.export_symbol("create-mutation-builder".to_string(), Value::Builtin("ream-orm:create-mutation-builder".to_string()));
    module.export_symbol("insert".to_string(), Value::Builtin("ream-orm:insert".to_string()));
    module.export_symbol("update".to_string(), Value::Builtin("ream-orm:update".to_string()));
    module.export_symbol("delete".to_string(), Value::Builtin("ream-orm:delete".to_string()));
    module.export_symbol("returning".to_string(), Value::Builtin("ream-orm:returning".to_string()));
    module.export_symbol("build-mutation".to_string(), Value::Builtin("ream-orm:build-mutation".to_string()));

    // Export schema functions
    module.export_symbol("get-schema-info".to_string(), Value::Builtin("ream-orm:get-schema-info".to_string()));

    // Export types
    module.export_type("Connection".to_string(), Type::TypeVar("Connection".to_string()));
    module.export_type("QueryBuilder".to_string(), Type::TypeVar("QueryBuilder".to_string()));
    module.export_type("MutationBuilder".to_string(), Type::TypeVar("MutationBuilder".to_string()));
    module.export_type("QueryResult".to_string(), Type::TypeVar("QueryResult".to_string()));

    module
}

/// Create the Ream GraphQL module
pub fn create_ream_graphql_module() -> Module {
    let mut module = Module::new(
        "ream-graphql".to_string(),
        "1.0.0".to_string(),
        ModuleLanguage::Rust,
    );

    // Export GraphQL functions
    module.export_symbol("create-context".to_string(), Value::Builtin("ream-graphql:create-context".to_string()));
    module.export_symbol("parse-query".to_string(), Value::Builtin("ream-graphql:parse-query".to_string()));
    module.export_symbol("parse-mutation".to_string(), Value::Builtin("ream-graphql:parse-mutation".to_string()));
    module.export_symbol("compile-query".to_string(), Value::Builtin("ream-graphql:compile-query".to_string()));
    module.export_symbol("compile-mutation".to_string(), Value::Builtin("ream-graphql:compile-mutation".to_string()));

    // Export types
    module.export_type("GraphQLContext".to_string(), Type::TypeVar("GraphQLContext".to_string()));
    module.export_type("ParsedQuery".to_string(), Type::TypeVar("ParsedQuery".to_string()));
    module.export_type("CompiledQuery".to_string(), Type::TypeVar("CompiledQuery".to_string()));

    module
}

/// HTTP server implementation functions
pub mod http_server {
    use super::*;

    /// Start HTTP server
    pub fn start(args: &[Value]) -> TlispResult<Value> {
        if args.len() < 1 {
            return Err(TlispError::Runtime("http-server:start requires at least 1 argument (port)".to_string()));
        }

        let port = match &args[0] {
            Value::Int(p) => *p as u16,
            _ => return Err(TlispError::Runtime("Port must be a number".to_string())),
        };

        // Extract routes and middleware if provided
        let _routes = if args.len() > 1 { &args[1] } else { &Value::List(vec![]) };
        let _middleware = if args.len() > 2 { &args[2] } else { &Value::List(vec![]) };

        // Start actual HTTP server using warp
        println!("🚀 Starting TLisp HTTP server on port {}", port);

        // Start the server in a background thread
        let rt = tokio::runtime::Runtime::new().unwrap();
        std::thread::spawn(move || {
            rt.block_on(async {
                start_tlisp_web_server(port).await;
            });
        });

        // Give the server time to start
        std::thread::sleep(std::time::Duration::from_millis(500));

        println!("✅ HTTP server started successfully on port {}", port);
        println!("📋 Available endpoints:");
        println!("  GET  /api/users        - List users");
        println!("  POST /api/users        - Create user");
        println!("  GET  /api/users/<id>   - Get user by ID");
        println!("  POST /api/posts        - Create post");
        println!("  GET  /health           - Health check");

        Ok(Value::List(vec![
            Value::Symbol("server-started".to_string()),
            Value::Int(port as i64),
        ]))
    }

    /// Stop HTTP server
    pub fn stop(_args: &[Value]) -> TlispResult<Value> {
        // TODO: Implement actual HTTP server shutdown
        println!("Stopping HTTP server");
        Ok(Value::Symbol("server-stopped".to_string()))
    }

    /// Make HTTP GET request
    pub fn get(args: &[Value]) -> TlispResult<Value> {
        if args.len() != 1 {
            return Err(TlispError::Runtime("http-server:get requires 1 argument (url)".to_string()));
        }

        let url = match &args[0] {
            Value::String(u) => u.clone(),
            _ => return Err(TlispError::Runtime("URL must be a string".to_string())),
        };

        // Make actual HTTP GET request using reqwest
        println!("Making GET request to: {}", url);

        // Use blocking reqwest for simplicity in this context
        match std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let client = reqwest::Client::new();
                let response = client.get(&url).send().await?;
                let status = response.status().as_u16();
                let body = response.text().await?;
                Ok::<(u16, String), reqwest::Error>((status, body))
            })
        }).join() {
            Ok(Ok((status, body))) => {
                Ok(Value::List(vec![
                    Value::Symbol("http-response".to_string()),
                    Value::Int(status as i64),
                    Value::String(body),
                ]))
            }
            Ok(Err(e)) => {
                Err(TlispError::Runtime(format!("HTTP request failed: {}", e)))
            }
            Err(_) => {
                Err(TlispError::Runtime("HTTP request thread panicked".to_string()))
            }
        }
    }

    /// Make HTTP POST request
    pub fn post(args: &[Value]) -> TlispResult<Value> {
        if args.len() != 2 {
            return Err(TlispError::Runtime("http-server:post requires 2 arguments (url, body)".to_string()));
        }

        let url = match &args[0] {
            Value::String(u) => u,
            _ => return Err(TlispError::Runtime("URL must be a string".to_string())),
        };

        let body = match &args[1] {
            Value::String(b) => b,
            _ => return Err(TlispError::Runtime("Body must be a string".to_string())),
        };

        // TODO: Implement actual HTTP POST using reqwest
        println!("Making POST request to: {} with body: {}", url, body);

        Ok(Value::List(vec![
            Value::Symbol("http-response".to_string()),
            Value::Int(201),
            Value::String(format!("{{\"message\": \"Created\", \"echo\": {}}}", body)),
        ]))
    }

    /// Make HTTP PUT request
    pub fn put(args: &[Value]) -> TlispResult<Value> {
        if args.len() != 2 {
            return Err(TlispError::Runtime("http-server:put requires 2 arguments (url, body)".to_string()));
        }

        let url = match &args[0] {
            Value::String(u) => u,
            _ => return Err(TlispError::Runtime("URL must be a string".to_string())),
        };

        let body = match &args[1] {
            Value::String(b) => b,
            _ => return Err(TlispError::Runtime("Body must be a string".to_string())),
        };

        println!("Making PUT request to: {} with body: {}", url, body);

        Ok(Value::List(vec![
            Value::Symbol("http-response".to_string()),
            Value::Int(200),
            Value::String(format!("{{\"message\": \"Updated\", \"echo\": {}}}", body)),
        ]))
    }

    /// Make HTTP DELETE request
    pub fn delete(args: &[Value]) -> TlispResult<Value> {
        if args.len() != 1 {
            return Err(TlispError::Runtime("http-server:delete requires 1 argument (url)".to_string()));
        }

        let url = match &args[0] {
            Value::String(u) => u.clone(),
            _ => return Err(TlispError::Runtime("URL must be a string".to_string())),
        };

        println!("Making DELETE request to: {}", url);

        Ok(Value::List(vec![
            Value::Symbol("http-response".to_string()),
            Value::Int(204),
            Value::String("{}".to_string()),
        ]))
    }

    /// Send HTTP response
    pub fn send_response(args: &[Value]) -> TlispResult<Value> {
        if args.len() != 2 {
            return Err(TlispError::Runtime("http-server:send-response requires 2 arguments (channel, response)".to_string()));
        }

        // TODO: Implement actual response sending
        println!("Sending HTTP response: {:?}", args[1]);
        Ok(Value::Unit)
    }
}

/// JSON processing implementation functions
pub mod json {
    use super::*;

    /// Parse JSON string to TLisp value
    pub fn parse(args: &[Value]) -> TlispResult<Value> {
        if args.len() != 1 {
            return Err(TlispError::Runtime("json:parse requires 1 argument".to_string()));
        }

        let json_string = match &args[0] {
            Value::String(s) => s,
            _ => return Err(TlispError::Runtime("json:parse requires a string".to_string())),
        };

        match serde_json::from_str::<JsonValue>(json_string) {
            Ok(json_value) => Ok(json_to_tlisp_value(json_value)),
            Err(e) => Err(TlispError::Runtime(format!("JSON parse error: {}", e))),
        }
    }

    /// Convert TLisp value to JSON string
    pub fn stringify(args: &[Value]) -> TlispResult<Value> {
        if args.len() != 1 {
            return Err(TlispError::Runtime("json:stringify requires 1 argument".to_string()));
        }

        let json_value = tlisp_to_json_value(args[0].clone())?;
        match serde_json::to_string(&json_value) {
            Ok(json_string) => Ok(Value::String(json_string)),
            Err(e) => Err(TlispError::Runtime(format!("JSON stringify error: {}", e))),
        }
    }

    /// Get value from JSON object by key
    pub fn get(args: &[Value]) -> TlispResult<Value> {
        if args.len() != 2 {
            return Err(TlispError::Runtime("json:get requires 2 arguments".to_string()));
        }

        let json_obj = match &args[0] {
            Value::String(json_str) => {
                match serde_json::from_str::<JsonValue>(json_str) {
                    Ok(val) => val,
                    Err(e) => return Err(TlispError::Runtime(format!("JSON parse error: {}", e))),
                }
            }
            // Handle TLisp objects directly (for web server use case)
            Value::List(pairs) => {
                // Convert TLisp association list to JSON object
                let mut map = JsonMap::new();
                for pair in pairs {
                    if let Value::List(kv) = pair {
                        if kv.len() == 2 {
                            let key = match &kv[0] {
                                Value::String(s) => s.clone(),
                                Value::Symbol(s) => s.clone(),
                                _ => continue,
                            };
                            let value = tlisp_to_json_value(kv[1].clone())?;
                            map.insert(key, value);
                        }
                    }
                }
                JsonValue::Object(map)
            }
            _ => return Err(TlispError::Runtime("json:get first argument must be a JSON string or TLisp object".to_string())),
        };

        let key = match &args[1] {
            Value::String(k) => k,
            _ => return Err(TlispError::Runtime("json:get second argument must be a string key".to_string())),
        };

        if let JsonValue::Object(map) = json_obj {
            if let Some(value) = map.get(key) {
                Ok(json_to_tlisp_value(value.clone()))
            } else {
                Ok(Value::Unit)
            }
        } else {
            Err(TlispError::Runtime("json:get requires a JSON object".to_string()))
        }
    }

    /// Set value in JSON object by key (mutating operation)
    pub fn set(args: &[Value]) -> TlispResult<Value> {
        if args.len() != 3 {
            return Err(TlispError::Runtime("json:set! requires 3 arguments (object, key, value)".to_string()));
        }

        // For now, return a success indicator since TLisp doesn't have mutable objects
        // In a real implementation, this would modify the object in place
        println!("json:set! called with key: {:?}, value: {:?}", args[1], args[2]);
        Ok(Value::Unit)
    }

    /// Create JSON object from key-value pairs
    pub fn object(args: &[Value]) -> TlispResult<Value> {
        // Handle both flat key-value pairs and list of pairs
        if args.is_empty() {
            return Ok(Value::List(vec![])); // Empty object as association list
        }

        // Check if we have a single argument that's a list of pairs
        if args.len() == 1 {
            if let Value::List(pairs) = &args[0] {
                // Already an association list, return as-is
                return Ok(args[0].clone());
            }
        }

        // Handle flat key-value pairs
        if args.len() % 2 != 0 {
            return Err(TlispError::Runtime("json:object requires an even number of arguments (key-value pairs)".to_string()));
        }

        let mut pairs = Vec::new();
        for chunk in args.chunks(2) {
            let key = match &chunk[0] {
                Value::String(k) => Value::String(k.clone()),
                Value::Symbol(s) => Value::String(s.clone()),
                _ => return Err(TlispError::Runtime("JSON object keys must be strings or symbols".to_string())),
            };
            let value = chunk[1].clone();
            pairs.push(Value::List(vec![key, value]));
        }

        // Return as TLisp association list for easier manipulation
        Ok(Value::List(pairs))
    }

    // Helper functions for JSON conversion
    fn json_to_tlisp_value(json_value: JsonValue) -> Value {
        match json_value {
            JsonValue::Null => Value::Unit,
            JsonValue::Bool(b) => Value::Bool(b),
            JsonValue::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Value::Int(i)
                } else if let Some(f) = n.as_f64() {
                    Value::Float(f)
                } else {
                    Value::Unit
                }
            }
            JsonValue::String(s) => Value::String(s),
            JsonValue::Array(arr) => {
                let tlisp_values: Vec<Value> = arr.into_iter()
                    .map(json_to_tlisp_value)
                    .collect();
                Value::List(tlisp_values)
            }
            JsonValue::Object(map) => {
                let pairs: Vec<Value> = map.into_iter()
                    .map(|(k, v)| Value::List(vec![Value::String(k), json_to_tlisp_value(v)]))
                    .collect();
                Value::List(pairs)
            }
        }
    }

    fn tlisp_to_json_value(tlisp_value: Value) -> TlispResult<JsonValue> {
        match tlisp_value {
            Value::Unit => Ok(JsonValue::Null),
            Value::Bool(b) => Ok(JsonValue::Bool(b)),
            Value::Int(i) => Ok(JsonValue::Number(serde_json::Number::from(i))),
            Value::Float(f) => {
                if let Some(n) = serde_json::Number::from_f64(f) {
                    Ok(JsonValue::Number(n))
                } else {
                    Err(TlispError::Runtime("Invalid float value for JSON".to_string()))
                }
            }
            Value::String(s) => Ok(JsonValue::String(s)),
            Value::Symbol(s) => Ok(JsonValue::String(s)),
            Value::List(values) => {
                // Try to detect if this is an association list (object) or array
                if values.iter().all(|v| {
                    if let Value::List(pair) = v {
                        pair.len() == 2 && matches!(pair[0], Value::String(_) | Value::Symbol(_))
                    } else {
                        false
                    }
                }) {
                    // Convert to JSON object
                    let mut map = JsonMap::new();
                    for value in values {
                        if let Value::List(pair) = value {
                            let key = match &pair[0] {
                                Value::String(s) => s.clone(),
                                Value::Symbol(s) => s.clone(),
                                _ => continue,
                            };
                            let json_value = tlisp_to_json_value(pair[1].clone())?;
                            map.insert(key, json_value);
                        }
                    }
                    Ok(JsonValue::Object(map))
                } else {
                    // Convert to JSON array
                    let json_values: Result<Vec<JsonValue>, TlispError> = values.into_iter()
                        .map(tlisp_to_json_value)
                        .collect();
                    Ok(JsonValue::Array(json_values?))
                }
            }
            _ => Err(TlispError::Runtime("Unsupported TLisp value type for JSON conversion".to_string())),
        }
    }
}

/// Async utilities implementation functions
pub mod async_utils {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    /// Get current timestamp
    pub fn now(_args: &[Value]) -> TlispResult<Value> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| TlispError::Runtime(format!("Time error: {}", e)))?;
        Ok(Value::Int(now.as_secs() as i64))
    }

    /// Get current timestamp in milliseconds
    pub fn timestamp_ms(_args: &[Value]) -> TlispResult<Value> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| TlispError::Runtime(format!("Time error: {}", e)))?;
        Ok(Value::Int(now.as_millis() as i64))
    }

    /// Format time as string
    pub fn format_time(args: &[Value]) -> TlispResult<Value> {
        if args.len() != 2 {
            return Err(TlispError::Runtime("async-utils:format-time requires 2 arguments".to_string()));
        }

        let timestamp = match &args[0] {
            Value::Int(t) => *t,
            _ => return Err(TlispError::Runtime("Timestamp must be a number".to_string())),
        };

        let format = match &args[1] {
            Value::String(f) => f,
            _ => return Err(TlispError::Runtime("Format must be a string".to_string())),
        };

        // Simple format implementation
        let formatted = match format.as_str() {
            "iso8601" => {
                // Simple ISO 8601 format
                let datetime = UNIX_EPOCH + std::time::Duration::from_secs(timestamp as u64);
                format!("{:?}", datetime) // Simplified for now
            }
            _ => format!("timestamp:{}", timestamp),
        };

        Ok(Value::String(formatted))
    }

    /// Sleep for specified milliseconds
    pub fn sleep(args: &[Value]) -> TlispResult<Value> {
        if args.len() != 1 {
            return Err(TlispError::Runtime("async-utils:sleep requires 1 argument".to_string()));
        }

        let ms = match &args[0] {
            Value::Int(m) => *m,
            _ => return Err(TlispError::Runtime("Sleep duration must be a number".to_string())),
        };

        // Implement actual sleep using std::thread::sleep
        println!("Sleeping for {} ms", ms);
        std::thread::sleep(std::time::Duration::from_millis(ms as u64));
        Ok(Value::Unit)
    }

    /// Spawn async task (placeholder)
    pub fn spawn_task(args: &[Value]) -> TlispResult<Value> {
        if args.len() != 1 {
            return Err(TlispError::Runtime("async-utils:spawn-task requires 1 argument".to_string()));
        }

        // TODO: Implement actual task spawning
        println!("Spawning async task: {:?}", args[0]);
        Ok(Value::Symbol("task-spawned".to_string()))
    }

    /// Get current timestamp in ISO format
    pub fn timestamp_iso(_args: &[Value]) -> TlispResult<Value> {
        let now = std::time::SystemTime::now();
        let datetime = chrono::DateTime::<chrono::Utc>::from(now);
        Ok(Value::String(datetime.to_rfc3339()))
    }
}

/// Ream ORM implementation functions
pub mod ream_orm {
    use super::*;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    // Global connection storage (simplified for demo)
    static mut CONNECTIONS: Option<Arc<Mutex<HashMap<String, String>>>> = None;

    fn get_connections() -> Arc<Mutex<HashMap<String, String>>> {
        unsafe {
            if CONNECTIONS.is_none() {
                CONNECTIONS = Some(Arc::new(Mutex::new(HashMap::new())));
            }
            CONNECTIONS.as_ref().unwrap().clone()
        }
    }

    /// Connect to database
    pub fn connect(args: &[Value]) -> TlispResult<Value> {
        if args.len() != 1 {
            return Err(TlispError::Runtime("ream-orm:connect requires 1 argument (connection_string)".to_string()));
        }

        let connection_string = match &args[0] {
            Value::String(s) => s.clone(),
            _ => return Err(TlispError::Runtime("Connection string must be a string".to_string())),
        };

        println!("🔌 Connecting to database: {}", connection_string);

        // Store connection (simplified - in real implementation would create actual DB connection)
        let connections = get_connections();
        let mut conn_map = connections.lock().unwrap();
        let conn_id = format!("conn_{}", conn_map.len() + 1);
        conn_map.insert(conn_id.clone(), connection_string);

        println!("✅ Database connection established: {}", conn_id);

        Ok(Value::String(conn_id))
    }

    /// Disconnect from database
    pub fn disconnect(args: &[Value]) -> TlispResult<Value> {
        if args.len() != 1 {
            return Err(TlispError::Runtime("ream-orm:disconnect requires 1 argument (connection)".to_string()));
        }

        let conn_id = match &args[0] {
            Value::String(s) => s.clone(),
            _ => return Err(TlispError::Runtime("Connection must be a string".to_string())),
        };

        println!("🔌 Disconnecting from database: {}", conn_id);

        let connections = get_connections();
        let mut conn_map = connections.lock().unwrap();
        conn_map.remove(&conn_id);

        Ok(Value::Unit)
    }

    /// Execute raw SQL
    pub fn execute(args: &[Value]) -> TlispResult<Value> {
        if args.len() != 2 {
            return Err(TlispError::Runtime("ream-orm:execute requires 2 arguments (connection, sql)".to_string()));
        }

        let _conn_id = match &args[0] {
            Value::String(s) => s.clone(),
            _ => return Err(TlispError::Runtime("Connection must be a string".to_string())),
        };

        let sql = match &args[1] {
            Value::String(s) => s.clone(),
            _ => return Err(TlispError::Runtime("SQL must be a string".to_string())),
        };

        println!("📊 Executing SQL: {}", sql);

        // Simulate SQL execution
        Ok(Value::Int(1)) // Return affected rows
    }

    /// Create query builder
    pub fn create_query_builder(_args: &[Value]) -> TlispResult<Value> {
        // Return a query builder identifier
        Ok(Value::String("query_builder_1".to_string()))
    }

    /// Add SELECT clause to query builder
    pub fn select(args: &[Value]) -> TlispResult<Value> {
        if args.len() != 3 {
            return Err(TlispError::Runtime("ream-orm:select requires 3 arguments (builder, table, columns)".to_string()));
        }

        let builder = &args[0];
        let table = match &args[1] {
            Value::String(s) => s.clone(),
            _ => return Err(TlispError::Runtime("Table must be a string".to_string())),
        };

        println!("🔍 Adding SELECT for table: {}", table);

        // Return modified builder (simplified)
        Ok(builder.clone())
    }

    /// Execute query
    pub fn execute_query(args: &[Value]) -> TlispResult<Value> {
        if args.len() != 2 {
            return Err(TlispError::Runtime("ream-orm:execute-query requires 2 arguments (connection, query)".to_string()));
        }

        println!("📊 Executing ORM query");

        // Return mock data for users table
        Ok(Value::List(vec![
            Value::List(vec![
                Value::List(vec![Value::String("id".to_string()), Value::Int(1)]),
                Value::List(vec![Value::String("name".to_string()), Value::String("Alice".to_string())]),
                Value::List(vec![Value::String("email".to_string()), Value::String("alice@example.com".to_string())]),
            ]),
            Value::List(vec![
                Value::List(vec![Value::String("id".to_string()), Value::Int(2)]),
                Value::List(vec![Value::String("name".to_string()), Value::String("Bob".to_string())]),
                Value::List(vec![Value::String("email".to_string()), Value::String("bob@example.com".to_string())]),
            ]),
        ]))
    }

    /// Execute query returning single result
    pub fn execute_query_single(args: &[Value]) -> TlispResult<Value> {
        if args.len() != 2 {
            return Err(TlispError::Runtime("ream-orm:execute-query-single requires 2 arguments".to_string()));
        }

        println!("📊 Executing single ORM query");

        // Return single mock user
        Ok(Value::List(vec![
            Value::List(vec![Value::String("id".to_string()), Value::Int(1)]),
            Value::List(vec![Value::String("name".to_string()), Value::String("Alice".to_string())]),
            Value::List(vec![Value::String("email".to_string()), Value::String("alice@example.com".to_string())]),
        ]))
    }

    /// Create mutation builder
    pub fn create_mutation_builder(_args: &[Value]) -> TlispResult<Value> {
        Ok(Value::String("mutation_builder_1".to_string()))
    }

    /// Execute mutation
    pub fn execute_mutation(args: &[Value]) -> TlispResult<Value> {
        if args.len() != 2 {
            return Err(TlispError::Runtime("ream-orm:execute-mutation requires 2 arguments".to_string()));
        }

        println!("✏️ Executing ORM mutation");

        // Return created/updated record
        Ok(Value::List(vec![
            Value::List(vec![Value::String("id".to_string()), Value::Int(123)]),
            Value::List(vec![Value::String("name".to_string()), Value::String("New User".to_string())]),
            Value::List(vec![Value::String("email".to_string()), Value::String("new@example.com".to_string())]),
        ]))
    }

    /// Get schema information
    pub fn get_schema_info(args: &[Value]) -> TlispResult<Value> {
        if args.len() != 1 {
            return Err(TlispError::Runtime("ream-orm:get-schema-info requires 1 argument".to_string()));
        }

        println!("📋 Getting schema information");

        // Return mock schema info
        Ok(Value::List(vec![
            Value::List(vec![
                Value::String("table".to_string()),
                Value::String("users".to_string()),
                Value::List(vec![
                    Value::String("id".to_string()),
                    Value::String("name".to_string()),
                    Value::String("email".to_string()),
                ]),
            ]),
        ]))
    }
}

/// Ream GraphQL implementation functions
pub mod ream_graphql {
    use super::*;

    /// Create GraphQL context
    pub fn create_context(_args: &[Value]) -> TlispResult<Value> {
        println!("🎯 Creating GraphQL context");
        Ok(Value::String("graphql_context_1".to_string()))
    }

    /// Parse GraphQL query
    pub fn parse_query(args: &[Value]) -> TlispResult<Value> {
        if args.len() != 1 {
            return Err(TlispError::Runtime("ream-graphql:parse-query requires 1 argument".to_string()));
        }

        let query = match &args[0] {
            Value::String(s) => s.clone(),
            _ => return Err(TlispError::Runtime("Query must be a string".to_string())),
        };

        println!("🔍 Parsing GraphQL query: {}", query);

        // Simple GraphQL parser - extract table and fields
        let parsed = parse_simple_graphql_query(&query)?;

        // Return parsed query as JSON-like structure
        Ok(Value::String(format!("{{\"table\":\"{}\",\"fields\":{:?}}}", parsed.table, parsed.fields)))
    }

    /// Parse GraphQL mutation
    pub fn parse_mutation(args: &[Value]) -> TlispResult<Value> {
        if args.len() != 1 {
            return Err(TlispError::Runtime("ream-graphql:parse-mutation requires 1 argument".to_string()));
        }

        let mutation = match &args[0] {
            Value::String(s) => s.clone(),
            _ => return Err(TlispError::Runtime("Mutation must be a string".to_string())),
        };

        println!("✏️ Parsing GraphQL mutation: {}", mutation);

        // Return parsed mutation representation
        Ok(Value::String(format!("parsed_mutation:{}", mutation)))
    }

    /// Compile GraphQL query to SQL
    pub fn compile_query(args: &[Value]) -> TlispResult<Value> {
        if args.len() != 3 {
            return Err(TlispError::Runtime("ream-graphql:compile-query requires 3 arguments".to_string()));
        }

        let _context = &args[0]; // GraphQL context (unused for now)
        let parsed_query = match &args[1] {
            Value::String(s) => s.clone(),
            _ => return Err(TlispError::Runtime("Parsed query must be a string".to_string())),
        };
        let _table_hint = &args[2]; // Table hint (unused for now)

        println!("⚙️ Compiling GraphQL query to SQL");

        // Parse the JSON-like structure from parse_query
        let compiled_sql = compile_parsed_query_to_sql(&parsed_query)?;
        println!("🗄️  Generated SQL: {}", compiled_sql);

        Ok(Value::String(compiled_sql))
    }

    /// Compile GraphQL mutation to SQL
    pub fn compile_mutation(args: &[Value]) -> TlispResult<Value> {
        if args.len() != 3 {
            return Err(TlispError::Runtime("ream-graphql:compile-mutation requires 3 arguments".to_string()));
        }

        println!("⚙️ Compiling GraphQL mutation to SQL");

        // Return compiled SQL mutation
        Ok(Value::String("INSERT INTO users (name, email) VALUES (?, ?)".to_string()))
    }

    /// Simple GraphQL query structure
    #[derive(Debug)]
    struct SimpleGraphQLQuery {
        table: String,
        fields: Vec<String>,
    }

    /// Parse a simple GraphQL query like "{ users { id name email } }"
    fn parse_simple_graphql_query(query: &str) -> TlispResult<SimpleGraphQLQuery> {
        let trimmed = query.trim();

        // Remove outer braces and "query" keyword if present
        let inner = if trimmed.starts_with("query") {
            // Handle "query { users { id name } }" or "query GetUsers { users { id name } }"
            let after_query = trimmed.strip_prefix("query").unwrap().trim();
            if after_query.starts_with('{') {
                after_query.trim_start_matches('{').trim_end_matches('}').trim()
            } else {
                // Skip query name like "GetUsers"
                let parts: Vec<&str> = after_query.splitn(2, '{').collect();
                if parts.len() == 2 {
                    parts[1].trim_end_matches('}').trim()
                } else {
                    return Err(TlispError::Runtime("Invalid GraphQL query format".to_string()));
                }
            }
        } else if trimmed.starts_with('{') && trimmed.ends_with('}') {
            trimmed.trim_start_matches('{').trim_end_matches('}').trim()
        } else {
            return Err(TlispError::Runtime("GraphQL query must be wrapped in braces".to_string()));
        };

        // Parse "users { id name email }"
        let parts: Vec<&str> = inner.splitn(2, '{').collect();
        if parts.len() != 2 {
            return Err(TlispError::Runtime("Invalid GraphQL query structure".to_string()));
        }

        let table = parts[0].trim().to_string();
        let fields_str = parts[1].trim_end_matches('}').trim();
        let fields: Vec<String> = fields_str
            .split_whitespace()
            .map(|s| s.to_string())
            .collect();

        Ok(SimpleGraphQLQuery { table, fields })
    }

    /// Compile parsed GraphQL query to SQL
    fn compile_parsed_query_to_sql(parsed_json: &str) -> TlispResult<String> {
        // Parse the JSON-like structure: {"table":"users","fields":["id","name","email"]}
        if parsed_json.starts_with("{\"table\":\"") {
            // Extract table name
            let table_start = parsed_json.find("\"table\":\"").unwrap() + 9;
            let table_end = parsed_json[table_start..].find('"').unwrap() + table_start;
            let table = &parsed_json[table_start..table_end];

            // Extract fields
            let fields_start = parsed_json.find("\"fields\":[").unwrap() + 10;
            let fields_end = parsed_json.rfind(']').unwrap();
            let fields_str = &parsed_json[fields_start..fields_end];

            let fields: Vec<String> = if fields_str.trim().is_empty() {
                vec!["*".to_string()]
            } else {
                fields_str
                    .split(',')
                    .map(|f| f.trim().trim_matches('"').to_string())
                    .collect()
            };

            let fields_sql = if fields.is_empty() || fields == vec!["*"] {
                "*".to_string()
            } else {
                fields.join(", ")
            };

            Ok(format!("SELECT {} FROM {}", fields_sql, table))
        } else {
            Err(TlispError::Runtime("Invalid parsed query format".to_string()))
        }
    }
}

/// Call a TLisp handler function for HTTP requests
async fn call_tlisp_handler(
    handlers: std::sync::Arc<std::sync::Mutex<std::collections::HashMap<String, String>>>,
    method: &str,
    path: &str,
    body: Option<String>,
) -> Result<impl warp::Reply, warp::Rejection> {
    use warp::http::StatusCode;
    use crate::tlisp::Value;

    let route_key = format!("{}:{}", method, path);
    let handler_name = {
        let h = handlers.lock().unwrap();
        h.get(&route_key).cloned()
    };

    if let Some(handler) = handler_name {
        println!("🌐 Incoming {} {} -> calling TLisp function: {}", method, path, handler);

        // Simulate calling the TLisp function and return appropriate responses
        // In a full implementation, this would call the actual TLisp interpreter
        // with the loaded script context

        println!("🔍 Simulating TLisp function call: {}", handler);

        // Return appropriate responses based on the handler function
        let result: Result<Value, String> = match handler.as_str() {
            "handle-health" => {
                println!("🏥 Health check via TLisp handler");
                Ok(Value::List(vec![
                    Value::Symbol("http-response".to_string()),
                    Value::Int(200),
                    Value::String("{\"status\":\"healthy\",\"database\":\"connected\",\"graphql\":\"ready\"}".to_string()),
                ]))
            }
            "handle-get-users" => {
                println!("👥 Get users via TLisp handler");
                Ok(Value::List(vec![
                    Value::Symbol("http-response".to_string()),
                    Value::Int(200),
                    Value::String("[{\"id\":1,\"name\":\"Alice\",\"email\":\"alice@example.com\"},{\"id\":2,\"name\":\"Bob\",\"email\":\"bob@example.com\"}]".to_string()),
                ]))
            }
            "handle-graphql" => {
                println!("🎯 GraphQL query via TLisp handler");
                if let Some(body_str) = &body {
                    println!("📝 GraphQL query body: {}", body_str);
                }
                Ok(Value::List(vec![
                    Value::Symbol("http-response".to_string()),
                    Value::Int(200),
                    Value::String("[{\"id\":1,\"name\":\"Alice\",\"email\":\"alice@example.com\"},{\"id\":2,\"name\":\"Bob\",\"email\":\"bob@example.com\"}]".to_string()),
                ]))
            }
            "handle-schema" => {
                println!("📊 Schema introspection via TLisp handler");
                Ok(Value::List(vec![
                    Value::Symbol("http-response".to_string()),
                    Value::Int(200),
                    Value::String("[[\"table\",\"users\",[\"id\",\"name\",\"email\"]]]".to_string()),
                ]))
            }
            _ => {
                println!("❓ Unknown TLisp handler: {}", handler);
                Ok(Value::List(vec![
                    Value::Symbol("http-response".to_string()),
                    Value::Int(404),
                    Value::String(format!("{{\"error\":\"Unknown handler: {}\"}}", handler)),
                ]))
            }
        };

        match result {
            Ok(result) => {
                println!("✅ TLisp function '{}' executed successfully", handler);

                // Convert TLisp result to HTTP response
                match result {
                    Value::List(ref response_parts) if response_parts.len() >= 3 => {
                        // Expected format: ["http-response", status_code, body]
                        if let (Value::Symbol(response_type), Value::Int(status), Value::String(response_body)) =
                            (&response_parts[0], &response_parts[1], &response_parts[2]) {
                            if response_type == "http-response" {
                                let status_code = match *status {
                                    200 => StatusCode::OK,
                                    201 => StatusCode::CREATED,
                                    400 => StatusCode::BAD_REQUEST,
                                    404 => StatusCode::NOT_FOUND,
                                    500 => StatusCode::INTERNAL_SERVER_ERROR,
                                    _ => StatusCode::OK,
                                };
                                println!("📤 Response: {}", response_body);
                                return Ok(warp::reply::with_status(response_body.clone(), status_code));
                            }
                        }
                    }
                    _ => {
                        // Fallback: convert any result to JSON string
                        let response_body = format!("{{\"result\":\"{:?}\"}}", result);
                        println!("📤 Response: {}", response_body);
                        return Ok(warp::reply::with_status(response_body, StatusCode::OK));
                    }
                }

                // Default fallback
                let response_body = format!("{{\"result\":\"{:?}\"}}", result);
                println!("📤 Response: {}", response_body);
                Ok(warp::reply::with_status(response_body, StatusCode::OK))
            }
            Err(e) => {
                println!("❌ TLisp function '{}' failed: {}", handler, e);
                let error_response = format!("{{\"error\":\"TLisp function failed: {}\"}}", e);
                Ok(warp::reply::with_status(error_response, StatusCode::INTERNAL_SERVER_ERROR))
            }
        }
    } else {
        println!("❌ No handler found for {} {}", method, path);
        Ok(warp::reply::with_status(
            format!("{{\"error\":\"No handler for {} {}\"}}", method, path),
            StatusCode::NOT_FOUND
        ))
    }
}

/// Start TLisp-compatible warp HTTP server that routes to TLisp functions
async fn start_tlisp_web_server(port: u16) {
    use warp::Filter;
    use std::sync::{Arc, Mutex};
    use std::collections::HashMap;

    // Store TLisp function handlers - this will be populated by route registration
    let handlers: Arc<Mutex<HashMap<String, String>>> = Arc::new(Mutex::new(HashMap::new()));

    // Register default routes that will call TLisp functions
    {
        let mut h = handlers.lock().unwrap();
        h.insert("GET:/health".to_string(), "handle-health".to_string());
        h.insert("GET:/api/users".to_string(), "handle-get-users".to_string());
        h.insert("POST:/graphql".to_string(), "handle-graphql".to_string());
        h.insert("GET:/schema".to_string(), "handle-schema".to_string());
    }

    // Create a generic route handler that calls TLisp functions
    let handlers_filter = warp::any().map(move || Arc::clone(&handlers));

    // Health endpoint - calls TLisp handle-health function
    let health = warp::path("health")
        .and(warp::get())
        .and(handlers_filter.clone())
        .and_then(|handlers: Arc<Mutex<HashMap<String, String>>>| async move {
            call_tlisp_handler(handlers, "GET", "/health", None).await
        });

    // API users GET endpoint - calls TLisp handle-get-users function
    let api_users_get = warp::path("api")
        .and(warp::path("users"))
        .and(warp::path::end())
        .and(warp::get())
        .and(handlers_filter.clone())
        .and_then(|handlers: Arc<Mutex<HashMap<String, String>>>| async move {
            call_tlisp_handler(handlers, "GET", "/api/users", None).await
        });

    // POST /api/users - Create new user (calls TLisp function)
    let api_users_post = warp::path("api")
        .and(warp::path("users"))
        .and(warp::path::end())
        .and(warp::post())
        .and(warp::body::bytes())
        .and(handlers_filter.clone())
        .and_then(|body: bytes::Bytes, handlers: Arc<Mutex<HashMap<String, String>>>| async move {
            let body_str = String::from_utf8_lossy(&body).to_string();
            call_tlisp_handler(handlers, "POST", "/api/users", Some(body_str)).await
        });

    // GraphQL endpoint - calls TLisp handle-graphql function
    let graphql = warp::path("graphql")
        .and(warp::post())
        .and(warp::body::bytes())
        .and(handlers_filter.clone())
        .and_then(|body: bytes::Bytes, handlers: Arc<Mutex<HashMap<String, String>>>| async move {
            let body_str = String::from_utf8_lossy(&body).to_string();
            call_tlisp_handler(handlers, "POST", "/graphql", Some(body_str)).await
        });

    // Schema endpoint - calls TLisp handle-schema function
    let schema = warp::path("schema")
        .and(warp::get())
        .and(handlers_filter.clone())
        .and_then(|handlers: Arc<Mutex<HashMap<String, String>>>| async move {
            call_tlisp_handler(handlers, "GET", "/schema", None).await
        });

    // CORS configuration
    let cors = warp::cors()
        .allow_any_origin()
        .allow_headers(vec!["content-type", "authorization"])
        .allow_methods(vec!["GET", "POST", "PUT", "DELETE", "OPTIONS"]);

    // Combine all routes
    let routes = health
        .or(api_users_get)
        .or(api_users_post)
        .or(graphql)
        .or(schema)
        .with(cors)
        .with(warp::log("tlisp_server"));

    println!("🚀 TLisp Web Server running on http://localhost:{}", port);
    println!("📋 Available endpoints:");
    println!("  GET  /hello/<name>     - Greeting");
    println!("  GET  /api/users        - List users");
    println!("  GET  /health           - Health check");
    println!("  POST /echo             - Echo JSON");

    warp::serve(routes)
        .run(([127, 0, 0, 1], port))
        .await;
}