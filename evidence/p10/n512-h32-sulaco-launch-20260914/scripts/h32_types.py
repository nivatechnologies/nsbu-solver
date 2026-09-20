"""Shared typed models for the Sulaco temporal-h32 launch packet.

The frozen plan is untrusted JSON until :func:`h32_contract.validate_plan_shape`
has run; every consumer accesses it through these TypedDict shapes so no
value ever degrades to an unknown type.  The shapes mirror exactly what the
validation and freeze tooling enforce at runtime; nothing is trusted that
those checks have not verified.  No logic lives here.
"""

from __future__ import annotations

from pathlib import Path
from typing import Callable, Mapping, NotRequired, Optional, Protocol, TypedDict

LogFn = Callable[[str], None]
EnvGet = Callable[[str], Optional[str]]
Env = Mapping[str, str]


class PlanGuard(TypedDict):
    first_step_clock: int
    first_step_bundle: str
    rhs_calls: int
    cache_hits: int
    cache_misses: int
    steady_allocations: int
    maximum_local_error_ratio: float
    maximum_first_step_integration_seconds: int
    first_step_wall_seconds: int
    absolute_deadline_epoch: int
    minimum_launch_margin_seconds: NotRequired[int]


class PlanCapture(TypedDict):
    all_committed_states: int
    epoch: int
    coefficient_bytes: int
    snapshot_bytes: NotRequired[int]


class PlanResources(TypedDict):
    exact_capture_peak_bytes: int
    memory_floor_bytes: int
    address_space_limit_bytes: int
    disk_bound_bytes: int
    disk_floor_bytes: int
    probe_path: str


class PlanBarrier(TypedDict):
    decision_margin_seconds: int
    armed_receipt_seconds: int


class Plan(TypedDict):
    schema: str
    host: str
    from_rest: bool
    resume: str
    qualification: bool
    accepted_windows: int
    prepared_utc: str
    source_binding: str
    identity_source: NotRequired[str]
    profile: str
    schedule_identity: str
    attempt_schema: str
    observer_state_schema: str
    capture_terminal_schema: str
    binary_sha256: str
    watchdog_sha256: str
    preflight_sha256: str
    guard: PlanGuard
    profile_identity: dict[str, str]
    capture: PlanCapture
    resources: PlanResources
    barrier: PlanBarrier
    deadline_status: str
    budget_evidence: dict[str, str]
    _observed_plan_sha256: NotRequired[str]


class ChildHandle(Protocol):
    """Structural shape of the ``subprocess.Popen`` handles the packet passes
    through its teardown and census paths (test fakes satisfy it too)."""

    @property
    def pid(self) -> int: ...

    @property
    def returncode(self) -> Optional[int]: ...

    def poll(self) -> Optional[int]: ...

    def wait(self, timeout: Optional[float] = ...) -> Optional[int]: ...

    def terminate(self) -> None: ...

    def kill(self) -> None: ...


MeasureFn = Callable[[Plan, int, bool, Path], tuple[dict[str, int], list[str]]]
