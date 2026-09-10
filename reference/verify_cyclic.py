"""Independent CyclicSine direct-DFT arithmetic study with complete preflight and exact input bits."""
import argparse
from dataclasses import replace
import json
from pathlib import Path
from typing import cast
from mpmath import mp
from reference.cyclic.bits import FixedForcing
from reference.cyclic.study import StudyPlan, trajectory
from reference.dft import Spectrum
from tools.json_types import Json, JsonObject

MAXIMUM_INPUT_BYTES = 16*1024**2


def state_rows(state: Spectrum, precision: int) -> list[Json]:
    """Retain every complex coefficient with explicit signed modes and declared decimal precision."""
    return [{'mode': list(mode),
             'value': [[mp.nstr(v.real,precision),mp.nstr(v.imag,precision)] for v in values]}
            for mode,values in state.items()]


def fixed_input(path: Path, n: int) -> FixedForcing:
    """Bound the byte read before JSON parsing; the caller admits fixture/parser storage first."""
    with path.open('rb') as source:
        raw = source.read(MAXIMUM_INPUT_BYTES+1)
    return FixedForcing(raw,n,MAXIMUM_INPUT_BYTES)


def execute(plan: StudyPlan, path: Path | None, dry_run: bool) -> JsonObject:
    """Admit all storage before loading fixed force data or constructing trajectory/DFT arrays."""
    admission = plan.reservation(fixture=path is not None)
    if path is not None:
        plan = replace(plan,forcing=fixed_input(path,plan.n))
    if dry_run:
        return {'status':'preflight','grid':plan.n,'precision':plan.precision,'method':plan.method,
                'tick_exponent':-16,'endpoint_ticks':'128','macro_step_ticks':'16',
                'preflight':admission,'force_input_byte_cap':MAXIMUM_INPUT_BYTES,
                'fixed_force_sha256':plan.forcing.sha256 if plan.forcing is not None else None,
                'accepted_pde_windows':0}
    state, report = trajectory(plan)
    report['state'] = state_rows(state,plan.precision)
    return report


def main() -> None:
    """Emit one JSON report; syntax exits 2, bounded refusals exit 1 and diagnostics/preflight exit 0."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--n',type=int,choices=(4,8,12),default=4)
    parser.add_argument('--precision',type=int,choices=(80,120),default=80)
    parser.add_argument('--method',choices=('CM','HO'),default='CM')
    parser.add_argument('--cap-bytes',type=int,default=4*1024**3)
    parser.add_argument('--force-bits',type=Path)
    parser.add_argument('--dry-run',action='store_true')
    args = parser.parse_args()
    plan = StudyPlan(cast(int,args.n),cast(int,args.precision),cast(str,args.method),cast(int,args.cap_bytes))
    try:
        result = execute(plan,cast(Path|None,args.force_bits),cast(bool,args.dry_run))
    except (ValueError,ArithmeticError,OSError) as error:
        print(json.dumps({'status':'diagnostic-refused','error':str(error),'accepted_pde_windows':0}))
        raise SystemExit(1) from error
    print(json.dumps(result,indent=2))


if __name__ == '__main__':
    main()
