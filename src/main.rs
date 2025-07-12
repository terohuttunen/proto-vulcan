use clap::{Parser, Subcommand};
use proto_vulcan::engine::DefaultEngine;
use proto_vulcan::interpreter::{Interpreter, InterpreterError};
use proto_vulcan::user::DefaultUser;
use rustyline::error::ReadlineError;
use rustyline::Editor;
use std::fs;

use std::path::PathBuf;
use std::process;

type ProtoVulcanInterpreter = Interpreter<DefaultUser, DefaultEngine<DefaultUser>>;

#[derive(Parser)]
#[command(name = "proto-vulcan")]
#[command(about = "A command-line tool for running proto-vulcan programs")]
#[command(version = "0.1.6")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Run a proto-vulcan program from a file
    Run {
        /// Path to the proto-vulcan source file
        file: PathBuf,
        /// Query to execute against the program
        #[arg(short, long)]
        query: Option<String>,
        /// Output format (pretty, json)
        #[arg(short, long, default_value = "pretty")]
        format: String,
    },
    /// Start an interactive REPL session
    Repl {
        /// Optional program file to load first
        file: Option<PathBuf>,
    },
    /// Parse and validate a proto-vulcan program
    Check {
        /// Path to the proto-vulcan source file
        file: PathBuf,
    },
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Some(Commands::Run {
            file,
            query,
            format,
        }) => run_file(file, query, format),
        Some(Commands::Repl { file }) => run_repl(file),
        Some(Commands::Check { file }) => check_file(file),
        None => run_repl(None),
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}

fn run_file(
    file: PathBuf,
    query: Option<String>,
    format: String,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut interpreter = ProtoVulcanInterpreter::with_stdlib();

    // Read and parse the program file
    let source = fs::read_to_string(&file)?;
    let program = proto_vulcan::interpreter::parser::parse_str(&source)
        .map_err(|e| format!("Parse error in {}: {}", file.display(), e))?;

    // Load the program into the interpreter
    interpreter
        .load_program(program)
        .map_err(|e| format!("Load error: {}", format_interpreter_error(&e)))?;

    match query {
        Some(query_str) => {
            // Execute the specified query
            let results = interpreter
                .query(&query_str)
                .map_err(|e| format!("Query error: {}", format_interpreter_error(&e)))?;

            print_results(&results, &format)?;
        }
        None => {
            println!("Program loaded successfully. Use --query to execute a query.");
        }
    }

    Ok(())
}

fn run_repl(initial_file: Option<PathBuf>) -> Result<(), Box<dyn std::error::Error>> {
    let mut interpreter = ProtoVulcanInterpreter::with_stdlib();
    let mut rl = Editor::<()>::new()?;

    // Load initial file if provided
    if let Some(file) = initial_file {
        let source = fs::read_to_string(&file)?;
        match proto_vulcan::interpreter::parser::parse_str(&source) {
            Ok(program) => match interpreter.load_program(program) {
                Ok(_) => println!("Loaded program from {}", file.display()),
                Err(e) => eprintln!("Error loading program: {}", format_interpreter_error(&e)),
            },
            Err(e) => eprintln!("Parse error in {}: {}", file.display(), e),
        }
    }

    println!("Proto-Vulcan REPL v0.1.6");
    println!("Type 'help' for commands, 'quit' to exit");

    loop {
        let readline = rl.readline("proto-vulcan> ");
        match readline {
            Ok(line) => {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }

                rl.add_history_entry(line);

                match line {
                    "quit" | "exit" => break,
                    "help" => print_help(),
                    "env" => print_environment(&interpreter),
                    line if line.starts_with("load ") => {
                        let file_path = line.strip_prefix("load ").unwrap().trim();
                        match load_file(&mut interpreter, file_path) {
                            Ok(_) => println!("Loaded program from {}", file_path),
                            Err(e) => eprintln!("Error: {}", e),
                        }
                    }
                    _ => {
                        // Treat as a query
                        match interpreter.query(line) {
                            Ok(results) => {
                                if let Err(e) = print_results(&results, "pretty") {
                                    eprintln!("Output error: {}", e);
                                }
                            }
                            Err(e) => eprintln!("Query error: {}", format_interpreter_error(&e)),
                        }
                    }
                }
            }
            Err(ReadlineError::Interrupted) => {
                println!("CTRL-C");
                continue;
            }
            Err(ReadlineError::Eof) => {
                println!("CTRL-D");
                break;
            }
            Err(err) => {
                eprintln!("Error: {:?}", err);
                break;
            }
        }
    }

    Ok(())
}

fn check_file(file: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let source = fs::read_to_string(&file)?;

    match proto_vulcan::interpreter::parser::parse_str(&source) {
        Ok(program) => {
            println!("✓ Parse successful");
            println!("  Items: {}", program.items.len());

            // Try to load into interpreter for semantic checking
            let mut interpreter = ProtoVulcanInterpreter::with_stdlib();
            match interpreter.load_program(program) {
                Ok(_) => println!("✓ Load successful"),
                Err(e) => {
                    eprintln!("✗ Load error: {}", format_interpreter_error(&e));
                    return Err(e.into());
                }
            }
        }
        Err(e) => {
            eprintln!("✗ Parse error: {}", e);
            return Err(e.into());
        }
    }

    Ok(())
}

fn load_file(
    interpreter: &mut ProtoVulcanInterpreter,
    file_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let source = fs::read_to_string(file_path)?;
    let program = proto_vulcan::interpreter::parser::parse_str(&source)
        .map_err(|e| format!("Parse error: {}", e))?;

    interpreter
        .load_program(program)
        .map_err(|e| format!("Load error: {}", format_interpreter_error(&e)))?;

    Ok(())
}

fn print_help() {
    println!("Available commands:");
    println!("  help                 - Show this help message");
    println!("  quit, exit           - Exit the REPL");
    println!("  env                  - Show current environment");
    println!("  load <file>          - Load a proto-vulcan program file");
    println!("  <query>              - Execute a query");
    println!();
    println!("Example queries:");
    println!("  parent(X, Y)         - Find all parent relationships");
    println!("  X == 42              - Simple equality query");
    println!("  member(X, [1,2,3])   - Find members of a list");
}

fn print_environment(interpreter: &ProtoVulcanInterpreter) {
    let env = interpreter.environment();
    println!("Environment:");
    println!("  Relations: {}", env.relations().len());
    println!("  Structs: {}", env.structs().len());
    println!("  Variables: {}", env.variables().len());

    if !env.relations().is_empty() {
        println!("  Available relations:");
        for name in env.relations().keys() {
            println!("    {}", name);
        }
    }

    if !env.structs().is_empty() {
        println!("  Available structs:");
        for name in env.structs().keys() {
            println!("    {}", name);
        }
    }
}

fn print_results(
    results: &[proto_vulcan::interpreter::query::QueryResult<
        DefaultUser,
        DefaultEngine<DefaultUser>,
    >],
    format: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if format == "json" {
        // Simple JSON output
        let json_results: Vec<_> = results
            .iter()
            .map(|r| {
                r.bindings
                    .iter()
                    .map(|(k, v)| (k.clone(), format!("{}", v)))
                    .collect::<std::collections::HashMap<_, _>>()
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&json_results)?);
    } else {
        // Pretty print
        println!("Results ({}):", results.len());
        for (i, result) in results.iter().enumerate() {
            if result.is_empty() {
                println!("  {}: No bindings", i + 1);
                continue;
            }
            println!("  {}:", i + 1);
            for (var, value) in &result.bindings {
                println!("    {}: {}", var, value);
            }
        }
    }
    Ok(())
}

fn format_interpreter_error(error: &InterpreterError) -> String {
    match error {
        InterpreterError::ParseError(e) => format!("Parse error: {}", e),
        InterpreterError::RuntimeError(e) => format!("Runtime error: {}", e),
        InterpreterError::UnknownRelation(name) => format!("Unknown relation: {}", name),
        InterpreterError::UnknownVariable(name) => format!("Unknown variable: {}", name),
        InterpreterError::DuplicateDefinition(name) => format!("Duplicate definition: {}", name),
        InterpreterError::ModuleNotFound(path) => format!("Module not found: {}", path.display()),
        InterpreterError::IoError(msg) => format!("I/O error: {}", msg),
    }
}
