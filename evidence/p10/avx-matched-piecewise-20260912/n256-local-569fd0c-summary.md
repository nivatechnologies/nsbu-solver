# Matched piecewise local N256 endpoint archive

Run `n256-local-569fd0c`; source commit `569fd0ced7a755b1f6c066a7c00016f537fbc4bf`; case SHA-256 `e1236f7b3c51537acd17381402ca420ba7872a7b9dbc64b2f0d9d5108a468f7e`; profile `n256-m384-h64to2048-h128to4096-cadv33-w3-f13c29c`. Retained N256, M384 integration force, M768 observer force, M512 observer conservative, 32 workers, rustfft 6.4.1 AVX/AVX2/FMA, separate RHS/force W3 providers, Cox–Matthews h64 then h128; Cadv 3.3.

Exact schedule: attempts 1–32 use 64 ticks over [0,2048), attempts 33–48 use 128 ticks over [2048,4096). All 48 attempts are committed and contiguous to endpoint 4096. Nine observer nodes are 0, 512, 1024, 1536, 2048, 2560, 3072, 3584, 4096. Steady allocations are zero and no refusal records/output were present. Terminal stdout reports `endpoint_complete_qualification_pending clock=4096`; timed process exit status is 0.

Runtime telemetry: GNU time reports elapsed 2:02:33, user 74782.63 s, system 50.12 s, and maximum RSS 67776512 KiB. Runtime stdout SHA-256 `39990fc0e58a080a7d86043218b7e5c8f7a733703ad72b07e7ae35d63f8a1fc0`; stderr is empty with SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`.

Identity hashes: plan `daf8f611baea051a387e1b1e62b41595f27fac1743975bda99a48b687377da2e`; binary `cebb88ed242881f016edf3488d74c31d1f2f3f57c9a08845468d9b8ddaa5f7d1`; watchdog `4b65e13b74bd7a32237044b5467d4f223c8dc3e6e35d6e8d3498e1938fa53e4b`; preflight `9e2e5de4343bf90b9370ff8c0b6effd619072a26766fc360e293626402e9b438`; launch script `2a6664d8fbbe8fd34e7ba530df8a7de572dba96177736fec6508bdb6160735df`.

Raw state bundles remain local and excluded from Git. The inventory binds all 48 attempt records, records, and state payloads. Packaging evidence only; qualification remains experimental/pending and no numerical interpretation is asserted.
