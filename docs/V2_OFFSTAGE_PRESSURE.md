# Exact-v2 reconstructed off-stage pressure

`v2_experiment::probes::pressure` consumes a complete current `ProbeFamily` publication. Before any pressure or force arithmetic it binds the producer and sample identities, exact manifest clock, all six source domains, and every accepted-node origin. It uses only each branch's reconstructed velocity value; the reconstructed time derivative is not pressure input.

The diagnostic fixes the source to the finest retained family domain and uses its doubled layout for both pressure coefficients and one fresh original exact-v2 force sample. The configured trajectory worker count is inherited. One shared conservative workspace and two pressure buffers construct the ten pair sides sequentially. Pressure and complete pressure-gradient errors are then sampled on the separately configured physical layout.

The consumer owns no gauge or analytical reference. Its five differences use one common force field, so the common force contribution cancels mathematically; this does not independently qualify force sampling or establish a pressure mean. Reports retain the exact reconstruction publication and its six origins. A failed attempt is charged, terminates the consumer, exposes no partial report, and preserves the last complete report.

The focused control compares an actual late off-stage sample against independent full-complex mode extraction, direct nonlinear convolution, and direct inverse DFT. That oracle is independent of `ConservativeWorkspace` and `construct_pressure`; it checks pressure and gradient differences, while the separate binding and force metadata checks cover the original-force wiring. The result is diagnostic evidence only and does not establish pressure convergence or a concentrating PDE window.
