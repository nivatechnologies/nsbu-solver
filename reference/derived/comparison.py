"""Bounded independent derivative/pressure arithmetic with separate state and prescribed-force effects."""
from collections.abc import Iterator
from dataclasses import dataclass
import hashlib
from mpmath import mp, mpf, mpc
from reference.cyclic.bits import FixedForcing
from reference.cyclic.comparison import Inputs, MAXIMUM_INPUT_BYTES, bounded_bytes
from reference.cyclic.results import reference_state, reality_defect
from reference.cyclic.schema import decode_fixture
from reference.derived import target
from reference.derived.reduction import Reduction
from reference.derived.schema import Derived, bind_endpoint_force, derived
from reference.derived.spectral import fields, inventory, pressure
from reference.dft import Spectrum, retained
from tools.json_types import Json, JsonObject, object_value, string_value

CAP_BYTES=32*1024**3
GROUPS=(('diagnostic_80_120','rust80','rust120'),('rust_vs_same_state_120','export','rust120'),
        ('fixed_80_120','fixed80','fixed120'),('rust_vs_fixed_120','export','fixed120'),
        ('independent_80_120','independent80','independent120'),
        ('fixed_vs_independent_120','fixed120','independent120'),
        ('rust_vs_independent_120','export','independent120'))
QUANTITIES={'velocity':(3,'1e-8'),'gradient':(9,'1e-7'),'hessian':(27,'1e-6'),
            'vorticity':(3,'1e-7'),'pressure':(1,'1e-8'),'pressure_gradient':(3,'1e-7')}


def preflight(n: int, cap_bytes: int) -> JsonObject:
    """Reserve bounded parser, field, generator and reduction storage before reading any artifact."""
    retained(n)
    points=(2*n)**3
    allocations={'raw_input_and_parser_allowance':6*MAXIMUM_INPUT_BYTES*12,
                 'parsed_rust_fields':46*points*4096,'current_generator_fields':6*points*8192,
                 'mixed_derivative_caches':18*points*8192,'complete_group_reductions':84*points*4096,
                 'pressure_coefficients_and_dft_scratch':18*points*8192,
                 'retained_states':6*n**3*3*8192,'metadata_and_interpreter':512*1024**2}
    total=sum(allocations.values())
    if total>cap_bytes:
        raise ValueError('Derived arithmetic exceeds its conservative memory reservation')
    return {'grid':n,'samples':2*n,'allocations':allocations,'reserved_bytes':total,'cap_bytes':cap_bytes,
            'classification':'conservative Python planning; no hard allocator guarantee',
            'profiles':6,'direct_scalar_transforms_per_profile':46,
            'scope':'complete sampled velocity derivatives, vorticity and pressure; no window qualification'}


@dataclass
class Profile:
    """One isolated-precision stream and its independently measured complex/product defects."""
    precision: int
    iterator: Iterator[list[mpc]]
    pressure_nyquist_product: mpf
    maximum_imaginary_component: mpf

    def next(self) -> list[mpc]:
        """Use a separate precision context for every generator advance; preserve computed words."""
        with mp.workdps(self.precision):
            values=next(self.iterator)
            self.maximum_imaginary_component=max(self.maximum_imaginary_component,max(abs(v.imag) for v in values))
        return values


def profile(state: Spectrum, forcing: Spectrum, n: int, precision: int) -> Profile:
    """Assemble pressure before streaming complete physical components at the declared precision."""
    with mp.workdps(precision):
        p,leakage=pressure(state,forcing,n)
    return Profile(precision,fields(state,p,n),leakage,mp.mpf(0))


def profiles(raw: dict[str,bytes], actual: Derived, n: int, method: str, fixed: FixedForcing) -> dict[str,Profile]:
    """Keep same-state, fixed-input evolution and independent target-input profiles separate."""
    result={f'rust{p}':profile(actual.state,actual.force,n,p) for p in (80,120)}
    for name,precision,is_fixed in (('fixed80',80,True),('fixed120',120,True),
                                    ('independent80',80,False),('independent120',120,False)):
        state=reference_state(object_value(decode_fixture(raw[name])),n,method,precision,fixed.sha256 if is_fixed else None)
        with mp.workdps(precision):
            forcing=actual.force if is_fixed else target.force(n,mp.mpf(1)/512)
        result[name]=profile(state,forcing,n,precision)
    return result


def quantity(name: str, orders: tuple[int,int,int]) -> str:
    """Map each ordered scalar entry to its complete physical tensor."""
    if name=='velocity':
        return ('velocity','gradient','hessian')[sum(orders)]
    return 'pressure_gradient' if name=='pressure' and sum(orders) else name


def reduce(actual: Derived, references: dict[str,Profile], n: int) -> JsonObject:
    """Accumulate every signed component into complete per-point comparison norms."""
    reductions={g:{q:Reduction.new((2*n)**3) for q in QUANTITIES} for g,_,_ in GROUPS}
    for row,(name,_component,orders) in enumerate(inventory()):
        samples={name:p.next() for name,p in references.items()}
        key=quantity(name,orders)
        for group,left,right in GROUPS:
            reductions[group][key].add(actual.fields[row] if left=='export' else samples[left],samples[right])
    return {g:{q:r.finish(QUANTITIES[q][0],mp.mpf(QUANTITIES[q][1])) for q,r in values.items()}
            for g,values in reductions.items()}


def compare(inputs: Inputs, n: int, method: str, cap_bytes: int=CAP_BYTES) -> JsonObject:
    """Compare complete actual exported fields and six independently computed precision profiles."""
    resources=preflight(n,cap_bytes)
    if method not in ('CM','HO'):
        raise ValueError('Unsupported derived method')
    paths={'rust':inputs.rust,'force':inputs.force,'independent80':inputs.independent80,
           'independent120':inputs.independent120,'fixed80':inputs.fixed80,'fixed120':inputs.fixed120}
    raw={name:bounded_bytes(path) for name,path in paths.items()}
    with mp.workdps(120):
        actual=derived(object_value(decode_fixture(raw['rust'])),n,method)
        fixed=FixedForcing(raw['force'],n,MAXIMUM_INPUT_BYTES)
        bind_endpoint_force(actual.force,fixed)
        references=profiles(raw,actual,n,method,fixed)
        comparisons=reduce(actual,references,n)
        defects: dict[str,Json]={name:{'pressure_nyquist_product':mp.nstr(p.pressure_nyquist_product,40),
                                'maximum_imaginary_component':mp.nstr(p.maximum_imaginary_component,40)}
                                for name,p in references.items()}
        return {'status':'diagnostic-completed','case':'CyclicSine','grid':n,'method':method,'endpoint':'1/512',
                'resources':resources,'input_sha256':{name:hashlib.sha256(b).hexdigest() for name,b in raw.items()},
                'comparisons':comparisons,'precision_ratios':precision_ratios(comparisons),
                'reference_defects':defects,'rust_reality_defect':mp.nstr(reality_defect(actual.state),40),
                'accepted_pde_windows':0,'artifact_origin':'externally supplied diagnostic numbers',
                'scope':'same-grid sampled derived-field arithmetic and separate force effects; no trajectory qualification'}


def precision_ratios(comparisons: JsonObject) -> JsonObject:
    """Report observed 80/120 changes relative to Rust discrepancies; zero denominators stay undefined."""
    result: JsonObject={}
    for precision,actual in (('diagnostic_80_120','rust_vs_same_state_120'),
                             ('fixed_80_120','rust_vs_fixed_120'),
                             ('independent_80_120','rust_vs_independent_120')):
        values: JsonObject={}
        for quantity in QUANTITIES:
            numerator=object_value(object_value(comparisons[precision])[quantity])
            denominator=object_value(object_value(comparisons[actual])[quantity])
            ratios: JsonObject={}
            for norm in ('rms_error','sampled_peak_error'):
                a,b=mp.mpf(string_value(numerator[norm])),mp.mpf(string_value(denominator[norm]))
                ratios[norm]=mp.nstr(a/b,40) if b else None
            values[quantity]=ratios
        result[precision]=values
    return result
