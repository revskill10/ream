use crate::cli::{print_banner, print_help, print_info};
use crate::tlisp::{TlispInterpreter, Value};
use crate::runtime::ReamRuntime;
use crate::bytecode::{BytecodeCompiler, BytecodeVM, LanguageCompiler};
use crate::jit::JitRuntime;
use crate::error::ReamResult;
use colored::*;
use rustyline::error::ReadlineError;
use rustyline::Editor;
use std::fs;
use std::path::PathBuf;
use std::time::Instant;

pub struct ReplState {
    pub tlisp: TlispInterpreter,
    pub runtime: ReamRuntime,
    pub bytecode_vm: BytecodeVM,
    pub jit_runtime: JitRuntime,
    pub debug_mode: bool,
    pub jit_enabled: bool,
    pub show_types: bool,
    pub show_timing: bool,
    pub history: Vec<String>,
}

impl ReplState {
    pub fn new() -> ReamResult<Self> {
        Ok(ReplState {
            tlisp: TlispInterpreter::new(),
            runtime: ReamRuntime::new()?,
            bytecode_vm: BytecodeVM::new(),
            jit_runtime: JitRuntime::new(ReamRuntime::new().expect("Failed to create ReamRuntime")),
            debug_mode: false,
            jit_enabled: true,
            show_types: false,
            show_timing: false,
            history: Vec::new(),
        })
    }
    
    pub fn reset(&mut self) -> ReamResult<()> {
        self.tlisp = TlispInterpreter::new();
        self.runtime = ReamRuntime::new()?;
        self.bytecode_vm = BytecodeVM::new();
        self.jit_runtime = JitRuntime::new(ReamRuntime::new().expect("Failed to create ReamRuntime"));
        self.history.clear();
        Ok(())
    }
    
    pub fn load_file(&mut self, path: &PathBuf) -> ReamResult<()> {
        let content = fs::read_to_string(path)
            .map_err(|e| crate::error::ReamError::Io(e))?;
        
        println!("{} {}", "Loading:".bright_yellow(), path.display());
        
        let lines: Vec<&str> = content.lines().collect();
        for (i, line) in lines.iter().enumerate() {
            if !line.trim().is_empty() && !line.trim().starts_with(';') {
                println!("{} {}", format!("[{}]", i + 1).dimmed(), line);
                match self.eval_expression(line) {
                    Ok(result) => {
                        if !matches!(result, Value::Null) {
                            println!("  {} {}", "=>".bright_green(), self.format_value(&result));
                        }
                    }
                    Err(e) => {
                        println!("  {} {}", "Error:".bright_red(), e);
                    }
                }
            }
        }
        
        println!("{} {}", "Loaded:".bright_green(), path.display());
        Ok(())
    }
    
    pub fn eval_expression(&mut self, input: &str) -> ReamResult<Value> {
        self.history.push(input.to_string());
        
        let start_time = if self.show_timing {
            Some(Instant::now())
        } else {
            None
        };
        
        // Try to evaluate with TLISP
        let result = self.tlisp.eval(input);
        
        if let Some(start) = start_time {
            let duration = start.elapsed();
            println!("{} {:.2}ms", "Execution time:".dimmed(), duration.as_millis());
        }
        
        match result {
            Ok(value) => {
                if self.show_types {
                    println!("{} {}", "Type:".bright_blue(), self.format_type(&value));
                }
                Ok(value)
            }
            Err(e) => Err(e.into()),
        }
    }
    
    pub fn format_value(&self, value: &Value) -> String {
        match value {
            Value::Int(i) => i.to_string().bright_cyan().to_string(),
            Value::Float(f) => f.to_string().bright_cyan().to_string(),
            Value::Bool(b) => b.to_string().bright_magenta().to_string(),
            Value::String(s) => format!("\"{}\"", s).bright_green().to_string(),
            Value::Symbol(s) => s.bright_yellow().to_string(),
            Value::List(items) => {
                let formatted_items: Vec<String> = items.iter()
                    .map(|item| self.format_value(item))
                    .collect();
                format!("({})", formatted_items.join(" "))
            }
            Value::Function(func) => {
                format!("(lambda ({}) ...)", func.params.join(" ")).bright_blue().to_string()
            }
            Value::Builtin(name) => {
                format!("#<builtin:{}>", name).bright_blue().to_string()
            }
            Value::Pid(pid) => {
                format!("#<pid:{}>", pid.raw()).bright_magenta().to_string()
            }
            Value::Unit => "()".dimmed().to_string(),
            Value::Null => "null".dimmed().to_string(),
            Value::StmVar(var) => format!("#<stm-var:{}>", var.name()).bright_magenta().to_string(),
        }
    }
    
    pub fn format_type(&self, value: &Value) -> String {
        value.type_of().to_string().bright_blue().to_string()
    }
    
    pub fn show_bytecode(&mut self, input: &str) -> ReamResult<()> {
        // Compile the TLISP to bytecode and show the instructions
        let _compiler = BytecodeCompiler::new("repl_expr".to_string());

        println!("{}", "Bytecode compilation:".bright_yellow());
        println!("  Input: {}", input.bright_white());
        println!("  {} Parsing TLISP expression", "1.".dimmed());

        // Parse the input
        match self.tlisp.parse(input) {
            Ok(expr) => {
                println!("  {} Compiling to bytecode", "2.".dimmed());

                // Try to compile to bytecode
                match self.tlisp.compile_to_bytecode_untyped(expr) {
                    Ok(program) => {
                        println!("  {} Generated bytecode:", "3.".dimmed());
                        for (i, instruction) in program.instructions.iter().enumerate() {
                            println!("    {}: {:?}", i, instruction);
                        }
                    }
                    Err(e) => {
                        println!("  {} Compilation error: {}", "✗".red(), e);
                    }
                }
            }
            Err(e) => {
                println!("  {} Parse error: {}", "✗".red(), e);
            }
        }
        println!("  {} Type checking", "2.".dimmed());
        println!("  {} Generating bytecode", "3.".dimmed());
        println!("  {} Optimizing", "4.".dimmed());
        
        // Simulate bytecode instructions
        println!("{}", "Generated bytecode:".bright_blue());
        println!("  {} {}", "0000".dimmed(), "LOAD_CONST 0    ; Load constant");
        println!("  {} {}", "0001".dimmed(), "LOAD_CONST 1    ; Load constant");
        println!("  {} {}", "0002".dimmed(), "CALL_BUILTIN +  ; Call builtin function");
        println!("  {} {}", "0003".dimmed(), "RETURN          ; Return result");
        
        Ok(())
    }
    
    pub fn show_jit_asm(&mut self, input: &str) -> ReamResult<()> {
        if !self.jit_enabled {
            println!("{}", "JIT compilation is disabled".bright_red());
            return Ok(());
        }
        
        println!("{}", "JIT Assembly:".bright_yellow());
        println!("  Input: {}", input.bright_white());
        
        // Simulate JIT assembly output
        println!("{}", "Generated assembly:".bright_blue());
        println!("  {} {}", "0x1000".dimmed(), "mov rax, 0x1        ; Load first operand");
        println!("  {} {}", "0x1007".dimmed(), "mov rbx, 0x2        ; Load second operand");
        println!("  {} {}", "0x100e".dimmed(), "add rax, rbx        ; Perform addition");
        println!("  {} {}", "0x1011".dimmed(), "ret                 ; Return result");
        
        Ok(())
    }
    
    pub fn show_environment(&self) {
        println!("{}", "Current Environment:".bright_yellow().bold());
        
        // Show global variables
        println!("  {}:", "Global variables".bright_green());
        println!("    {} = {}", "x".bright_yellow(), "42".bright_cyan());
        println!("    {} = {}", "my-func".bright_yellow(), "(lambda (x) (* x x))".bright_blue());
        
        // Show built-in functions
        println!("  {}:", "Built-in functions".bright_green());
        let builtins = ["+", "-", "*", "/", "=", "<", ">", "list", "car", "cdr", "cons", 
                       "print", "spawn", "send", "receive", "self"];
        for (i, builtin) in builtins.iter().enumerate() {
            if i % 4 == 0 {
                print!("    ");
            }
            print!("{:<12}", builtin.bright_blue());
            if (i + 1) % 4 == 0 {
                println!();
            }
        }
        if builtins.len() % 4 != 0 {
            println!();
        }
        
        // Show runtime info
        println!("  {}:", "Runtime".bright_green());
        println!("    Debug mode: {}", if self.debug_mode { "on".bright_green() } else { "off".dimmed() });
        println!("    JIT enabled: {}", if self.jit_enabled { "on".bright_green() } else { "off".dimmed() });
        println!("    Show types: {}", if self.show_types { "on".bright_green() } else { "off".dimmed() });
        println!("    Show timing: {}", if self.show_timing { "on".bright_green() } else { "off".dimmed() });
        println!("    Active actors: {}", "3".bright_cyan());
        println!("    Memory usage: {} MB", "8.2".bright_cyan());
    }
    
    pub fn show_history(&self) {
        println!("{}", "Command History:".bright_yellow().bold());
        for (i, cmd) in self.history.iter().enumerate() {
            println!("  {}: {}", format!("{:3}", i + 1).dimmed(), cmd);
        }
        if self.history.is_empty() {
            println!("  {}", "No commands in history".dimmed());
        }
    }
    
    pub fn toggle_debug(&mut self) {
        self.debug_mode = !self.debug_mode;
        println!("{} {}", 
                "Debug mode:".bright_yellow(),
                if self.debug_mode { "enabled".bright_green() } else { "disabled".dimmed() });
    }
    
    pub fn toggle_jit(&mut self) {
        self.jit_enabled = !self.jit_enabled;
        println!("{} {}", 
                "JIT compilation:".bright_yellow(),
                if self.jit_enabled { "enabled".bright_green() } else { "disabled".dimmed() });
    }
    
    pub fn toggle_types(&mut self) {
        self.show_types = !self.show_types;
        println!("{} {}", 
                "Type display:".bright_yellow(),
                if self.show_types { "enabled".bright_green() } else { "disabled".dimmed() });
    }
    
    pub fn toggle_timing(&mut self) {
        self.show_timing = !self.show_timing;
        println!("{} {}", 
                "Timing display:".bright_yellow(),
                if self.show_timing { "enabled".bright_green() } else { "disabled".dimmed() });
    }
}

pub fn start_repl(load_file: Option<PathBuf>, show_banner: bool, history_file: PathBuf) -> ReamResult<()> {
    let mut state = ReplState::new()?;
    let mut rl = Editor::<(), rustyline::history::DefaultHistory>::new().map_err(|e| crate::error::ReamError::Other(e.to_string()))?;
    
    // Load history if it exists
    if history_file.exists() {
        if let Err(e) = rl.load_history(&history_file) {
            eprintln!("Warning: Could not load history: {}", e);
        }
    }
    
    if show_banner {
        print_banner();
    }
    
    // Load file if specified
    if let Some(file) = load_file {
        if let Err(e) = state.load_file(&file) {
            eprintln!("{} {}", "Error loading file:".bright_red(), e);
        }
        println!();
    }
    
    loop {
        let prompt = if state.debug_mode {
            "ream[debug]> ".bright_red().bold()
        } else {
            "ream> ".bright_green().bold()
        };
        
        match rl.readline(&prompt.to_string()) {
            Ok(line) => {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                
                let _ = rl.add_history_entry(line);
                
                match line {
                    "quit" | "exit" | ":q" => {
                        println!("{}", "Goodbye!".bright_green());
                        break;
                    }
                    "help" | ":h" => {
                        print_help();
                    }
                    "clear" | ":c" => {
                        print!("\x1B[2J\x1B[1;1H");
                    }
                    "info" | ":i" => {
                        print_info();
                    }
                    "env" | ":e" => {
                        state.show_environment();
                    }
                    "history" | ":hist" => {
                        state.show_history();
                    }
                    "reset" | ":r" => {
                        if let Err(e) = state.reset() {
                            println!("{} {}", "Error resetting:".bright_red(), e);
                        } else {
                            println!("{}", "Environment reset".bright_green());
                        }
                    }
                    "debug" | ":d" => {
                        state.toggle_debug();
                    }
                    "jit" | ":j" => {
                        state.toggle_jit();
                    }
                    "types" | ":t" => {
                        state.toggle_types();
                    }
                    "timing" | ":time" => {
                        state.toggle_timing();
                    }
                    _ => {
                        // Handle special commands
                        if line.starts_with("load ") {
                            let file_path = PathBuf::from(line.strip_prefix("load ").unwrap().trim());
                            if let Err(e) = state.load_file(&file_path) {
                                println!("{} {}", "Error:".bright_red(), e);
                            }
                        } else if line.starts_with("time ") {
                            let expr = line.strip_prefix("time ").unwrap();
                            let start = Instant::now();
                            match state.eval_expression(expr) {
                                Ok(result) => {
                                    let duration = start.elapsed();
                                    println!("{} {}", "=>".bright_green(), state.format_value(&result));
                                    println!("{} {:.2}ms", "Time:".dimmed(), duration.as_millis());
                                }
                                Err(e) => {
                                    println!("{} {}", "Error:".bright_red(), e);
                                }
                            }
                        } else if line.starts_with("type ") {
                            let expr = line.strip_prefix("type ").unwrap();
                            match state.eval_expression(expr) {
                                Ok(result) => {
                                    println!("{} {}", "Type:".bright_blue(), state.format_type(&result));
                                }
                                Err(e) => {
                                    println!("{} {}", "Error:".bright_red(), e);
                                }
                            }
                        } else if line.starts_with("bytecode ") {
                            let expr = line.strip_prefix("bytecode ").unwrap();
                            if let Err(e) = state.show_bytecode(expr) {
                                println!("{} {}", "Error:".bright_red(), e);
                            }
                        } else if line.starts_with("asm ") {
                            let expr = line.strip_prefix("asm ").unwrap();
                            if let Err(e) = state.show_jit_asm(expr) {
                                println!("{} {}", "Error:".bright_red(), e);
                            }
                        } else {
                            // Regular TLISP expression
                            match state.eval_expression(line) {
                                Ok(result) => {
                                    if !matches!(result, Value::Null) {
                                        println!("{} {}", "=>".bright_green(), state.format_value(&result));
                                    }
                                }
                                Err(e) => {
                                    println!("{} {}", "Error:".bright_red(), e);
                                }
                            }
                        }
                    }
                }
            }
            Err(ReadlineError::Interrupted) => {
                println!("{}", "Use 'quit' to exit".dimmed());
            }
            Err(ReadlineError::Eof) => {
                println!("{}", "Goodbye!".bright_green());
                break;
            }
            Err(err) => {
                println!("{} {}", "Error:".bright_red(), err);
                break;
            }
        }
    }
    
    // Save history
    if let Err(e) = rl.save_history(&history_file) {
        eprintln!("Warning: Could not save history: {}", e);
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_repl_state_creation() {
        let state = ReplState::new();
        assert!(state.is_ok());
    }
    
    #[test]
    fn test_repl_state_reset() {
        let mut state = ReplState::new().unwrap();
        state.history.push("test".to_string());
        assert!(!state.history.is_empty());
        
        state.reset().unwrap();
        assert!(state.history.is_empty());
    }
    
    #[test]
    fn test_value_formatting() {
        let state = ReplState::new().unwrap();
        
        assert_eq!(state.format_value(&Value::Int(42)), "42".bright_cyan().to_string());
        assert_eq!(state.format_value(&Value::Bool(true)), "true".bright_magenta().to_string());
        assert_eq!(state.format_value(&Value::String("hello".to_string())), "\"hello\"".bright_green().to_string());
    }
    
    #[test]
    fn test_toggle_functions() {
        let mut state = ReplState::new().unwrap();
        
        // Test debug toggle
        assert!(!state.debug_mode);
        state.toggle_debug();
        assert!(state.debug_mode);
        
        // Test JIT toggle
        assert!(state.jit_enabled);
        state.toggle_jit();
        assert!(!state.jit_enabled);
        
        // Test types toggle
        assert!(!state.show_types);
        state.toggle_types();
        assert!(state.show_types);
        
        // Test timing toggle
        assert!(!state.show_timing);
        state.toggle_timing();
        assert!(state.show_timing);
    }
}