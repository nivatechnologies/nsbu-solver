"""Compare actual Rust derived fields with six independent high-precision diagnostic profiles."""
import argparse
import json
from pathlib import Path
from typing import cast
from reference.cyclic.comparison import Inputs
from reference.derived.comparison import CAP_BYTES, compare, preflight


def main() -> None:
    """Preflight needs no artifacts; complete comparisons require all six explicitly named inputs."""
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--n',type=int,choices=(4,8,12),required=True)
    parser.add_argument('--method',choices=('CM','HO'),required=True)
    parser.add_argument('--cap-bytes',type=int,default=CAP_BYTES)
    parser.add_argument('--dry-run',action='store_true')
    for name in ('rust','force','independent80','independent120','fixed80','fixed120'):
        parser.add_argument('--'+name,type=Path)
    args=parser.parse_args()
    n,method,cap=cast(int,args.n),cast(str,args.method),cast(int,args.cap_bytes)
    paths=(cast(Path|None,args.rust),cast(Path|None,args.force),cast(Path|None,args.independent80),
           cast(Path|None,args.independent120),cast(Path|None,args.fixed80),cast(Path|None,args.fixed120))
    if not cast(bool,args.dry_run) and any(p is None for p in paths):
        parser.error('complete comparison requires --rust, --force and all four precision trajectories')
    try:
        result=preflight(n,cap) if cast(bool,args.dry_run) else compare(
            Inputs(*(cast(Path,p) for p in paths)),n,method,cap)
    except (ValueError,ArithmeticError,OSError) as error:
        print(json.dumps({'status':'diagnostic-refused','error':str(error),'accepted_pde_windows':0}))
        raise SystemExit(1) from error
    print(json.dumps(result,indent=2))


if __name__=='__main__':
    main()
