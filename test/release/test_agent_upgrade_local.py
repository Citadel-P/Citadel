"""Safety and result checks for the private Agent browser fixture."""
import importlib.util
import io
import json
from pathlib import Path
import subprocess
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location(
    "agent_upgrade", Path(__file__).resolve().parents[1] / "scripts/test-agent-upgrade-local.py")
runner = importlib.util.module_from_spec(spec)
spec.loader.exec_module(runner)


class AgentUpgradeTests(unittest.TestCase):
    def setUp(self):
        self.fixture = runner.AgentUpgrade("unused", Path("/unused"))

    def test_nested_agents_keep_their_sockets_independent_of_the_core_daemon(self):
        for rootless, core_socket in [
            (False, "/var/run/docker.sock"),
            (True, "/run/user/1000/docker.sock"),
        ]:
            for kind, agent_socket in [
                ("direct", "/var/run/docker.sock"),
                ("edge", "/run/user/1001/docker.sock"),
            ]:
                with self.subTest(rootless=rootless, kind=kind):
                    fixture = runner.AgentUpgrade("unused", Path("/unused"), rootless=rootless)
                    with patch.object(fixture, "execute", return_value=subprocess.CompletedProcess([], 0)) as execute:
                        fixture.nested(kind, "info", timeout=10)
                    execute.assert_called_once_with(
                        "docker", "--host", "unix://" + core_socket,
                        "exec", "fixture-" + kind + "-docker", "docker",
                        "--host", "unix://" + agent_socket, "info", check=False, timeout=10)

    def test_browser_leaves_approval_to_user(self):
        with patch.object(self.fixture, "setup_agents", return_value=("admin-password", "token", [])) as setup, \
             patch.object(self.fixture, "http") as http, \
             patch.object(runner.time, "sleep", side_effect=KeyboardInterrupt), \
             patch("sys.stdout", new_callable=io.StringIO) as output:
            with self.assertRaises(KeyboardInterrupt):
                self.fixture.serve_agents(18081)
            setup.assert_called_once_with(browser_port=18081)
            http.assert_not_called()
            self.assertIn("http://localhost:18081/updates", output.getvalue())
            self.assertIn("Review 2 Agent updates", output.getvalue())
            self.assertIn("admin-password", output.getvalue())

    def test_installer_keeps_the_explicit_edge_host_across_core_registration(self):
        with patch.object(self.fixture, "execute"), patch.object(self.fixture, "compose"), \
             patch.object(self.fixture, "http"), patch.object(self.fixture, "register"), \
             patch.object(self.fixture, "write") as write, \
             patch("sys.stdout", new_callable=io.StringIO):
            self.fixture.setup_installation(browser_port=18081)
            environments = [call.args[1] for call in write.call_args_list
                            if call.args[0] == self.fixture.installation + "/.env"]
            self.assertEqual(len(environments), 2)
            for env in environments:
                self.assertIn("CITADEL_HTTP_ALLOWED_HOSTS=localhost;127.0.0.1;fixture-core\n", env)
                self.assertIn("CITADEL_EDGE_PUBLIC_GRPC_URL=http://fixture-core:8001\n", env)
                self.assertIn("CITADEL_PUBLIC_URL=http://localhost:18081\n", env)

    def test_agent_publish_requires_agent_repository_digest(self):
        digest = "ghcr.io/citadel-p/citadel.agent@sha256:" + "a" * 64
        core = "ghcr.io/citadel-p/citadel@sha256:" + "b" * 64
        with patch.object(self.fixture, "docker") as docker, \
             patch.object(self.fixture, "json_command", return_value=[{"RepoDigests": [core, digest]}]):
            self.assertEqual(self.fixture.publish_private("image", "target",
                             "ghcr.io/citadel-p/citadel.agent"), digest)
            docker.assert_any_call("push", "ghcr.io/citadel-p/citadel.agent:target", timeout=300)
        with patch.object(self.fixture, "docker"), \
             patch.object(self.fixture, "json_command", return_value=[{"RepoDigests": [core]}]):
            with self.assertRaisesRegex(AssertionError, "Unexpected private digest"):
                self.fixture.publish_private("image", "target", "ghcr.io/citadel-p/citadel.agent")

    def test_mount_identity_compares_effective_permissions_and_storage(self):
        socket = dict(Type="bind", Source="/var/run/docker.sock", Destination="/var/run/docker.sock",
                      RW=True, Propagation="rprivate", Mode="")
        volume = dict(Type="volume", Source="/volumes/identity", Name="identity",
                      Destination="/app/data", RW=True, Propagation="", Mode="z")
        before = {"Mounts": [socket, volume]}
        after = {"Mounts": [dict(volume, Mode="z,rw"), dict(socket, Mode="rw,rprivate")]}
        self.assertEqual(runner.mount_identity(before), runner.mount_identity(after))
        after["Mounts"][0]["Mode"] = "Z,rw"
        self.assertNotEqual(runner.mount_identity(before), runner.mount_identity(after))
        after["Mounts"][0]["Mode"] = "z,rw"
        after["Mounts"][0]["RW"] = False
        self.assertNotEqual(runner.mount_identity(before), runner.mount_identity(after))
        after["Mounts"][0].update(RW=True, Name="different", Source="/volumes/different")
        self.assertNotEqual(runner.mount_identity(before), runner.mount_identity(after))

    def test_success_requires_replacement_digest_and_preserved_storage(self):
        before = {"Id": "old", "Mounts": [{"Name": "identity"}], "HostConfig": {"PortBindings": {}},
                  "State": {"Running": False}, "NetworkSettings": {"Ports": {}, "Networks": {
                      "edge_default": {"NetworkID": "network-id", "Aliases": ["agent"]}}}}
        self.fixture.agent_before = {kind: before for kind in ("direct", "edge")}
        self.fixture.agent_target = "target-digest"
        after = dict(json.loads(json.dumps(before)), Id="new", Config={"Image": "target-digest"})
        retained = {"helper": False, "source": False}
        targets = [{"id": kind, "name": f"fixture-{kind}-agent", "status": "Current"} for kind in ("direct", "edge")]
        self.fixture.core_before = ("core", "started", 0)
        core = {"Id": "core", "State": {"StartedAt": "started"}, "RestartCount": 0}
        def http(path, *args, **kwargs):
            return {
                "updates/agents/review": {"id": "review"},
                "updates/agents/approve": {"id": "operation"},
                "updates/agents/operations/operation": {
                    "finished": True, "targets": [{"phase": "Succeeded"}]},
                "updates/agents": {"targets": targets},
            }[path]
        edge_checks = 0
        def nested(kind, *args, **kwargs):
            nonlocal edge_checks
            result = after
            if args[0] == "ps":
                if "--filter" in args:
                    output = b"helper\n" if retained["helper"] else b""
                else:
                    output = b"new\nold\n" if retained["source"] else b"new\n"
                return subprocess.CompletedProcess([], 0, output, b"")
            if kind == "edge" and args == ("inspect", "citadel-agent"):
                edge_checks += 1
                if edge_checks % 2 == 1:
                    result = before
            return subprocess.CompletedProcess([], 0, json.dumps([result]).encode(), b"")
        with patch.object(self.fixture, "setup_agents", return_value=("password", "token", targets)), \
             patch.object(self.fixture, "json_command", return_value=[core]), \
             patch.object(self.fixture, "http", side_effect=http), \
             patch.object(self.fixture, "nested", side_effect=nested), \
             patch("sys.stdout", new_callable=io.StringIO):
            self.fixture.run_agents()
            edge_checks = 0
            after["Id"] = "old"
            with self.assertRaisesRegex(AssertionError, "not replaced"):
                self.fixture.run_agents()

            edge_checks = 0
            after["Id"] = "new"
            after["Mounts"] = [{"Name": "different"}]
            with self.assertRaisesRegex(AssertionError, "identity mounts changed"):
                self.fixture.run_agents()

            after["Mounts"] = before["Mounts"]
            for expected, networks in [
                ("network attachments changed", {}),
                ("network identity changed", {"edge_default": {"NetworkID": "other", "Aliases": ["agent"]}}),
                ("network aliases changed", {"edge_default": {"NetworkID": "network-id", "Aliases": []}}),
            ]:
                edge_checks = 0
                after["NetworkSettings"]["Networks"] = networks
                with self.assertRaisesRegex(AssertionError, expected):
                    self.fixture.run_agents()
            edge_checks = 0
            after["NetworkSettings"]["Networks"] = before["NetworkSettings"]["Networks"]
            # Successful updates must remove both containers; rootless helper socket
            # selection is exercised by the real fixture's isolated Edge daemon.
            retained["helper"] = True
            with self.assertRaisesRegex(AssertionError, "retained an updater helper"):
                self.fixture.run_agents()
            edge_checks = 0
            retained["helper"] = False
            retained["source"] = True
            with self.assertRaisesRegex(AssertionError, "retained the previous Agent"):
                self.fixture.run_agents()

    def test_port_checks_use_bound_endpoints_and_detect_changed_addresses_or_ports(self):
        bindings = [{"HostIp": "0.0.0.0", "HostPort": "9000"}, {"HostIp": "::", "HostPort": "9000"}]
        before = {"NetworkSettings": {"Ports": {"9000/tcp": bindings}}}
        after = {"NetworkSettings": {"Ports": {"9000/tcp": list(reversed(bindings))}}}
        self.assertEqual(runner.published_ports(before), runner.published_ports(after))
        after["NetworkSettings"]["Ports"]["9000/tcp"] = [{"HostIp": "127.0.0.1", "HostPort": "9000"}]
        self.assertNotEqual(runner.published_ports(before), runner.published_ports(after))
        after["NetworkSettings"]["Ports"]["9000/tcp"] = [{"HostIp": "0.0.0.0", "HostPort": "9001"}]
        self.assertNotEqual(runner.published_ports(before), runner.published_ports(after))


if __name__ == "__main__":
    unittest.main()
