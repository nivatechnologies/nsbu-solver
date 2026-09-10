"""Bounded exact-v2 global pressure mean with separate quadrature/arithmetic refinements."""
import argparse
from fractions import Fraction
import json
from typing import cast
from reference.pressure_gauge import CAP_BYTES
from reference.pressure_study import preflight, study


def exact_time(value: str) -> Fraction:
    """Treat malformed or zero-denominator rational text as a command syntax error."""
    try:
        return Fraction(value)
    except (ValueError,ZeroDivisionError) as error:
        raise argparse.ArgumentTypeError('Require a rational physical time') from error


def main() -> None:
    """Exit 0 for a completed diagnostic/preflight, 1 for refusal, 2 for invalid command syntax."""
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--time',type=exact_time,default=Fraction(1,256))
    parser.add_argument('--panels',type=int,default=32,help='base even panel count; finest uses four times this')
    parser.add_argument('--cap-bytes',type=int,default=CAP_BYTES)
    parser.add_argument('--dry-run',action='store_true')
    args=parser.parse_args()
    time,panels,cap=cast(Fraction,args.time),cast(int,args.panels),cast(int,args.cap_bytes)
    try:
        result=preflight(time,panels,cap) if cast(bool,args.dry_run) else study(time,panels,cap)
    except (ValueError,ArithmeticError,OSError) as error:
        print(json.dumps({'status':'pressure-mean-refused','error':str(error),'accepted_pde_windows':0}))
        raise SystemExit(1) from error
    print(json.dumps(result,indent=2))


if __name__=='__main__':
    main()
