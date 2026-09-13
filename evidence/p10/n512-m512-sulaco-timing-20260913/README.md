# Sulaco N512/M512 scratch-tail timing preparation

Status: prepared and **not launched**, pending root review of the frozen launcher.

This package stages the existing release executable byte-for-byte on Sulaco for one
independent-host timing and candidate-coefficient hash observation. It uses the
frozen rest-to-64 Cox--Matthews profile: retained N=512, force M=512, 32 workers,
one attempt, advective limit 3.3, absolute tolerances `[1e-5,1e-4]`, and relative
tolerances `[1e-5,1e-5]`. The result is diagnostic cross-host evidence. It does not
qualify a PDE window and it never commits or publishes candidate state.

The staged executable SHA-256 is
`635726f54b5078fa6a2093c812a4750ce904d67e517728f58e82a9b2f003d9f0`.
Its production source is `0843b8b`, test-only source is `9eba11f`, and harness
adaptation is `f82df1b`. `source-sha256.list` binds all 13 inputs, including the
inherited `cache.rs` and `timed_rhs.rs` files. No rebuild occurred.

Sulaco's allocation-free preflight exited 0 and its stdout SHA-256 exactly matches
the local frozen preflight (`0892ddc0...`). It reported 207,576,840,688 internal
bytes. The remote admission floor adds a predeclared 32 GiB operational headroom:
241,936,579,056 bytes, rounded upward to 236,266,191 KiB. AS remains 256 GiB.
At the final preparation check Sulaco had 251,239,824 KiB MemAvailable, 1.581 TB
free in the staging filesystem, and no matching active numerical executable.

Local and Sulaco linked-library hashes are identical for `libgcc_s`, `libm`,
`libc`, and the ELF loader; both report glibc 2.39-0ubuntu8.9. The executable was
built by rustc 1.94.0 for x86_64-unknown-linux-gnu without an explicit target-cpu
override. The local host is an EPYC 7702P; Sulaco is an EPYC 7C13 and exposes
AVX/AVX2/FMA. Sulaco has no rustc installation, which does not affect execution.
No system library is copied or replaced.

The remote stage is `/tmp/nsbu-p10-sulaco-n512-m512-scratch-tail-20260913`.
`launch-reviewed.sh` requires its exact opt-in variable, source and executable
hashes, quiet process state, resource and disk floors, and at least 3,060 seconds
before the absolute 2026-09-13T19:52:54Z deadline. It binds the worker PID/PGID,
start time, and exact NUL-delimited argv twice before recording ownership. Its
identity-guarded trap terminates only the owned process group on interruption.

