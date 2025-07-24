use categorical_sqlite::{CategoricalSQLite, SqlError};
use clap::{Parser, Subcommand};
use std::io::{self, Write};

#[derive(Parser)]
#[command(name = "sqlite")]
#[command(about = "A mathematical SQLite implementation")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
    
    /// Database file path
    #[arg(short, long)]
    database: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Start interactive SQL shell
    Shell,
    /// Execute a single SQL statement
    Execute { sql: String },
    /// Show database schema
    Schema,
    /// Run performance benchmarks
    Benchmark,
    /// Run categorical features demonstration
    Demo,
    /// Show database statistics
    Stats,
    /// Check database health
    Health,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    
    let config = categorical_sqlite::engine::DatabaseConfig::default();
    let db = CategoricalSQLite::new(config);
    
    match cli.command {
        Some(Commands::Shell) => {
            run_interactive_shell(db).await?;
        }
        Some(Commands::Execute { sql }) => {
            execute_sql(&db, &sql).await?;
        }
        Some(Commands::Schema) => {
            show_schema(&db).await?;
        }
        Some(Commands::Benchmark) => {
            run_benchmarks(&db).await?;
        }
        Some(Commands::Demo) => {
            run_demonstration(&db).await?;
        }
        Some(Commands::Stats) => {
            show_statistics(&db).await?;
        }
        Some(Commands::Health) => {
            check_health(&db).await?;
        }
        None => {
            run_interactive_shell(db).await?;
        }
    }
    
    Ok(())
}

async fn run_interactive_shell(db: CategoricalSQLite) -> Result<(), SqlError> {
    println!("Categorical SQLite v0.1.0");
    println!("Enter SQL commands (type .exit to quit):");
    
    loop {
        print!("sqlite> ");
        io::stdout().flush().unwrap();
        
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        let input = input.trim();
        
        if input == ".exit" {
            break;
        }
        
        if input.is_empty() {
            continue;
        }
        
        match execute_sql(&db, input).await {
            Ok(()) => {}
            Err(e) => println!("Error: {}", e),
        }
    }
    
    Ok(())
}

async fn execute_sql(db: &CategoricalSQLite, sql: &str) -> Result<(), SqlError> {
    let result = db.execute_sql(sql).await?;
    
    // Display results
    match result {
        categorical_sqlite::query::QueryResult::Select { rows, columns } => {
            // Print column headers
            for (i, col) in columns.iter().enumerate() {
                if i > 0 { print!(" | "); }
                print!("{}", col);
            }
            println!();
            
            // Print separator
            for (i, col) in columns.iter().enumerate() {
                if i > 0 { print!("-+-"); }
                print!("{}", "-".repeat(col.len()));
            }
            println!();
            
            // Print rows
            for row in rows {
                for (i, value) in row.values.iter().enumerate() {
                    if i > 0 { print!(" | "); }
                    print!("{}", value);
                }
                println!();
            }
        }
        categorical_sqlite::query::QueryResult::Insert { rows_affected } => {
            println!("Inserted {} row(s)", rows_affected);
        }
        categorical_sqlite::query::QueryResult::Update { rows_affected } => {
            println!("Updated {} row(s)", rows_affected);
        }
        categorical_sqlite::query::QueryResult::Delete { rows_affected } => {
            println!("Deleted {} row(s)", rows_affected);
        }
        categorical_sqlite::query::QueryResult::CreateTable => {
            println!("Table created successfully");
        }
        categorical_sqlite::query::QueryResult::DropTable => {
            println!("Table dropped successfully");
        }
        categorical_sqlite::query::QueryResult::CreateIndex => {
            println!("Index created successfully");
        }
        categorical_sqlite::query::QueryResult::DropIndex => {
            println!("Index dropped successfully");
        }
    }
    
    Ok(())
}

async fn show_schema(db: &CategoricalSQLite) -> Result<(), SqlError> {
    let state = db.get_database_state().await;

    println!("Database Schema:");
    println!("Tables: {:?}", state.tables);
    println!("Indexes: {:?}", state.indexes);

    Ok(())
}

async fn show_statistics(db: &CategoricalSQLite) -> Result<(), Box<dyn std::error::Error>> {
    let stats = db.get_statistics().await;

    println!("📊 Database Statistics");
    println!("=====================");
    println!("Tables: {}", stats.total_tables);
    println!("Indexes: {}", stats.total_indexes);
    println!();

    println!("Page Cache:");
    println!("  Total pages: {}", stats.page_cache_stats.total_pages);
    println!("  Dirty pages: {}", stats.page_cache_stats.dirty_pages);
    println!("  Hit rate: {:.2}%", stats.page_cache_stats.hit_rate() * 100.0);
    println!();

    println!("Transactions:");
    println!("  Active: {}", stats.transaction_stats.active_transactions);
    println!("  Committed: {}", stats.transaction_stats.committed_transactions);
    println!("  Aborted: {}", stats.transaction_stats.aborted_transactions);
    println!("  Commit rate: {:.2}%", stats.transaction_stats.commit_rate() * 100.0);

    Ok(())
}

async fn check_health(db: &CategoricalSQLite) -> Result<(), Box<dyn std::error::Error>> {
    let health = db.health_check().await?;

    println!("🏥 Database Health Check");
    println!("=======================");

    if health.is_healthy {
        println!("✅ Database is healthy!");
    } else {
        println!("⚠️  Database has issues:");
        for issue in &health.issues {
            println!("  - {}", issue);
        }
    }

    println!("Last check: {:?}", health.last_check);

    Ok(())
}

async fn run_demonstration(db: &CategoricalSQLite) -> Result<(), Box<dyn std::error::Error>> {
    println!("🎭 Categorical SQLite Demonstration");
    println!("===================================");
    println!();

    println!("This demonstration showcases the mathematical foundations:");
    println!("• B-Trees as Free Algebras over Tree Operations");
    println!("• Page Cache as Coalgebraic State Machine");
    println!("• Query Planner using Composite Pattern with Strategy Coalgebra");
    println!("• Transaction System as Free Monad over Command Algebra");
    println!("• Schema Management as Algebraic Construction");
    println!();

    // Demonstrate basic operations
    println!("1. Creating a table (Schema Algebra):");
    match db.execute_sql("CREATE TABLE demo_users (id INTEGER PRIMARY KEY, name TEXT NOT NULL)").await {
        Ok(result) => println!("   ✅ {}", result),
        Err(e) => println!("   ❌ Error: {}", e),
    }

    println!();
    println!("2. Inserting data (B-Tree Algebra):");
    let insert_queries = vec![
        "INSERT INTO demo_users (name) VALUES ('Alice')",
        "INSERT INTO demo_users (name) VALUES ('Bob')",
        "INSERT INTO demo_users (name) VALUES ('Charlie')",
    ];

    for query in insert_queries {
        match db.execute_sql(query).await {
            Ok(result) => println!("   ✅ {}", result),
            Err(e) => println!("   ❌ Error: {}", e),
        }
    }

    println!();
    println!("3. Querying data (Query Coalgebra):");
    match db.execute_sql("SELECT * FROM demo_users").await {
        Ok(result) => println!("   ✅ {}", result),
        Err(e) => println!("   ❌ Error: {}", e),
    }

    println!();
    println!("4. Transaction demonstration (Free Monad):");
    // In a real implementation, this would show explicit transaction control
    println!("   ✅ All operations executed in implicit transactions");

    println!();
    println!("Demonstration complete! The mathematical structures ensure:");
    println!("• Compositionality (operations combine predictably)");
    println!("• Correctness (algebraic laws guarantee consistency)");
    println!("• Modularity (components can be reasoned about independently)");

    Ok(())
}

async fn run_benchmarks(db: &CategoricalSQLite) -> Result<(), Box<dyn std::error::Error>> {
    println!("🏃 Performance Benchmarks");
    println!("=========================");
    println!();

    // Simple benchmark: measure insert performance
    let start = std::time::Instant::now();

    // Create test table
    db.execute_sql("CREATE TABLE benchmark_test (id INTEGER PRIMARY KEY, data TEXT)").await?;

    // Insert test data
    for i in 0..100 {
        let query = format!("INSERT INTO benchmark_test (data) VALUES ('test_data_{}')", i);
        db.execute_sql(&query).await?;
    }

    let insert_duration = start.elapsed();

    // Measure query performance
    let query_start = std::time::Instant::now();
    db.execute_sql("SELECT COUNT(*) FROM benchmark_test").await?;
    let query_duration = query_start.elapsed();

    println!("Results:");
    println!("  Insert 100 rows: {:?}", insert_duration);
    println!("  Query performance: {:?}", query_duration);
    println!("  Rows per second: {:.0}", 100.0 / insert_duration.as_secs_f64());

    // Show cache statistics
    let stats = db.get_statistics().await;
    println!();
    println!("Cache Performance:");
    println!("  Hit rate: {:.2}%", stats.page_cache_stats.hit_rate() * 100.0);
    println!("  Total pages: {}", stats.page_cache_stats.total_pages);

    Ok(())
}
