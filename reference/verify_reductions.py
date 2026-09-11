"""Measure both production physical-reduction paths against independent 80/120-digit arithmetic."""
import argparse
import json
from pathlib import Path
from typing import cast
from reference.reduction_audit.study import CAP, compare, preflight


def main() -> None:
    """Preflight without files or emit the complete comparison; no accepted-window flag is produced."""
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--n',type=int,choices=(4,8,12),required=True)
    parser.add_argument('--input',type=Path)
    parser.add_argument('--magnitudes',type=Path)
    parser.add_argument('--cap-bytes',type=int,default=CAP)
    parser.add_argument('--dry-run',action='store_true')
    args=parser.parse_args()
    n,cap,dry=cast(int,args.n),cast(int,args.cap_bytes),cast(bool,args.dry_run)
    source,magnitudes=cast(Path|None,args.input),cast(Path|None,args.magnitudes)
    if not dry and (source is None or magnitudes is None):
        parser.error('execution requires --input sample.bin --magnitudes rust.bin')
    try:
        result=preflight(n,cap) if dry else compare(cast(Path,source),cast(Path,magnitudes),n,cap)
    except (ValueError,ArithmeticError,OSError) as error:
        print(json.dumps({'status':'reduction-study-refused','error':str(error),'accepted_pde_windows':0}))
        raise SystemExit(1) from error
    print(json.dumps(result,indent=2))


if __name__=='__main__':
    main()
