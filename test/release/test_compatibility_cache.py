"""Compatibility compiler cache lifetime and executable routing, without Docker services."""
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]

DOCKER = r'''#!/usr/bin/env python3
import json, os, pathlib, sys
args = sys.argv[1:]
with open(os.environ["FIXTURE_DOCKER_LOG"], "a") as output:
    output.write(json.dumps(args) + "\n")
if args[0] == "run" and "--entrypoint" in args:
    entrypoint = args[args.index("--entrypoint") + 1]
    if entrypoint == "cargo":
        mount = next(a for a in args if a.endswith(",dst=/source/target"))
        target = pathlib.Path(mount.removeprefix("type=bind,src=").removesuffix(",dst=/source/target"))
        count = target / "build-count"
        count.write_text(str(int(count.read_text()) + 1 if count.exists() else 1))
        if os.environ.get("FIXTURE_BUILD_FAILURE"):
            sys.exit(42)
        for name in ["compatibility", "live_docker", "edge_intake"]:
            relative = pathlib.Path("debug/deps") / name
            (target / relative).parent.mkdir(parents=True, exist_ok=True)
            (target / relative).touch()
            print(json.dumps({"reason": "compiler-artifact", "target": {"name": name},
                              "executable": str(pathlib.Path("/source/target") / relative)}))
    elif entrypoint == "/app/compatibility-tests":
        mount = next(a for a in args if a.endswith(",dst=/app/compatibility-tests,readonly"))
        binary = pathlib.Path(mount.removeprefix("type=bind,src=").split(",dst=", 1)[0])
        assert binary.is_file(), binary
'''


class CompatibilityCacheTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        self.bin = self.root / "bin"
        self.bin.mkdir()
        docker = self.bin / "docker"
        docker.write_text(DOCKER)
        docker.chmod(0o755)
        self.scratch = self.root / "scratch"
        self.scratch.mkdir()
        self.log = self.root / "docker.jsonl"
        self.env = {**os.environ, "PATH": str(self.bin) + os.pathsep + os.environ["PATH"],
                    "TMPDIR": str(self.scratch), "FIXTURE_DOCKER_LOG": str(self.log)}
        self.env.pop("CITADEL_AGENT_COMPATIBILITY_CACHE_DIR", None)

    def run_fixture(self, cache=None, fail=False):
        env = self.env.copy()
        if cache is not None:
            env["CITADEL_AGENT_COMPATIBILITY_CACHE_DIR"] = str(cache)
        if fail:
            env["FIXTURE_BUILD_FAILURE"] = "1"
        result = subprocess.run(["bash", str(ROOT / "test/scripts/test-agent-compatibility.sh"),
                                 "fixture-agent:local"], env=env, capture_output=True, text=True)
        self.assertEqual(result.returncode, 42 if fail else 0, result.stdout + result.stderr)
        self.assertEqual(list(self.scratch.iterdir()), [], "temporary fixtures must be removed")
        return [json.loads(line) for line in self.log.read_text().splitlines()]

    def test_explicit_cache_survives_repeated_runs_and_supplies_all_executables(self):
        cache = self.root / "retained target"
        self.run_fixture(cache)
        commands = self.run_fixture(cache)
        self.assertEqual((cache / "build-count").read_text(), "2")
        runners = [args for args in commands if "/app/compatibility-tests" in args]
        self.assertEqual(len(runners), 6)
        for args in runners:
            self.assertTrue(any(arg.startswith(f"type=bind,src={cache}/debug/deps/") for arg in args))
        builds = [args for args in commands if args[0] == "build"]
        self.assertEqual(len(builds), 2)
        for args in builds:
            self.assertEqual(args[args.index("--target") + 1], "rust-source")
            self.assertEqual(args[args.index("--file") + 1], "Dockerfile.agent")

    def test_default_compiler_output_is_removed(self):
        commands = self.run_fixture()
        builder = next(args for args in commands if "cargo" in args)
        mount = next(arg for arg in builder if arg.endswith(",dst=/source/target"))
        target = Path(mount.removeprefix("type=bind,src=").removesuffix(",dst=/source/target"))
        self.assertFalse(target.exists())

    def test_failure_preserves_only_the_explicit_cache(self):
        cache = self.root / "retained target"
        commands = self.run_fixture(cache, fail=True)
        self.assertEqual((cache / "build-count").read_text(), "1")
        self.assertFalse(any("/app/compatibility-tests" in args for args in commands))


if __name__ == "__main__":
    unittest.main()
