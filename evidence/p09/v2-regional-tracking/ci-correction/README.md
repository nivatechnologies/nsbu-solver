# Regional tracking CI correction

The hosted Rust quality run `34596983665` for source revision
`441da25d79cf51439c5b3e99b43c9828de1ba2db` failed its build, lint, document and
package step because the focused regional test's cognitive complexity was 22.
The earlier regional summary's cognitive maximum of 13 excluded this test and
does not cover that failure.

This correction extracts the test's existing regional sample and quantity
assertions into cohesive helpers. It preserves every assertion, fixture,
tolerance, and family evolution operation; it changes no production logic or
historical report. The corrected test file is 313 lines. Rust code analysis
reports cognitive complexity 8 for
`regional_findings_match_the_existing_global_path_on_identical_actual_states`,
with helper maxima of 3 and a file maximum of 8.

Validation on the correction worktree:

```text
cargo test -p nsbu-benchmarks --test v2_regional_tracking regional_findings_match_the_existing_global_path_on_identical_actual_states -- --nocapture  => 1 passed, 0 failed
cargo fmt --all                                                                                                                         => passed
cargo clippy -p nsbu-benchmarks --test v2_regional_tracking --locked -- -D warnings                                                      => passed
```

The focused test retains the endpoint regional Hessian diagnostic and all
scientific limits. Accepted concentrating PDE windows remain zero; this
correction provides no PDE proof or qualification claim.
