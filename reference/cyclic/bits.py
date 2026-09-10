"""Bounded immutable force-bit fixtures, interpreted exactly before high-precision projection."""
from collections.abc import Mapping
import hashlib
from types import MappingProxyType
from mpmath import mp, mpf
from reference.dft import Mode, Spectrum, project, retained
from reference.cyclic.schema import decode_fixture
from reference.tuples import triple
from tools.json_types import Json, array_value, object_value

Pair = tuple[int, int]
VectorBits = tuple[Pair, Pair, Pair]


def integer(value: Json) -> int:
    """JSON booleans and nonintegers cannot stand for exact ticks or floating-point bits."""
    if isinstance(value, bool) or not isinstance(value, int):
        raise ValueError('Expected an integer')
    return value


def binary64(bits: int) -> mpf:
    """Decode finite binary64 by integer mantissa/exponent, without decimal or float conversion."""
    if not 0 <= bits < 1 << 64:
        raise ValueError('Binary64 word outside u64')
    exponent = (bits >> 52) & 2047
    if exponent == 2047:
        raise ValueError('Nonfinite binary64 input')
    mantissa = bits & ((1 << 52) - 1)
    if exponent:
        mantissa += 1 << 52
    sign = -1 if bits >> 63 else 1
    with mp.workdps(max(mp.dps, 17)):
        return mp.mpf(sign * mantissa) * mp.mpf(2)**(exponent - 1075 if exponent else -1074)


def mode_at(index: int, n: int) -> Mode:
    """Canonical Rust half-storage order; the excluded Nyquist coordinate is positive."""
    half = n // 2 + 1
    position = (index // (n * half), (index // half) % n, index % half)
    return triple(i if i <= n // 2 else i - n for i in position)


def vector(value: Json) -> VectorBits:
    """Require precisely three complex words and validate all finite bit patterns."""
    pairs: list[Pair] = []
    for item in array_value(value):
        parts = array_value(item)
        if len(parts) != 2:
            raise ValueError('Complex value requires two bit words')
        a, b = integer(parts[0]), integer(parts[1])
        binary64(a)
        binary64(b)
        pairs.append((a, b))
    return triple(pairs)


def sample(value: Json, n: int) -> tuple[VectorBits, ...]:
    """Require every coefficient exactly once in canonical storage order, with strict Nyquist zero."""
    rows = array_value(value)
    if len(rows) != n*n*(n//2+1):
        raise ValueError('Force sample shape mismatch')
    result: list[VectorBits] = []
    for index, item in enumerate(rows):
        row = object_value(item)
        mode = triple(integer(v) for v in array_value(row.get('mode')))
        if mode != mode_at(index, n):
            raise ValueError('Force modes are missing, duplicated or reordered')
        words = vector(row.get('bits'))
        if n//2 in tuple(abs(k) for k in mode):
            if any(word & ((1 << 63)-1) for pair in words for word in pair):
                raise ValueError('Excluded Nyquist coefficient is nonzero')
        result.append(words)
    hermitian_plane(result, n)
    return tuple(result)


class FixedForcing:
    """Complete immutable 33-clock fixture; its digest identifies bytes, not their physical truth."""
    __slots__ = ('_n', '_sha256', '_samples')

    def __init__(self, data: bytes, n: int, maximum_bytes: int) -> None:
        """Refuse oversized bytes before decoding; caller reserves parser and retained fixture storage."""
        if n not in (4, 8, 12) or len(data) > maximum_bytes:
            raise ValueError('Force fixture admission failed')
        root = object_value(decode_fixture(data))
        if integer(root.get('grid')) != n or integer(root.get('tick_exponent')) != -16:
            raise ValueError('Force fixture grid or clock mismatch')
        rows = array_value(root.get('samples'))
        if len(rows) != 33:
            raise ValueError('Expected all 33 exact stage clocks')
        samples: dict[int, tuple[VectorBits, ...]] = {}
        for index, item in enumerate(rows):
            row = object_value(item)
            if integer(row.get('tick')) != index * 4:
                raise ValueError('Force clocks are missing, duplicated or reordered')
            samples[index * 4] = sample(row.get('force'), n)
        self._n = n
        self._sha256 = hashlib.sha256(data).hexdigest()
        self._samples: Mapping[int, tuple[VectorBits, ...]] = MappingProxyType(samples)

    @property
    def n(self) -> int:
        """Validated retained grid; no public setter can change the fixture profile."""
        return self._n

    @property
    def sha256(self) -> str:
        """Digest of all supplied bytes, without a physical-authenticity assertion."""
        return self._sha256

    @property
    def samples(self) -> Mapping[int, tuple[VectorBits, ...]]:
        """Immutable checked coefficients on all declared clocks."""
        return self._samples

    def evaluate(self, time: mpf) -> Spectrum:
        """Interpret identical prescribed coefficients exactly, then independently project the full band."""
        scaled = time * 65536
        if not mp.isfinite(scaled) or scaled < 0 or scaled > 128:
            raise ValueError('Force time outside declared stage window')
        tick = int(scaled)
        if tick != scaled or tick not in self.samples:
            raise ValueError('Force time is not an exact declared stage clock')
        raw = {mode_at(i, self.n): triple(mp.mpc(binary64(a), binary64(b)) for a, b in words)
               for i, words in enumerate(self.samples[tick])}
        output: Spectrum = {}
        for mode in retained(self.n):
            values = (triple(mp.conj(v) for v in raw[triple(-k for k in mode)])
                      if mode[2] < 0 else raw[mode])
            output[mode] = project(mode, values)
        return output


def zero_word(word: int) -> int:
    """Both signed zeros represent the same coefficient for reality admission."""
    return word if word & ((1 << 63)-1) else 0


def hermitian_plane(values: list[VectorBits], n: int) -> None:
    """Stored kz=0 coefficients must be exact conjugate pairs, as in the smooth producer."""
    half = n//2+1
    for x in range(n):
        for y in range(n):
            left = values[(x*n+y)*half]
            right = values[((-x) % n*n+(-y) % n)*half]
            for (a, b), (c, d) in zip(left, right):
                if zero_word(a) != zero_word(c) or zero_word(b) != zero_word(d ^ (1 << 63)):
                    raise ValueError('Force fixture violates Hermitian reality')
