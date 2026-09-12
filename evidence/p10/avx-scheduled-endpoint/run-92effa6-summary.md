# N192/M384 h32 Cadv0.45 AVX scheduled endpoint archive

Run `run-92effa6`; source identity `codex/p10-fft-batch-20260912@92effa6068d2`; case SHA-256 `e1236f7b3c51537acd17381402ca420ba7872a7b9dbc64b2f0d9d5108a468f7e`.

Profile: retained N192, M384 integration force, M768 observer force, 32 workers, rustfft 6.4.1 AVX/AVX2/FMA, parallel-reduced-attempt-cache, Cox–Matthews h32, endpoint clock 4096, 128 maximum attempts, Cadv 0.45. Frozen plan SHA-256: `43a9b4f38e9a4da8c885f4adbf4bd4154c02e2374228bf648dd215195bdc265b`.

Terminal evidence: endpoint marker is `endpoint_complete_qualification_pending` at clock 4096; timed process exit status 0. All 128 logical attempts are committed over [0,4096] in 32-tick intervals; eight scheduled attempt records are published in node directories, giving 128 records total. Allocation count is 0 and refusal count is 0. Watchdog reports `solver_exited_or_identity_changed,no_signal_sent`. Stderr is empty.

Exact artifact hashes: stdout `781a4ce20bebebba73ff521c67b6589c0d0434b895b98828b9e86cf1f9c4316e`; time `b00aba7a7ba1ffa1b307848f6fe14cc62df0b39a361a58d4390959a94a42e8b0` (elapsed 3:57:56; max RSS 45,168,640 KiB); stderr `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`; launch identity file `14bd0b17e1c010c90aa4cdf8d0f7a16020b887ff7d5022896b7ee89f14780a99`, binary `b9dc07fde45dd49a831a881ccf13eebed2afe3d27ceef3f33452f6b2948b63f1`; watchdog identity `b61690826c1e37fbaab6764ee2d2420301cc7a87b6835eebbe0ac59146017b6d`, terminal `b6b188802c05082d40e06fa5b6a00f8c4b3e4b3e9de889b7d28ea604c4f696c9`; preflight stdout/time `f598ed01260c73f65a40df7d7ad0e062362c11dc7560c873f77f6a72d49c66d6` / `465cf125089265e48b49abbceaf3b6a2c3a70eccd1550c753935aa7574243358`.

State payloads remain locally in the run directory and are excluded from Git. Their hashes, node record hashes, and all attempt record hashes are in `run-92effa6-attempt-inventory.tsv`. Packaging evidence only; no numerical interpretation.
