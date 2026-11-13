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
    def render(self, channel, project, port, extra="", rootless=None):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory)
            (path / "docker-compose.yml").write_text(remote.deployment_compose((ROOT / "deploy/install/docker-compose.yml").read_text()))
            (path / "demo.override.yml").write_text(remote.compose_override(channel, rootless))
            (path / ".env").write_text(
                f"COMPOSE_PROJECT_NAME={project}\nCITADEL_HTTP_PORT={port}\nCITADEL_EDGE_PORT={port + 1}\n"
                "PG_USER=fixture\nPG_PASSWORD=fixture-password\nPG_DATABASE=fixture\n" + extra)
            core = "ghcr.io/citadel-p/citadel@sha256:" + "a" * 64
            agent = "ghcr.io/citadel-p/citadel.agent@sha256:" + "b" * 64
            with (path / ".env").open("a") as environment:
                environment.write(f"CITADEL_IMAGE={core}\nCITADEL_EDGE_AGENT_IMAGE={agent}\n")
            result = subprocess.run(
                ["docker", "compose", "--env-file", ".env",
                 "-f", "docker-compose.yml", "-f", "demo.override.yml", "config", "--format", "json"],
                cwd=path, env={k: v for k, v in os.environ.items()
                               if not k.startswith(("CITADEL_", "COMPOSE_", "PG_"))},
                capture_output=True, text=True, check=True, timeout=30)
            value = json.loads(result.stdout)
            self.assertEqual(value["services"]["server"]["image"], core)
            self.assertEqual(value["services"]["server"]["environment"]["CITADEL_EDGE_AGENT_IMAGE"], agent)
            return value

    def test_ci_keeps_explicit_registration_without_automatic_state_mount(self):
        config = self.render("latest", "citadel_stable", 18000,
                             extra="CITADEL_UPDATE_STATE_DIRECTORY=/private/existing-updates\n")
        core = config["services"]["server"]
        self.assertEqual(core["environment"]["CITADEL_UPDATE_STATE_DIRECTORY"], "/private/existing-updates")
        self.assertFalse(any(mount["target"] == "/var/lib/citadel-updates" for mount in core["volumes"]))

    def test_targets_have_independent_resources_without_memory_caps(self):
        preview = self.render("dev", "citadel", 18000)
        stable = self.render("latest", "citadel-stable", 28000)
        self.assertNotEqual(preview["name"], stable["name"])
        for resource in ("volumes", "networks"):
            names = [{v["name"] for v in target[resource].values()} for target in (preview, stable)]
            self.assertTrue(names[0].isdisjoint(names[1]), resource)
        ports = []
        for target, cpus in ((preview, (1.0, 0.5)), (stable, (2.0, 1.0))):
            bindings = target["services"]["server"]["ports"]
            self.assertEqual({p["target"] for p in bindings}, {8000, 8001})
            self.assertTrue(all(p["host_ip"] == "127.0.0.1" for p in bindings))
            ports.append({p["published"] for p in bindings})
            for service, maximum in zip(("server", "pg_db"), cpus):
                settings = target["services"][service]
                self.assertEqual(int(settings.get("mem_limit", 0)), 0)
                self.assertEqual(int(settings.get("memswap_limit", 0)), 0)
                self.assertEqual(float(settings["cpus"]), maximum)
        self.assertTrue(ports[0].isdisjoint(ports[1]))

    def test_operator_can_tune_limits_without_changing_release_identity(self):
        value = self.render("dev", "citadel", 18000,
                            "CITADEL_CORE_MEMORY_LIMIT=1536m\nCITADEL_CORE_CPUS=0.75\n")
        self.assertEqual(int(value["services"]["server"]["mem_limit"]), 1536 * 1024**2)
        self.assertEqual(float(value["services"]["server"]["cpus"]), 0.75)

    def test_rootless_targets_never_inherit_host_socket_or_host_root(self):
        for channel, user, uid in (("dev", "citadel-preview", 1101), ("latest", "citadel-demo", 1102)):
            with self.subTest(channel=channel):
                value = self.render(channel, user, 18000, rootless={"user": user, "uid": uid})
                mounts = value["services"]["server"]["volumes"]
                self.assertEqual(len(mounts), 3)
                sources = {m["target"]: m["source"] for m in mounts}
                self.assertEqual(sources["/var/run/docker.sock"], f"/run/user/{uid}/docker.sock")
                self.assertNotIn("/host", sources)
                self.assertNotIn("/", sources.values())
                data = f"/home/{user}/.local/share/docker"
                self.assertEqual(sources["/host" + data], data)
                self.assertTrue(next(m for m in mounts if m["source"] == data)["read_only"])

    def test_rootless_recreation_preserves_trusted_proxy_gateway(self):
        network = {"subnet": "172.19.0.0/16", "gateway": "172.19.0.1"}
        value = self.render("latest", "citadel-stable", 28000,
                            rootless={"user": "citadel-demo", "uid": 1102, "network": network})
        self.assertEqual(value["networks"]["default"]["ipam"]["config"], [network])



@unittest.skipUnless(shutil.which("docker"), "Docker Compose is needed to render configuration")
class CanonicalConfigurationComposeTests(unittest.TestCase):
    def render(self, root, files, overrides=None):
        result = subprocess.run(
            ["docker", "compose", "--env-file", str(root / "fixture.env"),
             *[argument for file in files for argument in ("-f", str(root / file))],
             "config", "--format", "json"],
            cwd=root, env={**{k: v for k, v in os.environ.items()
                          if not k.startswith(("CITADEL_", "COMPOSE_", "PG_"))}, **(overrides or {})},
            capture_output=True, text=True, check=True, timeout=30)
        return json.loads(result.stdout)

    def configuration(self, root):
        value = ("PG_USER=fixture\nPG_PASSWORD=literal$not-expanded\nPG_DATABASE=fixture\n"
                 "CITADEL_TRANSPORT_MODE=Disabled\nCITADEL_HTTP_PORT=18443\nCITADEL_EDGE_PORT=18444\n"
                 "CITADEL_HTTP_LISTEN_PORT=8443\nCITADEL_EDGE_GRPC_LISTEN_PORT=8444\n"
                 "CITADEL_STATIC_ROOT=/custom/frontend\nCITADEL_TLS_CERTIFICATE_PATH=/etc/citadel/tls/core.pem\n"
                 "CITADEL_TLS_PRIVATE_KEY_PATH=/etc/citadel/tls/key.pem\n")
        (root / "fixture.env").write_text(value)
        return value

    def test_both_packages_and_direct_tls_keep_ports_and_raw_values_consistent(self):
        for package in ("deploy", "deploy/install"):
            with self.subTest(package=package), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                (root / "docker-compose.yml").write_text((ROOT / package / "docker-compose.yml").read_text())
                (root / "direct.yml").write_text((ROOT / "deploy/compose.direct-tls.yml").read_text())
                (root / ".env").write_text(self.configuration(root))
                files = ["docker-compose.yml"]
                if package == "deploy/install":
                    (root / "edge.yml").write_text((ROOT / "deploy/install/compose.edge.yml").read_text())
                    files.append("edge.yml")
                for overlay in ([], ["direct.yml"]):
                    server = self.render(root, [*files, *overlay])["services"]["server"]
                    ports = {int(p["published"]): p["target"] for p in server["ports"]}
                    self.assertEqual(ports, {18443: 8443, 18444: 8444})
                    env = server["environment"]
                    self.assertEqual(env["CITADEL_HTTP_LISTEN_PORT"], "8443")
                    self.assertEqual(env["CITADEL_EDGE_GRPC_LISTEN_PORT"], "8444")
                    self.assertEqual(env["CITADEL_STATIC_ROOT"], "/custom/frontend")
                    # Compose escapes literal dollars when serializing a reusable config.
                    self.assertEqual(env["PG_PASSWORD"], "literal$$not-expanded")
                    self.assertEqual(env["CITADEL_TRANSPORT_MODE"], "Direct" if overlay else "Disabled")
                    self.assertEqual(server["healthcheck"]["test"][-1], "healthcheck")
                server = self.render(root, files, {"CITADEL_HTTP_LISTEN_PORT": "9443", "CITADEL_EDGE_GRPC_LISTEN_PORT": "9444"})["services"]["server"]
                self.assertEqual({p["target"] for p in server["ports"]}, {9443, 9444})
                self.assertEqual(server["environment"]["CITADEL_HTTP_LISTEN_PORT"], "9443")
                self.assertEqual(server["environment"]["CITADEL_EDGE_GRPC_LISTEN_PORT"], "9444")

    def test_installer_needs_only_password_and_publishes_only_web_port(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for file in ("docker-compose.yml", "compose.edge.yml"):
                (root / file).write_text((ROOT / "deploy/install" / file).read_text())
            template = (ROOT / "deploy/install/.env.example").read_text()
            for extra, port in (("", 18000), ("CITADEL_HTTP_PORT=19000\n", 19000)):
                with self.subTest(port=port):
                    env = template.replace("PG_PASSWORD=", "PG_PASSWORD=fixture-password") + extra
                    (root / ".env").write_text(env)
                    (root / "fixture.env").write_text(env)
                    config = self.render(root, ["docker-compose.yml"])
                    server = config["services"]["server"]
                    self.assertEqual([(p["host_ip"], p["published"], p["target"]) for p in server["ports"]],
                                     [("127.0.0.1", str(port), 8000)])
                    settings = server["environment"]
                    self.assertEqual(settings["CITADEL_UPDATE_STATE_DIRECTORY"], "/var/lib/citadel-updates")
                    state_mount = [v for v in server["volumes"] if v["target"] == "/var/lib/citadel-updates"]
                    self.assertEqual(len(state_mount), 1)
                    self.assertEqual(state_mount[0]["source"], str(root / ".citadel-updates"))
                    self.assertTrue(state_mount[0]["read_only"])
                    self.assertNotIn("CITADEL_UPDATE_STATE_DIRECTORY", template)
                    self.assertEqual(settings["CITADEL_PUBLIC_URL"], f"http://localhost:{port}")
                    self.assertEqual(settings["CITADEL_TRANSPORT_MODE"], "Disabled")
                    self.assertEqual(settings["CITADEL_HTTP_ALLOWED_HOSTS"], "localhost;127.0.0.1")
                    database = config["services"]["pg_db"]["environment"]
                    for core, pg, expected in (("PG_USER", "POSTGRES_USER", "citadel"),
                                              ("PG_DATABASE", "POSTGRES_DB", "citadel"),
                                              ("PG_PASSWORD", "POSTGRES_PASSWORD", "fixture-password")):
                        self.assertEqual(settings[core], expected)
                        self.assertEqual(database[pg], expected)
                    self.assertEqual(config["name"], "citadel")
                    self.assertEqual(config["volumes"]["citadel_data"]["name"], "citadel_citadel_data")
                    self.assertEqual(config["volumes"]["postgres_data"]["name"], "citadel_postgres_data")
            overrides = {"CITADEL_PUBLIC_URL": "https://citadel.example.com",
                         "CITADEL_EDGE_PUBLIC_GRPC_URL": "https://edge.example.com",
                         "CITADEL_HTTP_ALLOWED_HOSTS": "citadel.example.com;edge.example.com",
                         "CITADEL_TRANSPORT_MODE": "ReverseProxy",
                         "PG_USER": "custom-user", "PG_DATABASE": "custom-db"}
            config = self.render(root, ["docker-compose.yml", "compose.edge.yml"], overrides)
            for key, value in overrides.items():
                self.assertEqual(config["services"]["server"]["environment"][key], value)
            self.assertEqual({p["target"] for p in config["services"]["server"]["ports"]}, {8000, 8001})
            self.assertEqual(config["services"]["pg_db"]["environment"]["POSTGRES_USER"], "custom-user")
            self.assertEqual(config["services"]["pg_db"]["environment"]["POSTGRES_DB"], "custom-db")
            (root / ".env").write_text(template)
            (root / "fixture.env").write_text(template)
            with self.assertRaises(subprocess.CalledProcessError) as failure:
                self.render(root, ["docker-compose.yml"])
            self.assertIn("Set PG_PASSWORD in .env", failure.exception.stderr)

    def test_development_mount_source_is_separate_from_container_socket(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for file in (".devcontainer/compose.yaml", ".devcontainer/compose.wsl.yaml", "deploy/compose.development.yml"):
                (root / file).parent.mkdir(parents=True, exist_ok=True)
                (root / file).write_text((ROOT / file).read_text())
            env = self.configuration(root) + (
                "CITADEL_DEV_UID=1000\nCITADEL_DEV_GID=1000\nCITADEL_DEV_DOCKER_GID=999\n"
                "CITADEL_DEV_DOCKER_SOCKET=/tmp/custom host/docker.sock\n"
                "CITADEL_DOCKER_SOCKET=/tmp/native.sock\nCITADEL_DATA_ROOT=/tmp/citadel-data\n"
                "CITADEL_DEV_BINARY=/tmp/citadel-data/core\n")
            (root / "fixture.env").write_text(env)
            (root / "deploy/.env.development").write_text(env)
            server = self.render(root, [".devcontainer/compose.yaml", ".devcontainer/compose.wsl.yaml", "deploy/compose.development.yml"])["services"]["core"]
            self.assertEqual(server["environment"]["CITADEL_DOCKER_SOCKET"], "/var/run/docker.sock")
            self.assertEqual(server["environment"]["DOCKER_HOST"], "unix:///var/run/docker.sock")
            socket = next(v for v in server["volumes"] if v["target"] == "/var/run/docker.sock")
            self.assertEqual(socket["source"], "/tmp/custom host/docker.sock")
            self.assertEqual({int(p["published"]): p["target"] for p in server["ports"]}, {18443: 8443, 18444: 8444})
            self.assertEqual(server["environment"]["CITADEL_STATIC_ROOT"], "/custom/frontend")

if __name__ == "__main__":
    unittest.main()
