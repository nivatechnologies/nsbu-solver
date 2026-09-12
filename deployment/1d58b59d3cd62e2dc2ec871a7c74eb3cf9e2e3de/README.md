# Bounded local N384 HO timing pilot

This source-bound pilot times one accepted Hochbruck–Ostermann h64 proposal from exact REST and then a separate observer at that uncommitted proposal. It never calls prepare_commit or commit and publishes no state, balance, history, frontier, or checkpoint. `attempt-timing.json` is durable before observer work begins; failure after that point preserves the attempt timing. Completion proves the committed owner remains REST at clock/epoch/accepted steps 0/0/0 with a zero spectral payload.

The immutable numerical profile is N384/M384, observer force/conservative layout 768, RustFFT 6.4.1 AVX/AVX2/FMA, separate width-three RHS and force FFT owners, 32 sampling workers, HO h64, Cadv 3.3, absolute tolerances 1e-5/1e-4 and relative tolerances 1e-5/1e-5. Exact reservation is 198,987,813,712 B under CAP224GiB. Launch admission requires live MemAvailable >=274,877,906,944 B. Declared integration work is 110,846,361,675; observer force work is 58,435,043,328 plus fixed conservative/transfer loops.

The launcher binds the exact binary, fresh preflight, actual solver PID/PGID/starttime/cmdline hash, and true v2 watchdog. The deadline is exactly 1,200 seconds after solver discovery, followed by a 60-second TERM grace before KILL. This is a contended local measurement while the old and matched N256 runs remain protected. It informs relative method cost but does not estimate quiescent performance or later-state/h128 cost.

Focused validation: 7/7 tests, rustfmt, clippy `-D warnings`; LLVM branch coverage 29.45% lines/53.33% branches for this maintained harness scope; maximum per-function CRAP 22.5 with no violations. Each Rust source file is below 500 lines.
