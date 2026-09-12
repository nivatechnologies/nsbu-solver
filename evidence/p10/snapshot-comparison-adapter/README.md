# Scheduled snapshot comparison adapter

This read-only evidence harness compares two reviewed `P10AVXSNAP1\0` state files. It does not expose a resume or state-injection API and does not alter a trajectory. Each JSON input manifest supplies the exact snapshot identity, source commit, frozen-plan path and SHA-256, expected coefficient and whole-file SHA-256 values, retained dimensions, and explicit evolution semantics. The adapter requires exact equality of case SHA-256, clock quantum/target, comparison endpoint, physical domain/viscosity, Cox–Matthews method, M384 integration-force grid, piecewise schedule, and absolute/relative tolerances. Backend and execution descriptions are side-specific, so a serial-component N192 state may be compared with a W3 N256 or N384 state. Side-specific identities, sources, plans, backends, and execution descriptions are preserved in the output. Nothing is inferred from snapshot length. Relative snapshot and plan paths resolve beside their manifest, and the referenced plan is bounded to 1 MiB and hash-verified. Reviewers remain responsible for checking that its content agrees with the explicit evolution fields.

The command is:

```text
cargo run --release --manifest-path evidence/p10/snapshot-comparison-adapter/harness/Cargo.toml -- LEFT.json RIGHT.json CAP_BYTES
```

The original three-argument command remains the only interface. Manifests that
omit `comparison_kind` retain the original `MATCHED_SPATIAL` policy. A temporal
screen is enabled only when both manifests explicitly say `TIME_DIAGNOSTIC`.
Mixed modes are refused before state allocation.

`TIME_DIAGNOSTIC` is a same-grid, same-physical-endpoint screen. It requires
bit-exact equality of the case, retained dimensions, domain lengths, viscosity,
clock quantum and target, comparison endpoint, Cox–Matthews method, M384 force
grid, and both tolerance arrays. Each side retains and reports its own complete
schedule, exact profile name, advective limit, maximum attempt guard, epoch,
accepted-step count, identity, source, backend, execution, and plan hash. The
adapter validates each schedule independently from zero through the endpoint
and derives both the epoch and accepted-step count from it. Only the schedule,
those derived counters, and the reported admission guards may differ.

Every time manifest must also carry the same `arithmetic_control` binding with
a nested `p10-time-arithmetic-review-v1` review. That object binds both sides' exact
source commit, backend, execution, and profile to a bounded, hash-verified
JSON review artifact. The adapter parses that artifact rather than trusting an
outcome string in the snapshot manifest. Its `measured_control` records the
source, backend, execution, and profile that actually produced the successful
exact-bit serial/W3 control. Its separate `reviewed_lineage` binds those control
roles to the two current, unrelabeled snapshot profiles with the honest
conclusion `reviewed-equivalence-supported-by-controls`. The review's case,
Cox–Matthews method, and M384 force grid must match the snapshot manifests.
Arbitrary hash-matching bytes, a malformed review, a mismatched reviewed copy,
or a review for another physical contract is refused. Reviewers remain
responsible for approving the unchanged-kernel lineage; the adapter does not
represent the current profile sources as the directly measured control sources
and does not treat cross-grid state hashes as arithmetic evidence. The example
in `time-diagnostic-manifest.example.json` shows one side of this binding. A
time result uses schema `p10-snapshot-time-diagnostic-output-v1`,
labels itself `TIME_DIAGNOSTIC`, and always reports acceptance as
`not_assessed` with zero accepted windows. It is not a trajectory, resume,
state-injection, acceptance, or PDE-qualification interface.

Before allocating either coefficient array, the harness checks both exact file lengths and admits `left coefficient bytes + right coefficient bytes + 1 MiB fixed overhead` under `CAP_BYTES`. The cushion conservatively covers bounded manifest and plan streaming, identity decode, SHA states, JSON output, and allocator bookkeeping. It streams each snapshot once and retains only its decoded three-component state. The decoder checks framing, exact identity and u128 clock fields, coefficient trailer SHA-256, finite values, Hermitian zero plane, and exact Nyquist zero. It reports the coefficient-state SHA-256 separately from the whole-file SHA-256; hashes across different retained grids are provenance, never an equality condition.

For cubic retained grids, the admitted pair bounds are 1,538,719,744 bytes for N192/N384 and 1,772,879,872 bytes for N256/N384. The Rust runtime and thread stack remain process overhead outside this single-threaded harness allocation bound.

The left grid must be componentwise no finer than the right grid. Endpoint elapsed, target, epoch, and accepted-step clocks must match exactly. Error metrics come directly from `nsbu_solver::diagnostics::comparison::ComparisonPlan`: no normalization or mean removal is applied, and strict positive-z Parseval weights produce full, common-band, and newly-resolved-shell L2, H1, vorticity L2, and divergence L2 outputs. The fine-state absolute norms use an allocation-free copy of the same `diagnostics/norms.rs` scaled-squares operation order so relative budgets need no 1.36 GB zero-state allocation; small controls compare all four outputs bit-for-bit with `ComparisonPlan` against explicit zeros.

Admission guards are provenance, not trajectory semantics. When supplied on a
matched-spatial manifest they are preserved side by side in the output and may
differ; they do not participate in the original strict spatial equality gate.

The archived `control-n192-self.json` decodes the completed legacy N192 scheduled `node-4096/state.bin` from source `92effa6068d20d69e815c6a83f1e82490ce37fe7` twice. Both coefficient and whole-file hashes match its reviewed manifest; full/common/new-shell and mean errors are exact zero. Its allocation-free absolute norms are bit-identical to the node record (`l2=1.8388998858949468`, `h1=50.872398531129306`, `vorticity_l2=50.8391520359154`, `divergence_l2=1.0945737929461952e-14`). This is an adapter/writer/schema smoke control, not a new trajectory or cross-grid PDE result.

No matched cross-grid endpoint pair was complete in this isolated worktree at construction time. A later endpoint comparison should archive both reviewed input manifests, stdout JSON, the adapter source commit, and a hash manifest beside the scheduled evidence.
