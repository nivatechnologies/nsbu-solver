# Local Qwen3.8 Flash Next pilot packet

This packet prepares one narrow two-agent pilot for two local DGX Spark Qwen3.8
Flash Next agents. It contains no endpoint, daemon, model launch, implementation,
or numerical evidence. The operator supplies endpoint details later; absent a
local LAN inference endpoint, the packet stays dormant. `remote_execution: false`
means no worker SSH, remote shell, solver launch, or daemon launch; it allows
calls to the operator-supplied Spark LAN inference endpoint.

Task runners, Codex clients, isolated worktrees, Rust checks, and numerical
validation remain on the current x86_64 server using its existing AVX backend
and toolchain. Sparks provide inference only; building or validating ARM
numerical substitutions on Sparks is forbidden.

The implementer may take only the reviewed regional force-correction plan from
commit `1f49f47` as its real bounded backlog task. The diagnostic compares
`N384/M512` with `N384/M384` at clock 512 on the fixed unshifted `M768` lattice
and five exclusive regions. It must report mathematically valid triangle bounds
for RMS and peak quantities, plus peak bounds normalized by the same fixed
archived floors. Vector alignment, cancellation, extrapolation, and PDE-window
acceptance are excluded. The output is a small-fixture diagnostic receipt. A
full `M768` run is forbidden until the Astra root gate explicitly passes, and
is not part of this packet.

The reviewer works independently at the same immutable base, applies the
implementer patch, hashes the applied patch and inputs, then reviews the diff,
tests, resource preflight, and output semantics before any heavy execution.
Independent controls may use the same model. Each task has a hard attempt and wall-clock cap. A failed attempt
leaves its receipt; retries apply only to the listed transient failures.

Both agents use disposable worktrees. The only default write path is
`evidence/pilot-local-qwen38/regional-force-correction/`; frozen `evidence/p10/`
inputs are read-only. Active `work/`, model files, numerical binaries, trajectory
logs, checks, and acceptance budgets are forbidden. Receipts must
bind the reviewed inputs, source identity, model identity, redacted local
endpoint label, resource preflight, timing, output hashes, and unperformed
work. Credentials are never recorded.

The JSON file is the authoritative packet. It records the requested label
`Qwen3.8 Flash Next` separately from the server-reported model ID; no API model
string is guessed. The root numerical orchestrator
controls start/retry and the merge or any public claim. The operator controls
endpoint details. Cloud fallback, remote execution, scope expansion, and
automatic acceptance are prohibited.
