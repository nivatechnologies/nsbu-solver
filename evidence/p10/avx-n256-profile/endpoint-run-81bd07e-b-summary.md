# Old local N256/M384 h32 AVX endpoint archive

Run `endpoint-run-81bd07e-b`; source identity `codex/p10-fft-batch-20260912@81bd07ec0e52`; case SHA-256 `e1236f7b3c51537acd17381402ca420ba7872a7b9dbc64b2f0d9d5108a468f7e`; profile `n256-m384`; retained N256, M384 integration force, M768 observer force, 32 workers, rustfft 6.4.1 AVX/AVX2/FMA, parallel-reduced-attempt-cache, Cox–Matthews h32.

Terminal marker is `endpoint_complete_qualification_pending` at clock 4096. All 128 attempts are committed over exact contiguous 32-tick ranges [0,4096], with nine observer nodes (0, 512, 1024, 1536, 2048, 2560, 3072, 3584, 4096). Steady allocations are zero and no refusal records/output were present. Process exited with status 0. The `.time` sidecar is empty; completed resource telemetry records elapsed 25462 seconds and maximum RSS 63335844 KiB.

Identity hashes: binary `6d939d4eef61d9b4cfd303319eafe689349eb82ec0801db11a053831427c6c27`; frozen profile `65e5aeb68900da0c8057dbf3c1cf2d2ccc76e0700328ff86104bf87321fa184f`; preflight stdout `131b2f5083c914170814c3a5ebfd8006377d083abe1b2a27a806fbb531c9b1e4`; watchdog script `09b860be378c0c9b658ffde006200a581de87c0eef9598bbb096c72b664868fb`.

Runtime hashes: stdout `39990fc0e58a080a7d86043218b7e5c8f7a733703ad72b07e7ae35d63f8a1fc0`; empty stderr `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`; empty time sidecar has the same empty-file hash; launch identity file `f13b8e9c7a725f93c01565a292c5228b714f9a7504848c4223bb506c2c728d89`; watchdog identity file `b12e7bdfca759ff851830757852fb16fc1b182eab5d3f91278c6a4414886cc82`; resource log `b31add904b543e0f18c0bc709ac61620854d135d60cbd05cbf54d086f2757492`; status correction `86373a6257c4069e78c69ce37f0a73fc90f095e7b576e8242b15abe89cbe8c72`.

Raw states remain local and excluded from Git. The inventory binds all 128 attempt records and nine node record/state payloads. This is packaging evidence only; qualification remains experimental/pending and no numerical interpretation is asserted.
