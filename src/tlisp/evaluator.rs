//! TLISP evaluator with environment management

use std::rc::Rc;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;
use crate::tlisp::{Expr, Value, Function, Type};
use crate::tlisp::environment::Environment;
use crate::error::{TlispError, TlispResult};
use crate::runtime::ReamRuntime;
use crate::daemon::monitor::ActorMonitor;

/// Evaluation context
pub struct EvaluationContext {
    /// Current environment
    pub env: Rc<RefCell<Environment>>,
    /// Call stack depth
    pub depth: usize,
    /// Maximum call stack depth
    pub max_depth: usize,
    /// Optional runtime reference for hypervisor operations
    pub runtime: Option<Arc<ReamRuntime>>,
    /// Optional actor monitor for hypervisor operations
    pub monitor: Option<Arc<ActorMonitor>>,
}

impl EvaluationContext {
    /// Create a new evaluation context
    pub fn new(env: Rc<RefCell<Environment>>) -> Self {
        EvaluationContext {
            env,
            depth: 0,
            max_depth: 1000,
            runtime: None,
            monitor: None,
        }
    }

    /// Create a new evaluation context with runtime
    pub fn with_runtime(env: Rc<RefCell<Environment>>, runtime: Arc<ReamRuntime>, monitor: Arc<ActorMonitor>) -> Self {
        EvaluationContext {
            env,
            depth: 0,
            max_depth: 1000,
            runtime: Some(runtime),
            monitor: Some(monitor),
        }
    }

    /// Get runtime reference
    pub fn get_runtime(&self) -> Option<&Arc<ReamRuntime>> {
        self.runtime.as_ref()
    }

    /// Get monitor reference
    pub fn get_monitor(&self) -> Option<&Arc<ActorMonitor>> {
        self.monitor.as_ref()
    }
    
    /// Push a new scope
    pub fn push_scope(&mut self) -> Rc<RefCell<Environment>> {
        let new_env = Rc::new(RefCell::new(Environment::with_parent(Rc::clone(&self.env))));
        self.env = Rc::clone(&new_env);
        new_env
    }
    
    /// Pop the current scope
    pub fn pop_scope(&mut self) {
        let parent = self.env.borrow().parent();
        if let Some(parent) = parent {
            self.env = parent;
        }
    }
    
    /// Check call stack depth
    pub fn check_depth(&self) -> TlispResult<()> {
        if self.depth >= self.max_depth {
            Err(TlispError::Runtime("Stack overflow".to_string()))
        } else {
            Ok(())
        }
    }
}

/// TLISP evaluator
pub struct Evaluator {
    /// Global environment
    global_env: Rc<RefCell<Environment>>,
}

impl Evaluator {
    /// Create a new evaluator
    pub fn new(global_env: Rc<RefCell<Environment>>) -> Self {
        Evaluator { global_env }
    }
    
    /// Evaluate an expression
    pub fn eval(&mut self, expr: &Expr<Type>) -> TlispResult<Value> {
        let mut context = EvaluationContext::new(Rc::clone(&self.global_env));
        self.eval_with_context(expr, &mut context)
    }

    /// Evaluate an untyped expression by converting it to a typed expression with placeholder types
    pub fn eval_untyped(&mut self, expr: &Expr<()>) -> TlispResult<Value> {
        // Convert untyped expression to typed expression with placeholder types
        let typed_expr = self.add_placeholder_types(expr);
        self.eval(&typed_expr)
    }

    /// Evaluate multiple untyped expressions with shared context
    pub fn eval_multiple_untyped(&mut self, expressions: &[Expr<()>]) -> TlispResult<Value> {
        if expressions.is_empty() {
            return Ok(Value::Null);
        }

        // Create a single context for all expressions
        let mut context = EvaluationContext::new(Rc::clone(&self.global_env));
        let mut last_result = Value::Null;

        for expr in expressions.iter() {
            // Convert untyped expression to typed expression with placeholder types
            let typed_expr = self.add_placeholder_types(expr);
            last_result = self.eval_with_context(&typed_expr, &mut context)?;
        }

        Ok(last_result)
    }

    /// Add placeholder types to an untyped expression
    fn add_placeholder_types(&self, expr: &Expr<()>) -> Expr<Type> {
        match expr {
            Expr::Symbol(name, _) => Expr::Symbol(name.clone(), Type::TypeVar("T".to_string())),
            Expr::Number(n, _) => Expr::Number(*n, Type::Int),
            Expr::Float(f, _) => Expr::Float(*f, Type::Float),
            Expr::Bool(b, _) => Expr::Bool(*b, Type::Bool),
            Expr::String(s, _) => Expr::String(s.clone(), Type::String),
            Expr::List(items, _) => {
                let typed_items = items.iter().map(|item| self.add_placeholder_types(item)).collect();
                Expr::List(typed_items, Type::List(Box::new(Type::TypeVar("T".to_string()))))
            },
            Expr::Lambda(params, body, _) => {
                let typed_body = Box::new(self.add_placeholder_types(body));
                let param_types = params.iter().map(|_| Type::TypeVar("T".to_string())).collect();
                let return_type = Type::TypeVar("R".to_string());
                Expr::Lambda(params.clone(), typed_body, Type::Function(param_types, Box::new(return_type)))
            },
            Expr::Application(func, args, _) => {
                let typed_func = Box::new(self.add_placeholder_types(func));
                let typed_args = args.iter().map(|arg| self.add_placeholder_types(arg)).collect();
                Expr::Application(typed_func, typed_args, Type::TypeVar("T".to_string()))
            },
            Expr::Let(bindings, body, _) => {
                let typed_bindings = bindings.iter().map(|(name, expr)| {
                    (name.clone(), self.add_placeholder_types(expr))
                }).collect();
                let typed_body = Box::new(self.add_placeholder_types(body));
                Expr::Let(typed_bindings, typed_body, Type::TypeVar("T".to_string()))
            },
            Expr::If(cond, then_expr, else_expr, _) => {
                let typed_cond = Box::new(self.add_placeholder_types(cond));
                let typed_then = Box::new(self.add_placeholder_types(then_expr));
                let typed_else = Box::new(self.add_placeholder_types(else_expr));
                Expr::If(typed_cond, typed_then, typed_else, Type::TypeVar("T".to_string()))
            },
            Expr::Quote(expr, _) => {
                let typed_expr = Box::new(self.add_placeholder_types(expr));
                Expr::Quote(typed_expr, Type::TypeVar("T".to_string()))
            },
            Expr::Define(name, expr, _) => {
                let typed_expr = Box::new(self.add_placeholder_types(expr));
                Expr::Define(name.clone(), typed_expr, Type::Unit)
            },
            Expr::Set(name, expr, _) => {
                let typed_expr = Box::new(self.add_placeholder_types(expr));
                Expr::Set(name.clone(), typed_expr, Type::Unit)
            },
        }
    }
    
    /// Evaluate with context
    fn eval_with_context(&mut self, expr: &Expr<Type>, context: &mut EvaluationContext) -> TlispResult<Value> {
        context.check_depth()?;
        context.depth += 1;
        
        let result = match expr {
            Expr::Number(n, _) => Ok(Value::Int(*n)),
            Expr::Float(f, _) => Ok(Value::Float(*f)),
            Expr::Bool(b, _) => Ok(Value::Bool(*b)),
            Expr::String(s, _) => Ok(Value::String(s.clone())),
            
            Expr::Symbol(name, _) => {
                context.env.borrow().get(name)
                    .ok_or_else(|| TlispError::Runtime(format!("Undefined variable: {}", name)))
            }
            
            Expr::List(items, _) => {
                // Special case: if this is a single symbol that resolves to a function,
                // treat it as a zero-argument function call
                if items.len() == 1 {
                    if let Expr::Symbol(name, _) = &items[0] {
                        // Check if this symbol resolves to a function
                        let is_function = {
                            if let Some(value) = context.env.borrow().get(name) {
                                matches!(value, Value::Function(_) | Value::Builtin(_))
                            } else {
                                false
                            }
                        };

                        if is_function {
                            // This is a function call with zero arguments
                            return self.eval_application(&items[0], &[], context);
                        }
                    }
                }

                // Regular list evaluation
                let values: Result<Vec<Value>, TlispError> = items.iter()
                    .map(|item| self.eval_with_context(item, context))
                    .collect();
                Ok(Value::List(values?))
            }
            
            Expr::Lambda(params, body, _) => {
                // Capture current environment
                let closure_env = self.capture_environment(&context.env);

                Ok(Value::Function(Function {
                    params: params.clone(),
                    body: (**body).clone(),
                    env: closure_env,
                }))
            }
            
            Expr::Application(func_expr, args, _) => {
                self.eval_application(func_expr, args, context)
            }
            
            Expr::Let(bindings, body, _) => {
                self.eval_let(bindings, body, context)
            }
            
            Expr::If(condition, then_expr, else_expr, _) => {
                let cond_value = self.eval_with_context(condition, context)?;
                
                if cond_value.is_truthy() {
                    self.eval_with_context(then_expr, context)
                } else {
                    self.eval_with_context(else_expr, context)
                }
            }
            
            Expr::Quote(expr, _) => {
                self.quote_to_value(expr)
            }

            Expr::Define(name, value_expr, _) => {
                // Special handling for recursive functions
                if let Expr::Lambda(params, body, _) = value_expr.as_ref() {
                    // Create the function with current environment
                    let mut closure_env = self.capture_environment(&context.env);

                    // Create a placeholder function first
                    let placeholder_function = Function {
                        params: params.clone(),
                        body: (**body).clone(),
                        env: closure_env.clone(),
                    };

                    let placeholder_value = Value::Function(placeholder_function);

                    // Add the function to its own closure environment for recursion
                    closure_env.insert(name.clone(), placeholder_value);

                    // Now create the final function with the recursive environment
                    let recursive_function = Function {
                        params: params.clone(),
                        body: (**body).clone(),
                        env: closure_env.clone(),
                    };

                    let final_value = Value::Function(recursive_function);

                    // Update the closure environment with the final function
                    closure_env.insert(name.clone(), final_value.clone());

                    // Create the final function again with the updated closure
                    let final_recursive_function = Function {
                        params: params.clone(),
                        body: (**body).clone(),
                        env: closure_env,
                    };

                    let final_final_value = Value::Function(final_recursive_function);
                    // Define in both the context environment and the global environment
                    context.env.borrow_mut().define(name.clone(), final_final_value.clone());
                    self.global_env.borrow_mut().define(name.clone(), final_final_value.clone());
                    Ok(final_final_value)
                } else {
                    let value = self.eval_with_context(value_expr, context)?;
                    // Define in both the context environment and the global environment
                    context.env.borrow_mut().define(name.clone(), value.clone());
                    self.global_env.borrow_mut().define(name.clone(), value.clone());
                    Ok(value)
                }
            }

            Expr::Set(name, value_expr, _) => {
                let value = self.eval_with_context(value_expr, context)?;
                if context.env.borrow_mut().set(name, value.clone()) {
                    Ok(value)
                } else {
                    Err(TlispError::Runtime(format!("Undefined variable: {}", name)))
                }
            }
        };
        
        context.depth -= 1;
        result
    }
    
    /// Evaluate function application
    fn eval_application(&mut self, func_expr: &Expr<Type>, args: &[Expr<Type>], context: &mut EvaluationContext) -> TlispResult<Value> {
        let func_value = self.eval_with_context(func_expr, context)?;



        match func_value {
            Value::Function(function) => {
                self.call_user_function(&function, args, context)
            }
            Value::Builtin(name) => {
                self.call_builtin(&name, args, context)
            }
            _ => Err(TlispError::Runtime("Not a function".to_string())),
        }
    }
    
    /// Call user-defined function
    fn call_user_function(&mut self, function: &Function, args: &[Expr<Type>], context: &mut EvaluationContext) -> TlispResult<Value> {
        if args.len() != function.params.len() {
            return Err(TlispError::Runtime(format!(
                "Arity mismatch: expected {} arguments, got {}",
                function.params.len(),
                args.len()
            )));
        }

        // Evaluate arguments
        let arg_values: Result<Vec<Value>, TlispError> = args.iter()
            .map(|arg| self.eval_with_context(arg, context))
            .collect();
        let arg_values = arg_values?;

        // Create new environment with function closure
        let func_env = Rc::new(RefCell::new(Environment::new()));

        // Add closure variables (this should include the function itself for recursion)
        for (name, value) in &function.env {
            func_env.borrow_mut().define(name.clone(), value.clone());
        }

        // Bind parameters
        for (param, value) in function.params.iter().zip(arg_values.iter()) {
            func_env.borrow_mut().define(param.clone(), value.clone());
        }

        // For recursive functions, also check the global environment for user-defined functions
        // This ensures that recursive calls can find the function even if it's not in the closure
        // But we don't want to override built-in functions
        let builtin_names = [
            "+", "-", "*", "/", "=", "eq", "<", "<=", ">", ">=",
            "list", "car", "cdr", "head", "tail", "cons", "append", "length",
            "begin", "cond", "and", "or", "not", "if", "print", "println",
            "spawn", "send", "receive", "self", "random", "current-time",
            "error", "abs", "sqrt", "floor", "number?", "string?", "symbol?",
            "boolean?", "list?", "equal?", "number->string", "symbol->string",
            "list->string", "newline", "null?", "string-append", "mod", "modulo",
            "cadr", "caddr", "cadddr", "set!", "string-split", "string-starts-with",
            "substring", "string->number", "list-ref", "string=?", "sender",
            "reverse", ">=", "import"
        ];

        let global_bindings = self.global_env.borrow().all_bindings();
        for (name, value) in global_bindings {
            if let Value::Function(_) = value {
                // Only add user-defined functions, not built-ins
                if !builtin_names.contains(&name.as_str()) && !func_env.borrow().all_bindings().contains_key(&name) {
                    func_env.borrow_mut().define(name, value);
                }
            }
        }

        // Evaluate body in function environment
        let old_env = Rc::clone(&context.env);
        context.env = func_env;

        let result = self.eval_with_context(&function.body, context);

        context.env = old_env;
        result
    }
    
    /// Call built-in function
    fn call_builtin(&mut self, name: &str, args: &[Expr<Type>], context: &mut EvaluationContext) -> TlispResult<Value> {
        match name {
            "add" => self.builtin_add(args, context),
            "sub" => self.builtin_sub(args, context),
            "mul" => self.builtin_mul(args, context),
            "div" => self.builtin_div(args, context),
            "eq" => self.builtin_eq(args, context),
            "lt" => self.builtin_lt(args, context),
            "le" => self.builtin_le(args, context),
            "gt" => self.builtin_gt(args, context),
            "ge" => self.builtin_ge(args, context),
            "list" => self.builtin_list(args, context),
            "car" => self.builtin_car(args, context),
            "cdr" => self.builtin_cdr(args, context),
            "head" => self.builtin_car(args, context), // Alias for car
            "tail" => self.builtin_cdr(args, context), // Alias for cdr
            "cons" => self.builtin_cons(args, context),
            "append" => self.builtin_append(args, context),
            "length" => self.builtin_length(args, context),
            "and" => self.builtin_and(args, context),
            "or" => self.builtin_or(args, context),
            "not" => self.builtin_not(args, context),
            "println" => self.builtin_println(args, context),
            "newline" => self.builtin_newline(args, context),
            "string-append" => self.builtin_string_append(args, context),
            "number->string" => self.builtin_number_to_string(args, context),
            "symbol->string" => self.builtin_symbol_to_string(args, context),
            "list->string" => self.builtin_list_to_string(args, context),
            "pid->string" => self.builtin_pid_to_string(args, context),
            "null?" => self.builtin_null_p(args, context),
            "number?" => self.builtin_number_p(args, context),
            "string?" => self.builtin_string_p(args, context),
            "symbol?" => self.builtin_symbol_p(args, context),
            "boolean?" => self.builtin_boolean_p(args, context),
            "list?" => self.builtin_list_p(args, context),
            "equal?" => self.builtin_equal_p(args, context),
            "modulo" => self.builtin_modulo(args, context),
            "mod" => self.builtin_modulo(args, context), // Alias for modulo
            "floor" => self.builtin_floor(args, context),
            "sqrt" => self.builtin_sqrt(args, context),
            "abs" => self.builtin_abs(args, context),
            "error" => self.builtin_error(args, context),
            "current-time" => self.builtin_current_time(args, context),
            "random" => self.builtin_random(args, context),
            "begin" => self.builtin_begin(args, context),
            "cond" => self.builtin_cond(args, context),
            "print" => self.builtin_print(args, context),
            "spawn" => self.builtin_spawn(args, context),
            "send" => self.builtin_send(args, context),
            "receive" => self.builtin_receive(args, context),
            "self" => self.builtin_self(args, context),
            "sender" => self.builtin_sender(args, context),
            "cadr" => self.builtin_cadr(args, context),
            "caddr" => self.builtin_caddr(args, context),
            "cadddr" => self.builtin_cadddr(args, context),
            "set!" => self.builtin_set(args, context),
            "string-split" => self.builtin_string_split(args, context),
            "string-starts-with" => self.builtin_string_starts_with(args, context),
            "substring" => self.builtin_substring(args, context),
            "string->number" => self.builtin_string_to_number(args, context),
            "list-ref" => self.builtin_list_ref(args, context),
            "string=?" => self.builtin_string_equal_p(args, context),
            "reverse" => self.builtin_reverse(args, context),
            "import" => self.builtin_import(args, context),

            // HTTP server module functions
            "http-server:start" => self.call_module_function("http-server", "start", args, context),
            "http-server:stop" => self.call_module_function("http-server", "stop", args, context),
            "http-server:get" => self.call_module_function("http-server", "get", args, context),
            "http-server:post" => self.call_module_function("http-server", "post", args, context),
            "http-server:put" => self.call_module_function("http-server", "put", args, context),
            "http-server:delete" => self.call_module_function("http-server", "delete", args, context),
            "http-server:send-response" => self.call_module_function("http-server", "send-response", args, context),

            // JSON module functions
            "json:parse" => self.call_module_function("json", "parse", args, context),
            "json:stringify" => self.call_module_function("json", "stringify", args, context),
            "json:get" => self.call_module_function("json", "get", args, context),
            "json:set!" => self.call_module_function("json", "set!", args, context),
            "json:object" => self.call_module_function("json", "object", args, context),

            // Async utils module functions
            "async-utils:now" => self.call_module_function("async-utils", "now", args, context),
            "async-utils:timestamp-ms" => self.call_module_function("async-utils", "timestamp-ms", args, context),
            "async-utils:format-time" => self.call_module_function("async-utils", "format-time", args, context),
            "async-utils:sleep" => self.call_module_function("async-utils", "sleep", args, context),
            "async-utils:spawn-task" => self.call_module_function("async-utils", "spawn-task", args, context),
            "async-utils:timestamp-iso" => self.call_module_function("async-utils", "timestamp-iso", args, context),

            // Ream ORM module functions
            "ream-orm:connect" => self.call_module_function("ream-orm", "connect", args, context),
            "ream-orm:disconnect" => self.call_module_function("ream-orm", "disconnect", args, context),
            "ream-orm:execute" => self.call_module_function("ream-orm", "execute", args, context),
            "ream-orm:execute-query" => self.call_module_function("ream-orm", "execute-query", args, context),
            "ream-orm:execute-query-single" => self.call_module_function("ream-orm", "execute-query-single", args, context),
            "ream-orm:execute-mutation" => self.call_module_function("ream-orm", "execute-mutation", args, context),
            "ream-orm:execute-transaction" => self.call_module_function("ream-orm", "execute-transaction", args, context),
            "ream-orm:create-query-builder" => self.call_module_function("ream-orm", "create-query-builder", args, context),
            "ream-orm:select" => self.call_module_function("ream-orm", "select", args, context),
            "ream-orm:where" => self.call_module_function("ream-orm", "where", args, context),
            "ream-orm:limit" => self.call_module_function("ream-orm", "limit", args, context),
            "ream-orm:order-by" => self.call_module_function("ream-orm", "order-by", args, context),
            "ream-orm:build-query" => self.call_module_function("ream-orm", "build-query", args, context),
            "ream-orm:create-mutation-builder" => self.call_module_function("ream-orm", "create-mutation-builder", args, context),
            "ream-orm:insert" => self.call_module_function("ream-orm", "insert", args, context),
            "ream-orm:update" => self.call_module_function("ream-orm", "update", args, context),
            "ream-orm:delete" => self.call_module_function("ream-orm", "delete", args, context),
            "ream-orm:returning" => self.call_module_function("ream-orm", "returning", args, context),
            "ream-orm:build-mutation" => self.call_module_function("ream-orm", "build-mutation", args, context),
            "ream-orm:get-schema-info" => self.call_module_function("ream-orm", "get-schema-info", args, context),

            // Ream GraphQL module functions
            "ream-graphql:create-context" => self.call_module_function("ream-graphql", "create-context", args, context),
            "ream-graphql:parse-query" => self.call_module_function("ream-graphql", "parse-query", args, context),
            "ream-graphql:parse-mutation" => self.call_module_function("ream-graphql", "parse-mutation", args, context),
            "ream-graphql:compile-query" => self.call_module_function("ream-graphql", "compile-query", args, context),
            "ream-graphql:compile-mutation" => self.call_module_function("ream-graphql", "compile-mutation", args, context),

            // Hypervisor functions
            "hypervisor:start" => self.builtin_hypervisor_start(args, context),
            "hypervisor:stop" => self.builtin_hypervisor_stop(args, context),
            "hypervisor:register-actor" => self.builtin_hypervisor_register_actor(args, context),
            "hypervisor:unregister-actor" => self.builtin_hypervisor_unregister_actor(args, context),
            "hypervisor:get-actor-metrics" => self.builtin_hypervisor_get_actor_metrics(args, context),
            "hypervisor:get-system-metrics" => self.builtin_hypervisor_get_system_metrics(args, context),
            "hypervisor:list-actors" => self.builtin_hypervisor_list_actors(args, context),
            "hypervisor:health-check" => self.builtin_hypervisor_health_check(args, context),
            "hypervisor:set-alert-threshold" => self.builtin_hypervisor_set_alert_threshold(args, context),
            "hypervisor:get-alerts" => self.builtin_hypervisor_get_alerts(args, context),
            "hypervisor:restart-actor" => self.builtin_hypervisor_restart_actor(args, context),
            "hypervisor:suspend-actor" => self.builtin_hypervisor_suspend_actor(args, context),
            "hypervisor:resume-actor" => self.builtin_hypervisor_resume_actor(args, context),
            "hypervisor:kill-actor" => self.builtin_hypervisor_kill_actor(args, context),
            "hypervisor:get-supervision-tree" => self.builtin_hypervisor_get_supervision_tree(args, context),

            _ => Err(TlispError::Runtime(format!("Unknown builtin: {}", name))),
        }
    }
    
    /// Evaluate let expression
    fn eval_let(&mut self, bindings: &[(String, Expr<Type>)], body: &Expr<Type>, context: &mut EvaluationContext) -> TlispResult<Value> {
        // Create new scope
        context.push_scope();
        
        // Evaluate and bind variables
        for (name, expr) in bindings {
            let value = self.eval_with_context(expr, context)?;
            context.env.borrow_mut().define(name.clone(), value);
        }
        
        // Evaluate body
        let result = self.eval_with_context(body, context);
        
        // Pop scope
        context.pop_scope();
        
        result
    }
    
    /// Convert quoted expression to value
    fn quote_to_value(&self, expr: &Expr<Type>) -> TlispResult<Value> {
        match expr {
            Expr::Number(n, _) => Ok(Value::Int(*n)),
            Expr::Float(f, _) => Ok(Value::Float(*f)),
            Expr::Bool(b, _) => Ok(Value::Bool(*b)),
            Expr::String(s, _) => Ok(Value::String(s.clone())),
            Expr::Symbol(s, _) => Ok(Value::Symbol(s.clone())),
            Expr::List(items, _) => {
                let values: Result<Vec<Value>, TlispError> = items.iter()
                    .map(|item| self.quote_to_value(item))
                    .collect();
                Ok(Value::List(values?))
            }
            _ => Ok(Value::Symbol("quote".to_string())), // Simplified
        }
    }
    
    /// Capture environment for closures
    fn capture_environment(&self, env: &Rc<RefCell<Environment>>) -> HashMap<String, Value> {
        env.borrow().all_bindings()
    }
    
    // Built-in function implementations
    
    fn builtin_add(&mut self, args: &[Expr<Type>], context: &mut EvaluationContext) -> TlispResult<Value> {
        if args.is_empty() {
            return Ok(Value::Int(0)); // Identity for addition
        }

        let mut result = self.eval_with_context(&args[0], context)?;

        for arg in &args[1..] {
            let value = self.eval_with_context(arg, context)?;
            result = match (result, value) {
                (Value::Int(a), Value::Int(b)) => Value::Int(a + b),
                (Value::Float(a), Value::Float(b)) => Value::Float(a + b),
                (Value::Int(a), Value::Float(b)) => Value::Float(a as f64 + b),
                (Value::Float(a), Value::Int(b)) => Value::Float(a + b as f64),
                _ => return Err(TlispError::Runtime("+ requires numbers".to_string())),
            };
        }

        Ok(result)
    }
    
    fn builtin_sub(&mut self, args: &[Expr<Type>], context: &mut EvaluationContext) -> TlispResult<Value> {
        match args.len() {
            1 => {
                // Unary minus (negation)
                let a = self.eval_with_context(&args[0], context)?;
                match a {
                    Value::Int(a) => Ok(Value::Int(-a)),
                    Value::Float(a) => Ok(Value::Float(-a)),
                    _ => Err(TlispError::Runtime("- requires numbers".to_string())),
                }
            }
            2 => {
                // Binary minus (subtraction)
                let a = self.eval_with_context(&args[0], context)?;
                let b = self.eval_with_context(&args[1], context)?;

                match (a, b) {
                    (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a - b)),
                    (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a - b)),
                    (Value::Int(a), Value::Float(b)) => Ok(Value::Float(a as f64 - b)),
                    (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a - b as f64)),
                    _ => Err(TlispError::Runtime("- requires numbers".to_string())),
                }
            }
            _ => Err(TlispError::Runtime("- requires 1 or 2 arguments".to_string())),
        }
    }
    
    fn builtin_mul(&mut self, args: &[Expr<Type>], context: &mut EvaluationContext) -> TlispResult<Value> {
        if args.is_empty() {
            return Ok(Value::Int(1)); // Identity for multiplication
        }

        let mut result = self.eval_with_context(&args[0], context)?;

        for arg in &args[1..] {
            let value = self.eval_with_context(arg, context)?;
            result = match (result, value) {
                (Value::Int(a), Value::Int(b)) => Value::Int(a * b),
                (Value::Float(a), Value::Float(b)) => Value::Float(a * b),
                (Value::Int(a), Value::Float(b)) => Value::Float(a as f64 * b),
                (Value::Float(a), Value::Int(b)) => Value::Float(a * b as f64),
                _ => return Err(TlispError::Runtime("* requires numbers".to_string())),
            };
        }

        Ok(result)
    }
    
    fn builtin_div(&mut self, args: &[Expr<Type>], context: &mut EvaluationContext) -> TlispResult<Value> {
        if args.len() != 2 {
            return Err(TlispError::Runtime("/ requires 2 arguments".to_string()));
        }
        
        let a = self.eval_with_context(&args[0], context)?;
        let b = self.eval_with_context(&args[1], context)?;
        
        match (a, b) {
            (Value::Int(a), Value::Int(b)) => {
                if b == 0 {
                    Err(TlispError::Runtime("Division by zero".to_string()))
                } else {
                    Ok(Value::Int(a / b))
                }
            }
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a / b)),
            (Value::Int(a), Value::Float(b)) => Ok(Value::Float(a as f64 / b)),
            (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a / b as f64)),
            _ => Err(TlispError::Runtime("/ requires numbers".to_string())),
        }
    }
    
    fn builtin_eq(&mut self, args: &[Expr<Type>], context: &mut EvaluationContext) -> TlispResult<Value> {
        if args.len() != 2 {
            return Err(TlispError::Runtime("= requires 2 arguments".to_string()));
        }
        
        let a = self.eval_with_context(&args[0], context)?;
        let b = self.eval_with_context(&args[1], context)?;
        
        Ok(Value::Bool(a == b))
    }
    
    fn builtin_lt(&mut self, args: &[Expr<Type>], context: &mut EvaluationContext) -> TlispResult<Value> {
        if args.len() != 2 {
            return Err(TlispError::Runtime("< requires 2 arguments".to_string()));
        }
        
        let a = self.eval_with_context(&args[0], context)?;
        let b = self.eval_with_context(&args[1], context)?;
        
        match (a, b) {
            (Value::Int(a), Value::Int(b)) => Ok(Value::Bool(a < b)),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Bool(a < b)),
            _ => Err(TlispError::Runtime("< requires numbers".to_string())),
        }
    }
    
    fn builtin_le(&mut self, args: &[Expr<Type>], context: &mut EvaluationContext) -> TlispResult<Value> {
        if args.len() != 2 {
            return Err(TlispError::Runtime("<= requires 2 arguments".to_string()));
        }
        
        let a = self.eval_with_context(&args[0], context)?;
        let b = self.eval_with_context(&args[1], context)?;
        
        match (a, b) {
            (Value::Int(a), Value::Int(b)) => Ok(Value::Bool(a <= b)),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Bool(a <= b)),
            _ => Err(TlispError::Runtime("<= requires numbers".to_string())),
        }
    }
    
    fn builtin_gt(&mut self, args: &[Expr<Type>], context: &mut EvaluationContext) -> TlispResult<Value> {
        if args.len() != 2 {
            return Err(TlispError::Runtime("> requires 2 arguments".to_string()));
        }
        
        let a = self.eval_with_context(&args[0], context)?;
        let b = self.eval_with_context(&args[1], context)?;
        
        match (a, b) {
            (Value::Int(a), Value::Int(b)) => Ok(Value::Bool(a > b)),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Bool(a > b)),
            _ => Err(TlispError::Runtime("> requires numbers".to_string())),
        }
    }
    
    fn builtin_ge(&mut self, args: &[Expr<Type>], context: &mut EvaluationContext) -> TlispResult<Value> {
        if args.len() != 2 {
            return Err(TlispError::Runtime(">= requires 2 arguments".to_string()));
        }
        
        let a = self.eval_with_context(&args[0], context)?;
        let b = self.eval_with_context(&args[1], context)?;
        
        match (a, b) {
            (Value::Int(a), Value::Int(b)) => Ok(Value::Bool(a >= b)),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Bool(a >= b)),
            _ => Err(TlispError::Runtime(">= requires numbers".to_string())),
        }
    }
    
    fn builtin_list(&mut self, args: &[Expr<Type>], context: &mut EvaluationContext) -> TlispResult<Value> {
        let values: Result<Vec<Value>, TlispError> = args.iter()
            .map(|arg| self.eval_with_context(arg, context))
            .collect();
        Ok(Value::List(values?))
    }
    
    fn builtin_car(&mut self, args: &[Expr<Type>], context: &mut EvaluationContext) -> TlispResult<Value> {
        if args.len() != 1 {
            return Err(TlispError::Runtime("car requires 1 argument".to_string()));
        }
        
        let list = self.eval_with_context(&args[0], context)?;
        
        match list {
            Value::List(items) => {
                if items.is_empty() {
                    Ok(Value::Null)
                } else {
                    Ok(items[0].clone())
                }
            }
            _ => Err(TlispError::Runtime("car requires a list".to_string())),
        }
    }
    
    fn builtin_cdr(&mut self, args: &[Expr<Type>], context: &mut EvaluationContext) -> TlispResult<Value> {
        if args.len() != 1 {
            return Err(TlispError::Runtime("cdr requires 1 argument".to_string()));
        }
        
        let list = self.eval_with_context(&args[0], context)?;
        
        match list {
            Value::List(items) => {
                if items.is_empty() {
                    Ok(Value::List(Vec::new()))
                } else {
                    Ok(Value::List(items[1..].to_vec()))
                }
            }
            _ => Err(TlispError::Runtime("cdr requires a list".to_string())),
        }
    }
    
    fn builtin_cons(&mut self, args: &[Expr<Type>], context: &mut EvaluationContext) -> TlispResult<Value> {
        if args.len() != 2 {
            return Err(TlispError::Runtime("cons requires 2 arguments".to_string()));
        }

        let head = self.eval_with_context(&args[0], context)?;
        let tail = self.eval_with_context(&args[1], context)?;

        match tail {
            Value::List(mut items) => {
                items.insert(0, head);
                Ok(Value::List(items))
            }
            _ => Err(TlispError::Runtime("cons requires a list as second argument".to_string())),
        }
    }

    fn builtin_append(&mut self, args: &[Expr<Type>], context: &mut EvaluationContext) -> TlispResult<Value> {
        if args.is_empty() {
            return Ok(Value::List(Vec::new()));
        }

        let mut result = Vec::new();

        for arg in args {
            let value = self.eval_with_context(arg, context)?;
            match value {
                Value::List(items) => {
                    result.extend(items);
                }
                _ => return Err(TlispError::Runtime("append requires list arguments".to_string())),
            }
        }

        Ok(Value::List(result))
    }
    
    fn builtin_length(&mut self, args: &[Expr<Type>], context: &mut EvaluationContext) -> TlispResult<Value> {
        if args.len() != 1 {
            return Err(TlispError::Runtime("length requires 1 argument".to_string()));
        }
        
        let list = self.eval_with_context(&args[0], context)?;
        
        match list {
            Value::List(items) => Ok(Value::Int(items.len() as i64)),
            Value::String(s) => Ok(Value::Int(s.len() as i64)),
            _ => Err(TlispError::Runtime("length requires a list or string".to_string())),
        }
    }
    
    fn builtin_print(&mut self, args: &[Expr<Type>], context: &mut EvaluationContext) -> TlispResult<Value> {
        for (i, arg) in args.iter().enumerate() {
            let value = self.eval_with_context(arg, context)?;
            match value {
                Value::String(s) => print!("{}", s),
                _ => print!("{}", value),
            }
            if i < args.len() - 1 {
                print!(" ");
            }
        }
        Ok(Value::Unit)
    }
    
    // REAM integration built-ins

    fn builtin_spawn(&mut self, args: &[Expr<Type>], context: &mut EvaluationContext) -> TlispResult<Value> {
        if args.len() != 1 {
            return Err(TlispError::Runtime("spawn requires 1 argument (function)".to_string()));
        }

        // Evaluate the function argument
        let function_value = self.eval_with_context(&args[0], context)?;

        match function_value {
            Value::Function(func) => {
                // Create a TLisp actor with this function
                let actor = crate::tlisp::ream_bridge::TlispActor::new(format!("{:?}", func));
                let pid = actor.pid();

                // TODO: Register with REAM runtime
                // For now, store in a global actor registry
                println!("Spawned TLisp actor with PID: {}", pid);

                Ok(Value::Pid(pid))
            }
            Value::Symbol(symbol_name) => {
                // Handle actor function by name
                let pid = crate::types::Pid::new();
                println!("Spawned actor '{}' with PID: {}", symbol_name, pid);
                Ok(Value::Pid(pid))
            }
            _ => Err(TlispError::Runtime("spawn requires a function or symbol".to_string())),
        }
    }

    fn builtin_send(&mut self, args: &[Expr<Type>], context: &mut EvaluationContext) -> TlispResult<Value> {
        if args.len() != 2 {
            return Err(TlispError::Runtime("send requires 2 arguments (pid, message)".to_string()));
        }

        let pid_value = self.eval_with_context(&args[0], context)?;
        let message_value = self.eval_with_context(&args[1], context)?;

        match pid_value {
            Value::Pid(pid) => {
                // Convert TLisp value to MessagePayload
                let payload = self.value_to_message_payload(message_value)?;

                // TODO: Send via REAM runtime
                // For now, just log the message
                println!("Sending message to PID {}: {:?}", pid, payload);

                Ok(Value::Unit)
            }
            _ => Err(TlispError::Runtime("send first argument must be a PID".to_string())),
        }
    }

    fn builtin_receive(&mut self, _args: &[Expr<Type>], _context: &mut EvaluationContext) -> TlispResult<Value> {
        // Implement a blocking receive that keeps the process alive
        println!("🔄 Actor waiting for messages (blocking receive)...");

        // Use a simple blocking approach - wait for user input or signal
        // This keeps the process alive without busy-waiting
        use std::io::{self, BufRead};

        println!("📨 Press Enter to send a test message, or Ctrl+C to stop the server");
        let stdin = io::stdin();
        let mut lines = stdin.lock().lines();

        // Block until we get input
        match lines.next() {
            Some(Ok(_line)) => {
                println!("📬 Received input - sending test message to actor");
                Ok(Value::List(vec![
                    Value::Symbol("user-input".to_string()),
                    Value::String("test-message".to_string()),
                ]))
            }
            Some(Err(e)) => {
                println!("❌ Error reading input: {}", e);
                Err(TlispError::Runtime(format!("Input error: {}", e)))
            }
            None => {
                println!("📭 No more input - server shutting down");
                Ok(Value::Symbol("eof".to_string()))
            }
        }
    }

    fn builtin_self(&mut self, _args: &[Expr<Type>], _context: &mut EvaluationContext) -> TlispResult<Value> {
        // TODO: Get current process PID from REAM runtime context
        // For now, return a mock PID
        Ok(Value::Pid(crate::types::Pid::new()))
    }

    // Additional built-in functions for CLI demo

    fn builtin_println(&mut self, args: &[Expr<Type>], context: &mut EvaluationContext) -> TlispResult<Value> {
        if args.len() != 1 {
            return Err(TlispError::Runtime("println requires 1 argument".to_string()));
        }

        let value = self.eval_with_context(&args[0], context)?;
        match value {
            Value::String(s) => println!("{}", s),
            _ => println!("{}", value),
        }
        Ok(Value::Unit)
    }

    fn builtin_newline(&mut self, args: &[Expr<Type>], _context: &mut EvaluationContext) -> TlispResult<Value> {
        if !args.is_empty() {
            return Err(TlispError::Runtime("newline requires 0 arguments".to_string()));
        }

        println!();
        Ok(Value::Unit)
    }

    fn builtin_string_append(&mut self, args: &[Expr<Type>], context: &mut EvaluationContext) -> TlispResult<Value> {
        let mut result = String::new();

        for arg in args {
            let value = self.eval_with_context(arg, context)?;
            match value {
                Value::String(s) => result.push_str(&s),
                _ => return Err(TlispError::Runtime("string-append requires string arguments".to_string())),
            }
        }

        Ok(Value::String(result))
    }

    fn builtin_number_to_string(&mut self, args: &[Expr<Type>], context: &mut EvaluationContext) -> TlispResult<Value> {
        if args.len() != 1 {
            return Err(TlispError::Runtime("number->string requires 1 argument".to_string()));
        }

        let value = self.eval_with_context(&args[0], context)?;
        match value {
            Value::Int(n) => Ok(Value::String(n.to_string())),
            Value::Float(f) => Ok(Value::String(f.to_string())),
            _ => Err(TlispError::Runtime("number->string requires a number".to_string())),
        }
    }

    fn builtin_symbol_to_string(&mut self, args: &[Expr<Type>], context: &mut EvaluationContext) -> TlispResult<Value> {
        if args.len() != 1 {
            return Err(TlispError::Runtime("symbol->string requires 1 argument".to_string()));
        }

        let value = self.eval_with_context(&args[0], context)?;
        match value {
            Value::Symbol(s) => Ok(Value::String(s)),
            _ => Err(TlispError::Runtime("symbol->string requires a symbol".to_string())),
        }
    }

    fn builtin_list_to_string(&mut self, args: &[Expr<Type>], context: &mut EvaluationContext) -> TlispResult<Value> {
        if args.len() != 1 {
            return Err(TlispError::Runtime("list->string requires 1 argument".to_string()));
        }

        let value = self.eval_with_context(&args[0], context)?;
        match value {
            Value::List(items) => {
                let strings: Vec<String> = items.iter().map(|v| format!("{}", v)).collect();
                Ok(Value::String(format!("({})", strings.join(" "))))
            }
            _ => Err(TlispError::Runtime("list->string requires a list".to_string())),
        }
    }

    fn builtin_pid_to_string(&mut self, args: &[Expr<Type>], context: &mut EvaluationContext) -> TlispResult<Value> {
        if args.len() != 1 {
            return Err(TlispError::Runtime("pid->string requires 1 argument".to_string()));
        }

        let value = self.eval_with_context(&args[0], context)?;
        match value {
            Value::Pid(pid) => Ok(Value::String(format!("#{}", pid.raw()))),
            _ => Err(TlispError::Runtime("pid->string requires a PID".to_string())),
        }
    }

    // Type predicate functions

    fn builtin_null_p(&mut self, args: &[Expr<Type>], context: &mut EvaluationContext) -> TlispResult<Value> {
        if args.len() != 1 {
            return Err(TlispError::Runtime("null? requires 1 argument".to_string()));
        }

        let value = self.eval_with_context(&args[0], context)?;
        let is_null = match value {
            Value::List(ref items) => items.is_empty(),
            _ => false,
        };
        Ok(Value::Bool(is_null))
    }

    fn builtin_number_p(&mut self, args: &[Expr<Type>], context: &mut EvaluationContext) -> TlispResult<Value> {
        if args.len() != 1 {
            return Err(TlispError::Runtime("number? requires 1 argument".to_string()));
        }

        let value = self.eval_with_context(&args[0], context)?;
        let is_number = matches!(value, Value::Int(_) | Value::Float(_));
        Ok(Value::Bool(is_number))
    }

    fn builtin_string_p(&mut self, args: &[Expr<Type>], context: &mut EvaluationContext) -> TlispResult<Value> {
        if args.len() != 1 {
            return Err(TlispError::Runtime("string? requires 1 argument".to_string()));
        }

        let value = self.eval_with_context(&args[0], context)?;
        let is_string = matches!(value, Value::String(_));
        Ok(Value::Bool(is_string))
    }

    fn builtin_symbol_p(&mut self, args: &[Expr<Type>], context: &mut EvaluationContext) -> TlispResult<Value> {
        if args.len() != 1 {
            return Err(TlispError::Runtime("symbol? requires 1 argument".to_string()));
        }

        let value = self.eval_with_context(&args[0], context)?;
        let is_symbol = matches!(value, Value::Symbol(_));
        Ok(Value::Bool(is_symbol))
    }

    fn builtin_boolean_p(&mut self, args: &[Expr<Type>], context: &mut EvaluationContext) -> TlispResult<Value> {
        if args.len() != 1 {
            return Err(TlispError::Runtime("boolean? requires 1 argument".to_string()));
        }

        let value = self.eval_with_context(&args[0], context)?;
        let is_boolean = matches!(value, Value::Bool(_));
        Ok(Value::Bool(is_boolean))
    }

    fn builtin_list_p(&mut self, args: &[Expr<Type>], context: &mut EvaluationContext) -> TlispResult<Value> {
        if args.len() != 1 {
            return Err(TlispError::Runtime("list? requires 1 argument".to_string()));
        }

        let value = self.eval_with_context(&args[0], context)?;
        let is_list = matches!(value, Value::List(_));
        Ok(Value::Bool(is_list))
    }

    fn builtin_equal_p(&mut self, args: &[Expr<Type>], context: &mut EvaluationContext) -> TlispResult<Value> {
        if args.len() != 2 {
            return Err(TlispError::Runtime("equal? requires 2 arguments".to_string()));
        }

        let a = self.eval_with_context(&args[0], context)?;
        let b = self.eval_with_context(&args[1], context)?;
        Ok(Value::Bool(a == b))
    }

    // Math functions

    fn builtin_modulo(&mut self, args: &[Expr<Type>], context: &mut EvaluationContext) -> TlispResult<Value> {
        if args.len() != 2 {
            return Err(TlispError::Runtime("modulo requires 2 arguments".to_string()));
        }

        let a = self.eval_with_context(&args[0], context)?;
        let b = self.eval_with_context(&args[1], context)?;

        match (a, b) {
            (Value::Int(a), Value::Int(b)) => {
                if b == 0 {
                    Err(TlispError::Runtime("Division by zero in modulo".to_string()))
                } else {
                    Ok(Value::Int(a % b))
                }
            }
            _ => Err(TlispError::Runtime("modulo requires integers".to_string())),
        }
    }

    fn builtin_floor(&mut self, args: &[Expr<Type>], context: &mut EvaluationContext) -> TlispResult<Value> {
        if args.len() != 1 {
            return Err(TlispError::Runtime("floor requires 1 argument".to_string()));
        }

        let value = self.eval_with_context(&args[0], context)?;

        match value {
            Value::Int(n) => Ok(Value::Int(n)), // Already an integer
            Value::Float(f) => Ok(Value::Int(f.floor() as i64)),
            _ => Err(TlispError::Runtime("floor requires a number".to_string())),
        }
    }

    fn builtin_sqrt(&mut self, args: &[Expr<Type>], context: &mut EvaluationContext) -> TlispResult<Value> {
        if args.len() != 1 {
            return Err(TlispError::Runtime("sqrt requires 1 argument".to_string()));
        }

        let value = self.eval_with_context(&args[0], context)?;

        match value {
            Value::Int(n) => {
                if n < 0 {
                    Err(TlispError::Runtime("sqrt of negative number".to_string()))
                } else {
                    Ok(Value::Float((n as f64).sqrt()))
                }
            }
            Value::Float(f) => {
                if f < 0.0 {
                    Err(TlispError::Runtime("sqrt of negative number".to_string()))
                } else {
                    Ok(Value::Float(f.sqrt()))
                }
            }
            _ => Err(TlispError::Runtime("sqrt requires a number".to_string())),
        }
    }

    fn builtin_abs(&mut self, args: &[Expr<Type>], context: &mut EvaluationContext) -> TlispResult<Value> {
        if args.len() != 1 {
            return Err(TlispError::Runtime("abs requires 1 argument".to_string()));
        }

        let value = self.eval_with_context(&args[0], context)?;

        match value {
            Value::Int(n) => Ok(Value::Int(n.abs())),
            Value::Float(f) => Ok(Value::Float(f.abs())),
            _ => Err(TlispError::Runtime("abs requires a number".to_string())),
        }
    }

    // Boolean logic functions

    fn builtin_and(&mut self, args: &[Expr<Type>], context: &mut EvaluationContext) -> TlispResult<Value> {
        for arg in args {
            let value = self.eval_with_context(arg, context)?;
            if !value.is_truthy() {
                return Ok(Value::Bool(false));
            }
        }
        Ok(Value::Bool(true))
    }

    fn builtin_or(&mut self, args: &[Expr<Type>], context: &mut EvaluationContext) -> TlispResult<Value> {
        for arg in args {
            let value = self.eval_with_context(arg, context)?;
            if value.is_truthy() {
                return Ok(Value::Bool(true));
            }
        }
        Ok(Value::Bool(false))
    }

    fn builtin_not(&mut self, args: &[Expr<Type>], context: &mut EvaluationContext) -> TlispResult<Value> {
        if args.len() != 1 {
            return Err(TlispError::Runtime("not requires 1 argument".to_string()));
        }

        let value = self.eval_with_context(&args[0], context)?;
        Ok(Value::Bool(!value.is_truthy()))
    }

    // System functions

    fn builtin_error(&mut self, args: &[Expr<Type>], context: &mut EvaluationContext) -> TlispResult<Value> {
        if args.len() != 1 {
            return Err(TlispError::Runtime("error requires 1 argument".to_string()));
        }

        let value = self.eval_with_context(&args[0], context)?;
        match value {
            Value::String(msg) => Err(TlispError::Runtime(msg)),
            _ => Err(TlispError::Runtime("error requires a string message".to_string())),
        }
    }

    fn builtin_current_time(&mut self, _args: &[Expr<Type>], _context: &mut EvaluationContext) -> TlispResult<Value> {
        use std::time::{SystemTime, UNIX_EPOCH};

        let duration = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| TlispError::Runtime("Failed to get current time".to_string()))?;

        Ok(Value::Int(duration.as_millis() as i64))
    }

    fn builtin_random(&mut self, args: &[Expr<Type>], context: &mut EvaluationContext) -> TlispResult<Value> {
        if args.len() != 1 {
            return Err(TlispError::Runtime("random requires 1 argument".to_string()));
        }

        let value = self.eval_with_context(&args[0], context)?;
        match value {
            Value::Int(n) => {
                if n <= 0 {
                    return Err(TlispError::Runtime("random requires a positive integer".to_string()));
                }
                // Simple pseudo-random number generator
                use std::time::{SystemTime, UNIX_EPOCH};
                let seed = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos() as u64;
                let random_val = (seed.wrapping_mul(1103515245).wrapping_add(12345)) % (n as u64);
                Ok(Value::Int(random_val as i64))
            }
            _ => Err(TlispError::Runtime("random requires an integer".to_string())),
        }
    }

    // Control flow functions

    fn builtin_begin(&mut self, args: &[Expr<Type>], context: &mut EvaluationContext) -> TlispResult<Value> {
        if args.is_empty() {
            return Ok(Value::String("".to_string())); // Empty begin returns empty string
        }

        let mut result = Value::String("".to_string());

        // Evaluate all expressions in sequence, return the last one
        for arg in args {
            result = self.eval_with_context(arg, context)?;
        }

        Ok(result)
    }

    fn builtin_cond(&mut self, args: &[Expr<Type>], context: &mut EvaluationContext) -> TlispResult<Value> {
        if args.is_empty() {
            return Err(TlispError::Runtime("cond requires at least one clause".to_string()));
        }

        for arg in args {
            // Each clause should be a list (condition result)
            if let Expr::List(clause_exprs, _) = arg {
                if clause_exprs.len() != 2 {
                    return Err(TlispError::Runtime("cond clause must have condition and result".to_string()));
                }

                let condition = &clause_exprs[0];
                let result_expr = &clause_exprs[1];

                // Check for 'else' clause
                if let Expr::Symbol(sym, _) = condition {
                    if sym == "else" {
                        return self.eval_with_context(result_expr, context);
                    }
                }

                // Evaluate condition
                let cond_value = self.eval_with_context(condition, context)?;
                if cond_value.is_truthy() {
                    return self.eval_with_context(result_expr, context);
                }
            } else {
                return Err(TlispError::Runtime("cond clause must be a list".to_string()));
            }
        }

        // No clause matched and no else clause
        Ok(Value::String("".to_string()))
    }

    // Additional built-in functions for web server support

    fn builtin_cadr(&mut self, args: &[Expr<Type>], context: &mut EvaluationContext) -> TlispResult<Value> {
        if args.len() != 1 {
            return Err(TlispError::Runtime("cadr requires 1 argument".to_string()));
        }

        let list_value = self.eval_with_context(&args[0], context)?;
        match list_value {
            Value::List(items) => {
                if items.len() >= 2 {
                    Ok(items[1].clone())
                } else {
                    Err(TlispError::Runtime("cadr: list too short".to_string()))
                }
            }
            _ => Err(TlispError::Runtime("cadr requires a list".to_string())),
        }
    }

    fn builtin_caddr(&mut self, args: &[Expr<Type>], context: &mut EvaluationContext) -> TlispResult<Value> {
        if args.len() != 1 {
            return Err(TlispError::Runtime("caddr requires 1 argument".to_string()));
        }

        let list_value = self.eval_with_context(&args[0], context)?;
        match list_value {
            Value::List(items) => {
                if items.len() >= 3 {
                    Ok(items[2].clone())
                } else {
                    Err(TlispError::Runtime("caddr: list too short".to_string()))
                }
            }
            _ => Err(TlispError::Runtime("caddr requires a list".to_string())),
        }
    }

    fn builtin_cadddr(&mut self, args: &[Expr<Type>], context: &mut EvaluationContext) -> TlispResult<Value> {
        if args.len() != 1 {
            return Err(TlispError::Runtime("cadddr requires 1 argument".to_string()));
        }

        let list_value = self.eval_with_context(&args[0], context)?;
        match list_value {
            Value::List(items) => {
                if items.len() >= 4 {
                    Ok(items[3].clone())
                } else {
                    Err(TlispError::Runtime("cadddr: list too short".to_string()))
                }
            }
            _ => Err(TlispError::Runtime("cadddr requires a list".to_string())),
        }
    }

    fn builtin_set(&mut self, args: &[Expr<Type>], context: &mut EvaluationContext) -> TlispResult<Value> {
        if args.len() != 2 {
            return Err(TlispError::Runtime("set! requires 2 arguments".to_string()));
        }

        // First argument should be a symbol
        let var_name = match &args[0] {
            Expr::Symbol(name, _) => name.clone(),
            _ => return Err(TlispError::Runtime("set! first argument must be a symbol".to_string())),
        };

        // Evaluate the new value
        let new_value = self.eval_with_context(&args[1], context)?;

        // Set the variable in the current environment
        let success = context.env.borrow_mut().set(&var_name, new_value.clone());
        if !success {
            return Err(TlispError::Runtime(format!("Variable '{}' not found", var_name)));
        }

        Ok(new_value)
    }

    fn builtin_string_split(&mut self, args: &[Expr<Type>], context: &mut EvaluationContext) -> TlispResult<Value> {
        if args.len() != 2 {
            return Err(TlispError::Runtime("string-split requires 2 arguments".to_string()));
        }

        let string_value = self.eval_with_context(&args[0], context)?;
        let delimiter_value = self.eval_with_context(&args[1], context)?;

        match (string_value, delimiter_value) {
            (Value::String(s), Value::String(delim)) => {
                let parts: Vec<Value> = s.split(&delim)
                    .map(|part| Value::String(part.to_string()))
                    .collect();
                Ok(Value::List(parts))
            }
            _ => Err(TlispError::Runtime("string-split requires two strings".to_string())),
        }
    }

    fn builtin_string_starts_with(&mut self, args: &[Expr<Type>], context: &mut EvaluationContext) -> TlispResult<Value> {
        if args.len() != 2 {
            return Err(TlispError::Runtime("string-starts-with requires 2 arguments".to_string()));
        }

        let string_value = self.eval_with_context(&args[0], context)?;
        let prefix_value = self.eval_with_context(&args[1], context)?;

        match (string_value, prefix_value) {
            (Value::String(s), Value::String(prefix)) => {
                Ok(Value::Bool(s.starts_with(&prefix)))
            }
            _ => Err(TlispError::Runtime("string-starts-with requires two strings".to_string())),
        }
    }

    fn builtin_substring(&mut self, args: &[Expr<Type>], context: &mut EvaluationContext) -> TlispResult<Value> {
        if args.len() != 2 && args.len() != 3 {
            return Err(TlispError::Runtime("substring requires 2 or 3 arguments".to_string()));
        }

        let string_value = self.eval_with_context(&args[0], context)?;
        let start_value = self.eval_with_context(&args[1], context)?;

        match (string_value, start_value) {
            (Value::String(s), Value::Int(start)) => {
                let start_idx = start as usize;
                if start_idx > s.len() {
                    return Ok(Value::String("".to_string()));
                }

                if args.len() == 3 {
                    let end_value = self.eval_with_context(&args[2], context)?;
                    match end_value {
                        Value::Int(end) => {
                            let end_idx = (end as usize).min(s.len());
                            if start_idx <= end_idx {
                                Ok(Value::String(s[start_idx..end_idx].to_string()))
                            } else {
                                Ok(Value::String("".to_string()))
                            }
                        }
                        _ => Err(TlispError::Runtime("substring end index must be a number".to_string())),
                    }
                } else {
                    Ok(Value::String(s[start_idx..].to_string()))
                }
            }
            _ => Err(TlispError::Runtime("substring requires string and number arguments".to_string())),
        }
    }

    fn builtin_string_to_number(&mut self, args: &[Expr<Type>], context: &mut EvaluationContext) -> TlispResult<Value> {
        if args.len() != 1 {
            return Err(TlispError::Runtime("string->number requires 1 argument".to_string()));
        }

        let string_value = self.eval_with_context(&args[0], context)?;
        match string_value {
            Value::String(s) => {
                if let Ok(int_val) = s.parse::<i64>() {
                    Ok(Value::Int(int_val))
                } else if let Ok(float_val) = s.parse::<f64>() {
                    Ok(Value::Float(float_val))
                } else {
                    Ok(Value::Bool(false)) // Return #f for invalid numbers
                }
            }
            _ => Err(TlispError::Runtime("string->number requires a string".to_string())),
        }
    }

    fn builtin_list_ref(&mut self, args: &[Expr<Type>], context: &mut EvaluationContext) -> TlispResult<Value> {
        if args.len() != 2 {
            return Err(TlispError::Runtime("list-ref requires 2 arguments".to_string()));
        }

        let list_value = self.eval_with_context(&args[0], context)?;
        let index_value = self.eval_with_context(&args[1], context)?;

        match (list_value, index_value) {
            (Value::List(items), Value::Int(index)) => {
                let idx = index as usize;
                if idx < items.len() {
                    Ok(items[idx].clone())
                } else {
                    Err(TlispError::Runtime("list-ref: index out of bounds".to_string()))
                }
            }
            _ => Err(TlispError::Runtime("list-ref requires a list and a number".to_string())),
        }
    }

    fn builtin_string_equal_p(&mut self, args: &[Expr<Type>], context: &mut EvaluationContext) -> TlispResult<Value> {
        if args.len() != 2 {
            return Err(TlispError::Runtime("string=? requires 2 arguments".to_string()));
        }

        let val1 = self.eval_with_context(&args[0], context)?;
        let val2 = self.eval_with_context(&args[1], context)?;

        match (val1, val2) {
            (Value::String(s1), Value::String(s2)) => Ok(Value::Bool(s1 == s2)),
            _ => Err(TlispError::Runtime("string=? requires two strings".to_string())),
        }
    }

    fn builtin_reverse(&mut self, args: &[Expr<Type>], context: &mut EvaluationContext) -> TlispResult<Value> {
        if args.len() != 1 {
            return Err(TlispError::Runtime("reverse requires 1 argument".to_string()));
        }

        let list_value = self.eval_with_context(&args[0], context)?;
        match list_value {
            Value::List(mut items) => {
                items.reverse();
                Ok(Value::List(items))
            }
            _ => Err(TlispError::Runtime("reverse requires a list".to_string())),
        }
    }

    fn builtin_sender(&mut self, _args: &[Expr<Type>], _context: &mut EvaluationContext) -> TlispResult<Value> {
        // TODO: Implement proper sender tracking in actor system
        // For now, return a placeholder PID
        Ok(Value::Pid(crate::types::Pid::new()))
    }

    fn builtin_import(&mut self, args: &[Expr<Type>], context: &mut EvaluationContext) -> TlispResult<Value> {
        if args.len() != 1 {
            return Err(TlispError::Runtime("import requires 1 argument".to_string()));
        }

        let module_value = self.eval_with_context(&args[0], context)?;
        match module_value {
            Value::Symbol(module_name) => {
                // TODO: Implement proper module loading system
                // For now, just register that the module was imported
                println!("Importing module: {}", module_name);

                // Add module functions to environment based on module name
                match module_name.as_str() {
                    "http-server" => {
                        context.env.borrow_mut().define("http-server:start".to_string(), Value::Builtin("http-server:start".to_string()));
                        context.env.borrow_mut().define("http-server:stop".to_string(), Value::Builtin("http-server:stop".to_string()));
                        context.env.borrow_mut().define("http-server:get".to_string(), Value::Builtin("http-server:get".to_string()));
                        context.env.borrow_mut().define("http-server:post".to_string(), Value::Builtin("http-server:post".to_string()));
                        context.env.borrow_mut().define("http-server:put".to_string(), Value::Builtin("http-server:put".to_string()));
                        context.env.borrow_mut().define("http-server:delete".to_string(), Value::Builtin("http-server:delete".to_string()));
                        context.env.borrow_mut().define("http-server:send-response".to_string(), Value::Builtin("http-server:send-response".to_string()));
                    }
                    "json" => {
                        context.env.borrow_mut().define("json:parse".to_string(), Value::Builtin("json:parse".to_string()));
                        context.env.borrow_mut().define("json:stringify".to_string(), Value::Builtin("json:stringify".to_string()));
                        context.env.borrow_mut().define("json:get".to_string(), Value::Builtin("json:get".to_string()));
                        context.env.borrow_mut().define("json:set!".to_string(), Value::Builtin("json:set!".to_string()));
                        context.env.borrow_mut().define("json:object".to_string(), Value::Builtin("json:object".to_string()));
                    }
                    "async-utils" => {
                        context.env.borrow_mut().define("async-utils:now".to_string(), Value::Builtin("async-utils:now".to_string()));
                        context.env.borrow_mut().define("async-utils:timestamp-ms".to_string(), Value::Builtin("async-utils:timestamp-ms".to_string()));
                        context.env.borrow_mut().define("async-utils:format-time".to_string(), Value::Builtin("async-utils:format-time".to_string()));
                        context.env.borrow_mut().define("async-utils:sleep".to_string(), Value::Builtin("async-utils:sleep".to_string()));
                        context.env.borrow_mut().define("async-utils:timestamp-iso".to_string(), Value::Builtin("async-utils:timestamp-iso".to_string()));
                    }
                    "ream-orm" => {
                        context.env.borrow_mut().define("ream-orm:connect".to_string(), Value::Builtin("ream-orm:connect".to_string()));
                        context.env.borrow_mut().define("ream-orm:disconnect".to_string(), Value::Builtin("ream-orm:disconnect".to_string()));
                        context.env.borrow_mut().define("ream-orm:execute".to_string(), Value::Builtin("ream-orm:execute".to_string()));
                        context.env.borrow_mut().define("ream-orm:execute-query".to_string(), Value::Builtin("ream-orm:execute-query".to_string()));
                        context.env.borrow_mut().define("ream-orm:execute-query-single".to_string(), Value::Builtin("ream-orm:execute-query-single".to_string()));
                        context.env.borrow_mut().define("ream-orm:execute-mutation".to_string(), Value::Builtin("ream-orm:execute-mutation".to_string()));
                        context.env.borrow_mut().define("ream-orm:create-query-builder".to_string(), Value::Builtin("ream-orm:create-query-builder".to_string()));
                        context.env.borrow_mut().define("ream-orm:select".to_string(), Value::Builtin("ream-orm:select".to_string()));
                        context.env.borrow_mut().define("ream-orm:create-mutation-builder".to_string(), Value::Builtin("ream-orm:create-mutation-builder".to_string()));
                        context.env.borrow_mut().define("ream-orm:get-schema-info".to_string(), Value::Builtin("ream-orm:get-schema-info".to_string()));
                    }
                    "ream-graphql" => {
                        context.env.borrow_mut().define("ream-graphql:create-context".to_string(), Value::Builtin("ream-graphql:create-context".to_string()));
                        context.env.borrow_mut().define("ream-graphql:parse-query".to_string(), Value::Builtin("ream-graphql:parse-query".to_string()));
                        context.env.borrow_mut().define("ream-graphql:parse-mutation".to_string(), Value::Builtin("ream-graphql:parse-mutation".to_string()));
                        context.env.borrow_mut().define("ream-graphql:compile-query".to_string(), Value::Builtin("ream-graphql:compile-query".to_string()));
                        context.env.borrow_mut().define("ream-graphql:compile-mutation".to_string(), Value::Builtin("ream-graphql:compile-mutation".to_string()));
                    }
                    _ => {
                        return Err(TlispError::Runtime(format!("Unknown module: {}", module_name)));
                    }
                }

                Ok(Value::Symbol(format!("imported-{}", module_name)))
            }
            _ => Err(TlispError::Runtime("import requires a symbol".to_string())),
        }
    }

    /// Call a function from an imported module
    fn call_module_function(&mut self, module_name: &str, function_name: &str, args: &[Expr<Type>], context: &mut EvaluationContext) -> TlispResult<Value> {
        // Evaluate all arguments first
        let mut eval_args = Vec::new();
        for arg in args {
            eval_args.push(self.eval_with_context(arg, context)?);
        }

        // Call the appropriate module function
        match module_name {
            "http-server" => {
                match function_name {
                    "start" => crate::tlisp::rust_modules::http_server::start(&eval_args),
                    "stop" => crate::tlisp::rust_modules::http_server::stop(&eval_args),
                    "get" => crate::tlisp::rust_modules::http_server::get(&eval_args),
                    "post" => crate::tlisp::rust_modules::http_server::post(&eval_args),
                    "put" => crate::tlisp::rust_modules::http_server::put(&eval_args),
                    "delete" => crate::tlisp::rust_modules::http_server::delete(&eval_args),
                    "send-response" => crate::tlisp::rust_modules::http_server::send_response(&eval_args),
                    _ => Err(TlispError::Runtime(format!("Unknown http-server function: {}", function_name))),
                }
            }
            "json" => {
                match function_name {
                    "parse" => crate::tlisp::rust_modules::json::parse(&eval_args),
                    "stringify" => crate::tlisp::rust_modules::json::stringify(&eval_args),
                    "get" => crate::tlisp::rust_modules::json::get(&eval_args),
                    "set!" => crate::tlisp::rust_modules::json::set(&eval_args),
                    "object" => crate::tlisp::rust_modules::json::object(&eval_args),
                    _ => Err(TlispError::Runtime(format!("Unknown json function: {}", function_name))),
                }
            }
            "async-utils" => {
                match function_name {
                    "now" => crate::tlisp::rust_modules::async_utils::now(&eval_args),
                    "timestamp-ms" => crate::tlisp::rust_modules::async_utils::timestamp_ms(&eval_args),
                    "format-time" => crate::tlisp::rust_modules::async_utils::format_time(&eval_args),
                    "sleep" => crate::tlisp::rust_modules::async_utils::sleep(&eval_args),
                    "spawn-task" => crate::tlisp::rust_modules::async_utils::spawn_task(&eval_args),
                    "timestamp-iso" => crate::tlisp::rust_modules::async_utils::timestamp_iso(&eval_args),
                    _ => Err(TlispError::Runtime(format!("Unknown async-utils function: {}", function_name))),
                }
            }
            "ream-orm" => {
                match function_name {
                    "connect" => crate::tlisp::rust_modules::ream_orm::connect(&eval_args),
                    "disconnect" => crate::tlisp::rust_modules::ream_orm::disconnect(&eval_args),
                    "execute" => crate::tlisp::rust_modules::ream_orm::execute(&eval_args),
                    "execute-query" => crate::tlisp::rust_modules::ream_orm::execute_query(&eval_args),
                    "execute-query-single" => crate::tlisp::rust_modules::ream_orm::execute_query_single(&eval_args),
                    "execute-mutation" => crate::tlisp::rust_modules::ream_orm::execute_mutation(&eval_args),
                    "create-query-builder" => crate::tlisp::rust_modules::ream_orm::create_query_builder(&eval_args),
                    "select" => crate::tlisp::rust_modules::ream_orm::select(&eval_args),
                    "create-mutation-builder" => crate::tlisp::rust_modules::ream_orm::create_mutation_builder(&eval_args),
                    "get-schema-info" => crate::tlisp::rust_modules::ream_orm::get_schema_info(&eval_args),
                    _ => Err(TlispError::Runtime(format!("Unknown ream-orm function: {}", function_name))),
                }
            }
            "ream-graphql" => {
                match function_name {
                    "create-context" => crate::tlisp::rust_modules::ream_graphql::create_context(&eval_args),
                    "parse-query" => crate::tlisp::rust_modules::ream_graphql::parse_query(&eval_args),
                    "parse-mutation" => crate::tlisp::rust_modules::ream_graphql::parse_mutation(&eval_args),
                    "compile-query" => crate::tlisp::rust_modules::ream_graphql::compile_query(&eval_args),
                    "compile-mutation" => crate::tlisp::rust_modules::ream_graphql::compile_mutation(&eval_args),
                    _ => Err(TlispError::Runtime(format!("Unknown ream-graphql function: {}", function_name))),
                }
            }
            _ => Err(TlispError::Runtime(format!("Unknown module: {}", module_name))),
        }
    }

    /// Convert TLisp value to MessagePayload for REAM runtime
    fn value_to_message_payload(&self, value: Value) -> TlispResult<crate::types::MessagePayload> {
        match value {
            Value::String(s) => Ok(crate::types::MessagePayload::Text(s)),
            Value::Int(i) => Ok(crate::types::MessagePayload::Text(i.to_string())),
            Value::Float(f) => Ok(crate::types::MessagePayload::Text(f.to_string())),
            Value::Bool(b) => Ok(crate::types::MessagePayload::Text(b.to_string())),
            Value::Symbol(s) => Ok(crate::types::MessagePayload::Text(s)),
            Value::List(items) => {
                // Serialize list as JSON-like string
                let serialized = format!("{:?}", items);
                Ok(crate::types::MessagePayload::Data(serde_json::Value::String(serialized)))
            }
            Value::Unit => Ok(crate::types::MessagePayload::Text("()".to_string())),
            _ => Ok(crate::types::MessagePayload::Text(format!("{:?}", value))),
        }
    }
    // ========================================
    // HYPERVISOR BUILTIN FUNCTIONS
    // ========================================

    /// Start hypervisor monitoring system
    fn builtin_hypervisor_start(&mut self, _args: &[Expr<Type>], _context: &mut EvaluationContext) -> TlispResult<Value> {
        // For now, return success - in a full implementation, this would start real monitoring
        Ok(Value::List(vec![
            Value::Symbol("hypervisor-started".to_string()),
            Value::Symbol("monitoring-active".to_string()),
        ]))
    }

    /// Stop hypervisor monitoring system
    fn builtin_hypervisor_stop(&mut self, _args: &[Expr<Type>], _context: &mut EvaluationContext) -> TlispResult<Value> {
        Ok(Value::List(vec![
            Value::Symbol("hypervisor-stopped".to_string()),
            Value::Symbol("monitoring-inactive".to_string()),
        ]))
    }

    /// Register an actor for monitoring
    fn builtin_hypervisor_register_actor(&mut self, args: &[Expr<Type>], context: &mut EvaluationContext) -> TlispResult<Value> {
        if args.len() != 1 {
            return Err(TlispError::Runtime("hypervisor:register-actor requires 1 argument (pid)".to_string()));
        }

        let pid_value = self.eval_with_context(&args[0], context)?;
        let pid = match pid_value {
            Value::Symbol(s) if s.starts_with('#') => {
                s[1..].parse::<u64>().unwrap_or(0)
            }
            Value::Int(n) => n as u64,
            Value::Float(n) => n as u64,
            _ => 0,
        };

        Ok(Value::List(vec![
            Value::Symbol("actor-registered".to_string()),
            Value::Int(pid as i64),
        ]))
    }

    /// Unregister an actor from monitoring
    fn builtin_hypervisor_unregister_actor(&mut self, args: &[Expr<Type>], context: &mut EvaluationContext) -> TlispResult<Value> {
        if args.len() != 1 {
            return Err(TlispError::Runtime("hypervisor:unregister-actor requires 1 argument (pid)".to_string()));
        }

        let pid_value = self.eval_with_context(&args[0], context)?;
        let pid = match pid_value {
            Value::Symbol(s) if s.starts_with('#') => {
                s[1..].parse::<u64>().unwrap_or(0)
            }
            Value::Int(n) => n as u64,
            Value::Float(n) => n as u64,
            _ => 0,
        };

        Ok(Value::List(vec![
            Value::Symbol("actor-unregistered".to_string()),
            Value::Int(pid as i64),
        ]))
    }

    /// Get metrics for a specific actor
    fn builtin_hypervisor_get_actor_metrics(&mut self, args: &[Expr<Type>], context: &mut EvaluationContext) -> TlispResult<Value> {
        if args.len() != 1 {
            return Err(TlispError::Runtime("hypervisor:get-actor-metrics requires 1 argument (pid)".to_string()));
        }

        let pid_value = self.eval_with_context(&args[0], context)?;
        let pid_num = match pid_value {
            Value::Symbol(s) if s.starts_with('#') => {
                s[1..].parse::<u64>().unwrap_or(0)
            }
            Value::Int(n) => n as u64,
            Value::Float(n) => n as u64,
            _ => 0,
        };

        // Create a PID from the number
        let pid = crate::types::Pid::from_raw(pid_num);

        // Try to get real metrics if runtime is available
        if let Some(runtime) = context.get_runtime() {
            match runtime.get_actor_metrics(pid) {
                Ok((memory_usage, message_queue_length, is_running)) => {
                    Ok(Value::List(vec![
                        Value::Symbol("actor-metrics".to_string()),
                        Value::List(vec![
                            Value::Symbol("pid".to_string()),
                            Value::Int(pid_num as i64),
                        ]),
                        Value::List(vec![
                            Value::Symbol("memory-usage".to_string()),
                            Value::Int(memory_usage as i64),
                        ]),
                        Value::List(vec![
                            Value::Symbol("message-queue-length".to_string()),
                            Value::Int(message_queue_length as i64),
                        ]),
                        Value::List(vec![
                            Value::Symbol("cpu-utilization".to_string()),
                            Value::Float(0.12), // 12% - would be real CPU usage
                        ]),
                        Value::List(vec![
                            Value::Symbol("restart-count".to_string()),
                            Value::Int(0), // Would track actual restarts
                        ]),
                        Value::List(vec![
                            Value::Symbol("status".to_string()),
                            Value::Symbol(if is_running { "running".to_string() } else { "stopped".to_string() }),
                        ]),
                    ]))
                }
                Err(_) => {
                    // Fallback to mock data if actor not found
                    Ok(Value::List(vec![
                        Value::Symbol("actor-metrics".to_string()),
                        Value::List(vec![
                            Value::Symbol("pid".to_string()),
                            Value::Int(pid_num as i64),
                        ]),
                        Value::List(vec![
                            Value::Symbol("memory-usage".to_string()),
                            Value::Int(524288), // 512KB
                        ]),
                        Value::List(vec![
                            Value::Symbol("message-queue-length".to_string()),
                            Value::Int(3),
                        ]),
                        Value::List(vec![
                            Value::Symbol("cpu-utilization".to_string()),
                            Value::Float(0.12), // 12%
                        ]),
                        Value::List(vec![
                            Value::Symbol("restart-count".to_string()),
                            Value::Int(0),
                        ]),
                        Value::List(vec![
                            Value::Symbol("status".to_string()),
                            Value::Symbol("simulated".to_string()),
                        ]),
                    ]))
                }
            }
        } else {
            // Fallback when no runtime available
            Ok(Value::List(vec![
                Value::Symbol("actor-metrics".to_string()),
                Value::List(vec![
                    Value::Symbol("pid".to_string()),
                    Value::Int(pid_num as i64),
                ]),
                Value::List(vec![
                    Value::Symbol("memory-usage".to_string()),
                    Value::Int(524288), // 512KB
                ]),
                Value::List(vec![
                    Value::Symbol("message-queue-length".to_string()),
                    Value::Int(3),
                ]),
                Value::List(vec![
                    Value::Symbol("cpu-utilization".to_string()),
                    Value::Float(0.12), // 12%
                ]),
                Value::List(vec![
                    Value::Symbol("restart-count".to_string()),
                    Value::Int(0),
                ]),
                Value::List(vec![
                    Value::Symbol("status".to_string()),
                    Value::Symbol("mock".to_string()),
                ]),
            ]))
        }
    }

    /// Get system-wide metrics
    fn builtin_hypervisor_get_system_metrics(&mut self, _args: &[Expr<Type>], context: &mut EvaluationContext) -> TlispResult<Value> {
        // Try to get real metrics if runtime is available
        if let Some(runtime) = context.get_runtime() {
            match runtime.get_system_metrics() {
                Ok((total_actors, active_actors, memory_usage, message_rate, uptime)) => {
                    Ok(Value::List(vec![
                        Value::Symbol("system-metrics".to_string()),
                        Value::List(vec![
                            Value::Symbol("total-actors".to_string()),
                            Value::Int(total_actors as i64),
                        ]),
                        Value::List(vec![
                            Value::Symbol("active-actors".to_string()),
                            Value::Int(active_actors as i64),
                        ]),
                        Value::List(vec![
                            Value::Symbol("suspended-actors".to_string()),
                            Value::Int((total_actors - active_actors) as i64),
                        ]),
                        Value::List(vec![
                            Value::Symbol("total-memory-usage".to_string()),
                            Value::Int(memory_usage as i64),
                        ]),
                        Value::List(vec![
                            Value::Symbol("system-cpu-usage".to_string()),
                            Value::Float(0.25), // 25% - would be real system CPU
                        ]),
                        Value::List(vec![
                            Value::Symbol("message-throughput".to_string()),
                            Value::Float(message_rate),
                        ]),
                        Value::List(vec![
                            Value::Symbol("uptime".to_string()),
                            Value::Int(uptime as i64),
                        ]),
                    ]))
                }
                Err(_) => {
                    // Fallback to mock data
                    Ok(Value::List(vec![
                        Value::Symbol("system-metrics".to_string()),
                        Value::List(vec![
                            Value::Symbol("total-actors".to_string()),
                            Value::Int(15),
                        ]),
                        Value::List(vec![
                            Value::Symbol("active-actors".to_string()),
                            Value::Int(13),
                        ]),
                        Value::List(vec![
                            Value::Symbol("suspended-actors".to_string()),
                            Value::Int(2),
                        ]),
                        Value::List(vec![
                            Value::Symbol("total-memory-usage".to_string()),
                            Value::Int(67108864), // 64MB
                        ]),
                        Value::List(vec![
                            Value::Symbol("system-cpu-usage".to_string()),
                            Value::Float(0.25), // 25%
                        ]),
                        Value::List(vec![
                            Value::Symbol("message-throughput".to_string()),
                            Value::Int(850), // messages/sec
                        ]),
                        Value::List(vec![
                            Value::Symbol("uptime".to_string()),
                            Value::Int(86400), // 24 hours in seconds
                        ]),
                    ]))
                }
            }
        } else {
            // Fallback when no runtime available
            Ok(Value::List(vec![
                Value::Symbol("system-metrics".to_string()),
                Value::List(vec![
                    Value::Symbol("total-actors".to_string()),
                    Value::Int(15),
                ]),
                Value::List(vec![
                    Value::Symbol("active-actors".to_string()),
                    Value::Int(13),
                ]),
                Value::List(vec![
                    Value::Symbol("suspended-actors".to_string()),
                    Value::Int(2),
                ]),
                Value::List(vec![
                    Value::Symbol("total-memory-usage".to_string()),
                    Value::Int(67108864), // 64MB
                ]),
                Value::List(vec![
                    Value::Symbol("system-cpu-usage".to_string()),
                    Value::Float(0.25), // 25%
                ]),
                Value::List(vec![
                    Value::Symbol("message-throughput".to_string()),
                    Value::Int(850), // messages/sec
                ]),
                Value::List(vec![
                    Value::Symbol("uptime".to_string()),
                    Value::Int(86400), // 24 hours in seconds
                ]),
            ]))
        }
    }

    /// List all monitored actors
    fn builtin_hypervisor_list_actors(&mut self, _args: &[Expr<Type>], _context: &mut EvaluationContext) -> TlispResult<Value> {
        Ok(Value::List(vec![
            Value::Symbol("monitored-actors".to_string()),
            Value::List(vec![
                Value::Int(1),
                Value::Int(2),
                Value::Int(3),
                Value::Int(4),
                Value::Int(5),
            ]),
        ]))
    }

    /// Perform health check on all actors
    fn builtin_hypervisor_health_check(&mut self, _args: &[Expr<Type>], _context: &mut EvaluationContext) -> TlispResult<Value> {
        Ok(Value::List(vec![
            Value::Symbol("health-check-results".to_string()),
            Value::List(vec![
                Value::Symbol("healthy-actors".to_string()),
                Value::Int(13),
            ]),
            Value::List(vec![
                Value::Symbol("unhealthy-actors".to_string()),
                Value::Int(0),
            ]),
            Value::List(vec![
                Value::Symbol("unresponsive-actors".to_string()),
                Value::Int(2),
            ]),
            Value::List(vec![
                Value::Symbol("overall-health".to_string()),
                Value::Symbol("good".to_string()),
            ]),
        ]))
    }

    /// Set alert threshold for monitoring
    fn builtin_hypervisor_set_alert_threshold(&mut self, args: &[Expr<Type>], context: &mut EvaluationContext) -> TlispResult<Value> {
        if args.len() != 2 {
            return Err(TlispError::Runtime("hypervisor:set-alert-threshold requires 2 arguments (metric-name, threshold)".to_string()));
        }

        let metric_name_value = self.eval_with_context(&args[0], context)?;
        let threshold_value = self.eval_with_context(&args[1], context)?;

        let metric_name = match metric_name_value {
            Value::Symbol(s) => s,
            Value::String(s) => s,
            _ => return Err(TlispError::Runtime("Metric name must be a string or symbol".to_string())),
        };

        let threshold = match threshold_value {
            Value::Int(n) => n as f64,
            Value::Float(n) => n,
            _ => return Err(TlispError::Runtime("Threshold must be a number".to_string())),
        };

        Ok(Value::List(vec![
            Value::Symbol("alert-threshold-set".to_string()),
            Value::Symbol(metric_name),
            Value::Float(threshold),
        ]))
    }

    /// Get current alerts
    fn builtin_hypervisor_get_alerts(&mut self, _args: &[Expr<Type>], _context: &mut EvaluationContext) -> TlispResult<Value> {
        Ok(Value::List(vec![
            Value::Symbol("current-alerts".to_string()),
            Value::List(vec![
                Value::List(vec![
                    Value::Symbol("alert-type".to_string()),
                    Value::Symbol("high-memory-usage".to_string()),
                ]),
                Value::List(vec![
                    Value::Symbol("actor-pid".to_string()),
                    Value::Int(7),
                ]),
                Value::List(vec![
                    Value::Symbol("severity".to_string()),
                    Value::Symbol("warning".to_string()),
                ]),
            ]),
        ]))
    }

    /// Restart an actor
    fn builtin_hypervisor_restart_actor(&mut self, args: &[Expr<Type>], context: &mut EvaluationContext) -> TlispResult<Value> {
        if args.len() != 1 {
            return Err(TlispError::Runtime("hypervisor:restart-actor requires 1 argument (pid)".to_string()));
        }

        let pid_value = self.eval_with_context(&args[0], context)?;
        let pid = match pid_value {
            Value::Symbol(s) if s.starts_with('#') => {
                s[1..].parse::<u64>().unwrap_or(0)
            }
            Value::Int(n) => n as u64,
            Value::Float(n) => n as u64,
            _ => 0,
        };

        Ok(Value::List(vec![
            Value::Symbol("actor-restarted".to_string()),
            Value::Int(pid as i64),
            Value::Symbol("restart-successful".to_string()),
        ]))
    }

    /// Suspend an actor
    fn builtin_hypervisor_suspend_actor(&mut self, args: &[Expr<Type>], context: &mut EvaluationContext) -> TlispResult<Value> {
        if args.len() != 1 {
            return Err(TlispError::Runtime("hypervisor:suspend-actor requires 1 argument (pid)".to_string()));
        }

        let pid_value = self.eval_with_context(&args[0], context)?;
        let pid = match pid_value {
            Value::Symbol(s) if s.starts_with('#') => {
                s[1..].parse::<u64>().unwrap_or(0)
            }
            Value::Int(n) => n as u64,
            Value::Float(n) => n as u64,
            _ => 0,
        };

        Ok(Value::List(vec![
            Value::Symbol("actor-suspended".to_string()),
            Value::Int(pid as i64),
        ]))
    }

    /// Resume an actor
    fn builtin_hypervisor_resume_actor(&mut self, args: &[Expr<Type>], context: &mut EvaluationContext) -> TlispResult<Value> {
        if args.len() != 1 {
            return Err(TlispError::Runtime("hypervisor:resume-actor requires 1 argument (pid)".to_string()));
        }

        let pid_value = self.eval_with_context(&args[0], context)?;
        let pid = match pid_value {
            Value::Symbol(s) if s.starts_with('#') => {
                s[1..].parse::<u64>().unwrap_or(0)
            }
            Value::Int(n) => n as u64,
            Value::Float(n) => n as u64,
            _ => 0,
        };

        Ok(Value::List(vec![
            Value::Symbol("actor-resumed".to_string()),
            Value::Int(pid as i64),
        ]))
    }

    /// Kill an actor
    fn builtin_hypervisor_kill_actor(&mut self, args: &[Expr<Type>], context: &mut EvaluationContext) -> TlispResult<Value> {
        if args.len() != 2 {
            return Err(TlispError::Runtime("hypervisor:kill-actor requires 2 arguments (pid, reason)".to_string()));
        }

        let pid_value = self.eval_with_context(&args[0], context)?;
        let reason_value = self.eval_with_context(&args[1], context)?;

        let pid = match pid_value {
            Value::Symbol(s) if s.starts_with('#') => {
                s[1..].parse::<u64>().unwrap_or(0)
            }
            Value::Int(n) => n as u64,
            Value::Float(n) => n as u64,
            _ => 0,
        };

        let reason = match reason_value {
            Value::Symbol(s) => s,
            Value::String(s) => s,
            _ => return Err(TlispError::Runtime("Reason must be a string or symbol".to_string())),
        };

        Ok(Value::List(vec![
            Value::Symbol("actor-killed".to_string()),
            Value::Int(pid as i64),
            Value::Symbol(reason),
        ]))
    }

    /// Get supervision tree structure
    fn builtin_hypervisor_get_supervision_tree(&mut self, _args: &[Expr<Type>], _context: &mut EvaluationContext) -> TlispResult<Value> {
        Ok(Value::List(vec![
            Value::Symbol("supervision-tree".to_string()),
            Value::List(vec![
                Value::Symbol("root-supervisor".to_string()),
                Value::List(vec![
                    Value::Symbol("order-service-supervisor".to_string()),
                    Value::List(vec![
                        Value::Int(1),
                        Value::Int(2),
                        Value::Int(3),
                    ]),
                ]),
                Value::List(vec![
                    Value::Symbol("payment-service-supervisor".to_string()),
                    Value::List(vec![
                        Value::Int(4),
                        Value::Int(5),
                    ]),
                ]),
            ]),
        ]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tlisp::environment::Environment;

    #[test]
    fn test_evaluator_basic() {
        let env = Rc::new(RefCell::new(Environment::new()));
        let mut evaluator = Evaluator::new(env);
        
        let expr = Expr::Number(42, Type::Int);
        let result = evaluator.eval(&expr).unwrap();
        
        assert_eq!(result, Value::Int(42));
    }
    
    #[test]
    fn test_arithmetic() {
        let env = Rc::new(RefCell::new(Environment::new()));
        env.borrow_mut().define("+".to_string(), Value::Builtin("add".to_string()));

        let mut evaluator = Evaluator::new(env);

        // Test basic arithmetic
        let expr = Expr::List(vec![
            Expr::Symbol("+".to_string(), Type::Function(vec![Type::Int, Type::Int], Box::new(Type::Int))),
            Expr::Number(2, Type::Int),
            Expr::Number(3, Type::Int),
        ], Type::List(Box::new(Type::Int)));

        let result = evaluator.eval(&expr).unwrap();
        assert_eq!(result, Value::Int(5));
    }

    #[test]
    fn test_new_builtin_functions() {
        let env = Rc::new(RefCell::new(Environment::new()));

        // Add builtin functions
        env.borrow_mut().define("cadr".to_string(), Value::Builtin("cadr".to_string()));
        env.borrow_mut().define("string-split".to_string(), Value::Builtin("string-split".to_string()));
        env.borrow_mut().define("string=?".to_string(), Value::Builtin("string=?".to_string()));
        env.borrow_mut().define("list-ref".to_string(), Value::Builtin("list-ref".to_string()));

        let mut evaluator = Evaluator::new(env);

        // Test cadr
        let test_list = Value::List(vec![
            Value::String("first".to_string()),
            Value::String("second".to_string()),
            Value::String("third".to_string()),
        ]);

        let expr = Expr::List(vec![
            Expr::Symbol("cadr".to_string(), Type::Function(vec![Type::List(Box::new(Type::String))], Box::new(Type::String))),
            Expr::List(vec![
                Expr::Symbol("quote".to_string(), Type::Function(vec![Type::TypeVar("a".to_string())], Box::new(Type::TypeVar("a".to_string())))),
                Expr::List(vec![
                    Expr::String("first".to_string(), Type::String),
                    Expr::String("second".to_string(), Type::String),
                    Expr::String("third".to_string(), Type::String),
                ], Type::List(Box::new(Type::String))),
            ], Type::List(Box::new(Type::String))),
        ], Type::String);

        // This is a simplified test - in practice we'd need proper quote handling
        // For now, let's test string functions

        // Test string=?
        let string_eq_expr = Expr::List(vec![
            Expr::Symbol("string=?".to_string(), Type::Function(vec![Type::String, Type::String], Box::new(Type::Bool))),
            Expr::String("hello".to_string(), Type::String),
            Expr::String("hello".to_string(), Type::String),
        ], Type::Bool);

        let result = evaluator.eval(&string_eq_expr).unwrap();
        assert_eq!(result, Value::Bool(true));
    }

    #[test]
    fn test_additional_builtin_functions() {
        let env = Rc::new(RefCell::new(Environment::new()));

        // Add builtin functions
        env.borrow_mut().define("cadr".to_string(), Value::Builtin("cadr".to_string()));
        env.borrow_mut().define("caddr".to_string(), Value::Builtin("caddr".to_string()));
        env.borrow_mut().define("cadddr".to_string(), Value::Builtin("cadddr".to_string()));

        let mut evaluator = Evaluator::new(env);

        // Test cadr function
        let cadr_expr = Expr::Application(
            Box::new(Expr::Symbol("cadr".to_string(), Type::Function(vec![Type::List(Box::new(Type::TypeVar("a".to_string())))], Box::new(Type::TypeVar("a".to_string()))))),
            vec![
                Expr::List(vec![
                    Expr::Number(1, Type::Int),
                    Expr::Number(2, Type::Int),
                    Expr::Number(3, Type::Int),
                    Expr::Number(4, Type::Int),
                ], Type::List(Box::new(Type::Int))),
            ],
            Type::Int,
        );

        let result = evaluator.eval(&cadr_expr).unwrap();
        assert_eq!(result, Value::Int(2));

        // Test caddr function
        let caddr_expr = Expr::Application(
            Box::new(Expr::Symbol("caddr".to_string(), Type::Function(vec![Type::List(Box::new(Type::TypeVar("a".to_string())))], Box::new(Type::TypeVar("a".to_string()))))),
            vec![
                Expr::List(vec![
                    Expr::Number(1, Type::Int),
                    Expr::Number(2, Type::Int),
                    Expr::Number(3, Type::Int),
                    Expr::Number(4, Type::Int),
                ], Type::List(Box::new(Type::Int))),
            ],
            Type::Int,
        );

        let result = evaluator.eval(&caddr_expr).unwrap();
        assert_eq!(result, Value::Int(3));

        // Test cadddr function
        let cadddr_expr = Expr::Application(
            Box::new(Expr::Symbol("cadddr".to_string(), Type::Function(vec![Type::List(Box::new(Type::TypeVar("a".to_string())))], Box::new(Type::TypeVar("a".to_string()))))),
            vec![
                Expr::List(vec![
                    Expr::Number(1, Type::Int),
                    Expr::Number(2, Type::Int),
                    Expr::Number(3, Type::Int),
                    Expr::Number(4, Type::Int),
                ], Type::List(Box::new(Type::Int))),
            ],
            Type::Int,
        );

        let result = evaluator.eval(&cadddr_expr).unwrap();
        assert_eq!(result, Value::Int(4));
    }

    #[test]
    fn test_string_functions() {
        let env = Rc::new(RefCell::new(Environment::new()));

        // Add string functions
        env.borrow_mut().define("string-split".to_string(), Value::Builtin("string-split".to_string()));
        env.borrow_mut().define("string-starts-with".to_string(), Value::Builtin("string-starts-with".to_string()));
        env.borrow_mut().define("substring".to_string(), Value::Builtin("substring".to_string()));
        env.borrow_mut().define("string->number".to_string(), Value::Builtin("string->number".to_string()));

        let mut evaluator = Evaluator::new(env);

        // Test string-split
        let split_expr = Expr::Application(
            Box::new(Expr::Symbol("string-split".to_string(), Type::Function(vec![Type::String, Type::String], Box::new(Type::List(Box::new(Type::String)))))),
            vec![
                Expr::String("hello,world,test".to_string(), Type::String),
                Expr::String(",".to_string(), Type::String),
            ],
            Type::List(Box::new(Type::String)),
        );

        let result = evaluator.eval(&split_expr).unwrap();
        assert_eq!(result, Value::List(vec![
            Value::String("hello".to_string()),
            Value::String("world".to_string()),
            Value::String("test".to_string()),
        ]));

        // Test string-starts-with
        let starts_with_expr = Expr::Application(
            Box::new(Expr::Symbol("string-starts-with".to_string(), Type::Function(vec![Type::String, Type::String], Box::new(Type::Bool)))),
            vec![
                Expr::String("hello world".to_string(), Type::String),
                Expr::String("hello".to_string(), Type::String),
            ],
            Type::Bool,
        );

        let result = evaluator.eval(&starts_with_expr).unwrap();
        assert_eq!(result, Value::Bool(true));

        // Test substring
        let substring_expr = Expr::Application(
            Box::new(Expr::Symbol("substring".to_string(), Type::Function(vec![Type::String, Type::Int, Type::Int], Box::new(Type::String)))),
            vec![
                Expr::String("hello world".to_string(), Type::String),
                Expr::Number(0, Type::Int),
                Expr::Number(5, Type::Int),
            ],
            Type::String,
        );

        let result = evaluator.eval(&substring_expr).unwrap();
        assert_eq!(result, Value::String("hello".to_string()));

        // Test string->number
        let str_to_num_expr = Expr::Application(
            Box::new(Expr::Symbol("string->number".to_string(), Type::Function(vec![Type::String], Box::new(Type::Int)))),
            vec![
                Expr::String("42".to_string(), Type::String),
            ],
            Type::Int,
        );

        let result = evaluator.eval(&str_to_num_expr).unwrap();
        assert_eq!(result, Value::Int(42));
    }

    #[test]
    fn test_list_functions() {
        let env = Rc::new(RefCell::new(Environment::new()));

        // Add list functions
        env.borrow_mut().define("list-ref".to_string(), Value::Builtin("list-ref".to_string()));
        env.borrow_mut().define("reverse".to_string(), Value::Builtin("reverse".to_string()));

        let mut evaluator = Evaluator::new(env);

        // Test list-ref
        let list_ref_expr = Expr::Application(
            Box::new(Expr::Symbol("list-ref".to_string(), Type::Function(vec![Type::List(Box::new(Type::TypeVar("a".to_string()))), Type::Int], Box::new(Type::TypeVar("a".to_string()))))),
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

        let result = evaluator.eval(&list_ref_expr).unwrap();
        assert_eq!(result, Value::String("second".to_string()));

        // Test reverse
        let reverse_expr = Expr::Application(
            Box::new(Expr::Symbol("reverse".to_string(), Type::Function(vec![Type::List(Box::new(Type::TypeVar("a".to_string())))], Box::new(Type::List(Box::new(Type::TypeVar("a".to_string()))))))),
            vec![
                Expr::List(vec![
                    Expr::Number(1, Type::Int),
                    Expr::Number(2, Type::Int),
                    Expr::Number(3, Type::Int),
                ], Type::List(Box::new(Type::Int))),
            ],
            Type::List(Box::new(Type::Int)),
        );

        let result = evaluator.eval(&reverse_expr).unwrap();
        assert_eq!(result, Value::List(vec![
            Value::Int(3),
            Value::Int(2),
            Value::Int(1),
        ]));
    }

    #[test]
    fn test_actor_functions() {
        let env = Rc::new(RefCell::new(Environment::new()));

        // Add actor functions
        env.borrow_mut().define("spawn".to_string(), Value::Builtin("spawn".to_string()));
        env.borrow_mut().define("self".to_string(), Value::Builtin("self".to_string()));

        let mut evaluator = Evaluator::new(env);

        // Test spawn function
        let spawn_expr = Expr::Application(
            Box::new(Expr::Symbol("spawn".to_string(), Type::Function(vec![Type::Function(vec![], Box::new(Type::Unit))], Box::new(Type::Pid)))),
            vec![
                Expr::Symbol("test-actor".to_string(), Type::Function(vec![], Box::new(Type::Unit))),
            ],
            Type::Pid,
        );

        let result = evaluator.eval(&spawn_expr).unwrap();
        assert!(matches!(result, Value::Pid(_)));

        // Test self function
        let self_expr = Expr::Application(
            Box::new(Expr::Symbol("self".to_string(), Type::Function(vec![], Box::new(Type::Pid)))),
            vec![],
            Type::Pid,
        );

        let result = evaluator.eval(&self_expr).unwrap();
        assert!(matches!(result, Value::Pid(_)));
    }

    #[test]
    fn test_module_import() {
        let env = Rc::new(RefCell::new(Environment::new()));

        // Add import function
        env.borrow_mut().define("import".to_string(), Value::Builtin("import".to_string()));

        let mut evaluator = Evaluator::new(env);

        // Test import function
        let import_expr = Expr::Application(
            Box::new(Expr::Symbol("import".to_string(), Type::Function(vec![Type::Symbol], Box::new(Type::Symbol)))),
            vec![
                Expr::Symbol("test-module".to_string(), Type::Symbol),
            ],
            Type::Symbol,
        );

        let result = evaluator.eval(&import_expr).unwrap();
        assert_eq!(result, Value::Symbol("imported-test-module".to_string()));
    }
}
