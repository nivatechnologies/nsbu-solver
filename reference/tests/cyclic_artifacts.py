"""Synthetic parser-contract artifacts, not independently verified physical trajectories."""
import hashlib
import json
from pathlib import Path
from reference.cyclic.comparison import Inputs
from reference.cyclic.bits import mode_at
from reference.dft import retained
from reference.tests.test_cyclic_reference import encode, fixture
from tools.json_types import JsonObject


def reference_report(precision: int, fixed: str | None, method: str = 'CM') -> JsonObject:
    """An explicit zero-state parser fixture with the declared report schema."""
    return {'case':'CyclicSine' if fixed is None else 'fixed-stage-arithmetic-fixture',
            'nominal_comparison_case':'CyclicSine','grid':4,'method':method,
            'precision':precision,'endpoint':'1/512','macro_step':'1/4096','macro_steps':8,
            'fine_steps_committed':16,'rhs_calls':96 if method == 'CM' else 120,
            'reference_assignments':0,'fixed_force_sha256':fixed,'status':'diagnostic-completed',
            'accepted_pde_windows':0,
            'state':[{'mode':list(mode),'value':[['0','0'],['0','0'],['0','0']]}
                     for mode in retained(4)]}


def rust_report(method: str = 'CM') -> JsonObject:
    """Canonical half-storage zeros; the consumer must retain every coefficient."""
    return {'grid':4,'method':'CoxMatthews' if method == 'CM' else 'HochbruckOstermann',
            'tick_exponent':-16,'elapsed_ticks':128,
            'macro_step_ticks':16,'macro_steps':8,'fine_steps_committed':16,
            'rhs_calls':96 if method == 'CM' else 120,'reference_assignments':0,'accepted_pde_windows':0,
            'state':[{'mode':list(mode_at(i,4)),'bits':[[0,0],[0,0],[0,0]]} for i in range(48)]}


def write_inputs(directory: Path, method: str = 'CM') -> Inputs:
    """Write six different named artifacts with byte-bound fixed-input identity."""
    force = encode(fixture())
    sha = hashlib.sha256(force).hexdigest()
    (directory/'force.json').write_bytes(force)
    (directory/'rust.json').write_text(json.dumps(rust_report(method)))
    for name, precision, fixed in (('independent80',80,None),('independent120',120,None),
                                  ('fixed80',80,sha),('fixed120',120,sha)):
        (directory/(name+'.json')).write_text(json.dumps(reference_report(precision,fixed,method)))
    return Inputs(*(directory/(name+'.json') for name in
                    ('rust','force','independent80','independent120','fixed80','fixed120')))
