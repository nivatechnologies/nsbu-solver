# V2 M192 worker correctness and timing study

This frozen-source study evaluates the original M192 prescribed force retained on
N24 at clock 4096 with serial, 12-worker and 32-worker providers. It compares
all output coefficient binary64 words, not just hashes. The 32-worker provider
also evaluates nonmonotone clocks 2047, 4096 and 2047 again. No trajectory,
state, nonlinear product or full RHS is involved.

Serial, 12-worker and 32-worker endpoint coefficients are exact-word equal. The
32-worker first/repeated 2047 payloads are exact-word equal. At 4096 the measured
force times are 150.024862s serial, 15.498093s with 12 workers and 8.755700s with
32 workers. All endpoint calls report work 7,078,668 and three transforms.

The executed harness contains one deliberately retained transcription error: its
expected archived M192/4096 SHA byte is `c9`, while the source four-grid raw has
`9c`. It therefore printed `archived12_match=false` for that one check. The
separate [erratum postprocessor](erratum/reconcile.py) reads the archived raw,
this study raw, and the unchanged executed harness; its JSON proves the actual
archived hash equals all observed serial/12/32 hashes and that only the control
constant differs. The harness is preserved unchanged and is not relabelled as a
corrected execution.

The 32-worker provider is a tested explicit profile, not an automatic portable
choice. Even at 8.755700 seconds per M192 call, a single N24/h16 CM trajectory
to clock 4096 would require 3,072 RHS calls and roughly 7.5 hours of force
sampling alone. No M192 trajectory is launched by this evidence.

To reproduce the executed harness exactly, copy `executed-harness/` as a direct
child of a checkout at the recorded source commit, then run:

```sh
/usr/bin/time -v timeout 900s cargo run --release \
  --manifest-path executed-harness/Cargo.toml
```
