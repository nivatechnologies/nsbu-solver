import gzip, hashlib, json
from pathlib import Path
root=Path(__file__).parents[1]
expected=json.loads((root/'input-sha256.json').read_text())['snapshots']
for name,want in expected.items():
    data=gzip.decompress((root/'snapshots'/f'{name}.coeff.bin.gz').read_bytes())
    got=hashlib.sha256(data).hexdigest()
    assert got == want, (name, got, want)
    print(f'{name} bytes={len(data)} sha256={got} ok')
print(f'snapshots={len(expected)} terminal=hash-validation-complete')
