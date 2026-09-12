//! Fixed-capacity allocation tracing for the one authorized M512 diagnostic run.
use std::{
    alloc::{GlobalAlloc, Layout, System},
    ffi::c_void,
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
};

const TRACE_CAPACITY: usize = 16;
const TRACE_DEPTH: usize = 24;
const ALLOC: usize = 1;
const DEALLOC: usize = 2;
const REALLOC: usize = 3;

static TRACKING: AtomicBool = AtomicBool::new(false);
static CAPTURING: AtomicBool = AtomicBool::new(false);
static PHASE: AtomicUsize = AtomicUsize::new(0);
static EVENT_COUNT: AtomicUsize = AtomicUsize::new(0);
static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);
static DEALLOCATIONS: AtomicUsize = AtomicUsize::new(0);
static REALLOCATIONS: AtomicUsize = AtomicUsize::new(0);
static OWNER_THREAD: AtomicUsize = AtomicUsize::new(0);
static EVENTS: [TraceEvent; TRACE_CAPACITY] = [const { TraceEvent::new() }; TRACE_CAPACITY];

/// Global allocator wrapper with allocation-free, fixed-capacity event storage.
pub struct TraceAllocator;

struct TraceEvent {
    operation: AtomicUsize,
    phase: AtomicUsize,
    size: AtomicUsize,
    new_size: AtomicUsize,
    align: AtomicUsize,
    thread: AtomicUsize,
    depth: AtomicUsize,
    frames: [AtomicUsize; TRACE_DEPTH],
}

impl TraceEvent {
    const fn new() -> Self {
        Self {
            operation: AtomicUsize::new(0),
            phase: AtomicUsize::new(0),
            size: AtomicUsize::new(0),
            new_size: AtomicUsize::new(0),
            align: AtomicUsize::new(0),
            thread: AtomicUsize::new(0),
            depth: AtomicUsize::new(0),
            frames: [const { AtomicUsize::new(0) }; TRACE_DEPTH],
        }
    }
}

/// Counts retained by the original zero-allocation assertion.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AllocationCounts {
    /// Allocation calls in the measured region.
    pub allocations: usize,
    /// Deallocation calls in the measured region.
    pub deallocations: usize,
    /// Reallocation calls in the measured region.
    pub reallocations: usize,
}

struct TraceCursor {
    event: usize,
    depth: usize,
}

#[repr(C)]
struct UnwindContext {
    _private: [u8; 0],
}

unsafe extern "C" {
    fn _Unwind_Backtrace(
        callback: unsafe extern "C" fn(*mut UnwindContext, *mut c_void) -> i32,
        argument: *mut c_void,
    ) -> i32;
    fn _Unwind_GetIP(context: *mut UnwindContext) -> usize;
    fn pthread_self() -> usize;
}

unsafe extern "C" fn unwind_frame(context: *mut UnwindContext, argument: *mut c_void) -> i32 {
    let cursor = unsafe { &mut *argument.cast::<TraceCursor>() };
    if cursor.depth == TRACE_DEPTH {
        return 5;
    }
    let instruction = unsafe { _Unwind_GetIP(context) };
    EVENTS[cursor.event].frames[cursor.depth].store(instruction, Ordering::Relaxed);
    cursor.depth += 1;
    0
}

unsafe impl GlobalAlloc for TraceAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let pointer = unsafe { System.alloc(layout) };
        record(ALLOC, layout, layout.size());
        pointer
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let pointer = unsafe { System.alloc_zeroed(layout) };
        record(ALLOC, layout, layout.size());
        pointer
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        record(DEALLOC, layout, 0);
        unsafe { System.dealloc(pointer, layout) };
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let result = unsafe { System.realloc(pointer, layout, new_size) };
        record(REALLOC, layout, new_size);
        result
    }
}

fn record(operation: usize, layout: Layout, new_size: usize) {
    if !TRACKING.load(Ordering::Relaxed) {
        return;
    }
    match operation {
        ALLOC => ALLOCATIONS.fetch_add(1, Ordering::Relaxed),
        DEALLOC => DEALLOCATIONS.fetch_add(1, Ordering::Relaxed),
        REALLOC => REALLOCATIONS.fetch_add(1, Ordering::Relaxed),
        _ => unreachable!(),
    };
    let index = EVENT_COUNT.fetch_add(1, Ordering::Relaxed);
    if index >= TRACE_CAPACITY {
        return;
    }
    let event = &EVENTS[index];
    event.operation.store(operation, Ordering::Relaxed);
    event
        .phase
        .store(PHASE.load(Ordering::Relaxed), Ordering::Relaxed);
    event.size.store(layout.size(), Ordering::Relaxed);
    event.new_size.store(new_size, Ordering::Relaxed);
    event.align.store(layout.align(), Ordering::Relaxed);
    event
        .thread
        .store(unsafe { pthread_self() }, Ordering::Relaxed);
    if CAPTURING
        .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
        .is_ok()
    {
        let mut cursor = TraceCursor {
            event: index,
            depth: 0,
        };
        unsafe { _Unwind_Backtrace(unwind_frame, (&mut cursor as *mut TraceCursor).cast()) };
        event.depth.store(cursor.depth, Ordering::Release);
        CAPTURING.store(false, Ordering::Release);
    }
}

/// Begin the original region immediately before its first W3 evaluation.
pub fn begin_measurement() {
    OWNER_THREAD.store(unsafe { pthread_self() }, Ordering::Relaxed);
    EVENT_COUNT.store(0, Ordering::Relaxed);
    ALLOCATIONS.store(0, Ordering::Relaxed);
    DEALLOCATIONS.store(0, Ordering::Relaxed);
    REALLOCATIONS.store(0, Ordering::Relaxed);
    TRACKING.store(true, Ordering::Release);
}

/// Tag the current repeated evaluation without allocating.
pub fn set_phase(phase: usize) {
    PHASE.store(phase, Ordering::Relaxed);
}

/// End the unchanged measured region and return its complete counts.
pub fn finish_measurement() -> AllocationCounts {
    TRACKING.store(false, Ordering::Release);
    PHASE.store(0, Ordering::Relaxed);
    AllocationCounts {
        allocations: ALLOCATIONS.load(Ordering::Relaxed),
        deallocations: DEALLOCATIONS.load(Ordering::Relaxed),
        reallocations: REALLOCATIONS.load(Ordering::Relaxed),
    }
}

/// Print recorded sizes, phases, and raw instruction pointers outside the measured region.
pub fn print_traces() {
    let count = EVENT_COUNT.load(Ordering::Relaxed).min(TRACE_CAPACITY);
    for (index, event) in EVENTS.iter().enumerate().take(count) {
        print!(
            "allocation_trace index={index} operation={} phase={} size={} new_size={} align={} thread={} owner_thread={} frames=",
            event.operation.load(Ordering::Relaxed),
            event.phase.load(Ordering::Relaxed),
            event.size.load(Ordering::Relaxed),
            event.new_size.load(Ordering::Relaxed),
            event.align.load(Ordering::Relaxed),
            event.thread.load(Ordering::Relaxed),
            OWNER_THREAD.load(Ordering::Relaxed),
        );
        let depth = event.depth.load(Ordering::Acquire);
        for frame in event.frames.iter().take(depth) {
            print!(" {:#x}", frame.load(Ordering::Relaxed));
        }
        println!();
    }
}
