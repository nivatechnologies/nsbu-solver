# Installation

## What can be installed now

This is a design-and-verification checkout. It has no Cargo workspace, published crate, compiled solver, or `nsbu` executable yet. The supported current workflow installs Python dependencies and runs the repository and mathematical checks. Do not use `cargo install nsbu-solver`; no such package is provided by this bootstrap.

Use Python 3.12. The original package recorded Python 3.12.8; this Linux checkout uses Python 3.12.3, SymPy 1.14.0 and mpmath 1.3.0. The pinned development dependencies are separate from the future Rust runtime dependencies.

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

## Preparing for Rust implementation

Rust is not needed for this checkout's existing commands. Once implementing P01, install Rust using the [official Rust instructions](https://rust-lang.org/tools/install/). On macOS/Linux, the documented installer is:

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

P01 must commit an exact tested `rust-toolchain.toml`, workspace manifests and `Cargo.lock`. The following commands are the **planned validation workflow after those files and crates exist**:

```sh
cargo build --workspace --locked
cargo test --workspace --locked
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo install --path crates/nsbu-cli --locked
nsbu --help
```

These commands are not supported by the current bootstrap. Published binary and crates.io instructions must be added only after the corresponding artifacts exist and a clean installation has been tested. Dependency versions and platform claims belong to the tested release, not this plan.

## Troubleshooting

A missing SymPy module usually means dependencies were installed with a different interpreter. Use the virtual environment's `python -m pip` and rerun with that same `python`. A frozen-hash failure means a reviewed input changed; restore the adopted bytes or create a reviewed amendment instead of simply changing the expected hash. A mathematical assertion failure is evidence to investigate, not a reason to run with `-O`.

No Niva account, checkout, adapter, GPU, or private service is required for these checks.

## Reference implementation and initial quality measurements

The same Python development environment supports the current independent
[reference commands](../reference/README.md). It still supplies no Rust binary.
For the verified Python quality gate, additionally install:

```sh
python -m pip install -r requirements-quality.txt
```

These tools measure coverage, cyclomatic and cognitive complexity, Halstead
metrics, strict typing, duplication, dead-code candidates, and mutations. The
Python mutation gate passes in its declared execution profile. See [quality evidence](QUALITY.md) for
per-metric results and limitations. A fresh installation of this pinned
environment was tested on Linux with Python 3.12.3.
