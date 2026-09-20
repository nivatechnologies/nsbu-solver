"""Operations tooling tests: source inventory, stage freeze, preflight-only,
census mains and supervisor resource measurement — driven in-process so the
coverage evidence covers the shipped scripts, not copies of them."""

from __future__ import annotations

import contextlib
import hashlib
import io
import json
import os
import socket
import subprocess
import sys
import tempfile
import time
import unittest
from pathlib import Path
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "scripts"))
import freeze_sulaco_stage as freeze  # noqa: E402
import h32_contract as contract  # noqa: E402
import h32_launch_supervisor as driver  # noqa: E402
import leak_census  # noqa: E402
import runner_kill_census  # noqa: E402
import simulated_sulaco_preflight  # noqa: E402
import source_inventory  # noqa: E402
import process_guard as guard  # noqa: E402
from h32_owner import child_pids  # noqa: E402

PACKET = Path(__file__).resolve().parents[1]
FAST_METHOD = ("test_h32_receipts.CreateOnlyTests"
               ".test_written_proves_durable_content_and_directory")
PRELIM = 207627647760


class SourceInventoryTests(unittest.TestCase):
    def test_inventory_covers_case_json_and_workspace_root(self):
        lines, run_source = source_inventory.build_list()
        self.assertEqual(len(lines), len(source_inventory.inventory()))
        self.assertGreater(len(lines), 500)
        joined = "\n".join(lines)
        self.assertIn("similarity-mms-v2.json", joined)
        self.assertIn("rust-toolchain.toml", joined)
        self.assertNotIn("/target/", joined)
        self.assertEqual(len(run_source), 64)

    def test_missing_local_crate_is_refused(self):
        with tempfile.TemporaryDirectory() as name:
            with self.assertRaises(SystemExit):
                source_inventory.inventory(Path(name))

    def test_main_write_is_reproducible(self):
        lines, run_source = source_inventory.build_list()
        again = hashlib.sha256(("\n".join(lines) + "\n").encode()).hexdigest()
        self.assertEqual(run_source, again)
        with tempfile.TemporaryDirectory() as name:
            out = Path(name) / "list"
            with mock.patch.object(source_inventory, "LIST_OUT", out):
                rc = source_inventory.main(["--write"])
            self.assertEqual(rc, 0)
            self.assertEqual(len(out.read_text().splitlines()), len(lines))


class FreezeTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.binary = self.root / "solver"
        self.binary.write_text(
            "#!/bin/sh\n"
            f"echo 'preflight source=... total={PRELIM} cap={PRELIM}'\n")
        self.binary.chmod(0o755)
        self.stage = self.root / "stage"
        self.patches = [
            mock.patch.object(freeze, "BINARY", self.binary),
            mock.patch.object(freeze, "STAGE", self.stage),
            mock.patch.object(freeze, "DEADLINE_HOURS", 48),
        ]
        for patch in self.patches:
            patch.start()
            self.addCleanup(patch.stop)

    def test_budget_is_double_measured_cost(self):
        budget = freeze.budget()
        self.assertEqual(budget["first_step_wall_seconds"], 2103)
        self.assertEqual(budget["integration"], 1910)
        self.assertEqual(budget["deadline_seconds"], 48 * 3600)

    def test_exact_peak_requires_stated_equal_total_and_cap(self):
        self.assertEqual(freeze._exact_peak(f"total={PRELIM} cap={PRELIM}"),
                         PRELIM)
        with self.assertRaises(SystemExit):
            freeze._exact_peak("no numbers here")
        with self.assertRaises(SystemExit):
            freeze._exact_peak(f"total={PRELIM} cap={PRELIM + 1}")

    def test_create_exclusive_never_overwrites(self):
        target = self.root / "x"
        freeze.create_exclusive(target, b"one")
        with self.assertRaises(FileExistsError):
            freeze.create_exclusive(target, b"two")
        self.assertEqual(target.read_bytes(), b"one")

    def test_profile_identity_must_be_internally_exact(self):
        fields = freeze.profile_identity()
        self.assertEqual(fields["source"], source_inventory.build_list()[1])
        with mock.patch.object(freeze, "IDENTITY_DOC", self.root / "bad.json"):
            (self.root / "bad.json").write_text(json.dumps(
                {"schema": "x", "fields": [["source", "0" * 64]]}))
            with self.assertRaises(SystemExit):
                freeze.profile_identity()

    def test_full_freeze_binds_binary_watchdog_preflight_identity(self):
        now = int(time.time())
        result = freeze.freeze(now=now)
        plan, sha = result["plan"], result["plan_sha256"]
        self.assertEqual(len(sha), 64)
        self.assertEqual(plan["guard"]["absolute_deadline_epoch"],
                         now + 48 * 3600)
        self.assertEqual(plan["deadline_status"], "proposed_for_root")
        self.assertEqual(plan["binary_sha256"],
                         freeze.sha256_file(self.stage / "solver"))
        self.assertEqual(plan["watchdog_sha256"], freeze.sha256_file(
            PACKET / "watchdog" / "pgid-watchdog-v3.sh"))
        self.assertEqual(plan["identity_source"],
                         freeze.sha256_file(freeze.RUN_SOURCE_LIST))
        self.assertEqual(plan["profile_identity"]["source"],
                         plan["identity_source"])
        self.assertEqual(freeze.sha256_file(self.stage / "preflight.stdout"),
                         plan["preflight_sha256"])
        with self.assertRaises(SystemExit):  # create-only stage
            freeze.freeze(now=now)

    def test_frozen_stage_passes_real_preflight_only_path(self):
        result = freeze.freeze()
        plan_sha = result["plan_sha256"]
        argv = ["--stage", str(self.stage), "--plan-sha256", plan_sha]
        optin = {contract.OPTIN_ENV: "1",
                 contract.REVIEWED_HOST_ENV: "sulaco"}
        with mock.patch.object(driver, "real_mem_available_bytes",
                               lambda: 10 ** 15), \
                mock.patch.object(driver, "real_disk_free_bytes",
                                  lambda _p: 10 ** 15), \
                mock.patch.dict(os.environ, optin):
            code = simulated_sulaco_preflight.main(argv)
        self.assertEqual(code, 0, "simulated-sulaco preflight must be READY")

    def _launch_env(self):
        return (mock.patch.object(driver, "real_hostname", lambda: "sulaco"),
                mock.patch.object(driver, "real_mem_available_bytes",
                                  lambda: 10 ** 15),
                mock.patch.object(driver, "real_disk_free_bytes",
                                  lambda _p: 10 ** 15),
                mock.patch.dict(os.environ, {contract.OPTIN_ENV: "1",
                                             contract.REVIEWED_HOST_ENV:
                                             "sulaco"}))

    def test_launch_refuses_plan_sha_mismatch(self):
        freeze.freeze()
        with mock.patch.object(driver, "real_hostname", lambda: "sulaco"):
            code = driver.main(["launch", "--stage", str(self.stage),
                                "--plan-sha256", "0" * 64])
        self.assertNotEqual(code, 0)
        self.assertFalse((self.stage / "launch-receipt.json").exists())

    def test_launch_refuses_when_barrier_dir_missing(self):
        result = freeze.freeze()
        os.rmdir(self.stage / "barrier")
        patches = self._launch_env()
        for patch in patches:
            patch.start()
        self.addCleanup(mock.patch.stopall)
        code = driver.main(["launch", "--stage", str(self.stage),
                            "--plan-sha256", result["plan_sha256"]])
        self.assertEqual(code, 64)

    def test_launch_refuses_when_log_file_exists(self):
        result = freeze.freeze()
        (self.stage / "logs" / "solver.stdout").write_text("stale")
        patches = self._launch_env()
        for patch in patches:
            patch.start()
        self.addCleanup(mock.patch.stopall)
        code = driver.main(["launch", "--stage", str(self.stage),
                            "--plan-sha256", result["plan_sha256"]])
        self.assertIn(code, (64, 68), "stale logs must refuse the launch")

    def test_launch_refuses_without_verified_subreaper(self):
        import h32_owner
        result = freeze.freeze()
        patches = list(self._launch_env()) + [
            mock.patch.object(h32_owner, "enable_child_subreaper",
                              lambda *a, **k: False)]
        for patch in patches:
            patch.start()
        self.addCleanup(mock.patch.stopall)
        code = driver.main(["launch", "--stage", str(self.stage),
                            "--plan-sha256", result["plan_sha256"]])
        self.assertEqual(code, 70)
        self.assertFalse((self.stage / "launch-receipt.json").exists())

    def test_launch_refuses_missing_optin_through_reasons_loop(self):
        result = freeze.freeze()
        with mock.patch.dict(os.environ, clear=False):
            os.environ.pop(contract.OPTIN_ENV, None)
            os.environ.pop(contract.REVIEWED_HOST_ENV, None)
            with contextlib.redirect_stdout(io.StringIO()) as captured:
                code = driver.main(["launch", "--stage", str(self.stage),
                                    "--plan-sha256", result["plan_sha256"]])
        self.assertNotEqual(code, 0)
        self.assertIn("refused:", captured.getvalue())
        self.assertFalse((self.stage / "launch-receipt.json").exists())

    def test_launch_recreates_absent_logs_dir_then_refuses_subreaper(self):
        import h32_owner
        result = freeze.freeze()
        os.rmdir(self.stage / "logs")
        patches = list(self._launch_env()) + [
            mock.patch.object(h32_owner, "enable_child_subreaper",
                              lambda *a, **k: False)]
        for patch in patches:
            patch.start()
        self.addCleanup(mock.patch.stopall)
        code = driver.main(["launch", "--stage", str(self.stage),
                            "--plan-sha256", result["plan_sha256"]])
        self.assertEqual(code, 70)
        self.assertTrue((self.stage / "logs").is_dir())
        self.assertFalse((self.stage / "launch-receipt.json").exists())

    @unittest.skipIf(socket.gethostname() == "sulaco",
                     "running on the gated host itself")
    def test_host_gate_refusal_is_real(self):
        result = freeze.freeze()
        argv = ["preflight-only", "--stage", str(self.stage),
                "--plan-sha256", result["plan_sha256"]]
        with mock.patch.object(driver, "real_mem_available_bytes",
                               lambda: 10 ** 15), \
                mock.patch.object(driver, "real_disk_free_bytes",
                                  lambda _p: 10 ** 15):
            code = driver.main(argv)  # un-spoofed real hostname
        self.assertNotEqual(code, 0)


class CensusMainTests(unittest.TestCase):
    def test_leak_census_main_reports_clean(self):
        with tempfile.TemporaryDirectory() as name:
            out = Path(name)
            jobs = [(FAST_METHOD,
                     [sys.executable, "-m", "unittest", "-v", FAST_METHOD])]
            with mock.patch.object(leak_census, "build_jobs",
                                   lambda _tests: jobs):
                rc = leak_census.main(["--repeats", "1", "--out", str(out)])
            self.assertEqual(rc, 0)
            records = [json.loads(line) for line in
                       (out / "census.jsonl").read_text().splitlines()]
            self.assertEqual(len(records), 1)
            self.assertEqual(records[0]["rc"], 0)
            self.assertEqual(records[0]["leak_count"], 0)

    def test_runner_kill_main_cleans_its_adoptions(self):
        with tempfile.TemporaryDirectory() as name:
            out = Path(name)
            rc = runner_kill_census.main(
                ["--methods", FAST_METHOD, "--delays", "0.35",
                 "--out", str(out)])
            self.assertIn(rc, (0, 1))
            records = [json.loads(line) for line in
                       (out / "runner-kill.jsonl").read_text().splitlines()]
            self.assertEqual(len(records), 1)
            self.assertEqual(records[0]["method"], FAST_METHOD)
        self.assertEqual(child_pids(Path("/proc"), os.getpid()), [])


class SupervisorResourceTests(unittest.TestCase):
    def _plan(self, **overrides):
        resources = {"exact_capture_peak_bytes": 1000,
                     "memory_floor_bytes": 500,
                     "disk_bound_bytes": 100,
                     "disk_floor_bytes": 120,
                     "address_space_limit_bytes": 2 ** 40}
        resources.update(overrides)
        return {"resources": resources,
                "capture": {"coefficient_bytes": 4096}}

    def test_rss_outside_cap_is_reported(self):
        with mock.patch.object(driver, "real_rss_bytes",
                               lambda _pid: 10 ** 18), \
                mock.patch.object(driver, "real_mem_available_bytes",
                                  lambda: 10 ** 9), \
                mock.patch.object(driver, "real_disk_free_bytes",
                                  lambda _p: 10 ** 9):
            _sample, reasons = driver.measure_resources(
                self._plan(), os.getpid(), True, Path("/tmp"))
        self.assertIn("rss_outside_cap(rss=1000000000000000000,cap=1000)",
                      reasons)

    def test_clean_measurement_passes(self):
        with mock.patch.object(driver, "real_rss_bytes", lambda _pid: 999), \
                mock.patch.object(driver, "real_mem_available_bytes",
                                  lambda: 10 ** 9), \
                mock.patch.object(driver, "real_disk_free_bytes",
                                  lambda _p: 10 ** 9):
            sample, reasons = driver.measure_resources(
                self._plan(), os.getpid(), True, Path("/tmp"))
        self.assertEqual(reasons, [])
        self.assertEqual(sample["solver_rss_bytes"], 999)

    def test_open_exclusive_refuses_existing(self):
        with tempfile.TemporaryDirectory() as name:
            target = Path(name) / "x"
            target.write_text("keep")
            with self.assertRaises(FileExistsError):
                driver.open_exclusive(target)

    def test_meminfo_reads_good_absent_and_unreadable(self):
        with tempfile.TemporaryDirectory() as name:
            proc = Path(name)
            (proc / "meminfo").write_text("MemTotal: 1 kB\nMemAvailable: 7 kB\n")
            self.assertEqual(driver.real_mem_available_bytes(proc), 7 * 1024)
            (proc / "meminfo").write_text("MemTotal: 1 kB\n")
            self.assertEqual(driver.real_mem_available_bytes(proc), -1)
            (proc / "meminfo").write_text("MemAvailable: junk kB\n")
            self.assertEqual(driver.real_mem_available_bytes(proc), -1)
        self.assertEqual(driver.real_mem_available_bytes(Path(name) / "none"), -1)

    def test_rss_reads_good_missing_and_unparsable(self):
        with tempfile.TemporaryDirectory() as name:
            proc = Path(name)
            (proc / "42").mkdir()
            (proc / "42" / "status").write_text("VmRSS: 5 kB\n")
            self.assertEqual(driver.real_rss_bytes(42, proc), 5 * 1024)
            (proc / "42" / "status").write_text("VmRSS: junk kB\n")
            self.assertEqual(driver.real_rss_bytes(42, proc), -1)
            (proc / "42" / "status").write_text("Name: solver\nUmask: 0022\n")
            self.assertEqual(driver.real_rss_bytes(42, proc), -1)
            self.assertEqual(driver.real_rss_bytes(424242, proc), -1)

    def test_ancestor_census_excludes_own_tree(self):
        with tempfile.TemporaryDirectory() as name:
            proc = Path(name)
            tree = {"1": "1 (init) S 0 0 0", "2": "2 (systemd) S 1 0 0",
                    "10": "10 (bash) S 2 0 0", "11": "11 (python) S 10 0 0",
                    "20": "20 (other) S 1 0 0", "99": "unreadable"}
            for pid, stat in tree.items():
                (proc / pid).mkdir()
                if pid != "99":
                    (proc / pid / "stat").write_text(stat)
            self.assertEqual(driver._ancestor_pids(proc, 10), {10, 11, 2})

    def test_census_cmdline_rendering(self):
        self.assertEqual(leak_census._show(b"sleep\x00300\x00"), "sleep 300")
        self.assertEqual(leak_census._show(b"\x00\x00"), "[none]")


if __name__ == "__main__":
    unittest.main()
