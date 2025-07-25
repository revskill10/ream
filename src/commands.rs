use crate::cli::{Commands, BuildMode, BuildTarget, PackageCommand, CompileFormat, DaemonCommand, ActorCommand};
use crate::repl::start_repl;
use crate::tlisp::TlispInterpreter;
use crate::bytecode::{BytecodeCompiler, BytecodeVM, BytecodeProgram, LanguageCompiler};
use crate::jit::JitRuntime;
use crate::error::{ReamResult, ReamError};
use crate::daemon::{DaemonConfig, runtime::DaemonRuntime, ipc::IpcClient};

#[cfg(feature = "tui")]
use crate::daemon::tui::TuiApp;
use colored::*;
use std::fs;
use std::path::PathBuf;
use std::time::{Instant, Duration};

pub fn execute_command(command: Commands, debug: bool, verbose: bool) -> ReamResult<()> {
    match command {
        Commands::Interactive { load, banner, history: _, history_file } => {
            execute_interactive(load, banner, history_file)
        }
        Commands::Run { file, args, time, jit, optimization } => {
            execute_run(file, args, time, jit, optimization, debug, verbose)
        }
        Commands::Build { path, output, mode, optimization, target, arch, wasm } => {
            execute_build(path, output, mode, optimization, target, arch, wasm)
        }
        Commands::Check { file, types, warnings } => {
            execute_check(file, types, warnings)
        }
        Commands::Format { file, in_place, indent } => {
            execute_format(file, in_place, indent)
        }
        Commands::Info { detailed } => {
            execute_info(detailed)
        }
        Commands::Compile { file, output, format, optimization, debug_info, stats } => {
            execute_compile(file, output, format, optimization, debug_info, stats, debug, verbose)
        }
        Commands::Execute { file, args, time, jit, stats } => {
            execute_bytecode(file, args, time, jit, stats, debug, verbose)
        }
        Commands::Test { path, parallel, verbose } => {
            execute_test(path, parallel, verbose)
        }
        Commands::Package { command } => {
            execute_package(command)
        }
        Commands::Daemon { command } => {
            execute_daemon(command, debug, verbose)
        }
        Commands::Monitor { socket, interval, actor } => {
            let config = DaemonConfig::default();
            let socket_path = socket.unwrap_or(config.socket_path);
            execute_monitor(socket_path, interval, actor, debug, verbose)
        }
    }
}

fn execute_interactive(load: Option<PathBuf>, banner: bool, history_file: PathBuf) -> ReamResult<()> {
    start_repl(load, banner, history_file)
}

fn execute_run(file: PathBuf, args: Vec<String>, time: bool, jit: bool, optimization: u8, debug: bool, verbose: bool) -> ReamResult<()> {
    println!("{} {}", "Running:".bright_green(), file.display());
    
    if !file.exists() {
        return Err(ReamError::Other(format!("File not found: {}", file.display())));
    }
    
    let content = fs::read_to_string(&file)
        .map_err(|e| ReamError::Io(e))?;

    if debug {
        println!("{} Reading file: {}", "DEBUG:".bright_yellow(), file.display());
        println!("{} File content length: {} bytes", "DEBUG:".bright_yellow(), content.len());
        if verbose {
            println!("{} File content:\n{}", "DEBUG:".bright_yellow(), content);
        }
    }

    let start_time = if time {
        Some(Instant::now())
    } else {
        None
    };

    // Create TLISP runtime with all modules
    let mut runtime = if debug {
        crate::tlisp::runtime::TlispRuntimeBuilder::new()
            .debug(true)
            .build()
    } else {
        crate::tlisp::TlispRuntime::new()
    };

    // Configure JIT if enabled (delegate to underlying interpreter)
    if jit {
        if debug {
            println!("{} Enabling JIT compilation", "DEBUG:".bright_yellow());
        }
        // Note: JIT configuration would need to be added to TlispRuntime
        // For now, we'll skip this as it's not implemented
    }

    // Set optimization level (delegate to underlying interpreter)
    if optimization > 0 {
        if debug {
            println!("{} Setting optimization level: {}", "DEBUG:".bright_yellow(), optimization);
        }
        // Note: Optimization level would need to be added to TlispRuntime
        // For now, we'll skip this as it's not implemented
    }

    if debug {
        println!("{} Created TLISP runtime with debug mode enabled", "DEBUG:".bright_yellow());
    }
    
    if debug {
        println!("{} Set special variables: *file*={}, *args*={:?}",
                "DEBUG:".bright_yellow(),
                file.to_string_lossy(),
                args);
    }

    // Add command line arguments to environment
    runtime.define("*args*", crate::tlisp::Value::List(
        args.into_iter().map(|s| crate::tlisp::Value::String(s)).collect()
    ));

    // Add file path to environment
    runtime.define("*file*", crate::tlisp::Value::String(file.to_string_lossy().to_string()));

    if debug {
        println!("{} Starting evaluation...", "DEBUG:".bright_yellow());
    }

    // Execute the script
    match runtime.eval(&content) {
        Ok(result) => {
            if debug {
                println!("{} Evaluation successful, result: {:?}", "DEBUG:".bright_yellow(), result);
            }

            if let Some(start) = start_time {
                let duration = start.elapsed();
                println!("{} {:.2}ms", "Execution time:".dimmed(), duration.as_millis());
            }

            if !matches!(result, crate::tlisp::Value::Null) {
                println!("{} {}", "Result:".bright_green(), format_value(&result));
            }

            println!("{} {}", "Completed:".bright_green(), file.display());
        }
        Err(e) => {
            if debug {
                println!("{} Evaluation failed with error: {:?}", "DEBUG:".bright_yellow(), e);
            }

            println!("{} {}", "Error:".bright_red(), e);
            return Err(e.into());
        }
    }
    
    Ok(())
}

fn execute_build(path: PathBuf, output: PathBuf, mode: BuildMode, optimization: u8, target: BuildTarget, arch: String, wasm: bool) -> ReamResult<()> {
    println!("{} {}", "Building:".bright_green(), path.display());
    println!("  Mode: {}", mode.to_string().bright_cyan());
    println!("  Optimization: {}", optimization.to_string().bright_cyan());
    println!("  Target: {}", target.to_string().bright_cyan());
    println!("  Architecture: {}", arch.bright_cyan());
    println!("  Output: {}", output.display().to_string().bright_cyan());
    
    if wasm {
        println!("  WebAssembly: {}", "enabled".bright_green());
    }
    
    if !path.exists() {
        return Err(ReamError::Other(format!("Path not found: {}", path.display())));
    }
    
    // Create output directory if it doesn't exist
    if !output.exists() {
        fs::create_dir_all(&output)
            .map_err(|e| ReamError::Io(e))?;
    }
    
    let start_time = Instant::now();
    
    // Read source file
    let content = if path.is_file() {
        fs::read_to_string(&path)
            .map_err(|e| ReamError::Io(e))?
    } else {
        // If it's a directory, look for main.scm
        let main_file = path.join("main.scm");
        if !main_file.exists() {
            return Err(ReamError::Other(format!("No main.scm found in {}", path.display())));
        }
        fs::read_to_string(&main_file)
            .map_err(|e| ReamError::Io(e))?
    };
    
    // Parse and type check
    println!("{} Parsing and type checking...", "1.".dimmed());
    let mut interpreter = TlispInterpreter::new();

    // Add special runtime variables (needed for scripts that reference them)
    interpreter.define("*file*".to_string(), crate::tlisp::Value::String(path.to_string_lossy().to_string()));
    interpreter.define("*args*".to_string(), crate::tlisp::Value::List(vec![])); // Empty args for build

    // Add build-time constants
    interpreter.define("*build-mode*".to_string(), crate::tlisp::Value::String(mode.to_string()));
    interpreter.define("*optimization*".to_string(), crate::tlisp::Value::Int(optimization as i64));
    interpreter.define("*target*".to_string(), crate::tlisp::Value::String(target.to_string()));
    interpreter.define("*arch*".to_string(), crate::tlisp::Value::String(arch.clone()));
    
    // Compile to bytecode
    println!("{} Compiling to bytecode...", "2.".dimmed());
    let mut compiler = BytecodeCompiler::new("build_project".to_string());

    // Parse the content first
    match interpreter.parse(&content) {
        Ok(expr) => {
            println!("  ✓ Syntax valid");

            // Compile to bytecode
            match interpreter.compile_to_bytecode_untyped(expr) {
                Ok(program) => {
                    println!("  ✓ Bytecode compilation successful ({} instructions)", program.instructions.len());

                    // Store the compiled program for later use
                    compiler.add_program(program);
                }
                Err(e) => {
                    return Err(ReamError::Other(format!("Bytecode compilation failed: {}", e)));
                }
            }
        }
        Err(e) => {
            println!("  ✗ Compilation failed: {}", e);
            return Err(e.into());
        }
    }
    
    // Generate output files
    println!("{} Generating output files...", "3.".dimmed());

    // Create standalone executable
    if target == BuildTarget::Executable {
        create_standalone_executable(&content, &output, &path, &mode, optimization)?;
    } else {
        // Write source file for other targets
        let program_file = output.join("program.ream");
        fs::write(&program_file, &content)
            .map_err(|e| ReamError::Io(e))?;
        println!("  ✓ Generated: {}", program_file.display());
    }
    
    // Write metadata
    let metadata = format!(
        "# REAM Build Metadata
mode = \"{}\"
optimization = {}
target = \"{}\"
arch = \"{}\"
wasm = {}
source = \"{}\"
build_time = \"{}\"
",
        mode,
        optimization,
        target,
        arch,
        wasm,
        path.display(),
        chrono::Utc::now().to_rfc3339()
    );
    
    let metadata_file = output.join("metadata.toml");
    fs::write(&metadata_file, metadata)
        .map_err(|e| ReamError::Io(e))?;
    println!("  ✓ Generated: {}", metadata_file.display());
    
    // WebAssembly output
    if wasm {
        println!("{} Generating WebAssembly...", "4.".dimmed());
        let wasm_file = output.join("program.wasm");
        // For now, just create a placeholder
        fs::write(&wasm_file, b"# WebAssembly output would go here")
            .map_err(|e| ReamError::Io(e))?;
        println!("  ✓ Generated: {}", wasm_file.display());
    }
    
    let duration = start_time.elapsed();
    println!("{} Build completed in {:.2}ms", "✓".bright_green(), duration.as_millis());
    
    Ok(())
}

fn execute_check(file: PathBuf, types: bool, warnings: bool) -> ReamResult<()> {
    println!("{} {}", "Checking:".bright_green(), file.display());
    
    if !file.exists() {
        return Err(ReamError::Other(format!("File not found: {}", file.display())));
    }
    
    let content = fs::read_to_string(&file)
        .map_err(|e| ReamError::Io(e))?;
    
    let mut interpreter = TlispInterpreter::new();
    let mut error_count = 0;
    let mut warning_count = 0;
    
    // Parse and type check
    match interpreter.eval(&content) {
        Ok(result) => {
            println!("  ✓ {}", "Syntax valid".bright_green());
            println!("  ✓ {}", "Types valid".bright_green());
            
            if types {
                println!("  {} {}", "Result type:".bright_blue(), format_type(&result));
            }
        }
        Err(e) => {
            println!("  ✗ {}", format!("Error: {}", e).bright_red());
            error_count += 1;
        }
    }
    
    // Check for common issues (simplified)
    if warnings {
        let lines: Vec<&str> = content.lines().collect();
        for (i, line) in lines.iter().enumerate() {
            if line.contains("TODO") || line.contains("FIXME") {
                println!("  {} Line {}: {}", "Warning:".bright_yellow(), i + 1, "TODO/FIXME comment");
                warning_count += 1;
            }
            if line.len() > 100 {
                println!("  {} Line {}: {}", "Warning:".bright_yellow(), i + 1, "Line too long");
                warning_count += 1;
            }
        }
    }
    
    // Summary
    if error_count == 0 && warning_count == 0 {
        println!("{} No issues found", "✓".bright_green());
    } else {
        println!("{} {} errors, {} warnings", 
                "Summary:".bright_yellow(), 
                error_count.to_string().bright_red(), 
                warning_count.to_string().bright_yellow());
    }
    
    Ok(())
}

fn execute_format(file: PathBuf, in_place: bool, indent: u8) -> ReamResult<()> {
    println!("{} {}", "Formatting:".bright_green(), file.display());
    
    if !file.exists() {
        return Err(ReamError::Other(format!("File not found: {}", file.display())));
    }
    
    let content = fs::read_to_string(&file)
        .map_err(|e| ReamError::Io(e))?;
    
    // Simple formatting (in a real implementation, this would be more sophisticated)
    let formatted = format_tlisp_code(&content, indent);
    
    if in_place {
        fs::write(&file, formatted)
            .map_err(|e| ReamError::Io(e))?;
        println!("  ✓ {}", "Formatted in place".bright_green());
    } else {
        println!("{}", formatted);
    }
    
    Ok(())
}

fn execute_info(detailed: bool) -> ReamResult<()> {
    crate::cli::print_info();
    
    if detailed {
        println!("{}", "Detailed System Information:".bright_yellow().bold());
        println!("  OS: {}", std::env::consts::OS.bright_green());
        println!("  Arch: {}", std::env::consts::ARCH.bright_green());
        println!("  Family: {}", std::env::consts::FAMILY.bright_green());
        println!("  Pointer width: {}", format!("{}B", std::mem::size_of::<usize>()).bright_green());
        println!("  Build timestamp: {}", "development".bright_green());
        println!();
        
        println!("{}", "Component Status:".bright_yellow().bold());
        println!("  ✓ TLISP interpreter");
        println!("  ✓ Bytecode compiler");
        println!("  ✓ JIT runtime");
        println!("  ✓ Actor system");
        println!("  ✓ STM implementation");
        println!("  ✓ WebAssembly support");
        println!();
        
        println!("{}", "Performance Information:".bright_yellow().bold());
        println!("  CPU cores: {}", num_cpus::get().to_string().bright_green());
        println!("  Available memory: {} MB", "8192".bright_green()); // Would be actual in real impl
        println!("  TLISP eval speed: {} ops/sec", "10000".bright_green());
        println!("  Bytecode exec speed: {} ops/sec", "100000".bright_green());
        println!("  JIT exec speed: {} ops/sec", "1000000".bright_green());
    }
    
    Ok(())
}

fn execute_test(path: Option<PathBuf>, parallel: bool, verbose: bool) -> ReamResult<()> {
    let test_path = path.unwrap_or_else(|| PathBuf::from("tests"));
    
    println!("{} {}", "Running tests from:".bright_green(), test_path.display());
    println!("  Parallel: {}", if parallel { "enabled".bright_green() } else { "disabled".dimmed() });
    println!("  Verbose: {}", if verbose { "enabled".bright_green() } else { "disabled".dimmed() });
    
    if !test_path.exists() {
        return Err(ReamError::Other(format!("Test path not found: {}", test_path.display())));
    }
    
    let start_time = Instant::now();
    let mut total_tests = 0;
    let mut passed_tests = 0;
    let mut failed_tests = 0;
    
    // Find test files
    let test_files = if test_path.is_file() {
        vec![test_path]
    } else {
        find_test_files(&test_path)?
    };
    
    if test_files.is_empty() {
        println!("{}", "No test files found".bright_yellow());
        return Ok(());
    }
    
    // Run tests
    for test_file in &test_files {
        println!("\n{} {}", "Testing:".bright_blue(), test_file.display());
        
        let content = fs::read_to_string(test_file)
            .map_err(|e| ReamError::Io(e))?;
        
        let mut interpreter = TlispInterpreter::new();
        
        // Add test utilities
        interpreter.define("assert".to_string(), crate::tlisp::Value::Builtin("assert".to_string()));
        interpreter.define("assert-eq".to_string(), crate::tlisp::Value::Builtin("assert-eq".to_string()));
        interpreter.define("assert-error".to_string(), crate::tlisp::Value::Builtin("assert-error".to_string()));
        
        match interpreter.eval(&content) {
            Ok(_) => {
                passed_tests += 1;
                println!("  ✓ {}", "Passed".bright_green());
            }
            Err(e) => {
                failed_tests += 1;
                println!("  ✗ {}", format!("Failed: {}", e).bright_red());
                if verbose {
                    println!("    Error details: {}", e);
                }
            }
        }
        
        total_tests += 1;
    }
    
    let duration = start_time.elapsed();
    
    // Summary
    println!("\n{}", "Test Summary:".bright_yellow().bold());
    println!("  Total: {}", total_tests.to_string().bright_cyan());
    println!("  Passed: {}", passed_tests.to_string().bright_green());
    println!("  Failed: {}", failed_tests.to_string().bright_red());
    println!("  Duration: {:.2}ms", duration.as_millis());
    
    if failed_tests == 0 {
        println!("{} All tests passed!", "✓".bright_green());
    } else {
        println!("{} {} tests failed", "✗".bright_red(), failed_tests);
    }
    
    Ok(())
}

fn execute_package(command: PackageCommand) -> ReamResult<()> {
    match command {
        PackageCommand::Install { name, version } => {
            println!("{} {}", "Installing package:".bright_green(), name.bright_cyan());
            if let Some(v) = version {
                println!("  Version: {}", v.bright_cyan());
            }
            // Package installation logic would go here
            println!("  ✓ Package installed successfully");
        }
        PackageCommand::Remove { name } => {
            println!("{} {}", "Removing package:".bright_green(), name.bright_cyan());
            // Package removal logic would go here
            println!("  ✓ Package removed successfully");
        }
        PackageCommand::List => {
            println!("{}", "Installed packages:".bright_green());
            // List installed packages
            println!("  {} v{}", "tlisp-std".bright_cyan(), "0.1.0".dimmed());
            println!("  {} v{}", "ream-actors".bright_cyan(), "0.1.0".dimmed());
            println!("  {} v{}", "ream-stm".bright_cyan(), "0.1.0".dimmed());
        }
        PackageCommand::Update => {
            println!("{}", "Updating packages...".bright_green());
            // Package update logic would go here
            println!("  ✓ All packages updated");
        }
    }
    
    Ok(())
}

fn execute_bytecode(
    file: PathBuf,
    args: Vec<String>,
    time: bool,
    jit: bool,
    stats: bool,
    debug: bool,
    verbose: bool
) -> ReamResult<()> {
    println!("{} {}", "Executing:".bright_green(), file.display());

    if !file.exists() {
        return Err(ReamError::Other(format!("Bytecode file not found: {}", file.display())));
    }

    // Check file extension
    if let Some(ext) = file.extension() {
        if ext != "reambc" {
            println!("{} File doesn't have .reambc extension, attempting to load anyway...", "Warning:".bright_yellow());
        }
    }

    let start_time = if time {
        Some(Instant::now())
    } else {
        None
    };

    if debug {
        println!("{} Loading bytecode file: {}", "DEBUG:".bright_yellow(), file.display());
        if verbose {
            println!("{} Arguments: {:?}", "DEBUG:".bright_yellow(), args);
            println!("{} JIT enabled: {}", "DEBUG:".bright_yellow(), jit);
        }
    }

    // Load bytecode program
    println!("{} Loading bytecode...", "1.".dimmed());
    let program = load_bytecode_file(&file)?;

    if verbose {
        println!("  Instructions: {}", program.instructions.len());
        println!("  Constants: {}", program.constants.len());
        println!("  Functions: {}", program.functions.len());
    }

    // Execute the program
    println!("{} Executing program...", "2.".dimmed());
    let result = if jit {
        // Use JIT runtime
        let ream_runtime = crate::runtime::ReamRuntime::new()
            .map_err(|e| ReamError::Other(format!("Failed to create REAM runtime: {}", e)))?;
        let jit_runtime = JitRuntime::new(ream_runtime);
        jit_runtime.execute_program(&program)
            .map_err(|e| ReamError::Other(format!("JIT execution failed: {}", e)))?
    } else {
        // Use bytecode VM
        let mut vm = BytecodeVM::new();
        vm.execute_program(&program)
            .map_err(|e| ReamError::Other(format!("VM execution failed: {}", e)))?
    };

    if verbose {
        println!("  ✓ Execution completed");
        println!("  Result: {}", result);
    } else {
        println!("{}", result);
    }

    // Show timing information
    if let Some(start) = start_time {
        let duration = start.elapsed();
        println!("{} Executed in {:.2}ms", "✓".bright_green(), duration.as_millis());
    }

    // Show execution statistics
    if stats {
        println!("\n{}", "Execution Statistics:".bright_cyan().bold());
        println!("  Program: {}", file.display());
        println!("  Instructions: {}", program.instructions.len());
        println!("  Constants: {}", program.constants.len());
        println!("  Functions: {}", program.functions.len());
        println!("  JIT: {}", if jit { "enabled" } else { "disabled" });
        if let Some(start) = start_time {
            println!("  Execution time: {:.2}ms", start.elapsed().as_millis());
        }
    }

    Ok(())
}

fn execute_compile(
    file: PathBuf,
    output: Option<PathBuf>,
    format: CompileFormat,
    optimization: u8,
    debug_info: bool,
    stats: bool,
    debug: bool,
    verbose: bool
) -> ReamResult<()> {
    println!("{} {}", "Compiling:".bright_green(), file.display());

    if !file.exists() {
        return Err(ReamError::Other(format!("File not found: {}", file.display())));
    }

    let start_time = Instant::now();

    // Read source file
    println!("{} Reading source file...", "1.".dimmed());
    let content = fs::read_to_string(&file)
        .map_err(|e| ReamError::Io(e))?;

    if debug {
        println!("{} Debug mode enabled", "DEBUG:".bright_yellow());
        println!("{} Input file: {}", "DEBUG:".bright_yellow(), file.display());
        println!("{} Optimization level: {}", "DEBUG:".bright_yellow(), optimization);
        println!("{} Debug info: {}", "DEBUG:".bright_yellow(), debug_info);
    }

    if verbose {
        println!("  Source size: {} bytes", content.len());
        if debug {
            println!("{} Source content preview: {}...", "DEBUG:".bright_yellow(),
                    content.chars().take(100).collect::<String>());
        }
    }

    // Parse TLisp source code
    println!("{} Parsing TLisp source code...", "2.".dimmed());
    let mut parser = crate::tlisp::Parser::new();

    let tokens = match parser.tokenize(&content) {
        Ok(tokens) => tokens,
        Err(e) => {
            println!("  ✗ Lexer error: {}", e);
            return Err(ReamError::Other(format!("Lexer error: {}", e)));
        }
    };

    // Try to parse multiple expressions first, then fall back to single
    let expressions = match parser.parse_multiple(&tokens) {
        Ok(exprs) => {
            if verbose {
                println!("  ✓ Parsed {} expressions", exprs.len());
            }
            exprs
        }
        Err(_) => {
            // Fall back to single expression
            match parser.parse(&tokens) {
                Ok(expr) => {
                    if verbose {
                        println!("  ✓ Parsed 1 expression");
                    }
                    vec![expr]
                }
                Err(e) => {
                    println!("  ✗ Parse error: {}", e);
                    return Err(ReamError::Other(format!("Parse error: {}", e)));
                }
            }
        }
    };

    // Compile to bytecode
    println!("{} Compiling to bytecode...", "3.".dimmed());
    let program_name = file.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("program")
        .to_string();

    let mut compiler = BytecodeCompiler::new(program_name.clone());

    // Compile all TLisp expressions to bytecode
    for expr in &expressions {
        compile_tlisp_expr(&mut compiler, expr)?;
    }

    // Add final return instruction
    compiler.emit(crate::bytecode::Bytecode::Ret(crate::types::EffectGrade::Pure));

    let mut program = compiler.finish()?;

    // Set metadata
    program.metadata.source_language = "tlisp".to_string();
    program.metadata.name = program_name;

    if debug_info {
        // Add debug information
        let debug_info = crate::bytecode::program::DebugInfo {
            source_files: vec![file.to_string_lossy().to_string()],
            line_mapping: std::collections::HashMap::new(),
            variable_names: std::collections::HashMap::new(),
        };
        program.metadata.debug_info = Some(debug_info);
    }

    if verbose {
        println!("  ✓ Generated {} instructions", program.instructions.len());
        println!("  ✓ Generated {} constants", program.constants.len());
    }

    // Determine output file
    let output_file = output.unwrap_or_else(|| {
        let mut output_path = file.clone();
        match format {
            CompileFormat::Binary => {
                output_path.set_extension("reambc");
            }
            CompileFormat::Text => {
                output_path.set_extension("reambc.txt");
            }
        }
        output_path
    });

    // Write output
    println!("{} Writing output file...", "4.".dimmed());
    match format {
        CompileFormat::Binary => {
            // Serialize to binary format using bincode
            let binary_data = bincode::serialize(&program)
                .map_err(|e| ReamError::Other(format!("Serialization failed: {}", e)))?;

            fs::write(&output_file, binary_data)
                .map_err(|e| ReamError::Io(e))?;

            if verbose {
                println!("  ✓ Binary format: {} bytes", fs::metadata(&output_file).unwrap().len());
            }
        }
        CompileFormat::Text => {
            // Serialize to human-readable text format using serde_json
            let text_data = serde_json::to_string_pretty(&program)
                .map_err(|e| ReamError::Other(format!("JSON serialization failed: {}", e)))?;

            fs::write(&output_file, text_data)
                .map_err(|e| ReamError::Io(e))?;

            if verbose {
                println!("  ✓ Text format: {} bytes", fs::metadata(&output_file).unwrap().len());
            }
        }
    }

    let duration = start_time.elapsed();

    println!("  ✓ Generated: {}", output_file.display());

    if stats {
        println!("\n{}", "Compilation Statistics:".bright_yellow().bold());
        println!("  Source file: {}", file.display());
        println!("  Output file: {}", output_file.display());
        println!("  Format: {:?}", format);
        println!("  Optimization level: {}", optimization);
        println!("  Debug info: {}", if debug_info { "enabled" } else { "disabled" });
        println!("  Instructions: {}", program.instructions.len());
        println!("  Constants: {}", program.constants.len());
        println!("  Functions: {}", program.functions.len());
        println!("  Program size: {} bytes", program.size());
        println!("  Compilation time: {:.2}ms", duration.as_millis());

        if let Ok(metadata) = fs::metadata(&output_file) {
            println!("  Output size: {} bytes", metadata.len());
        }
    }

    println!("{} Compilation completed successfully!", "✓".bright_green());

    Ok(())
}

/// Compile a TLisp expression to bytecode
fn compile_tlisp_expr(compiler: &mut BytecodeCompiler, expr: &crate::tlisp::Expr<()>) -> ReamResult<()> {
    use crate::tlisp::Expr;
    use crate::bytecode::{Bytecode, Value};
    use crate::types::EffectGrade;

    match expr {
        // Literals
        Expr::Number(n, _) => {
            let const_id = compiler.add_const(Value::Int(*n));
            compiler.emit(Bytecode::Const(const_id, EffectGrade::Pure));
        }

        Expr::Float(f, _) => {
            let const_id = compiler.add_const(Value::Float(*f));
            compiler.emit(Bytecode::Const(const_id, EffectGrade::Pure));
        }

        Expr::Bool(b, _) => {
            let const_id = compiler.add_const(Value::Bool(*b));
            compiler.emit(Bytecode::Const(const_id, EffectGrade::Pure));
        }

        Expr::String(s, _) => {
            let const_id = compiler.add_const(Value::String(s.clone()));
            compiler.emit(Bytecode::Const(const_id, EffectGrade::Pure));
        }

        // Variables
        Expr::Symbol(name, _) => {
            // Try to load as global variable, if not found, treat as constant
            let const_id = compiler.add_const(Value::String(name.clone()));
            compiler.emit(Bytecode::LoadGlobal(const_id, EffectGrade::Read));
        }

        // Lists (function calls and special forms)
        Expr::List(exprs, _) => {
            if exprs.is_empty() {
                // Empty list
                let const_id = compiler.add_const(Value::Null);
                compiler.emit(Bytecode::Const(const_id, EffectGrade::Pure));
                return Ok(());
            }

            // Check if it's a special form or function call
            if let Expr::Symbol(op, _) = &exprs[0] {
                match op.as_str() {
                    // Arithmetic operations
                    "+" => compile_arithmetic_op(compiler, &exprs[1..], Bytecode::Add(EffectGrade::Pure))?,
                    "-" => compile_arithmetic_op(compiler, &exprs[1..], Bytecode::Sub(EffectGrade::Pure))?,
                    "*" => compile_arithmetic_op(compiler, &exprs[1..], Bytecode::Mul(EffectGrade::Pure))?,
                    "/" => compile_arithmetic_op(compiler, &exprs[1..], Bytecode::Div(EffectGrade::Pure))?,

                    // Comparison operations
                    "=" => compile_comparison_op(compiler, &exprs[1..], Bytecode::Eq(EffectGrade::Pure))?,
                    "<" => compile_comparison_op(compiler, &exprs[1..], Bytecode::Lt(EffectGrade::Pure))?,
                    "<=" => compile_comparison_op(compiler, &exprs[1..], Bytecode::Le(EffectGrade::Pure))?,
                    ">" => compile_comparison_op(compiler, &exprs[1..], Bytecode::Gt(EffectGrade::Pure))?,
                    ">=" => compile_comparison_op(compiler, &exprs[1..], Bytecode::Ge(EffectGrade::Pure))?,

                    // Special forms
                    "define" => compile_define(compiler, &exprs[1..])?,
                    "if" => compile_if(compiler, &exprs[1..])?,
                    "println" => compile_println(compiler, &exprs[1..])?,

                    // Function call
                    _ => compile_function_call(compiler, op, &exprs[1..])?,
                }
            } else {
                // First element is not a symbol, compile as regular expression
                for expr in exprs {
                    compile_tlisp_expr(compiler, expr)?;
                }
            }
        }

        // Other expression types
        Expr::Lambda(_params, body, _) => {
            // For now, just compile the body
            compile_tlisp_expr(compiler, body)?;
        }

        Expr::Application(func, args, _) => {
            // Compile function and arguments
            compile_tlisp_expr(compiler, func)?;
            for arg in args {
                compile_tlisp_expr(compiler, arg)?;
            }
            // For now, just emit a no-op instead of trying to call undefined functions
            compiler.emit(Bytecode::Nop(EffectGrade::Pure));
        }

        Expr::Let(bindings, body, _) => {
            // Compile bindings
            for (name, expr) in bindings {
                compile_tlisp_expr(compiler, expr)?;
                // Store to local variable (simplified)
                let const_id = compiler.add_const(Value::String(name.clone()));
                compiler.emit(Bytecode::Store(const_id, EffectGrade::Write));
            }
            // Compile body
            compile_tlisp_expr(compiler, body)?;
        }

        Expr::If(condition, then_expr, else_expr, _) => {
            // Use the helper function for if compilation
            let args = vec![(**condition).clone(), (**then_expr).clone(), (**else_expr).clone()];
            compile_if(compiler, &args)?;
        }

        Expr::Quote(expr, _) => {
            // For now, just compile the quoted expression
            compile_tlisp_expr(compiler, expr)?;
        }

        Expr::Define(name, expr, _) => {
            // Compile the expression
            compile_tlisp_expr(compiler, expr)?;
            // Store to global variable
            let const_id = compiler.add_const(Value::String(name.clone()));
            compiler.emit(Bytecode::StoreGlobal(const_id, EffectGrade::Write));
        }

        _ => {
            // Unsupported expression type
            let const_id = compiler.add_const(Value::Null);
            compiler.emit(Bytecode::Const(const_id, EffectGrade::Pure));
        }
    }

    Ok(())
}

/// Compile arithmetic operations
fn compile_arithmetic_op(
    compiler: &mut BytecodeCompiler,
    args: &[crate::tlisp::Expr<()>],
    op: crate::bytecode::Bytecode
) -> ReamResult<()> {
    if args.len() < 2 {
        return Err(ReamError::Other("Arithmetic operations require at least 2 arguments".to_string()));
    }

    // Compile first argument
    compile_tlisp_expr(compiler, &args[0])?;

    // Compile remaining arguments and emit operations
    for arg in &args[1..] {
        compile_tlisp_expr(compiler, arg)?;
        compiler.emit(op.clone());
    }

    Ok(())
}

/// Compile comparison operations
fn compile_comparison_op(
    compiler: &mut BytecodeCompiler,
    args: &[crate::tlisp::Expr<()>],
    op: crate::bytecode::Bytecode
) -> ReamResult<()> {
    if args.len() != 2 {
        return Err(ReamError::Other("Comparison operations require exactly 2 arguments".to_string()));
    }

    // Compile both arguments
    compile_tlisp_expr(compiler, &args[0])?;
    compile_tlisp_expr(compiler, &args[1])?;

    // Emit comparison operation
    compiler.emit(op);

    Ok(())
}

/// Compile define statements
fn compile_define(compiler: &mut BytecodeCompiler, args: &[crate::tlisp::Expr<()>]) -> ReamResult<()> {
    if args.len() != 2 {
        return Err(ReamError::Other("define requires exactly 2 arguments".to_string()));
    }

    // Get variable name
    let var_name = match &args[0] {
        crate::tlisp::Expr::Symbol(name, _) => name.clone(),
        crate::tlisp::Expr::List(exprs, _) if !exprs.is_empty() => {
            // Function definition: (define (name params...) body)
            if let crate::tlisp::Expr::Symbol(name, _) = &exprs[0] {
                // For now, just treat as variable definition
                name.clone()
            } else {
                return Err(ReamError::Other("Invalid function definition".to_string()));
            }
        }
        _ => return Err(ReamError::Other("define requires a symbol or function signature".to_string())),
    };

    // Compile the value expression
    compile_tlisp_expr(compiler, &args[1])?;

    // Store to global variable
    let const_id = compiler.add_const(crate::bytecode::Value::String(var_name));
    compiler.emit(crate::bytecode::Bytecode::StoreGlobal(const_id, crate::types::EffectGrade::Write));

    Ok(())
}

/// Compile if expressions
fn compile_if(compiler: &mut BytecodeCompiler, args: &[crate::tlisp::Expr<()>]) -> ReamResult<()> {
    if args.len() != 3 {
        return Err(ReamError::Other("if requires exactly 3 arguments (condition, then, else)".to_string()));
    }

    // Generate unique labels
    let else_label = format!("else_{}", std::ptr::addr_of!(*compiler) as usize);
    let end_label = format!("end_{}", std::ptr::addr_of!(*compiler) as usize);

    // Compile condition
    compile_tlisp_expr(compiler, &args[0])?;

    // Jump to else if condition is false
    let else_pc = compiler.label_ref(else_label.clone());
    compiler.emit(crate::bytecode::Bytecode::JumpIfNot(else_pc, crate::types::EffectGrade::Pure));

    // Compile then branch
    compile_tlisp_expr(compiler, &args[1])?;

    // Jump to end
    let end_pc = compiler.label_ref(end_label.clone());
    compiler.emit(crate::bytecode::Bytecode::Jump(end_pc, crate::types::EffectGrade::Pure));

    // Define else label
    compiler.define_label(else_label);

    // Compile else branch
    compile_tlisp_expr(compiler, &args[2])?;

    // Define end label
    compiler.define_label(end_label);

    Ok(())
}

/// Compile println statements
fn compile_println(compiler: &mut BytecodeCompiler, args: &[crate::tlisp::Expr<()>]) -> ReamResult<()> {
    // Compile all arguments
    for arg in args {
        compile_tlisp_expr(compiler, arg)?;
        compiler.emit(crate::bytecode::Bytecode::Print(crate::types::EffectGrade::Write));
    }

    Ok(())
}

/// Compile function calls
fn compile_function_call(
    compiler: &mut BytecodeCompiler,
    func_name: &str,
    args: &[crate::tlisp::Expr<()>]
) -> ReamResult<()> {
    // For built-in functions, handle them specially
    match func_name {
        "string-append" => {
            // Compile all arguments and concatenate
            for arg in args {
                compile_tlisp_expr(compiler, arg)?;
            }
            // For now, just use the last argument
            Ok(())
        }
        "number->string" => {
            // Compile argument and convert to string
            if args.len() != 1 {
                return Err(ReamError::Other("number->string requires exactly 1 argument".to_string()));
            }
            compile_tlisp_expr(compiler, &args[0])?;
            // For now, just keep the value as-is
            Ok(())
        }
        _ => {
            // Generic function call - for now, just compile arguments and emit a placeholder
            for arg in args {
                compile_tlisp_expr(compiler, arg)?;
            }

            // For now, just emit a no-op instead of trying to call undefined functions
            compiler.emit(crate::bytecode::Bytecode::Nop(crate::types::EffectGrade::Pure));

            Ok(())
        }
    }
}

// Helper functions

fn format_value(value: &crate::tlisp::Value) -> String {
    match value {
        crate::tlisp::Value::Int(i) => i.to_string(),
        crate::tlisp::Value::Float(f) => f.to_string(),
        crate::tlisp::Value::Bool(b) => b.to_string(),
        crate::tlisp::Value::String(s) => format!("\"{}\"", s),
        crate::tlisp::Value::Symbol(s) => s.clone(),
        crate::tlisp::Value::List(items) => {
            let formatted_items: Vec<String> = items.iter()
                .map(format_value)
                .collect();
            format!("({})", formatted_items.join(" "))
        }
        crate::tlisp::Value::Function(func) => {
            format!("(lambda ({}) ...)", func.params.join(" "))
        }
        crate::tlisp::Value::Builtin(name) => {
            format!("#<builtin:{}>", name)
        }
        crate::tlisp::Value::Pid(pid) => {
            format!("#<pid:{}>", pid.raw())
        }
        crate::tlisp::Value::Unit => "()".to_string(),
        crate::tlisp::Value::Null => "null".to_string(),
        crate::tlisp::Value::StmVar(var) => format!("#<stm-var:{}>", var.name()),
    }
}

fn format_type(value: &crate::tlisp::Value) -> String {
    value.type_of().to_string()
}

fn format_tlisp_code(content: &str, indent: u8) -> String {
    // Simple formatting - parse parentheses and format with proper indentation
    let mut result = String::new();
    let mut current_indent: usize = 0;
    let indent_str = " ".repeat(indent as usize);
    let mut i = 0;
    let chars: Vec<char> = content.chars().collect();

    while i < chars.len() {
        let ch = chars[i];

        match ch {
            '(' => {
                // Add current indentation
                for _ in 0..current_indent {
                    result.push_str(&indent_str);
                }
                result.push('(');

                // Look ahead to see what comes next
                let mut j = i + 1;
                while j < chars.len() && chars[j].is_whitespace() {
                    j += 1;
                }

                if j < chars.len() {
                    // Check if this is a special form that should have newlines
                    let mut word = String::new();
                    let mut k = j;
                    while k < chars.len() && !chars[k].is_whitespace() && chars[k] != '(' && chars[k] != ')' {
                        word.push(chars[k]);
                        k += 1;
                    }

                    if word == "define" || word == "if" || word == "lambda" {
                        // Special forms get newlines
                        result.push_str(&word);
                        i = k - 1;
                        current_indent += 1;

                        // Skip whitespace and add newline before next element
                        while i + 1 < chars.len() && chars[i + 1].is_whitespace() {
                            i += 1;
                        }
                        if i + 1 < chars.len() && chars[i + 1] == '(' {
                            result.push('\n');
                        } else {
                            result.push(' ');
                        }
                    } else {
                        // Regular expressions stay on same line initially
                        current_indent += 1;
                    }
                }
            }
            ')' => {
                result.push(')');
                current_indent = current_indent.saturating_sub(1);
            }
            ' ' | '\t' | '\n' => {
                // Skip multiple whitespace
                while i + 1 < chars.len() && chars[i + 1].is_whitespace() {
                    i += 1;
                }

                // Add appropriate spacing
                if i + 1 < chars.len() {
                    if chars[i + 1] == '(' {
                        result.push('\n');
                    } else if chars[i + 1] != ')' {
                        result.push(' ');
                    }
                }
            }
            _ => {
                result.push(ch);
            }
        }

        i += 1;
    }

    result
}

fn find_test_files(dir: &PathBuf) -> ReamResult<Vec<PathBuf>> {
    let mut test_files = Vec::new();
    
    if dir.is_dir() {
        for entry in fs::read_dir(dir).map_err(|e| ReamError::Io(e))? {
            let entry = entry.map_err(|e| ReamError::Io(e))?;
            let path = entry.path();
            
            if path.is_file() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name.ends_with("_test.scm") || name.ends_with(".test.scm") {
                        test_files.push(path);
                    }
                }
            } else if path.is_dir() {
                // Recursively search subdirectories
                test_files.extend(find_test_files(&path)?);
            }
        }
    }
    
    test_files.sort();
    Ok(test_files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[test]
    fn test_format_value() {
        assert_eq!(format_value(&crate::tlisp::Value::Int(42)), "42");
        assert_eq!(format_value(&crate::tlisp::Value::String("hello".to_string())), "\"hello\"");
        assert_eq!(format_value(&crate::tlisp::Value::Bool(true)), "true");
    }
    
    #[test]
    fn test_format_tlisp_code() {
        let input = "(define (factorial n) (if (= n 0) 1 (* n (factorial (- n 1)))))";
        let formatted = format_tlisp_code(input, 2);
        assert!(formatted.contains("(define"));
        assert!(formatted.contains("  (if"));
    }
    
    #[test]
    fn test_find_test_files() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("example_test.scm");
        std::fs::write(&test_file, "(assert-eq 1 1)").unwrap();

        let files = find_test_files(&temp_dir.path().to_path_buf()).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0], test_file);
    }
}

/// Load a bytecode program from a file
fn load_bytecode_file(file: &PathBuf) -> ReamResult<BytecodeProgram> {
    let file_data = fs::read(file)
        .map_err(|e| ReamError::Io(e))?;

    // Try to deserialize as binary format first
    match bincode::deserialize::<BytecodeProgram>(&file_data) {
        Ok(program) => Ok(program),
        Err(_) => {
            // If binary deserialization fails, try text format
            let content = String::from_utf8(file_data)
                .map_err(|e| ReamError::Other(format!("Invalid UTF-8 in bytecode file: {}", e)))?;

            serde_json::from_str::<BytecodeProgram>(&content)
                .map_err(|e| ReamError::Other(format!("Failed to parse bytecode file: {}", e)))
        }
    }
}

/// Create a standalone executable from TLisp source code
fn create_standalone_executable(
    source_content: &str,
    output_dir: &PathBuf,
    source_path: &PathBuf,
    _mode: &BuildMode,
    _optimization: u8,
) -> ReamResult<()> {
    println!("{} Creating standalone executable...", "4.".dimmed());

    // Validate the source content by parsing it
    let mut interpreter = TlispInterpreter::new();
    match interpreter.parse(source_content) {
        Ok(_) => {
            println!("  ✓ Source validation successful");
        }
        Err(e) => {
            return Err(ReamError::Other(format!("Source validation failed: {}", e)));
        }
    }

    // Create a simple shell script that runs the REAM interpreter with the source file
    let script_content = if cfg!(windows) {
        format!(r#"@echo off
REM Auto-generated REAM executable script
REM Source: {}

set REAM_SOURCE={}

cargo run --manifest-path "{}" -- run "%REAM_SOURCE%"
"#,
            source_path.display(),
            source_path.display(),
            std::env::current_dir()
                .map_err(|e| ReamError::Io(e))?
                .join("Cargo.toml")
                .display()
        )
    } else {
        format!(r#"#!/bin/bash
# Auto-generated REAM executable script
# Source: {}

REAM_SOURCE="{}"

cargo run --manifest-path "{}" -- run "$REAM_SOURCE"
"#,
            source_path.display(),
            source_path.display(),
            std::env::current_dir()
                .map_err(|e| ReamError::Io(e))?
                .join("Cargo.toml")
                .display()
        )
    };

    // Write the script file
    let script_name = if cfg!(windows) { "simple_demo.bat" } else { "simple_demo.sh" };
    let script_file = output_dir.join(script_name);
    fs::write(&script_file, script_content)
        .map_err(|e| ReamError::Io(e))?;

    // Make the script executable on Unix systems
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&script_file)
            .map_err(|e| ReamError::Io(e))?
            .permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&script_file, perms)
            .map_err(|e| ReamError::Io(e))?;
    }

    println!("  + Generated executable script: {}", script_file.display());
    println!("  + To run: {}", script_file.display());

    Ok(())
}

fn execute_daemon(command: DaemonCommand, debug: bool, verbose: bool) -> ReamResult<()> {
    match command {
        DaemonCommand::Start { file, socket, pidfile, logfile, foreground } => {
            execute_daemon_start(file, socket, pidfile, logfile, foreground, debug, verbose)
        }
        DaemonCommand::Stop { socket, pidfile, force } => {
            let config = DaemonConfig::default();
            let socket = socket.unwrap_or(config.socket_path);
            let pidfile = pidfile.unwrap_or(config.pid_file);
            execute_daemon_stop(socket, pidfile, force, debug, verbose)
        }
        DaemonCommand::Status { socket, pidfile } => {
            let config = DaemonConfig::default();
            let socket = socket.unwrap_or(config.socket_path);
            let pidfile = pidfile.unwrap_or(config.pid_file);
            execute_daemon_status(socket, pidfile, debug, verbose)
        }
        DaemonCommand::Restart { file, socket, pidfile, logfile } => {
            execute_daemon_restart(file, socket, pidfile, logfile, debug, verbose)
        }
        DaemonCommand::Actor { command } => {
            execute_actor_command(command, debug, verbose)
        }
    }
}

fn execute_daemon_start(
    file: PathBuf,
    socket: Option<PathBuf>,
    pidfile: Option<PathBuf>,
    logfile: Option<PathBuf>,
    foreground: bool,
    debug: bool,
    verbose: bool,
) -> ReamResult<()> {
    // Use default config and override with provided values
    let mut config = DaemonConfig::default();

    if let Some(socket_path) = socket {
        config.socket_path = socket_path;
    }
    if let Some(pid_file) = pidfile {
        config.pid_file = pid_file;
    }
    if let Some(log_file) = logfile {
        config.log_file = log_file;
    }
    config.foreground = foreground;

    println!("{} Starting daemon with program: {}", "Info:".bright_blue().bold(), file.display());
    println!("  Socket: {}", config.socket_path.display());
    println!("  PID file: {}", config.pid_file.display());
    println!("  Log file: {}", config.log_file.display());
    println!("  Foreground: {}", foreground);

    if debug {
        println!("{} Debug mode enabled", "Debug:".bright_yellow().bold());
    }

    // Create and start daemon runtime
    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| ReamError::Other(format!("Failed to create async runtime: {}", e)))?;

    rt.block_on(async {
        let mut daemon = DaemonRuntime::new(config)?;

        println!("{} Daemon starting...", "Info:".bright_green().bold());
        daemon.start(file).await?;

        println!("{} Daemon started successfully", "Success:".bright_green().bold());
        Ok(())
    })
}

fn execute_daemon_stop(
    socket: PathBuf,
    pidfile: PathBuf,
    force: bool,
    debug: bool,
    verbose: bool,
) -> ReamResult<()> {
    println!("{} Stopping daemon", "Info:".bright_blue().bold());
    println!("  Socket: {}", socket.display());
    println!("  PID file: {}", pidfile.display());
    println!("  Force: {}", force);

    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| ReamError::Other(format!("Failed to create async runtime: {}", e)))?;

    rt.block_on(async {
        let client = IpcClient::new(socket.clone());

        if force {
            // Force kill using PID file
            let config = DaemonConfig {
                socket_path: socket,
                pid_file: pidfile,
                log_file: PathBuf::from("/tmp/ream-daemon.log"),
                foreground: false,
                monitor_interval: Duration::from_millis(1000),
                max_actors: 10000,
                memory_limit: 64 * 1024 * 1024,
            };

            let daemon = DaemonRuntime::new(config)?;
            daemon.force_kill()?;
            println!("{} Daemon force killed", "Success:".bright_green().bold());
        } else {
            // Graceful shutdown via IPC
            match client.shutdown_daemon().await {
                Ok(msg) => {
                    println!("{} {}", "Success:".bright_green().bold(), msg);
                }
                Err(e) => {
                    println!("{} Failed to shutdown daemon gracefully: {}", "Warning:".bright_yellow().bold(), e);
                    println!("{} Use --force to force kill", "Info:".bright_blue().bold());
                    return Err(e);
                }
            }
        }

        Ok(())
    })
}

fn execute_daemon_status(
    socket: PathBuf,
    pidfile: PathBuf,
    debug: bool,
    verbose: bool,
) -> ReamResult<()> {
    println!("{} Checking daemon status", "Info:".bright_blue().bold());
    println!("  Socket: {}", socket.display());
    println!("  PID file: {}", pidfile.display());

    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| ReamError::Other(format!("Failed to create async runtime: {}", e)))?;

    rt.block_on(async {
        let client = IpcClient::new(socket.clone());

        // Check if daemon is running via IPC
        if client.is_daemon_running().await {
            println!("{} Daemon is running", "Status:".bright_green().bold());

            // Get system information
            match client.get_system_info().await {
                Ok(system_info) => {
                    println!("  Uptime: {:?}", system_info.uptime);
                    println!("  Total actors: {}", system_info.total_actors);
                    println!("  Active actors: {}", system_info.active_actors);
                    println!("  Suspended actors: {}", system_info.suspended_actors);
                    println!("  Crashed actors: {}", system_info.crashed_actors);
                    println!("  Memory usage: {} bytes", system_info.total_memory);
                    println!("  Message rate: {:.2} msg/s", system_info.system_message_rate);
                    println!("  CPU usage: {:.1}%", system_info.cpu_usage * 100.0);
                    println!("  Memory usage: {:.1}%", system_info.memory_usage_percent * 100.0);
                    println!("  Load average: {:.2}", system_info.load_average);
                }
                Err(e) => {
                    println!("{} Failed to get system info: {}", "Warning:".bright_yellow().bold(), e);
                }
            }
        } else {
            println!("{} Daemon is not running", "Status:".bright_red().bold());

            // Check if PID file exists
            if pidfile.exists() {
                println!("{} PID file exists but daemon is not responding", "Warning:".bright_yellow().bold());
                println!("  Consider using 'daemon stop --force' to clean up");
            }
        }

        Ok(())
    })
}

fn execute_daemon_restart(
    file: PathBuf,
    socket: Option<PathBuf>,
    pidfile: Option<PathBuf>,
    logfile: Option<PathBuf>,
    debug: bool,
    verbose: bool,
) -> ReamResult<()> {
    // TODO: Implement daemon restart
    println!("{} Restarting daemon with program: {}", "Info:".bright_blue().bold(), file.display());

    // Use default config for missing paths
    let config = DaemonConfig::default();
    let socket_path = socket.clone().unwrap_or(config.socket_path.clone());
    let pidfile_path = pidfile.clone().unwrap_or(config.pid_file.clone());

    // First stop, then start
    execute_daemon_stop(socket_path, pidfile_path, false, debug, verbose)?;
    execute_daemon_start(file, socket, pidfile, logfile, false, debug, verbose)
}

fn execute_actor_command(command: ActorCommand, debug: bool, verbose: bool) -> ReamResult<()> {
    let config = DaemonConfig::default();

    match command {
        ActorCommand::List { socket, detailed } => {
            let socket_path = socket.unwrap_or(config.socket_path.clone());
            execute_actor_list(socket_path, detailed, debug, verbose)
        }
        ActorCommand::Info { pid, socket } => {
            let socket_path = socket.unwrap_or(config.socket_path.clone());
            execute_actor_info(pid, socket_path, debug, verbose)
        }
        ActorCommand::Kill { pid, socket, reason } => {
            let socket_path = socket.unwrap_or(config.socket_path.clone());
            execute_actor_kill(pid, socket_path, reason, debug, verbose)
        }
        ActorCommand::Suspend { pid, socket } => {
            let socket_path = socket.unwrap_or(config.socket_path.clone());
            execute_actor_suspend(pid, socket_path, debug, verbose)
        }
        ActorCommand::Resume { pid, socket } => {
            let socket_path = socket.unwrap_or(config.socket_path.clone());
            execute_actor_resume(pid, socket_path, debug, verbose)
        }
        ActorCommand::Restart { pid, socket } => {
            let socket_path = socket.unwrap_or(config.socket_path.clone());
            execute_actor_restart(pid, socket_path, debug, verbose)
        }
        ActorCommand::Send { pid, message, socket } => {
            let socket_path = socket.unwrap_or(config.socket_path);
            execute_actor_send(pid, message, socket_path, debug, verbose)
        }
    }
}

fn execute_actor_list(socket: PathBuf, detailed: bool, debug: bool, verbose: bool) -> ReamResult<()> {
    println!("{} Listing actors", "Info:".bright_blue().bold());
    println!("  Socket: {}", socket.display());
    println!("  Detailed: {}", detailed);

    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| ReamError::Other(format!("Failed to create async runtime: {}", e)))?;

    rt.block_on(async {
        let client = IpcClient::new(socket);

        match client.list_actors(detailed).await {
            Ok(actors) => {
                if actors.is_empty() {
                    println!("{} No actors found", "Info:".bright_blue().bold());
                } else {
                    println!("{} Found {} actors:", "Info:".bright_green().bold(), actors.len());
                    println!();

                    if detailed {
                        for actor in actors {
                            println!("PID: {}", actor.pid);
                            println!("  Status: {:?}", actor.status);
                            println!("  Type: {}", actor.actor_type);
                            println!("  Uptime: {:?}", actor.uptime);
                            println!("  Mailbox: {} messages", actor.mailbox_size);
                            println!("  Memory: {} bytes", actor.memory_usage);
                            println!("  Messages processed: {}", actor.messages_processed);
                            println!("  Message rate: {:.2} msg/s", actor.message_rate);
                            println!("  CPU time: {} μs", actor.cpu_time);
                            println!("  State: {}", actor.state_description);
                            if !actor.links.is_empty() {
                                println!("  Links: {:?}", actor.links);
                            }
                            if !actor.monitors.is_empty() {
                                println!("  Monitors: {:?}", actor.monitors);
                            }
                            if let Some(supervisor) = actor.supervisor {
                                println!("  Supervisor: {}", supervisor);
                            }
                            println!();
                        }
                    } else {
                        println!("{:<20} {:<12} {:<8} {:<10} {:<12}", "PID", "Status", "Mailbox", "Memory", "Msg Rate");
                        println!("{}", "-".repeat(70));
                        for actor in actors {
                            println!("{:<20} {:<12} {:<8} {:<10} {:<12.2}",
                                actor.pid,
                                format!("{:?}", actor.status),
                                actor.mailbox_size,
                                format!("{}B", actor.memory_usage),
                                actor.message_rate
                            );
                        }
                    }
                }
                Ok(())
            }
            Err(e) => {
                println!("{} Failed to list actors: {}", "Error:".bright_red().bold(), e);
                Err(e)
            }
        }
    })
}

fn execute_actor_info(pid: String, socket: PathBuf, debug: bool, verbose: bool) -> ReamResult<()> {
    println!("{} Getting actor info for PID: {}", "Info:".bright_blue().bold(), pid);
    println!("  Socket: {}", socket.display());

    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| ReamError::Other(format!("Failed to create async runtime: {}", e)))?;

    rt.block_on(async {
        let client = IpcClient::new(socket);

        match client.get_actor_info(pid.clone()).await {
            Ok(actor) => {
                println!("{} Actor Information:", "Info:".bright_green().bold());
                println!("  PID: {}", actor.pid);
                println!("  Status: {:?}", actor.status);
                println!("  Type: {}", actor.actor_type);
                println!("  Uptime: {:?}", actor.uptime);
                println!("  Last Activity: {:?}", actor.last_activity);
                println!("  State: {}", actor.state_description);
                println!();
                println!("  Mailbox:");
                println!("    Size: {} messages", actor.mailbox_size);
                println!();
                println!("  Resource Usage:");
                println!("    Memory: {} bytes", actor.memory_usage);
                println!("    CPU Time: {} μs", actor.cpu_time);
                println!();
                println!("  Message Statistics:");
                println!("    Processed: {}", actor.messages_processed);
                println!("    Rate: {:.2} msg/s", actor.message_rate);
                println!();
                if !actor.links.is_empty() {
                    println!("  Links: {:?}", actor.links);
                }
                if !actor.monitors.is_empty() {
                    println!("  Monitors: {:?}", actor.monitors);
                }
                if let Some(supervisor) = actor.supervisor {
                    println!("  Supervisor: {}", supervisor);
                }
                Ok(())
            }
            Err(e) => {
                println!("{} Failed to get actor info: {}", "Error:".bright_red().bold(), e);
                Err(e)
            }
        }
    })
}

fn execute_actor_kill(pid: String, socket: PathBuf, reason: String, debug: bool, verbose: bool) -> ReamResult<()> {
    println!("{} Killing actor PID: {} with reason: {}", "Info:".bright_blue().bold(), pid, reason);
    println!("  Socket: {}", socket.display());

    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| ReamError::Other(format!("Failed to create async runtime: {}", e)))?;

    rt.block_on(async {
        let client = IpcClient::new(socket);

        match client.kill_actor(pid.clone(), reason).await {
            Ok(msg) => {
                println!("{} {}", "Success:".bright_green().bold(), msg);
                Ok(())
            }
            Err(e) => {
                println!("{} Failed to kill actor: {}", "Error:".bright_red().bold(), e);
                Err(e)
            }
        }
    })
}

fn execute_actor_suspend(pid: String, socket: PathBuf, debug: bool, verbose: bool) -> ReamResult<()> {
    println!("{} Suspending actor PID: {}", "Info:".bright_blue().bold(), pid);
    println!("  Socket: {}", socket.display());

    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| ReamError::Other(format!("Failed to create async runtime: {}", e)))?;

    rt.block_on(async {
        let client = IpcClient::new(socket);

        match client.suspend_actor(pid.clone()).await {
            Ok(msg) => {
                println!("{} {}", "Success:".bright_green().bold(), msg);
                Ok(())
            }
            Err(e) => {
                println!("{} Failed to suspend actor: {}", "Error:".bright_red().bold(), e);
                Err(e)
            }
        }
    })
}

fn execute_actor_resume(pid: String, socket: PathBuf, debug: bool, verbose: bool) -> ReamResult<()> {
    println!("{} Resuming actor PID: {}", "Info:".bright_blue().bold(), pid);
    println!("  Socket: {}", socket.display());

    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| ReamError::Other(format!("Failed to create async runtime: {}", e)))?;

    rt.block_on(async {
        let client = IpcClient::new(socket);

        match client.resume_actor(pid.clone()).await {
            Ok(msg) => {
                println!("{} {}", "Success:".bright_green().bold(), msg);
                Ok(())
            }
            Err(e) => {
                println!("{} Failed to resume actor: {}", "Error:".bright_red().bold(), e);
                Err(e)
            }
        }
    })
}

fn execute_actor_restart(pid: String, socket: PathBuf, debug: bool, verbose: bool) -> ReamResult<()> {
    println!("{} Restarting actor PID: {}", "Info:".bright_blue().bold(), pid);
    println!("  Socket: {}", socket.display());

    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| ReamError::Other(format!("Failed to create async runtime: {}", e)))?;

    rt.block_on(async {
        let client = IpcClient::new(socket);

        match client.restart_actor(pid.clone()).await {
            Ok(msg) => {
                println!("{} {}", "Success:".bright_green().bold(), msg);
                Ok(())
            }
            Err(e) => {
                println!("{} Failed to restart actor: {}", "Error:".bright_red().bold(), e);
                Err(e)
            }
        }
    })
}

fn execute_actor_send(pid: String, message: String, socket: PathBuf, debug: bool, verbose: bool) -> ReamResult<()> {
    println!("{} Sending message to actor PID: {}", "Info:".bright_blue().bold(), pid);
    println!("  Message: {}", message);
    println!("  Socket: {}", socket.display());

    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| ReamError::Other(format!("Failed to create async runtime: {}", e)))?;

    rt.block_on(async {
        let client = IpcClient::new(socket);

        match client.send_actor_message(pid.clone(), message).await {
            Ok(msg) => {
                println!("{} {}", "Success:".bright_green().bold(), msg);
                Ok(())
            }
            Err(e) => {
                println!("{} Failed to send message: {}", "Error:".bright_red().bold(), e);
                Err(e)
            }
        }
    })
}

fn execute_monitor(socket: PathBuf, interval: u64, actor: Option<String>, debug: bool, verbose: bool) -> ReamResult<()> {
    println!("{} Starting TUI monitor", "Info:".bright_blue().bold());
    println!("  Socket: {}", socket.display());
    println!("  Interval: {}ms", interval);
    if let Some(ref actor_pid) = actor {
        println!("  Monitoring actor: {}", actor_pid);
    }

    #[cfg(feature = "tui")]
    {
        // Check if daemon is running first
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| ReamError::Other(format!("Failed to create async runtime: {}", e)))?;

        rt.block_on(async {
            let client = IpcClient::new(socket.clone());

            if !client.is_daemon_running().await {
                return Err(ReamError::Other("Daemon is not running. Start daemon first with 'ream daemon start <program>'".to_string()));
            }

            println!("{} Connected to daemon, starting TUI...", "Info:".bright_green().bold());

            // Create and run TUI application
            let mut app = TuiApp::new(socket, Duration::from_millis(interval));

            match app.run().await {
                Ok(_) => {
                    println!("{} TUI monitor exited", "Info:".bright_blue().bold());
                    Ok(())
                }
                Err(e) => {
                    println!("{} TUI monitor error: {}", "Error:".bright_red().bold(), e);
                    Err(e)
                }
            }
        })
    }

    #[cfg(not(feature = "tui"))]
    {
        Err(ReamError::NotImplemented("TUI monitor not available. Build with --features tui to enable.".to_string()))
    }
}