# README diagnostic figure

The figure plots eight independently started HO runs from the published
`alpha-20260911` binary, sampled at 4-step increments through 32 steps.
All use the default N=4/M=4 similarity-mms-v2 profile. The zero point is the
prescribed initial condition, not a separately measured runtime report.
Lines connect measured points; no interpolation accuracy is asserted.
Energy and enstrophy use the runtime’s dimensionless conventions.

The JSON reports and binary hash are retained here. Runtime checkpoints were
used only to stop the diagnostic at each requested sample and are not distributed.
This coarse profile is not spatially/force resolved and establishes no accepted
PDE window or blow-up result.

Re-render from the repository root with Python 3 and matplotlib 3.10.8:

```sh
python3 evidence/runtime-alpha/readme-visual/render.py
```

Matplotlib is an optional figure-generation dependency, not a solver requirement.
