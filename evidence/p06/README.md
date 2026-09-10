# P06 initial Rust concentrating diagnostic

The first bounded Rust v2 probe starts from exact rest and reaches 1/256 using
32 transactional macro attempts. Each commits the two-half-step result, so the
accepted path has 64 CM steps of duration 1/16384. The coarse steps are used only
for the local discrepancy test. The analytical reference is never requested by
the integration driver. [The record](summary.json) and [full half-spectrum](initial-probe-state.tsv)
retain actual counts and budgets. No PDE window is accepted.

The N=4 retained and force-evaluation grids are intentionally diagnostic. Neither
spatial nor force-sampling resolution is established. Two independent Python
trajectories at 80 and 120 digits are being evaluated for the same committed fine
step size. Smooth nonlinear temporal and grid studies and the public bounded
runner are under separate P06 verification.

## Reproduction recipe

This is a one-off execution recipe using the P05 public APIs, not a maintained
simulation command. From checkout commit `73ca2b3`, create `work/p06-probe/Cargo.toml`
and `work/p06-probe/src/main.rs` with the following contents, then run
`cargo run --release --manifest-path work/p06-probe/Cargo.toml`. The driver writes
coefficients to stdout and metadata to stderr. Its reservation includes 1 MiB of
explicit metadata/stack/allocator allowance. Later package work must retain this
probe's diagnostic classification unless complete refinement requirements pass.

```toml
[package]
name = "nsbu-p06-diagnostic-probe"
version = "0.0.0"
edition = "2021"
[workspace]
[dependencies]
nsbu-solver = { path = "../../crates/nsbu-solver" }
nsbu-benchmarks = { path = "../../crates/nsbu-benchmarks" }
```

```rust
use nsbu_solver::domain::{Domain,Epoch,ExtraStorage,ResourcePlan,SpectralState,TickClock};
use nsbu_solver::integrators::{attempt::AttemptWorkspace,indicator::Tolerances,rhs::SpectralRhs,transaction::{commit_candidate,CandidateState}};
use nsbu_benchmarks::provider::V2Force;
fn main() {
 let domain=Domain::new([4;3],[1.0;3],1.0).unwrap();
 let limits=V2Force::preflight(domain,domain.layout()).unwrap();
 let source=SpectralRhs::<V2Force>::reservation(domain,limits).unwrap();
 let plan=ResourcePlan::new(domain,ExtraStorage{fft:0,force:source,diagnostics:AttemptWorkspace::reservation(domain).unwrap(),overhead:1024*1024},2*1024*1024,Epoch(0)).unwrap();
 eprintln!("diagnostic-only preflight_bytes={} cap_bytes={} reference_assignments=0",plan.total(),2*1024*1024);
 let clock=TickClock::from_rest(-20,8192).unwrap();
 let mut state=SpectralState::from_rest(plan,clock,Epoch(0)).unwrap();
 let mut candidate=CandidateState::new(plan,clock,Epoch(0)).unwrap();
 let mut work=AttemptWorkspace::new(plan).unwrap();
 let provider=V2Force::new(domain,domain.layout(),limits.storage_bytes).unwrap();
 let mut rhs=SpectralRhs::new(domain,provider,0.3,source).unwrap();
 let mut maximum=[0.0_f64;2];
 let mut calls=0;
 for _ in 0..32 {
  let attempt=work.try_advance(&state,&mut candidate,128,Tolerances{absolute:[1e-5,1e-4],relative:[1e-5;2]},&mut rhs).unwrap();
  for (value,ratio) in maximum.iter_mut().zip(attempt.indicators.ratios) {*value=value.max(ratio);}
  let Some(token)=attempt.accepted else {panic!("local rejection at elapsed {}: {:?}",state.clock().elapsed(),attempt.indicators);};
  commit_candidate(plan,&mut state,&mut candidate,token).unwrap();
  calls+=rhs.consumption()[0];
 }
 eprintln!("elapsed_ticks={} remaining_ticks={} accepted_macro_steps={} rhs_calls={} max_local_ratios={maximum:?} accepted_pde_windows=0",state.clock().elapsed(),state.clock().remaining(),state.accepted_steps(),calls);
 for index in 0..domain.layout().half_len(){
  let position=domain.layout().position(index).unwrap();
  if domain.layout().is_nyquist(position).unwrap(){continue;}
  let mode=domain.layout().mode(position).unwrap();
  print!("{}\t{}\t{}",mode[0],mode[1],mode[2]);
  for axis in 0..3 {let v=state.component(axis).unwrap()[index];print!("\t{:.17e}\t{:.17e}",v.re,v.im);}
  println!();
 }
}
```

## Observed execution metadata

```text
diagnostic-only preflight_bytes=1154784 cap_bytes=2097152 reference_assignments=0
elapsed_ticks=4096 remaining_ticks=4096 accepted_macro_steps=32 rhs_calls=384 max_local_ratios=[0.7249142752846672, 0.7678012206754569] accepted_pde_windows=0
```
