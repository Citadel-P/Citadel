"""Render both deployment projects without starting containers or touching a VPS."""
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

from test_development import remote
from test_release import ROOT


@unittest.skipUnless(shutil.which("docker"), "Docker Compose is needed to render installation files")
class PublicDeploymentComposeTests(unittest.TestCase):
    def render(self, channel, project, port, extra=""):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory)
            (path / "docker-compose.yml").write_text((ROOT / "deploy/install/docker-compose.yml").read_text())
            (path / "demo.override.yml").write_text(remote.compose_override(channel))
            (path / ".env").write_text(
                f"COMPOSE_PROJECT_NAME={project}\nCITADEL_HTTP_PORT={port}\nCITADEL_EDGE_PORT={port + 1}\n"
                "PG_USER=fixture\nPG_PASSWORD=fixture-password\nPG_DATABASE=fixture\n" + extra)
            core = "ghcr.io/citadel-p/citadel@sha256:" + "a" * 64
            agent = "ghcr.io/citadel-p/citadel.agent@sha256:" + "b" * 64
            (path / ".release.env").write_text(f"CITADEL_IMAGE={core}\nCITADEL_EDGE_AGENT_IMAGE={agent}\n")
            result = subprocess.run(
                ["docker", "compose", "--env-file", ".env", "--env-file", ".release.env",
                 "-f", "docker-compose.yml", "-f", "demo.override.yml", "config", "--format", "json"],
                cwd=path, env={k: v for k, v in os.environ.items()
                               if not k.startswith(("CITADEL_", "COMPOSE_", "PG_", "Transport__"))},
                capture_output=True, text=True, check=True, timeout=30)
            value = json.loads(result.stdout)
            self.assertEqual(value["services"]["server"]["image"], core)
            self.assertEqual(value["services"]["server"]["environment"]["CITADEL_EDGE_AGENT_IMAGE"], agent)
            return value

    def test_targets_have_independent_volumes_ports_networks_and_resource_budgets(self):
        preview = self.render("dev", "citadel", 18000)
        stable = self.render("latest", "citadel-stable", 28000)
        self.assertNotEqual(preview["name"], stable["name"])
        for resource in ("volumes", "networks"):
            names = [{v["name"] for v in target[resource].values()} for target in (preview, stable)]
            self.assertTrue(names[0].isdisjoint(names[1]), resource)
        ports = []
        for target, budget in ((preview, (1024**3, 512 * 1024**2)), (stable, (2 * 1024**3, 1024**3))):
            bindings = target["services"]["server"]["ports"]
            self.assertTrue(all(p["host_ip"] == "127.0.0.1" for p in bindings))
            ports.append({p["published"] for p in bindings})
            for service, maximum in zip(("server", "pg_db"), budget):
                settings = target["services"][service]
                self.assertEqual(int(settings["mem_limit"]), maximum)
                self.assertEqual(int(settings["memswap_limit"]), maximum)
                self.assertGreater(float(settings["cpus"]), 0)
        self.assertTrue(ports[0].isdisjoint(ports[1]))
        self.assertLess(sum(int(preview["services"][s]["mem_limit"]) + int(stable["services"][s]["mem_limit"])
                            for s in ("server", "pg_db")), 5 * 1024**3)

    def test_operator_can_tune_limits_without_changing_release_identity(self):
        value = self.render("dev", "citadel", 18000,
                            "CITADEL_CORE_MEMORY_LIMIT=1536m\nCITADEL_CORE_CPUS=0.75\n")
        self.assertEqual(int(value["services"]["server"]["mem_limit"]), 1536 * 1024**2)
        self.assertEqual(float(value["services"]["server"]["cpus"]), 0.75)


if __name__ == "__main__":
    unittest.main()
