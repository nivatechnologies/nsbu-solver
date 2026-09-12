# Generation preflight and frozen runs used a 4 GiB per-process RLIMIT_AS.
ulimit -v 4194304; timeout --signal=TERM --kill-after=5s 300s /usr/bin/time -v p10-reference-spectrum-hi 384 --preflight
ulimit -v 4194304; timeout --signal=TERM --kill-after=5s 120s /usr/bin/time -v p10-reference-spectrum-hi 192 --run
ulimit -v 4194304; timeout --signal=TERM --kill-after=5s 480s /usr/bin/time -v p10-reference-spectrum-hi 384 --run
# Separate comparison used a 2 GiB per-process RLIMIT_AS and 60 s timeout.
ulimit -v 2097152; timeout --signal=TERM --kill-after=5s 60s /usr/bin/time -v p10-force-snapshot-tool-hi compare M192 M384 4096 192 192 192 384 1 1 EXPECTED_SHA EXPECTED_SHA
