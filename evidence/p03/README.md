# P03 spectral operators

The normalized R2C transform uses original mixed-radix 2/3 complex transforms,
explicit root tables and caller-owned scratch. Forward coefficients divide by
all physical samples; inverse transforms use the unnormalized sum. The selected
backend supports axis lengths at most 1024 with prime factors only 2 and 3.
Retained grids still obey P02's four-multiple contract; their padded grids must
also satisfy the backend profile. Unsupported lengths and inadequate caps fail
before allocation. This bounded backend replaces the unused prospective FFT pins;
the alternative P01 spike remains historical evidence.

Tests compare actual coefficients with independent direct sums on cubic and
anisotropic grids, including negative modes and raw Nyquist coefficients. Strict
three-halves padding/cropping preserves amplitudes and the entire retained band.
Sparse-vector conservative coefficient convolution independently checks the
rotational implementation, physical pressure and semidiscrete inviscid energy
identity. A deliberately unpadded physical product aliases a nonzero interaction
into a retained mode; padded sampling and the production operator remove it.
Mean velocity, mean acceleration and pressure from gradient forcing are checked.

Input finite/conjugacy checks and transform arithmetic-overflow refusals are
covered. Operator errors leave inputs unchanged and invalidate only output scratch.
A dedicated process instruments allocations: construction remains within the
reported reservation and twenty repeated operator calls allocate, deallocate and
reallocate zero times. Reservations cover owned elements and object headers;
callers must separately declare input/output, provider and allocator overhead.
One evaluation including physical pressure uses ten scalar 3D transforms; the
future twelve-evaluation CM attempt must budget 120, not silently retain 108.

[summary.json](summary.json) records each quality metric, complete source hashes,
coverage scope and mutation accounting. Raw coverage, complexity, duplication,
mutation outcomes, test output, dependency inventory and package verification are
preserved alongside it. Stable Rust 1.94.0 builds and lints; the pinned nightly
measures executable lines and instrumented branches including integration tests.
LLVM region/instantiation coverage is separately reported and is not claimed as
100%. Mutation tests exclude test code as targets but all test source remains in
coverage and static-analysis scope. Initial runs exposed weak accounting and
conjugacy-boundary tests and an unbounded mutant of factor reduction; explicit
boundary tests and bounded factor reduction resolve those failures. Timeouts in
those exploratory runs are not counted as kills in the final fresh run.

SOLID review: storage, layout, radix transforms, transform normalization, modal
operators, band transfer and rotational assembly have separate responsibilities.
Public contract tests exercise their interfaces. Reference convolution remains
independent of production FFT and projection. Mutable scratch belongs to a single
operator instance; there is no shared state, reference-state injection, I/O,
scheduling or hidden retry. Compiler/Clippy, exported API usage review and token
duplication checks find no dead or redundant maintained code. No dynamic type
erasure is used.

This is operator verification, not a Rust PDE trajectory or convergence result.
No timestep, accepted PDE window or visualization is implemented by P03.
Hosted verification remains pending until its run is recorded here.
