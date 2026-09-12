# Cached M96 user-interrupted midpoint controls

This archive records three frozen-source `def4730b08025fdd06e7a8a0d78116aea24b6e2c` runs interrupted for a user-requested hardware pause. It contains their complete raw stdout, empty or completed timing wrapper output, command/provenance records, interruption records, and the accepted clock-2048 coefficient snapshots. The controls did not reach endpoint 4096 and have no numerical-failure or endpoint claim.

N24 has no wrapper exit file because its session leader and separately grouped timeout child required two TERM actions; both are recorded. N32 and N48 report exit status 143 after TERM. No continuation is possible because these cached owners were terminated in-process and cached archive/resume is unsupported.

The N48 erratum preserves the superseded wrong snapshot-buffer dry run and the corrected exact buffer context. `MANIFEST.sha256` hashes all archival files except itself and its post-check transcript. `MANIFEST.verify.txt` records the actual verification result.
