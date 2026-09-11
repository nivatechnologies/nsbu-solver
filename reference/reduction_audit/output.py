"""Strict exact-word production statistic/magnitude packets, bound to one full input digest."""
from dataclasses import dataclass
import hashlib
from reference.reduction_audit.input import SampleInput

HEADER=488


@dataclass(frozen=True)
class Magnitudes:
    """Imported audit outputs; digest agreement is content binding, not execution authentication."""
    data: bytes
    points: int

    def word(self, group: int, role: int, point: int) -> int:
        """One complete error or reference magnitude word at its original lattice point."""
        if not 0<=group<6 or not 0<=role<2 or not 0<=point<self.points:
            raise ValueError('Magnitude index outside complete packet')
        return integer(self.data,HEADER+8*((group*2+role)*self.points+point))

    def statistics(self, group: int, path: int) -> tuple[int,...]:
        """RMS, sampled absolute peak, pointwise relative peak and reference peak, in that order."""
        if not 0<=group<6 or not 0<=path<2:
            raise ValueError('Statistic index outside complete packet')
        offset=104+8*(group*8+path*4)
        return tuple(integer(self.data,offset+8*i) for i in range(4))


def integer(data: bytes, offset: int) -> int:
    """Private fixed-width access after complete outer shape validation."""
    return int.from_bytes(data[offset:offset+8],'little')


def parse(data: bytes, source: SampleInput) -> Magnitudes:
    """Require all header identities and nonnegative finite statistics/magnitudes before use."""
    if len(data)!=HEADER+12*source.points*8 or data[:8]!=b'NSBUMAG1':
        raise ValueError('Invalid magnitude packet magic or complete length')
    if integer(data,8)!=source.n or integer(data,16)!=source.points:
        raise ValueError('Magnitude lattice disagrees with the actual sample input')
    if data[24:56]!=hashlib.sha256(source.data).digest():
        raise ValueError('Magnitude packet belongs to a different exact input')
    floors=tuple(integer(data,56+8*i) for i in range(6))
    if floors!=source.floor_words:
        raise ValueError('Magnitude relative floors disagree with the original exact words')
    for offset in range(104,len(data),8):
        word=integer(data,offset)
        if ((word>>52)&2047)==2047 or (word>>63 and word&((1<<63)-1)):
            raise ValueError('Negative or nonfinite audit statistic or magnitude')
    return Magnitudes(data,source.points)
