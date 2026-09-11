"""Complete reduction comparisons with independent arithmetic and explicit exact-zero handling."""
import hashlib
from pathlib import Path
from mpmath import mp, mpf
from reference.cyclic.bits import binary64
from reference.reduction_audit import input, output
from reference.reduction_audit.oracle import GROUPS, STATISTICS, GroupResult, evaluate
from tools.json_types import Json, JsonObject

CAP=128*1024**2


def preflight(n: int, cap_bytes: int=CAP) -> JsonObject:
    """Bound both exact byte packets and constant-memory MP scratch before reading files."""
    if n not in (4,8,12):
        raise ValueError('Reduction study requires N=4,8,12')
    points=(2*n)**3
    source_bytes=input.HEADER+2*input.ROWS*points*8
    magnitude_bytes=output.HEADER+12*points*8
    reserved=2*(source_bytes+magnitude_bytes)+64*1024**2
    if reserved>cap_bytes:
        raise ValueError('Reduction study exceeds its conservative Python reservation')
    return {'grid':n,'samples':2*n,'physical_points':points,'input_bytes':source_bytes,
            'magnitude_bytes':magnitude_bytes,'reserved_bytes':reserved,'cap_bytes':cap_bytes,
            'precision_profiles':[80,120],'scalar_component_visits':2*2*input.ROWS*points,
            'classification':'conservative Python planning; no hard allocator guarantee',
            'accepted_pde_windows':0}


def bounded(path: Path, size: int) -> bytes:
    """Read at most the exact admitted size plus one sentinel byte; reject changes and trailing data."""
    with path.open('rb') as stream:
        data=stream.read(size+1)
    if len(data)!=size:
        raise ValueError('Packet length does not match the admitted exact sample lattice')
    return data


def compare(source_path: Path, magnitude_path: Path, n: int, cap_bytes: int=CAP) -> JsonObject:
    """Keep imported data binding distinct from independent arithmetic qualification."""
    resources=preflight(n,cap_bytes)
    points=(2*n)**3
    source=input.parse(bounded(source_path,input.HEADER+2*input.ROWS*points*8))
    if source.n!=n:
        raise ValueError('Input grid differs from the preflight profile')
    actual=output.parse(bounded(magnitude_path,output.HEADER+12*points*8),source)
    profiles={p:evaluate(source,actual,p) for p in (80,120)}
    with mp.workdps(120):
        quantities: JsonObject={}
        for index,(name,rows) in enumerate(GROUPS):
            quantities[name]=quantity(actual,index,profiles[80][index],profiles[120][index],len(rows))
    return {'schema':'NSBU_REDUCTION_STUDY_1','status':'complete arithmetic measurements; no PDE qualification',
            'resources':resources,'input':input.identity(source),
            'magnitude_sha256':hashlib.sha256(actual.data).hexdigest(),'quantities':quantities,
            'accepted_pde_windows':0,'precision_policy':'80/120 changes must be below 1e-40 times each nonzero measured binary64 discrepancy; exact zero is explicit',
            'scope':'same imported sampled component words; FFT, integrated trajectories, force and physical sampling were measured separately'}


def errors(actual: tuple[int,...], low: tuple[mpf,...], high: tuple[mpf,...]) -> JsonObject:
    """Per-statistic discrepancy and precision separation; no positive floor replaces an exact zero."""
    result: JsonObject={}
    for name,word,a,b in zip(STATISTICS,actual,low,high,strict=True):
        discrepancy=abs(binary64(word)-b);change=abs(a-b)
        result[name]={'absolute_error':mp.nstr(discrepancy,120),
                      'relative_to_oracle':mp.nstr(discrepancy/abs(b),120) if b else None,
                      'oracle_precision_change':mp.nstr(change,120),
                      'precision_to_error_ratio':mp.nstr(change/discrepancy,120) if discrepancy else None,
                      'precision_separated':change==0 or change<discrepancy*mp.mpf('1e-40'),
                      'exact_zero_discrepancy':discrepancy==0}
    return result


def profile(value: GroupResult, precision: int) -> JsonObject:
    """Preserve every measured precision result before deriving any summary."""
    def numbers(values: tuple[mpf,...]) -> list[Json]:
        return [mp.nstr(v,precision) for v in values]
    return {'precision':precision,'exact_component_statistics':numbers(value.exact_components),
            'rounded_magnitude_statistics':numbers(value.rounded_magnitudes),
            'magnitude_maximum_absolute_errors':numbers(value.magnitude_maximum_absolute_errors),
            'magnitude_rms_errors':numbers(value.magnitude_rms_errors)}


def quantity(actual: output.Magnitudes, index: int, low: GroupResult, high: GroupResult, components: int) -> JsonObject:
    """Both public reducer paths, plus isolated reduction of production-rounded magnitudes."""
    direct,physical=actual.statistics(index,0),actual.statistics(index,1)
    direct_words: list[Json]=list(direct);physical_words: list[Json]=list(physical)
    return {'ordered_components':components,'samples':actual.points,'statistic_order':list(STATISTICS),
            'magnitude_role_order':['error','reference'],'rust_direct_words':direct_words,
            'rust_physical_words':physical_words,'profiles':[profile(low,80),profile(high,120)],
            'direct_vs_exact_components':errors(direct,low.exact_components,high.exact_components),
            'physical_vs_exact_components':errors(physical,low.exact_components,high.exact_components),
            'physical_vs_rounded_magnitudes':errors(physical,low.rounded_magnitudes,high.rounded_magnitudes)}
