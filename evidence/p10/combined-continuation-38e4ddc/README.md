# Combined continuation integration reruns

This archive binds the local combined-branch reruns for the accepted nominal-coverage integration and standalone shared-force adapter. Adapter tests and allocation checks ran at `e83d685`; nominal coordinator/export/region tests, allocation, formatting, all-target Clippy and repository validation ran after both source increments were present at `eb4e60d`. Rust source and manifests are unchanged from `eb4e60d` through the documentation-only `38e4ddc`, where whole-workspace static metrics were measured.

The adapter and nominal packages retain their separate exact-source coverage/CRAP evidence. This combined archive does not substitute for the pending source-matched whole-workspace hosted coverage gate. It changes no release, qualifies no PDE window, and preserves zero accepted windows.
