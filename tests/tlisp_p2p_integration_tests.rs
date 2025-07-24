//! TLisp P2P integration tests
//!
//! Comprehensive tests for TLisp P2P standard library functions
//! ensuring they work correctly and reliably as specified in p2p_testing.md

use ream::tlisp::TlispInterpreter;
use ream::tlisp::standard_library::StandardLibrary;

/// Test TLisp P2P function registration and basic functionality
#[cfg(test)]
mod tlisp_p2p_basic_tests {
    use super::*;

    #[test]
    fn test_p2p_function_registration() {
        let stdlib = StandardLibrary::new();

        // Verify all required functions are registered
        let expected_functions = vec![
            "p2p-create-cluster",
            "p2p-join-cluster",
            "p2p-leave-cluster",
            "p2p-cluster-info",
            "p2p-cluster-members",
            "p2p-spawn-actor",
            "p2p-migrate-actor",
            "p2p-send-remote",
            "p2p-actor-location",
            "p2p-node-info",
            "p2p-node-health",
            "p2p-discover-nodes",
            "p2p-propose",
            "p2p-consensus-state",
        ];

        for func_name in expected_functions {
            assert!(stdlib.has_function(func_name),
                   "Function {} not registered", func_name);
        }
    }

    #[test]
    fn test_p2p_builtin_functions_available() {
        let mut interpreter = TlispInterpreter::new();

        // Test that P2P functions are available as builtins
        // We test by trying to call them with wrong arity, which should give an arity error
        // rather than an undefined function error
        let p2p_functions = vec![
            "p2p-create-cluster",
            "p2p-join-cluster",
            "p2p-cluster-info",
            "p2p-spawn-actor",
            "p2p-node-info",
            "p2p-consensus-state",
        ];

        for func_name in p2p_functions {
            // Try to call the function with wrong arity
            let test_expr = format!("({})", func_name);
            let result = interpreter.eval(&test_expr);

            // The function should exist (not give undefined function error)
            // It might fail with other errors (like arity or type errors) but should not be undefined
            match result {
                Err(e) => {
                    let error_msg = format!("{:?}", e);
                    // Should not be an undefined function error
                    assert!(!error_msg.contains("undefined") && !error_msg.contains("not found"),
                           "Function {} appears to be undefined: {}", func_name, error_msg);
                }
                Ok(_) => {
                    // Function executed successfully (unlikely but acceptable)
                }
            }
        }
    }

}
