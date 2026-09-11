"""Independent high-precision sum-of-squares oracle, without scaled binary64 reduction kernels."""
from dataclasses import dataclass
from mpmath import mp, mpf
from reference.cyclic.bits import binary64
from reference.reduction_audit.input import SampleInput
from reference.reduction_audit.output import Magnitudes

GROUPS=(('velocity',(0,13,26)),('gradient',(1,2,3,14,15,16,27,28,29)),
        ('hessian',tuple(range(4,13))+tuple(range(17,26))+tuple(range(30,39))),
        ('vorticity',(39,40,41)),('pressure',(42,)),('pressure_gradient',(43,44,45)))
STATISTICS=('rms_error','sampled_peak_error','sampled_peak_relative_error','sampled_reference_peak')


@dataclass
class Sums:
    """Constant-memory independent exact-component sum and sampled peaks at one fixed precision."""
    sum_squares: mpf
    peak: mpf
    relative_peak: mpf
    reference_peak: mpf

    @classmethod
    def zero(cls) -> 'Sums':
        return cls(mp.mpf(0),mp.mpf(0),mp.mpf(0),mp.mpf(0))

    def add(self, error_squared: mpf, reference_squared: mpf, floor: mpf) -> tuple[mpf,mpf]:
        """No component averaging or subtraction of norms; one full point contributes its squares."""
        error,reference=mp.sqrt(error_squared),mp.sqrt(reference_squared)
        self.sum_squares+=error_squared
        self.peak=max(self.peak,error)
        self.reference_peak=max(self.reference_peak,reference)
        self.relative_peak=max(self.relative_peak,error/max(reference,floor))
        return error,reference

    def finish(self, points: int) -> tuple[mpf,...]:
        return (mp.sqrt(self.sum_squares/points),self.peak,self.relative_peak,self.reference_peak)


@dataclass(frozen=True)
class GroupResult:
    """Unrounded high-precision values; serialization occurs only after precision comparison."""
    exact_components: tuple[mpf,...]
    rounded_magnitudes: tuple[mpf,...]
    magnitude_maximum_absolute_errors: tuple[mpf,mpf]
    magnitude_rms_errors: tuple[mpf,mpf]


def evaluate(source: SampleInput, actual: Magnitudes, precision: int) -> tuple[GroupResult,...]:
    """Repeat every component operation in isolated 80/120-digit contexts, never mutate inputs."""
    if precision not in (80,120):
        raise ValueError('The reduction study requires an independent 80/120 precision pair')
    with mp.workdps(precision):
        return tuple(group(source,actual,index,rows) for index,(_,rows) in enumerate(GROUPS))


def group(source: SampleInput, actual: Magnitudes, index: int, rows: tuple[int,...]) -> GroupResult:
    """Separate component-to-magnitude arithmetic from reducing already-rounded magnitudes."""
    direct,rounded=Sums.zero(),Sums.zero()
    floor=binary64(source.floor_words[index])
    maxima=[mp.mpf(0),mp.mpf(0)];squares=[mp.mpf(0),mp.mpf(0)]
    for point in range(source.points):
        left=tuple(binary64(source.word(0,row,point)) for row in rows)
        right=tuple(binary64(source.word(1,row,point)) for row in rows)
        errors=sum(((a-b)**2 for a,b in zip(left,right,strict=True)),mp.mpf(0))
        refs=sum((value**2 for value in right),mp.mpf(0))
        exact=direct.add(errors,refs,floor)
        observed=tuple(binary64(actual.word(index,role,point)) for role in (0,1))
        rounded.add(observed[0]**2,observed[1]**2,floor)
        for role in (0,1):
            difference=abs(exact[role]-observed[role])
            maxima[role]=max(maxima[role],difference);squares[role]+=difference**2
    return GroupResult(direct.finish(source.points),rounded.finish(source.points),
                       (maxima[0],maxima[1]),(mp.sqrt(squares[0]/source.points),mp.sqrt(squares[1]/source.points)))
