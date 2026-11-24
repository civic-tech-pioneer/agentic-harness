//! One exported call of the WASM wrapper (TECHNICAL 2.1, "A failed call commits nothing", S32 R1):
//! the kernel runs over a `Buffered` state, the host's `new_id` keeps its counter in the same
//! buffer, and `Ok` commits the writes while `Err` drops them, and with them the call's timers (a
//! refused call arms nothing that writes later, S35 r1 F1.1). Host-independent, so the wrapper's
//! rule is tested natively.

use crate::sessions::{Host, Kernel, Outcome, Request, Started, StreamError, Tool};
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

/// A timer call the call holds until it commits.
enum Timer {
    After(String, u64),
    Cancel(String),
}

/// The kernel's host for one call: `inner` for everything but `new_id`, which counts in the
/// call's buffer, and the timers, which wait for the call's `Ok`.
pub struct CallHost<S: State, H: Host> {
    buf: Buf<S>,
    timers: Vec<Timer>,
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
        self.timers.push(Timer::After(id.to_string(), delay_ms));
    }
    fn timer_cancel(&mut self, id: &str) {
        self.timers.push(Timer::Cancel(id.to_string()));
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
    fn https_start(&mut self, request: &str) -> Result<String, String> {
        self.inner.https_start(request)
    }
    fn tools_open(&mut self, toolsets: &[String]) -> Result<String, String> {
        self.inner.tools_open(toolsets)
    }
    fn tools_describe(&mut self, handle: &str) -> Result<Vec<Tool>, String> {
        self.inner.tools_describe(handle)
    }
    fn tools_call(
        &mut self,
        handle: &str,
        key: &str,
        tool: &str,
        args: &str,
    ) -> Result<String, String> {
        self.inner.tools_call(handle, key, tool, args)
    }
    fn tools_close(&mut self, handle: &str) {
        self.inner.tools_close(handle)
    }
}

/// The kernel one call runs.
pub type CallKernel<S, H> = Kernel<CallState<S>, CallHost<S, H>>;

/// Runs one crate call over `state` and `host`: `Ok` commits the buffered writes in order, then
/// sets and cancels the call's timers in order; `Err` drops both. Other host calls go out as the
/// crate makes them. Gives the state and the host back.
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
            timers: Vec::new(),
            inner: host,
        },
    );
    let r = f(&mut k);
    drop(k.state);
    let CallHost {
        buf: held,
        timers,
        inner: mut host,
    } = k.host;
    drop(held);
    let Ok(cell) = Rc::try_unwrap(buf) else {
        unreachable!("the kernel is dropped")
    };
    let b = cell.into_inner();
    if r.is_err() {
        return (r, b.discard(), host);
    }
    let state = b.commit();
    for t in timers {
        match t {
            Timer::After(id, ms) => host.timer_after(&id, ms),
            Timer::Cancel(id) => host.timer_cancel(&id),
        }
    }
    (r, state, host)
}
