# nsbu-solver

Public NSBU Solver library with geometry, half-spectrum layouts, exact dyadic
clocks, checked epochs, resource reservations and independently owned from-rest
state buffers. Spectral operators provide normalized transforms, strict-band
three-halves transfers, projection, curl, rotational acceleration and physical
pressure. Time integration remains planned. Licensed under Apache-2.0, with no
private Niva dependency.

The built-in CPU FFT supports lengths at most 1024 with only factors 2 and 3.
Both retained and padded layouts must fit that profile. Planning reserves owned
roots and scratch before allocation; transforms and operator evaluations do not
allocate. Callers must separately budget input/output and allocator overhead.
Operator outputs are scratch and become invalid on error; input state is immutable.
