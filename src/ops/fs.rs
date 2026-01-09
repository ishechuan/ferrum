//! File System Operations (Ops)
//!
//! This module provides file system operations that can be called from JavaScript.
//! Each operation checks permissions before executing.

use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use thiserror::Error;

use crate::permissions::{Permissions, PermissionError};

/// Errors that can occur during file system operations
#[derive(Error, Debug)]
pub enum FsError {
    /// Underlying IO error from the file system
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    /// Permission denied for the requested operation
    #[error("Permission error: {0}")]
    Permission(#[from] PermissionError),

    /// The provided path is invalid
    #[error("Invalid path: {0}")]
    InvalidPath(String),

    /// The requested path does not exist
    #[error("Path does not exist: {0}")]
    NotFound(String),

    /// A file or directory already exists at the path
    #[error("Already exists: {0}")]
    AlreadyExists(String),

    /// The path exists but is not a directory
    #[error("Not a directory: {0}")]
    NotADirectory(String),

    /// The path exists but is not a file
    #[error("Not a file: {0}")]
    NotAFile(String),
}

/// Result type for file system operations
pub type FsResult<T> = Result<T, FsError>;

/// Read a file and return its contents as a string
pub fn read_text_file(path: &str, permissions: &Permissions) -> FsResult<String> {
    permissions.check_read(path)?;

    let content = fs::read_to_string(path)?;
    Ok(content)
}

/// Read a file and return its contents as bytes
pub fn read_file(path: &str, permissions: &Permissions) -> FsResult<Vec<u8>> {
    permissions.check_read(path)?;

    let content = fs::read(path)?;
    Ok(content)
}

/// Write a string to a file
pub fn write_text_file(path: &str, data: &str, permissions: &Permissions) -> FsResult<()> {
    permissions.check_write(path)?;

    // Ensure parent directory exists
    if let Some(parent) = Path::new(path).parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            fs::create_dir_all(parent)?;
        }
    }

    fs::write(path, data)?;
    Ok(())
}

/// Write bytes to a file
pub fn write_file(path: &str, data: &[u8], permissions: &Permissions) -> FsResult<()> {
    permissions.check_write(path)?;

    // Ensure parent directory exists
    if let Some(parent) = Path::new(path).parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            fs::create_dir_all(parent)?;
        }
    }

    fs::write(path, data)?;
    Ok(())
}

/// Append a string to a file
pub fn append_text_file(path: &str, data: &str, permissions: &Permissions) -> FsResult<()> {
    permissions.check_write(path)?;

    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;

    file.write_all(data.as_bytes())?;
    Ok(())
}

/// Append bytes to a file
pub fn append_file(path: &str, data: &[u8], permissions: &Permissions) -> FsResult<()> {
    permissions.check_write(path)?;

    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;

    file.write_all(data)?;
    Ok(())
}

/// Check if a path exists
pub fn exists(path: &str, permissions: &Permissions) -> FsResult<bool> {
    permissions.check_read(path)?;
    Ok(Path::new(path).exists())
}

/// Get file metadata
#[derive(Debug, Clone, serde::Serialize)]
pub struct FileMetadata {
    /// Whether the path is a regular file
    pub is_file: bool,
    /// Whether the path is a directory
    pub is_directory: bool,
    /// Whether the path is a symbolic link
    pub is_symlink: bool,
    /// Size in bytes
    pub size: u64,
    /// Last modified time as Unix timestamp
    pub modified: Option<u64>,
    /// Last accessed time as Unix timestamp
    pub accessed: Option<u64>,
    /// Creation time as Unix timestamp
    pub created: Option<u64>,
    /// Whether the file is marked as read-only
    pub readonly: bool,
}

/// Get metadata for a file or directory
///
/// # Arguments
/// * `path` - The path to query
/// * `permissions` - The permissions context to check read access
///
/// # Returns
/// File metadata including size, timestamps, and type information
pub fn metadata(path: &str, permissions: &Permissions) -> FsResult<FileMetadata> {
    permissions.check_read(path)?;

    let metadata = fs::metadata(path)?;

    Ok(FileMetadata {
        is_file: metadata.is_file(),
        is_directory: metadata.is_dir(),
        is_symlink: metadata.is_symlink(),
        size: metadata.len(),
        modified: metadata.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_secs()),
        accessed: metadata.accessed().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_secs()),
        created: metadata.created().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_secs()),
        readonly: metadata.permissions().readonly(),
    })
}

/// Create a directory
pub fn create_dir(path: &str, permissions: &Permissions, recursive: bool) -> FsResult<()> {
    permissions.check_write(path)?;

    if recursive {
        fs::create_dir_all(path)?;
    } else {
        fs::create_dir(path)?;
    }

    Ok(())
}

/// Remove a file or directory
pub fn remove(path: &str, permissions: &Permissions, recursive: bool) -> FsResult<()> {
    permissions.check_write(path)?;

    let path_obj = Path::new(path);

    if path_obj.is_dir() {
        if recursive {
            fs::remove_dir_all(path)?;
        } else {
            fs::remove_dir(path)?;
        }
    } else {
        fs::remove_file(path)?;
    }

    Ok(())
}

/// Rename a file or directory
pub fn rename(old_path: &str, new_path: &str, permissions: &Permissions) -> FsResult<()> {
    permissions.check_write(old_path)?;
    permissions.check_write(new_path)?;

    fs::rename(old_path, new_path)?;
    Ok(())
}

/// Copy a file
pub fn copy(src: &str, dest: &str, permissions: &Permissions) -> FsResult<u64> {
    permissions.check_read(src)?;
    permissions.check_write(dest)?;

    let bytes = fs::copy(src, dest)?;
    Ok(bytes)
}

/// List directory entries
#[derive(Debug, Clone, serde::Serialize)]
pub struct DirEntry {
    /// Name of the entry
    pub name: String,
    /// Whether the entry is a file
    pub is_file: bool,
    /// Whether the entry is a directory
    pub is_directory: bool,
    /// Whether the entry is a symbolic link
    pub is_symlink: bool,
}

/// Read directory entries
///
/// # Arguments
/// * `path` - The directory path to read
/// * `permissions` - The permissions context to check read access
///
/// # Returns
/// A vector of directory entries with metadata
pub fn read_dir(path: &str, permissions: &Permissions) -> FsResult<Vec<DirEntry>> {
    permissions.check_read(path)?;

    let entries = fs::read_dir(path)?;

    let mut result = Vec::new();
    for entry in entries {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();
        let file_type = entry.file_type()?;

        result.push(DirEntry {
            name,
            is_file: file_type.is_file(),
            is_directory: file_type.is_dir(),
            is_symlink: file_type.is_symlink(),
        });
    }

    Ok(result)
}

/// Get the current working directory
pub fn cwd() -> FsResult<String> {
    let path = std::env::current_dir()?;
    Ok(path.to_string_lossy().to_string())
}

/// Change the current working directory
pub fn chdir(path: &str, permissions: &Permissions) -> FsResult<()> {
    permissions.check_read(path)?;
    std::env::set_current_dir(path)?;
    Ok(())
}

/// Realpath - resolve symlinks and return the canonical path
pub fn realpath(path: &str, permissions: &Permissions) -> FsResult<String> {
    permissions.check_read(path)?;

    let canonical = fs::canonicalize(path)?;
    Ok(canonical.to_string_lossy().to_string())
}

/// Make a file temporary by renaming with a .tmp suffix
pub fn make_temp(path: &str, permissions: &Permissions) -> FsResult<String> {
    permissions.check_write(path)?;

    let temp_path = format!("{}.tmp", path);
    fs::rename(path, &temp_path)?;

    Ok(temp_path)
}

/// Read a file in chunks (for large files)
pub struct FileReader {
    file: fs::File,
    buffer_size: usize,
}

impl FileReader {
    /// Open a file for chunked reading
    ///
    /// # Arguments
    /// * `path` - The file path to open
    /// * `permissions` - The permissions context to check read access
    /// * `buffer_size` - The size of chunks to read (minimum 1024 bytes)
    pub fn open(path: &str, permissions: &Permissions, buffer_size: usize) -> FsResult<Self> {
        permissions.check_read(path)?;

        let file = fs::File::open(path)?;

        Ok(Self {
            file,
            buffer_size: buffer_size.max(1024),
        })
    }

    /// Read a single chunk from the file
    ///
    /// # Returns
    /// * `Some(Vec<u8>)` - A chunk of data if available
    /// * `None` - End of file reached
    pub fn read_chunk(&mut self) -> FsResult<Option<Vec<u8>>> {
        let mut buffer = vec![0u8; self.buffer_size];
        let n = self.file.read(&mut buffer)?;

        if n == 0 {
            Ok(None)
        } else {
            buffer.truncate(n);
            Ok(Some(buffer))
        }
    }

    /// Read all remaining data from the file
    pub fn read_all(&mut self) -> FsResult<Vec<u8>> {
        let mut buffer = Vec::new();
        self.file.read_to_end(&mut buffer)?;
        Ok(buffer)
    }
}

/// Write a file in chunks (for large files)
pub struct FileWriter {
    file: fs::File,
}

impl FileWriter {
    /// Create or truncate a file for writing
    ///
    /// # Arguments
    /// * `path` - The file path to create
    /// * `permissions` - The permissions context to check write access
    pub fn create(path: &str, permissions: &Permissions) -> FsResult<Self> {
        permissions.check_write(path)?;

        // Ensure parent directory exists
        if let Some(parent) = Path::new(path).parent() {
            if !parent.as_os_str().is_empty() && !parent.exists() {
                fs::create_dir_all(parent)?;
            }
        }

        let file = fs::File::create(path)?;

        Ok(Self { file })
    }

    /// Open a file for appending
    ///
    /// # Arguments
    /// * `path` - The file path to open
    /// * `permissions` - The permissions context to check write access
    pub fn append(path: &str, permissions: &Permissions) -> FsResult<Self> {
        permissions.check_write(path)?;

        let file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;

        Ok(Self { file })
    }

    /// Write data to the file
    pub fn write(&mut self, data: &[u8]) -> FsResult<()> {
        self.file.write_all(data)?;
        Ok(())
    }

    /// Flush buffered data to disk
    pub fn flush(&mut self) -> FsResult<()> {
        self.file.flush()?;
        Ok(())
    }
}

/// Event types for file watching
///
/// Represents the different types of file system events that can be detected.
#[derive(Debug, Clone, PartialEq)]
pub enum FileWatcherEvent {
    /// A file or directory was created
    Create(PathBuf),
    /// A file or directory was modified
    Modify(PathBuf),
    /// A file or directory was removed
    Remove(PathBuf),
    /// A file or directory was renamed (old path, new path)
    Rename(PathBuf, PathBuf),
}

/// Configuration for file watcher
///
/// Controls the behavior of file watching operations.
#[derive(Debug, Clone, Copy)]
pub struct FileWatcherConfig {
    /// Whether to watch directories recursively
    pub recursive: bool,
    /// Optional debounce time in milliseconds to reduce duplicate events
    pub debounce_ms: Option<u64>,
}

impl Default for FileWatcherConfig {
    fn default() -> Self {
        Self {
            recursive: true,
            debounce_ms: None,
        }
    }
}

/// Watch a file or directory for changes
///
/// Uses the `notify` crate to monitor file system events. The watcher will
/// call the provided callback function for each detected event.
///
/// # Arguments
///
/// * `path` - Path to the file or directory to watch
/// * `permissions` - Permission object to verify read access
/// * `callback` - Function to call for each file system event
/// * `config` - Configuration options for the watcher
///
/// # Returns
///
/// Returns a `FileWatcher` that will stop watching when dropped.
///
/// # Errors
///
/// Returns `FsError` if:
/// - Permission is denied for the path
/// - The path does not exist
/// - The watcher cannot be created
///
/// # Example
///
/// ```no_run
/// use ferrum::ops::fs::{FileWatcher, FileWatcherConfig};
/// use ferrum::permissions::Permissions;
///
/// let watcher = FileWatcher::watch(
///     "/path/to/watch",
///     &Permissions::allow_all(),
///     |event| println!("Event: {:?}", event),
///     FileWatcherConfig::default(),
/// ).unwrap();
/// // Watcher runs until dropped
/// ```
pub struct FileWatcher {
    _watcher: notify::RecommendedWatcher,
    // Use Option to allow taking ownership in Drop
    _abort_tx: Option<tokio::sync::oneshot::Sender<()>>,
}

impl FileWatcher {
    /// Start watching a path for changes
    ///
    /// Creates a new file system watcher that monitors the specified path
    /// for events (create, modify, remove, rename).
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the file or directory to watch
    /// * `permissions` - Permission object to verify read access
    /// * `callback` - Function to call for each file system event
    /// * `config` - Configuration options for the watcher
    ///
    /// # Returns
    ///
    /// Returns a `FileWatcher` that will stop watching when dropped.
    ///
    /// # Errors
    ///
    /// Returns `FsError` if:
    /// - Permission is denied for the path
    /// - The path does not exist
    /// - The watcher cannot be created
    #[allow(clippy::type_complexity)]
    pub fn watch(
        path: &str,
        permissions: &Permissions,
        callback: impl Fn(FileWatcherEvent) + Send + 'static,
        config: FileWatcherConfig,
    ) -> FsResult<Self> {
        use notify::Watcher; // Required for watch() method

        permissions.check_read(path)?;

        // Verify the path exists before watching
        let watch_path = PathBuf::from(path);
        if !watch_path.exists() {
            return Err(FsError::NotFound(path.to_string()));
        }

        // Create channel for notify events
        let (watcher_tx, mut watcher_rx) = tokio::sync::mpsc::unbounded_channel();

        // Create the notify watcher
        let mut watcher = notify::recommended_watcher(move |res| {
            if let Ok(event) = res {
                let _ = watcher_tx.send(event);
            }
        })
        .map_err(|e| FsError::Io(io::Error::new(io::ErrorKind::Other, e)))?;

        // Configure recursive mode based on config
        let recursive_mode = if config.recursive {
            notify::RecursiveMode::Recursive
        } else {
            notify::RecursiveMode::NonRecursive
        };

        // Start watching the path
        watcher
            .watch(&watch_path, recursive_mode)
            .map_err(|e| FsError::Io(io::Error::new(io::ErrorKind::Other, e)))?;

        // Spawn task to process events
        let (abort_tx, mut abort_rx) = tokio::sync::oneshot::channel();

        tokio::spawn(async move {
            let mut last_event_time: Option<std::time::Instant> = None;
            let mut pending_event: Option<FileWatcherEvent> = None;

            loop {
                tokio::select! {
                    // Received a file system event
                    Some(event) = watcher_rx.recv() => {
                        let our_event = convert_notify_event(event);

                        // Apply debouncing if configured
                        if let Some(debounce_ms) = config.debounce_ms {
                            let now = std::time::Instant::now();

                            if let Some(last_time) = last_event_time {
                                if now.duration_since(last_time).as_millis() < debounce_ms as u128 {
                                    // Store as pending and continue waiting
                                    pending_event = Some(our_event);
                                    continue;
                                }
                            }

                            // Check if we have a pending event to emit
                            if let Some(pending) = pending_event.take() {
                                callback(pending);
                            }

                            last_event_time = Some(now);
                        }

                        // No debouncing or debounce period passed
                        callback(our_event);
                    }
                    // Abort signal received
                    _ = &mut abort_rx => {
                        // Shutdown requested
                        break;
                    }
                }
            }
        });

        Ok(Self {
            _watcher: watcher,
            _abort_tx: Some(abort_tx),
        })
    }
}

impl Drop for FileWatcher {
    fn drop(&mut self) {
        // Send shutdown signal to the watcher task
        // Take ownership of the sender from the Option
        if let Some(abort_tx) = self._abort_tx.take() {
            // Try to send the shutdown signal
            // If the receiver has already been dropped, this will fail silently
            let _ = abort_tx.send(());
        }
    }
}

/// Convert a notify Event to our FileWatcherEvent type
fn convert_notify_event(event: notify::Event) -> FileWatcherEvent {
    use notify::EventKind;

    match event.kind {
        EventKind::Create(_) => {
            FileWatcherEvent::Create(event.paths.get(0).cloned().unwrap_or_default())
        }
        EventKind::Modify(_) => {
            FileWatcherEvent::Modify(event.paths.get(0).cloned().unwrap_or_default())
        }
        EventKind::Remove(_) => {
            FileWatcherEvent::Remove(event.paths.get(0).cloned().unwrap_or_default())
        }
        // For rename events, notify provides both old and new paths
        EventKind::Any | EventKind::Other => {
            if event.paths.len() >= 2 {
                FileWatcherEvent::Rename(
                    event.paths[0].clone(),
                    event.paths[1].clone(),
                )
            } else {
                FileWatcherEvent::Modify(event.paths.get(0).cloned().unwrap_or_default())
            }
        }
        // Access events - treat as modify
        EventKind::Access(_) => {
            FileWatcherEvent::Modify(event.paths.get(0).cloned().unwrap_or_default())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn test_perms() -> Permissions {
        Permissions::allow_all()
    }

    #[test]
    fn test_read_write_text_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        let path_str = file_path.to_str().unwrap();

        let content = "Hello, Ferrum!";
        write_text_file(path_str, content, &test_perms()).unwrap();

        let read_content = read_text_file(path_str, &test_perms()).unwrap();
        assert_eq!(read_content, content);
    }

    #[test]
    fn test_read_write_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.bin");
        let path_str = file_path.to_str().unwrap();

        let data = vec![1u8, 2, 3, 4, 5];
        write_file(path_str, &data, &test_perms()).unwrap();

        let read_data = read_file(path_str, &test_perms()).unwrap();
        assert_eq!(read_data, data);
    }

    #[test]
    fn test_append_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("append.txt");
        let path_str = file_path.to_str().unwrap();

        write_text_file(path_str, "Hello, ", &test_perms()).unwrap();
        append_text_file(path_str, "Ferrum!", &test_perms()).unwrap();

        let content = read_text_file(path_str, &test_perms()).unwrap();
        assert_eq!(content, "Hello, Ferrum!");
    }

    #[test]
    fn test_exists() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("exists.txt");
        let path_str = file_path.to_str().unwrap();

        assert!(!exists(path_str, &test_perms()).unwrap());

        write_text_file(path_str, "test", &test_perms()).unwrap();
        assert!(exists(path_str, &test_perms()).unwrap());
    }

    #[test]
    fn test_metadata() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("meta.txt");
        let path_str = file_path.to_str().unwrap();

        write_text_file(path_str, "test content", &test_perms()).unwrap();

        let meta = metadata(path_str, &test_perms()).unwrap();
        assert!(meta.is_file);
        assert!(!meta.is_directory);
        assert_eq!(meta.size, 12); // "test content" = 12 bytes
    }

    #[test]
    fn test_create_dir() {
        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path().join("test_dir");
        let path_str = dir_path.to_str().unwrap();

        create_dir(path_str, &test_perms(), false).unwrap();
        assert!(exists(path_str, &test_perms()).unwrap());

        let meta = metadata(path_str, &test_perms()).unwrap();
        assert!(meta.is_directory);
    }

    #[test]
    fn test_create_dir_recursive() {
        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path().join("parent").join("child").join("grandchild");
        let path_str = dir_path.to_str().unwrap();

        create_dir(path_str, &test_perms(), true).unwrap();
        assert!(exists(path_str, &test_perms()).unwrap());
    }

    #[test]
    fn test_read_dir() {
        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path().join("test_read");
        let dir_str = dir_path.to_str().unwrap();

        create_dir(dir_str, &test_perms(), true).unwrap();

        write_text_file(
            &dir_path.join("file1.txt").to_str().unwrap(),
            "test1",
            &test_perms(),
        ).unwrap();
        write_text_file(
            &dir_path.join("file2.txt").to_str().unwrap(),
            "test2",
            &test_perms(),
        ).unwrap();

        let entries = read_dir(dir_str, &test_perms()).unwrap();
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn test_rename() {
        let temp_dir = TempDir::new().unwrap();
        let old_path = temp_dir.path().join("old.txt");
        let new_path = temp_dir.path().join("new.txt");

        write_text_file(old_path.to_str().unwrap(), "test", &test_perms()).unwrap();
        rename(old_path.to_str().unwrap(), new_path.to_str().unwrap(), &test_perms()).unwrap();

        assert!(!exists(old_path.to_str().unwrap(), &test_perms()).unwrap());
        assert!(exists(new_path.to_str().unwrap(), &test_perms()).unwrap());
    }

    #[test]
    fn test_copy() {
        let temp_dir = TempDir::new().unwrap();
        let src_path = temp_dir.path().join("src.txt");
        let dst_path = temp_dir.path().join("dst.txt");

        write_text_file(src_path.to_str().unwrap(), "test", &test_perms()).unwrap();
        let bytes = copy(src_path.to_str().unwrap(), dst_path.to_str().unwrap(), &test_perms()).unwrap();

        assert_eq!(bytes, 4);
        assert!(exists(dst_path.to_str().unwrap(), &test_perms()).unwrap());
    }

    #[test]
    fn test_remove() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("remove.txt");
        let path_str = file_path.to_str().unwrap();

        write_text_file(path_str, "test", &test_perms()).unwrap();
        assert!(exists(path_str, &test_perms()).unwrap());

        remove(path_str, &test_perms(), false).unwrap();
        assert!(!exists(path_str, &test_perms()).unwrap());
    }

    #[test]
    fn test_permission_denied_read() {
        let perms = Permissions::default(); // No permissions

        let result = read_text_file("/etc/passwd", &perms);
        assert!(matches!(result, Err(FsError::Permission(_))));
    }

    #[test]
    fn test_permission_denied_write() {
        let perms = Permissions::default(); // No permissions

        let result = write_text_file("/tmp/test.txt", "test", &perms);
        assert!(matches!(result, Err(FsError::Permission(_))));
    }

    #[test]
    fn test_file_reader() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("chunked.txt");
        let path_str = file_path.to_str().unwrap();

        let data = b"Hello, Ferrum!";
        write_file(path_str, data, &test_perms()).unwrap();

        let mut reader = FileReader::open(path_str, &test_perms(), 5).unwrap();
        let mut collected = Vec::new();

        while let Some(chunk) = reader.read_chunk().unwrap() {
            collected.extend_from_slice(&chunk);
        }

        assert_eq!(collected, data);
    }

    #[test]
    fn test_file_writer() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("write.txt");
        let path_str = file_path.to_str().unwrap();

        let mut writer = FileWriter::create(path_str, &test_perms()).unwrap();
        writer.write(b"Hello, ").unwrap();
        writer.write(b"Ferrum!").unwrap();
        writer.flush().unwrap();

        let content = read_text_file(path_str, &test_perms()).unwrap();
        assert_eq!(content, "Hello, Ferrum!");
    }

    #[test]
    fn test_realpath() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        let path_str = file_path.to_str().unwrap();

        write_text_file(path_str, "test", &test_perms()).unwrap();

        let resolved = realpath(path_str, &test_perms()).unwrap();
        assert!(resolved.ends_with("test.txt"));
    }

    // File watcher tests
    #[tokio::test]
    async fn test_file_watcher_create() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        let dir_str = temp_dir.path().to_str().unwrap();

        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

        let tx_clone = tx.clone();
        let _watcher = FileWatcher::watch(
            dir_str,
            &test_perms(),
            move |event| {
                let _ = tx_clone.send(event);
            },
            FileWatcherConfig::default(),
        ).unwrap();

        // Give the watcher a moment to start
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Trigger event
        fs::write(&file_path, "test content").unwrap();

        // Wait for event
        let event = tokio::time::timeout(
            tokio::time::Duration::from_secs(2),
            rx.recv(),
        ).await;

        assert!(event.is_ok());
        let received = event.unwrap();
        assert!(received.is_some());
        assert!(matches!(received, Some(FileWatcherEvent::Create(_))));
    }

    #[tokio::test]
    async fn test_file_watcher_modify() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("modify.txt");
        let dir_str = temp_dir.path().to_str().unwrap();

        // Create the file first
        fs::write(&file_path, "initial").unwrap();

        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

        let tx_clone = tx.clone();
        let _watcher = FileWatcher::watch(
            dir_str,
            &test_perms(),
            move |event| {
                let _ = tx_clone.send(event);
            },
            FileWatcherConfig::default(),
        ).unwrap();

        // Give the watcher a moment to start
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Modify the file
        fs::write(&file_path, "modified content").unwrap();

        // Wait for event (skip any initial create events)
        let event = tokio::time::timeout(
            tokio::time::Duration::from_secs(2),
            async {
                loop {
                    if let Some(e) = rx.recv().await {
                        if matches!(e, FileWatcherEvent::Modify(_)) {
                            return e;
                        }
                    }
                }
            },
        ).await;

        assert!(event.is_ok());
        assert!(matches!(event.unwrap(), FileWatcherEvent::Modify(_)));
    }

    #[tokio::test]
    async fn test_file_watcher_remove() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("remove.txt");
        let dir_str = temp_dir.path().to_str().unwrap();

        // Create the file first
        fs::write(&file_path, "to be removed").unwrap();

        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

        let tx_clone = tx.clone();
        let _watcher = FileWatcher::watch(
            dir_str,
            &test_perms(),
            move |event| {
                let _ = tx_clone.send(event);
            },
            FileWatcherConfig::default(),
        ).unwrap();

        // Give the watcher a moment to start
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Remove the file
        fs::remove_file(&file_path).unwrap();

        // Wait for remove event
        let event = tokio::time::timeout(
            tokio::time::Duration::from_secs(2),
            async {
                loop {
                    if let Some(e) = rx.recv().await {
                        if matches!(e, FileWatcherEvent::Remove(_)) {
                            return e;
                        }
                    }
                }
            },
        ).await;

        assert!(event.is_ok());
        assert!(matches!(event.unwrap(), FileWatcherEvent::Remove(_)));
    }

    #[tokio::test]
    async fn test_file_watcher_permission_denied() {
        let no_perms = Permissions::default();

        let result = FileWatcher::watch(
            "/some/path",
            &no_perms,
            |_event| {},
            FileWatcherConfig::default(),
        );

        assert!(matches!(result, Err(FsError::Permission(_))));
    }

    #[tokio::test]
    async fn test_file_watcher_nonexistent_path() {
        let result = FileWatcher::watch(
            "/nonexistent/path/xyz123",
            &test_perms(),
            |_event| {},
            FileWatcherConfig::default(),
        );

        assert!(matches!(result, Err(FsError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_file_watcher_debounce() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("debounce.txt");
        let dir_str = temp_dir.path().to_str().unwrap();

        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let event_count = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let event_count_clone = event_count.clone();

        let _watcher = FileWatcher::watch(
            dir_str,
            &test_perms(),
            move |event| {
                event_count_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                let _ = tx.send(event);
            },
            FileWatcherConfig {
                recursive: true,
                debounce_ms: Some(200), // 200ms debounce
            },
        ).unwrap();

        // Give the watcher a moment to start
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Trigger multiple rapid events
        for i in 0..5 {
            fs::write(&file_path, format!("content {}", i)).unwrap();
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }

        // Wait for debounce period and a bit more
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        // With debouncing, we should receive fewer events than writes
        // (exact count depends on timing, but should be less than 5)
        let count = event_count.load(std::sync::atomic::Ordering::SeqCst);
        assert!(count < 5, "Expected fewer events with debouncing, got {}", count);
    }
}
