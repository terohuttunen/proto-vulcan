use clap::{Parser, Subcommand};
use proto_vulcan::engine::DefaultEngine;
use proto_vulcan::interpreter::parser::parse_str;
use proto_vulcan::interpreter::test_runner::{TestRunOptions, TestRunner};
use proto_vulcan::interpreter::{Interpreter, InterpreterError};
use proto_vulcan::user::DefaultUser;
use std::env;
use std::path::PathBuf;

type DefaultInterpreter = Interpreter<DefaultUser, DefaultEngine<DefaultUser>>;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(after_help = "EXAMPLES:
    proto-vulcan file.pv                    # Run a proto-vulcan file
    proto-vulcan run --file file.pv         # Same as above (explicit)
    proto-vulcan check file.pv              # Check syntax without running
    proto-vulcan test                       # Run all tests
    proto-vulcan test my_test               # Run specific test
    proto-vulcan test --file file.pv        # Run tests from specific file")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// The proto-vulcan file to run (shorthand for 'run --file FILE')
    #[arg(value_name = "FILE", help = "Proto-vulcan file to run")]
    file: Option<PathBuf>,

    /// The query to execute when using positional file argument
    #[arg(short, long, default_value = "main()", help = "Query to execute")]
    query: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Runs a proto-vulcan file
    Run {
        /// The path to the file to run
        #[arg(short, long, value_name = "FILE")]
        file: PathBuf,

        /// The query to execute (defaults to "main()")
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
    },
}

fn main() {
    let cli = Cli::parse();

    // Handle positional file argument (shorthand for run command)
    if let Some(file) = cli.file {
        if cli.command.is_some() {
            eprintln!("Error: Cannot specify both a file argument and a subcommand.");
            eprintln!("Use either 'proto-vulcan file.pv' or 'proto-vulcan run --file file.pv'");
            std::process::exit(1);
        }

        let query = cli.query.unwrap_or_else(|| "main()".to_string());
        if let Err(e) = run_file(file, query) {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
        return;
    }

    // Handle subcommands
    match cli.command {
        Some(Commands::Run { file, query }) => {
            if let Err(e) = run_file(file, query) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Some(Commands::Check { file, show_ast }) => {
            if let Err(e) = parse_file(file, show_ast) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Some(Commands::Test {
            test_name,
            filter,
            file,
            failures_only,
            timing,
            parallel,
        }) => {
            // Use filter if provided, otherwise use test_name
            let effective_filter = filter.or(test_name);

            let options = TestRunOptions {
                filter: effective_filter,
                show_failures_only: failures_only,
                show_timing: timing,
                parallel,
            };

            if let Err(e) = run_tests_with_options(options, file) {
                eprintln!("Error running tests: {}", e);
                std::process::exit(1);
            }
        }
        None => {
            // No subcommand and no file provided - show help
            eprintln!("Error: No file or command specified.");
            eprintln!("Use 'proto-vulcan --help' for usage information.");
            eprintln!();
            eprintln!("Quick examples:");
            eprintln!("  proto-vulcan file.pv          # Run a file");
            eprintln!("  proto-vulcan check file.pv    # Check syntax");
            eprintln!("  proto-vulcan test             # Run tests");
            eprintln!("  proto-vulcan test --file file.pv  # Run tests from file");
            std::process::exit(1);
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

fn run_file(path: PathBuf, query: String) -> Result<(), Box<dyn std::error::Error>> {
    let mut interpreter = DefaultInterpreter::with_stdlib();
    let file_contents = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read file '{}': {}", path.display(), e))?;

    let program =
        parse_str(&file_contents).map_err(|e| InterpreterError::ParseError(e.to_string()))?;
    interpreter.load_program(program)?;

    match interpreter.query(&query) {
        Ok(results) => {
            if !results.is_empty() {
                println!("Query results:");
                for result in results {
                    println!("{:?}", result);
                }
            } else {
                println!("Query '{}' succeeded with no results.", query);
            }
        }
        Err(e) => {
            eprintln!("Error running query '{}': {}", query, e);
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
