# N512/M512 v4 parallel-FFT capture preparation

This packet implements but does not deploy the replacement N512/M512 capture
trajectory. The live Sulaco v3 run and its stage are untouched. Numerical
library source `477c418d30d9f2c8b117ae05240fc5596ecbd33b` provides the validated
shared W3 executor; constructor factoring source
`1ed699568be70dedf72492324be08600d4407c02` changes only construction helpers.
The capture harness RUN_SOURCE is
`431bddf26d868823663b26596d887cef12b2c254`.

The profile starts from exact rest at clock exponent -20 and target 8192. It
retains the 32 steps of 64 through clock 2048, 16 steps of 128 through 4096,
Cox--Matthews, tolerances `[1e-5,1e-4]` and `[1e-5,1e-5]`, advective limit 3.3,
48 transactional state publications, eight offline observer markers, and no
resume decoder. RHS M768 and force M512 each own one eight-helper Rayon pool
entered by three persistent W3 callers, for 11 pool workers per owner.

The capture harness exact API reports 207,627,451,152 bytes: catalog 29,362,480,
force storage 29,180,171,864, RHS 91,860,200,792, attempt 30,182,212,728, and
capture overhead 1,310,736. One byte under is refused. The host memory floor is
241,987,189,520 bytes, exact peak plus 32 GiB. State payload remains
155,226,537,984 bytes. A shared source/archive filesystem must have at least
344,812,814,336 bytes free before launch.

`proposed-launch/v4-launch-plan.json` and `v4-launch-receipt.json` bind the
binary, exact preflight, source, unchanged v3 identity watchdog, archive helper,
and launcher. A future deployment supplies a fresh 20-hour absolute deadline;
the current v3 deadline is not reused or changed. The v4 launcher also checks
clock 64 has 12 timed RHS calls, seven cache hits, five misses, zero steady
allocations, integration no longer than 1200 seconds, and coefficient hash
`5df15fc393b5224170c016aa48cbaad4336e0fbc74d380b5f6c72b5fbe9d21ff`
before continuing.

Focused commands completed with status 0:

```
RUN_SOURCE=431bddf26d868823663b26596d887cef12b2c254 cargo test --manifest-path evidence/p10/avx-scheduled-endpoint/harness/Cargo.toml --features n512-m512-piecewise-cadv33
RUN_SOURCE=431bddf26d868823663b26596d887cef12b2c254 cargo clippy --manifest-path evidence/p10/avx-scheduled-endpoint/harness/Cargo.toml --features n512-m512-piecewise-cadv33 --all-targets -- -D warnings
cargo test -p nsbu-solver w3_parallel
RUN_SOURCE=431bddf26d868823663b26596d887cef12b2c254 cargo test --manifest-path evidence/p10/avx-scheduled-endpoint/harness/Cargo.toml --features n384-m512-piecewise-cadv33
RUN_SOURCE=431bddf26d868823663b26596d887cef12b2c254 cargo build --release --manifest-path evidence/p10/avx-scheduled-endpoint/harness/Cargo.toml --features n512-m512-piecewise-cadv33
```

The default launcher refusal is 64. Its deadline, v3 identity, stable owner,
delayed setsid, archive timeout, TERM-resistant child cleanup, and archive
collision tests all pass. No large attempt, remote copy, signal, switch, or
deployment was performed. The separately running Baccus attempt remains an
external validation input and is not represented as completed here.
