"""Exact sampled-word audit packets; this format is diagnostic data, never a checkpoint."""
from dataclasses import dataclass
import hashlib
from pathlib import Path
import struct
from mpmath import mp
from reference.cyclic.bits import binary64, integer
from reference.cyclic.comparison import bounded_bytes, MAXIMUM_INPUT_BYTES
from reference.derived.schema import derived
from tools.json_types import Json, JsonObject, array_value, decode, object_value

MAGIC=b'NSBURED1'
HEADER=136
ROWS=46
FLOORS=('1e-8','1e-7','1e-6','1e-7','1e-8','1e-7')
CAP_BYTES=2*1024**3


def preflight(n: int, cap_bytes: int=CAP_BYTES) -> JsonObject:
    """Admit bounded source parsing/validation and a complete packet before reading either input."""
    if n not in (4,8,12):
        raise ValueError('Reduction audit requires N=4,8,12')
    points=(2*n)**3
    reserved=2*MAXIMUM_INPUT_BYTES*24+2*ROWS*points*768+128*1024**2
    if reserved>cap_bytes:
        raise ValueError('Sample-word preparation exceeds its conservative Python reservation')
    return {'grid':n,'samples':2*n,'physical_points':points,'source_rows_per_role':ROWS,
            'packet_bytes':HEADER+2*ROWS*points*8,'reserved_bytes':reserved,'cap_bytes':cap_bytes,
            'classification':'conservative Python planning; no hard allocator guarantee',
            'roles':['CM','HO'],'accepted_pde_windows':0}


def prepare(left: Path, right: Path, n: int, cap_bytes: int=CAP_BYTES) -> bytes:
    """Validate both complete actual-export schemas, retain every bit including signed zeros."""
    preflight(n,cap_bytes)
    raw=(bounded_bytes(left),bounded_bytes(right))
    roots=tuple(object_value(decode(value.decode())) for value in raw)
    with mp.workdps(80):
        a,b=(derived(root,n,method) for root,method in zip(roots,('CM','HO'),strict=True))
        if a.force!=b.force:
            raise ValueError('The two sampled trajectories must have identical prescribed endpoint force')
    header=MAGIC+n.to_bytes(8,'little')+((2*n)**3).to_bytes(8,'little')
    header+=b''.join(struct.pack('<d',float(floor)) for floor in FLOORS)
    header+=b''.join(hashlib.sha256(value).digest() for value in raw)
    packet=bytearray(header)
    for root in roots:
        for item in array_value(root.get('fields')):
            for value in array_value(object_value(item).get('bits')):
                packet.extend(integer(value).to_bytes(8,'little'))
    return bytes(packet)


@dataclass(frozen=True)
class SampleInput:
    """Complete immutable little-endian component words and original JSON artifact hashes."""
    data: bytes
    n: int
    points: int
    floor_words: tuple[int,...]
    source_hashes: tuple[str,str]

    def word(self, role: int, row: int, point: int) -> int:
        """Bound each index before interpreting a finite binary64 word."""
        if not 0<=role<2 or not 0<=row<ROWS or not 0<=point<self.points:
            raise ValueError('Sample-word index outside the declared complete packet')
        offset=HEADER+8*((role*ROWS+row)*self.points+point)
        return int.from_bytes(self.data[offset:offset+8],'little')


def parse(data: bytes) -> SampleInput:
    """Require the entire fixed schema, finite words and positive exact-word relative floors."""
    if len(data)<HEADER or data[:8]!=MAGIC:
        raise ValueError('Invalid reduction-input magic or truncated header')
    n=int.from_bytes(data[8:16],'little')
    points=int.from_bytes(data[16:24],'little')
    preflight(n)
    if points!=(2*n)**3 or len(data)!=HEADER+2*ROWS*points*8:
        raise ValueError('Reduction-input length or sample lattice mismatch')
    floors=tuple(int.from_bytes(data[i:i+8],'little') for i in range(24,72,8))
    if any(binary64(word)<=0 for word in floors):
        raise ValueError('Every relative floor must be positive and finite')
    for offset in range(HEADER,len(data),8):
        word=int.from_bytes(data[offset:offset+8],'little')
        if ((word>>52)&2047)==2047:
            raise ValueError('Nonfinite physical sample word')
    return SampleInput(data,n,points,floors,(data[72:104].hex(),data[104:136].hex()))


def identity(value: SampleInput) -> JsonObject:
    """Content identities identify imported samples; they do not authenticate an execution."""
    sources: list[Json]=list(value.source_hashes)
    floors: list[Json]=list(value.floor_words)
    return {'schema':'NSBU_REDUCTION_INPUT_1','grid':value.n,'samples':2*value.n,
            'sha256':hashlib.sha256(value.data).hexdigest(),'source_json_sha256':sources,
            'relative_floor_words':floors,'scope':'imported physical samples; no checkpoint or PDE qualification'}
