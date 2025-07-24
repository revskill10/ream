use ream::cli::Cli;
use ream::commands::execute_command;
use ream::repl::start_repl;
use clap::Parser;
use colored::*;
use std::process;
use std::path::PathBuf;

fn main() {
    // Parse command line arguments
    let cli = Cli::parse();
    
    // Set up colored output
    if cli.no_color {
        colored::control::set_override(false);
    }
    
    // Handle the command
    let result = match cli.command {
        Some(command) => execute_command(command, cli.debug, cli.verbose),
        None => {
            // Default to interactive REPL mode
            start_repl(None, true, PathBuf::from(".ream_history"))
        }
    };
    
    // Handle any errors
    if let Err(e) = result {
        eprintln!("{} {}", "Error:".bright_red().bold(), e);
        process::exit(1);
    }
}

#[cfg(test)]
mod tests {

    
    #[test]
    fn test_main_compiles() {
        // This test just ensures the main function compiles correctly
        // In a real implementation, we'd have more comprehensive tests
        assert!(true);
    }
}