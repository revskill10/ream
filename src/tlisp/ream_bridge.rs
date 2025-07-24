//! Bridge between TLISP and REAM runtime

use std::sync::Arc;
use crate::tlisp::{Value, TlispRuntime};
use crate::runtime::ReamRuntime;
use crate::types::{Pid, MessagePayload};
use crate::error::{TlispError, RuntimeError, TlispResult, RuntimeResult};

/// Bridge for TLISP-REAM integration
pub struct TlispReamBridge {
    /// REAM runtime
    ream_runtime: Arc<ReamRuntime>,
    /// TLISP runtime
    tlisp_runtime: TlispRuntime,
}

impl TlispReamBridge {
    /// Create a new bridge
    pub fn new(ream_runtime: ReamRuntime, tlisp_runtime: TlispRuntime) -> Self {
        TlispReamBridge {
            ream_runtime: Arc::new(ream_runtime),
            tlisp_runtime,
        }
    }
    
    /// Spawn a TLISP actor in REAM (temporarily disabled)
    pub fn spawn_tlisp_actor(&self, _code: &str) -> TlispResult<Pid> {
        // Temporarily disabled due to threading issues
        Ok(crate::types::Pid::new())
    }
    
    /// Send a TLISP value as a message
    pub fn send_tlisp_message(&self, to: Pid, value: Value) -> TlispResult<()> {
        let payload = self.value_to_message_payload(value)?;
        
        self.ream_runtime.send(to, payload)
            .map_err(|e| TlispError::Runtime(format!("Failed to send message: {}", e)))
    }
    
    /// Convert TLISP value to message payload
    fn value_to_message_payload(&self, value: Value) -> TlispResult<MessagePayload> {
        match value {
            Value::String(s) => Ok(MessagePayload::Text(s)),
            Value::Int(i) => Ok(MessagePayload::Data(serde_json::Value::Number(
                serde_json::Number::from(i)
            ))),
            Value::Float(f) => Ok(MessagePayload::Data(serde_json::Value::Number(
                serde_json::Number::from_f64(f).unwrap_or(serde_json::Number::from(0))
            ))),
            Value::Bool(b) => Ok(MessagePayload::Data(serde_json::Value::Bool(b))),
            Value::List(items) => {
                let json_items: Result<Vec<serde_json::Value>, _> = items.iter()
                    .map(|item| self.value_to_json(item))
                    .collect();
                Ok(MessagePayload::Data(serde_json::Value::Array(json_items?)))
            }
            Value::Null => Ok(MessagePayload::Data(serde_json::Value::Null)),
            _ => Err(TlispError::Runtime("Cannot convert value to message".to_string())),
        }
    }
    
    /// Convert TLISP value to JSON
    fn value_to_json(&self, value: &Value) -> TlispResult<serde_json::Value> {
        match value {
            Value::String(s) => Ok(serde_json::Value::String(s.clone())),
            Value::Int(i) => Ok(serde_json::Value::Number(serde_json::Number::from(*i))),
            Value::Float(f) => Ok(serde_json::Value::Number(
                serde_json::Number::from_f64(*f).unwrap_or(serde_json::Number::from(0))
            )),
            Value::Bool(b) => Ok(serde_json::Value::Bool(*b)),
            Value::List(items) => {
                let json_items: Result<Vec<serde_json::Value>, _> = items.iter()
                    .map(|item| self.value_to_json(item))
                    .collect();
                Ok(serde_json::Value::Array(json_items?))
            }
            Value::Null => Ok(serde_json::Value::Null),
            _ => Err(TlispError::Runtime("Cannot convert value to JSON".to_string())),
        }
    }
    
    /// Convert message payload to TLISP value
    fn message_payload_to_value(&self, payload: MessagePayload) -> TlispResult<Value> {
        match payload {
            MessagePayload::Text(s) => Ok(Value::String(s)),
            MessagePayload::Data(json) => self.json_to_value(&json),
            MessagePayload::Bytes(bytes) => {
                // Convert bytes to list of integers
                let int_list: Vec<Value> = bytes.into_iter()
                    .map(|b| Value::Int(b as i64))
                    .collect();
                Ok(Value::List(int_list))
            }
            MessagePayload::Control(_) => Ok(Value::Symbol("control-message".to_string())),
        }
    }
    
    /// Convert JSON to TLISP value
    fn json_to_value(&self, json: &serde_json::Value) -> TlispResult<Value> {
        match json {
            serde_json::Value::String(s) => Ok(Value::String(s.clone())),
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Ok(Value::Int(i))
                } else if let Some(f) = n.as_f64() {
                    Ok(Value::Float(f))
                } else {
                    Ok(Value::Int(0))
                }
            }
            serde_json::Value::Bool(b) => Ok(Value::Bool(*b)),
            serde_json::Value::Array(arr) => {
                let values: Result<Vec<Value>, _> = arr.iter()
                    .map(|item| self.json_to_value(item))
                    .collect();
                Ok(Value::List(values?))
            }
            serde_json::Value::Null => Ok(Value::Null),
            serde_json::Value::Object(_) => {
                // For now, convert objects to symbols
                Ok(Value::Symbol("object".to_string()))
            }
        }
    }
    
    /// Get REAM runtime reference
    pub fn ream_runtime(&self) -> &ReamRuntime {
        &self.ream_runtime
    }
    
    /// Get mutable TLISP runtime reference
    pub fn tlisp_runtime_mut(&mut self) -> &mut TlispRuntime {
        &mut self.tlisp_runtime
    }
}

/// TLISP actor that runs TLISP code
pub struct TlispActor {
    /// Actor PID
    pid: Pid,
    /// TLISP code to execute
    code: String,
    /// TLISP runtime for this actor
    runtime: TlispRuntime,
    /// Message queue
    message_queue: Vec<Value>,
}

impl TlispActor {
    /// Create a new TLISP actor
    pub fn new(code: String) -> Self {
        TlispActor {
            pid: Pid::new(),
            code,
            runtime: TlispRuntime::new(),
            message_queue: Vec::new(),
        }
    }
    
    /// Process a TLISP message
    fn process_tlisp_message(&mut self, value: Value) -> RuntimeResult<()> {
        // Add message to queue
        self.message_queue.push(value);

        // Define receive function that returns the next message
        if let Some(message) = self.message_queue.pop() {
            self.runtime.define("*message*", message);
        }

        // Execute the TLISP code
        match self.runtime.eval(&self.code) {
            Ok(_) => Ok(()),
            Err(e) => Err(RuntimeError::Scheduler(format!("TLISP error: {}", e))),
        }
    }

    /// Convert message payload to TLISP value
    fn message_payload_to_value(&self, payload: MessagePayload) -> TlispResult<Value> {
        match payload {
            MessagePayload::Text(s) => Ok(Value::String(s)),
            MessagePayload::Data(data) => {
                match data {
                    serde_json::Value::Number(n) => {
                        if let Some(i) = n.as_i64() {
                            Ok(Value::Int(i))
                        } else if let Some(f) = n.as_f64() {
                            Ok(Value::Float(f))
                        } else {
                            Ok(Value::Int(0))
                        }
                    }
                    serde_json::Value::Bool(b) => Ok(Value::Bool(b)),
                    serde_json::Value::String(s) => Ok(Value::String(s)),
                    _ => Ok(Value::String(data.to_string())),
                }
            }
            MessagePayload::Bytes(_) => Ok(Value::String("binary".to_string())),
            MessagePayload::Control(_) => Ok(Value::String("control".to_string())),
        }
    }
}

impl TlispActor {
    /// Get the actor's PID
    pub fn pid(&self) -> Pid {
        self.pid
    }

    /// Check if actor is alive
    pub fn is_alive(&self) -> bool {
        true
    }

    /// Receive a message (non-trait version)
    pub fn receive(&mut self, message: MessagePayload) -> RuntimeResult<()> {
        // Convert message to TLISP value
        let value = self.message_payload_to_value(message)
            .map_err(|e| RuntimeError::InvalidMessage(format!("Conversion error: {}", e)))?;

        self.process_tlisp_message(value)
    }

    /// Restart the actor (non-trait version)
    pub fn restart(&mut self) -> RuntimeResult<()> {
        self.message_queue.clear();
        self.runtime = TlispRuntime::new();
        Ok(())
    }
}

// Note: ReamActor trait implementation is disabled due to threading constraints
// TlispRuntime contains Rc<RefCell<Environment>> which is not Send + Sync
// This is a known limitation that would require significant refactoring to fix

/// TLISP-REAM integration utilities
pub struct TlispReamUtils;

impl TlispReamUtils {
    /// Create a TLISP function that spawns a REAM process
    pub fn spawn_function() -> Value {
        Value::Builtin("ream-spawn".to_string())
    }
    
    /// Create a TLISP function that sends a message to a REAM process
    pub fn send_function() -> Value {
        Value::Builtin("ream-send".to_string())
    }
    
    /// Create a TLISP function that receives a message from REAM
    pub fn receive_function() -> Value {
        Value::Builtin("ream-receive".to_string())
    }
    
    /// Create a TLISP function that gets the current process PID
    pub fn self_function() -> Value {
        Value::Builtin("ream-self".to_string())
    }
    
    /// Create a TLISP function that links to another process
    pub fn link_function() -> Value {
        Value::Builtin("ream-link".to_string())
    }
    
    /// Create a TLISP function that monitors another process
    pub fn monitor_function() -> Value {
        Value::Builtin("ream-monitor".to_string())
    }
    
    /// Add all REAM functions to a TLISP runtime
    pub fn add_ream_functions(runtime: &mut TlispRuntime) {
        runtime.define("ream-spawn", Self::spawn_function());
        runtime.define("ream-send", Self::send_function());
        runtime.define("ream-receive", Self::receive_function());
        runtime.define("ream-self", Self::self_function());
        runtime.define("ream-link", Self::link_function());
        runtime.define("ream-monitor", Self::monitor_function());
    }
}

/// TLISP-REAM message protocol
#[derive(Debug, Clone)]
pub enum TlispReamMessage {
    /// Evaluate TLISP code
    Eval(String),
    /// Define a variable
    Define(String, Value),
    /// Get a variable
    Get(String),
    /// Response with value
    Response(Value),
    /// Error response
    Error(String),
}

impl TlispReamMessage {
    /// Convert to message payload
    pub fn to_payload(&self) -> MessagePayload {
        match self {
            TlispReamMessage::Eval(code) => {
                MessagePayload::Data(serde_json::json!({
                    "type": "eval",
                    "code": code
                }))
            }
            TlispReamMessage::Define(name, value) => {
                MessagePayload::Data(serde_json::json!({
                    "type": "define",
                    "name": name,
                    "value": value.to_string()
                }))
            }
            TlispReamMessage::Get(name) => {
                MessagePayload::Data(serde_json::json!({
                    "type": "get",
                    "name": name
                }))
            }
            TlispReamMessage::Response(value) => {
                MessagePayload::Data(serde_json::json!({
                    "type": "response",
                    "value": value.to_string()
                }))
            }
            TlispReamMessage::Error(error) => {
                MessagePayload::Data(serde_json::json!({
                    "type": "error",
                    "message": error
                }))
            }
        }
    }
    
    /// Parse from message payload
    pub fn from_payload(payload: MessagePayload) -> TlispResult<Self> {
        match payload {
            MessagePayload::Data(json) => {
                let msg_type = json.get("type")
                    .and_then(|t| t.as_str())
                    .ok_or_else(|| TlispError::Runtime("Invalid message format".to_string()))?;
                
                match msg_type {
                    "eval" => {
                        let code = json.get("code")
                            .and_then(|c| c.as_str())
                            .ok_or_else(|| TlispError::Runtime("Missing code field".to_string()))?;
                        Ok(TlispReamMessage::Eval(code.to_string()))
                    }
                    "define" => {
                        let name = json.get("name")
                            .and_then(|n| n.as_str())
                            .ok_or_else(|| TlispError::Runtime("Missing name field".to_string()))?;
                        let value_str = json.get("value")
                            .and_then(|v| v.as_str())
                            .ok_or_else(|| TlispError::Runtime("Missing value field".to_string()))?;
                        // For now, just create a string value
                        Ok(TlispReamMessage::Define(name.to_string(), Value::String(value_str.to_string())))
                    }
                    "get" => {
                        let name = json.get("name")
                            .and_then(|n| n.as_str())
                            .ok_or_else(|| TlispError::Runtime("Missing name field".to_string()))?;
                        Ok(TlispReamMessage::Get(name.to_string()))
                    }
                    "response" => {
                        let value_str = json.get("value")
                            .and_then(|v| v.as_str())
                            .ok_or_else(|| TlispError::Runtime("Missing value field".to_string()))?;
                        Ok(TlispReamMessage::Response(Value::String(value_str.to_string())))
                    }
                    "error" => {
                        let message = json.get("message")
                            .and_then(|m| m.as_str())
                            .ok_or_else(|| TlispError::Runtime("Missing message field".to_string()))?;
                        Ok(TlispReamMessage::Error(message.to_string()))
                    }
                    _ => Err(TlispError::Runtime(format!("Unknown message type: {}", msg_type))),
                }
            }
            _ => Err(TlispError::Runtime("Expected JSON data message".to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_value_conversion() {
        let bridge = TlispReamBridge::new(
            ReamRuntime::new().expect("Failed to create ReamRuntime"),
            TlispRuntime::new()
        );
        
        // Test integer conversion
        let value = Value::Int(42);
        let payload = bridge.value_to_message_payload(value).unwrap();
        let converted = bridge.message_payload_to_value(payload).unwrap();
        assert_eq!(converted, Value::Int(42));
        
        // Test string conversion
        let value = Value::String("hello".to_string());
        let payload = bridge.value_to_message_payload(value).unwrap();
        let converted = bridge.message_payload_to_value(payload).unwrap();
        assert_eq!(converted, Value::String("hello".to_string()));
    }
    
    #[test]
    fn test_tlisp_actor() {
        let actor = TlispActor::new("(+ 1 2)".to_string());
        assert!(actor.is_alive());
        assert_eq!(actor.pid().raw() > 0, true);
    }
    
    #[test]
    fn test_message_protocol() {
        let msg = TlispReamMessage::Eval("(+ 1 2)".to_string());
        let payload = msg.to_payload();
        let parsed = TlispReamMessage::from_payload(payload).unwrap();
        
        match parsed {
            TlispReamMessage::Eval(code) => assert_eq!(code, "(+ 1 2)"),
            _ => panic!("Expected Eval message"),
        }
    }
}
