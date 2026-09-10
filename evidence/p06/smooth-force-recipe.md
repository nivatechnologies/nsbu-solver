# Independent smooth force fixture

Run from the repository root with the pinned Python environment:

```python
from pathlib import Path
from mpmath import mp
from reference.dft import preflight,grid,forward_vector
import json,hashlib
results=[]
for precision in (80,120):
 preflight(8,precision,1024**3)
 with mp.workdps(precision):
  k=2*mp.pi;g=[mp.sin(mp.mpf(w)/64) for w in (13,17,19)];dg=[w*mp.cos(mp.mpf(w)/64) for w in (13,17,19)]
  samples=[]
  for p in grid(8):
   phase=[k*i/8 for i in p]
   samples.append(tuple((dg[c]+k*k*g[c])*mp.sin(phase[(c+1)%3])+k*g[c]*g[(c+1)%3]*mp.cos(phase[(c+1)%3])*mp.sin(phase[(c+2)%3]) for c in range(3)))
  spectrum=forward_vector(samples,8,8);results.append(spectrum)
  if precision==120:
   rows=['\t'.join([*(str(m) for m in mode),*(mp.nstr(part,120) for v in values for part in (v.real,v.imag))]) for mode,values in sorted(spectrum.items()) if mode[2]>=0]
   path=Path('crates/nsbu-benchmarks/tests/fixtures/smooth-force.tsv');path.write_text('\n'.join(rows)+'\n')
with mp.workdps(120):
 change=max(abs(a-b)/(1+abs(b)) for mode in results[0] for a,b in zip(results[0][mode],results[1][mode]))
 report={'precision_digits':[80,120],'scaled_precision_change':str(change),'sha256':hashlib.sha256(path.read_bytes()).hexdigest(),'n':8,'time':'1/64','nu':'1','force':'unprojected exact cyclic-sine force; raw pressure zero'}
 Path('work/p06-smooth-force-fixture.json').write_text(json.dumps(report,indent=2)+'\n');print(report)
```

The three temporal frequencies are 13, 17 and 19. The physical samples use
independent trigonometric evaluation before direct DFT; the production Fourier
coefficient construction is not used.
