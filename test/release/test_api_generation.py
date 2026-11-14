"""Development startup generates contracts before use, without overlapping writers."""
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[2]


class ApiGenerationTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory(prefix="citadel api generation ")
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        self.script = self.root / "src/tools/build/generate-api.sh"
        self.script.parent.mkdir(parents=True)
        shutil.copyfile(ROOT / "src/tools/build/generate-api.sh", self.script)
        self.bin = self.root / "bin"
        self.bin.mkdir()
        cargo = self.bin / "cargo"
        cargo.write_text("""#!/usr/bin/env python3
import json, os, pathlib, sys, time
root = pathlib.Path.cwd()
with (root / 'calls.jsonl').open('a') as output:
    output.write(json.dumps({'args': sys.argv[1:], 'offline': os.environ['SQLX_OFFLINE']}) + '\\n')
# An exclusive marker catches concurrent generation even when both callers exit 0.
with (root / 'writer').open('x'):
    time.sleep(0.15)
(root / 'writer').unlink()
sys.exit(int(os.environ.get('GENERATOR_STATUS', '0')))
""")
        cargo.chmod(0o755)
        self.env = dict(os.environ, PATH=f"{self.bin}:{os.environ['PATH']}")

    def test_concurrent_startups_serialize_and_use_workspace_contract_generator(self):
        processes = [subprocess.Popen(
            ["bash", str(self.script)], cwd=self.root.parent, env=self.env,
            stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True,
        ) for _ in range(2)]
        try:
            for process in processes:
                stdout, stderr = process.communicate(timeout=10)
                self.assertEqual(process.returncode, 0, stdout + stderr)
        finally:
            for process in processes:
                if process.poll() is None:
                    process.kill()
                process.wait()
        calls = [json.loads(line) for line in (self.root / "calls.jsonl").read_text().splitlines()]
        self.assertEqual(len(calls), 2)
        for call in calls:
            self.assertEqual(call, {
                "args": ["run", "--locked", "-p", "xtask", "--", "openapi"],
                "offline": "true",
            })

    def test_generator_failure_propagates_and_releases_lock(self):
        failed = subprocess.run(["bash", str(self.script)], env=dict(self.env, GENERATOR_STATUS="23"), timeout=10)
        self.assertEqual(failed.returncode, 23)
        retry = subprocess.run(["bash", str(self.script)], env=self.env, timeout=10)
        self.assertEqual(retry.returncode, 0)

    @unittest.skipUnless(shutil.which("npm"), "npm is required for the startup hook test")
    def test_frontend_does_not_start_when_generation_fails(self):
        frontend = self.root / "src/frontend"
        frontend.mkdir()
        scripts = json.loads((ROOT / "src/frontend/package.json").read_text())["scripts"]
        (frontend / "package.json").write_text(json.dumps({"scripts": {
            "predev": scripts["predev"],
            "dev": "touch frontend-started",
        }}))
        result = subprocess.run(
            ["npm", "run", "dev"], cwd=frontend,
            env=dict(self.env, GENERATOR_STATUS="23", npm_config_ignore_scripts="false"),
            capture_output=True, text=True, timeout=15,
        )
        self.assertNotEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertTrue((self.root / "calls.jsonl").exists())
        self.assertFalse((frontend / "frontend-started").exists())
        result = subprocess.run(
            ["npm", "run", "dev"], cwd=frontend,
            env=dict(self.env, npm_config_ignore_scripts="false"),
            capture_output=True, text=True, timeout=15,
        )
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertTrue((frontend / "frontend-started").exists())
