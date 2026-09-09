# Independent P04 fixture generation

These are archived execution recipes using the already verified Python reference
API, not newly installed repository commands. They ran from the repository root
with Python 3.12 and the existing pinned reference environment. They do not import
Rust code or a production FFT. The checked-in TSV fixtures contain final values;
the adjoining JSON files record 80/120-digit agreement and oracle source hashes.
The dyadic step fixture uses 1/128 so Rust's exact tick profile can represent it;
the earlier 1/100 reference fixture is preserved unchanged.

## Coefficient execution

```python
import json,math,hashlib
from pathlib import Path
from mpmath import mp
from reference.steps import cm_weights
root=Path('work/p04-draft/crates/nsbu-solver/tests/fixtures');root.mkdir(exist_ok=True)
points=[0.0,-1e-16,-0.1,-0.5,-1.0,math.nextafter(-1.0,0),math.nextafter(-1.0,-math.inf),-2.0,math.nextafter(-2.0,0),math.nextafter(-2.0,-math.inf),-10.0,-50.0,math.nextafter(-50.0,0),math.nextafter(-50.0,-math.inf),-100.,-1e4,-1e100,-float.fromhex('0x1.fffffffffffffp+1023')]
mp.dps=120
r=mp.findroot(lambda z:cm_weights(z)[1],(-2,-4));points.extend([float(r),math.nextafter(float(r),0),math.nextafter(float(r),-math.inf)])
def values(z,dps):
 with mp.workdps(dps):
  x=mp.mpf(z);return [mp.exp(x),mp.exp(x/2),*cm_weights(x)]
lines=[];max_change=mp.mpf(0)
for z in points:
 a=values(z,80); b=values(z,120)
 max_change=max(max_change,max(abs(x-y) for x,y in zip(a,b)))
 lines.append('\t'.join([repr(z)]+[mp.nstr(v,125) for v in b]))
p=root/'cm-coefficients.tsv';p.write_text('\n'.join(lines)+'\n')
(root/'cm-coefficients.json').write_text(json.dumps({'oracle':'reference.steps.cm_weights; independent mpmath hypergeometric phi functions','precision_digits':[80,120],'max_absolute_precision_change':mp.nstr(max_change,30),'float_inputs':'Exact conversion of listed binary64 arguments into mpmath','columns':['z','exp(z)','exp(z/2)','q','w1','w2','w3'],'sha256':hashlib.sha256(p.read_bytes()).hexdigest()},indent=2)+'\n')
print(max_change)
```

## Direct-DFT step execution

```python
import json,hashlib
from pathlib import Path
from mpmath import mp
from reference.dft import preflight,retained
from reference.steps import cm_step
from reference.verify_steps import right_hand_side
out=Path('work/p04-draft/crates/nsbu-solver/tests/fixtures')
results=[]
for digits in [80,120]:
 with mp.workdps(digits):
  reservation=preflight(4,digits,1024**3)
  state={k:(mp.mpc(0),)*3 for k in retained(4)}
  dt=mp.mpf(1)/128;rhs=right_hand_side(4)
  full=cm_step(state,mp.mpf(0),dt,rhs,mp.mpf(1))
  middle=cm_step(state,mp.mpf(0),dt/2,rhs,mp.mpf(1))
  fine=cm_step(middle,dt/2,dt/2,rhs,mp.mpf(1))
  results.append((full,fine,reservation))
mp.dps=120
change=max(abs(a-b) for left,right in zip(results[0][:2],results[1][:2]) for k in left for a,b in zip(left[k],right[k]))
assert change<mp.mpf('1e-60')
lines=[]
for path,state in zip(['full','fine'],results[1][:2]):
 for k in sorted(state):
  if k[2]<0:continue
  lines.append('\t'.join([path,*map(str,k),*[mp.nstr(part,125) for z in state[k] for part in [z.real,z.imag]]]))
p=out/'cm-step.tsv';p.write_text('\n'.join(lines)+'\n')
(out/'cm-step.json').write_text(json.dumps({'initial_state':'rest','dt':'1/128','n':4,'viscosity':1,'problem':'reference.verify_steps.force: mean (1,-2,3); cyclic sin amplitudes (1,2,3)*(1+sin(7t))','method':'CM','precision_digits':[80,120],'max_absolute_precision_change':mp.nstr(change,30),'preflight':[r[2] for r in results],'reference_assignments':0,'sha256':hashlib.sha256(p.read_bytes()).hexdigest(),'scope':'independent Python direct-DFT full step and two half steps; not a PDE convergence window'},indent=2)+'\n')
print(change)
```

The output directory in these historical recipes is the isolated P04 draft used
during development. The reviewed outputs were then imported into
`crates/nsbu-solver/tests/fixtures/`; metadata was augmented with exact oracle
source checksums. No original reference input or expected frozen hash was changed.
