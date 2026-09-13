#!/usr/bin/env python3
import hashlib, json, math, pathlib, sys

candidate, bridge_path, binary_path = map(pathlib.Path, sys.argv[1:])
if candidate.stat().st_size > 65536:
    raise SystemExit('candidate exceeds 64 KiB')
bridge_raw = bridge_path.read_bytes()
bridge = json.loads(bridge_raw)
report = json.loads(candidate.read_text())
base = bridge_path.parent
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
expected_bridge_sha = {
    512: '1386090c1d2bfcf8a2cc846eb523029bd56cea3a84cdec83027d8dba2b700435',
    4096: 'e2ed263e3d27a0cc8585320b02af76ad5c8e55e8c171212eb8ac54495199b1d4',
}.get(bridge['elapsed'])
if hashlib.sha256(bridge_raw).hexdigest() != expected_bridge_sha:
    raise SystemExit('bridge manifest hash mismatch')
if sha(binary_path) != bridge['binary_sha256']:
    raise SystemExit('binary hash mismatch')
snapshot_path = base / bridge['snapshot_manifest']
if sha(snapshot_path) != bridge['snapshot_manifest_sha256']:
    raise SystemExit('snapshot manifest hash mismatch')
snapshot = json.loads(snapshot_path.read_text())
if sha(snapshot_path.parent / snapshot['snapshot']) != snapshot['file_sha256']:
    raise SystemExit('snapshot file hash mismatch')
for source in bridge['reference_sources']:
    if sha(base / source['path']) != source['sha256']:
        raise SystemExit(f"source hash mismatch: {source['role']}")
expected_binding = {key: value for key, value in bridge.items() if key not in {'schema', 'snapshot_manifest', 'snapshot_manifest_sha256'}}
expected_binding['reference_sources'] = [
    {'role': item['role'], 'sha256': item['sha256']} for item in bridge['reference_sources']
]
if report.get('schema') != 'p10-external-reference-bridge-output-v1':
    raise SystemExit('output schema mismatch')
if report.get('status') != 'sampled-reference-diagnostic-complete':
    raise SystemExit('output is not complete')
if report.get('bridge') != expected_binding:
    raise SystemExit('reported bridge binding mismatch')
if report.get('snapshot_identity') != snapshot['identity']:
    raise SystemExit('snapshot identity mismatch')
for report_key, snapshot_key in [
    ('snapshot_source_commit', 'source_commit'),
    ('snapshot_plan_sha256', 'plan_sha256'),
    ('snapshot_coefficient_sha256', 'coefficient_sha256'),
    ('snapshot_file_sha256', 'file_sha256'),
]:
    if report.get(report_key) != snapshot[snapshot_key]:
        raise SystemExit(f'{report_key} mismatch')
if report.get('preflight', {}).get('storage', {}).get('total') != bridge['execution_cap_bytes']:
    raise SystemExit('preflight cap mismatch')
if report.get('reference_assignments') != 0 or report.get('resume_or_import_interfaces') != 0:
    raise SystemExit('read-only invariants mismatch')
for name in ('difference', 'actual', 'sampled_reference'):
    norms = report.get(name, {})
    if set(norms) != {'l2', 'h1', 'vorticity_l2', 'divergence_l2'}:
        raise SystemExit(f'{name} norm fields mismatch')
    if any(not isinstance(value, (int, float)) or not math.isfinite(value) or value < 0 for value in norms.values()):
        raise SystemExit(f'{name} has invalid norm')
mean = report.get('signed_reference_minus_actual_mean')
if not isinstance(mean, list) or len(mean) != 3 or any(not math.isfinite(value) for value in mean):
    raise SystemExit('invalid signed mean')
print('validated complete bound diagnostic')
