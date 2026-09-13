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
