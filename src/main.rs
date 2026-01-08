//! Ferrum CLI - Main entry point
//!
//! This is the main executable that handles command-line arguments
//! and dispatches to the appropriate functionality.

use std::process::ExitCode;

use tracing::{error, info};

// Import Ferrum library
use ferrum::{
    cli::{parse_args, Commands},
    init_v8, shutdown_v8,
    repl::{start_repl, ReplConfig},
    runtime::{RuntimeConfig, RuntimeError},
};

/// Main entry point
fn main() -> ExitCode {
    // Parse CLI arguments
    let args = parse_args();

    // Initialize tracing
    init_logging(&args);

    // Initialize V8 platform (thread-safe)
    init_v8();

    // Ensure V8 is shutdown on exit
    let result = run(&args);

    // Shutdown V8
    unsafe {
        shutdown_v8();
    }

    match result {
        Ok(_) => ExitCode::SUCCESS,
        Err(e) => {
            error!("Error: {}", e);
            ExitCode::FAILURE
        }
    }
}

/// Initialize logging based on CLI arguments
fn init_logging(args: &ferrum::Cli) {
    let filter = tracing_subscriber::EnvFilter::builder()
        .with_default_directive(
            format!("ferrum={}", args.log_level).parse().unwrap(),
        )
        .from_env_lossy();

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .init();
}

/// Run the appropriate command
fn run(args: &ferrum::Cli) -> Result<(), FerrumError> {
    info!("Ferrum v{}", ferrum::VERSION);

    match &args.command {
        Commands::Run {
            script,
            eval,
            check,
            import_map: _,
            ..
        } => {
            if *check {
                return run_check(script);
            }

            if let Some(code) = eval {
                return run_eval(code, &args.command);
            }

            run_script(script, &args.command)
        }

        Commands::Repl { .. } => run_repl(&args.command),

        Commands::Fmt { files, check, .. } => {
            if files.is_empty() {
                println!("No files specified for formatting");
                return Ok(());
            }
            run_format(files, *check)
        }

        Commands::Test { files, .. } => {
            if files.is_empty() {
                // Search for test files
                run_tests(&[".".to_string()], &args.command)
            } else {
                run_tests(files, &args.command)
            }
        }

        Commands::Bundle {
            input,
            output,
            import_map: _,
            ..
        } => run_bundle(input, output),

        Commands::Install { name, args: script_args, .. } => {
            run_install(name, script_args, &args.command)
        },

        Commands::Cache { subcommand } => run_cache(subcommand),

        Commands::Compile {
            input,
            output,
            ..
        } => run_compile(input, output),

        Commands::Info { module, .. } => run_info(module),

        Commands::Lint { files, .. } => {
            if files.is_empty() {
                println!("No files specified for linting");
                Ok(())
            } else {
                run_lint(files)
            }
        }

        Commands::Check { files } => {
            if files.is_empty() {
                println!("No files specified for checking");
                Ok(())
            } else {
                for file in files {
                    run_check(file)?;
                }
                Ok(())
            }
        }

        Commands::Doc {
            files,
            output,
            serve,
            port,
        } => run_doc(files, output, *serve, *port),

        Commands::Upgrade { version, .. } => run_upgrade(version.as_deref()),

        Commands::Completions { shell } => run_completions(shell),
    }
}

/// Run a JavaScript/TypeScript file
fn run_script(script: &str, command: &Commands) -> Result<(), FerrumError> {
    let permissions = command.permissions();

    info!("Running script: {}", script);
    info!("Permissions: {:?}", permissions);

    let config = RuntimeConfig::default();
    let mut runtime = ferrum::JsRuntime::new(config, permissions)
        .map_err(|e| FerrumError::Runtime(e.to_string()))?;

    // Execute the script
    match runtime.execute_file(script) {
        Ok(_) => {
            info!("Script executed successfully");
            Ok(())
        }
        Err(e) => {
            error!("Script execution failed: {}", e);
            Err(FerrumError::Runtime(e.to_string()))
        }
    }
}

/// Evaluate a JavaScript expression
fn run_eval(code: &str, command: &Commands) -> Result<(), FerrumError> {
    let permissions = command.permissions();

    let config = RuntimeConfig::default();
    let mut runtime = ferrum::JsRuntime::new(config, permissions)
        .map_err(|e| FerrumError::Runtime(e.to_string()))?;

    match runtime.execute(code, Some("<eval>")) {
        Ok(output) => {
            if !output.is_empty() {
                println!("{}", output);
            }
            Ok(())
        }
        Err(e) => Err(FerrumError::Runtime(e.to_string())),
    }
}

/// Run type check on a file
fn run_check(file: &str) -> Result<(), FerrumError> {
    info!("Checking: {}", file);

    // TODO: Implement actual type checking with TypeScript compiler
    println!("Type checking is not yet fully implemented");
    println!("File: {}", file);

    Ok(())
}

/// Run the REPL
fn run_repl(command: &Commands) -> Result<(), FerrumError> {
    let permissions = command.permissions();
    let _config = ReplConfig::default();

    info!("Starting REPL");

    start_repl(permissions).map_err(|e| FerrumError::Runtime(e.to_string()))
}

/// Format code
fn run_format(files: &[String], check: bool) -> Result<(), FerrumError> {
    info!("Formatting files: {:?}", files);

    for file in files {
        info!("Processing: {}", file);
        // TODO: Implement actual formatting
        if check {
            println!("Checking: {}", file);
        } else {
            println!("Formatting: {}", file);
        }
    }

    println!("Formatting not yet implemented");
    Ok(())
}

/// Run tests
fn run_tests(files: &[String], command: &Commands) -> Result<(), FerrumError> {
    let _permissions = command.permissions();

    info!("Running tests: {:?}", files);

    for file in files {
        info!("Testing: {}", file);
        // TODO: Implement actual test runner
    }

    println!("Test runner not yet implemented");
    Ok(())
}

/// Bundle modules
fn run_bundle(input: &str, output: &std::path::PathBuf) -> Result<(), FerrumError> {
    info!("Bundling: {} -> {:?}", input, output);

    // TODO: Implement bundling
    println!("Bundling not yet implemented");
    Ok(())
}

/// Install and run a script
fn run_install(name: &str, _args: &[String], command: &Commands) -> Result<(), FerrumError> {
    let _permissions = command.permissions();

    info!("Installing: {}", name);

    // TODO: Implement installation
    println!("Install command not yet implemented");
    Ok(())
}

/// Cache operations
fn run_cache(subcommand: &ferrum::cli::CacheCommands) -> Result<(), FerrumError> {
    match subcommand {
        ferrum::cli::CacheCommands::Clear => {
            info!("Clearing cache");
            println!("Cache cleared");
            Ok(())
        }
        ferrum::cli::CacheCommands::Info => {
            info!("Cache info");
            println!("Cache information not yet implemented");
            Ok(())
        }
        ferrum::cli::CacheCommands::Prune => {
            info!("Pruning cache");
            println!("Cache pruning not yet implemented");
            Ok(())
        }
    }
}

/// Compile to executable
fn run_compile(input: &str, output: &std::path::PathBuf) -> Result<(), FerrumError> {
    info!("Compiling: {} -> {:?}", input, output);

    // TODO: Implement compilation
    println!("Compilation not yet implemented");
    Ok(())
}

/// Show module info
fn run_info(module: &str) -> Result<(), FerrumError> {
    info!("Module info: {}", module);

    // TODO: Implement module info
    println!("Module info not yet implemented");
    Ok(())
}

/// Lint code
fn run_lint(files: &[String]) -> Result<(), FerrumError> {
    info!("Linting files: {:?}", files);

    for file in files {
        info!("Linting: {}", file);
        // TODO: Implement linting
    }

    println!("Linting not yet implemented");
    Ok(())
}

/// Generate documentation
fn run_doc(files: &[String], _output: &std::path::PathBuf, _serve: bool, _port: u16) -> Result<(), FerrumError> {
    info!("Generating documentation for: {:?}", files);

    // TODO: Implement documentation generation
    println!("Documentation generation not yet implemented");
    Ok(())
}

/// Upgrade to latest version
fn run_upgrade(version: Option<&str>) -> Result<(), FerrumError> {
    info!("Upgrading to {:?}", version);

    // TODO: Implement upgrade
    println!("Upgrade not yet implemented");
    Ok(())
}

/// Generate shell completions
fn run_completions(shell: &str) -> Result<(), FerrumError> {
    use clap::CommandFactory;

    info!("Generating completions for: {}", shell);

    let mut cmd = ferrum::Cli::command();

    match shell {
        "bash" => {
            clap_complete::generate(clap_complete::Shell::Bash, &mut cmd, "ferrum", &mut std::io::stdout());
        }
        "elvish" => {
            clap_complete::generate(clap_complete::Shell::Elvish, &mut cmd, "ferrum", &mut std::io::stdout());
        }
        "fish" => {
            clap_complete::generate(clap_complete::Shell::Fish, &mut cmd, "ferrum", &mut std::io::stdout());
        }
        "powershell" => {
            clap_complete::generate(clap_complete::Shell::PowerShell, &mut cmd, "ferrum", &mut std::io::stdout());
        }
        "zsh" => {
            clap_complete::generate(clap_complete::Shell::Zsh, &mut cmd, "ferrum", &mut std::io::stdout());
        }
        _ => {
            return Err(FerrumError::Unknown(format!("Unsupported shell: {}", shell)));
        }
    }

    Ok(())
}

/// Error types for the CLI
#[derive(Debug, thiserror::Error)]
enum FerrumError {
    #[error("Runtime error: {0}")]
    Runtime(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Module error: {0}")]
    Module(String),

    #[error("Unknown error: {0}")]
    Unknown(String),
}

impl From<RuntimeError> for FerrumError {
    fn from(e: RuntimeError) -> Self {
        FerrumError::Runtime(e.to_string())
    }
}

impl From<ferrum::module_loader::ModuleError> for FerrumError {
    fn from(e: ferrum::module_loader::ModuleError) -> Self {
        FerrumError::Module(e.to_string())
    }
}
