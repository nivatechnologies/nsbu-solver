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

Profile provenance has two closed forms. `identity-profile-field` requires the
binding value to equal the snapshot identity's exact `profile=` field.
`legacy-full-identity` exists only for the legacy h32 writer that published no
profile field; its value must equal the complete published identity byte for
byte. It does not append or infer a profile name. The output and reviewed
lineage preserve both the binding kind/value and the exact full identity.

Every time manifest must also carry the same `arithmetic_control` binding with
a nested `p10-time-arithmetic-review-v1` review. That object binds both sides' exact
source commit, backend, execution, and profile to a bounded, hash-verified
JSON review artifact. The review artifact has its own 64 KiB cap because its
complete parsed content is already embedded in a manifest with the same cap;
the independent frozen-plan stream retains its 1 MiB cap. The adapter parses
the review artifact rather than trusting an
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

Two additional modes are closed, directional diagnostics for completed N384
states. `FORCE_RESOLUTION_DIAGNOSTIC` admits only a left M384 and right M512
integration-force grid with Cox–Matthews. Retained N384 dimensions, the complete
schedule and clocks, case, quantum, endpoint, domain, viscosity, and tolerances
must otherwise match bit for bit. `METHOD_DIAGNOSTIC` admits only left
`cox-matthews` and right `hochbruck-ostermann` at retained N384 and integration
force M384, again with every other evolution field identical. Both modes
validate each exact identity/profile, schedule-derived epoch and accepted-step
count, and require those committed steps to fit within the admission guard's
maximum-attempt cap. They preserve the exact cap and both sides' guards,
identities, sources, backends, executions, plans, hashes, schedules, and clocks
in their outputs. An arithmetic-control object is refused in these modes because the
time-specific review schema cannot override their changed force or method
contract. Their output schemas are respectively
`p10-snapshot-force-resolution-diagnostic-output-v1` and
`p10-snapshot-method-diagnostic-output-v1`; both report acceptance as
`not_assessed` with zero accepted windows. Mixed modes, reversed directions,
other force sizes or methods, and any additional physics difference are
refused before state allocation. Neither mode may be used before a separate
root review of the exact manifests.

`MATCHED_M512_SPATIAL_DIAGNOSTIC` is a separate closed spatial screen for the
completed r6 N384/M512 endpoint and the prepared N512/M512 capture profile. It
admits only left N384 source `326eeb5cbd5ebe39a7d5f7be77f9acfab8d0db72`
and right N512 source `9eba11f196a25f0843f0cbd0f4ed08c9f7ae4645`,
with their exact profile names. Both sides must use the same M512 force, case,
clock-4096 endpoint, 32-by-64 then 16-by-128 schedule, Cox--Matthews method,
tolerances, domain, viscosity, and 3.3/48 admission guard. Its output reports
acceptance as `not_assessed`; common-band and newly-resolved-shell norms are
sampled diagnostics rather than qualification. The exact pair admission is
4,600,889,344 bytes.

The optional ordered-Hessian screen uses `LEFT.json RIGHT.json CAP_BYTES
--ordered-hessian` and retains the matched-spatial manifest contract. The closed
three-state force/space screen uses `COARSE.json BASELINE.json FORCE.json
CAP_BYTES --mixed-force-space`. It binds N256/M384, N384/M384, and N384/M512 at
one clock and reports the spatial difference A, force difference B, combined
difference C=A+B, and weighted cross terms without assigning either acceptance
budget. Both are read-only diagnostics with separate output schemas.

After the reviewed clock-512 mixed result was archived, source commit
`61e7d5778bb3a1dbb1de77151299c8788e109d32` separated manifest/output ownership
in `mixed.rs` from the unchanged traversal and accumulation order in
`mixed_math.rs`. It also centralized the existing two-form profile matcher and
split request parsing, admission, loading, calculation, and publication by
resource ownership. The archived result remains bound to numerical source
`62b763546e70ff59edd2cc56396e72998e31e290`; it was not rerun or relabeled by
this structural cleanup.

Before allocating either coefficient array, the harness checks both exact file lengths and admits `left coefficient bytes + right coefficient bytes + 1 MiB fixed overhead` under `CAP_BYTES`. The cushion conservatively covers bounded manifest and plan streaming, identity decode, SHA states, JSON output, and allocator bookkeeping. It streams each snapshot once and retains only its decoded three-component state. The decoder checks framing, exact identity and u128 clock fields, coefficient trailer SHA-256, finite values, Hermitian zero plane, and exact Nyquist zero. It reports the coefficient-state SHA-256 separately from the whole-file SHA-256; hashes across different retained grids are provenance, never an equality condition.

For cubic retained grids, the admitted pair bounds are 1,538,719,744 bytes for N192/N384 and 1,772,879,872 bytes for N256/N384. The Rust runtime and thread stack remain process overhead outside this single-threaded harness allocation bound.

The left grid must be componentwise no finer than the right grid. Endpoint elapsed, target, epoch, and accepted-step clocks must match exactly. Error metrics come directly from `nsbu_solver::diagnostics::comparison::ComparisonPlan`: no normalization or mean removal is applied, and strict positive-z Parseval weights produce full, common-band, and newly-resolved-shell L2, H1, vorticity L2, and divergence L2 outputs. The fine-state absolute norms use an allocation-free copy of the same `diagnostics/norms.rs` scaled-squares operation order so relative budgets need no 1.36 GB zero-state allocation; small controls compare all four outputs bit-for-bit with `ComparisonPlan` against explicit zeros.

Admission guards are provenance, not trajectory semantics. When supplied on a
matched-spatial manifest they are preserved side by side in the output and may
differ; they do not participate in the original strict spatial equality gate.

The archived `control-n192-self.json` decodes the completed legacy N192 scheduled `node-4096/state.bin` from source `92effa6068d20d69e815c6a83f1e82490ce37fe7` twice. Both coefficient and whole-file hashes match its reviewed manifest; full/common/new-shell and mean errors are exact zero. Its allocation-free absolute norms are bit-identical to the node record (`l2=1.8388998858949468`, `h1=50.872398531129306`, `vorticity_l2=50.8391520359154`, `divergence_l2=1.0945737929461952e-14`). This is an adapter/writer/schema smoke control, not a new trajectory or cross-grid PDE result.

No matched cross-grid endpoint pair was complete in this isolated worktree at construction time. A later endpoint comparison should archive both reviewed input manifests, stdout JSON, the adapter source commit, and a hash manifest beside the scheduled evidence.
