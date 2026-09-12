use crate::{
    row::{reservation_delta, Reservation, RowExecutor, COMPONENTS, STACK_BYTES},
    transform::{Lane, Plans, Snapshot, Timing},
    GLOBAL,
};
use stats_alloc::{Region, Stats};
use std::{
    sync::{Arc, Mutex},
    thread::{self, JoinHandle},
};

#[derive(Clone, Copy, Debug)]
pub enum Mode {
    Serial,
    RowParallel,
}

#[derive(Clone, Copy)]
enum Command {
    Reset(usize),
    Forward,
    Inverse,
    Snapshot(bool),
    Direct(usize),
    Inject(usize),
    Stop,
}

enum Reply {
    Ready(Stats),
    Done(Timing),
    Snapshot(Snapshot),
    Direct(f64),
    Ack,
    Failed(String),
}

struct Control {
    state: Mutex<ControlState>,
}
struct ControlState {
    command: Option<Command>,
    reply: Option<Reply>,
}

impl Control {
    fn new() -> Self {
        Self {
            state: Mutex::new(ControlState {
                command: None,
                reply: None,
            }),
        }
    }
    fn send(&self, command: Command) -> Result<(), String> {
        loop {
            let mut state = self.state.lock().map_err(|_| "control poisoned")?;
            if state.command.is_none() {
                state.command = Some(command);
                return Ok(());
            }
            drop(state);
            thread::yield_now();
        }
    }
    fn recv(&self) -> Result<Reply, String> {
        loop {
            let mut state = self.state.lock().map_err(|_| "control poisoned")?;
            if let Some(reply) = state.reply.take() {
                return Ok(reply);
            }
            drop(state);
            thread::yield_now();
        }
    }
    fn next(&self) -> Result<Command, String> {
        loop {
            let mut state = self.state.lock().map_err(|_| "control poisoned")?;
            if let Some(command) = state.command.take() {
                return Ok(command);
            }
            drop(state);
            thread::yield_now();
        }
    }
    fn reply(&self, reply: Reply) -> Result<(), String> {
        loop {
            let mut state = self.state.lock().map_err(|_| "control poisoned")?;
            if state.reply.is_none() {
                state.reply = Some(reply);
                return Ok(());
            }
            drop(state);
            thread::yield_now();
        }
    }
}

struct Component {
    control: Arc<Control>,
    thread: Option<JoinHandle<()>>,
}

pub struct TripletOwner {
    n: usize,
    mode: Mode,
    components: Vec<Component>,
    terminated: bool,
    published_generation: usize,
    dispatched: usize,
    construction: Stats,
}

impl TripletOwner {
    pub fn new(n: usize, mode: Mode) -> Result<Self, String> {
        if !matches!(n, 6 | 384 | 576) {
            return Err("closed prototype length".into());
        }
        let plans = Arc::new(Plans::new(n)?);
        let mut components = Vec::with_capacity(COMPONENTS);
        let mut construction = Stats::default();
        for component in 0..COMPONENTS {
            let control = Arc::new(Control::new());
            let worker_control = control.clone();
            let component_plans = plans.clone();
            let handle = thread::Builder::new()
                .name(format!("row-avx-component-{component}"))
                .stack_size(STACK_BYTES)
                .spawn(move || component_loop(n, component, mode, component_plans, worker_control))
                .map_err(|error| format!("component spawn: {error}"))?;
            let ready = control.recv()?;
            match ready {
                Reply::Ready(stats) => add_stats(&mut construction, stats),
                Reply::Failed(error) => return Err(error),
                _ => return Err("invalid startup reply".into()),
            }
            components.push(Component {
                control,
                thread: Some(handle),
            });
        }
        Ok(Self {
            n,
            mode,
            components,
            terminated: false,
            published_generation: 0,
            dispatched: 0,
            construction,
        })
    }

    pub fn reset(&mut self, variant: usize) -> Result<(), String> {
        self.broadcast(Command::Reset(variant))?;
        self.drain_ack()
    }

    pub fn cycle(&mut self) -> Result<(), String> {
        self.forward()?;
        self.inverse()?;
        Ok(())
    }

    pub fn forward(&mut self) -> Result<Timing, String> {
        self.transform(Command::Forward)
    }
    pub fn inverse(&mut self) -> Result<Timing, String> {
        self.transform(Command::Inverse)
    }

    fn transform(&mut self, command: Command) -> Result<Timing, String> {
        if self.terminated {
            return Err("triplet owner permanently terminated".into());
        }
        self.broadcast(command)?;
        self.dispatched += COMPONENTS;
        let mut timing = Timing::default();
        let mut error = None;
        for component in &self.components {
            match component.control.recv()? {
                Reply::Done(value) => timing.add(value),
                Reply::Failed(value) => {
                    error.get_or_insert(value);
                }
                _ => return Err("invalid transform reply".into()),
            };
        }
        if let Some(value) = error {
            self.terminated = true;
            return Err(value);
        }
        self.published_generation += 1;
        Ok(timing)
    }

    pub fn snapshot(&mut self, include_values: bool) -> Result<Vec<Snapshot>, String> {
        self.broadcast(Command::Snapshot(include_values))?;
        self.components
            .iter()
            .map(|component| match component.control.recv()? {
                Reply::Snapshot(value) => Ok(value),
                _ => Err("invalid snapshot reply".into()),
            })
            .collect()
    }

    pub fn direct_error(&mut self) -> Result<f64, String> {
        self.broadcast(Command::Direct(29))?;
        let mut maximum = 0.0_f64;
        for component in &self.components {
            match component.control.recv()? {
                Reply::Direct(value) => maximum = maximum.max(value),
                _ => return Err("invalid direct reply".into()),
            }
        }
        Ok(maximum)
    }

    pub fn audit(&self) -> Audit {
        Audit {
            reservation: reservation_delta(self.n).expect("admitted reservation"),
            construction: self.construction,
            mode: self.mode,
        }
    }

    fn broadcast(&self, command: Command) -> Result<(), String> {
        for component in &self.components {
            component.control.send(command)?;
        }
        Ok(())
    }

    fn drain_ack(&self) -> Result<(), String> {
        for component in &self.components {
            match component.control.recv()? {
                Reply::Ack => (),
                Reply::Failed(e) => return Err(e),
                _ => return Err("invalid ack".into()),
            }
        }
        Ok(())
    }

    fn inject(&mut self, component: usize, helper: usize) -> Result<(), String> {
        self.components[component]
            .control
            .send(Command::Inject(helper))?;
        match self.components[component].control.recv()? {
            Reply::Ack => Ok(()),
            _ => Err("invalid inject reply".into()),
        }
    }
}

impl Drop for TripletOwner {
    fn drop(&mut self) {
        for component in &self.components {
            let _ = component.control.send(Command::Stop);
        }
        for component in &mut self.components {
            if let Some(handle) = component.thread.take() {
                let _ = handle.join();
            }
        }
    }
}

fn component_loop(
    n: usize,
    component: usize,
    mode: Mode,
    plans: Arc<Plans>,
    control: Arc<Control>,
) {
    let mut lane = match Lane::new(n, &plans) {
        Ok(value) => value,
        Err(error) => {
            let _ = control.reply(Reply::Failed(error));
            return;
        }
    };
    let region = Region::new(GLOBAL);
    let mut row = match mode {
        Mode::Serial => None,
        Mode::RowParallel => match RowExecutor::new(n, &plans) {
            Ok(value) => Some(value),
            Err(error) => {
                let _ = control.reply(Reply::Failed(error));
                return;
            }
        },
    };
    let construction = region.change();
    if control.reply(Reply::Ready(construction)).is_err() {
        return;
    }
    while let Ok(command) = control.next() {
        let reply = match command {
            Command::Reset(variant) => {
                lane.reset(variant, component);
                Reply::Ack
            }
            Command::Forward => match row.as_mut() {
                Some(exec) => exec.forward(&mut lane),
                None => lane.forward_serial(),
            }
            .map(Reply::Done)
            .unwrap_or_else(Reply::Failed),
            Command::Inverse => match row.as_mut() {
                Some(exec) => exec.inverse(&mut lane),
                None => lane.inverse_serial(),
            }
            .map(Reply::Done)
            .unwrap_or_else(Reply::Failed),
            Command::Snapshot(include) => Reply::Snapshot(lane.snapshot(include)),
            Command::Direct(variant) => Reply::Direct(
                lane.direct_error(variant, component)
                    .unwrap_or(f64::INFINITY),
            ),
            Command::Inject(helper) => {
                if let Some(exec) = row.as_mut() {
                    exec.inject_failure(helper);
                }
                Reply::Ack
            }
            Command::Stop => break,
        };
        if control.reply(reply).is_err() {
            break;
        }
    }
}

pub fn failure_control() -> Result<(), String> {
    let mut owner = TripletOwner::new(6, Mode::RowParallel)?;
    owner.reset(41)?;
    owner.forward()?;
    let generation = owner.published_generation;
    let dispatched = owner.dispatched;
    owner.inject(1, 1)?;
    if owner.inverse().is_ok() {
        return Err("injected helper failure succeeded".into());
    }
    if owner.published_generation != generation {
        return Err("failure published external generation".into());
    }
    let after_failure = owner.dispatched;
    if after_failure != dispatched + COMPONENTS {
        return Err("failure did not drain all component owners".into());
    }
    if owner.forward().is_ok() {
        return Err("terminated owner accepted later request".into());
    }
    if owner.dispatched != after_failure {
        return Err("later refusal dispatched fresh work".into());
    }
    Ok(())
}

fn add_stats(total: &mut Stats, value: Stats) {
    total.allocations += value.allocations;
    total.deallocations += value.deallocations;
    total.reallocations += value.reallocations;
    total.bytes_allocated += value.bytes_allocated;
    total.bytes_deallocated += value.bytes_deallocated;
    total.bytes_reallocated += value.bytes_reallocated;
}

pub struct Audit {
    reservation: Reservation,
    construction: Stats,
    mode: Mode,
}
impl std::fmt::Display for Audit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} mode={:?} measured_row_construction_allocations={} measured_row_construction_deallocations={} measured_row_construction_reallocations={} measured_row_construction_bytes_allocated={} measured_row_construction_bytes_deallocated={} measured_row_construction_net_bytes={}", self.reservation, self.mode, self.construction.allocations, self.construction.deallocations, self.construction.reallocations, self.construction.bytes_allocated, self.construction.bytes_deallocated, self.construction.bytes_allocated.saturating_sub(self.construction.bytes_deallocated))
    }
}
