cargo build --release --manifest-path work/p10-reference-spectrum/Cargo.toml --locked
cargo clippy --release --manifest-path work/p10-reference-spectrum/Cargo.toml --locked -- -D warnings
RUSTDOCFLAGS='-D warnings' cargo doc --manifest-path work/p10-reference-spectrum/Cargo.toml --locked --no-deps
ulimit -v 524288; timeout --signal=TERM --kill-after=5s 900s /usr/bin/time -v work/p10-reference-spectrum/target/release/p10-reference-spectrum 96 --run
ulimit -v 524288; timeout --signal=TERM --kill-after=5s 900s /usr/bin/time -v work/p10-reference-spectrum/target/release/p10-reference-spectrum 192 --run
/usr/bin/time -v work/p10-force-snapshot-tool/target/release/p10-force-snapshot-tool compare reference-m96 reference-m192 4096 96 96 96 192 1 1 EXPECTED_SHA256 EXPECTED_SHA256
