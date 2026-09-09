# P05 fixture execution recipes

These are one-off execution provenance using the already verified independent
Python APIs, not additional maintained Python commands. Original fixtures and
reviewed files were not changed. The exported tables are checked into the public
benchmark crate and bound by the P05 source/fixture inventory.

`fields.tsv` is a tab-separated export of the 120-digit rows in
`fixtures/reference/fields.json`: sample name, x/y/z/t rational strings, velocity,
force, then row-major force gradient. The Rust test represents the non-dyadic late
time by the nearest 2^-100 tick using integer rational arithmetic. All original
rational strings remain in the fixture. The remaining-time quantization is below
2^-101 and is separate from the binary64 conversion error.

The force and root exports below were executed from the root with Python 3.12,
`requirements-dev.txt` installed, and `PYTHONPATH` pointing to the checkout. Their
80/120-digit changes and resulting SHA-256 values are recorded in
[force-fixture.json](force-fixture.json) and [root-fixture.json](root-fixture.json).
The paths under `work/p05-draft` identify the isolated implementation draft used
for generation; those files were then copied unchanged into the public crate.

## Force direct DFT

```python
from pathlib import Path
import hashlib,json
from mpmath import mp
from reference.evaluator import evaluate
from reference.dft import grid,forward_vector,preflight
from reference.scalar import rational
results=[]
for precision in (80,120):
 preflight(4,precision,1024**3)
 with mp.workdps(precision):
  samples=[]
  for p in grid(4):
   coordinates=tuple(mp.mpf(i if i<2 else i-4)/4 for i in p)
   samples.append(evaluate(*coordinates,rational('1/1024')).force)
  coefficients=forward_vector(samples,4,4)
  results.append(coefficients)
  if precision==120:
   lines=[]
   for mode,vector in sorted(coefficients.items()):
    if mode[2]<0: continue
    lines.append('\t'.join([*(str(k) for k in mode),*(mp.nstr(part,120) for v in vector for part in (v.real,v.imag))]))
   path=Path('work/p05-draft/crates/nsbu-benchmarks/tests/fixtures/force-n4.tsv')
   path.write_text('\n'.join(lines)+'\n')
with mp.workdps(120):
 change=max(abs(a-b)/(1+abs(b)) for k,v in results[0].items() for a,b in zip(v,results[1][k]))
 report={'precision_digits':[80,120],'maximum_scaled_precision_change':str(change),'n':4,'time':'1/1024','classification':'sampled force coefficient oracle; sampling convergence not established','sha256':hashlib.sha256(path.read_bytes()).hexdigest()}
 Path('work/p05-force-fixture.json').write_text(json.dumps(report,indent=2)+'\n')
 print(json.dumps(report))
```

## Implicit jets

```python
from pathlib import Path
import json,hashlib
from mpmath import mp
from reference.jets import Jet
from reference.evaluator import implicit_root
from reference.scalar import rational
samples=[('axis','0','1/256'),('interior','1/10','1/1024'),('late','1/10000','999999/128000000')]
results=[]
for precision in (80,120):
 with mp.workdps(precision):
  rows=[]
  for name,z,t in samples:
   q=implicit_root(Jet.variable(rational(z),2),Jet.variable(rational(t),3)).jet
   rows.append(q.coefficients)
  results.append(rows)
  if precision==120:
   out=['\t'.join([name,z,t]+[mp.nstr(v,120) for v in row]) for (name,z,t),row in zip(samples,rows)]
   path=Path('work/p05-draft/crates/nsbu-benchmarks/tests/fixtures/root-jets.tsv')
   path.write_text('\n'.join(out)+'\n')
with mp.workdps(120):
 change=max(abs(a-b)/(1+abs(b)) for row1,row2 in zip(*results) for a,b in zip(row1,row2))
 report={'precision_digits':[80,120],'maximum_scaled_precision_change':str(change),'sha256':hashlib.sha256(path.read_bytes()).hexdigest()}
 Path('work/p05-root-fixture.json').write_text(json.dumps(report,indent=2)+'\n')
 print(json.dumps(report))
```
