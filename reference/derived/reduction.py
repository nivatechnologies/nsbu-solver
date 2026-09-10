"""Complete pointwise field differences, without replacing them by differences of norms."""
from dataclasses import dataclass
from mpmath import mp, mpf, mpc
from tools.json_types import JsonObject

@dataclass
class Reduction:
    """One ordered tensor's full per-point error/reference squares at fixed high precision."""
    error: list[mpf]
    reference: list[mpf]
    components: int = 0
    valid: bool = True

    @classmethod
    def new(cls, points: int) -> 'Reduction':
        if points <= 0:
            raise ValueError('A sampled error requires physical points')
        return cls([mp.mpf(0)]*points,[mp.mpf(0)]*points)

    def add(self, left: list[mpc] | tuple[mpf,...], right: list[mpc]) -> None:
        """Retain complex defects and full ordered entries; inputs are never changed."""
        if not self.valid:
            raise ValueError('A failed reduction cannot publish or retry partial data')
        self.valid=False
        if len(left)!=len(self.error) or len(right)!=len(self.error):
            raise ValueError('Mismatched complete physical fields')
        for index,(a,b) in enumerate(zip(left,right,strict=True)):
            if not mp.isfinite(a) or not mp.isfinite(b):
                raise ValueError('Nonfinite physical comparison')
            self.error[index] += abs(a-b)**2
            self.reference[index] += abs(b)**2
        self.components += 1
        self.valid=True

    def finish(self, expected: int, floor: mpf) -> JsonObject:
        """Full-field RMS and sampled absolute/relative peaks; no division by component count."""
        if not self.valid or self.components!=expected or not mp.isfinite(floor) or floor<=0:
            raise ValueError('Incomplete tensor or invalid fixed relative floor')
        errors=[mp.sqrt(v) for v in self.error]
        references=[mp.sqrt(v) for v in self.reference]
        return {'ordered_components':self.components,'samples':len(errors),'relative_floor':mp.nstr(floor,40),
                'rms_error':mp.nstr(mp.sqrt(sum(self.error,mp.mpf(0))/len(errors)),40),
                'sampled_peak_error':mp.nstr(max(errors),40),'sampled_reference_peak':mp.nstr(max(references),40),
                'sampled_peak_relative_error':mp.nstr(max(a/max(b,floor) for a,b in zip(errors,references)),40)}
