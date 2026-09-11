# Exact-v2 force-sampling trajectory family

This increment adds three independently evolved exact-v2 trajectories at fixed
N4, CM step 64 and endpoint 512 while the nested prescribed-force sample grid is
M4/M8/M16. See the [public guide](../../../docs/V2_FORCE_REFINEMENTS.md).

Four focused harness tests, one isolated allocation executable and the runnable
example pass. The actual post-startup endpoint is 1/2048. Full velocity L2
differences are 2.4424439651500145e-2 and 5.726571685966544e-3; complete
derivative-sensitive H1 differences are 2.2572582941869e-1 and
5.11740607059927e-2. Both pairs match the independent signed full-complex Fourier
oracle. An ownership/self-comparison negative control demonstrates the false zero
from reusing one state; the existing solver force-aliasing tests remain separate.

Focused LLVM coverage across the seven instrumented implementation/test files is
666/681 executable lines (97.80%) and 35/42 branches (83.33%). The runnable
example passed separately and is included in static analysis, but it was omitted
from the final LLVM replay to avoid a second instrumented execution of the same
long post-startup profile. Maximum per-function CRAP is 13.8222. Across all eight
focused Rust files, static maxima are CC13, cognitive10, Halstead53.2895 and 338
physical lines. Focused strict Clippy, formatting and whole-workspace Rustdoc pass.

Admission and evolution allocate nothing; the shorter allocation profile uses
2,279,616 actual constructed heap bytes within 2,498,488 reserved bytes. Tests
also cover zero/backend-invalid/non-nested force grids, cap refusal, all identity
policy words, exact manifest changes, private owners, terminal child failure,
retained committed state and spent attempts.

An initial focused report found CRAP26.3231 in the combined validation function.
That failure is retained in development history; separating force-grid, schedule
and domain validation reduced the final maximum below the required limit. A
package-only command also could not resolve the unpublished local `nsbu-solver`
from crates.io; workspace compilation and Rustdoc succeeded, and the root hosted
workflow remains the source-matched package gate.

These results establish a bounded partial L2/H1 force-sampling trajectory study.
They do not establish monotonic convergence, force sufficiency, other observable
channels or an accepted concentrating window. P08/P09/P10 remain incomplete.
