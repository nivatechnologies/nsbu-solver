# Exact-v2 off-stage residual consumer

`v2_experiment::probes::residuals::ResidualFamily` consumes complete reports
from one admitted exact-v2 `ProbeFamily`.  At each declared genuine non-stage
clock it computes six unprojected momentum defects and publishes the six norms,
five complete doubled-band comparisons and real accepted-node geometries as one
transaction.  The default exact-v2 family, CLI and checkpoint formats remain
unchanged.

Admission verifies every clock against the reconstruction manifest, constructs
all six three-node off-stage geometries, verifies the nested temporal refinement,
and reserves the six private residual workspaces, comparison scratch, complete
attempt work and joint probe-plus-consumer storage.  A charged binding,
geometric or numerical failure clears the previous publication and terminates
the consumer.  Rejected reconstruction families and foreign identities cannot
publish residual fields.

Each child retains its residual on its own conservative doubled band: `N=4`,
`N=8` and `N=12` therefore produce `2N=8`, `2N=16` and `2N=24` residual grids.
All six children evaluate the original serial `V2Force` on one fixed `M=24`
sampling grid derived from the probe family's configured force grid.  Admission
requires that grid to cover the finest doubled band.  Spatial, temporal and
method residual comparisons therefore hold the diagnostic force input profile
fixed; each branch still performs a fresh force evaluation at the actual probe
clock.

The focused operator oracle explicitly convolves signed Fourier modes, applies
the Helmholtz projection, viscosity and reconstructed time derivative, and
compares a late clock-95 residual coefficient against the stored doubled-band
field.  It also checks force omission, force sign reversal and nonlinear
omission controls.  The force is freshly evaluated with the same admitted
`M=24` source profile.  The independent arithmetic covers the tested modal
assembly; it does not replace broad force, transform or all-mode qualification.

This increment does not import external reconstruction, assemble a PDE
acceptance window, qualify continuous-time convergence, or establish a pressure
mean or gauge.  P09 and P10 remain incomplete.
