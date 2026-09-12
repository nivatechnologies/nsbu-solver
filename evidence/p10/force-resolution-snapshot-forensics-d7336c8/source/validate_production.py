import re
from pathlib import Path

raw = Path('/mnt/niva-array/nsbu-solver/work/p10-cached-m48-walkthrough/work/ignored-cached-m48-pilot/full.stdout')
outdir = Path('work/p10-force-snapshot-forensics')
fields = r'l2: ([^,]+), h1: ([^,]+), vorticity_l2: ([^,]+), divergence_l2: ([^ }]+)'
for clock, event in ((2048, 3), (4096, 5)):
    line = next(x for x in raw.read_text().splitlines() if x.startswith(f'event={event} elapsed={clock} '))
    producer = re.findall(r'BandComparison \{ full: Norms \{ ' + fields, line)[:2]
    observed = []
    for pair in ('n12-n16', 'n16-n24'):
        text = (outdir / f'compare-m48-{pair}-{clock}.stdout').read_text()
        observed.append(re.search(r'full=Norms \{ ' + fields, text).groups())
    assert observed == producer, (clock, observed, producer)
    print(f'clock={clock} pairs=2 exact_textual_f64_fields=true producer_event={event}')
print('terminal=production-equivalence-complete')
