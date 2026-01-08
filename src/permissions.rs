//! Permission system for secure runtime operations.
//!
//! This module implements a security model similar to Deno's permission system.
//! All operations that access sensitive resources (file system, network, etc.)
//! require explicit permission grants.

use std::collections::HashSet;
use thiserror::Error;

/// Errors that can occur when checking permissions
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum PermissionError {
    /// Permission denied for the requested operation
    #[error("Permission denied: {0}")]
    Denied(String),

    /// Invalid path specified for permission
    #[error("Invalid permission path: {0}")]
    InvalidPath(String),

    /// Invalid network address specified for permission
    #[error("Invalid net address: {0}")]
    InvalidAddress(String),
}

/// Result type for permission checks
pub type PermissionResult<T> = Result<T, PermissionError>;

/// Represents the permission state for a resource
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PermissionState {
    /// Permission granted with no restrictions
    Granted,
    /// Permission granted for specific paths/addresses
    GrantedPartial {
        /// Set of paths or addresses that are allowed
        paths: HashSet<String>
    },
    /// Permission denied
    Denied,
    /// Permission prompt pending (for future interactive use)
    PromptPending,
}

impl Default for PermissionState {
    fn default() -> Self {
        Self::Denied
    }
}

impl PermissionState {
    /// Check if access is granted for a specific path
    pub fn is_granted(&self, check: Option<&str>) -> bool {
        match self {
            PermissionState::Granted => true,
            PermissionState::Denied => false,
            PermissionState::PromptPending => false,
            PermissionState::GrantedPartial { paths } => {
                if let Some(check_path) = check {
                    paths.contains(&check_path.to_string())
                        || paths.iter().any(|p| check_path.starts_with(p))
                } else {
                    false
                }
            }
        }
    }
}

/// File system read permission
#[derive(Debug, Clone, Default)]
pub struct ReadPermission {
    state: PermissionState,
}

impl ReadPermission {
    /// Create a new read permission (default: denied)
    pub fn new() -> Self {
        Self::default()
    }

    /// Grant read access to all paths
    pub fn grant_all(&mut self) {
        self.state = PermissionState::Granted;
    }

    /// Grant read access to specific paths
    pub fn grant_paths(&mut self, paths: Vec<String>) {
        let path_set: HashSet<String> = paths.into_iter().collect();
        self.state = PermissionState::GrantedPartial { paths: path_set };
    }

    /// Check if read access is granted for a path
    pub fn check(&self, path: &str) -> PermissionResult<()> {
        if self.state.is_granted(Some(path)) {
            Ok(())
        } else {
            Err(PermissionError::Denied(format!(
                "Requires read access to '{}'",
                path
            )))
        }
    }

    /// Query the current permission state
    pub fn query(&self) -> &PermissionState {
        &self.state
    }
}

/// File system write permission
#[derive(Debug, Clone, Default)]
pub struct WritePermission {
    state: PermissionState,
}

impl WritePermission {
    /// Create a new write permission (default: denied)
    pub fn new() -> Self {
        Self::default()
    }

    /// Grant write access to all paths
    pub fn grant_all(&mut self) {
        self.state = PermissionState::Granted;
    }

    /// Grant write access to specific paths
    pub fn grant_paths(&mut self, paths: Vec<String>) {
        let path_set: HashSet<String> = paths.into_iter().collect();
        self.state = PermissionState::GrantedPartial { paths: path_set };
    }

    /// Check if write access is granted for a path
    pub fn check(&self, path: &str) -> PermissionResult<()> {
        if self.state.is_granted(Some(path)) {
            Ok(())
        } else {
            Err(PermissionError::Denied(format!(
                "Requires write access to '{}'",
                path
            )))
        }
    }

    /// Query the current permission state
    pub fn query(&self) -> &PermissionState {
        &self.state
    }
}

/// Network permission
#[derive(Debug, Clone, Default)]
pub struct NetPermission {
    state: PermissionState,
}

impl NetPermission {
    /// Create a new network permission (default: denied)
    pub fn new() -> Self {
        Self::default()
    }

    /// Grant network access to all addresses
    pub fn grant_all(&mut self) {
        self.state = PermissionState::Granted;
    }

    /// Grant network access to specific domains/addresses
    pub fn grant_addresses(&mut self, addresses: Vec<String>) {
        let addr_set: HashSet<String> = addresses.into_iter().collect();
        self.state = PermissionState::GrantedPartial { paths: addr_set };
    }

    /// Check if network access is granted for an address
    pub fn check(&self, address: &str) -> PermissionResult<()> {
        if self.state.is_granted(Some(address)) {
            Ok(())
        } else {
            Err(PermissionError::Denied(format!(
                "Requires network access to '{}'",
                address
            )))
        }
    }

    /// Query the current permission state
    pub fn query(&self) -> &PermissionState {
        &self.state
    }
}

/// Environment variable access permission
#[derive(Debug, Clone, Default)]
pub struct EnvPermission {
    state: PermissionState,
}

impl EnvPermission {
    /// Create a new env permission (default: denied)
    pub fn new() -> Self {
        Self::default()
    }

    /// Grant access to all environment variables
    pub fn grant_all(&mut self) {
        self.state = PermissionState::Granted;
    }

    /// Grant access to specific environment variables
    pub fn grant_vars(&mut self, vars: Vec<String>) {
        let var_set: HashSet<String> = vars.into_iter().collect();
        self.state = PermissionState::GrantedPartial { paths: var_set };
    }

    /// Check if access is granted for an environment variable
    pub fn check(&self, var: &str) -> PermissionResult<()> {
        if self.state.is_granted(Some(var)) {
            Ok(())
        } else {
            Err(PermissionError::Denied(format!(
                "Requires access to environment variable '{}'",
                var
            )))
        }
    }

    /// Query the current permission state
    pub fn query(&self) -> &PermissionState {
        &self.state
    }
}

/// Subprocess execution permission
#[derive(Debug, Clone, Default)]
pub struct RunPermission {
    state: PermissionState,
}

impl RunPermission {
    /// Create a new run permission (default: denied)
    pub fn new() -> Self {
        Self::default()
    }

    /// Grant permission to run all commands
    pub fn grant_all(&mut self) {
        self.state = PermissionState::Granted;
    }

    /// Grant permission to run specific commands
    pub fn grant_commands(&mut self, commands: Vec<String>) {
        let cmd_set: HashSet<String> = commands.into_iter().collect();
        self.state = PermissionState::GrantedPartial { paths: cmd_set };
    }

    /// Check if permission is granted for a command
    pub fn check(&self, command: &str) -> PermissionResult<()> {
        if self.state.is_granted(Some(command)) {
            Ok(())
        } else {
            Err(PermissionError::Denied(format!(
                "Requires permission to run '{}'",
                command
            )))
        }
    }

    /// Query the current permission state
    pub fn query(&self) -> &PermissionState {
        &self.state
    }
}

/// Complete set of permissions for the runtime
#[derive(Debug, Clone)]
pub struct Permissions {
    /// Read file system permission
    pub read: ReadPermission,
    /// Write file system permission
    pub write: WritePermission,
    /// Network permission
    pub net: NetPermission,
    /// Environment variable permission
    pub env: EnvPermission,
    /// Subprocess permission
    pub run: RunPermission,
}

impl Default for Permissions {
    fn default() -> Self {
        Self {
            read: ReadPermission::new(),
            write: WritePermission::new(),
            net: NetPermission::new(),
            env: EnvPermission::new(),
            run: RunPermission::new(),
        }
    }
}

impl Permissions {
    /// Create a new permissions set with all permissions granted
    pub fn allow_all() -> Self {
        let mut perms = Self::default();
        perms.read.grant_all();
        perms.write.grant_all();
        perms.net.grant_all();
        perms.env.grant_all();
        perms.run.grant_all();
        perms
    }

    /// Helper method to check read permission
    pub fn check_read(&self, path: &str) -> PermissionResult<()> {
        self.read.check(path)
    }

    /// Helper method to check write permission
    pub fn check_write(&self, path: &str) -> PermissionResult<()> {
        self.write.check(path)
    }

    /// Helper method to check network permission
    pub fn check_net(&self, address: &str) -> PermissionResult<()> {
        self.net.check(address)
    }

    /// Helper method to check env permission
    pub fn check_env(&self, var: &str) -> PermissionResult<()> {
        self.env.check(var)
    }

    /// Helper method to check run permission
    pub fn check_run(&self, command: &str) -> PermissionResult<()> {
        self.run.check(command)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_permission_denied_by_default() {
        let perm = ReadPermission::new();
        assert!(matches!(
            perm.check("/some/path"),
            Err(PermissionError::Denied(_))
        ));
    }

    #[test]
    fn test_read_permission_grant_all() {
        let mut perm = ReadPermission::new();
        perm.grant_all();
        assert!(perm.check("/any/path").is_ok());
    }

    #[test]
    fn test_read_permission_grant_specific() {
        let mut perm = ReadPermission::new();
        perm.grant_paths(vec!["/tmp".to_string(), "/home/user".to_string()]);

        assert!(perm.check("/tmp/file.txt").is_ok());
        assert!(perm.check("/home/user/docs").is_ok());
        assert!(perm.check("/etc/passwd").is_err());
    }

    #[test]
    fn test_write_permission_denied_by_default() {
        let perm = WritePermission::new();
        assert!(matches!(
            perm.check("/some/path"),
            Err(PermissionError::Denied(_))
        ));
    }

    #[test]
    fn test_net_permission_denied_by_default() {
        let perm = NetPermission::new();
        assert!(matches!(
            perm.check("example.com"),
            Err(PermissionError::Denied(_))
        ));
    }

    #[test]
    fn test_net_permission_grant_specific() {
        let mut perm = NetPermission::new();
        perm.grant_addresses(vec!["example.com".to_string(), "api.test.com".to_string()]);

        assert!(perm.check("example.com").is_ok());
        assert!(perm.check("api.test.com").is_ok());
        assert!(perm.check("other.com").is_err());
    }

    #[test]
    fn test_env_permission_denied_by_default() {
        let perm = EnvPermission::new();
        assert!(matches!(
            perm.check("HOME"),
            Err(PermissionError::Denied(_))
        ));
    }

    #[test]
    fn test_run_permission_denied_by_default() {
        let perm = RunPermission::new();
        assert!(matches!(
            perm.check("ls"),
            Err(PermissionError::Denied(_))
        ));
    }

    #[test]
    fn test_permissions_allow_all() {
        let perms = Permissions::allow_all();
        assert!(perms.check_read("/any/path").is_ok());
        assert!(perms.check_write("/any/path").is_ok());
        assert!(perms.check_net("any.com").is_ok());
        assert!(perms.check_env("ANY_VAR").is_ok());
        assert!(perms.check_run("any-command").is_ok());
    }

    #[test]
    fn test_permission_state_granted() {
        let state = PermissionState::Granted;
        assert!(state.is_granted(None));
        assert!(state.is_granted(Some("/any/path")));
    }

    #[test]
    fn test_permission_state_denied() {
        let state = PermissionState::Denied;
        assert!(!state.is_granted(None));
        assert!(!state.is_granted(Some("/any/path")));
    }

    #[test]
    fn test_permission_state_partial() {
        let mut paths = HashSet::new();
        paths.insert("/tmp".to_string());
        paths.insert("/home".to_string());

        let state = PermissionState::GrantedPartial { paths };
        assert!(state.is_granted(Some("/tmp/file.txt")));
        assert!(state.is_granted(Some("/home/user")));
        assert!(!state.is_granted(Some("/etc/passwd")));
    }

    #[test]
    fn test_permissions_default_all_denied() {
        let perms = Permissions::default();
        assert!(perms.check_read("/any").is_err());
        assert!(perms.check_write("/any").is_err());
        assert!(perms.check_net("any.com").is_err());
        assert!(perms.check_env("ANY").is_err());
        assert!(perms.check_run("any").is_err());
    }
}
