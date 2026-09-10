"""Compare independent smooth arithmetic runs against raw Rust state and force bits."""
import argparse
import json
from pathlib import Path
from typing import cast
from reference.cyclic.comparison import Inputs, RESERVATION_BYTES, compare


def main() -> None:
    """Emit diagnostic JSON; invalid syntax exits 2, input/resource refusals exit 1."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--n',type=int,choices=(4,8,12),required=True)
    parser.add_argument('--method',choices=('CM','HO'),required=True)
    for name in ('rust','force','independent80','independent120','fixed80','fixed120'):
        parser.add_argument('--'+name,type=Path,required=True)
    parser.add_argument('--cap-bytes',type=int,default=RESERVATION_BYTES)
    args = parser.parse_args()
    inputs = Inputs(cast(Path,args.rust),cast(Path,args.force),cast(Path,args.independent80),
                    cast(Path,args.independent120),cast(Path,args.fixed80),cast(Path,args.fixed120))
    try:
        report = compare(inputs,cast(int,args.n),cast(str,args.method),cast(int,args.cap_bytes))
    except (ValueError,ArithmeticError,OSError) as error:
        print(json.dumps({'status':'diagnostic-refused','error':str(error),'accepted_pde_windows':0}))
        raise SystemExit(1) from error
    print(json.dumps(report,indent=2))


if __name__ == '__main__':
    main()
