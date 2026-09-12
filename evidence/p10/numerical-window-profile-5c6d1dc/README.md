# Concentrating-run FFT bottleneck profile

The fixed-M96 N64/N96 baseline profiles showed low whole-process CPU use after
the parallel force workers quiesced. Isolated scalar transforms confirmed that
the owned serial FFT dominated the integration and doubled-grid observer work.

Source `1b89176f63737bbeb3ed65c7e3c7fe6dbeef8f02` specializes the existing
radix-2 and radix-3 combination loops without changing recursion, row order,
roots, normalization, storage, allocation, or the public API. Four-transform
profiles improved by 2.03x at M96, 2.06x at M144, and 2.08x at M192. All three
optimized output hashes equal the baseline hashes. A complete four-attempt N64
trajectory improved from 146.23 to 94.20 seconds and reproduced the baseline
clock-64 snapshot byte-for-byte.

Independent controls cover every stored mode on an anisotropic direct-DFT
fixture, lengths 6/18/144, Hermitian and Nyquist planes, signed zero, large
finite values, overflow refusal, repeat determinism, steady allocation, and the
existing direct convolution oracle. The full `nsbu-solver` test set and its
all-target Clippy check pass. This is a bit-identical implementation speedup,
not improved-arithmetic evidence and not a concentrating-window result.
