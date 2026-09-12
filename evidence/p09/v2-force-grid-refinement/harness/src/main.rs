use nsbu_benchmarks::runtime_force::ForceSettings;
use nsbu_solver::{diagnostics::{comparison::ComparisonPlan, norms::Norms}, domain::{Domain, Layout, TickClock}, integrators::forcing::{ForceWork, PrescribedForce}, spectral::modal, Complex64, SolverError};
use sha2::{Digest, Sha256};
const CLOCKS:[u128;2]=[2047,4096]; const GRIDS:[usize;4]=[24,48,96,192]; const CAP:usize=1024*1024*1024;
fn main()->Result<(),SolverError>{
 let dry=std::env::args().skip(1).collect::<Vec<_>>().as_slice()==["--dry-run"];
 let domain=Domain::new([24;3],[1.;3],1.)?;
 let settings=[
  ForceSettings{samples:Layout::new([24;3])?,workers:12}, ForceSettings{samples:Layout::new([48;3])?,workers:12},
  ForceSettings{samples:Layout::new([96;3])?,workers:12}, ForceSettings{samples:Layout::new([192;3])?,workers:12},
 ];
 let limits=[settings[0].limits(domain)?,settings[1].limits(domain)?,settings[2].limits(domain)?,settings[3].limits(domain)?];
 let fields=4usize.checked_mul(3).and_then(|v|v.checked_mul(domain.layout().half_len())).and_then(|v|v.checked_mul(16)).ok_or(SolverError::SizeOverflow)?; let peak=limits[3].storage_bytes.checked_add(fields).and_then(|v|v.checked_add(4096)).ok_or(SolverError::SizeOverflow)?;
 let provider_work=limits.into_iter().map(|v|v.work_units).try_fold(0usize,|x,y|x.checked_add(y)).and_then(|v|v.checked_mul(CLOCKS.len())).ok_or(SolverError::SizeOverflow)?; let comparisons=ComparisonPlan::new(domain,domain)?.work_units().checked_mul(14).and_then(|v|v.checked_mul(CLOCKS.len())).ok_or(SolverError::SizeOverflow)?; let projection=4*3*domain.layout().half_len()*CLOCKS.len();
 println!("preflight source=N24 grids={GRIDS:?} workers=12 clocks={CLOCKS:?} limits={limits:?} fields_bytes={fields} sequential_peak_bytes={peak} cap={CAP} evaluations=8 provider_work={provider_work} transforms=24 comparison_work={comparisons} projection_component_visits={projection}"); if peak>CAP{return Err(SolverError::ResourceLimit)} if dry{return Ok(())}
 let zero=std::array::from_fn(|_|vec![Complex64::new(0.,0.);domain.layout().half_len()]);let compare=ComparisonPlan::new(domain,domain)?;
 for elapsed in CLOCKS {let clock=TickClock::restore(-20,8192,elapsed,8192-elapsed)?;let mut fields=[zero.clone(),zero.clone(),zero.clone(),zero.clone()];let mut works=[ForceWork{work_units:0,scalar_transforms:0};4];for i in 0..4{let mut force=settings[i].build(domain,CAP)?;works[i]=force.evaluate(clock,force.limits().ok_or(SolverError::UnknownProviderCost)?,fields[i].each_mut().map(Vec::as_mut_slice))?;}
 let raw=report(&compare,&zero,&fields)?;let raw_hash=fields.each_ref().map(|x|digest(x));for f in &mut fields{project(domain,f)?};let projected=report(&compare,&zero,&fields)?;println!("clock={elapsed} works={works:?} raw_hashes={raw_hash:02x?} raw={raw:?} projected_hashes={:02x?} projected={projected:?}",fields.each_ref().map(|x|digest(x)));}
 Ok(())
}
fn report(compare:&ComparisonPlan,zero:&[Vec<Complex64>;3],f:&[[Vec<Complex64>;3];4])->Result<([Norms;4],[Norms;3]),SolverError>{
 let n=[0,1,2,3].map(|i|compare.compare(zero.each_ref().map(Vec::as_slice),f[i].each_ref().map(Vec::as_slice)).map(|x|x.full));
 let p=[0,1,2].map(|i|compare.compare(f[i].each_ref().map(Vec::as_slice),f[i+1].each_ref().map(Vec::as_slice)).map(|x|x.full));
 Ok(([n[0]?,n[1]?,n[2]?,n[3]?],[p[0]?,p[1]?,p[2]?]))
}
fn project(domain:Domain,f:&mut[Vec<Complex64>;3])->Result<(),SolverError>{for i in 0..domain.layout().half_len(){let p=domain.layout().position(i)?;let v=if domain.layout().is_nyquist(p)?{[Complex64::new(0.,0.);3]}else{modal::project(modal::wavevector(domain,domain.layout().mode(p)?)?,std::array::from_fn(|a|f[a][i]))?};for(a,x)in v.into_iter().enumerate(){f[a][i]=x}}Ok(())}
fn digest(f:&[Vec<Complex64>;3])->[u8;32]{let mut h=Sha256::new();for c in f{for x in c{h.update(x.re.to_bits().to_le_bytes());h.update(x.im.to_bits().to_le_bytes())}}h.finalize().into()}
