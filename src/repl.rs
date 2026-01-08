//! Read-Eval-Print Loop (REPL)
//!
//! This module provides an interactive JavaScript shell for experimentation
//! and quick testing.

use std::io::{self, Write};
use std::rc::Rc;
use std::cell::RefCell;

use crate::permissions::Permissions;
use crate::runtime::{JsRuntime, RuntimeConfig, RuntimeError};

/// Result type for REPL operations
pub type ReplResult<T> = Result<T, ReplError>;

/// Errors that can occur during REPL operations
#[derive(thiserror::Error, Debug)]
pub enum ReplError {
    /// JavaScript runtime error
    #[error("Runtime error: {0}")]
    Runtime(RuntimeError),

    /// Input reading or parsing error
    #[error("Input error: {0}")]
    Input(String),

    /// Empty input provided (no code to execute)
    #[error("Empty input")]
    EmptyInput,
}

impl From<RuntimeError> for ReplError {
    fn from(err: RuntimeError) -> Self {
        ReplError::Runtime(err)
    }
}

/// REPL editor configuration
#[derive(Debug, Clone)]
pub struct ReplConfig {
    /// Prompt string
    pub prompt: String,
    /// Continuation prompt for multi-line input
    pub continuation_prompt: String,
    /// Whether to enable syntax highlighting
    pub highlight: bool,
    /// Whether to show result values
    pub show_result: bool,
    /// History file path
    pub history_file: Option<String>,
    /// Maximum history size
    pub history_size: usize,
}

impl Default for ReplConfig {
    fn default() -> Self {
        Self {
            prompt: "> ".to_string(),
            continuation_prompt: "... ".to_string(),
            highlight: true,
            show_result: true,
            history_file: None,
            history_size: 1000,
        }
    }
}

/// History entry
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct HistoryEntry {
    input: String,
    output: Option<String>,
    timestamp: u64,
}

/// REPL state
pub struct Repl {
    /// JavaScript runtime
    runtime: JsRuntime,
    /// REPL configuration
    config: ReplConfig,
    /// Command history
    history: Rc<RefCell<Vec<HistoryEntry>>>,
    /// Multi-line input buffer
    input_buffer: String,
    /// Running state
    running: bool,
}

impl Repl {
    /// Create a new REPL instance
    pub fn new(config: ReplConfig, permissions: Permissions) -> ReplResult<Self> {
        let runtime_config = RuntimeConfig::default();
        let runtime = JsRuntime::new(runtime_config, permissions)?;

        Ok(Self {
            runtime,
            config,
            history: Rc::new(RefCell::new(Vec::new())),
            input_buffer: String::new(),
            running: true,
        })
    }

    /// Start the REPL loop
    pub fn start(&mut self) -> ReplResult<()> {
        self.print_welcome();

        while self.running {
            // Get input
            let input = self.read_input()?;

            // Handle empty input
            if input.trim().is_empty() {
                continue;
            }

            // Handle special commands
            if self.handle_command(&input)? {
                continue;
            }

            // Execute the input
            self.execute(&input)?;
        }

        Ok(())
    }

    /// Print welcome message
    fn print_welcome(&self) {
        println!("Ferrum REPL v{}", env!("CARGO_PKG_VERSION"));
        println!("Type '.help' for available commands, '.exit' to quit");
        println!();
    }

    /// Read input from the user
    fn read_input(&self) -> ReplResult<String> {
        let prompt = if self.input_buffer.is_empty() {
            &self.config.prompt
        } else {
            &self.config.continuation_prompt
        };

        print!("{}", prompt);
        io::stdout().flush()
            .map_err(|e| ReplError::Input(format!("Failed to flush stdout: {}", e)))?;

        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .map_err(|e| ReplError::Input(format!("Failed to read input: {}", e)))?;

        Ok(input)
    }

    /// Handle special REPL commands
    ///
    /// Returns true if the input was a command (and shouldn't be executed as JS)
    fn handle_command(&mut self, input: &str) -> ReplResult<bool> {
        let trimmed = input.trim();

        if !trimmed.starts_with('.') {
            return Ok(false);
        }

        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        let command = parts.get(0).map(|s| &s[1..]).unwrap_or("");

        match command {
            "exit" | "quit" => {
                self.running = false;
                println!("Goodbye!");
            }
            "help" => {
                self.print_help();
            }
            "clear" => {
                // Clear screen
                print!("\x1b[2J\x1b[H");
                io::stdout().flush().ok();
            }
            "history" => {
                self.print_history();
            }
            "version" => {
                println!("Ferrum v{}", env!("CARGO_PKG_VERSION"));
            }
            "load" => {
                if parts.len() < 2 {
                    println!("Usage: .load <file>");
                } else {
                    self.load_file(parts[1])?;
                }
            }
            "save" => {
                if parts.len() < 2 {
                    println!("Usage: .save <file>");
                } else {
                    self.save_history(parts[1])?;
                }
            }
            "reset" => {
                self.input_buffer.clear();
                println!("Context reset");
            }
            "permissions" => {
                self.print_permissions();
            }
            _ => {
                println!("Unknown command: .{}", command);
                println!("Type .help for available commands");
            }
        }

        Ok(true)
    }

    /// Execute JavaScript code
    fn execute(&mut self, input: &str) -> ReplResult<()> {
        // Add to input buffer
        self.input_buffer.push_str(input);
        self.input_buffer.push('\n');

        // Check if the input is complete
        if !self.is_complete_input(&self.input_buffer) {
            return Ok(());
        }

        let code = self.input_buffer.clone();
        self.input_buffer.clear();

        // Execute the code
        let start = std::time::Instant::now();
        let result = self.runtime.execute(&code, Some("<repl>"));
        let duration = start.elapsed();

        match result {
            Ok(output) => {
                if self.config.show_result && !output.is_empty() {
                    println!("{}", output);
                }

                // Add to history
                let entry = HistoryEntry {
                    input: code,
                    output: Some(output),
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                };
                self.history.borrow_mut().push(entry);
            }
            Err(e) => {
                eprintln!("Error: {}", e);
            }
        }

        // Print execution time if it took longer than 100ms
        if duration.as_millis() > 100 {
            eprintln!("(took {}ms)", duration.as_millis());
        }

        Ok(())
    }

    /// Check if the input is syntactically complete
    fn is_complete_input(&self, input: &str) -> bool {
        let trimmed = input.trim();

        // Simple check: count braces/parens
        let mut brace_count = 0;
        let mut paren_count = 0;
        let mut bracket_count = 0;

        for ch in trimmed.chars() {
            match ch {
                '{' => brace_count += 1,
                '}' => brace_count -= 1,
                '(' => paren_count += 1,
                ')' => paren_count -= 1,
                '[' => bracket_count += 1,
                ']' => bracket_count -= 1,
                _ => {}
            }
        }

        brace_count == 0 && paren_count == 0 && bracket_count == 0
    }

    /// Print help information
    fn print_help(&self) {
        println!("Available commands:");
        println!("  .exit, .quit    Exit the REPL");
        println!("  .help           Show this help message");
        println!("  .clear          Clear the screen");
        println!("  .history        Show command history");
        println!("  .version        Show version information");
        println!("  .load <file>    Load and execute a file");
        println!("  .save <file>    Save history to a file");
        println!("  .reset          Reset the context (clear variables)");
        println!("  .permissions    Show current permissions");
        println!();
        println!("Keyboard shortcuts:");
        println!("  Ctrl+C          Cancel current input or exit");
        println!("  Ctrl+D          Exit the REPL");
        println!();
    }

    /// Print command history
    fn print_history(&self) {
        let history = self.history.borrow();

        if history.is_empty() {
            println!("No history yet");
            return;
        }

        for (i, entry) in history.iter().enumerate() {
            println!("  {}: {}", i + 1, entry.input.lines().next().unwrap_or(""));
        }
    }

    /// Load and execute a file
    fn load_file(&mut self, path: &str) -> ReplResult<()> {
        match self.runtime.execute_file(path) {
            Ok(output) => {
                if !output.is_empty() {
                    println!("{}", output);
                }
                println!("Loaded: {}", path);
                Ok(())
            }
            Err(e) => {
                eprintln!("Error loading file: {}", e);
                Err(ReplError::Runtime(e))
            }
        }
    }

    /// Save history to a file
    fn save_history(&self, path: &str) -> ReplResult<()> {
        let history = self.history.borrow();

        let mut content = String::new();
        for entry in history.iter() {
            content.push_str(&entry.input);
            content.push_str("\n");
        }

        std::fs::write(path, content)
            .map_err(|e| ReplError::Input(format!("Failed to save history: {}", e)))?;

        println!("History saved to: {}", path);
        Ok(())
    }

    /// Print current permissions
    fn print_permissions(&self) {
        let perms = self.runtime.permissions();

        println!("Current permissions:");
        println!("  Read:    {:?}", perms.read.query());
        println!("  Write:   {:?}", perms.write.query());
        println!("  Network: {:?}", perms.net.query());
        println!("  Env:     {:?}", perms.env.query());
        println!("  Run:     {:?}", perms.run.query());
    }

    /// Get the runtime for direct manipulation
    pub fn runtime(&mut self) -> &mut JsRuntime {
        &mut self.runtime
    }
}

/// Start a REPL with default configuration
pub fn start_repl(permissions: Permissions) -> ReplResult<()> {
    let config = ReplConfig::default();
    let mut repl = Repl::new(config, permissions)?;
    repl.start()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::init_v8_platform;
    use std::sync::Once;

    static INIT: Once = Once::new();

    /// Initialize V8 platform for REPL tests (thread-safe, one-time initialization)
    fn init_v8_for_repl_tests() {
        INIT.call_once(|| {
            init_v8_platform();
        });
    }

    #[test]
    fn test_repl_config_default() {
        let config = ReplConfig::default();
        assert_eq!(config.prompt, "> ");
        assert_eq!(config.continuation_prompt, "... ");
        assert!(config.highlight);
        assert!(config.show_result);
    }

    #[test]
    fn test_repl_creation() {
        init_v8_for_repl_tests();
        let perms = Permissions::allow_all();
        let config = ReplConfig::default();

        let repl = Repl::new(config, perms);
        assert!(repl.is_ok());
    }

    #[test]
    fn test_is_complete_input() {
        init_v8_for_repl_tests();
        let perms = Permissions::allow_all();
        let config = ReplConfig::default();

        let repl = Repl::new(config, perms).unwrap();
        // Complete expressions
        assert!(repl.is_complete_input("1 + 1"));
        assert!(repl.is_complete_input("const x = 5;"));
        assert!(repl.is_complete_input("function foo() { return 42; }"));

        // Incomplete expressions
        assert!(!repl.is_complete_input("function foo() {"));
        assert!(!repl.is_complete_input("const x = {"));
        assert!(!repl.is_complete_input("[1, 2,"));
    }

    #[test]
    fn test_handle_command_exit() {
        init_v8_for_repl_tests();
        let perms = Permissions::allow_all();
        let config = ReplConfig::default();

        let mut repl = Repl::new(config, perms).unwrap();
        assert!(repl.running);

        let _ = repl.handle_command(".exit");

        assert!(!repl.running);
    }

    #[test]
    fn test_handle_command_help() {
        init_v8_for_repl_tests();
        let perms = Permissions::allow_all();
        let config = ReplConfig::default();

        let mut repl = Repl::new(config, perms).unwrap();
        let result = repl.handle_command(".help");
        assert!(result.is_ok());
        assert!(result.unwrap()); // Should return true (was a command)
    }

    #[test]
    fn test_handle_command_not_command() {
        init_v8_for_repl_tests();
        let perms = Permissions::allow_all();
        let config = ReplConfig::default();

        let mut repl = Repl::new(config, perms).unwrap();
        let result = repl.handle_command("console.log('test')");
        assert!(result.is_ok());
        assert!(!result.unwrap()); // Should return false (not a command)
    }

    #[test]
    fn test_history_entry() {
        let entry = HistoryEntry {
            input: "1 + 1".to_string(),
            output: Some("2".to_string()),
            timestamp: 12345,
        };

        assert_eq!(entry.input, "1 + 1");
        assert_eq!(entry.output, Some("2".to_string()));
        assert_eq!(entry.timestamp, 12345);
    }
}
