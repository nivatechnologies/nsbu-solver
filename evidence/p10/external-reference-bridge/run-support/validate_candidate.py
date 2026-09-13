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
    ('a39b4811138a0f5dd39540e4bc70b83f5f48b8bce5de1c5b822b420ace201047', 512):
        '1386090c1d2bfcf8a2cc846eb523029bd56cea3a84cdec83027d8dba2b700435',
    ('a39b4811138a0f5dd39540e4bc70b83f5f48b8bce5de1c5b822b420ace201047', 4096):
        'e2ed263e3d27a0cc8585320b02af76ad5c8e55e8c171212eb8ac54495199b1d4',
    ('7dc458760b2453b2c61a69db4c211389dfb6d8be80f1a993a64e656247854893', 512):
        '8f10e1dd124a2d06a823bf71fc8a82aa02ac033686995056abb098fc069ef2bf',
    ('87656e3654a5a9c1a3861a785f9f856e5d3ba3dadeda48a0215ea73964eb6ebc', 1024):
        '9fde34d036493db73864bedb7c214e18e71047b017698a8593e93c2274e6d3e5',
    ('4d18638cc040d86e48eb330daff8ee46db985fef23a9247b7ad2df990f9f3a1d', 2048):
        'a975f09bacd6ecdd1c47d6918db822a1560ff8b670559e0300b270b16d85433a',
    ('782444a29870e1a678fa8393149bd9ba0c28e4cc08dff3d635a7a0bb98c3cb52', 3072):
        'c6f1fdb94617dadc11516cf3c1bdb30c49806b6bd3ce9c4e2b5adc27db8428b0',
}.get((bridge['binary_sha256'], bridge['elapsed']))
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
