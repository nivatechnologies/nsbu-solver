# Revised quality policy: local execution evidence

The user lowered executable line and branch coverage to 80% and made mutation,
dead-code and duplication findings informational. Complexity, file-size, CRAP
and type requirements remain enforced. Numerical acceptance requirements are
unchanged. Normal Rust CI no longer runs the complete mutation campaign; that
report is available through the explicit workflow-dispatch option.

The complete maintained Python suite passed **159 tests**. Measured coverage is
3309/3314 executable lines (99.85%) and 607/612 branches (99.18%), with no excluded
lines. Strict typing, complexity checks and the repository/design checks passed.
The separate bootstrap invocation passed 41 tests. The CRAP checker reports
maximum Python CRAP 15 and Rust CRAP 21 using measured function coverage.

The Rust measurements come from the independently recorded
[artifact/replay snapshot](../p09/artifacts/README.md). Its full mutation campaign
was already executed under the previous policy; those results remain historical
evidence, not new mandatory thresholds. Generated workspace API documentation
also builds with warnings denied.

The raw reports and source/configuration hashes are listed in
[summary.json](summary.json). The logged full Python run supersedes an earlier
detached invocation whose exit status was unavailable. Informational Python
reports retain eight Vulture findings (dynamically discovered test classes and
schema fields) and no Pylint duplication findings. Tool findings do not prove
the absence of dead or redundant code.

Commands were run in the pinned Python 3.12 development environment:

```sh
coverage erase
coverage run -m pytest -q
coverage combine
coverage json
basedpyright
radon cc -j -s quality reference tools
radon hal -j -f quality reference tools
complexipy quality reference tools --max-complexity-allowed 21
python quality/check_crap.py python work/policy-v2-cyclomatic.json work/full-coverage.json work/policy-v2-python-crap.json
python tools/check_repository.py
python -m unittest discover -s tools/tests -v
python tools/verify_design.py --output work/policy-v2-design-checks.json
```

The checker CLI was additionally executed under coverage before the final
coverage combination. Rust CRAP was measured with the corresponding archived
Rust metrics and LLVM coverage. Hosted execution of the revised workflows is
pending; neither these quality results nor documentation builds qualify a PDE
trajectory.
