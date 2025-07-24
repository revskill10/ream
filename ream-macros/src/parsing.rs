//! Parsing utilities for REAM macros

use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    token::Comma,
    Expr, Ident, LitStr, Result, Token,
};

/// Configuration for runtime macro
#[derive(Debug, Default)]
pub struct RuntimeConfig {
    pub max_actors: Option<usize>,
    pub worker_threads: Option<usize>,
    pub gc_interval: Option<u64>,
    pub distributed: Option<bool>,
}

impl Parse for RuntimeConfig {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut config = RuntimeConfig::default();
        
        while !input.is_empty() {
            let key: Ident = input.parse()?;
            input.parse::<Token![=]>()?;
            
            match key.to_string().as_str() {
                "actors" => {
                    let value: syn::LitInt = input.parse()?;
                    config.max_actors = Some(value.base10_parse()?);
                }
                "threads" => {
                    let value: syn::LitInt = input.parse()?;
                    config.worker_threads = Some(value.base10_parse()?);
                }
                "gc_interval" => {
                    let value: syn::LitInt = input.parse()?;
                    config.gc_interval = Some(value.base10_parse()?);
                }
                "distributed" => {
                    let value: syn::LitBool = input.parse()?;
                    config.distributed = Some(value.value);
                }
                _ => {
                    return Err(syn::Error::new(
                        key.span(),
                        format!("Unknown runtime configuration key: {}", key),
                    ));
                }
            }
            
            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }
        
        Ok(config)
    }
}

/// Actor configuration
#[derive(Debug, Default)]
pub struct ActorConfig {
    pub priority: Option<String>,
    pub mailbox_size: Option<usize>,
    pub restart_strategy: Option<String>,
}

impl Parse for ActorConfig {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut config = ActorConfig::default();
        
        while !input.is_empty() {
            let key: Ident = input.parse()?;
            input.parse::<Token![=]>()?;
            
            match key.to_string().as_str() {
                "priority" => {
                    let value: LitStr = input.parse()?;
                    config.priority = Some(value.value());
                }
                "mailbox_size" => {
                    let value: syn::LitInt = input.parse()?;
                    config.mailbox_size = Some(value.base10_parse()?);
                }
                "restart_strategy" => {
                    let value: LitStr = input.parse()?;
                    config.restart_strategy = Some(value.value());
                }
                _ => {
                    return Err(syn::Error::new(
                        key.span(),
                        format!("Unknown actor configuration key: {}", key),
                    ));
                }
            }
            
            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }
        
        Ok(config)
    }
}

/// Message enum definition for actor_messages! macro
#[derive(Debug)]
pub struct MessageEnum {
    pub name: Ident,
    pub variants: Punctuated<syn::Variant, Comma>,
}

impl Parse for MessageEnum {
    fn parse(input: ParseStream) -> Result<Self> {
        input.parse::<Token![enum]>()?;
        let name: Ident = input.parse()?;
        
        let content;
        syn::braced!(content in input);
        let variants = content.parse_terminated(syn::Variant::parse, Token![,])?;
        
        Ok(MessageEnum { name, variants })
    }
}

/// Send expression for send! macro
#[derive(Debug)]
pub struct SendExpression {
    pub actor: Expr,
    pub message: Expr,
}

impl Parse for SendExpression {
    fn parse(input: ParseStream) -> Result<Self> {
        let actor = input.parse()?;
        input.parse::<Token![,]>()?;
        let message = input.parse()?;
        
        Ok(SendExpression { actor, message })
    }
}

/// Ask expression for ask! macro
#[derive(Debug)]
pub struct AskExpression {
    pub actor: Expr,
    pub message: Expr,
}

impl Parse for AskExpression {
    fn parse(input: ParseStream) -> Result<Self> {
        let actor = input.parse()?;
        input.parse::<Token![,]>()?;
        let message = input.parse()?;
        
        Ok(AskExpression { actor, message })
    }
}

/// Type bridge configuration
#[derive(Debug, Default)]
pub struct TypeBridgeConfig {
    pub derive_debug: bool,
    pub derive_clone: bool,
    pub custom_serialization: bool,
}

impl Parse for TypeBridgeConfig {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut config = TypeBridgeConfig::default();
        
        while !input.is_empty() {
            let key: Ident = input.parse()?;
            
            match key.to_string().as_str() {
                "debug" => config.derive_debug = true,
                "clone" => config.derive_clone = true,
                "custom_serialization" => config.custom_serialization = true,
                _ => {
                    return Err(syn::Error::new(
                        key.span(),
                        format!("Unknown type bridge option: {}", key),
                    ));
                }
            }
            
            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }
        
        Ok(config)
    }
}
