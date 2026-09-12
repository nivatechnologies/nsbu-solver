# M512 allocation attribution diagnostic

This directory preserves the one authorized source-changed attribution run.
The diagnostic source is commit
`29adfa60287edea6e1d748ffcf07bc6c7fb5e1d0`. The original failed run is
preserved separately in `../review-control-20260912T2218Z`.

The diagnostic replaced `stats_alloc` only for attribution. Its fixed-capacity
global allocator wrapper records operation, size, phase, pthread identity, and
raw `_Unwind_Backtrace` instruction pointers in static atomics. Recursive trace
capture is suppressed without allocating. Measurement begins immediately
before the first of three real W3 evaluations and ends immediately after the
third, exactly matching the failed control. There is no warm-up, owner-thread
filter, assertion change, or second diagnostic execution.

The run reproduced `(2,2,1)`. Each event occurred in phase 1 on pthread
`129064448048128`; the test owner was pthread `129064444622528`. The addresses
resolve into Rust libtest's pretty formatter, console event handler, and test
runner. Event sizes show a 512-byte buffer allocation plus an 82-byte string
allocation, growth to 164 bytes, and both deallocations. This exactly coincides
with libtest's one slow-test status line after 60 seconds. No event came from
the W3 owner thread.

At this diagnostic stage the zero-allocation resource gate remained failed.
Root subsequently approved one standalone execution of the same first-attempt
three-evaluation region without libtest. That closure passed and is preserved
in `../review-standalone-20260912T2237Z`; this failed diagnostic record remains
unchanged.
