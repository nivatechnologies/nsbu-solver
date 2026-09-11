# Fixed exact-v2 full diagnostic JSON export evidence

Final source commits `3223628` and admission-test follow-up `ace250b` are based
on frozen alpha.1 commit `88015d7`. Earlier amended snapshots `f4648e4` and
`7239d64` are superseded, pre-correction history; their reports are not claimed
as final evidence. The focused fixture evolves the existing seven-event
N=4/8/12 startup profile once and uses Serde JSON 1.0.145 with
`float_roundtrip` only as an independent dev decoder. The production exporter
is a manual constant-storage writer with no Serde runtime dependency.

The first exact-bit decoder run failed because default Serde JSON decoding
rounded one emitted `.17e` value by one ULP (decoded bits
4487126258331716665, source bits 4487126258331716666). Enabling the documented
dev-only `float_roundtrip` feature corrected the independent decoder; the
emitted decimal representation did not change. Stale coverage runs stopped or
combined after later review fixes are historical and are not reported as final
evidence.

The final oracle traverses all seven events and compares every full/common/newly
resolved Norms member and mean word, every physical and pressure LocalError and
extremum, all global and five-class regional reports, every node provenance
field, and every probe/residual origin and geometry. Binary64 values compare by
`to_bits`, including signed zero; u128 clocks/counters compare as decimal
strings. Missing and not-scheduled variants, foreign fixed schedules, cap
refusal, no-output validation refusal, short-write accounting, preserved
ENOSPC, and a one-call Interrupted failure are explicit controls. A zero-run
alternate M=16, N=12 plan checks pressure force layout 2N independently from
residual force layout 2M.

This remains an UnqualifiedDiagnostic fixed-current-grid binary64 artifact. It
does not establish complete artifact provenance, a pressure gauge/reference,
continuum bounds, convergence, nominal-region volume coverage, or a qualified
PDE window. The five sampled spatial classes are exclusive under the current
classifier; the exporter does not add coverage or volume enclosures.
