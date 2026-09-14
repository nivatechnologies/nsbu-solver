# Baccus parallel FFT attempt

This authorized attempt completed 64 ticks from rest with exit status zero. It used eight helper
workers and three persistent callers per executor, reported zero steady allocations, and produced
candidate coefficient SHA-256 `5df15fc393b5224170c016aa48cbaad4336e0fbc74d380b5f6c72b5fbe9d21ff`.
That hash, both local error ratio words, and the 12-call/5-miss/7-hit cache counters exactly match the
earlier scalar-FFT attempt.

The parallel attempt measured 1,262.211683181 integration seconds, 1,158.127429176 timed-RHS
seconds, and 1,456.83 wall seconds. Against the earlier shared-host scalar trial (1,470.457289408
integration, 1,371.158965519 RHS, 1,655.39 wall), these are reductions of 14.161962%, 15.536604%,
and 11.994757%, respectively. These single trials were not run concurrently or on an exclusive
host, so the timing comparison is diagnostic rather than a controlled benchmark.

The attempt did not commit or publish the candidate. This evidence does not qualify a PDE state or
by itself authorize deployment. The top-level prepared receipt remains frozen at its historical
pre-launch status.
