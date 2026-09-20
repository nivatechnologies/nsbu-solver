"""Signal-deferral core shared by the guarded launch driver.

Rules enforced here (each covered by a focused test):
* SIGINT/SIGTERM are deferred across every spawn-plus-registration window;
* the driver turns any termination into a :class:`Terminated` so the owned
  cleanup ALWAYS runs before the exit code is reported;
* during the cleanup+abort window SIGINT/SIGTERM are additionally BLOCKED:
  a second termination signal must never interrupt an in-flight teardown;
  a cleanup exception is never suppressed — it is folded into the abort
  descriptor and logged.

Identity primitives live in :mod:`h32_owner`; teardown in
:mod:`h32_teardown`.  ``h32_children`` re-exports all of them.
"""

from __future__ import annotations

import resource
import signal
import threading
from pathlib import Path
from types import FrameType
from typing import Callable, Iterable, Optional, cast

from h32_owner import Identity, read_identity

DEFERRED_SIGNALS = frozenset({signal.SIGINT, signal.SIGTERM})
GRACE_SECONDS = 60


class Terminated(BaseException):
    """Raised inside the driver when SIGTERM arrives during a guarded launch."""

    def __init__(self, signum: int) -> None:
        super().__init__(f"signal={signum}")
        self.signum = signum


def stable_identity(pid: int, proc_root: Path, sleep: Callable[[float], None],
                    samples: int = 20, interval: float = 0.05) -> Optional[Identity]:
    previous: Optional[Identity] = None
    for _ in range(samples):
        current = read_identity(pid, proc_root)
        if (current is not None and current.state != "Z" and current.pgid == current.pid
                and previous is not None and previous.pgid == current.pgid
                and previous.starttime == current.starttime
                and previous.cmdline_sha256 == current.cmdline_sha256):
            return current
        previous = current
        sleep(interval)
    return None


# --- spawn-window signal deferral ---------------------------------------------


class _DeferredSignals:
    """Block SIGINT/SIGTERM for the Popen-plus-registration window only."""

    _previous: Optional[set[signal.Signals]]

    def __enter__(self) -> None:
        try:
            self._previous = cast(set[signal.Signals], signal.pthread_sigmask(
                signal.SIG_BLOCK, set(DEFERRED_SIGNALS)))
        except (OSError, ValueError):  # pragma: no cover - non-main thread
            self._previous = None

    def __exit__(self, exc_type: object, exc: object, tb: object) -> None:
        if self._previous is not None:
            signal.pthread_sigmask(signal.SIG_SETMASK, self._previous)


def deferred_signals() -> "_DeferredSignals":
    return _DeferredSignals()


def restore_child_signal_mask() -> None:
    signal.pthread_sigmask(signal.SIG_UNBLOCK, set(DEFERRED_SIGNALS))


def make_preexec(address_space_bytes: int) -> Callable[[], None]:
    def _preexec() -> None:
        resource.setrlimit(resource.RLIMIT_AS, (address_space_bytes, address_space_bytes))
        restore_child_signal_mask()

    return _preexec


SignalHandler = Callable[[int, "FrameType | None"], object]


# --- driver-level interruption plumbing ---------------------------------------


def _raise_on_signal(signum: int, _frame: object) -> None:
    raise Terminated(signum)


def install_signal_handlers() -> Optional[dict[int, object]]:
    if threading.current_thread() is not threading.main_thread():
        return None
    try:
        return {
            signal.SIGTERM: signal.signal(signal.SIGTERM, _raise_on_signal),
            signal.SIGINT: signal.signal(signal.SIGINT, _raise_on_signal),
        }
    except (ValueError, OSError):
        return None


def restore_signal_handlers(previous: Optional[dict[int, object]]) -> None:
    if not previous:
        return
    for signum, handler in previous.items():
        try:
            signal.signal(signum, cast(SignalHandler, handler))
        except (ValueError, OSError):
            pass


def describe_signal(error: BaseException) -> str:
    if isinstance(error, Terminated):
        try:
            return f"signal_{signal.Signals(error.signum).name}"
        except ValueError:
            return f"signal_{error.signum}"
    if isinstance(error, KeyboardInterrupt):
        return "signal_KeyboardInterrupt"
    return f"abort_{type(error).__name__}"


def _block_deferred_signals() -> Optional[set[signal.Signals]]:
    try:
        return cast(set[signal.Signals],
                    signal.pthread_sigmask(signal.SIG_BLOCK, set(DEFERRED_SIGNALS)))
    except (OSError, ValueError):  # pragma: no cover - non-main-thread
        return None


def run_with_owned_cleanup(body: Callable[[], int], cleanup: Callable[[], None],
                           log: Callable[[str], None], cleanup_code: int = 70,
                           on_abort: Optional[Callable[[str], None]] = None) -> int:
    """Run the guarded body; on ANY caught signal/exception run cleanup and
    then call ``on_abort(descriptor)`` UNCONDITIONALLY, so a failure receipt
    is published even when the cleanup itself was completely clean."""
    previous = install_signal_handlers()
    if previous is None:
        log("warning: signal handlers not installed (not the main thread)")
    try:
        return body()
    except BaseException as error:  # noqa: BLE001 - must cover BaseException
        descriptor = describe_signal(error)
        log(f"abort: {descriptor}")
        blocked = _block_deferred_signals()
        try:
            try:
                cleanup()
            except Exception as nested:  # noqa: BLE001 - reported, never silent
                descriptor = f"{descriptor}+cleanup_error({nested!r})"
                log(f"cleanup_error: {nested!r}")
            if on_abort is not None:
                try:
                    on_abort(descriptor)
                except Exception as nested:  # noqa: BLE001 - reported, never silent
                    log(f"abort_receipt_error: {nested!r}")
        finally:
            if blocked is not None:
                try:
                    signal.pthread_sigmask(signal.SIG_SETMASK, blocked)
                except (OSError, ValueError):  # pragma: no cover
                    pass
        return cleanup_code
    finally:
        restore_signal_handlers(previous)


def watchdog_started(log_lines: Iterable[str], solver_pid: int, pgid: int) -> bool:
    marker = f"started leader_pid={solver_pid} process_group={pgid}"
    return any(marker in line for line in log_lines)
