#!/usr/bin/env python3
"""Independent construction of the partial review-profile semantics digest."""
import hashlib
FIRST=0x56321001
# descriptor tuple: source, quantity, statistic, region, units
rows=[]
def add(source, quantity, statistic, region, units):
    rows.append((FIRST+len(rows),source,quantity,statistic,region,units))
# Complete-band accepted spectra.
add(1,1,5,7,1); add(1,1,6,7,11); add(1,4,5,7,2); add(1,10,5,7,2)
abs_units={1:1,2:2,3:3,4:2,5:4,6:5}
def sampled(source, region, quantities):
    for quantity in quantities:
        for statistic in (1,2,3):
            add(source,quantity,statistic,region,6 if statistic==3 else abs_units[quantity])
sampled(2,1,(1,2,3,4))
sampled(3,1,(5,6))
for region in (2,3,4,5,6): sampled(4,region,(1,2,3,4))
add(5,7,5,7,7); add(5,7,6,7,8); add(5,4,5,7,8); add(5,10,5,7,8)
add(6,8,4,7,9); add(6,9,4,7,10)
gaps=bytes(range(1,10))
assert len(rows)==88 and rows[-1][0]==FIRST+87
h=hashlib.sha256(); h.update(b'NSBUV2OBSERVABLES0001'); h.update(len(rows).to_bytes(16,'little'))
for key,*meaning in rows: h.update(key.to_bytes(4,'little')); h.update(bytes(meaning))
h.update(len(gaps).to_bytes(16,'little')); h.update(gaps)
print(f'rows={len(rows)} semantics_bytes={21+16+9*len(rows)+16+len(gaps)}')
print(h.hexdigest())
