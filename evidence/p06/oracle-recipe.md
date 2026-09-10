# Independent concentrating trajectory fixture recipe

Run the preserved Python reference implementation from the repository root with
Python 3.12 and the pinned development requirements. No Rust code participates in
fixture construction. Run each precision separately:

```python
import json
from pathlib import Path
from reference.pilot import pilot

for precision in (80, 120):
    result = pilot(4, 16384, precision, 'CM', 1024**3)
    Path(f'work/p06-python-{precision}.json').write_text(
        json.dumps(result, indent=2) + '\n')
```

The resource preflight covers the allocating Python diagnostic and its finite
force cache. Each trajectory starts from exact rest and takes 64 independent CM
steps of 1/16384, reaching 1/256. Its 256 RHS calls use the independent direct-DFT
and coefficient-convolution path. Reference tracking is computed only after the
integrated state has been produced. Local full/two-half acceptance is a Rust
transaction policy and is not part of this fixed-step reference trajectory.

Require both reports to be diagnostic-completed, with 64 steps, 256 RHS calls,
elapsed and remaining ticks both 4096 on the 2^-20 clock, and zero reference
assignments. Compare every complex coefficient at 120-digit precision using
abs(low-high)/(1+abs(high)); require the maximum to be below 1e-60 before export.
Export the 18 strict retained modes with nonnegative third coordinate from the
120-digit state, recording all three complex components without binary64
conversion. Preserve the two source reports, the exported fixture SHA-256 and
maximum precision difference in the package evidence.

The corresponding Rust trajectory commits 32 fine results, each obtained from
two half steps. The committed path therefore contains the same 64 CM steps.
The additional 32 coarse comparison steps account for the Rust run's total of
384 RHS calls. Compare every exported coefficient with an explicit binary64
scaled tolerance. Successful arithmetic agreement validates this coarse discrete
trajectory comparison, not the continuum PDE, force sampling or any endpoint.
