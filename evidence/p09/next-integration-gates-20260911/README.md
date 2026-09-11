# P09 adapter/export integration gates — 2026-09-11

The requested non-numerical integration gates passed against exact source commit
`99cf7f8cb4969b98fd27dab7c27cf87b0c12363b`. Commit
`cc1a137efaf71731c7803221d40c5056207a5557` landed afterward and changes only
project documentation; it was not part of this source-bound run. No binder
source was present in the tested tree.

The archived logs replace the local worktree and temporary packaging directory
with `$WORKTREE` and `$PACKAGE_TARGET`. `source-tree.txt` records the Git object
identity for the complete checked Rust workspace, manifests, lockfile,
repository checker and Rust CI workflow. The full Rust metrics JSON Lines report
is retained for independent recomputation.

These checks do not include numerical tests or coverage and make no scientific
qualification claim.
