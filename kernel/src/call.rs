//! One exported call of the WASM wrapper (TECHNICAL 2.1, "A failed call commits nothing", S32 R1):
//! the kernel runs over a `Buffered` state, the host's `new_id` keeps its counter in the same
//! buffer, and `Ok` commits the writes while `Err` drops them. Host-independent, so the wrapper's
//! rule is tested natively.

use crate::sessions::{Host, Kernel, Outcome, Request, Started, StreamError};
use crate::state::{Buffered, State, TooLarge};
use crate::KernelError;
use serde_json::Value;
use std::cell::RefCell;
use std::rc::Rc;

/// The key of the session id counter (`s1`, `s2`, ...).
pub const NEXT_ID: &str = "next-id";

type Buf<S> = Rc<RefCell<Buffered<S>>>;

/// The kernel's state for one call: the shared buffer.
pub struct CallState<S: State>(Buf<S>);

impl<S: State> State for CallState<S> {
    fn get(&mut self, key: &str) -> Option<Vec<u8>> {
        self.0.borrow_mut().get(key)
    }
    fn put(&mut self, key: &str, value: Vec<u8>) -> Result<(), TooLarge> {
        self.0.borrow_mut().put(key, value)
    }
    fn delete(&mut self, key: &str) {
        self.0.borrow_mut().delete(key)
    }
}

/// The kernel's host for one call: `inner` for everything but `new_id`, which counts in the
/// call's buffer.
pub struct CallHost<S: State, H: Host> {
    buf: Buf<S>,
    pub inner: H,
}

impl<S: State, H: Host> Host for CallHost<S, H> {
    fn generation(&mut self) -> u64 {
        self.inner.generation()
    }
    fn now(&mut self) -> u64 {
        self.inner.now()
    }
    fn new_id(&mut self) -> String {
        let mut b = self.buf.borrow_mut();
        let n = b
            .get(NEXT_ID)
            .and_then(|v| String::from_utf8(v).ok())
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(1);
        let _ = b.put(NEXT_ID, (n + 1).to_string().into_bytes());
        format!("s{n}")
    }
    fn timer_after(&mut self, id: &str, delay_ms: u64) {
        self.inner.timer_after(id, delay_ms)
    }
    fn timer_cancel(&mut self, id: &str) {
        self.inner.timer_cancel(id)
    }
    fn request(&mut self, r: Request) {
        self.inner.request(r)
    }
    fn open_stream(&mut self) -> Result<String, StreamError> {
        self.inner.open_stream()
    }
    fn start_provider(&mut self, launch: &str, input: &str) -> Result<Started, String> {
        self.inner.start_provider(launch, input)
    }
    fn cancel_stream(&mut self, stream: &str) {
        self.inner.cancel_stream(stream)
    }
    fn push(&mut self, stream: &str, value: Value) -> Result<(), StreamError> {
        self.inner.push(stream, value)
    }
    fn close_stream(&mut self, stream: &str, outcome: Outcome) {
        self.inner.close_stream(stream, outcome)
    }
    fn resume_stream(&mut self, stream: &str) {
        self.inner.resume_stream(stream)
    }
}

/// The kernel one call runs.
pub type CallKernel<S, H> = Kernel<CallState<S>, CallHost<S, H>>;

/// Runs one crate call over `state` and `host`: `Ok` commits the buffered writes in order, `Err`
/// drops them. Gives the state and the host back.
pub fn run<S: State, H: Host, T>(
    state: S,
    host: H,
    f: impl FnOnce(&mut CallKernel<S, H>) -> Result<T, KernelError>,
) -> (Result<T, KernelError>, S, H) {
    let buf: Buf<S> = Rc::new(RefCell::new(Buffered::new(state)));
    let mut k = Kernel::new(
        CallState(buf.clone()),
        CallHost {
            buf: buf.clone(),
            inner: host,
        },
    );
    let r = f(&mut k);
    drop(k.state);
    let CallHost {
        buf: held,
        inner: host,
    } = k.host;
    drop(held);
    let Ok(cell) = Rc::try_unwrap(buf) else {
        unreachable!("the kernel is dropped")
    };
    let b = cell.into_inner();
    let state = if r.is_ok() { b.commit() } else { b.discard() };
    (r, state, host)
}
