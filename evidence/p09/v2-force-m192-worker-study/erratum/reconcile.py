#!/usr/bin/env python3
"""Reconcile the archived worker-study hash with the source four-grid raw artifact."""
import gzip, hashlib, json, pathlib, re, sys
ROOT=pathlib.Path(__file__).resolve().parents[4]
SOURCE=ROOT/'evidence/p09/v2-force-grid-refinement/raw/force-grid.stdout.gz'
WORKER=ROOT/'evidence/p09/v2-force-m192-worker-study/raw/worker-study.stdout.gz'
HARNESS=ROOT/'evidence/p09/v2-force-m192-worker-study/executed-harness/src/main.rs'
def text(path):
    with gzip.open(path,'rt') as f: return f.read()
def hash_file(path): return hashlib.sha256(path.read_bytes()).hexdigest()
def hash_at(raw, clock):
    line=next(x for x in raw.splitlines() if x.startswith(f'clock={clock} '))
    values=re.findall(r'raw_hashes=\[\[[^]]+\], \[[^]]+\], \[[^]]+\], \[([^]]+)\]',line)
    if len(values)!=1: raise ValueError('missing M192 raw hash')
    return ''.join(f'{int(x.strip(),16):02x}' for x in values[0].split(','))
def observed(raw, label):
    line=next(x for x in raw.splitlines() if x.startswith(label))
    value=re.search(r'hash=\[([^]]+)\]',line)
    if value is None: raise ValueError('missing observed hash')
    return ''.join(f'{int(x.strip(),16):02x}' for x in value.group(1).split(','))
source=text(SOURCE); worker=text(WORKER); expected=hash_at(source,4096)
values={name:observed(worker,name) for name in ['serial_endpoint','workers12_endpoint','workers32_4096']}
constant=re.search(r'const ARCHIVED_4096:\[u8;32\]=\[([^;]+)\];',HARNESS.read_text()).group(1)
control=''.join(f'{int(x.strip(),16):02x}' for x in constant.split(','))
result={'source_four_grid_raw':str(SOURCE.relative_to(ROOT)),'worker_raw':str(WORKER.relative_to(ROOT)),'executed_harness':str(HARNESS.relative_to(ROOT)),'sha256':{str(SOURCE.relative_to(ROOT)):hash_file(SOURCE),str(WORKER.relative_to(ROOT)):hash_file(WORKER),str(HARNESS.relative_to(ROOT)):hash_file(HARNESS)},'actual_archived_m192_4096_hash':expected,'observed_hashes':values,'executed_control_constant':control,'control_matches_actual':control==expected,'all_observed_match_actual':all(x==expected for x in values.values())}
if not result['all_observed_match_actual'] or result['control_matches_actual']: sys.exit('reconciliation invariant failed')
print(json.dumps(result,sort_keys=True,indent=2))
