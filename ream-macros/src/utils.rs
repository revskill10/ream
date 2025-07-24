//! Utility functions for macro implementation

use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{Error, Result};

/// Generate a unique identifier with a given prefix
pub fn generate_unique_ident(prefix: &str) -> syn::Ident {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    
    let id = COUNTER.fetch_add(1, Ordering::SeqCst);
    syn::Ident::new(&format!("{}_{}", prefix, id), Span::call_site())
}

/// Create a compile-time error with a message
pub fn compile_error(message: &str) -> TokenStream {
    let error = Error::new(Span::call_site(), message);
    error.to_compile_error()
}

/// Validate that a string is valid TLISP code
pub fn validate_tlisp_code(code: &str) -> Result<()> {
    // Basic validation - check for balanced parentheses
    let mut depth = 0;
    for ch in code.chars() {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth < 0 {
                    return Err(Error::new(
                        Span::call_site(),
                        "Unmatched closing parenthesis in TLISP code"
                    ));
                }
            }
            _ => {}
        }
    }
    
    if depth != 0 {
        return Err(Error::new(
            Span::call_site(),
            "Unmatched opening parenthesis in TLISP code"
        ));
    }
    
    Ok(())
}

/// Generate runtime error handling code
pub fn generate_error_handling() -> TokenStream {
    quote! {
        .map_err(|e| ream::error::RuntimeError::TlispError(e.to_string()))
    }
}

/// Generate code to get current runtime
pub fn generate_runtime_access() -> TokenStream {
    quote! {
        {
            use ream::runtime::ReamRuntime;
            ReamRuntime::current()
        }
    }
}

/// Generate code to create TLISP interpreter
pub fn generate_interpreter_creation() -> TokenStream {
    quote! {
        {
            use ream::tlisp::TlispInterpreter;
            TlispInterpreter::new()
        }
    }
}

/// Generate debug information for macro expansion
pub fn generate_debug_info(macro_name: &str, input: &str) -> TokenStream {
    quote! {
        #[cfg(debug_assertions)]
        {
            eprintln!("REAM Macro Debug: {} expanded with input: {}", #macro_name, #input);
        }
    }
}
