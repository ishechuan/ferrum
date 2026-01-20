//! Async Operations Module
//!
//! This module provides the infrastructure for async operations in Ferrum.
//! It bridges Rust async/await with JavaScript Promises through V8.
//!
//! # Architecture
//!
//! The async system consists of:
//! - `PendingOp`: Represents an in-flight async operation
//! - `OpState`: Shared state for managing pending operations
//! - `EventLoop`: Drives both V8 microtasks and Rust futures
//!
//! # Flow
//!
//! 1. JS calls an async API (e.g., `Deno.readTextFile`)
//! 2. Rust creates a V8 Promise and its resolver
//! 3. Rust spawns an async task and stores it as a PendingOp
//! 4. EventLoop polls pending ops and V8 microtasks
//! 5. When the Rust task completes, it resolves/rejects the Promise
//! 6. V8 microtask queue processes the resolution
//! 7. JS continuation runs

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::task::{Context, Poll, Waker};

use tokio::sync::Mutex;

pub type OpId = u64;

#[derive(Debug, Clone)]
pub enum OpResult {
    Ok(String),
    OkBytes(Vec<u8>),
    OkBool(bool),
    OkNumber(f64),
    OkJson(String),
    Err(String),
}

impl OpResult {
    pub fn ok(value: impl Into<String>) -> Self {
        OpResult::Ok(value.into())
    }

    pub fn err(error: impl Into<String>) -> Self {
        OpResult::Err(error.into())
    }
}

pub type OpFuture = Pin<Box<dyn Future<Output = OpResult> + Send>>;

pub struct PendingOp {
    pub id: OpId,
    pub future: OpFuture,
    pub waker: Option<Waker>,
}

impl PendingOp {
    pub fn new(id: OpId, future: OpFuture) -> Self {
        Self {
            id,
            future,
            waker: None,
        }
    }
}

pub struct OpState {
    pending_ops: HashMap<OpId, PendingOp>,
    next_id: AtomicU64,
    completed_ops: Vec<(OpId, OpResult)>,
}

impl OpState {
    pub fn new() -> Self {
        Self {
            pending_ops: HashMap::new(),
            next_id: AtomicU64::new(1),
            completed_ops: Vec::new(),
        }
    }

    pub fn next_id(&self) -> OpId {
        self.next_id.fetch_add(1, Ordering::SeqCst)
    }

    pub fn add_pending(&mut self, op: PendingOp) {
        self.pending_ops.insert(op.id, op);
    }

    pub fn has_pending(&self) -> bool {
        !self.pending_ops.is_empty()
    }

    pub fn pending_count(&self) -> usize {
        self.pending_ops.len()
    }

    pub fn poll_ops(&mut self, cx: &mut Context<'_>) -> Vec<(OpId, OpResult)> {
        let mut completed = Vec::new();
        let mut to_remove = Vec::new();

        for (id, op) in self.pending_ops.iter_mut() {
            op.waker = Some(cx.waker().clone());
            match Pin::new(&mut op.future).poll(cx) {
                Poll::Ready(result) => {
                    completed.push((*id, result));
                    to_remove.push(*id);
                }
                Poll::Pending => {}
            }
        }

        for id in to_remove {
            self.pending_ops.remove(&id);
        }

        completed
    }

    pub fn take_completed(&mut self) -> Vec<(OpId, OpResult)> {
        std::mem::take(&mut self.completed_ops)
    }
}

impl Default for OpState {
    fn default() -> Self {
        Self::new()
    }
}

pub type SharedOpState = Arc<Mutex<OpState>>;

pub fn create_op_state() -> SharedOpState {
    Arc::new(Mutex::new(OpState::new()))
}

pub struct PromiseRegistry {
    resolvers: HashMap<OpId, v8::Global<v8::PromiseResolver>>,
}

impl PromiseRegistry {
    pub fn new() -> Self {
        Self {
            resolvers: HashMap::new(),
        }
    }

    pub fn register(&mut self, id: OpId, resolver: v8::Global<v8::PromiseResolver>) {
        self.resolvers.insert(id, resolver);
    }

    pub fn take(&mut self, id: OpId) -> Option<v8::Global<v8::PromiseResolver>> {
        self.resolvers.remove(&id)
    }

    pub fn has_pending(&self) -> bool {
        !self.resolvers.is_empty()
    }

    pub fn pending_count(&self) -> usize {
        self.resolvers.len()
    }
}

impl Default for PromiseRegistry {
    fn default() -> Self {
        Self::new()
    }
}

pub struct EventLoop {
    op_state: SharedOpState,
}

impl EventLoop {
    pub fn new(op_state: SharedOpState) -> Self {
        Self { op_state }
    }

    pub async fn run_until_complete<'s>(
        &self,
        scope: &mut v8::HandleScope<'s>,
        promise_registry: &mut PromiseRegistry,
    ) -> Result<(), String> {
        loop {
            scope.perform_microtask_checkpoint();

            let completed = {
                let waker = futures::task::noop_waker();
                let mut cx = Context::from_waker(&waker);
                let mut state = self.op_state.lock().await;
                state.poll_ops(&mut cx)
            };

            for (op_id, result) in completed {
                if let Some(resolver) = promise_registry.take(op_id) {
                    let resolver = v8::Local::new(scope, resolver);
                    match result {
                        OpResult::Ok(value) => {
                            if let Some(v8_str) = v8::String::new(scope, &value) {
                                resolver.resolve(scope, v8_str.into());
                            }
                        }
                        OpResult::OkBytes(bytes) => {
                            let buffer = v8::ArrayBuffer::new(scope, bytes.len());
                            {
                                let backing_store = buffer.get_backing_store();
                                for (i, byte) in bytes.iter().enumerate() {
                                    backing_store[i].set(*byte);
                                }
                            }
                            let uint8_array =
                                v8::Uint8Array::new(scope, buffer, 0, bytes.len()).unwrap();
                            resolver.resolve(scope, uint8_array.into());
                        }
                        OpResult::OkBool(value) => {
                            let bool_val = v8::Boolean::new(scope, value);
                            resolver.resolve(scope, bool_val.into());
                        }
                        OpResult::OkNumber(value) => {
                            let num_val = v8::Number::new(scope, value);
                            resolver.resolve(scope, num_val.into());
                        }
                        OpResult::OkJson(json) => {
                            if let Some(v8_str) = v8::String::new(scope, &json) {
                                let context = scope.get_current_context();
                                let global = context.global(scope);
                                let json_key = v8::String::new(scope, "JSON").unwrap();
                                if let Some(json_val) = global.get(scope, json_key.into()) {
                                    if let Ok(json_obj) = v8::Local::<v8::Object>::try_from(json_val)
                                    {
                                        let parse_key = v8::String::new(scope, "parse").unwrap();
                                        if let Some(parse_val) =
                                            json_obj.get(scope, parse_key.into())
                                        {
                                            if let Ok(parse_func) =
                                                v8::Local::<v8::Function>::try_from(parse_val)
                                            {
                                                let args = [v8_str.into()];
                                                if let Some(parsed) =
                                                    parse_func.call(scope, json_obj.into(), &args)
                                                {
                                                    resolver.resolve(scope, parsed);
                                                } else {
                                                    resolver.resolve(scope, v8_str.into());
                                                }
                                            } else {
                                                resolver.resolve(scope, v8_str.into());
                                            }
                                        } else {
                                            resolver.resolve(scope, v8_str.into());
                                        }
                                    } else {
                                        resolver.resolve(scope, v8_str.into());
                                    }
                                } else {
                                    resolver.resolve(scope, v8_str.into());
                                }
                            }
                        }
                        OpResult::Err(error) => {
                            let error_str = v8::String::new(scope, &error).unwrap();
                            let exception = v8::Exception::error(scope, error_str);
                            resolver.reject(scope, exception);
                        }
                    }
                }
            }

            scope.perform_microtask_checkpoint();

            let has_pending = {
                let state = self.op_state.lock().await;
                state.has_pending()
            };

            if !has_pending && !promise_registry.has_pending() {
                break;
            }

            tokio::task::yield_now().await;
        }

        Ok(())
    }

    pub fn poll_once<'s>(
        &self,
        scope: &mut v8::HandleScope<'s>,
        promise_registry: &mut PromiseRegistry,
        op_state: &mut OpState,
    ) -> bool {
        scope.perform_microtask_checkpoint();

        let waker = futures::task::noop_waker();
        let mut cx = Context::from_waker(&waker);
        let completed = op_state.poll_ops(&mut cx);

        for (op_id, result) in completed {
            if let Some(resolver) = promise_registry.take(op_id) {
                let resolver = v8::Local::new(scope, resolver);
                match result {
                    OpResult::Ok(value) => {
                        if let Some(v8_str) = v8::String::new(scope, &value) {
                            resolver.resolve(scope, v8_str.into());
                        }
                    }
                    OpResult::OkBytes(bytes) => {
                        let buffer = v8::ArrayBuffer::new(scope, bytes.len());
                        {
                            let backing_store = buffer.get_backing_store();
                            for (i, byte) in bytes.iter().enumerate() {
                                backing_store[i].set(*byte);
                            }
                        }
                        let uint8_array =
                            v8::Uint8Array::new(scope, buffer, 0, bytes.len()).unwrap();
                        resolver.resolve(scope, uint8_array.into());
                    }
                    OpResult::OkBool(value) => {
                        let bool_val = v8::Boolean::new(scope, value);
                        resolver.resolve(scope, bool_val.into());
                    }
                    OpResult::OkNumber(value) => {
                        let num_val = v8::Number::new(scope, value);
                        resolver.resolve(scope, num_val.into());
                    }
                    OpResult::OkJson(json) => {
                        if let Some(v8_str) = v8::String::new(scope, &json) {
                            resolver.resolve(scope, v8_str.into());
                        }
                    }
                    OpResult::Err(error) => {
                        let error_str = v8::String::new(scope, &error).unwrap();
                        let exception = v8::Exception::error(scope, error_str);
                        resolver.reject(scope, exception);
                    }
                }
            }
        }

        scope.perform_microtask_checkpoint();

        op_state.has_pending() || promise_registry.has_pending()
    }
}

pub fn create_promise<'s>(
    scope: &mut v8::HandleScope<'s>,
) -> Option<(v8::Local<'s, v8::Promise>, v8::Local<'s, v8::PromiseResolver>)> {
    let resolver = v8::PromiseResolver::new(scope)?;
    let promise = resolver.get_promise(scope);
    Some((promise, resolver))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_op_state_creation() {
        let state = OpState::new();
        assert!(!state.has_pending());
        assert_eq!(state.pending_count(), 0);
    }

    #[test]
    fn test_op_id_generation() {
        let state = OpState::new();
        let id1 = state.next_id();
        let id2 = state.next_id();
        let id3 = state.next_id();
        assert_eq!(id1, 1);
        assert_eq!(id2, 2);
        assert_eq!(id3, 3);
    }

    #[test]
    fn test_op_result_variants() {
        let ok = OpResult::ok("hello");
        assert!(matches!(ok, OpResult::Ok(s) if s == "hello"));

        let err = OpResult::err("error");
        assert!(matches!(err, OpResult::Err(s) if s == "error"));

        let bytes = OpResult::OkBytes(vec![1, 2, 3]);
        assert!(matches!(bytes, OpResult::OkBytes(v) if v == vec![1, 2, 3]));

        let num = OpResult::OkNumber(42.0);
        assert!(matches!(num, OpResult::OkNumber(n) if n == 42.0));

        let bool_val = OpResult::OkBool(true);
        assert!(matches!(bool_val, OpResult::OkBool(true)));
    }

    #[test]
    fn test_promise_registry() {
        let registry = PromiseRegistry::new();
        assert!(!registry.has_pending());
        assert_eq!(registry.pending_count(), 0);
    }

    #[tokio::test]
    async fn test_pending_op_completion() {
        let mut state = OpState::new();
        let id = state.next_id();

        let future: OpFuture = Box::pin(async { OpResult::ok("completed") });

        let op = PendingOp::new(id, future);
        state.add_pending(op);

        assert!(state.has_pending());
        assert_eq!(state.pending_count(), 1);

        let waker = futures::task::noop_waker();
        let mut cx = Context::from_waker(&waker);

        tokio::task::yield_now().await;

        let completed = state.poll_ops(&mut cx);
        assert_eq!(completed.len(), 1);
        assert_eq!(completed[0].0, id);
        assert!(matches!(completed[0].1, OpResult::Ok(ref s) if s == "completed"));

        assert!(!state.has_pending());
    }

    #[tokio::test]
    async fn test_multiple_pending_ops() {
        let mut state = OpState::new();

        for i in 0..5 {
            let id = state.next_id();
            let future: OpFuture = Box::pin(async move { OpResult::ok(format!("result_{}", i)) });
            let op = PendingOp::new(id, future);
            state.add_pending(op);
        }

        assert_eq!(state.pending_count(), 5);

        tokio::task::yield_now().await;

        let waker = futures::task::noop_waker();
        let mut cx = Context::from_waker(&waker);
        let completed = state.poll_ops(&mut cx);

        assert_eq!(completed.len(), 5);
        assert!(!state.has_pending());
    }
}
