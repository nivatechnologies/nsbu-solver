"""Small dense four-variable Taylor algebra, independent of production kernels.

Coefficients are derivative / multi-index factorial. All operands have the same
configured total degree, at most four. This implementation favors auditability.
"""
from dataclasses import dataclass
from itertools import product
from functools import lru_cache
from math import factorial
from mpmath import mp, mpf

Index = tuple[int, int, int, int]


@lru_cache(maxsize=5)
def indices(degree: int) -> tuple[Index, ...]:
    return tuple(sorted((i for i in product(range(degree+1), repeat=4) if sum(i) <= degree),
                        key=lambda i: (sum(i), i)))


@dataclass(frozen=True)
class Jet:
    degree: int
    coefficients: tuple[mpf, ...]

    def __post_init__(self) -> None:
        if self.degree not in (0, 1, 2, 3, 4):
            raise ValueError('Supported jet degrees are zero through four')
        if len(self.coefficients) != len(indices(self.degree)):
            raise ValueError('Incorrect coefficient count')

    @classmethod
    def constant(cls, value: mpf, degree: int = 4) -> 'Jet':
        return cls(degree, (mp.mpf(value),) + (mp.mpf(0),)*(len(indices(degree))-1))

    @classmethod
    def variable(cls, value: mpf, axis: int, degree: int = 4) -> 'Jet':
        if axis not in range(4) or degree < 1:
            raise ValueError('Invalid variable axis or degree')
        powers = indices(degree)
        coefficients = [mp.mpf(0)]*len(powers)
        coefficients[0] = mp.mpf(value)
        unit = tuple(int(i == axis) for i in range(4))
        coefficients[powers.index(unit)] = mp.mpf(1)
        return cls(degree, tuple(coefficients))

    @property
    def value(self) -> mpf:
        return self.coefficients[0]

    def _coerce(self, other: 'Jet | mpf | int') -> 'Jet':
        result = other if isinstance(other, Jet) else Jet.constant(other, self.degree)
        if result.degree != self.degree:
            raise ValueError('Jet degree mismatch')
        return result

    def __add__(self, other: 'Jet | mpf | int') -> 'Jet':
        right = self._coerce(other)
        return Jet(self.degree, tuple(a+b for a,b in zip(self.coefficients, right.coefficients)))

    __radd__ = __add__

    def __neg__(self) -> 'Jet':
        return Jet(self.degree, tuple(-a for a in self.coefficients))

    def __sub__(self, other: 'Jet | mpf | int') -> 'Jet':
        return self + (-self._coerce(other))

    def __rsub__(self, other: 'Jet | mpf | int') -> 'Jet':
        return -self + other

    def __mul__(self, other: 'Jet | mpf | int') -> 'Jet':
        right = self._coerce(other)
        powers = indices(self.degree)
        lookup = {power: position for position, power in enumerate(powers)}
        result = [mp.mpf(0)]*len(powers)
        for a, ca in zip(powers, self.coefficients):
            if not ca:
                continue
            for b, cb in zip(powers, right.coefficients):
                power = tuple(x+y for x,y in zip(a,b))
                if sum(power) <= self.degree:
                    result[lookup[power]] += ca*cb
        return Jet(self.degree, tuple(result))

    __rmul__ = __mul__

    def __pow__(self, exponent: mpf | int) -> 'Jet':
        if self.value <= 0:
            raise ValueError('Fractional jet powers require a positive constant')
        exponent = mp.mpf(exponent)
        delta = (self-self.value)*(1/self.value)
        term = Jet.constant(mp.mpf(1), self.degree)
        result = term
        factor = mp.mpf(1)
        for k in range(1, self.degree+1):
            term = term*delta
            factor *= (exponent-k+1)/k
            result = result + factor*term
        return result * self.value**exponent

    def __truediv__(self, other: 'Jet | mpf | int') -> 'Jet':
        return self * self._coerce(other)**mp.mpf(-1)

    def __rtruediv__(self, other: 'Jet | mpf | int') -> 'Jet':
        return self**mp.mpf(-1) * other

    def exp(self) -> 'Jet':
        delta = self-self.value
        term = Jet.constant(mp.mpf(1), self.degree)
        result = term
        for k in range(1, self.degree+1):
            term = term*delta / k
            result = result+term
        return result*mp.exp(self.value)

    def derivative(self, axis: int) -> 'Jet':
        if axis not in range(4):
            raise ValueError('Invalid derivative axis')
        powers = indices(self.degree)
        lookup = dict(zip(powers, self.coefficients))
        values = []
        for power in powers:
            source = tuple(v+int(i == axis) for i,v in enumerate(power))
            values.append((power[axis]+1)*lookup.get(source, mp.mpf(0)))
        return Jet(self.degree, tuple(values))

    def partial(self, power: Index) -> mpf:
        position = indices(self.degree).index(power)
        scale = 1
        for item in power:
            scale *= factorial(item)
        return self.coefficients[position]*scale
