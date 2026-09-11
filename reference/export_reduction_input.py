"""Prepare exact CM/HO sampled component words for a bounded production-reducer audit."""
import argparse
import json
from pathlib import Path
from typing import cast
from reference.reduction_audit.input import CAP_BYTES, identity, parse, preflight, prepare


def main() -> None:
    """Dry-run needs no artifacts; actual execution writes a new complete diagnostic packet."""
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--n',type=int,choices=(4,8,12),required=True)
    parser.add_argument('--left',type=Path)
    parser.add_argument('--right',type=Path)
    parser.add_argument('--output',type=Path)
    parser.add_argument('--cap-bytes',type=int,default=CAP_BYTES)
    parser.add_argument('--dry-run',action='store_true')
    args=parser.parse_args()
    n,cap,dry=cast(int,args.n),cast(int,args.cap_bytes),cast(bool,args.dry_run)
    paths=(cast(Path|None,args.left),cast(Path|None,args.right),cast(Path|None,args.output))
    if not dry and any(path is None for path in paths):
        parser.error('execution requires --left CM.json --right HO.json --output packet.bin')
    try:
        result=preflight(n,cap)
        if not dry:
            left,right,output=(cast(Path,path) for path in paths)
            data=prepare(left,right,n,cap)
            result['input']=identity(parse(data))
            with output.open('xb') as stream:
                stream.write(data)
            result['status']='reduction-input-prepared'
    except (ValueError,ArithmeticError,OSError) as error:
        print(json.dumps({'status':'reduction-input-refused','error':str(error),'accepted_pde_windows':0}))
        raise SystemExit(1) from error
    print(json.dumps(result,indent=2))


if __name__=='__main__':
    main()
