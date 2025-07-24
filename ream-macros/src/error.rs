//! Error handling for REAM macros

use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{Error, Result};

/// Macro-specific error types
#[derive(Debug)]
pub enum MacroError {
    /// Invalid TLISP syntax
    InvalidTlispSyntax(String),
    /// Invalid actor definition
    InvalidActorDefinition(String),
    /// Invalid runtime configuration
    InvalidRuntimeConfig(String),
    /// Invalid type bridge definition
    InvalidTypeBridge(String),
    /// Missing required attribute
    MissingAttribute(String),
    /// Unsupported feature
    UnsupportedFeature(String),
}

impl MacroError {
    /// Convert to a syn::Error
    pub fn to_syn_error(&self, span: Span) -> Error {
        let message = match self {
            MacroError::InvalidTlispSyntax(msg) => {
                format!("Invalid TLISP syntax: {}", msg)
            }
            MacroError::InvalidActorDefinition(msg) => {
                format!("Invalid actor definition: {}", msg)
            }
            MacroError::InvalidRuntimeConfig(msg) => {
                format!("Invalid runtime configuration: {}", msg)
            }
            MacroError::InvalidTypeBridge(msg) => {
                format!("Invalid type bridge definition: {}", msg)
            }
            MacroError::MissingAttribute(attr) => {
                format!("Missing required attribute: {}", attr)
            }
            MacroError::UnsupportedFeature(feature) => {
                format!("Unsupported feature: {}", feature)
            }
        };
        Error::new(span, message)
    }
    
    /// Convert to compile-time error tokens
    pub fn to_compile_error(&self, span: Span) -> TokenStream {
        self.to_syn_error(span).to_compile_error()
    }
}

/// Result type for macro operations
pub type MacroResult<T> = std::result::Result<T, MacroError>;

/// Helper function to create TLISP syntax errors
pub fn tlisp_syntax_error(message: &str) -> MacroError {
    MacroError::InvalidTlispSyntax(message.to_string())
}

/// Helper function to create actor definition errors
pub fn actor_definition_error(message: &str) -> MacroError {
    MacroError::InvalidActorDefinition(message.to_string())
}

/// Validate function signature for actor macros
pub fn validate_actor_function(func: &syn::ItemFn) -> MacroResult<()> {
    // Check that function is async
    if func.sig.asyncness.is_none() {
        return Err(actor_definition_error(
            "Actor functions must be async"
        ));
    }
    
    Ok(())
}

/// Validate struct for type bridge macros
pub fn validate_bridge_struct(item_struct: &syn::ItemStruct) -> MacroResult<()> {
    // Check that struct has named fields
    match &item_struct.fields {
        syn::Fields::Named(_) => Ok(()),
        syn::Fields::Unnamed(_) => Err(MacroError::InvalidTypeBridge(
            "Tuple structs are not supported for type bridging".to_string()
        )),
        syn::Fields::Unit => Err(MacroError::InvalidTypeBridge(
            "Unit structs are not supported for type bridging".to_string()
        )),
    }
}
