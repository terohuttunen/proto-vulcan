use clap::{Parser, Subcommand};
use proto_vulcan::engine::{DefaultEngine, Engine};
use proto_vulcan::interpreter::parser::parse_str;
use proto_vulcan::interpreter::test_runner::TestRunner;
use proto_vulcan::interpreter::{Interpreter, InterpreterError};
use proto_vulcan::user::{DefaultUser, User};
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
    Test,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Run { file } => {
            if let Err(e) = run_file(file) {
                eprintln!("Error: {}", e);
            }
        }
        Commands::Test => {
            if let Err(e) = run_tests() {
                eprintln!("Error running tests: {}", e);
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

fn run_tests() -> Result<(), Box<dyn std::error::Error>> {
    println!("Discovering tests...");
    let current_dir = env::current_dir()?;
    let runner = TestRunner::new(&current_dir)?;

    println!("Discovered {} tests.", runner.discovered_tests.len());

    runner.run();

    Ok(())
}
