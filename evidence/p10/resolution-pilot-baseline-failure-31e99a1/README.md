# Baseline first-endpoint pilot failure at `31e99a1`

This archive preserves the bounded N12/N16/N24 spatial and single N12 M24-to-M48 force-control pilot exactly as it failed. The run started from rest at 2026-09-12 00:21:58 UTC and terminated at 01:01:35 UTC after 2376.97 seconds with `Family(Numerical(InvalidSpectrum))`. It emitted only the rest event and the offstage clock-2047 probe/residual event. It did not publish accepted clock 2048 or endpoint 4096, and its only force-control row was the exact rest zero.

The harness printed a diagnostic terminal but returned shell status 0. That status is misleading and is explicitly not a pass; later harnesses must exit nonzero for a diagnostic failure. The 4 GiB admission included 1,393,212,608 bytes for the coordinator, 120,898,776 bytes for the independent high-force run, and 552 bytes of owned comparison/hash scratch. The measured 575,488 KiB maximum RSS is process usage, not an allocation reservation.

The separate 0.44-second, 256 MiB-cap isolator evaluated the unchanged original force at exact clock 2048 on retained N24 for M24 and M48. All source spectra were finite, Nyquist-zero, and within their source Hermitian check. Physical x/y derivatives then reproduced `InvalidSpectrum`; the retained mode-level witness is in `summary.json` and the complete raw output. This identifies a derivative-staging contract mismatch consistent with the pilot failure, while it does not prove which coordinator consumer failed first.

`raw/full/` contains deterministic-gzip stdout, stderr, GNU time, terminal status, exact UTC boundaries, dry-run admission, and the prelaunch source freeze. `raw/isolator/` contains the before-fix reproducer output and timing. `source/` contains both exact harnesses and lockfiles. `source-sha256.json` binds every retained artifact.

This is failed feasibility evidence only. It contains no first-endpoint refinement result and makes no convergence, force-sufficiency, PDE-qualification, or accepted-window claim.
