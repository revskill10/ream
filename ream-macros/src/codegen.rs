//! Code generation utilities and macro implementations

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{parse_macro_input, LitStr, Expr, ItemFn, ItemStruct, ItemEnum};

use crate::{
    utils::{validate_tlisp_code, generate_debug_info},
    error::{validate_actor_function, validate_bridge_struct},
    parsing::{RuntimeConfig, ActorConfig, MessageEnum, SendExpression, AskExpression, TypeBridgeConfig},
};

//
// TLISP Macro Implementations
//

pub fn tlisp_impl(input: TokenStream) -> TokenStream {
    let tlisp_code = parse_macro_input!(input as LitStr);
    let code = tlisp_code.value();

    // Validate TLISP code at compile time with dependent type support
    if let Err(e) = validate_tlisp_code(&code) {
        return e.to_compile_error().into();
    }

    // Generate debug information in debug builds
    let debug_info = generate_debug_info("tlisp!", &code);

    let expanded = quote! {
        {
            #debug_info

            use ream::tlisp::TlispInterpreter;

            // Get or create interpreter with dependent type support
            let mut interpreter = TlispInterpreter::new();

            // Compile and execute TLISP code (includes parsing and type checking)
            interpreter.eval(#code)
                .map_err(|e| ream::error::RuntimeError::TlispError(e.to_string()))
        }
    };

    TokenStream::from(expanded)
}

pub fn inline_tlisp_impl(input: TokenStream) -> TokenStream {
    // For now, fall back to regular tlisp! implementation
    tlisp_impl(input)
}

pub fn trace_tlisp_impl(input: TokenStream) -> TokenStream {
    let tlisp_code = parse_macro_input!(input as LitStr);
    let code = tlisp_code.value();
    
    if let Err(e) = validate_tlisp_code(&code) {
        return e.to_compile_error().into();
    }
    
    let expanded = quote! {
        {
            use ream::tlisp::TlispInterpreter;
            
            let mut interpreter = TlispInterpreter::new();
            
            // Enable tracing (placeholder for now)
            println!("TLISP Trace: Executing {}", #code);
            
            let result = interpreter.eval(#code)
                .map_err(|e| ream::error::RuntimeError::TlispError(e.to_string()));
            
            println!("TLISP Trace: Result {:?}", result);
            
            result
        }
    };
    
    TokenStream::from(expanded)
}

pub fn to_tlisp_impl(input: TokenStream) -> TokenStream {
    let expr = parse_macro_input!(input as Expr);
    
    let expanded = quote! {
        {
            use ream::tlisp::types::ToTlisp;
            Ok((#expr).to_tlisp())
        }
    };
    
    TokenStream::from(expanded)
}

pub fn from_tlisp_impl(input: TokenStream) -> TokenStream {
    let expr = parse_macro_input!(input as Expr);
    
    let expanded = quote! {
        {
            use ream::tlisp::types::FromTlisp;
            <_>::from_tlisp(&#expr)
        }
    };
    
    TokenStream::from(expanded)
}

//
// Actor Macro Implementations
//

pub fn actor_impl(args: TokenStream, input: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(input as ItemFn);
    let _config = if args.is_empty() {
        ActorConfig::default()
    } else {
        match syn::parse(args) {
            Ok(config) => config,
            Err(e) => return e.to_compile_error().into(),
        }
    };
    
    // Validate the function signature
    if let Err(e) = validate_actor_function(&input_fn) {
        return e.to_compile_error(Span::call_site()).into();
    }
    
    let actor_name = &input_fn.sig.ident;
    let fn_block = &input_fn.block;
    let vis = &input_fn.vis;
    
    let expanded = quote! {
        #[derive(Debug)]
        #vis struct #actor_name {
            _phantom: std::marker::PhantomData<()>,
        }
        
        impl #actor_name {
            pub fn new() -> Self {
                Self {
                    _phantom: std::marker::PhantomData,
                }
            }
        }
        
        impl ream::runtime::actor::ReamActor for #actor_name {
            fn receive(&mut self, _message: ream::types::MessagePayload) -> ream::error::RuntimeResult<()> {
                // Basic message handling - would be expanded in full implementation
                Ok(())
            }

            fn pid(&self) -> ream::types::Pid {
                ream::types::Pid::new()
            }

            fn restart(&mut self) -> ream::error::RuntimeResult<()> {
                // Restart implementation
                Ok(())
            }
        }
    };
    
    TokenStream::from(expanded)
}

pub fn actor_messages_impl(input: TokenStream) -> TokenStream {
    let message_enum = parse_macro_input!(input as MessageEnum);
    let enum_name = &message_enum.name;
    let variants = &message_enum.variants;
    
    let expanded = quote! {
        #[derive(Debug, Clone)]
        #[derive(serde::Serialize, serde::Deserialize)]
        pub enum #enum_name {
            #variants
        }
        
        impl ream::runtime::message::MessageTrait for #enum_name {
            fn serialize(&self) -> Vec<u8> {
                bincode::serialize(self).unwrap_or_default()
            }
            
            fn deserialize(data: &[u8]) -> Result<Self, ream::error::RuntimeError> {
                bincode::deserialize(data)
                    .map_err(|e| ream::error::RuntimeError::SerializationError(e.to_string()))
            }
        }
    };
    
    TokenStream::from(expanded)
}

pub fn spawn_actor_impl(input: TokenStream) -> TokenStream {
    let actor_expr = parse_macro_input!(input as Expr);
    
    let expanded = quote! {
        {
            use ream::runtime::ReamRuntime;

            let runtime = ReamRuntime::current();
            runtime.spawn_actor(|| {
                tokio::spawn(async move {
                    let _actor = #actor_expr;
                    // Actor execution would go here
                })
            })
        }
    };
    
    TokenStream::from(expanded)
}

pub fn send_impl(input: TokenStream) -> TokenStream {
    let send_expr = parse_macro_input!(input as SendExpression);
    let actor = &send_expr.actor;
    let message = &send_expr.message;
    
    let expanded = quote! {
        {
            use ream::runtime::ReamRuntime;
            
            let runtime = ReamRuntime::current();
            let actor_ref = #actor;
            let message = #message;
            
            runtime.send_message(actor_ref, message)
        }
    };
    
    TokenStream::from(expanded)
}

pub fn ask_impl(input: TokenStream) -> TokenStream {
    let ask_expr = parse_macro_input!(input as AskExpression);
    let actor = &ask_expr.actor;
    let message = &ask_expr.message;
    
    let expanded = quote! {
        {
            use ream::runtime::ReamRuntime;
            
            let runtime = ReamRuntime::current();
            let actor_ref = #actor;
            let message = #message;
            
            runtime.ask_actor(actor_ref, message).await
        }
    };
    
    TokenStream::from(expanded)
}

pub fn receive_impl(_input: TokenStream) -> TokenStream {
    let expanded = quote! {
        {
            use ream::runtime::actor::ActorContext;
            ActorContext::current().receive().await
        }
    };

    TokenStream::from(expanded)
}

//
// Runtime Macro Implementations
//

pub fn runtime_impl(args: TokenStream, input: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(input as ItemFn);
    let config = if args.is_empty() {
        RuntimeConfig::default()
    } else {
        match syn::parse(args) {
            Ok(config) => config,
            Err(e) => return e.to_compile_error().into(),
        }
    };

    let fn_name = &input_fn.sig.ident;
    let fn_vis = &input_fn.vis;
    let fn_inputs = &input_fn.sig.inputs;
    let fn_output = &input_fn.sig.output;
    let fn_block = &input_fn.block;
    let fn_asyncness = &input_fn.sig.asyncness;

    let max_actors = config.max_actors.unwrap_or(1_000_000);
    let _worker_threads = config.worker_threads.unwrap_or(4);
    let _gc_interval = config.gc_interval.unwrap_or(1);
    let _distributed = config.distributed.unwrap_or(false);

    let expanded = quote! {
        #fn_vis #fn_asyncness fn #fn_name(#fn_inputs) #fn_output {
            use ream::runtime::ReamRuntime;
            use ream::types::ReamConfig;

            // Initialize runtime with configuration
            let config = ReamConfig {
                max_processes: #max_actors,
                scheduler_quantum: 1000,
                max_message_queue_size: 10_000,
                gc_threshold: 64 * 1024 * 1024,
                enable_jit: true,
                jit_opt_level: 2,
            };

            let runtime = ReamRuntime::with_config(config);
            runtime.start();
            ReamRuntime::set_current(runtime);

            // Execute the original function
            #fn_block
        }
    };

    TokenStream::from(expanded)
}

pub fn init_runtime_impl(_input: TokenStream) -> TokenStream {
    let expanded = quote! {
        {
            use ream::runtime::{ReamRuntime, RuntimeConfig};

            let config = RuntimeConfig::default();
            let runtime = ReamRuntime::with_config(config);
            runtime.start();
            ReamRuntime::set_current(runtime);
        }
    };

    TokenStream::from(expanded)
}

pub fn spawn_task_impl(input: TokenStream) -> TokenStream {
    let task_expr = parse_macro_input!(input as Expr);

    let expanded = quote! {
        {
            use ream::runtime::ReamRuntime;

            let runtime = ReamRuntime::current();
            runtime.spawn_task(#task_expr)
        }
    };

    TokenStream::from(expanded)
}

pub fn block_on_impl(input: TokenStream) -> TokenStream {
    let future_expr = parse_macro_input!(input as Expr);

    let expanded = quote! {
        {
            use ream::runtime::ReamRuntime;

            let runtime = ReamRuntime::current();
            runtime.block_on(#future_expr)
        }
    };

    TokenStream::from(expanded)
}

//
// Type Bridge Macro Implementations
//

pub fn bridge_type_impl(args: TokenStream, input: TokenStream) -> TokenStream {
    let _config = if args.is_empty() {
        TypeBridgeConfig::default()
    } else {
        match syn::parse(args) {
            Ok(config) => config,
            Err(e) => return e.to_compile_error().into(),
        }
    };

    // Try to parse as struct first, then enum
    if let Ok(item_struct) = syn::parse::<ItemStruct>(input.clone()) {
        if let Err(e) = validate_bridge_struct(&item_struct) {
            return e.to_compile_error(Span::call_site()).into();
        }

        return generate_struct_bridge_impl(&item_struct);
    }

    if let Ok(item_enum) = syn::parse::<ItemEnum>(input.clone()) {
        return generate_enum_bridge_impl(&item_enum);
    }

    // If neither struct nor enum, return error
    let error = syn::Error::new(Span::call_site(), "bridge_type can only be applied to structs and enums");
    error.to_compile_error().into()
}

fn generate_struct_bridge_impl(item_struct: &ItemStruct) -> TokenStream {
    let struct_name = &item_struct.ident;

    let expanded = quote! {
        // Keep the original struct
        #item_struct

        // Implement ToTlisp trait
        impl ream::tlisp::types::ToTlisp for #struct_name {
            fn to_tlisp(&self) -> ream::tlisp::Value {
                // Basic implementation - would be expanded based on fields
                ream::tlisp::Value::Unit
            }
        }

        // Implement FromTlisp trait
        impl ream::tlisp::types::FromTlisp for #struct_name {
            fn from_tlisp(_value: &ream::tlisp::Value) -> Result<Self, ream::error::TypeError> {
                // Basic implementation - would be expanded based on fields
                Err(ream::error::TypeError::TypeMismatch(
                    stringify!(#struct_name).to_string(),
                    "unknown".to_string()
                ))
            }
        }
    };

    TokenStream::from(expanded)
}

fn generate_enum_bridge_impl(item_enum: &ItemEnum) -> TokenStream {
    let enum_name = &item_enum.ident;

    let expanded = quote! {
        // Keep the original enum
        #item_enum

        // Implement ToTlisp trait
        impl ream::tlisp::types::ToTlisp for #enum_name {
            fn to_tlisp(&self) -> ream::tlisp::Value {
                // Basic implementation - would be expanded based on variants
                ream::tlisp::Value::Unit
            }
        }

        // Implement FromTlisp trait
        impl ream::tlisp::types::FromTlisp for #enum_name {
            fn from_tlisp(_value: &ream::tlisp::Value) -> Result<Self, ream::error::TypeError> {
                // Basic implementation - would be expanded based on variants
                Err(ream::error::TypeError::TypeMismatch(
                    stringify!(#enum_name).to_string(),
                    "unknown".to_string()
                ))
            }
        }
    };

    TokenStream::from(expanded)
}

pub fn derive_bridge_impl(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);

    match input.data {
        syn::Data::Struct(_) => {
            let struct_name = &input.ident;

            let expanded = quote! {
                impl ream::tlisp::types::ToTlisp for #struct_name {
                    fn to_tlisp(&self) -> ream::tlisp::Value {
                        ream::tlisp::Value::Unit
                    }
                }

                impl ream::tlisp::types::FromTlisp for #struct_name {
                    fn from_tlisp(_value: &ream::tlisp::Value) -> Result<Self, ream::error::TypeError> {
                        Err(ream::error::TypeError::TypeMismatch(
                            stringify!(#struct_name).to_string(),
                            "unknown".to_string()
                        ))
                    }
                }
            };

            TokenStream::from(expanded)
        }
        _ => {
            let error = syn::Error::new(Span::call_site(), "Bridge derive only supports structs");
            error.to_compile_error().into()
        }
    }
}

//
// Test Macro Implementations
//

pub fn actor_test_impl(_args: TokenStream, input: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(input as ItemFn);
    let fn_name = &input_fn.sig.ident;
    let fn_block = &input_fn.block;
    let fn_vis = &input_fn.vis;

    let expanded = quote! {
        #[tokio::test]
        #fn_vis async fn #fn_name() {
            use ream::runtime::{ReamRuntime, RuntimeConfig};

            // Create isolated test runtime
            let config = RuntimeConfig::default();
            let test_runtime = ReamRuntime::with_config(config);
            test_runtime.start();
            ReamRuntime::set_current(test_runtime.clone());

            // Run the test
            let test_result = async move #fn_block.await;

            // Cleanup test environment
            test_runtime.shutdown();

            test_result
        }
    };

    TokenStream::from(expanded)
}

pub fn spawn_test_actor_impl(input: TokenStream) -> TokenStream {
    let actor_expr = parse_macro_input!(input as Expr);

    let expanded = quote! {
        {
            use ream::runtime::ReamRuntime;

            let runtime = ReamRuntime::current();
            runtime.spawn_actor(#actor_expr)
        }
    };

    TokenStream::from(expanded)
}

//
// Debug Macro Implementations
//

pub fn debug_actor_impl(_args: TokenStream, input: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(input as ItemFn);
    let fn_name = &input_fn.sig.ident;
    let fn_vis = &input_fn.vis;

    let expanded = quote! {
        #[derive(Debug)]
        #fn_vis struct #fn_name {
            debug_info: ream::debug::ActorDebugInfo,
        }

        impl #fn_name {
            pub fn new() -> Self {
                Self {
                    debug_info: ream::debug::ActorDebugInfo::new(stringify!(#fn_name)),
                }
            }
        }

        impl ream::runtime::actor::ReamActor for #fn_name {
            fn receive(&mut self, message: ream::types::MessagePayload) -> ream::error::RuntimeResult<()> {
                // Debug logging
                println!("Actor {} received message: {:?}", stringify!(#fn_name), message);

                // Update debug statistics
                self.debug_info.message_count += 1;

                Ok(())
            }

            fn pid(&self) -> ream::types::Pid {
                self.debug_info.actor_id
            }

            fn restart(&mut self) -> ream::error::RuntimeResult<()> {
                // Reset debug info on restart
                self.debug_info = ream::debug::ActorDebugInfo::new(stringify!(#fn_name));
                Ok(())
            }
        }
    };

    TokenStream::from(expanded)
}

pub fn trace_execution_impl(_args: TokenStream, input: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(input as ItemFn);
    let fn_name = &input_fn.sig.ident;
    let fn_inputs = &input_fn.sig.inputs;
    let fn_output = &input_fn.sig.output;
    let fn_block = &input_fn.block;
    let fn_vis = &input_fn.vis;
    let fn_asyncness = &input_fn.sig.asyncness;

    let expanded = if fn_asyncness.is_some() {
        quote! {
            #fn_vis #fn_asyncness fn #fn_name(#fn_inputs) #fn_output {
                println!("Function {} started", stringify!(#fn_name));
                let start_time = std::time::Instant::now();

                let result = async move #fn_block.await;

                let execution_time = start_time.elapsed();
                println!("Function {} completed in {:?}", stringify!(#fn_name), execution_time);

                result
            }
        }
    } else {
        quote! {
            #fn_vis fn #fn_name(#fn_inputs) #fn_output {
                println!("Function {} started", stringify!(#fn_name));
                let start_time = std::time::Instant::now();

                let result = #fn_block;

                let execution_time = start_time.elapsed();
                println!("Function {} completed in {:?}", stringify!(#fn_name), execution_time);

                result
            }
        }
    };

    TokenStream::from(expanded)
}

pub fn debug_print_impl(input: TokenStream) -> TokenStream {
    let format_args = parse_macro_input!(input as Expr);

    let expanded = quote! {
        {
            #[cfg(debug_assertions)]
            {
                println!("[DEBUG] {}", #format_args);
            }
        }
    };

    TokenStream::from(expanded)
}
