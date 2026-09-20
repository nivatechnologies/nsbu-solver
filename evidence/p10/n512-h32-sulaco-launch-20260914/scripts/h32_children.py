"""Owned-child lifecycle facade: deferred signals, immutable identity,
full drain, persistent subreaper adoption and complete reaping.

The implementation is split for file-size discipline:

* :mod:`h32_owner` — verified PR_SET_CHILD_SUBREAPER ownership, immutable
  PID/PGID/starttime/cmdline identity primitives, PPID-descendant discovery,
  identity-bound signaling and complete waitpid reaping;
* :mod:`h32_children_core` — SIGINT/SIGTERM deferral windows, driver signal
  handlers and the owned-cleanup driver loop (cleanup can never be
  interrupted by a second signal, and its exceptions are reported, never
  suppressed);
* :mod:`h32_teardown` — group drain, identity-bound direct escalation,
  truthful watchdog teardown and the phase-independent, unconditional
  ``cleanup_children``.

Every name below is re-exported unchanged so ``import h32_children as kids``
keeps working for the driver, the tests and the census tooling.
"""

from __future__ import annotations

from h32_children_core import (
    DEFERRED_SIGNALS,
    GRACE_SECONDS,
    Terminated,
    deferred_signals,
    describe_signal,
    install_signal_handlers,
    make_preexec,
    restore_child_signal_mask,
    restore_signal_handlers,
    run_with_owned_cleanup,
    stable_identity,
    watchdog_started,
)
from h32_owner import (
    PROTECTED_PIDS,
    Identity,
    child_subreaper_active,
    enable_child_subreaper,
    identity_matches,
    parse_proc_stat,
    pump_adoptions,
    read_identity,
    reap_all,
    reap_untracked_zombies,
    sha256_bytes,
)
from h32_teardown import (
    drain_one as _drain_one,
    cleanup_children,
    drain_group,
    group_member_identities,
    group_members,
    reap,
    stop_watchdog,
)

__all__ = [
    "DEFERRED_SIGNALS", "GRACE_SECONDS", "PROTECTED_PIDS", "Identity",
    "Terminated", "child_subreaper_active", "cleanup_children",
    "describe_signal", "deferred_signals", "drain_group", "_drain_one",
    "enable_child_subreaper", "group_member_identities", "group_members",
    "identity_matches", "install_signal_handlers", "make_preexec",
    "parse_proc_stat", "pump_adoptions", "read_identity", "reap", "reap_all",
    "reap_untracked_zombies", "restore_child_signal_mask",
    "restore_signal_handlers", "run_with_owned_cleanup", "sha256_bytes",
    "stable_identity", "stop_watchdog", "watchdog_started",
]
