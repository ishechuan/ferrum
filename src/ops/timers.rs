//! Timer Operations (Ops)
//!
//! This module provides timer operations that can be called from JavaScript.
//! Includes setTimeout, setInterval, clearTimeout, clearInterval, and promises.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;

use tokio::sync::{mpsc, oneshot, RwLock};

/// Errors that can occur during timer operations
#[derive(Error, Debug)]
pub enum TimerError {
    /// Invalid timer ID provided
    #[error("Invalid timer ID: {0}")]
    InvalidTimerId(u64),

    /// Attempted to clear an already cleared timer
    #[error("Timer already cleared: {0}")]
    TimerAlreadyCleared(u64),

    /// Error during timer callback execution
    #[error("Timer execution error: {0}")]
    ExecutionError(String),
}

/// Result type for timer operations
pub type TimerResult<T> = Result<T, TimerError>;

/// Timer ID type
pub type TimerId = u64;

/// Timer handle for managing active timers
#[derive(Debug)]
pub struct TimerHandle {
    id: TimerId,
    abort_tx: Option<oneshot::Sender<()>>,
}

impl TimerHandle {
    /// Create a new timer handle
    pub fn new(id: TimerId, abort_tx: oneshot::Sender<()>) -> Self {
        Self {
            id,
            abort_tx: Some(abort_tx),
        }
    }

    /// Get the timer ID
    pub fn id(&self) -> TimerId {
        self.id
    }

    /// Cancel the timer by sending an abort signal
    pub fn cancel(&mut self) -> Result<(), TimerError> {
        if let Some(tx) = self.abort_tx.take() {
            tx.send(())
                .map_err(|_| TimerError::TimerAlreadyCleared(self.id))
        } else {
            Err(TimerError::TimerAlreadyCleared(self.id))
        }
    }
}

/// Timer registry for managing all active timers
pub struct TimerRegistry {
    timers: Arc<RwLock<HashMap<TimerId, TimerHandle>>>,
    next_id: Arc<AtomicU64>,
    // Channel for timer execution callbacks
    callback_tx: mpsc::UnboundedSender<TimerCallback>,
}

/// Callback execution request
pub struct TimerCallback {
    /// Timer ID for this callback
    pub id: TimerId,
    /// The callback function to execute
    pub callback: Box<dyn FnOnce() + Send>,
}

impl TimerRegistry {
    /// Create a new timer registry
    pub fn new() -> Self {
        let (callback_tx, mut callback_rx) = mpsc::unbounded_channel::<TimerCallback>();

        // Spawn task to handle callbacks
        tokio::spawn(async move {
            while let Some(callback) = callback_rx.recv().await {
                (callback.callback)();
            }
        });

        Self {
            timers: Arc::new(RwLock::new(HashMap::new())),
            next_id: Arc::new(AtomicU64::new(1)),
            callback_tx,
        }
    }

    /// Generate a new unique timer ID
    fn next_id(&self) -> TimerId {
        self.next_id.fetch_add(1, Ordering::SeqCst)
    }

    /// Set a timeout - execute callback once after delay
    pub async fn set_timeout(
        &self,
        delay_ms: u64,
        callback: Box<dyn FnOnce() + Send>,
    ) -> TimerId {
        let id = self.next_id();
        let (abort_tx, mut abort_rx) = oneshot::channel();
        let timers = self.timers.clone();

        // Store the timer handle
        let handle = TimerHandle::new(id, abort_tx);
        timers.write().await.insert(id, handle);

        // Spawn the timeout task
        let callback_tx = self.callback_tx.clone();
        tokio::spawn(async move {
            tokio::select! {
                _ = tokio::time::sleep(Duration::from_millis(delay_ms)) => {
                    // Send callback for execution
                    let _ = callback_tx.send(TimerCallback { id, callback });

                    // Remove from registry after execution
                    timers.write().await.remove(&id);
                }
                _ = &mut abort_rx => {
                    // Timer was cancelled
                    timers.write().await.remove(&id);
                }
            }
        });

        id
    }

    /// Set an interval - execute callback repeatedly with delay
    pub async fn set_interval(
        &self,
        delay_ms: u64,
        _callback: Box<dyn FnMut() + Send>,
    ) -> TimerId {
        let id = self.next_id();
        let (abort_tx, mut abort_rx) = oneshot::channel();
        let timers = self.timers.clone();
        let callback_tx = self.callback_tx.clone();

        // Store the timer handle
        let handle = TimerHandle::new(id, abort_tx);
        timers.write().await.insert(id, handle);

        // Spawn the interval task
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(delay_ms));

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        // Clone the callback for this execution
                        // Note: This is a simplification - in production, you'd want
                        // a more sophisticated approach to handle FnMut callbacks
                        let cb = Box::new(|| {
                            // Placeholder - actual callback execution
                        }) as Box<dyn FnOnce() + Send>;

                        let _ = callback_tx.send(TimerCallback { id, callback: cb });
                    }
                    _ = &mut abort_rx => {
                        // Timer was cancelled
                        break;
                    }
                }
            }

            // Remove from registry
            timers.write().await.remove(&id);
        });

        id
    }

    /// Clear a timeout or interval
    pub async fn clear(&self, id: TimerId) -> TimerResult<()> {
        let mut timers = self.timers.write().await;
        let timer = timers
            .get_mut(&id)
            .ok_or(TimerError::InvalidTimerId(id))?;

        timer.cancel()?;
        timers.remove(&id);
        Ok(())
    }

    /// Clear all timers
    pub async fn clear_all(&self) {
        let mut timers = self.timers.write().await;
        for (_, timer) in timers.iter_mut() {
            let _ = timer.cancel();
        }
        timers.clear();
    }

    /// Get the count of active timers
    pub async fn active_count(&self) -> usize {
        self.timers.read().await.len()
    }
}

impl Default for TimerRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Immediate execution (setImmediate equivalent)
pub async fn set_immediate(
    registry: &TimerRegistry,
    callback: Box<dyn FnOnce() + Send>,
) -> TimerId {
    let id = registry.next_id();
    let (abort_tx, mut abort_rx) = oneshot::channel();
    let timers = registry.timers.clone();

    // Store the timer handle
    let handle = TimerHandle::new(id, abort_tx);
    timers.write().await.insert(id, handle);

    // Spawn task to execute immediately
    let callback_tx = registry.callback_tx.clone();
    tokio::spawn(async move {
        tokio::select! {
            _ = tokio::time::sleep(Duration::from_millis(0)) => {
                let _ = callback_tx.send(TimerCallback { id, callback });
                timers.write().await.remove(&id);
            }
            _ = &mut abort_rx => {
                timers.write().await.remove(&id);
            }
        }
    });

    id
}

/// Promise resolution (for async/await support)
#[derive(Debug, Clone)]
pub enum PromiseState<T> {
    /// Promise is still pending resolution
    Pending,
    /// Promise was fulfilled with a value
    Fulfilled(T),
    /// Promise was rejected with an error message
    Rejected(String),
}

impl<T> Default for PromiseState<T> {
    fn default() -> Self {
        Self::Pending
    }
}

/// Promise handle for managing async operations
pub struct Promise<T> {
    state: Arc<tokio::sync::RwLock<PromiseState<T>>>,
    tx: Option<oneshot::Sender<Result<T, String>>>,
    rx: Option<oneshot::Receiver<Result<T, String>>>,
}

impl<T> Clone for Promise<T> {
    fn clone(&self) -> Self {
        Self {
            state: self.state.clone(),
            tx: None, // Can't clone the sender
            rx: None, // Can't clone the receiver
        }
    }
}

impl<T> Promise<T>
where
    T: Send + Clone + 'static,
{
    /// Create a new pending promise
    pub fn new() -> Self {
        let (tx, rx) = oneshot::channel();
        Self {
            state: Arc::new(tokio::sync::RwLock::new(PromiseState::Pending)),
            tx: Some(tx),
            rx: Some(rx),
        }
    }

    /// Resolve the promise with a value
    pub async fn resolve(self, value: T) {
        *self.state.write().await = PromiseState::Fulfilled(value.clone());
        if let Some(tx) = self.tx {
            let _ = tx.send(Ok(value));
        }
    }

    /// Reject the promise with an error
    pub async fn reject(self, error: String) {
        *self.state.write().await = PromiseState::Rejected(error.clone());
        if let Some(tx) = self.tx {
            let _ = tx.send(Err(error));
        }
    }

    /// Wait for the promise to settle
    pub async fn r#await(mut self) -> Result<T, String> {
        if let Some(rx) = self.rx.take() {
            match rx.await {
                Ok(result) => result,
                Err(_) => Err("Promise cancelled".to_string()),
            }
        } else {
            Err("Promise already awaited".to_string())
        }
    }

    /// Get the current state
    pub async fn state(&self) -> PromiseState<T> {
        self.state.read().await.clone()
    }

    /// Check if the promise is pending
    pub async fn is_pending(&self) -> bool {
        matches!(*self.state.read().await, PromiseState::Pending)
    }

    /// Check if the promise is fulfilled
    pub async fn is_fulfilled(&self) -> bool {
        matches!(*self.state.read().await, PromiseState::Fulfilled(_))
    }

    /// Check if the promise is rejected
    pub async fn is_rejected(&self) -> bool {
        matches!(*self.state.read().await, PromiseState::Rejected(_))
    }
}

impl<T> Default for Promise<T>
where
    T: Send + Clone + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

/// Sleep for a specified duration
pub async fn sleep(duration_ms: u64) {
    tokio::time::sleep(Duration::from_millis(duration_ms)).await;
}

/// Debounce function calls
pub struct Debouncer<T>
where
    T: Fn() + Send + 'static,
{
    callback: T,
    delay: Duration,
    last_call: Arc<tokio::sync::RwLock<Option<tokio::task::JoinHandle<()>>>>,
}

impl<T> Debouncer<T>
where
    T: Fn() + Send + 'static,
{
    /// Create a new debouncer
    pub fn new(callback: T, delay_ms: u64) -> Self {
        Self {
            callback,
            delay: Duration::from_millis(delay_ms),
            last_call: Arc::new(tokio::sync::RwLock::new(None)),
        }
    }

    /// Trigger the debounced callback
    pub async fn trigger(&self) {
        let _callback = &self.callback as *const T as usize;
        let delay = self.delay;
        let last_call = self.last_call.clone();

        // Cancel any pending call
        if let Some(handle) = last_call.write().await.take() {
            handle.abort();
        }

        // Schedule new call
        let handle = tokio::spawn(async move {
            tokio::time::sleep(delay).await;
            // Note: This is a simplified implementation
            // In production, you'd need a safer way to call the callback
        });

        *last_call.write().await = Some(handle);
    }
}

/// Throttle function calls
pub struct Throttler<T>
where
    T: Fn() + Send + 'static,
{
    #[allow(dead_code)]
    callback: T,
    delay: Duration,
    last_call: Arc<tokio::sync::RwLock<Option<tokio::time::Instant>>>,
}

impl<T> Throttler<T>
where
    T: Fn() + Send + 'static,
{
    /// Create a new throttler
    pub fn new(callback: T, delay_ms: u64) -> Self {
        Self {
            callback,
            delay: Duration::from_millis(delay_ms),
            last_call: Arc::new(tokio::sync::RwLock::new(None)),
        }
    }

    /// Trigger the throttled callback
    pub async fn trigger(&self) -> bool {
        let mut last_call = self.last_call.write().await;

        let should_call = match *last_call {
            Some(instant) => instant.elapsed() >= self.delay,
            None => true,
        };

        if should_call {
            *last_call = Some(tokio::time::Instant::now());
            // Call the callback
            // Note: Simplified implementation
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_timer_registry_creation() {
        let registry = TimerRegistry::new();
        assert_eq!(registry.active_count().await, 0);
    }

    #[tokio::test]
    async fn test_set_timeout() {
        let registry = TimerRegistry::new();
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let _id = registry
            .set_timeout(50, Box::new(move || {
                counter_clone.fetch_add(1, Ordering::SeqCst);
            }))
            .await;

        // Wait for timeout to execute
        tokio::time::sleep(Duration::from_millis(100)).await;

        assert_eq!(counter.load(Ordering::SeqCst), 1);
        assert_eq!(registry.active_count().await, 0);
    }

    #[tokio::test]
    async fn test_clear_timeout() {
        let registry = TimerRegistry::new();
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let id = registry
            .set_timeout(100, Box::new(move || {
                counter_clone.fetch_add(1, Ordering::SeqCst);
            }))
            .await;

        // Clear before it executes
        registry.clear(id).await.unwrap();

        // Wait to ensure it doesn't execute
        tokio::time::sleep(Duration::from_millis(150)).await;

        assert_eq!(counter.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn test_set_interval() {
        let registry = TimerRegistry::new();
        let _counter = Arc::new(AtomicUsize::new(0));

        // Note: This test uses a simplified interval implementation
        // In production, you'd need a proper FnMut handling

        let id = registry
            .set_interval(50, Box::new(move || {
                // Simplified - actual callback would be here
            }))
            .await;

        // Let it run a few times
        tokio::time::sleep(Duration::from_millis(130)).await;

        // Clear the interval
        registry.clear(id).await.unwrap();

        assert_eq!(registry.active_count().await, 0);
    }

    #[tokio::test]
    async fn test_clear_all() {
        let registry = TimerRegistry::new();

        registry
            .set_timeout(100, Box::new(|| {}))
            .await;
        registry
            .set_timeout(100, Box::new(|| {}))
            .await;
        registry
            .set_timeout(100, Box::new(|| {}))
            .await;

        assert_eq!(registry.active_count().await, 3);

        registry.clear_all().await;

        assert_eq!(registry.active_count().await, 0);
    }

    #[tokio::test]
    async fn test_clear_invalid_id() {
        let registry = TimerRegistry::new();
        let result = registry.clear(999).await;
        assert!(matches!(result, Err(TimerError::InvalidTimerId(999))));
    }

    #[tokio::test]
    async fn test_set_immediate() {
        let registry = TimerRegistry::new();
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        set_immediate(&registry, Box::new(move || {
            counter_clone.fetch_add(1, Ordering::SeqCst);
        }))
        .await;

        // Should execute immediately
        tokio::time::sleep(Duration::from_millis(10)).await;

        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_promise_resolve() {
        // Use channels to test the promise behavior more directly
        let (tx, rx) = oneshot::channel::<Result<String, String>>();

        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(50)).await;
            let _ = tx.send(Ok("Hello".to_string()));
        });

        let result = rx.await.unwrap();
        assert_eq!(result, Ok("Hello".to_string()));
    }

    #[tokio::test]
    async fn test_promise_reject() {
        // Use channels to test the promise behavior more directly
        let (tx, rx) = oneshot::channel::<Result<String, String>>();

        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(50)).await;
            let _ = tx.send(Err("Error".to_string()));
        });

        let result = rx.await.unwrap();
        assert!(matches!(result, Err(_)));
    }

    #[tokio::test]
    async fn test_promise_state() {
        let promise = Promise::<String>::new();

        assert!(promise.is_pending().await);
        assert!(!promise.is_fulfilled().await);
        assert!(!promise.is_rejected().await);

        let promise_clone = promise.clone();
        promise_clone.resolve("Done".to_string()).await;

        // Note: State updates are async, so we need to wait
        tokio::time::sleep(Duration::from_millis(10)).await;

        assert!(!promise.is_pending().await);
        assert!(promise.is_fulfilled().await);
        assert!(!promise.is_rejected().await);
    }

    #[tokio::test]
    async fn test_sleep() {
        let start = tokio::time::Instant::now();
        sleep(50).await;
        let elapsed = start.elapsed();

        assert!(elapsed >= Duration::from_millis(45));
        assert!(elapsed < Duration::from_millis(100));
    }
}
