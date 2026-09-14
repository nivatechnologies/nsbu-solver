import unittest.mock

from tools.local_run_monitor import classify


def test_classifies_live_terminal_and_unreachable() -> None:
    live = classify(0, '{"started":true}\nstep 12\n', "")
    assert live["state"] == "nonterminal"
    assert live["last_lines"][-1] == "step 12"
    success = classify(0, "EXIT_STATUS=0\nfinished", "")
    assert success["state"] == "terminal_success"
    failure = classify(0, "EXIT_STATUS=124\nlast step", "")
    assert failure["state"] == "terminal_failure"
    assert failure["exit_status"] == "124"
    unreachable = classify(255, "", "connection refused")
    assert unreachable["state"] == "unreachable"
    assert unreachable["error"] == "connection refused"


def test_retains_only_latest_eight_lines() -> None:
    stdout = "\n".join(f"line {i}" for i in range(1, 21))
    obs = classify(0, stdout, "")
    assert len(obs["last_lines"]) == 8
    assert obs["last_lines"] == [f"line {i}" for i in range(13, 21)]
    assert obs["exit_status"] is None
    assert obs["state"] == "nonterminal"


def test_removes_exit_status_marker_from_returned_lines() -> None:
    obs = classify(0, "step 1\nEXIT_STATUS=0\nstep 2\n", "")
    assert obs["exit_status"] == "0"
    assert obs["state"] == "terminal_success"
    assert obs["last_lines"] == ["step 1", "step 2"]
    assert not any(line.startswith("EXIT_STATUS=") for line in obs["last_lines"])


def test_missing_marker_is_nonterminal_even_with_finished_prose() -> None:
    obs = classify(0, "run finished successfully in 42s\n", "")
    assert obs["state"] == "nonterminal"
    assert obs["exit_status"] is None
    assert obs["error"] is None


def test_ssh_error_with_empty_stderr_yields_fallback_error() -> None:
    obs = classify(255, "", "")
    assert obs["state"] == "unreachable"
    assert obs["exit_status"] is None
    assert obs["last_lines"] == []
    assert obs["error"] == "ssh exited 255"


def test_utc_epoch_reflects_frozen_clock() -> None:
    with unittest.mock.patch("tools.local_run_monitor.time.time", return_value=1_757_740_800):
        terminal = classify(0, "EXIT_STATUS=0\n", "")
        nonterminal = classify(0, "step 9\n", "")
        unreachable = classify(1, "", "boom")
    assert terminal["utc_epoch"] == 1_757_740_800
    assert nonterminal["utc_epoch"] == 1_757_740_800
    assert unreachable["utc_epoch"] == 1_757_740_800
