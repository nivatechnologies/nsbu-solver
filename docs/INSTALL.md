# Installation

## What can be installed now

The checkout provides Python verification and a Rust workspace containing the
installable `nsbu-solver` library, benchmark library, and `nsbu` CLI. The runtime alpha supports bounded CM/HO
diagnostics from rest, resource preflight, and same-profile external checkpoint
continuation. Alpha output is diagnostic-only and unqualified; no crate or
binary release is promised by this source checkout until the tracked readiness
gate is enabled. The concentrating experiment workflow remains in progress.

Use Python 3.12. The original package recorded Python 3.12.8; this Linux checkout uses Python 3.12.3, SymPy 1.14.0 and mpmath 1.3.0. The pinned development dependencies are separate from the Rust runtime dependencies.

## Downloaded alpha binary

Dated prereleases are published through the [alpha release workflow](ALPHA_RELEASES.md)
after its readiness and exact-source CI gates pass. The downloadable binary
supports Linux x86_64 with GNU/glibc and is tested on Ubuntu 24.04. Its source
manifest records actual dynamic-library and GLIBC symbol requirements. This
artifact does not support Alpine/musl, macOS or Windows. Use a source build for
other environments; those platforms remain unverified.

Extract the complete tar archive, verify its detached checksum and then run
`sha256sum -c SHA256SUMS` inside the extracted directory. The executable is
`bin/nsbu`; the rest of the directory is the matching complete source tree.

## macOS and Linux

Obtain a checkout or extract the source archive, then enter its root directory:

```sh
git clone https://github.com/nivatechnologies/nsbu-solver.git
cd nsbu-solver
python3 -m venv .venv
. .venv/bin/activate
python -m pip install -r requirements-dev.txt
python tools/check_repository.py
python -m unittest discover -s tools/tests -v
python tools/verify_design.py --output work/design-checks.json
```

When using an extracted archive, omit the clone command and enter the extracted `nsbu-solver` directory. Extract the complete archive, including `.github/`, `.gitignore` and `.gitattributes`. Do not transfer only the visible files. The tools do not require Git history to verify the packaged inputs. The runner creates the report's parent directory as needed.

## Windows PowerShell

```powershell
git clone https://github.com/nivatechnologies/nsbu-solver.git
cd nsbu-solver
py -3.12 -m venv .venv
.venv\Scripts\python.exe -m pip install -r requirements-dev.txt
.venv\Scripts\python.exe tools\check_repository.py
.venv\Scripts\python.exe -m unittest discover -s tools\tests -v
.venv\Scripts\python.exe tools\verify_design.py --output work\design-checks.json
```

Calling the virtual environment's Python directly avoids depending on shell activation. Windows execution is a target workflow, not a locally demonstrated platform result.

## Interpreting success

`check_repository.py` uses the Python standard library. It checks imported public component hashes, the exact benchmark's problem identity, required files, local Markdown file targets, and common accidental private-file/path inclusions. It is a packaging check, not a numerical solver test or comprehensive secret scanner.

The unit tests exercise incomplete transfers, changed frozen bytes, missing hidden files, broken documentation links, private adapter inclusion, disabled assertions, stale success reports and attempts to overwrite preserved evidence.

`verify_design.py` runs the preserved mathematical verification script with assertions enabled. It needs SymPy and mpmath, returns a nonzero exit status on failure, and refuses optimized Python because the preserved script uses assertions. The recomputed scientific report must equal the preserved report before execution metadata is added. The JSON report includes the unperformed work explicitly. Keep the original evidence under `docs/design/` unchanged; reports inside the checkout must use a `.json` path under ignored `work/` or under `evidence/`. Review new evidence before committing it. Relative output paths resolve against the checkout root, independently of the caller's working directory.

A fresh dependency download and hosted CI run require network access. Passing local checks with already-installed pinned dependencies is distinct from validating a fresh online installation.

## Building the Rust workspace

Rust is not needed for the Python checks. For the Rust workspace, install Rust using the [official Rust instructions](https://rust-lang.org/tools/install/). On macOS/Linux, the documented installer is:

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

The committed toolchain pins Rust 1.94.0. The workspace and lockfile use public dependencies only. Run:

```sh
cargo build --workspace --locked
cargo test --workspace --locked
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings -A dead_code
cargo install --path crates/nsbu-cli --locked
nsbu --help
nsbu --version
```

The v2 runtime commands are `nsbu v2` and `nsbu resume-v2`; their exact
options are shown by `nsbu v2 --help` and `nsbu resume-v2 --help`. The
default profile is documented in [Runtime alpha](RUNTIME_ALPHA.md). Unsupported
arguments return exit code 2. Resource, provider, checkpoint, or terminal
trajectory failures return nonzero. These build/install checks have been
exercised on Linux; other platforms remain unverified. This is bounded
diagnostic infrastructure, not evidence of a qualified PDE window.

For numerical smoke runs, prefer the optimized binary installed above or
`cargo run --release -q -p nsbu-cli -- v2 --dry-run`; debug builds can be much
slower at the default 32-attempt profile.

## Troubleshooting

A missing SymPy module usually means dependencies were installed with a different interpreter. Use the virtual environment's `python -m pip` and rerun with that same `python`. A frozen-hash failure means a reviewed input changed; restore the adopted bytes or create a reviewed amendment instead of simply changing the expected hash. A mathematical assertion failure is evidence to investigate, not a reason to run with `-O`.

No Niva account, checkout, adapter, GPU, or private service is required for these checks.

## Reference implementation and initial quality measurements

The same Python development environment supports the current independent
[reference commands](../reference/README.md). Rust build tools are independent of this Python environment.
For the verified Python quality gate, additionally install:

```sh
python -m pip install -r requirements-quality.txt
```

These tools measure coverage, cyclomatic and cognitive complexity, Halstead
metrics, strict typing, duplication, dead-code candidates, and mutations. The
Python mutation gate passes in its declared execution profile. See [quality evidence](QUALITY.md) for
per-metric results and limitations. A fresh installation of this pinned
environment was tested on Linux with Python 3.12.3.

For a local workspace packaging check, use a fresh target directory when retaining
the same development version across source changes:

```sh
package_target=$(mktemp -d)
cargo package --workspace --locked --target-dir "$package_target"
```

Cargo creates a temporary registry for workspace dependencies. Reusing that
registry path and package version can retain an older extracted dependency in the
local Cargo cache; a fresh target avoids this during package verification. This
command verifies archives and does not publish to a registry.
