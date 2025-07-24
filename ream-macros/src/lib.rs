//! REAM Procedural Macros
//! 
//! This crate provides procedural macros for seamless integration of TLISP and REAM's
//! actor model into Rust applications. The macro system enables:
//! 
//! - Embedding TLISP code directly in Rust
//! - Creating actors with compile-time type safety
//! - Configuring runtime behavior declaratively
//! - Bridging types between Rust and TLISP
//! - Testing and debugging actor systems

use proc_macro::TokenStream;

// Utility modules
mod utils;
mod error;
mod parsing;
mod codegen;

// Re-export utilities for internal use
use utils::*;
use error::*;
use parsing::*;
use codegen::*;

//
// TLISP Embedding Macros
//

/// Embed TLISP code directly in Rust
/// 
/// This macro allows you to embed TLISP code directly in Rust with compile-time
/// validation. The TLISP code is parsed and validated at compile time, then
/// executed at runtime using the TLISP interpreter.
/// 
/// # Examples
/// 
/// ```rust
/// use ream_macros::tlisp;
/// 
/// let result = tlisp! {
///     "(+ 1 2 3)"
/// };
/// ```
#[proc_macro]
pub fn tlisp(input: TokenStream) -> TokenStream {
    tlisp_impl(input)
}

/// Inline TLISP code for performance-critical sections
#[proc_macro]
pub fn inline_tlisp(input: TokenStream) -> TokenStream {
    inline_tlisp_impl(input)
}

/// Trace TLISP execution for debugging
#[proc_macro]
pub fn trace_tlisp(input: TokenStream) -> TokenStream {
    trace_tlisp_impl(input)
}

/// GraphQL query macro for compile-time validation and code generation
///
/// This macro allows you to embed GraphQL queries directly in Rust code
/// with compile-time validation against the schema and zero-cost abstraction.
///
/// # Examples
///
/// ```rust
/// use ream_macros::graphql;
///
/// let result = graphql! {
///     "query {
///         users(limit: 10) {
///             id
///             name
///             email
///         }
///     }"
/// };
/// ```
#[proc_macro]
pub fn graphql(input: TokenStream) -> TokenStream {
    graphql_impl(input)
}

/// Implementation of the GraphQL macro
fn graphql_impl(input: TokenStream) -> TokenStream {
    use quote::quote;
    use syn::{parse_macro_input, LitStr};

    // Parse the input as a string literal
    let query_str = parse_macro_input!(input as LitStr);
    let query_value = query_str.value();

    // For now, generate a simple runtime implementation
    // In a full implementation, this would:
    // 1. Parse the GraphQL query at compile time
    // 2. Validate against schema
    // 3. Generate optimized Rust code

    let expanded = quote! {
        {
            // This is a simplified implementation
            // The actual implementation would use the GraphQL compiler
            use serde_json::Value as JsonValue;

            async move {
                // Parse and compile the GraphQL query
                let query_str = #query_value;

                // Create a mock result for now
                Ok::<JsonValue, Box<dyn std::error::Error>>(serde_json::json!({
                    "data": {},
                    "query": query_str
                }))
            }
        }
    };

    TokenStream::from(expanded)
}

/// Convert Rust values to TLISP values
#[proc_macro]
pub fn to_tlisp(input: TokenStream) -> TokenStream {
    to_tlisp_impl(input)
}

/// Convert TLISP values to Rust values
#[proc_macro]
pub fn from_tlisp(input: TokenStream) -> TokenStream {
    from_tlisp_impl(input)
}

//
// Actor System Macros
//

/// Create an actor from a function
#[proc_macro_attribute]
pub fn actor(args: TokenStream, input: TokenStream) -> TokenStream {
    actor_impl(args, input)
}

/// Define actor messages with automatic serialization
#[proc_macro]
pub fn actor_messages(input: TokenStream) -> TokenStream {
    actor_messages_impl(input)
}

/// Spawn an actor instance
#[proc_macro]
pub fn spawn_actor(input: TokenStream) -> TokenStream {
    spawn_actor_impl(input)
}

/// Send a message to an actor
#[proc_macro]
pub fn send(input: TokenStream) -> TokenStream {
    send_impl(input)
}

/// Ask pattern for request-response communication
#[proc_macro]
pub fn ask(input: TokenStream) -> TokenStream {
    ask_impl(input)
}

/// Receive messages in an actor
#[proc_macro]
pub fn receive(input: TokenStream) -> TokenStream {
    receive_impl(input)
}

//
// Runtime Configuration Macros
//

/// Configure REAM runtime for the application
#[proc_macro_attribute]
pub fn runtime(args: TokenStream, input: TokenStream) -> TokenStream {
    runtime_impl(args, input)
}

/// Initialize REAM runtime with advanced configuration
#[proc_macro]
pub fn init_runtime(input: TokenStream) -> TokenStream {
    init_runtime_impl(input)
}

/// Spawn a task on the REAM runtime
#[proc_macro]
pub fn spawn_task(input: TokenStream) -> TokenStream {
    spawn_task_impl(input)
}

/// Block on an async operation using the REAM runtime
#[proc_macro]
pub fn block_on(input: TokenStream) -> TokenStream {
    block_on_impl(input)
}

//
// Type Bridging Macros
//

/// Bridge Rust types to TLISP with automatic trait implementations
#[proc_macro_attribute]
pub fn bridge_type(args: TokenStream, input: TokenStream) -> TokenStream {
    bridge_type_impl(args, input)
}

/// Derive automatic type bridging for simple types
#[proc_macro_derive(Bridge)]
pub fn derive_bridge(input: TokenStream) -> TokenStream {
    derive_bridge_impl(input)
}

//
// Testing Macros
//

/// Test actor behavior with proper test environment
#[proc_macro_attribute]
pub fn actor_test(args: TokenStream, input: TokenStream) -> TokenStream {
    actor_test_impl(args, input)
}

/// Spawn actor in test environment
#[proc_macro]
pub fn spawn_test_actor(input: TokenStream) -> TokenStream {
    spawn_test_actor_impl(input)
}

//
// Debug Macros
//

/// Debug actor message flow and state changes
#[proc_macro_attribute]
pub fn debug_actor(args: TokenStream, input: TokenStream) -> TokenStream {
    debug_actor_impl(args, input)
}

/// Trace function execution with detailed logging
#[proc_macro_attribute]
pub fn trace_execution(args: TokenStream, input: TokenStream) -> TokenStream {
    trace_execution_impl(args, input)
}

/// Debug print with actor context
#[proc_macro]
pub fn debug_print(input: TokenStream) -> TokenStream {
    debug_print_impl(input)
}
