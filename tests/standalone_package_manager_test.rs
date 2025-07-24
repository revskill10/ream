//! Tests for the standalone REAM package manager binary
//!
//! This test suite verifies that the standalone package manager binary
//! works correctly and provides all expected functionality.

use std::process::Command;
use std::path::PathBuf;

/// Test that the package manager binary can be executed and shows help
#[test]
fn test_package_manager_help() {
    let output = Command::new("cargo")
        .args(&["run", "--bin", "ream-pkg", "--", "--help"])
        .output()
        .expect("Failed to execute package manager");

    assert!(output.status.success(), "Package manager should exit successfully");
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("REAM Package Manager"), "Should show package manager title");
    assert!(stdout.contains("install"), "Should show install command");
    assert!(stdout.contains("remove"), "Should show remove command");
    assert!(stdout.contains("list"), "Should show list command");
    assert!(stdout.contains("search"), "Should show search command");
    assert!(stdout.contains("registry"), "Should show registry command");
}

/// Test that the package manager can list packages (even if empty)
#[test]
fn test_package_manager_list() {
    let output = Command::new("cargo")
        .args(&["run", "--bin", "ream-pkg", "--", "list"])
        .output()
        .expect("Failed to execute package manager list");

    assert!(output.status.success(), "List command should exit successfully");
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should show either packages or "No packages installed"
    assert!(
        stdout.contains("No packages installed") || stdout.contains("package"),
        "Should show package list status"
    );
}

/// Test that the package manager can handle install command
#[test]
fn test_package_manager_install() {
    let output = Command::new("cargo")
        .args(&["run", "--bin", "ream-pkg", "--", "install", "test-package", "--verbose"])
        .output()
        .expect("Failed to execute package manager install");

    assert!(output.status.success(), "Install command should exit successfully");
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Installing packages"), "Should show install message");
    assert!(stdout.contains("test-package"), "Should mention the package name");
}

/// Test that the package manager can handle registry commands
#[test]
fn test_package_manager_registry() {
    let output = Command::new("cargo")
        .args(&["run", "--bin", "ream-pkg", "--", "registry", "list"])
        .output()
        .expect("Failed to execute package manager registry");

    assert!(output.status.success(), "Registry command should exit successfully");
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Registry command executed"), "Should execute registry command");
}

/// Test that the package manager can handle search commands
#[test]
fn test_package_manager_search() {
    let output = Command::new("cargo")
        .args(&["run", "--bin", "ream-pkg", "--", "search", "test", "--verbose"])
        .output()
        .expect("Failed to execute package manager search");

    assert!(output.status.success(), "Search command should exit successfully");
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Searching for"), "Should show search message");
    assert!(stdout.contains("test"), "Should mention the search query");
}

/// Test that the package manager can handle config commands
#[test]
fn test_package_manager_config() {
    let output = Command::new("cargo")
        .args(&["run", "--bin", "ream-pkg", "--", "config"])
        .output()
        .expect("Failed to execute package manager config");

    assert!(output.status.success(), "Config command should exit successfully");
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Configuration command executed"), "Should execute config command");
}

/// Test that the package manager can handle clean commands
#[test]
fn test_package_manager_clean() {
    let output = Command::new("cargo")
        .args(&["run", "--bin", "ream-pkg", "--", "clean"])
        .output()
        .expect("Failed to execute package manager clean");

    assert!(output.status.success(), "Clean command should exit successfully");
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Cache cleaned"), "Should show clean message");
}

/// Test that the package manager can handle info commands
#[test]
fn test_package_manager_info() {
    let output = Command::new("cargo")
        .args(&["run", "--bin", "ream-pkg", "--", "info", "test-package", "--verbose"])
        .output()
        .expect("Failed to execute package manager info");

    assert!(output.status.success(), "Info command should exit successfully");
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Getting info for"), "Should show info message");
    assert!(stdout.contains("test-package"), "Should mention the package name");
}

/// Test that the package manager handles invalid commands gracefully
#[test]
fn test_package_manager_invalid_command() {
    let output = Command::new("cargo")
        .args(&["run", "--bin", "ream-pkg", "--", "invalid-command"])
        .output()
        .expect("Failed to execute package manager with invalid command");

    assert!(!output.status.success(), "Invalid command should fail");
    
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("error") || stderr.contains("unrecognized"), "Should show error for invalid command");
}

/// Test that the package manager shows version information
#[test]
fn test_package_manager_version() {
    let output = Command::new("cargo")
        .args(&["run", "--bin", "ream-pkg", "--", "--version"])
        .output()
        .expect("Failed to execute package manager version");

    assert!(output.status.success(), "Version command should exit successfully");
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("0.1.0"), "Should show version number");
}

/// Integration test: Test that package manager can be built as a standalone binary
#[test]
fn test_package_manager_build() {
    let output = Command::new("cargo")
        .args(&["build", "--bin", "ream-pkg"])
        .output()
        .expect("Failed to build package manager");

    assert!(output.status.success(), "Package manager should build successfully");
    
    // Check that the binary exists
    let binary_path = PathBuf::from("target/debug/ream-pkg.exe");
    assert!(binary_path.exists() || PathBuf::from("target/debug/ream-pkg").exists(), 
            "Package manager binary should exist after build");
}
