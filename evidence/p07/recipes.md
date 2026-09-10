# P07 independent fixture recipes

Run from the repository root with the pinned Python environment. Save a code
block under `work/`, then invoke it as `PYTHONPATH=. .venv/bin/python work/NAME.py`.
No Rust implementation participates in these Python calculations.

## Hypergeometric coefficient construction

```python
"""Independent hypergeometric HO coefficient fixtures, with cancellation guard digits."""
import hashlib,json,math
from pathlib import Path
from mpmath import mp
arguments=[0.0,-1e-300,-1e-16,-1e-8,-0.1,math.nextafter(-1.0,0.0),-1.0,math.nextafter(-1.0,-math.inf),math.nextafter(-2.0,0.0),-2.0,math.nextafter(-2.0,-math.inf),-4.0,-20.0,math.nextafter(-50.0,0.0),-50.0,math.nextafter(-50.0,-math.inf),-100.0,-1e6,-1e100,-1e308,-float.fromhex('0x1.fffffffffffffp+1023')]
with mp.workdps(450):
 cancellation_root=mp.findroot(lambda z: mp.hyp1f1(1,2,z)-3*mp.hyp1f1(1,3,z)/2+4*mp.hyp1f1(1,4,z)/6,(-2,-4))
 root_float=float(cancellation_root)
 arguments.extend([math.nextafter(root_float,0.0),root_float,math.nextafter(root_float,-math.inf)])
rows=[]
for argument in arguments:
 with mp.workdps(450):
  z=mp.mpf(argument)
  def phi(value,order):
   return mp.hyp1f1(1,order+1,value)/mp.factorial(order)
  p1,p2,p3=[phi(z,j) for j in (1,2,3)]
  h1,h2,h3=[phi(z/2,j) for j in (1,2,3)]
  a52=h2/2-p3+p2/4-h3/2;a54=h2/4-a52
  table=[[0]*5,[h1/2,0,0,0,0],[h1/2-h2,h2,0,0,0],[p1-2*p2,p2,p2,0,0],[h1/2-2*a52-a54,a52,a52,a54,0]]
  weights=[p1-3*p2+4*p3,0,0,-p2+4*p3,4*p2-8*p3]
  values=[mp.exp(z),mp.exp(z/2),*(v for row in table for v in row),*weights]
  rows.append('\t'.join([repr(argument),*(mp.nstr(v,120) for v in values)]))
path=Path('crates/nsbu-solver/tests/fixtures/ho-coefficients.tsv')
path.write_text('\n'.join(rows)+'\n')
print(json.dumps({'arguments':len(rows),'cancellation_root':mp.nstr(cancellation_root,120),'working_digits':450,'export_digits':120,'sha256':hashlib.sha256(path.read_bytes()).hexdigest()},indent=2))
```

## Independent full/two-half direct-DFT steps

```python
import json,hashlib
from pathlib import Path
from mpmath import mp
from reference.dft import preflight,retained
from reference.steps import ho_step
from reference.verify_steps import right_hand_side
out=Path('crates/nsbu-solver/tests/fixtures')
results=[]
for digits in [80,120]:
 with mp.workdps(digits):
  reservation=preflight(4,digits,1024**3)
  state={k:(mp.mpc(0),)*3 for k in retained(4)}
  dt=mp.mpf(1)/128;rhs=right_hand_side(4)
  full=ho_step(state,mp.mpf(0),dt,rhs,mp.mpf(1))
  middle=ho_step(state,mp.mpf(0),dt/2,rhs,mp.mpf(1))
  fine=ho_step(middle,dt/2,dt/2,rhs,mp.mpf(1))
  results.append((full,fine,reservation))
mp.dps=120
change=max(abs(a-b) for left,right in zip(results[0][:2],results[1][:2]) for k in left for a,b in zip(left[k],right[k]))
assert change<mp.mpf('1e-60')
lines=[]
for path,state in zip(['full','fine'],results[1][:2]):
 for k in sorted(state):
  if k[2]<0:continue
  lines.append('\t'.join([path,*map(str,k),*[mp.nstr(part,125) for z in state[k] for part in [z.real,z.imag]]]))
p=out/'ho-step.tsv';p.write_text('\n'.join(lines)+'\n')
(out/'ho-step.json').write_text(json.dumps({'initial_state':'rest','dt':'1/128','n':4,'viscosity':1,'problem':'reference.verify_steps.force: mean (1,-2,3); cyclic sin amplitudes (1,2,3)*(1+sin(7t))','method':'HO','precision_digits':[80,120],'max_absolute_precision_change':mp.nstr(change,30),'preflight':[r[2] for r in results],'reference_assignments':0,'sha256':hashlib.sha256(p.read_bytes()).hexdigest(),'scope':'independent Python direct-DFT full step and two half steps; not a PDE convergence window'},indent=2)+'\n')
print(change)
```

## Independent stiff scalar refinements

```python
"""Independent 80/120-digit resolved-regime scalar HO trajectory fixtures."""
import json,hashlib,time
from pathlib import Path
from mpmath import mp
from reference.steps import ho_tableau

def evolve(decay,frequency,count):
 dt=mp.mpf(1)/(16*count);z=-decay*dt
 rows,weights=ho_tableau(z);nodes=tuple(map(mp.mpf,('0','.5','.5','1','.5')))
 exponential=[mp.exp(c*z) for c in nodes];full=mp.exp(z);value=mp.mpf(0)
 for index in range(count):
  sources=[]
  for stage,node in enumerate(nodes):
   trial=exponential[stage]*value+dt*sum((rows[stage][j]*sources[j] for j in range(stage)),mp.mpf(0))
   phase=frequency*(index+node)*dt
   sources.append(frequency*mp.cos(phase)+(decay+2)*mp.sin(phase)-2*trial)
  value=full*value+dt*sum((a*b for a,b in zip(weights,sources)),mp.mpf(0))
 return value

start=time.monotonic();reports=[]
for precision in (80,120):
 with mp.workdps(precision):
  result={}
  for decay,frequency,base in ((0,13,8),(20,13,8),(1000,13,32),(100000,1300,2048),(100000,13,2048)):
   for multiplier in (1,2,4,8):
    count=base*multiplier
    result[(decay,frequency,count)]=mp.nstr(evolve(decay,frequency,count),precision)
  reports.append(result)
with mp.workdps(120):
 difference=max(abs(mp.mpf(reports[0][key])-mp.mpf(value)) for key,value in reports[1].items())
 assert difference<mp.mpf('1e-60')
 path=Path('crates/nsbu-solver/tests/fixtures/ho-stiff-trajectories.tsv')
 path.write_text('\n'.join('\t'.join([*map(str,key),value]) for key,value in reports[1].items())+'\n')
 metadata={'precision_digits':[80,120],'trajectories_per_precision':len(reports[1]),'maximum_absolute_precision_change':mp.nstr(difference,80),'initial_state':'rest','reference_assignments':0,'endpoint':'1/16','equation':'u\'= -lambda*u + frequency*cos(frequency*t) + (lambda+2)*sin(frequency*t) - 2*u','exact_solution':'sin(frequency*t)','fixture_sha256':hashlib.sha256(path.read_bytes()).hexdigest(),'execution_seconds':time.monotonic()-start}
 Path('work/p07-refined-scalar-fixtures.json').write_text(json.dumps(metadata,indent=2)+'\n')
 print(json.dumps(metadata,indent=2),flush=True)
```

## Independent coarse scalar order-reduction study

```python
"""Independent high-precision scalar HO evolution; diagnostic order-reduction study."""
import json
from pathlib import Path
from mpmath import mp
from reference.steps import ho_tableau
reports=[]
with mp.workdps(120):
 for decay in (0,20,1000,100000):
  earlier=None
  for count in (8,16,32,64,128,256,512):
   dt=mp.mpf(1)/(16*count); z=-decay*dt
   rows,weights=ho_tableau(z); nodes=tuple(map(mp.mpf,('0','.5','.5','1','.5')))
   exponent=[mp.exp(c*z) for c in nodes]; full=mp.exp(z)
   value=mp.mpf(0)
   for index in range(count):
    source=[]
    for stage,c in enumerate(nodes):
     trial=exponent[stage]*value+dt*sum((rows[stage][j]*source[j] for j in range(stage)),mp.mpf(0))
     time=(index+c)*dt
     source.append(13*mp.cos(13*time)+(decay+2)*mp.sin(13*time)-2*trial)
    value=full*value+dt*sum((w*f for w,f in zip(weights,source)),mp.mpf(0))
   error=abs(value-mp.sin(mp.mpf(13)/16))
   reports.append({'decay':decay,'steps':count,'value':mp.nstr(value,120),'error':mp.nstr(error,120),'order':None if earlier is None else mp.nstr(mp.log(earlier/error,2),40)})
   earlier=error
Path('work/p07-stiff-oracle.json').write_text(json.dumps({'precision':120,'reference_assignments':0,'reports':reports},indent=2)+'\n')
print('completed',len(reports),'scalar trajectory studies')
```

## Concentrating trajectories

Use the preserved `reference.pilot.pilot(4, 16384, precision, "HO", 1024**3)`
for each precision 80 and 120, starting independently from rest. Require
`diagnostic-completed`, 64 steps, 320 RHS evaluations, elapsed/remaining ticks
both 4096 on the 2^-20 clock, and zero reference assignments. Compare every
coefficient before exporting the 18 nonnegative-third-coordinate strict modes
from the 120-digit result. The saved input reports and checksum comparison
retain the actual completed runs. This validates a discrete coarse trajectory,
not a PDE convergence window.
