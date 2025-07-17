use clap::{Parser, Subcommand, ValueEnum};
use colored::*;
use proto_vulcan::engine::DefaultEngine;
use proto_vulcan::interpreter::parser::parse_str;
use proto_vulcan::interpreter::query::QueryResult;
use proto_vulcan::interpreter::test_runner::{TestRunOptions, TestRunner};
use proto_vulcan::interpreter::{
    create_main_query, find_main_relation,
    query::QueryConfig,
    trace::{TraceConfig, TraceLevel},
    Interpreter, InterpreterError,
};
use proto_vulcan::user::DefaultUser;
use std::env;
use std::path::PathBuf;

type DefaultInterpreter = Interpreter<DefaultUser, DefaultEngine<DefaultUser>>;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum ColorChoice {
    /// Automatically detect if colors should be used (default)
    Auto,
    /// Always use colors
    Always,
    /// Never use colors
    Never,
}

#[derive(Parser)]
#[command(name = "proto-vulcan")]
#[command(about = "A logic programming language")]
#[command(long_about = r#"Proto-Vulcan Logic Programming Language

EXAMPLES:
    # Run a file with @main relation
    proto-vulcan examples/zebra.pv
    
    # Run a specific query  
    proto-vulcan --query "member(X, [1, 2, 3])" std/list.pv
    
    # Control output format and limit results
    proto-vulcan --format table --limit 10 --query "append(X, Y, [1, 2, 3])" std/list.pv
    proto-vulcan --format numbered --limit 5 examples/zebra.pv
    proto-vulcan --format json --query "member(X, [1, 2, 3])" std/list.pv
    
    # Control colors (auto-enabled for console output)
    proto-vulcan --color always examples/zebra.pv
    proto-vulcan --color never --format table examples/zebra.pv
    proto-vulcan --color auto examples/zebra.pv   # default behavior
    
    # Enable tracing to see search execution
    proto-vulcan --trace --query "member(X, [1, 2, 3])" std/list.pv
    proto-vulcan --trace --trace-level 1 --query "append(X, Y, [1, 2])" std/list.pv  # basic
    proto-vulcan --trace --trace-level 3 --query "member(X, [1, 2, 3])" std/list.pv  # detailed
    
    # Run tests
    proto-vulcan test
    proto-vulcan test --file examples/zebra.pv"#)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// File to run (when not using subcommands)
    file: Option<PathBuf>,

    /// Query to execute instead of @main relation
    #[arg(short, long)]
    query: Option<String>,

    /// Output format for results
    #[arg(short, long, value_enum, default_value = "numbered")]
    format: OutputFormat,

    /// Maximum number of results to show (0 = unlimited)
    #[arg(short, long, default_value = "0")]
    limit: usize,

    /// When to use colors in output
    #[arg(long, value_enum, default_value = "auto")]
    color: ColorChoice,

    /// Enable search tracing to show execution flow
    #[arg(long)]
    trace: bool,

    /// Trace detail level (1=basic, 2=medium, 3=detailed)
    #[arg(long, value_name = "LEVEL", default_value = "2")]
    trace_level: u8,

    /// Timeout for query execution in seconds (0 = no timeout)
    #[arg(long, value_name = "SECONDS", default_value = "0")]
    timeout: u64,
}

#[derive(Subcommand)]
enum Commands {
    /// Runs a proto-vulcan file
    Run {
        /// The path to the file to run
        #[arg(short, long, value_name = "FILE")]
        file: PathBuf,

        /// The query to execute (defaults to @main relation or "main()" if no @main found)
        #[arg(short, long, default_value = "main()")]
        query: String,
    },
    /// Check syntax of a proto-vulcan file without running it
    Check {
        /// The path to the file to check
        #[arg(value_name = "FILE")]
        file: PathBuf,

        /// Show the parsed AST structure
        #[arg(long, help = "Display the parsed abstract syntax tree")]
        show_ast: bool,
    },
    /// Runs the test suite
    Test {
        /// Test name or pattern to run (supports * and ? wildcards)
        test_name: Option<String>,

        /// Filter tests by name pattern (supports * and ? wildcards)
        /// If both test_name and filter are provided, filter takes precedence
        #[arg(short, long)]
        filter: Option<String>,

        /// Run tests from a specific file instead of the entire test suite
        #[arg(long, value_name = "FILE")]
        file: Option<PathBuf>,

        /// Show only failed tests in output
        #[arg(long)]
        failures_only: bool,

        /// Show timing information for each test
        #[arg(long)]
        timing: bool,

        /// Enable parallel test execution (not yet implemented)
        #[arg(long)]
        parallel: bool,

        /// Timeout for individual tests in seconds (default: 10)
        #[arg(long, value_name = "SECONDS", default_value = "10")]
        timeout: u64,
    },
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum OutputFormat {
    /// Automatically choose format based on result characteristics
    Auto,
    /// Numbered list format (1: X = value, 2: Y = value)
    Numbered,
    /// Table format with aligned columns
    Table,
    /// JSON format for programmatic consumption
    Json,
    /// Debug format showing internal structure
    Debug,
}

fn main() {
    let cli = Cli::parse();

    // Set up colors based on CLI choice
    setup_colors(cli.color);

    let result = match &cli.command {
        Some(Commands::Run { file, query }) => run_file(
            file.clone(),
            query.clone(),
            cli.format,
            cli.limit,
            cli.trace,
            cli.trace_level,
            cli.timeout,
        ),
        Some(Commands::Check { file, show_ast }) => parse_file(file.clone(), *show_ast),
        Some(Commands::Test {
            test_name,
            filter,
            file,
            failures_only,
            timing,
            parallel,
            timeout,
        }) => {
            // Use filter if provided, otherwise use test_name
            let effective_filter = filter.clone().or(test_name.clone());

            let options = TestRunOptions {
                filter: effective_filter,
                show_failures_only: *failures_only,
                show_timing: *timing,
                parallel: *parallel,
                timeout_ms: Some(*timeout * 1000), // Convert seconds to milliseconds
            };

            run_tests_with_options(options, file.clone())
        }
        None => {
            if let Some(file) = &cli.file {
                let query_str = cli.query.clone().unwrap_or_else(|| "main()".to_string());
                run_file(
                    file.clone(),
                    query_str,
                    cli.format,
                    cli.limit,
                    cli.trace,
                    cli.trace_level,
                    cli.timeout,
                )
            } else {
                eprintln!("Error: Please provide a file to run or specify a subcommand.");
                std::process::exit(1);
            }
        }
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

/// Set up color output based on user choice
fn setup_colors(choice: ColorChoice) {
    match choice {
        ColorChoice::Always => {
            colored::control::set_override(true);
        }
        ColorChoice::Never => {
            colored::control::set_override(false);
        }
        ColorChoice::Auto => {
            // colored crate automatically detects TTY by default, so do nothing
            // This will enable colors when outputting to terminal, disable when piping to file
        }
    }
}

fn parse_file(path: PathBuf, show_ast: bool) -> Result<(), Box<dyn std::error::Error>> {
    let file_contents = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read file '{}': {}", path.display(), e))?;

    match parse_str(&file_contents) {
        Ok(program) => {
            println!("✓ Syntax check passed for '{}'", path.display());
            if show_ast {
                println!("\nParsed AST:");
                println!("{:#?}", program);
            }
        }
        Err(e) => {
            eprintln!("✗ Parse error in '{}': {}", path.display(), e);
            return Err(e.into());
        }
    }
    Ok(())
}

fn run_file(
    path: PathBuf,
    query: String,
    format: OutputFormat,
    limit: usize,
    trace_enabled: bool,
    trace_level: u8,
    timeout_secs: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut interpreter = DefaultInterpreter::with_stdlib();
    let file_contents = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read file '{}': {}", path.display(), e))?;

    let program =
        parse_str(&file_contents).map_err(|e| InterpreterError::ParseError(e.to_string()))?;

    // Check for @main relation first
    let actual_query = match find_main_relation(&program)? {
        Some(main_rel) => {
            let main_query = create_main_query(main_rel);
            main_query
        }
        None => {
            // No @main relation found, use provided query
            if query == "main()" {
                return Err(format!(
                    "No @main relation found in '{}' and no explicit query provided. \
                    Either add a @main attribute to a relation or specify a query with --query",
                    path.display()
                )
                .into());
            }
            query
        }
    };

    interpreter.load_program(program)?;

    let timeout_ms = if timeout_secs > 0 {
        Some(timeout_secs * 1000) // Convert seconds to milliseconds
    } else {
        None
    };

    let query_result = if trace_enabled {
        // Print query before starting trace
        println!(
            "{} {}",
            "Query:".bright_blue().bold(),
            actual_query.bright_white()
        );

        // Create trace configuration and use QueryConfig
        let trace_config = TraceConfig {
            enabled: true,
            level: TraceLevel::from_u8(trace_level),
        };
        let config = QueryConfig {
            timeout: timeout_ms,
            trace: Some(trace_config),
        };

        // Run query with unified configuration
        interpreter.execute_query(&actual_query, config)
    } else {
        // Run normal query with timeout
        interpreter.query_with_timeout(&actual_query, timeout_ms)
    };

    match query_result {
        Ok(mut results) => {
            // Apply limit if specified (0 means unlimited)
            let was_limited = limit > 0 && results.len() > limit;
            if was_limited {
                results.truncate(limit);
                if format != OutputFormat::Json {
                    println!("Note: Showing first {} of many results.", limit);
                }
            }

            if !results.is_empty() {
                if trace_enabled {
                    // Don't print query again since we already printed it before trace
                    print_results_without_query(&results, format, was_limited);
                } else {
                    print_query_results(&results, format, was_limited, &actual_query);
                }
            } else {
                if format == OutputFormat::Json {
                    let query_for_json = if trace_enabled { "" } else { &actual_query };
                    print_json_results(&[], false, query_for_json);
                } else if !trace_enabled {
                    println!(
                        "{} {}",
                        "Query:".bright_blue().bold(),
                        actual_query.bright_white()
                    );
                }
                if format != OutputFormat::Json {
                    println!("Query succeeded with no results.");
                }
            }
        }
        Err(e) => {
            eprintln!("Error running query '{}': {}", actual_query, e);
            return Err(e.into());
        }
    }
    Ok(())
}

fn run_tests_with_options(
    options: TestRunOptions,
    file: Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Discovering tests...");

    let runner = if let Some(test_file) = file {
        // Run tests from a specific file
        TestRunner::from_file(&test_file)?
    } else {
        // Run tests from the entire directory
        let current_dir = env::current_dir()?;
        TestRunner::new(&current_dir)?
    };

    println!("Discovered {} tests.", runner.discovered_tests.len());

    let stats = runner.run_with_options(&options);

    // Exit with error code if any tests failed
    if stats.failed > 0 || stats.errors > 0 {
        std::process::exit(1);
    }

    Ok(())
}

// Keep the old function for backward compatibility
fn run_tests() -> Result<(), Box<dyn std::error::Error>> {
    let options = TestRunOptions::default();
    run_tests_with_options(options, None)
}

/// Print results without query header (for trace mode)
fn print_results_without_query(
    results: &[QueryResult<DefaultUser, DefaultEngine<DefaultUser>>],
    format: OutputFormat,
    was_limited: bool,
) {
    match format {
        OutputFormat::Auto => {
            // Choose formatting style based on result complexity
            let result_count = results.len();
            if result_count <= 5 {
                print_numbered_results(results);
            } else {
                print_compact_results(results);
            }
        }
        OutputFormat::Numbered => {
            print_numbered_results(results);
        }
        OutputFormat::Table => {
            if results.is_empty() {
                println!("{}", "Query succeeded with no results.".bright_green());
                return;
            }
            let first_result = &results[0];
            let mut all_var_names: Vec<String> = first_result.bindings.keys().cloned().collect();
            all_var_names.sort();
            print_table_format(results, &all_var_names);
        }
        OutputFormat::Json => print_json_results(results, was_limited, ""),
        OutputFormat::Debug => {
            println!("{}", "Results:".bright_blue().bold());
            for (i, result) in results.iter().enumerate() {
                println!(
                    "  {} {}: {:?}",
                    "Solution".bright_green(),
                    format!("{}", i + 1).bright_yellow().bold(),
                    result
                );
            }
        }
    }
}

/// Pretty print query results with mathematical formatting and where-clauses
fn print_query_results(
    results: &[QueryResult<DefaultUser, DefaultEngine<DefaultUser>>],
    format: OutputFormat,
    was_limited: bool,
    query: &str,
) {
    match format {
        OutputFormat::Auto => {
            println!("{} {}", "Query:".bright_blue().bold(), query.bright_white());
            // Choose formatting style based on result complexity
            let result_count = results.len();
            if result_count <= 5 {
                print_numbered_results(results);
            } else {
                print_compact_results(results);
            }
        }
        OutputFormat::Numbered => {
            println!("{} {}", "Query:".bright_blue().bold(), query.bright_white());
            print_numbered_results(results);
        }
        OutputFormat::Table => {
            println!("{} {}", "Query:".bright_blue().bold(), query.bright_white());
            if results.is_empty() {
                println!("{}", "Query succeeded with no results.".bright_green());
                return;
            }
            let first_result = &results[0];
            let mut all_var_names: Vec<String> = first_result.bindings.keys().cloned().collect();
            all_var_names.sort();
            print_table_format(results, &all_var_names);
        }
        OutputFormat::Json => print_json_results(results, was_limited, query),
        OutputFormat::Debug => {
            println!("{} {}", "Query:".bright_blue().bold(), query.bright_white());
            println!("{}", "Results:".bright_blue().bold());
            for (i, result) in results.iter().enumerate() {
                println!(
                    "  {} {}: {:?}",
                    "Solution".bright_green(),
                    format!("{}", i + 1).bright_yellow().bold(),
                    result
                );
            }
        }
    }
}

/// Print multiple results with clear numbering
fn print_numbered_results(results: &[QueryResult<DefaultUser, DefaultEngine<DefaultUser>>]) {
    println!("{}", "Solutions:".bright_blue().bold());
    for (i, result) in results.iter().enumerate() {
        print!("  {}: ", format!("{}", i + 1).bright_yellow().bold());
        if result.bindings.is_empty() {
            println!("{}", "true".bright_green());
        } else {
            let mut vars: Vec<_> = result.bindings.iter().collect();
            vars.sort_by_key(|(name, _)| name.as_str());

            for (j, (var_name, lresult)) in vars.iter().enumerate() {
                if j == 0 {
                    print!("{} {} {}", var_name.bright_cyan(), "=".white(), lresult);
                } else {
                    print!(", {} {} {}", var_name.bright_cyan(), "=".white(), lresult);
                }
            }
            println!();
        }
    }
}

/// Print many results in compact tabular format
fn print_compact_results(results: &[QueryResult<DefaultUser, DefaultEngine<DefaultUser>>]) {
    if results.is_empty() {
        return;
    }

    // Get all variable names and determine if we can use table format
    let first_result = &results[0];
    let mut all_var_names: Vec<String> = first_result.bindings.keys().cloned().collect();
    all_var_names.sort();

    // Check if all results have the same variables (for table format)
    let can_use_table = results.iter().all(|result| {
        let mut result_vars: Vec<String> = result.bindings.keys().cloned().collect();
        result_vars.sort();
        result_vars == all_var_names
    });

    if can_use_table
        && all_var_names.len() <= 4
        && all_var_names.iter().all(|name| name.len() <= 20)
    {
        print_table_format(results, &all_var_names);
    } else {
        print_numbered_results(results);
    }
}

/// Print results in table format (for uniform, simple results)
fn print_table_format(
    results: &[QueryResult<DefaultUser, DefaultEngine<DefaultUser>>],
    var_names: &[String],
) {
    println!(
        "{} ({} {})",
        "Solutions".bright_blue().bold(),
        format!("{}", results.len()).bright_yellow().bold(),
        "found):".bright_blue().bold()
    );

    // Print header
    print!("  ");
    for (i, var_name) in var_names.iter().enumerate() {
        if i == 0 {
            print!("{:15}", var_name.bright_cyan().bold());
        } else {
            print!(" {} {:15}", "|".white(), var_name.bright_cyan().bold());
        }
    }
    println!();

    // Print separator
    print!("  ");
    for i in 0..var_names.len() {
        if i == 0 {
            print!("{}", format!("{:-<15}", "").white());
        } else {
            print!("{}{}", "-|-".white(), format!("{:-<15}", "").white());
        }
    }
    println!();

    // Print data rows
    for result in results.iter() {
        print!("  ");
        for (i, var_name) in var_names.iter().enumerate() {
            let value_str = if let Some(lresult) = result.bindings.get(var_name) {
                // For table format, only show the term without constraints to keep it compact
                format!("{}", lresult.0)
            } else {
                "unbound".bright_red().to_string()
            };

            let truncated = if value_str.len() > 15 {
                format!("{}...", &value_str[..12])
            } else {
                value_str
            };

            if i == 0 {
                print!("{:15}", truncated.bright_white());
            } else {
                print!(" {} {:15}", "|".white(), truncated.bright_white());
            }
        }
        println!();
    }
}

/// Print results in JSON format for programmatic consumption
fn print_json_results(
    results: &[QueryResult<DefaultUser, DefaultEngine<DefaultUser>>],
    was_limited: bool,
    query: &str,
) {
    use serde_json::{json, Map, Value};

    let mut solutions = Vec::new();

    for result in results {
        let mut solution = Map::new();

        for (var_name, lresult) in &result.bindings {
            // Convert LResult to string representation for JSON
            let value_str = format!("{}", lresult);
            solution.insert(var_name.clone(), Value::String(value_str));
        }

        solutions.push(Value::Object(solution));
    }

    let json_output = json!({
        "query": query,
        "solutions": solutions,
        "count": results.len(),
        "limited": was_limited
    });

    println!("{}", serde_json::to_string_pretty(&json_output).unwrap());
}
