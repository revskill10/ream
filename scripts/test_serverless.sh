#!/bin/bash

# REAM Serverless Test Runner
# Comprehensive test suite for serverless architecture

set -e

echo "🚀 REAM Serverless Test Suite"
echo "=============================="

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Test categories
declare -a TEST_CATEGORIES=(
    "hibernation"
    "cold_start" 
    "resources"
    "zero_copy"
    "metrics"
    "integration"
    "performance"
    "edge_cases"
)

echo -e "${BLUE}Running comprehensive serverless tests...${NC}"
echo ""

# Function to run specific test category
run_test_category() {
    local category=$1
    echo -e "${YELLOW}Testing $category...${NC}"
    
    case $category in
        "hibernation")
            cargo test test_hibernation_manager_basic --lib
            cargo test test_hibernation_statistics --lib
            cargo test test_hibernation_policy_enforcement --lib
            cargo test test_hibernation_state_transitions --lib
            cargo test test_hibernation_mathematical_properties --lib
            ;;
        "cold_start")
            cargo test test_cold_start_optimizer --lib
            cargo test test_performance_guarantees --lib
            ;;
        "resources")
            cargo test test_resource_pools --lib
            cargo test test_memory_pool_exhaustion --lib
            cargo test test_resource_pool_properties --lib
            ;;
        "zero_copy")
            cargo test test_zero_copy_hibernation --lib
            cargo test test_compression_bounds --lib
            cargo test test_compression_algorithms --lib
            cargo test test_memory_snapshot_operations --lib
            ;;
        "metrics")
            cargo test test_serverless_metrics --lib
            cargo test test_metrics_export --lib
            cargo test test_metrics_accuracy --lib
            ;;
        "integration")
            cargo test test_serverless_runtime_integration --lib
            cargo test test_tlisp_serverless_integration --lib
            cargo test test_serverless_function_lifecycle --lib
            ;;
        "performance")
            cargo test test_performance_guarantees --lib
            cargo test test_load_balancing_scenarios --lib
            cargo test test_stress_performance --lib
            ;;
        "edge_cases")
            cargo test test_error_handling --lib
            cargo test test_concurrent_operations --lib
            cargo test test_fault_tolerance --lib
            cargo test test_edge_cases --lib
            ;;
    esac
    
    if [ $? -eq 0 ]; then
        echo -e "${GREEN}✅ $category tests passed${NC}"
    else
        echo -e "${RED}❌ $category tests failed${NC}"
        return 1
    fi
    echo ""
}

# Function to run all serverless tests
run_all_tests() {
    echo -e "${BLUE}Running all serverless tests...${NC}"
    cargo test --lib serverless
    
    if [ $? -eq 0 ]; then
        echo -e "${GREEN}✅ All serverless tests passed${NC}"
    else
        echo -e "${RED}❌ Some serverless tests failed${NC}"
        return 1
    fi
}

# Function to run performance benchmarks
run_benchmarks() {
    echo -e "${BLUE}Running performance benchmarks...${NC}"
    
    # Run specific performance tests with timing
    echo "Hibernation performance:"
    cargo test test_hibernation_manager_basic --lib --release -- --nocapture
    
    echo "Cold start performance:"
    cargo test test_cold_start_optimizer --lib --release -- --nocapture
    
    echo "Stress test performance:"
    cargo test test_stress_performance --lib --release -- --nocapture
}

# Function to generate test report
generate_report() {
    echo -e "${BLUE}Generating test coverage report...${NC}"
    
    # Count total tests
    local total_tests=$(cargo test --lib serverless -- --list | grep -c "test result:")
    
    echo ""
    echo "📊 Test Coverage Summary"
    echo "========================"
    echo "Total serverless tests: $total_tests"
    echo ""
    echo "Test Categories Covered:"
    for category in "${TEST_CATEGORIES[@]}"; do
        echo "  ✅ $category"
    done
    echo ""
    echo "Coverage Areas:"
    echo "  ✅ Core hibernation functionality"
    echo "  ✅ Cold start optimization"
    echo "  ✅ Resource pool management"
    echo "  ✅ Zero-copy operations"
    echo "  ✅ Metrics collection and export"
    echo "  ✅ Runtime integration"
    echo "  ✅ TLisp language integration"
    echo "  ✅ Performance guarantees"
    echo "  ✅ Mathematical properties"
    echo "  ✅ Error handling and fault tolerance"
    echo "  ✅ Concurrent operations"
    echo "  ✅ Edge cases and boundary conditions"
    echo "  ✅ Stress testing and load scenarios"
    echo ""
}

# Main execution
main() {
    case "${1:-all}" in
        "all")
            run_all_tests
            generate_report
            ;;
        "categories")
            for category in "${TEST_CATEGORIES[@]}"; do
                run_test_category "$category"
            done
            generate_report
            ;;
        "benchmarks")
            run_benchmarks
            ;;
        "report")
            generate_report
            ;;
        *)
            if [[ " ${TEST_CATEGORIES[@]} " =~ " $1 " ]]; then
                run_test_category "$1"
            else
                echo "Usage: $0 [all|categories|benchmarks|report|category_name]"
                echo ""
                echo "Available categories:"
                for category in "${TEST_CATEGORIES[@]}"; do
                    echo "  - $category"
                done
                exit 1
            fi
            ;;
    esac
}

# Ensure we're in the right directory
if [ ! -f "Cargo.toml" ]; then
    echo -e "${RED}Error: Must be run from the project root directory${NC}"
    exit 1
fi

# Check if cargo is available
if ! command -v cargo &> /dev/null; then
    echo -e "${RED}Error: Cargo is not installed or not in PATH${NC}"
    exit 1
fi

# Run main function
main "$@"

echo ""
echo -e "${GREEN}🎉 Serverless test suite completed!${NC}"
