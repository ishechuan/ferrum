//! Command-line interface parsing and configuration
//!
//! This module handles all CLI argument parsing using clap.

use clap::{Parser, Subcommand};
use std::path::PathBuf;

use crate::permissions::Permissions;

/// Ferrum - A simple, secure, and modern JavaScript/TypeScript Runtime
#[derive(Parser, Debug)]
#[command(name = "ferrum")]
#[command(author = "Ferrum Contributors")]
#[command(version = "0.1.0")]
#[command(about = "A JavaScript/TypeScript runtime", long_about = None)]
pub struct Cli {
    /// Enable verbose output
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Set log level (trace, debug, info, warn, error)
    #[arg(long, global = true, default_value = "info")]
    pub log_level: String,

    /// Subcommand to execute
    #[command(subcommand)]
    pub command: Commands,
}

/// Available commands
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Run a JavaScript or TypeScript file
    Run {
        /// Path to the script file
        #[arg(value_name = "SCRIPT")]
        script: String,

        /// Arguments to pass to the script
        #[arg(value_name = "ARGS", trailing_var_arg = true)]
        args: Vec<String>,

        /// Allow file system read access
        #[arg(long)]
        allow_read: bool,

        /// Allow file system read access to specific paths
        #[arg(long, value_name = "PATHS", value_delimiter = ',')]
        allow_read_path: Option<Vec<String>>,

        /// Allow file system write access
        #[arg(long)]
        allow_write: bool,

        /// Allow file system write access to specific paths
        #[arg(long, value_name = "PATHS", value_delimiter = ',')]
        allow_write_path: Option<Vec<String>>,

        /// Allow network access
        #[arg(long)]
        allow_net: bool,

        /// Allow network access to specific domains
        #[arg(long, value_name = "DOMAINS", value_delimiter = ',')]
        allow_net_domain: Option<Vec<String>>,

        /// Allow environment variable access
        #[arg(long)]
        allow_env: bool,

        /// Allow access to specific environment variables
        #[arg(long, value_name = "VARS", value_delimiter = ',')]
        allow_env_var: Option<Vec<String>>,

        /// Allow running subprocesses
        #[arg(long)]
        allow_run: bool,

        /// Allow running specific commands
        #[arg(long, value_name = "COMMANDS", value_delimiter = ',')]
        allow_run_command: Option<Vec<String>>,

        /// Allow all permissions
        #[arg(long)]
        allow_all: bool,

        /// Disable permission checks (DANGEROUS!)
        #[arg(long, hide = true)]
        unsafe_no_permissions: bool,

        /// Set import map path
        #[arg(long, value_name = "PATH")]
        import_map: Option<PathBuf>,

        /// Enable inspector for debugging
        #[arg(long)]
        inspect: bool,

        /// Inspector port
        #[arg(long, default_value = "9229")]
        inspect_port: u16,

        /// Enable source map support
        #[arg(long)]
        enable_source_maps: bool,

        /// Check script without executing
        #[arg(long)]
        check: bool,

        /// Evaluate script from string instead of file
        #[arg(long, value_name = "CODE")]
        eval: Option<String>,

        /// Watch mode for development
        #[arg(long)]
        watch: bool,
    },

    /// Start an interactive REPL
    Repl {
        /// Allow all permissions in REPL
        #[arg(long)]
        allow_all: bool,

        /// Enable source map support
        #[arg(long)]
        enable_source_maps: bool,
    },

    /// Format JavaScript/TypeScript code
    Fmt {
        /// Files or directories to format
        #[arg(value_name = "FILES")]
        files: Vec<String>,

        /// Check formatting without making changes
        #[arg(long)]
        check: bool,

        /// Use single quote instead of double quote
        #[arg(long)]
        single_quote: bool,

        /// Use 4 spaces instead of 2
        #[arg(long)]
        use_tabs: bool,

        /// Print output to stdout instead of writing to files
        #[arg(long)]
        stdout: bool,
    },

    /// Run tests
    Test {
        /// Files or directories containing tests
        #[arg(value_name = "FILES")]
        files: Vec<String>,

        /// Allow all permissions for tests
        #[arg(long)]
        allow_all: bool,

        /// Run tests matching the pattern
        #[arg(long, value_name = "PATTERN")]
        filter: Option<String>,

        /// Disable parallel test execution
        #[arg(long)]
        no_parallel: bool,

        /// Show ignored tests
        #[arg(long)]
        ignored: bool,
    },

    /// Bundle JavaScript/TypeScript modules
    Bundle {
        /// Entry point file
        #[arg(value_name = "INPUT")]
        input: String,

        /// Output file
        #[arg(short, long, value_name = "OUTPUT")]
        output: PathBuf,

        /// Set import map path
        #[arg(long, value_name = "PATH")]
        import_map: Option<PathBuf>,

        /// Output format (esm, cjs, iife)
        #[arg(long, default_value = "esm")]
        format: String,

        /// Minify output
        #[arg(long)]
        minify: bool,

        /// Source map type (none, inline, external)
        #[arg(long, default_value = "external")]
        source_map: String,
    },

    /// Install and run a script from a URL
    Install {
        /// URL or package name to install
        #[arg(value_name = "NAME")]
        name: String,

        /// Arguments to pass to the script
        #[arg(value_name = "ARGS", trailing_var_arg = true)]
        args: Vec<String>,

        /// Allow all permissions
        #[arg(long)]
        allow_all: bool,

        /// Force reinstallation
        #[arg(long)]
        force: bool,
    },

    /// Cache management
    Cache {
        /// Cache subcommand to execute
        #[command(subcommand)]
        subcommand: CacheCommands,
    },

    /// Compile script to standalone executable
    Compile {
        /// Input script file
        #[arg(value_name = "INPUT")]
        input: String,

        /// Output executable path
        #[arg(short, long, value_name = "OUTPUT")]
        output: PathBuf,

        /// Target architecture
        #[arg(long, value_name = "TARGET")]
        target: Option<String>,

        /// Include source map
        #[arg(long)]
        source_map: bool,
    },

    /// Show module dependency tree
    Info {
        /// Module file or URL to analyze
        #[arg(value_name = "MODULE")]
        module: String,

        /// Show import URLs
        #[arg(long)]
        imports: bool,

        /// Show JSON output
        #[arg(long)]
        json: bool,
    },

    /// Lint JavaScript/TypeScript code
    Lint {
        /// Files or directories to lint
        #[arg(value_name = "FILES")]
        files: Vec<String>,

        /// Fix issues automatically
        #[arg(long)]
        fix: bool,

        /// Show JSON output
        #[arg(long)]
    json: bool,
    },

    /// Type check TypeScript code
    Check {
        /// Files or directories to check
        #[arg(value_name = "FILES")]
        files: Vec<String>,
    },

    /// Documentation generator
    Doc {
        /// Files to generate documentation for
        #[arg(value_name = "FILES")]
        files: Vec<String>,

        /// Output directory
        #[arg(short, long, value_name = "DIR")]
        output: PathBuf,

        /// Serve documentation
        #[arg(long)]
        serve: bool,

        /// Port for documentation server
        #[arg(long, default_value = "8080")]
        port: u16,
    },

    /// Upgrade to latest version
    Upgrade {
        /// Version to upgrade to (defaults to latest)
        #[arg(long, value_name = "VERSION")]
        version: Option<String>,

        /// Use prerelease versions
        #[arg(long)]
        prerelease: bool,
    },

    /// Show completions for a shell
    Completions {
        /// Shell type (bash, elvish, fish, powershell, zsh)
        #[arg(value_name = "SHELL")]
        shell: String,
    },
}

/// Cache subcommands
#[derive(Subcommand, Debug)]
pub enum CacheCommands {
    /// Clear the module cache
    Clear,
    /// Show cache information
    Info,
    /// Prune unused cache entries
    Prune,
}

impl Commands {
    /// Extract permissions from the command
    pub fn permissions(&self) -> Permissions {
        match self {
            Commands::Run {
                allow_all,
                allow_read,
                allow_read_path,
                allow_write,
                allow_write_path,
                allow_net,
                allow_net_domain,
                allow_env,
                allow_env_var,
                allow_run,
                allow_run_command,
                unsafe_no_permissions,
                ..
            } => {
                if *unsafe_no_permissions {
                    // DANGEROUS! Disable all permission checks
                    return Permissions::allow_all();
                }

                if *allow_all {
                    return Permissions::allow_all();
                }

                let mut perms = Permissions::default();

                // Read permissions
                if *allow_read {
                    perms.read.grant_all();
                } else if let Some(paths) = allow_read_path {
                    perms.read.grant_paths(paths.clone());
                }

                // Write permissions
                if *allow_write {
                    perms.write.grant_all();
                } else if let Some(paths) = allow_write_path {
                    perms.write.grant_paths(paths.clone());
                }

                // Network permissions
                if *allow_net {
                    perms.net.grant_all();
                } else if let Some(domains) = allow_net_domain {
                    perms.net.grant_addresses(domains.clone());
                }

                // Environment permissions
                if *allow_env {
                    perms.env.grant_all();
                } else if let Some(vars) = allow_env_var {
                    perms.env.grant_vars(vars.clone());
                }

                // Run permissions
                if *allow_run {
                    perms.run.grant_all();
                } else if let Some(commands) = allow_run_command {
                    perms.run.grant_commands(commands.clone());
                }

                perms
            }
            Commands::Repl { allow_all, .. } => {
                if *allow_all {
                    Permissions::allow_all()
                } else {
                    // REPL gets default permissions for safety
                    Permissions::default()
                }
            }
            Commands::Test { allow_all, .. } => {
                if *allow_all {
                    Permissions::allow_all()
                } else {
                    // Tests typically need some permissions
                    let mut perms = Permissions::default();
                    perms.read.grant_paths(vec![".".to_string()]);
                    perms
                }
            }
            Commands::Install { allow_all, .. } => {
                if *allow_all {
                    Permissions::allow_all()
                } else {
                    let mut perms = Permissions::default();
                    // Installation typically needs network access
                    perms.net.grant_all();
                    perms
                }
            }
            _ => Permissions::default(),
        }
    }

    /// Get script arguments (for `run` command)
    pub fn script_args(&self) -> Option<&[String]> {
        match self {
            Commands::Run { args, .. } => Some(args),
            Commands::Install { args, .. } => Some(args),
            _ => None,
        }
    }

    /// Get the script path (for `run` command)
    pub fn script_path(&self) -> Option<&str> {
        match self {
            Commands::Run { script, .. } => Some(script),
            Commands::Info { module, .. } => Some(module),
            Commands::Check { files, .. } => files.first().map(|s| s.as_str()),
            _ => None,
        }
    }

    /// Check if inspector is enabled
    pub fn inspect_enabled(&self) -> Option<u16> {
        match self {
            Commands::Run {
                inspect,
                inspect_port,
                ..
            } => {
                if *inspect {
                    Some(*inspect_port)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Check if watch mode is enabled
    pub fn watch_mode(&self) -> bool {
        match self {
            Commands::Run { watch, .. } => *watch,
            _ => false,
        }
    }

    /// Get import map path
    pub fn import_map(&self) -> Option<&PathBuf> {
        match self {
            Commands::Run { import_map, .. } => import_map.as_ref(),
            Commands::Bundle { import_map, .. } => import_map.as_ref(),
            _ => None,
        }
    }
}

/// Parse CLI arguments
pub fn parse_args() -> Cli {
    Cli::parse()
}

/// Parse CLI arguments from a string slice (useful for testing)
pub fn parse_args_from<I: IntoIterator<Item = String>>(args: I) -> Result<Cli, clap::Error> {
    Cli::try_parse_from(args)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper function to convert string slices to owned strings for testing
    fn strs(args: &[&str]) -> Vec<String> {
        args.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn test_parse_run_command() {
        let cli = parse_args_from(strs(&[
            "ferrum",
            "run",
            "script.js",
            "--allow-read",
            "--allow-net",
        ]))
        .unwrap();

        assert!(matches!(cli.command, Commands::Run { .. }));
        assert_eq!(cli.command.script_path(), Some("script.js"));
    }

    #[test]
    fn test_parse_run_with_args() {
        let cli = parse_args_from(strs(&["ferrum", "run", "script.js", "arg1", "arg2"])).unwrap();

        let args = cli.command.script_args().unwrap();
        assert_eq!(args, &["arg1", "arg2"]);
    }

    #[test]
    fn test_parse_allow_all() {
        let cli = parse_args_from(strs(&["ferrum", "run", "script.js", "--allow-all"])).unwrap();

        let perms = cli.command.permissions();
        assert!(perms.check_read("/any/path").is_ok());
        assert!(perms.check_write("/any/path").is_ok());
        assert!(perms.check_net("example.com").is_ok());
    }

    #[test]
    fn test_parse_allow_specific() {
        let cli = parse_args_from(strs(&[
            "ferrum",
            "run",
            "script.js",
            "--allow-read-path",
            "/tmp,/home",
            "--allow-net-domain",
            "example.com",
        ]))
        .unwrap();

        let perms = cli.command.permissions();
        assert!(perms.check_read("/tmp/file.txt").is_ok());
        assert!(perms.check_read("/etc/passwd").is_err());
        assert!(perms.check_net("example.com").is_ok());
        assert!(perms.check_net("other.com").is_err());
    }

    #[test]
    fn test_parse_repl_command() {
        let cli = parse_args_from(strs(&["ferrum", "repl"])).unwrap();

        assert!(matches!(cli.command, Commands::Repl { .. }));
    }

    #[test]
    fn test_parse_inspect() {
        let cli = parse_args_from(strs(&["ferrum", "run", "script.js", "--inspect"])).unwrap();

        assert_eq!(cli.command.inspect_enabled(), Some(9229));
    }

    #[test]
    fn test_parse_inspect_custom_port() {
        let cli = parse_args_from(strs(&[
            "ferrum",
            "run",
            "script.js",
            "--inspect",
            "--inspect-port",
            "3000",
        ]))
        .unwrap();

        assert_eq!(cli.command.inspect_enabled(), Some(3000));
    }

    #[test]
    fn test_parse_test_command() {
        let cli = parse_args_from(strs(&["ferrum", "test", "--allow-all"])).unwrap();

        assert!(matches!(cli.command, Commands::Test { .. }));
    }

    #[test]
    fn test_parse_bundle_command() {
        let cli = parse_args_from(strs(&[
            "ferrum",
            "bundle",
            "input.ts",
            "-o",
            "output.js",
        ]))
        .unwrap();

        assert!(matches!(cli.command, Commands::Bundle { .. }));
    }

    #[test]
    fn test_parse_verbose() {
        let cli = parse_args_from(strs(&["ferrum", "-v", "run", "script.js"])).unwrap();

        assert!(cli.verbose);
    }

    #[test]
    fn test_parse_log_level() {
        let cli = parse_args_from(strs(&["ferrum", "--log-level", "debug", "run", "script.js"]))
            .unwrap();

        assert_eq!(cli.log_level, "debug");
    }

    #[test]
    fn test_parse_cache_clear() {
        let cli = parse_args_from(strs(&["ferrum", "cache", "clear"])).unwrap();

        assert!(matches!(
            cli.command,
            Commands::Cache {
                subcommand: CacheCommands::Clear
            }
        ));
    }
}
