# Production physical-reduction arithmetic

The reduction auditor compares both public `TensorErrors` entry paths with an
independent 80/120-digit sum-of-squares oracle. It uses the actual CM/HO physical
component words from the [derived-field study](DERIVED_ARITHMETIC.md). The two
trajectories were evolved independently from rest. Reading these diagnostic
samples does not reconstruct or authenticate an integrated state.

The complete quantities are velocity (3 entries), gradient (9), ordered Hessian
(27), vorticity (3), pressure (1) and pressure gradient (3). Mixed Hessian entries
remain duplicated in their Cartesian positions. Every point contributes its
complete Euclidean/Frobenius error; RMS divides by the number of physical points,
never the number of components. Relative peaks use the maximum of the pointwise
reference magnitude and an explicit positive floor in the quantity's units.

## Run the checked fixture

From an installed developer checkout:

```sh
cargo run --release -p nsbu-benchmarks --example reduction_audit -- --dry-run 4
mkdir -p work
cargo run --release -p nsbu-benchmarks --example reduction_audit -- \
  crates/nsbu-benchmarks/data/reduction-n4.bin work/reduction-n4.bin
python -m reference.verify_reductions --n 4 --dry-run
python -m reference.verify_reductions --n 4 \
  --input crates/nsbu-benchmarks/data/reduction-n4.bin \
  --magnitudes work/reduction-n4.bin > work/reduction-n4.json
```

The Rust command refuses an existing output path. Use a new filename for another
run. Its optional third positional argument is a byte cap (default 64 MiB).
`--dry-run N CAP_BYTES` performs admission without opening an input file.
Unsupported grids, malformed packets, trailing bytes, nonfinite samples and
invalid floors are explicit errors. All numerical groups complete before an
output file is created. A later I/O error can leave a partial diagnostic file;
the strict reader rejects it. This is not a checkpoint persistence protocol.

To prepare another complete pair from the existing Rust `smooth_derived`
exporter, use N=4,8,12 and the fixed exporter profile described in the linked
study. Preserve the original JSON files and their provenance:

```sh
python -m reference.export_reduction_input --n 12 --dry-run
python -m reference.export_reduction_input --n 12 \
  --left work/derived-n12-cm.json --right work/derived-n12-ho.json \
  --output work/reduction-input-n12.bin
cargo run --release -p nsbu-benchmarks --example reduction_audit -- \
  work/reduction-input-n12.bin work/reduction-magnitudes-n12.bin
python -m reference.verify_reductions --n 12 \
  --input work/reduction-input-n12.bin \
  --magnitudes work/reduction-magnitudes-n12.bin > work/reduction-n12.json
```

The preparer checks both complete derived-export schemas, fixed method roles,
46-row inventory and identical prescribed endpoint force. It retains every
sample word, including signed zeros, without a decimal round trip. Schema and
SHA-256 agreement identify supplied bytes; they do not prove who computed them.
The original execution evidence supplies that separate provenance.

## Separate arithmetic effects

The Rust audit invokes the production `TensorErrors::push` component reducer.
It separately reproduces the physical workspace's sequential `hypot` component
order, retains every resulting magnitude word and invokes the production
`TensorErrors::push_magnitudes` reducer. This audit does not call the FFT-based
physical workspace or evolve another trajectory.

The Python oracle sums exact binary64 component differences at 80 and 120 decimal
digits using an independent ordinary sum-of-squares formulation. It also reduces
the exact words of Rust's already rounded magnitudes. The report preserves:

- Component-entry statistics versus exact-component MP statistics.
- Physical magnitude-entry statistics versus exact-component MP statistics.
- Physical magnitude-entry statistics versus MP reductions of the same rounded magnitudes.
- Maximum and RMS component-to-magnitude discrepancies for error and reference magnitudes.

Every statistic retains its absolute error, relative error when the oracle value
is nonzero, 80/120 precision change, and precision-change/error ratio when the
binary64 discrepancy is nonzero. Exact-zero errors are explicit; no artificial
positive floor conceals them. The study's declared arithmetic-separation criterion
is a precision change below 1e-40 times each nonzero measured discrepancy, with
zero change accepted explicitly. This is a precision study criterion, not a PDE
error allocation. Completed measurement output preserves unresolved criteria;
command success alone is not numerical acceptance.

## Measured profile and resources

The N=4 and N=12 inputs use M=2N physical samples, t=1/512, quantum 2^-16,
macro step 16 ticks, viscosity 1 and independently integrated CM/HO trajectories.
Each grid has 72 per-statistic comparisons, covering all six quantities, four
statistics and three arithmetic effects. All 144 measured comparisons meet the
stated precision-separation criterion. Maximum nonzero precision-change/error
ratios are 4.98e-64 for N=4 and 2.10e-64 for N=12 (rounded upward).

At N=12 the physical path's RMS relative arithmetic discrepancies range from
5.25e-16 to 2.13e-15 across these quantities. Absolute errors and units differ
substantially; the complete report retains every value. These small smooth-grid
measurements do not establish arithmetic floors at concentrating production grids.

| Stage | Default cap | Scope |
|---|---:|---|
| JSON-to-word preparation | 2 GiB | Both bounded JSON parsers, validated fields, packet and Python allowance |
| Rust audit | 64 MiB | Complete packet, twelve magnitude arrays and 1 MiB metadata/stack allowance |
| Python precision study | 128 MiB | Bounded packets, independent constant-memory MP accumulators and interpreter allowance |

The preparer's N=12 reservation is 1,916,272,640 bytes. Python reservations are
conservative planning estimates, not hard allocator guarantees. Rust allocation
instrumentation verifies one packet plus twelve magnitude allocations within the
joint reservation; admission refusals and output streaming allocate nothing.
No diagnostic memory budget includes a concurrently running solver owner.

## Packet formats and preserved fixture

All integers and float words use little-endian u64. Floating values are stored
as original IEEE binary64 bits. Input `NSBURED1` has a 136-byte header:
magic (8), N (8), physical point count (8), six floor words (48), then the two
original JSON SHA-256 digests (64). Payload order is CM then HO, each with the
canonical 46 rows, each row containing all `(2N)^3` point words.

Output `NSBUMAG1` has a 488-byte header: magic (8), N (8), points (8), full input
SHA-256 (32), six floor words (48), then six quantities × two entry paths × four
statistic words (384). The payload holds six quantity groups, each with complete
error then reference magnitude rows. Statistic order is RMS, absolute peak,
pointwise relative peak, reference peak. Complete lengths are checked before use.

[The N=4 input](../crates/nsbu-benchmarks/data/reduction-n4.bin) and
[actual output](../fixtures/reference/reduction-n4-magnitudes.bin) are generated
project data under Apache-2.0. Original JSON inputs remain unchanged in the
[derived study inventory](../evidence/p09/derived-arithmetic/artifact-sha256.json).
The packet's original JSON hashes, source inventory and new execution reports
provide a reviewable chain; no expected frozen hash is changed.

P08/P09 remain incomplete. Residual/balance/regional/location arithmetic, complete
reference/force/transfer refinements, benchmark/artifact integration and actual
concentrating convergence remain. No concentrating PDE window is accepted.

The complete [executed evidence and source inventory](../evidence/p09/reduction-arithmetic/README.md)
records the quality profiles, original packet identities and clean reproduction.
