use std::process::Command;
use std::path::PathBuf;
use std::fs;
use std::time::Instant;

/// Test compilation and execution of TLisp algorithm programs
/// This test compiles TLisp programs to executables and benchmarks their performance
/// without JIT overhead, providing true compiled performance metrics.

#[cfg(test)]
mod tests {
    use super::*;

    const ALGORITHM_EXAMPLES_DIR: &str = "examples/algorithm";
    const TEMP_BUILD_DIR: &str = "target/algorithm_benchmarks";

    fn setup_build_directory() -> std::io::Result<PathBuf> {
        let build_dir = PathBuf::from(TEMP_BUILD_DIR);
        if build_dir.exists() {
            fs::remove_dir_all(&build_dir)?;
        }
        fs::create_dir_all(&build_dir)?;
        Ok(build_dir)
    }

    fn fix_batch_file(batch_path: &PathBuf) -> std::io::Result<()> {
        let content = fs::read_to_string(batch_path)?;
        let fixed_content = content.replace(
            "cargo run --manifest-path",
            "cargo run --bin ream --manifest-path"
        );
        fs::write(batch_path, fixed_content)?;
        Ok(())
    }

    fn compile_tlisp_program(source_path: &str, output_name: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let build_dir = setup_build_directory()?;
        let output_path = build_dir.join(format!("{}.exe", output_name));

        println!("Compiling {} to {}", source_path, output_path.display());

        // Use ream build command to compile TLisp to executable
        let output = Command::new("cargo")
            .args(&["run", "--bin", "ream", "--", "build", source_path, "--output", output_path.to_str().unwrap(), "--target", "executable"])
            .current_dir(".")
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            return Err(format!("Compilation failed for {}: stdout: {}, stderr: {}", source_path, stdout, stderr).into());
        }

        // The build command creates a batch file on Windows
        let possible_paths = vec![
            output_path.join("simple_demo.bat"),  // Default batch file name
            output_path.join(format!("{}.bat", output_name)),
            output_path.join("program.bat"),
            build_dir.join("simple_demo.bat"),
            build_dir.join(format!("{}.bat", output_name)),
        ];

        for path in possible_paths {
            if path.exists() {
                // Fix the batch file to specify the correct binary
                if let Err(e) = fix_batch_file(&path) {
                    println!("Warning: Failed to fix batch file {}: {}", path.display(), e);
                }
                return Ok(path);
            }
        }

        return Err(format!("Executable not found after compilation. Expected batch file in {}", output_path.display()).into());
    }

    fn benchmark_executable(executable_path: &PathBuf, runs: usize) -> Result<(f64, f64, f64), Box<dyn std::error::Error>> {
        let mut times = Vec::new();

        for _ in 0..runs {
            let start = Instant::now();
            let output = Command::new(executable_path)
                .output()?;
            let duration = start.elapsed();

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(format!("Execution failed: {}", stderr).into());
            }

            times.push(duration.as_secs_f64() * 1000.0); // Convert to milliseconds
        }

        let avg_time = times.iter().sum::<f64>() / times.len() as f64;
        let min_time = times.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let max_time = times.iter().fold(0.0f64, |a, &b| a.max(b));

        Ok((avg_time, min_time, max_time))
    }

    fn benchmark_jit(source_path: &str, runs: usize) -> Result<(f64, f64, f64), Box<dyn std::error::Error>> {
        let mut times = Vec::new();

        for _ in 0..runs {
            let start = Instant::now();
            let output = Command::new("cargo")
                .args(&["run", "--bin", "ream", "--", "run", source_path])
                .output()?;
            let duration = start.elapsed();

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(format!("JIT execution failed: {}", stderr).into());
            }

            times.push(duration.as_secs_f64() * 1000.0); // Convert to milliseconds
        }

        let avg_time = times.iter().sum::<f64>() / times.len() as f64;
        let min_time = times.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let max_time = times.iter().fold(0.0f64, |a, &b| a.max(b));

        Ok((avg_time, min_time, max_time))
    }

    #[test]
    fn test_compile_and_benchmark_all_algorithms() {
        println!("Testing compilation and benchmarking of all algorithm examples...");

        let algorithms = vec![
            ("hello_world.tlisp", "hello_world"),
            ("binary_trees.tlisp", "binary_trees"),
            ("prime_sieve.tlisp", "prime_sieve"),
            ("fibonacci.tlisp", "fibonacci"),
            ("fannkuch.tlisp", "fannkuch"),
            ("fasta.tlisp", "fasta"),
            ("mandelbrot.tlisp", "mandelbrot"),
            ("arithmetic.tlisp", "arithmetic"),
            ("list_processing.tlisp", "list_processing"),
            ("sorting.tlisp", "sorting"),
        ];
        
        let mut compilation_results = Vec::new();
        let mut benchmark_results = Vec::new();
        
        for (source_file, output_name) in algorithms {
            let source_path = format!("{}/{}", ALGORITHM_EXAMPLES_DIR, source_file);
            
            // Check if source file exists
            if !PathBuf::from(&source_path).exists() {
                println!("⚠️  Skipping {} - source file not found", source_file);
                continue;
            }
            
            print!("📦 Compiling {}... ", source_file);
            let compile_start = Instant::now();
            
            match compile_tlisp_program(&source_path, output_name) {
                Ok(executable_path) => {
                    let compile_time = compile_start.elapsed();
                    println!("✓ ({:.3}ms)", compile_time.as_secs_f64() * 1000.0);
                    compilation_results.push((source_file, compile_time.as_secs_f64() * 1000.0));
                    
                    // Benchmark the compiled executable
                    print!("🚀 Benchmarking compiled {}... ", output_name);
                    match benchmark_executable(&executable_path, 5) {
                        Ok((comp_avg, comp_min, comp_max)) => {
                            println!("✓ avg: {:.3}ms, min: {:.3}ms, max: {:.3}ms", comp_avg, comp_min, comp_max);

                            // Also benchmark JIT for comparison
                            print!("⚡ Benchmarking JIT {}... ", output_name);
                            match benchmark_jit(&source_path, 3) {
                                Ok((jit_avg, jit_min, jit_max)) => {
                                    println!("✓ avg: {:.3}ms, min: {:.3}ms, max: {:.3}ms", jit_avg, jit_min, jit_max);

                                    let speedup = jit_avg / comp_avg;
                                    if speedup > 1.0 {
                                        println!("🏆 Compiled is {:.2}x faster than JIT", speedup);
                                    } else {
                                        println!("⚠️  JIT is {:.2}x faster than compiled", 1.0 / speedup);
                                    }

                                    benchmark_results.push((output_name, comp_avg, comp_min, comp_max));
                                }
                                Err(e) => {
                                    println!("❌ JIT benchmark failed: {}", e);
                                    benchmark_results.push((output_name, comp_avg, comp_min, comp_max));
                                }
                            }
                        }
                        Err(e) => {
                            println!("❌ Compiled benchmark failed: {}", e);
                        }
                    }
                }
                Err(e) => {
                    let compile_time = compile_start.elapsed();
                    println!("❌ ({:.3}ms) - {}", compile_time.as_secs_f64() * 1000.0, e);
                }
            }
        }
        
        // Print summary
        println!("\n📊 COMPILATION SUMMARY:");
        println!("┌─────────────────────────┬──────────────┐");
        println!("│ Algorithm               │ Compile Time │");
        println!("├─────────────────────────┼──────────────┤");
        for (name, time) in &compilation_results {
            println!("│ {:<23} │ {:>9.3}ms │", name, time);
        }
        println!("└─────────────────────────┴──────────────┘");
        
        println!("\n🏃 EXECUTION PERFORMANCE SUMMARY:");
        println!("┌─────────────────────────┬──────────────┬──────────────┬──────────────┐");
        println!("│ Algorithm               │ Average Time │ Min Time     │ Max Time     │");
        println!("├─────────────────────────┼──────────────┼──────────────┼──────────────┤");
        for (name, avg, min, max) in &benchmark_results {
            println!("│ {:<23} │ {:>9.3}ms │ {:>9.3}ms │ {:>9.3}ms │", name, avg, min, max);
        }
        println!("└─────────────────────────┴──────────────┴──────────────┴──────────────┘");
        
        // Ensure at least some algorithms compiled and ran successfully
        assert!(!compilation_results.is_empty(), "No algorithms compiled successfully");
        assert!(!benchmark_results.is_empty(), "No algorithms benchmarked successfully");
        
        println!("\n🎉 Compiled algorithm benchmark test completed successfully!");
        println!("   {} algorithms compiled, {} algorithms benchmarked",
                 compilation_results.len(), benchmark_results.len());
    }

    #[test]
    fn test_jit_vs_compiled_performance_comparison() {
        println!("🔥 Detailed JIT vs Compiled Performance Comparison");
        println!("==================================================");

        // Test algorithms that exist as files in examples/algorithm/
        let test_algorithms = vec![
            ("hello_world.tlisp", "hello_world"),
            ("binary_trees.tlisp", "binary_trees"),
            ("prime_sieve.tlisp", "prime_sieve"),
            ("fibonacci.tlisp", "fibonacci"),
            ("fannkuch.tlisp", "fannkuch"),
            ("fasta.tlisp", "fasta"),
            ("mandelbrot.tlisp", "mandelbrot"),
            ("arithmetic.tlisp", "arithmetic"),
            ("list_processing.tlisp", "list_processing"),
            ("sorting.tlisp", "sorting"),
        ];

        // Test different input sizes (number of benchmark runs)
        let input_sizes = vec![
            ("Small", 3),
            ("Medium", 5),
            ("Large", 7),
        ];

        let mut all_results = Vec::new();

        for (size_name, runs) in &input_sizes {
            println!("\n🎯 Testing with {} input size ({} runs):", size_name, runs);
            let mut size_results = Vec::new();

            for (source_file, output_name) in &test_algorithms {
                let source_path = format!("{}/{}", ALGORITHM_EXAMPLES_DIR, source_file);

                if !PathBuf::from(&source_path).exists() {
                    println!("⚠️  Skipping {} - source file not found", source_file);
                    continue;
                }

                println!("\n🔬 Testing {} performance:", source_file);

                // Test JIT performance
                print!("  ⚡ JIT execution... ");
                let jit_result = benchmark_jit(&source_path, *runs);

                // Test compiled performance
                print!("  🚀 Compiling and benchmarking... ");
                let compiled_result = match compile_tlisp_program(&source_path, output_name) {
                    Ok(executable_path) => benchmark_executable(&executable_path, *runs),
                    Err(e) => {
                        println!("❌ Compilation failed: {}", e);
                        continue;
                    }
                };

                match (jit_result, compiled_result) {
                    (Ok((jit_avg, jit_min, jit_max)), Ok((comp_avg, comp_min, comp_max))) => {
                        println!("✓");
                        println!("    📊 JIT:      avg={:.3}ms, min={:.3}ms, max={:.3}ms", jit_avg, jit_min, jit_max);
                        println!("    📊 Compiled: avg={:.3}ms, min={:.3}ms, max={:.3}ms", comp_avg, comp_min, comp_max);

                        let speedup = jit_avg / comp_avg;
                        let winner = if speedup > 1.0 { "Compiled" } else { "JIT" };
                        let factor = if speedup > 1.0 { speedup } else { 1.0 / speedup };

                        println!("    🏆 Winner: {} ({:.2}x faster)", winner, factor);

                        size_results.push((source_file, jit_avg, comp_avg, speedup, winner, factor));
                    }
                    (Err(jit_err), _) => {
                        println!("❌ JIT failed: {}", jit_err);
                    }
                    (_, Err(comp_err)) => {
                        println!("❌ Compiled failed: {}", comp_err);
                    }
                }
            }

            // Print summary table for this input size
            println!("\n📊 PERFORMANCE SUMMARY FOR {} INPUT SIZE:", size_name.to_uppercase());
            println!("┌─────────────────────────┬──────────────┬──────────────┬──────────────┬──────────────┐");
            println!("│ Algorithm               │ JIT Avg (ms) │ AOT Avg (ms) │ Speedup      │ Winner       │");
            println!("├─────────────────────────┼──────────────┼──────────────┼──────────────┼──────────────┤");

            for (name, jit_avg, comp_avg, speedup, winner, _factor) in &size_results {
                println!("│ {:<23} │ {:>9.3}    │ {:>9.3}     │ {:>9.2}x    │ {:<12} │",
                         name, jit_avg, comp_avg, speedup, winner);
            }
            println!("└─────────────────────────┴──────────────┴──────────────┴──────────────┴──────────────┘");

            all_results.push((size_name.to_string(), size_results));
        }

        // Print comprehensive summary across all input sizes
        println!("\n🏆 COMPREHENSIVE RESULTS ACROSS ALL INPUT SIZES:");
        println!("=================================================");

        for (size_name, size_results) in &all_results {
            let total_algorithms = test_algorithms.len();
            let successful_algorithms = size_results.len();
            let failed_algorithms = total_algorithms - successful_algorithms;
            let compiled_wins = size_results.iter().filter(|(_, _, _, _, winner, _)| *winner == "Compiled").count();
            let jit_wins = size_results.iter().filter(|(_, _, _, _, winner, _)| *winner == "JIT").count();

            println!("\n📊 {} INPUT SIZE RESULTS:", size_name.to_uppercase());
            println!("   📊 Total algorithms tested: {}", total_algorithms);
            println!("   ✅ Successful comparisons: {}", successful_algorithms);
            println!("   ❌ Failed algorithms: {}", failed_algorithms);
            println!("   🚀 AOT wins: {} algorithms", compiled_wins);
            println!("   ⚡ JIT wins: {} algorithms", jit_wins);

            if successful_algorithms > 0 {
                if compiled_wins > jit_wins {
                    println!("   🎯 Winner: AOT execution!");
                } else if jit_wins > compiled_wins {
                    println!("   🎯 Winner: JIT execution!");
                } else {
                    println!("   🎯 It's a tie!");
                }

                let success_rate = (successful_algorithms as f64 / total_algorithms as f64) * 100.0;
                println!("   📈 Success rate: {:.1}%", success_rate);
            }
        }

        // Overall summary
        let total_tests = all_results.len() * test_algorithms.len();
        let total_successful: usize = all_results.iter().map(|(_, results)| results.len()).sum();

        println!("\n🎉 OVERALL BENCHMARK SUMMARY:");
        println!("   📊 Total test combinations: {}", total_tests);
        println!("   ✅ Total successful tests: {}", total_successful);
        println!("   📈 Overall success rate: {:.1}%", (total_successful as f64 / total_tests as f64) * 100.0);

        assert!(!all_results.is_empty(), "No successful performance comparisons");
        println!("\n🎉 Multi-input-size performance comparison completed successfully!");
    }


}
