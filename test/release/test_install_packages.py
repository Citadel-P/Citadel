"""Exercise CI package installation without sudo or modifying the host packages."""
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]

COMMAND = r'''#!/usr/bin/env python3
import json, os, pathlib, sys, time
name = pathlib.Path(sys.argv[0]).name
args = sys.argv[1:]
with open(os.environ["FIXTURE_LOG"], "a") as output:
    output.write(json.dumps([name, *args]) + "\n")
if name == "dpkg-query":
    status = json.loads(os.environ["FIXTURE_PACKAGES"]).get(args[-1])
    if status is None:
        sys.exit(1)
    print(status, end="")
elif name == "sudo":
    assert args[0] == "-n", args
    os.execvp(args[1], args[1:])
elif name == "timeout":
    assert args[:2] == ["--kill-after=10s", "180s"], args
    # Exercise the real process timeout, with a short budget for the fixture.
    os.execv(os.environ["FIXTURE_TIMEOUT"], ["timeout", "--kill-after=1s", "0.5s", *args[2:]])
elif name == "apt-get":
    assert os.environ["DEBIAN_FRONTEND"] == "noninteractive"
    for option in ["Acquire::Retries=2", "Acquire::http::Timeout=20",
                   "Acquire::https::Timeout=20", "DPkg::Lock::Timeout=30"]:
        assert option in args, args
    operation = "update" if "update" in args else "install"
    if operation == "update":
        assert "APT::Update::Error-Mode=any" in args, args
    if os.environ.get("FIXTURE_HANG") == operation:
        time.sleep(60)
    sys.exit(int(os.environ.get("FIXTURE_" + operation.upper() + "_EXIT", "0")))
'''


class InstallPackagesTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        self.bin = self.root / "bin"
        self.bin.mkdir()
        for command in ["dpkg-query", "sudo", "timeout", "apt-get"]:
            path = self.bin / command
            path.write_text(COMMAND)
            path.chmod(0o755)
        self.log = self.root / "commands.jsonl"
        self.env = {
            **os.environ,
            "PATH": str(self.bin) + os.pathsep + os.environ["PATH"],
            "FIXTURE_LOG": str(self.log),
            "FIXTURE_TIMEOUT": shutil.which("timeout"),
        }

    def run_fixture(self, packages, installed=None, **env):
        result = subprocess.run(
            ["bash", str(ROOT / ".github/scripts/install-packages.sh"), *packages],
            env={**self.env, "FIXTURE_PACKAGES": json.dumps(installed or {}), **env},
            capture_output=True, text=True, timeout=10,
        )
        commands = [json.loads(line) for line in self.log.read_text().splitlines()] if self.log.exists() else []
        return result, commands

    def test_installed_packages_skip_all_network_and_privileged_commands(self):
        result, commands = self.run_fixture(
            ["pkg-config", "libssl-dev"],
            {"pkg-config": "install ok installed", "libssl-dev": "install ok installed"},
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("skipping APT", result.stdout)
        self.assertTrue(all(command[0] == "dpkg-query" for command in commands))

    def test_only_missing_and_incompletely_installed_packages_are_installed(self):
        result, commands = self.run_fixture(
            ["pkg-config", "libssl-dev", "skopeo"],
            {"pkg-config": "install ok installed", "libssl-dev": "deinstall ok config-files"},
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        apt = [command for command in commands if command[0] == "apt-get"]
        self.assertEqual(len(apt), 2)
        self.assertIn("update", apt[0])
        self.assertEqual(apt[1][apt[1].index("install"):],
                         ["install", "-y", "--no-install-recommends", "libssl-dev", "skopeo"])

    def test_update_failure_stops_before_install(self):
        result, commands = self.run_fixture(["skopeo"], FIXTURE_UPDATE_EXIT="100")
        self.assertEqual(result.returncode, 100)
        self.assertIn("APT update failed", result.stderr)
        self.assertFalse(any("install" in command for command in commands))

    def test_install_failure_is_propagated(self):
        result, _ = self.run_fixture(["skopeo"], FIXTURE_INSTALL_EXIT="100")
        self.assertEqual(result.returncode, 100)
        self.assertIn("APT install failed", result.stderr)

    def test_stalled_mirror_is_terminated_and_install_is_not_attempted(self):
        result, commands = self.run_fixture(["skopeo"], FIXTURE_HANG="update")
        self.assertEqual(result.returncode, 124, result.stderr)
        self.assertIn("APT update failed", result.stderr)
        self.assertFalse(any("install" in command for command in commands))

    def test_empty_package_list_is_rejected(self):
        result, commands = self.run_fixture([])
        self.assertEqual(result.returncode, 2)
        self.assertEqual(commands, [])


if __name__ == "__main__":
    unittest.main()
