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
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Runs a proto-vulcan file
    Run {
        /// The path to the file to run
        #[arg(short, long)]
        file: PathBuf,
    },
    /// Runs the test suite
    Test {
        /// Test name or pattern to run (supports * and ? wildcards)
        test_name: Option<String>,

        /// Filter tests by name pattern (supports * and ? wildcards)
        /// If both test_name and filter are provided, filter takes precedence
        #[arg(short, long)]
        filter: Option<String>,

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

    match cli.command {
        Commands::Run { file } => {
            if let Err(e) = run_file(file) {
                eprintln!("Error: {}", e);
            }
        }
        Commands::Test {
            test_name,
            filter,
            failures_only,
            timing,
            parallel,
        } => {
            // Use filter if provided, otherwise use test_name
            let effective_filter = filter.or(test_name);

            let options = TestRunOptions {
                filter: effective_filter,
                show_failures_only: failures_only,
                show_timing: timing,
                parallel,
            };

            if let Err(e) = run_tests_with_options(options) {
                eprintln!("Error running tests: {}", e);
                std::process::exit(1);
            }
        }
    }
}

fn run_file(path: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let mut interpreter = DefaultInterpreter::with_stdlib();
    let file_contents = std::fs::read_to_string(path)?;

    let program =
        parse_str(&file_contents).map_err(|e| InterpreterError::ParseError(e.to_string()))?;
    interpreter.load_program(program)?;

    match interpreter.query("main()") {
        Ok(results) => {
            if !results.is_empty() {
                println!("Query results:");
                for result in results {
                    println!("{:?}", result);
                }
            } else {
                println!("Query 'main()' succeeded with no results.");
            }
        }
        Err(e) => {
            eprintln!("Error running query 'main()': {}", e);
        }
    }
    Ok(())
}

fn run_tests_with_options(options: TestRunOptions) -> Result<(), Box<dyn std::error::Error>> {
    println!("Discovering tests...");
    let current_dir = env::current_dir()?;
    let runner = TestRunner::new(&current_dir)?;

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
    run_tests_with_options(options)
}
