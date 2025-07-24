use clap::{Parser, Subcommand};
use colored::*;
use std::path::PathBuf;

/// REAM - Rust Erlang Abstract Machine
/// A mathematically-grounded actor runtime with bytecode JIT compilation and TLISP
#[derive(Parser)]
#[command(name = "ream")]
#[command(author = "REAM Team")]
#[command(version = "0.1.0")]
#[command(about = "Rust Erlang Abstract Machine - Actor runtime with TLISP and JIT compilation")]
#[command(long_about = "
REAM is a mathematically-grounded actor runtime featuring:
- Erlang-style actor model with supervision trees
- Bytecode VM with JIT compilation for performance
- TLISP (Typed Lisp) for functional programming
- Fault-tolerant distributed computing
- WebAssembly integration
- Software Transactional Memory (STM)

Usage examples:
  ream                    # Start interactive REPL
  ream run script.scm     # Run a TLISP script
  ream build project.scm  # Build a TLISP project
")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
    
    /// Enable verbose output
    #[arg(short, long, global = true)]
    pub verbose: bool,
    
    /// Enable debug mode
    #[arg(short, long, global = true)]
    pub debug: bool,
    
    /// Disable colored output
    #[arg(long, global = true)]
    pub no_color: bool,
}

/// Available commands for the REAM CLI
#[derive(Subcommand)]
pub enum Commands {
    /// Start interactive REPL (default)
    #[command(alias = "repl")]
    Interactive {
        /// Load script file at startup
        #[arg(short, long)]
        load: Option<PathBuf>,
        
        /// Show startup banner
        #[arg(short, long, default_value = "true")]
        banner: bool,
        
        /// Enable history
        #[arg(long, default_value = "true")]
        history: bool,
        
        /// History file path
        #[arg(long, default_value = ".ream_history")]
        history_file: PathBuf,
    },
    
    /// Run a TLISP script file
    Run {
        /// Path to the TLISP script file
        #[arg(value_name = "FILE")]
        file: PathBuf,
        
        /// Arguments to pass to the script
        #[arg(last = true)]
        args: Vec<String>,
        
        /// Show execution timing
        #[arg(short, long)]
        time: bool,
        
        /// Enable JIT compilation
        #[arg(short, long, default_value = "true")]
        jit: bool,
        
        /// Optimization level (0-3)
        #[arg(short = 'O', long, default_value = "2")]
        optimization: u8,
    },
    
    /// Build a TLISP project
    Build {
        /// Path to the project file or directory
        #[arg(value_name = "PATH")]
        path: PathBuf,
        
        /// Output directory
        #[arg(short, long, default_value = "build")]
        output: PathBuf,
        
        /// Build mode (debug/release)
        #[arg(short, long, default_value = "release")]
        mode: BuildMode,
        
        /// Enable optimizations
        #[arg(short = 'O', long, default_value = "2")]
        optimization: u8,
        
        /// Target type (executable, library, etc.)
        #[arg(long, default_value = "executable")]
        target: BuildTarget,

        /// Target architecture
        #[arg(short, long, default_value = "native")]
        arch: String,
        
        /// Enable WebAssembly output
        #[arg(long)]
        wasm: bool,
    },
    
    /// Check TLISP code for errors
    Check {
        /// Path to the TLISP file
        #[arg(value_name = "FILE")]
        file: PathBuf,
        
        /// Show detailed type information
        #[arg(long)]
        types: bool,
        
        /// Show warnings
        #[arg(short, long, default_value = "true")]
        warnings: bool,
    },
    
    /// Format TLISP code
    Format {
        /// Path to the TLISP file
        #[arg(value_name = "FILE")]
        file: PathBuf,
        
        /// Format in place
        #[arg(short, long)]
        in_place: bool,
        
        /// Indentation size
        #[arg(short, long, default_value = "2")]
        indent: u8,
    },
    
    /// Show system information
    Info {
        /// Show detailed system info
        #[arg(long)]
        detailed: bool,
    },
    
    /// Run tests
    Test {
        /// Path to test file or directory
        #[arg(value_name = "PATH")]
        path: Option<PathBuf>,
        
        /// Run tests in parallel
        #[arg(short, long, default_value = "true")]
        parallel: bool,
        
        /// Show test output
        #[arg(short, long)]
        verbose: bool,
    },
    
    /// Compile TLISP code to bytecode
    Compile {
        /// Path to the TLISP file
        #[arg(value_name = "FILE")]
        file: PathBuf,

        /// Output file path (defaults to input file with .reambc extension)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Output format (binary or text)
        #[arg(short, long, default_value = "binary")]
        format: CompileFormat,

        /// Optimization level (0-3)
        #[arg(short = 'O', long, default_value = "2")]
        optimization: u8,

        /// Include debug information
        #[arg(long)]
        debug_info: bool,

        /// Show compilation statistics
        #[arg(long)]
        stats: bool,
    },

    /// Execute compiled bytecode
    #[command(alias = "exec")]
    Execute {
        /// Path to the bytecode file (.reambc)
        #[arg(value_name = "FILE")]
        file: PathBuf,

        /// Command line arguments to pass to the program
        #[arg(value_name = "ARGS")]
        args: Vec<String>,

        /// Show execution timing
        #[arg(short, long)]
        time: bool,

        /// Use JIT compilation
        #[arg(long, default_value = "true")]
        jit: bool,

        /// Show execution statistics
        #[arg(long)]
        stats: bool,
    },

    /// Package management
    Package {
        #[command(subcommand)]
        command: PackageCommand,
    },

    /// Daemon mode operations
    Daemon {
        #[command(subcommand)]
        command: DaemonCommand,
    },

    /// Monitor running system
    Monitor {
        /// Daemon socket path
        #[arg(short, long)]
        socket: Option<PathBuf>,

        /// Refresh interval in milliseconds
        #[arg(short, long, default_value = "1000")]
        interval: u64,

        /// Show only specific actor by PID
        #[arg(long)]
        actor: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum PackageCommand {
    /// Install a package
    Install {
        /// Package name
        name: String,
        
        /// Package version
        #[arg(short, long)]
        version: Option<String>,
    },
    
    /// Remove a package
    Remove {
        /// Package name
        name: String,
    },
    
    /// List installed packages
    List,
    
    /// Update packages
    Update,
}

/// Daemon management commands
#[derive(Subcommand)]
pub enum DaemonCommand {
    /// Start daemon with a TLisp program
    Start {
        /// TLisp program file to run
        #[arg(value_name = "FILE")]
        file: PathBuf,

        /// Daemon socket path
        #[arg(short, long)]
        socket: Option<PathBuf>,

        /// PID file path
        #[arg(short, long)]
        pidfile: Option<PathBuf>,

        /// Log file path
        #[arg(short, long)]
        logfile: Option<PathBuf>,

        /// Run in foreground (don't daemonize)
        #[arg(long)]
        foreground: bool,
    },

    /// Stop running daemon
    Stop {
        /// Daemon socket path
        #[arg(short, long)]
        socket: Option<PathBuf>,

        /// PID file path
        #[arg(short, long)]
        pidfile: Option<PathBuf>,

        /// Force kill if graceful shutdown fails
        #[arg(long)]
        force: bool,
    },

    /// Check daemon status
    Status {
        /// Daemon socket path
        #[arg(short, long)]
        socket: Option<PathBuf>,

        /// PID file path
        #[arg(short, long)]
        pidfile: Option<PathBuf>,
    },

    /// Restart daemon
    Restart {
        /// TLisp program file to run
        #[arg(value_name = "FILE")]
        file: PathBuf,

        /// Daemon socket path
        #[arg(short, long)]
        socket: Option<PathBuf>,

        /// PID file path
        #[arg(short, long)]
        pidfile: Option<PathBuf>,

        /// Log file path
        #[arg(short, long)]
        logfile: Option<PathBuf>,
    },

    /// Manage actors in running daemon
    Actor {
        #[command(subcommand)]
        command: ActorCommand,
    },
}

/// Actor management commands
#[derive(Subcommand)]
pub enum ActorCommand {
    /// List all actors
    List {
        /// Daemon socket path
        #[arg(short, long)]
        socket: Option<PathBuf>,

        /// Show detailed information
        #[arg(long)]
        detailed: bool,
    },

    /// Show actor details
    Info {
        /// Actor PID
        #[arg(value_name = "PID")]
        pid: String,

        /// Daemon socket path
        #[arg(short, long)]
        socket: Option<PathBuf>,
    },

    /// Kill an actor
    Kill {
        /// Actor PID
        #[arg(value_name = "PID")]
        pid: String,

        /// Daemon socket path
        #[arg(short, long)]
        socket: Option<PathBuf>,

        /// Reason for killing
        #[arg(short, long, default_value = "normal")]
        reason: String,
    },

    /// Suspend an actor
    Suspend {
        /// Actor PID
        #[arg(value_name = "PID")]
        pid: String,

        /// Daemon socket path
        #[arg(short, long)]
        socket: Option<PathBuf>,
    },

    /// Resume a suspended actor
    Resume {
        /// Actor PID
        #[arg(value_name = "PID")]
        pid: String,

        /// Daemon socket path
        #[arg(short, long)]
        socket: Option<PathBuf>,
    },

    /// Restart an actor
    Restart {
        /// Actor PID
        #[arg(value_name = "PID")]
        pid: String,

        /// Daemon socket path
        #[arg(short, long)]
        socket: Option<PathBuf>,
    },

    /// Send a message to an actor
    Send {
        /// Actor PID
        #[arg(value_name = "PID")]
        pid: String,

        /// Message content (TLisp expression)
        #[arg(value_name = "MESSAGE")]
        message: String,

        /// Daemon socket path
        #[arg(short, long)]
        socket: Option<PathBuf>,
    },
}

/// Build mode for compilation
#[derive(clap::ValueEnum, Clone, Debug)]
pub enum BuildMode {
    /// Debug build with debug symbols and no optimizations
    Debug,
    /// Release build with optimizations
    Release,
}

/// Target format for build output
#[derive(clap::ValueEnum, Clone, Debug, PartialEq)]
pub enum BuildTarget {
    /// Standalone executable
    Executable,
    /// Shared library
    Library,
    /// Static library
    StaticLib,
    /// WebAssembly module
    Wasm,
}

/// Output format for compilation
#[derive(clap::ValueEnum, Clone, Debug)]
pub enum CompileFormat {
    /// Binary bytecode format
    Binary,
    /// Human-readable text format
    Text,
}

impl std::fmt::Display for BuildMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BuildMode::Debug => write!(f, "debug"),
            BuildMode::Release => write!(f, "release"),
        }
    }
}

impl std::fmt::Display for BuildTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BuildTarget::Executable => write!(f, "executable"),
            BuildTarget::Library => write!(f, "library"),
            BuildTarget::StaticLib => write!(f, "static-lib"),
            BuildTarget::Wasm => write!(f, "wasm"),
        }
    }
}

pub fn print_banner() {
    println!("{}", "
██████╗ ███████╗ █████╗ ███╗   ███╗
██╔══██╗██╔════╝██╔══██╗████╗ ████║
██████╔╝█████╗  ███████║██╔████╔██║
██╔══██╗██╔══╝  ██╔══██║██║╚██╔╝██║
██║  ██║███████╗██║  ██║██║ ╚═╝ ██║
╚═╝  ╚═╝╚══════╝╚═╝  ╚═╝╚═╝     ╚═╝
".bright_cyan());
    
    println!("{}", "Rust Erlang Abstract Machine".bright_white().bold());
    println!("{}", "Version 0.1.0".dimmed());
    println!("{}", "Type 'help' for available commands, 'quit' to exit".dimmed());
    println!();
}

pub fn print_help() {
    println!("{}", "Available REPL commands:".bright_yellow().bold());
    println!("  {}  - Show this help message", "help".bright_green());
    println!("  {}  - Exit the REPL", "quit".bright_green());
    println!("  {}  - Clear the screen", "clear".bright_green());
    println!("  {}  - Show system information", "info".bright_green());
    println!("  {}  - Show current environment", "env".bright_green());
    println!("  {}  - Load a script file", "load <file>".bright_green());
    println!("  {}  - Reset the environment", "reset".bright_green());
    println!("  {}  - Show command history", "history".bright_green());
    println!("  {}  - Time expression evaluation", "time <expr>".bright_green());
    println!("  {}  - Show type of expression", "type <expr>".bright_green());
    println!("  {}  - Show bytecode for expression", "bytecode <expr>".bright_green());
    println!("  {}  - Show JIT assembly for expression", "asm <expr>".bright_green());
    println!("  {}  - Toggle debug mode", "debug".bright_green());
    println!("  {}  - Toggle JIT compilation", "jit".bright_green());
    println!();
    println!("{}", "TLISP expressions:".bright_yellow().bold());
    println!("  {}  - Define a variable", "(define x 42)".bright_blue());
    println!("  {}  - Lambda function", "(lambda (x) (* x x))".bright_blue());
    println!("  {}  - Function application", "(+ 1 2 3)".bright_blue());
    println!("  {}  - Conditional", "(if (> x 0) \"positive\" \"negative\")".bright_blue());
    println!("  {}  - List operations", "(list 1 2 3)".bright_blue());
    println!("  {}  - Actor operations", "(spawn (lambda () (loop)))".bright_blue());
    println!();
}

pub fn print_info() {
    println!("{}", "REAM System Information".bright_yellow().bold());
    println!("  Version: {}", env!("CARGO_PKG_VERSION").bright_green());
    println!("  Build: {}", if cfg!(debug_assertions) { "debug" } else { "release" }.bright_green());
    println!("  Target: {}", std::env::consts::ARCH.bright_green());
    println!("  Rust: {}", "stable".bright_green());
    println!();
    
    println!("{}", "Runtime Features:".bright_yellow().bold());
    println!("  ✓ Actor System with supervision trees");
    println!("  ✓ Bytecode VM with JIT compilation");
    println!("  ✓ TLISP with Hindley-Milner type inference");
    println!("  ✓ Software Transactional Memory (STM)");
    println!("  ✓ WebAssembly integration");
    println!("  ✓ Fault tolerance and hot code reloading");
    println!();
    
    println!("{}", "Memory Usage:".bright_yellow().bold());
    // This would show actual memory usage in a real implementation
    println!("  Heap: {} MB", "12.5".bright_green());
    println!("  Stack: {} KB", "256".bright_green());
    println!("  Actors: {} active", "42".bright_green());
    println!();
}

pub fn colorize_output(text: &str, color: &str) -> String {
    if colored::control::SHOULD_COLORIZE.should_colorize() {
        match color {
            "red" => text.red().to_string(),
            "green" => text.green().to_string(),
            "yellow" => text.yellow().to_string(),
            "blue" => text.blue().to_string(),
            "magenta" => text.magenta().to_string(),
            "cyan" => text.cyan().to_string(),
            "white" => text.white().to_string(),
            "bright_red" => text.bright_red().to_string(),
            "bright_green" => text.bright_green().to_string(),
            "bright_yellow" => text.bright_yellow().to_string(),
            "bright_blue" => text.bright_blue().to_string(),
            "bright_magenta" => text.bright_magenta().to_string(),
            "bright_cyan" => text.bright_cyan().to_string(),
            "bright_white" => text.bright_white().to_string(),
            "bold" => text.bold().to_string(),
            "dimmed" => text.dimmed().to_string(),
            _ => text.to_string(),
        }
    } else {
        text.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_cli_parsing() {
        // Test default (no subcommand)
        let cli = Cli::parse_from(&["ream"]);
        assert!(cli.command.is_none());
        
        // Test run subcommand
        let cli = Cli::parse_from(&["ream", "run", "script.scm"]);
        match cli.command {
            Some(Commands::Run { file, .. }) => {
                assert_eq!(file, PathBuf::from("script.scm"));
            }
            _ => panic!("Expected Run command"),
        }
        
        // Test build subcommand
        let cli = Cli::parse_from(&["ream", "build", "project.scm", "--mode", "debug"]);
        match cli.command {
            Some(Commands::Build { path, mode, .. }) => {
                assert_eq!(path, PathBuf::from("project.scm"));
                assert!(matches!(mode, BuildMode::Debug));
            }
            _ => panic!("Expected Build command"),
        }
    }
    
    #[test]
    fn test_global_flags() {
        let cli = Cli::parse_from(&["ream", "--verbose", "--debug", "run", "script.scm"]);
        assert!(cli.verbose);
        assert!(cli.debug);
    }
    
    #[test]
    fn test_build_mode_display() {
        assert_eq!(BuildMode::Debug.to_string(), "debug");
        assert_eq!(BuildMode::Release.to_string(), "release");
    }
}