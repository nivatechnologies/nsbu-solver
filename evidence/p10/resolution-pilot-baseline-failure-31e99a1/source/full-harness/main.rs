use nsbu_benchmarks::{
    runtime_force::ForceSettings,
    v2_experiment::{diagnostic::{DiagnosticDriver, DiagnosticPlan, DiagnosticSettings}, probes::ProbePlan, FamilyPlan, FamilySettings},
    v2_run::{Plan as RunPlan, Run, Settings as RunSettings}, CASE_SHA256,
};
use nsbu_solver::{
    diagnostics::comparison::ComparisonPlan,
    domain::{Domain, Layout, SpectralState, TickClock}, experiment::control::{Configuration, Outcome},
    integrators::{indicator::Tolerances, method::Method, trajectory::RunLimits}, verification::times::TestedTimes,
};
use sha2::{Digest,Sha256};
use std::{io::Write,time::Instant};
const CAP:usize=4*1024*1024*1024;
fn main(){
 let mode=std::env::args().nth(1).unwrap_or_else(||"--dry-run".into()); let endpoint=if mode=="--preflight"{128}else{4096};
 let ae:&[u128]=if endpoint==128{&[0,64,128]}else{&[0,2048,4096]}; let pe:&[u128]=if endpoint==128{&[0,63,64,127,128]}else{&[0,2047,2048,4095,4096]}; let re:&[u128]=if endpoint==128{&[63,127]}else{&[2047,4095]};
 let accepted=clocks(ae);let probes=clocks(pe);let residual=clocks(re);let family=family_plan(endpoint,&accepted);
 let probe=ProbePlan::new(family,TestedTimes::new(&probes,probes.len()).unwrap(),probes.len(),CAP).unwrap();
 let diagnostic=DiagnosticPlan::new(family,probe,&residual,policy(),CAP).unwrap();let high_plan=high_force_plan(endpoint,accepted[0]);
 let scratch=std::mem::size_of::<ComparisonPlan>()+std::mem::size_of::<nsbu_solver::diagnostics::comparison::BandComparison>()+2*std::mem::size_of::<Sha256>()+2*std::mem::size_of::<[u8;32]>();
 let combined=diagnostic.bounds().joint_storage_bytes+high_plan.resources().total()+scratch;
 println!("source={} case_sha256={} mode={} endpoint={} cap={} combined_bytes={} harness_comparison_hash_scratch_bytes={}",env!("PILOT_SOURCE"),CASE_SHA256,mode,endpoint,CAP,combined,scratch);
 println!("spatial_settings={:?} diagnostic_bounds={:?}",family.settings(),diagnostic.bounds());println!("force_control_base=N12/M24 branch0 high={:?} high_resources={:?}",high_plan.settings(),high_plan.resources());assert!(combined<=CAP);
 if mode=="--dry-run"{println!("terminal=dry-run-complete");return} run(diagnostic,high_plan)
}
fn run(plan:DiagnosticPlan<'_>,high_plan:RunPlan){let started=Instant::now();let mut driver=DiagnosticDriver::new(plan).unwrap();let mut high=Run::from_rest(high_plan).unwrap();let mut events=0;let mut controls=0;
 loop{match driver.advance(){Ok(Some(event))=>{events+=1;let force=if event.accepted().sample().is_some(){while high.state().clock().elapsed()<event.clock().elapsed(){let o=high.step().unwrap();if !matches!(o,Outcome::Committed(_)){println!("terminal=high-force-stopped outcome={o:?}");return}}let base=driver.ordinary().branch(0).unwrap().state();assert_eq!(base.clock(),high.state().clock());let cmp=compare(base,high.state());controls+=1;println!("force_control=N12_M24_to_M48 full={:?} common={:?} newly_resolved={:?} mean_error={:?} base_hash={:02x?} high_hash={:02x?} base_clock={} high_clock={} base_settings={:?} high_settings={:?}",cmp.full,cmp.common,cmp.newly_resolved,cmp.mean_error,hash(base),hash(high.state()),base.clock().elapsed(),high.state().clock().elapsed(),driver.ordinary().branch(0).unwrap().plan().settings(),high.plan().settings()); true}else{false};let mut out=std::io::stdout().lock();writeln!(out,"event={} wall_seconds={:.6} elapsed={} force_control_emitted={} spatial_raw={:?}",events,started.elapsed().as_secs_f64(),event.clock().elapsed(),force,event).unwrap();out.flush().unwrap()},Ok(None)=>{println!("terminal=complete events={} force_controls={} wall_seconds={:.6} charged={:?} consumers={:?}",events,controls,started.elapsed().as_secs_f64(),driver.charged_work(),driver.consumer_work());break},Err(e)=>{println!("terminal=diagnostic-error events={} force_controls={} wall_seconds={:.6} error={:?} charged={:?} consumers={:?}",events,controls,started.elapsed().as_secs_f64(),e,driver.charged_work(),driver.consumer_work());break}}}}
fn compare(a:&SpectralState,b:&SpectralState)->nsbu_solver::diagnostics::comparison::BandComparison{ComparisonPlan::new(a.plan().domain(),b.plan().domain()).unwrap().compare([a.component(0).unwrap(),a.component(1).unwrap(),a.component(2).unwrap()],[b.component(0).unwrap(),b.component(1).unwrap(),b.component(2).unwrap()]).unwrap()}
fn hash(s:&SpectralState)->[u8;32]{let mut h=Sha256::new();for axis in 0..3{for z in s.component(axis).unwrap(){h.update(z.re.to_bits().to_le_bytes());h.update(z.im.to_bits().to_le_bytes())}}h.finalize().into()}
fn family_plan<'a>(endpoint:u128,times:&'a[TickClock])->FamilyPlan<'a>{FamilyPlan::new(FamilySettings{grids:[12,16,24],steps:[64,32,16],force:ForceSettings{samples:Layout::new([24;3]).unwrap(),workers:12},endpoint,tolerances:tolerances(),advective_limit:0.3},TestedTimes::new(times,times.len()).unwrap(),CAP).unwrap()}
fn high_force_plan(endpoint:u128,initial:TickClock)->RunPlan{RunPlan::from_rest(RunSettings{domain:Domain::new([12;3],[1.0;3],1.0).unwrap(),force:ForceSettings{samples:Layout::new([48;3]).unwrap(),workers:12},initial_clock:initial,configuration:Configuration{method:Method::CoxMatthews,limits:RunLimits{endpoint,step_ticks:16,maximum_attempts:usize::try_from(endpoint/16).unwrap()},tolerances:tolerances()},advective_limit:0.3},CAP).unwrap()}
fn policy()->DiagnosticSettings{DiagnosticSettings{physical_samples:Layout::new([24;3]).unwrap(),pressure_samples:Layout::new([48;3]).unwrap(),reference_samples:Layout::new([24;3]).unwrap(),physical_floors:[1e-8,1e-7,1e-6,1e-7],pressure_floors:[1e-8,1e-7],reference_floors:[1e-8,1e-7,1e-6,1e-7],regional_root_budget:128}}
fn tolerances()->Tolerances{Tolerances{absolute:[1e-5,1e-4],relative:[0.0;2]}}
fn clocks(v:&[u128])->Vec<TickClock>{v.iter().map(|&e|TickClock::restore(-20,8192,e,8192-e).unwrap()).collect()}
