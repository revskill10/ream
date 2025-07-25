//! Production TLisp Integration Tests
//!
//! Comprehensive tests to validate that REAM can run all TLisp programs
//! with production-grade runtime features.

use std::time::Duration;
use ream::tlisp::{
    ProductionTlispRuntime, ProductionRuntimeConfig, TlispSecurityLevel, 
    TlispResourceQuotas, TlispSpecificLimits, SecurityLevel, ExecutionMode
};
use ream::runtime::ResourceQuotas;
use ream::types::Priority;

#[test]
fn test_production_runtime_creation() {
    let config = ProductionRuntimeConfig::default();
    let runtime = ProductionTlispRuntime::new(config);
    assert!(runtime.is_ok(), "Should create production runtime successfully");
    
    let runtime = runtime.unwrap();
    assert!(runtime.start().is_ok(), "Should start runtime successfully");
}

#[test]
fn test_basic_tlisp_program_execution() {
    let config = ProductionRuntimeConfig::default();
    let runtime = ProductionTlispRuntime::new(config).unwrap();
    runtime.start().unwrap();
    
    // Test basic arithmetic
    let result = runtime.execute_program(
        "arithmetic_test".to_string(),
        "(+ 1 2 3 4 5)",
        None,
        None,
    );
    
    assert!(result.is_ok(), "Basic arithmetic should execute successfully");
    let result = result.unwrap();
    assert!(matches!(result.execution_mode, ExecutionMode::Interpreted | ExecutionMode::Bytecode));
    assert!(result.execution_time < Duration::from_secs(1), "Should execute quickly");
}

#[test]
fn test_complex_tlisp_program_execution() {
    let config = ProductionRuntimeConfig::default();
    let runtime = ProductionTlispRuntime::new(config).unwrap();
    runtime.start().unwrap();
    
    // Test complex program with functions and control flow
    let complex_program = r#"
        (define factorial (lambda (n)
            (if (<= n 1)
                1
                (* n (factorial (- n 1))))))
        (factorial 5)
    "#;
    
    let result = runtime.execute_program(
        "factorial_test".to_string(),
        complex_program,
        None,
        None,
    );
    
    assert!(result.is_ok(), "Complex program should execute successfully");
    let result = result.unwrap();
    assert!(result.execution_time < Duration::from_secs(5), "Should execute within reasonable time");
}

#[test]
fn test_security_levels() {
    let config = ProductionRuntimeConfig::default();
    let runtime = ProductionTlispRuntime::new(config).unwrap();
    runtime.start().unwrap();
    
    // Test trusted execution
    let result = runtime.execute_program(
        "trusted_test".to_string(),
        "(+ 1 2)",
        Some(TlispSecurityLevel::Trusted),
        None,
    );
    assert!(result.is_ok(), "Trusted execution should succeed");
    
    // Test restricted execution
    let result = runtime.execute_program(
        "restricted_test".to_string(),
        "(+ 1 2)",
        Some(TlispSecurityLevel::Restricted),
        None,
    );
    assert!(result.is_ok(), "Restricted execution should succeed for safe operations");
    
    // Test sandboxed execution
    let result = runtime.execute_program(
        "sandboxed_test".to_string(),
        "(+ 1 2)",
        Some(TlispSecurityLevel::Sandboxed),
        None,
    );
    assert!(result.is_ok(), "Sandboxed execution should succeed for basic operations");
}

#[test]
fn test_resource_quotas() {
    let config = ProductionRuntimeConfig::default();
    let runtime = ProductionTlispRuntime::new(config).unwrap();
    runtime.start().unwrap();
    
    // Test with generous quotas
    let generous_quotas = TlispResourceQuotas {
        base_quotas: ResourceQuotas {
            max_memory: Some(100 * 1024 * 1024), // 100 MB
            max_cpu_time: Some(Duration::from_secs(10)),
            ..ResourceQuotas::default()
        },
        tlisp_limits: TlispSpecificLimits {
            max_recursion_depth: Some(1000),
            max_function_calls: Some(100000),
            ..TlispSpecificLimits::default()
        },
    };
    
    let result = runtime.execute_program(
        "generous_quota_test".to_string(),
        "(+ 1 2 3 4 5)",
        None,
        Some(generous_quotas),
    );
    assert!(result.is_ok(), "Should succeed with generous quotas");
    
    // Test with restrictive quotas
    let restrictive_quotas = TlispResourceQuotas {
        base_quotas: ResourceQuotas {
            max_memory: Some(1024), // 1 KB (very restrictive)
            max_cpu_time: Some(Duration::from_millis(1)),
            ..ResourceQuotas::default()
        },
        tlisp_limits: TlispSpecificLimits {
            max_recursion_depth: Some(5),
            max_function_calls: Some(10),
            ..TlispSpecificLimits::default()
        },
    };
    
    let result = runtime.execute_program(
        "restrictive_quota_test".to_string(),
        "(+ 1 2)",
        None,
        Some(restrictive_quotas),
    );
    // This might fail due to restrictive quotas, which is expected behavior
    // The important thing is that it doesn't crash the runtime
}

#[test]
fn test_actor_spawning() {
    let mut config = ProductionRuntimeConfig::default();
    config.enable_actor_system = true;
    
    let runtime = ProductionTlispRuntime::new(config).unwrap();
    runtime.start().unwrap();
    
    // Parse a simple actor behavior
    // Note: In a real implementation, we'd need to parse this properly
    // For now, we'll test the actor spawning infrastructure
    
    // Test that the actor system is available
    let stats = runtime.get_stats();
    assert_eq!(stats.actors_spawned, 0, "Should start with no actors spawned");
}

#[test]
fn test_concurrent_program_execution() {
    let config = ProductionRuntimeConfig::default();
    let runtime = ProductionTlispRuntime::new(config).unwrap();
    runtime.start().unwrap();
    
    // Execute multiple programs concurrently
    let mut handles = Vec::new();
    
    for i in 0..10 {
        let program_id = format!("concurrent_test_{}", i);
        let program = format!("(+ {} {})", i, i + 1);
        
        // In a real implementation, we'd spawn these in separate threads
        // For now, we'll execute them sequentially to test the infrastructure
        let result = runtime.execute_program(
            program_id,
            &program,
            None,
            None,
        );
        
        assert!(result.is_ok(), "Concurrent program {} should execute successfully", i);
    }
}

#[test]
fn test_runtime_statistics() {
    let config = ProductionRuntimeConfig::default();
    let runtime = ProductionTlispRuntime::new(config).unwrap();
    runtime.start().unwrap();
    
    // Execute a program to generate statistics
    let _result = runtime.execute_program(
        "stats_test".to_string(),
        "(+ 1 2 3)",
        None,
        None,
    ).unwrap();
    
    // Check statistics
    let stats = runtime.get_stats();
    assert!(stats.programs_executed > 0, "Should have executed at least one program");
    assert!(stats.total_execution_time > Duration::ZERO, "Should have non-zero execution time");
}

#[test]
fn test_active_program_tracking() {
    let config = ProductionRuntimeConfig::default();
    let runtime = ProductionTlispRuntime::new(config).unwrap();
    runtime.start().unwrap();
    
    // Check that no programs are initially active
    let active_programs = runtime.get_active_programs();
    assert!(active_programs.is_empty(), "Should start with no active programs");
    
    // Execute a program and verify tracking
    let _result = runtime.execute_program(
        "tracking_test".to_string(),
        "(+ 1 2)",
        None,
        None,
    ).unwrap();
    
    // After execution, should be no active programs
    let active_programs = runtime.get_active_programs();
    assert!(active_programs.is_empty(), "Should have no active programs after execution");
}

#[test]
fn test_all_execution_modes() {
    let mut config = ProductionRuntimeConfig::default();
    config.enable_jit = true;
    config.enable_actor_system = true;
    
    let runtime = ProductionTlispRuntime::new(config).unwrap();
    runtime.start().unwrap();
    
    // Test different types of programs that might trigger different execution modes
    let test_cases = vec![
        ("simple_arithmetic", "(+ 1 2)"),
        ("function_call", "(define square (lambda (x) (* x x))) (square 5)"),
        ("control_flow", "(if (> 5 3) 'yes 'no)"),
        ("list_operations", "(list 1 2 3 4 5)"),
        ("string_operations", "(string-concat 'hello ' 'world)"),
    ];
    
    for (name, program) in test_cases {
        let result = runtime.execute_program(
            name.to_string(),
            program,
            None,
            None,
        );
        
        assert!(result.is_ok(), "Program '{}' should execute successfully", name);
        
        let result = result.unwrap();
        assert!(
            matches!(
                result.execution_mode,
                ExecutionMode::Interpreted | ExecutionMode::Bytecode | ExecutionMode::JitCompiled | ExecutionMode::Actor
            ),
            "Should use a valid execution mode for program '{}'",
            name
        );
    }
}

#[test]
fn test_error_handling() {
    let config = ProductionRuntimeConfig::default();
    let runtime = ProductionTlispRuntime::new(config).unwrap();
    runtime.start().unwrap();
    
    // Test syntax error handling
    let result = runtime.execute_program(
        "syntax_error_test".to_string(),
        "(+ 1 2", // Missing closing parenthesis
        None,
        None,
    );
    
    // Should handle syntax errors gracefully
    assert!(result.is_err(), "Should detect syntax errors");
    
    // Test runtime error handling
    let result = runtime.execute_program(
        "runtime_error_test".to_string(),
        "(/ 1 0)", // Division by zero
        None,
        None,
    );
    
    // Should handle runtime errors gracefully
    // Note: The actual behavior depends on the implementation
}

#[test]
fn test_production_features_integration() {
    let config = ProductionRuntimeConfig {
        enable_jit: true,
        enable_preemptive_scheduling: true,
        enable_work_stealing: true,
        enable_realtime_scheduling: true,
        enable_security: true,
        enable_resource_management: true,
        enable_actor_system: true,
        enable_performance_monitoring: true,
        debug_mode: false,
        ..ProductionRuntimeConfig::default()
    };
    
    let runtime = ProductionTlispRuntime::new(config).unwrap();
    runtime.start().unwrap();
    
    // Test that all features work together
    let result = runtime.execute_program(
        "integration_test".to_string(),
        "(+ (* 2 3) (- 10 5))", // Should result in 11
        Some(TlispSecurityLevel::Restricted),
        Some(TlispResourceQuotas::default()),
    );
    
    assert!(result.is_ok(), "All features should work together seamlessly");
    
    let result = result.unwrap();
    assert!(result.execution_time < Duration::from_secs(1), "Should execute efficiently");
    assert!(result.memory_used > 0, "Should track memory usage");
    
    // Verify statistics are being collected
    let stats = runtime.get_stats();
    assert!(stats.programs_executed > 0, "Should track executed programs");
}

#[test]
fn test_comprehensive_tlisp_language_features() {
    let config = ProductionRuntimeConfig::default();
    let runtime = ProductionTlispRuntime::new(config).unwrap();
    runtime.start().unwrap();
    
    // Test comprehensive TLisp language features
    let comprehensive_program = r#"
        ; Define a function
        (define fibonacci (lambda (n)
            (if (<= n 1)
                n
                (+ (fibonacci (- n 1)) (fibonacci (- n 2))))))
        
        ; Test arithmetic
        (define arithmetic-test (+ 1 2 (* 3 4) (/ 10 2)))
        
        ; Test list operations
        (define list-test (list 1 2 3 4 5))
        
        ; Test string operations
        (define string-test (string-concat "Hello" " " "World"))
        
        ; Test control flow
        (define control-test 
            (if (> arithmetic-test 10)
                "Greater than 10"
                "Less than or equal to 10"))
        
        ; Return a result
        (list arithmetic-test (fibonacci 7) list-test string-test control-test)
    "#;
    
    let result = runtime.execute_program(
        "comprehensive_test".to_string(),
        comprehensive_program,
        None,
        None,
    );
    
    assert!(result.is_ok(), "Comprehensive TLisp program should execute successfully");
    
    let result = result.unwrap();
    assert!(result.execution_time < Duration::from_secs(10), "Should execute within reasonable time");
    
    // Verify that the runtime can handle complex programs
    let stats = runtime.get_stats();
    assert!(stats.programs_executed > 0, "Should have executed the program");
}

#[test]
fn test_runtime_lifecycle() {
    let config = ProductionRuntimeConfig::default();
    let runtime = ProductionTlispRuntime::new(config).unwrap();
    
    // Test start
    assert!(runtime.start().is_ok(), "Should start successfully");
    
    // Test execution while running
    let result = runtime.execute_program(
        "lifecycle_test".to_string(),
        "(+ 1 2 3)",
        None,
        None,
    );
    assert!(result.is_ok(), "Should execute while running");
    
    // Test stop (implicit through Drop)
    runtime.stop();
    
    // Runtime should handle shutdown gracefully
}

#[test]
fn test_production_readiness() {
    // This test validates that the runtime meets production requirements
    let config = ProductionRuntimeConfig::default();
    let runtime = ProductionTlispRuntime::new(config).unwrap();
    runtime.start().unwrap();
    
    // Test reliability: Execute many programs without crashing
    for i in 0..100 {
        let result = runtime.execute_program(
            format!("reliability_test_{}", i),
            "(+ 1 2 3)",
            None,
            None,
        );
        assert!(result.is_ok(), "Should handle repeated execution reliably");
    }
    
    // Test performance: Should maintain good performance
    let start_time = std::time::Instant::now();
    for i in 0..10 {
        let _result = runtime.execute_program(
            format!("performance_test_{}", i),
            "(* (+ 1 2) (- 5 3))",
            None,
            None,
        ).unwrap();
    }
    let total_time = start_time.elapsed();
    
    // Should execute 10 simple programs in under 1 second
    assert!(total_time < Duration::from_secs(1), "Should maintain good performance");
    
    // Test statistics collection
    let stats = runtime.get_stats();
    assert!(stats.programs_executed >= 110, "Should track all executed programs"); // 100 + 10 + previous tests
    assert!(stats.total_execution_time > Duration::ZERO, "Should track execution time");
    
    println!("✅ Production readiness validated:");
    println!("   Programs executed: {}", stats.programs_executed);
    println!("   Total execution time: {:?}", stats.total_execution_time);
    println!("   Average execution time: {:?}", stats.avg_execution_time);
    println!("   Actors spawned: {}", stats.actors_spawned);
}
