"""Exercise bounded CI execution without Cargo, Docker or a database."""
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time
import unittest

ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / ".github/scripts/run-ci-check.sh"


class CiCheckTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.directory = Path(temporary.name)
        self.logs = self.directory / "logs"
        self.summary = self.directory / "summary.md"
        self.env = {**os.environ, "CITADEL_CI_LOG_DIRECTORY": str(self.logs),
                    "GITHUB_STEP_SUMMARY": str(self.summary)}

    def run_check(self, *command, budget="5", name="fixture"):
        return subprocess.run(["bash", str(SCRIPT), name, budget, *command],
                              env=self.env, capture_output=True, text=True, timeout=10)

    def test_success_retains_both_streams_and_literal_arguments(self):
        argument = "spaces $(touch should-not-exist) ; literal"
        result = self.run_check(sys.executable, "-c",
                                "import sys; print(sys.argv[1]); print('stderr', file=sys.stderr)", argument)
        self.assertEqual(result.returncode, 0, result.stderr)
        log = (self.logs / "fixture.log").read_text()
        self.assertIn(argument, log)
        self.assertIn("stderr", log)
        self.assertIn("exit=0", (self.logs / "results.tsv").read_text())
        self.assertIn("exit **0**", self.summary.read_text())

    def test_failure_is_preserved_without_retry(self):
        count = self.directory / "attempts"
        result = self.run_check(sys.executable, "-c",
                                "import pathlib,sys; p=pathlib.Path(sys.argv[1]); "
                                "p.write_text(p.read_text()+'x' if p.exists() else 'x'); "
                                "print('assertion failed'); sys.exit(17)", str(count))
        self.assertEqual(result.returncode, 17, result.stderr)
        self.assertEqual(count.read_text(), "x")
        self.assertIn("assertion failed", (self.logs / "fixture.log").read_text())
        self.assertIn("::error::fixture failed", result.stdout)
        self.assertIn("exit=17", (self.logs / "results.tsv").read_text())

    def test_timeout_stops_child_process_and_retains_output(self):
        marker = self.directory / "child-finished"
        child = "import pathlib,sys,time; time.sleep(3); pathlib.Path(sys.argv[1]).touch()"
        parent = "import subprocess,sys,time; print('started',flush=True); " \
                 "subprocess.Popen([sys.executable,'-c',sys.argv[1],sys.argv[2]]); time.sleep(60)"
        result = self.run_check(sys.executable, "-c", parent, child, str(marker), budget="1")
        self.assertEqual(result.returncode, 124, result.stderr)
        self.assertIn("started", (self.logs / "fixture.log").read_text())
        self.assertIn("exit=124", (self.logs / "results.tsv").read_text())
        time.sleep(3)
        self.assertFalse(marker.exists(), "timed-out check left its child running")

    def test_invalid_input_does_not_execute_command(self):
        marker = self.directory / "executed"
        for name, budget in [("../escape", "5"), ("fixture", "0"), ("fixture", "forever")]:
            with self.subTest(name=name, budget=budget):
                result = self.run_check("touch", str(marker), name=name, budget=budget)
                self.assertEqual(result.returncode, 2)
                self.assertFalse(marker.exists())


class DatabaseSuiteCleanupTests(unittest.TestCase):
    def run_fixture(self, creation_fails=False):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            commands = root / "commands.jsonl"
            docker = root / "docker"
            docker.write_text("""#!/usr/bin/env python3
import json, os, sys
with open(os.environ['FIXTURE_COMMANDS'], 'a') as output:
    output.write(json.dumps(sys.argv[1:]) + '\\n')
if sys.argv[1] == 'create' and os.environ['FIXTURE_CREATE_FAILS'] == '1':
    sys.exit(19)
if sys.argv[1] == 'port':
    print('127.0.0.1:54321')
""")
            docker.chmod(0o755)
            cargo = root / "cargo"
            cargo.write_text("#!/bin/sh\nprintf 'fixture assertion failed\\n'\nexit 23\n")
            cargo.chmod(0o755)
            result = subprocess.run(
                ["bash", str(ROOT / ".github/scripts/test-database-regressions.sh")],
                cwd=ROOT, env={**os.environ, "PATH": str(root) + os.pathsep + os.environ["PATH"],
                               "RUNNER_TEMP": str(root), "GITHUB_STEP_SUMMARY": str(root / "summary.md"),
                               "FIXTURE_COMMANDS": str(commands),
                               "FIXTURE_CREATE_FAILS": "1" if creation_fails else "0"},
                capture_output=True, text=True, timeout=10,
            )
            calls = [json.loads(line) for line in commands.read_text().splitlines()]
            return result, calls, (root / "ci-tests/database-waits.log").exists()

    def test_failure_collects_waits_and_cleans_only_owned_fixture(self):
        result, calls, diagnostics = self.run_fixture()
        self.assertEqual(result.returncode, 23, result.stderr)
        create = next(call for call in calls if call[0] == 'create')
        name = create[create.index('--name') + 1]
        self.assertTrue(name.startswith('citadel-ci-postgres-'))
        self.assertEqual(calls[-1], ['rm', '-f', '-v', name])
        self.assertTrue(diagnostics)
        self.assertTrue(any('pg_blocking_pids' in argument for call in calls for argument in call))

    def test_creation_failure_does_not_remove_an_unowned_container(self):
        result, calls, diagnostics = self.run_fixture(creation_fails=True)
        self.assertEqual(result.returncode, 19, result.stderr)
        self.assertEqual([call[0] for call in calls], ['create'])
        self.assertFalse(diagnostics)


if __name__ == "__main__":
    unittest.main()
