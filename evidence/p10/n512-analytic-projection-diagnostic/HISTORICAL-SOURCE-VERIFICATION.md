# Source-specific checksum verification

The neighboring N384 diagnostic's `SHA256SUMS` is an immutable evidence manifest for the files as
recorded at evidence commit `ae9691834d842dc89eb3a4ccf6a27317f7bc131a`. Its N384 analytical
projection implementation was frozen at source commit
`dc3c49ebe4b147ff89405b91ccf7180be9a2c75c`. Later N512 preparation parameterized shared harness
entry points, so applying that historical checksum manifest directly to the current checkout is not
a valid source verification and must not be used to replace its expected hashes.

Verify the complete historical N384 evidence tree in a temporary directory:

```text
REPO=/mnt/niva-array/nsbu-solver
TEMP=$(mktemp -d)
git -C "$REPO" archive ae9691834d842dc89eb3a4ccf6a27317f7bc131a \
  evidence/p10/n384-regional-snapshot-diagnostic | tar -x -C "$TEMP"
(cd "$TEMP/evidence/p10/n384-regional-snapshot-diagnostic" && sha256sum -c SHA256SUMS)
echo "verified archive retained at $TEMP"
```

The current N512 source is separately frozen at
`a1d04a7ffc866fa3c826c01eae7269c575ba927a`. Its complete harness source and Cargo metadata hashes
are in `SOURCE_SHA256SUMS`. Verify them from the repository root with:

```text
git diff --exit-code a1d04a7ffc866fa3c826c01eae7269c575ba927a -- \
  evidence/p10/n384-regional-snapshot-diagnostic/harness
git diff --exit-code -- \
  evidence/p10/n384-regional-snapshot-diagnostic/harness
sha256sum -c evidence/p10/n512-analytic-projection-diagnostic/SOURCE_SHA256SUMS
```

The first check binds the current harness to the frozen N512 source commit, the second refuses local
source edits, and the checksum file verifies the explicit current-source identities. These checks do
not alter either historical result or the preserved frozen release binary.
