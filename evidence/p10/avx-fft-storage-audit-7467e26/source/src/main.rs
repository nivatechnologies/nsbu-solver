//! Reconstruct the finite AVX catalog measurement outside production targets.
use nsbu_solver::spectral::{FftBackend, FftCatalog};
use rustfft::{FftDirection, FftPlannerAvx};
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

struct RequestedBytes;

static ACTIVE: AtomicBool = AtomicBool::new(false);
static LIVE: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);
static TOTAL: AtomicUsize = AtomicUsize::new(0);
static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);

#[global_allocator]
static ALLOCATOR: RequestedBytes = RequestedBytes;

unsafe impl GlobalAlloc for RequestedBytes {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let pointer = unsafe { System.alloc(layout) };
        if !pointer.is_null() {
            record_allocation(layout.size());
        }
        pointer
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let pointer = unsafe { System.alloc_zeroed(layout) };
        if !pointer.is_null() {
            record_allocation(layout.size());
        }
        pointer
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        unsafe { System.dealloc(pointer, layout) };
        LIVE.fetch_sub(layout.size(), Ordering::SeqCst);
    }

    unsafe fn realloc(
        &self,
        pointer: *mut u8,
        layout: Layout,
        new_size: usize,
    ) -> *mut u8 {
        let replacement = unsafe { System.realloc(pointer, layout, new_size) };
        if !replacement.is_null() {
            LIVE.fetch_sub(layout.size(), Ordering::SeqCst);
            record_allocation(new_size);
        }
        replacement
    }
}

fn record_allocation(bytes: usize) {
    let live = LIVE.fetch_add(bytes, Ordering::SeqCst) + bytes;
    PEAK.fetch_max(live, Ordering::SeqCst);
    if ACTIVE.load(Ordering::SeqCst) {
        TOTAL.fetch_add(bytes, Ordering::SeqCst);
        ALLOCATIONS.fetch_add(1, Ordering::SeqCst);
    }
}

struct Measurement {
    retained: usize,
    peak_requested_live: usize,
    construction_total: usize,
    allocations: usize,
}

fn measure<T>(construct: impl FnOnce() -> T) -> (T, Measurement) {
    let baseline = LIVE.load(Ordering::SeqCst);
    PEAK.store(baseline, Ordering::SeqCst);
    TOTAL.store(0, Ordering::SeqCst);
    ALLOCATIONS.store(0, Ordering::SeqCst);
    ACTIVE.store(true, Ordering::SeqCst);
    let value = construct();
    ACTIVE.store(false, Ordering::SeqCst);
    (
        value,
        Measurement {
            retained: LIVE.load(Ordering::SeqCst) - baseline,
            peak_requested_live: PEAK.load(Ordering::SeqCst) - baseline,
            construction_total: TOTAL.load(Ordering::SeqCst),
            allocations: ALLOCATIONS.load(Ordering::SeqCst),
        },
    )
}

fn main() {
    if !cfg!(target_arch = "x86_64")
        || !std::is_x86_feature_detected!("avx")
        || !std::is_x86_feature_detected!("avx2")
        || !std::is_x86_feature_detected!("fma")
    {
        println!("SKIP required AVX/AVX2/FMA feature absent");
        return;
    }
    let lengths = [
        6usize, 96, 128, 144, 192, 256, 288, 384, 512, 576, 768, 1024, 1152, 1536,
    ];
    for length in lengths {
        for direction in [FftDirection::Forward, FftDirection::Inverse] {
            let (plan, report) = measure(|| {
                let mut planner = FftPlannerAvx::<f64>::new().expect("AVX planner");
                planner.plan_fft(length, direction)
            });
            println!(
                "SINGLE length={length} direction={direction:?} scratch={} retained={} peak_requested_live={} construction_total={} allocations={}",
                plan.get_inplace_scratch_len(),
                report.retained,
                report.peak_requested_live,
                report.construction_total,
                report.allocations,
            );
            drop(plan);
        }
    }
    let (catalog, report) = measure(|| {
        FftCatalog::new(FftBackend::RustFft6_4_1AvxFma, usize::MAX).expect("catalog")
    });
    println!(
        "CATALOG retained={} peak_requested_live={} construction_total={} allocations={} reservation={}",
        report.retained,
        report.peak_requested_live,
        report.construction_total,
        report.allocations,
        FftCatalog::reservation(FftBackend::RustFft6_4_1AvxFma).expect("reservation"),
    );
    drop(catalog);
}
