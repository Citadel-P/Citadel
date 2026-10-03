#!/usr/bin/env python3
"""Capture the CPU refactor measurement on disposable PostgreSQL and Docker-in-Docker.

Use an isolated fixture with pinned image versions. Only the named nested
daemon receives workload mutations. Uses stdlib only; never records API tokens.
"""
import argparse
import asyncio
import collections
import hashlib
import json
import math
import os
from pathlib import Path
import platform
import re
import signal
import socket
import tempfile
import time
import urllib.request
import uuid

ROOT = Path(__file__).resolve().parents[2]
POSTGRES = "citadel-cpu-measurement-postgres"
ENGINE = "citadel-cpu-measurement-docker"
IMAGE = "alpine@sha256:85fe1e81d6758c208f3e1eed4338a1997e19d4be002d4dd32d3100c9a8c010a0"


async def command(*args):
    process = await asyncio.create_subprocess_exec(
        *args, stdout=asyncio.subprocess.PIPE, stderr=asyncio.subprocess.PIPE
    )
    out, err = await process.communicate()
    if process.returncode:
        raise RuntimeError(f"{args[0]} failed: {err.decode()}")
    return out.decode().strip()


def metrics(text):
    result = {}
    for line in text.splitlines():
        if line and not line.startswith("#"):
            key, value = line.rsplit(" ", 1)
            result[key] = float(value)
    return result


def process_ticks(pid):
    # comm can contain spaces; the fields after its final ')' begin with state.
    fields = Path(f"/proc/{pid}/stat").read_text().rsplit(")", 1)[1].split()
    return int(fields[11]) + int(fields[12])


def process_residency(pid):
    status = Path(f"/proc/{pid}/status").read_text()
    memory = Path(f"/proc/{pid}/smaps_rollup").read_text()
    values = {}
    for key in ("Rss", "Pss"):
        match = re.search(rf"^{key}:\s+(\d+)", memory, re.MULTILINE)
        values[key.lower() + "_bytes"] = int(match[1]) * 1024
    values["threads"] = int(re.search(r"^Threads:\s+(\d+)", status, re.MULTILINE)[1])
    return values


def network_bytes(text):
    rx, tx = 0, 0
    for line in text.splitlines():
        if ":" not in line:
            continue
        interface, counters = line.split(":", 1)
        # Only the external fixture interface; summing bridge/veth pairs double-counts.
        if interface.strip() != "eth0":
            continue
        fields = counters.split()
        rx += int(fields[0])
        tx += int(fields[8])
    return {"rx": rx, "tx": tx}


async def capture(args):
    # Refuse an occupied API port before any setup request could reach another Core.
    with socket.socket() as probe:
        probe.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        probe.bind(("127.0.0.1", 58049))
    existing = await command("docker", "exec", ENGINE, "docker", "ps", "-aq")
    if existing:
        raise RuntimeError("the dedicated nested daemon must have no containers before capture")
    args.output.mkdir(parents=True, exist_ok=False)
    binary = args.binary.resolve()
    database = "cpu_" + uuid.uuid4().hex
    await command("docker", "exec", POSTGRES, "psql", "-U", "citadel_cpu",
                  "-d", "citadel_cpu", "-v", "ON_ERROR_STOP=1", "-c",
                  f"CREATE DATABASE {database}")

    async def sql(query):
        return await command("docker", "exec", POSTGRES, "psql", "-XAt", "-U",
                             "citadel_cpu", "-d", database, "-v", "ON_ERROR_STOP=1",
                             "-c", query)

    await sql("CREATE EXTENSION pg_stat_statements")
    # Fail before starting Core if either pinned fixture is unavailable.
    await command("docker", "exec", ENGINE, "docker", "image", "inspect", IMAGE)
    metadata = {
        "commit": await command("git", "-C", str(ROOT), "rev-parse", "HEAD"),
        "dirty": bool(await command("git", "-C", str(ROOT), "status", "--porcelain")),
        "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
        "kernel": platform.platform(), "logical_cpus": os.cpu_count(),
        "cpu_model": next((line.split(":", 1)[1].strip() for line in
                           Path("/proc/cpuinfo").read_text().splitlines()
                           if line.startswith("model name")), "unknown"),
        "database": database, "monitoring_seconds": 10,
        "idle_seconds": args.idle_seconds, "stats_seconds": args.stats_seconds,
        "realtime_subscribers": 0, "workload_image": IMAGE,
        "postgres_image": await command("docker", "inspect", "--format", "{{.Image}}", POSTGRES),
        "docker_image": await command("docker", "inspect", "--format", "{{.Image}}", ENGINE),
        "capture_version": 2,
        "started_utc": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
    }
    (args.output / "environment.json").write_text(json.dumps(metadata, indent=2) + "\n")
    requests = []
    active_proxy_tasks = set()
    workload_names = []
    token = None
    base = "http://127.0.0.1:58049"

    async def api(path, body=None, method=None):
        def request():
            headers = {"Content-Type": "application/json"}
            if token:
                headers["Authorization"] = "Bearer " + token
            req = urllib.request.Request(base + path, method=method, headers=headers,
                                         data=None if body is None else json.dumps(body).encode())
            with urllib.request.urlopen(req, timeout=45) as response:
                payload = response.read().decode()
                if not payload:
                    return None
                if response.headers.get_content_type() == "application/json":
                    return json.loads(payload)
                return payload
        return await asyncio.to_thread(request)

    async def proxy(reader, writer):
        task = asyncio.current_task()
        active_proxy_tasks.add(task)
        remote_writer = None
        jobs = []
        try:
            remote, remote_writer = await asyncio.open_connection("127.0.0.1", 52389)

            async def inbound():
                while True:
                    header = await reader.readuntil(b"\r\n\r\n")
                    # Deliberately omit headers and payloads (may contain secrets).
                    first = header.split(b"\r\n", 1)[0].decode("ascii")
                    requests.append({"time": time.time(), "request": first})
                    remote_writer.write(header)
                    match = re.search(rb"(?im)^content-length:\s*(\d+)", header)
                    if match:
                        remote_writer.write(await reader.readexactly(int(match[1])))
                    await remote_writer.drain()

            async def outbound():
                while chunk := await remote.read(65536):
                    writer.write(chunk)
                    await writer.drain()

            jobs = [asyncio.create_task(inbound()), asyncio.create_task(outbound())]
            await asyncio.wait(jobs, return_when=asyncio.FIRST_COMPLETED)
        except (ConnectionError, asyncio.IncompleteReadError):
            pass
        finally:
            for job in jobs:
                job.cancel()
            await asyncio.gather(*jobs, return_exceptions=True)
            writer.close()
            if remote_writer:
                remote_writer.close()
            active_proxy_tasks.discard(task)

    statement_query = """
SELECT COALESCE(json_agg(s), '[]') FROM (
 SELECT queryid::text, calls, rows, total_exec_time, query
 FROM pg_stat_statements
 WHERE dbid=(SELECT oid FROM pg_database WHERE datname=current_database())
 AND query NOT LIKE '%pg_stat_statements%'
) s
"""

    async def snapshot():
        runtime, statements, postgres_net, docker_net = await asyncio.gather(
            api("/metrics"), sql(statement_query),
            command("docker", "exec", POSTGRES, "cat", "/proc/net/dev"),
            command("docker", "exec", ENGINE, "cat", "/proc/net/dev"),
        )
        return {"metrics": metrics(runtime), "statements": json.loads(statements),
                "network": {"postgres": network_bytes(postgres_net),
                            "docker": network_bytes(docker_net)}}

    with tempfile.TemporaryDirectory(prefix="citadel-cpu-") as temp:
        sock = Path(temp) / "docker.sock"
        proxy_server = await asyncio.start_unix_server(proxy, path=sock)
        env = {"PATH": os.environ["PATH"], "LANG": "C.UTF-8",
               "DATABASE_URL": f"postgres://citadel_cpu:citadel_cpu_fixture@127.0.0.1:55449/{database}",
               "Transport__Mode": "Disabled", "Transport__ApiPort": "58049",
               "Jwt__Key": "cpu-measurement-disposable-signing-key-32bytes",
               "Secrets__EncryptionKey": "AQIDBAUGBwgJCgsMDQ4PEBESExQVFhcYGRobHB0eHyA=",
               "CITADEL_DATA_ROOT": str(Path(temp) / "state"),
               "CITADEL_RUST_DOCKER_SOCKET": str(sock),
               "CITADEL_RUST_DB_MAX_CONNECTIONS": "5",
               "CITADEL_RUST_EVENT_QUEUE_CAPACITY": "256",
               "JobConfiguration__MonitoringInterval": "10",
               "RUST_LOG": "citadel_server=info,citadel_adapters=info"}
        with (args.output / "server.log").open("w") as log:
            proc = await asyncio.create_subprocess_exec(str(binary), "serve", env=env,
                                                        cwd=temp, stdout=log, stderr=log)
            samples = []

            async def sample_cpu():
                tick_hz = os.sysconf("SC_CLK_TCK")
                previous, at = process_ticks(proc.pid), time.monotonic()
                while True:
                    await asyncio.sleep(1)
                    current, now = process_ticks(proc.pid), time.monotonic()
                    samples.append({"time": time.time(), "cpu_percent":
                                    (current - previous) / tick_hz / (now - at) * 100,
                                    **process_residency(proc.pid)})
                    previous, at = current, now

            sampler = asyncio.create_task(sample_cpu())
            results = []

            async def window(name, action, seconds):
                before = await snapshot()
                started, ticks, offset = time.time(), process_ticks(proc.pid), len(requests)
                action_started = time.monotonic()
                await action()
                action_seconds = time.monotonic() - action_started
                await asyncio.sleep(max(0, seconds - action_seconds))
                finished, end_ticks = time.time(), process_ticks(proc.pid)
                end_offset = len(requests)
                after = await snapshot()
                inside = [s for s in samples if started + 1 <= s["time"] <= finished]
                values = sorted(s["cpu_percent"] for s in inside)
                old = {s["queryid"]: s for s in before["statements"]}
                statements = []
                for row in after["statements"]:
                    prior = old.get(row["queryid"], {})
                    delta = {k: row[k] - prior.get(k, 0) for k in ("calls", "rows", "total_exec_time")}
                    if delta["calls"]:
                        statements.append({**delta, "query": row["query"]})
                paths = collections.Counter()
                for row in requests[offset:end_offset]:
                    method, path, _ = row["request"].split(" ", 2)
                    path = re.sub(r"^/v[\d.]+", "", path.split("?", 1)[0])
                    path = re.sub(r"/containers/[^/]+/(json|stats|stop)$", r"/containers/{id}/\1", path)
                    paths[f"{method} {path}"] += 1
                duration = finished - started
                result = {"scenario": name, "seconds": duration, "action_seconds": action_seconds,
                          "cpu_average": (end_ticks - ticks) / os.sysconf("SC_CLK_TCK") / duration * 100,
                          "cpu_p95": values[max(0, math.ceil(len(values) * .95) - 1)] if values else None,
                          "cpu_max": max(values, default=None), "docker_requests": dict(paths),
                          "db_calls": sum(s["calls"] for s in statements), "statements": statements,
                          "metrics_delta": {k: v - before["metrics"].get(k, 0) for k, v in after["metrics"].items()
                                            if k.endswith("_total") or "_total{" in k},
                          "gauges_at_end": {k: v for k, v in after["metrics"].items()
                                            if not (k.endswith("_total") or "_total{" in k)},
                          "residency_mean": {key: sum(s[key] for s in inside) / len(inside)
                                             for key in ("rss_bytes", "pss_bytes", "threads")} if inside else {},
                          "network_bytes": {service: {direction: count - before["network"][service][direction]
                                                       for direction, count in values.items()}
                                            for service, values in after["network"].items()},
                          "started": started, "finished": finished}
                (args.output / f"{name}.json").write_text(json.dumps(result, indent=2) + "\n")
                results.append(result)
                print(json.dumps({k: result[k] for k in ("scenario", "cpu_average", "db_calls", "docker_requests")}), flush=True)

            async def no_action():
                pass

            async def run_container(index):
                name = f"{database}-{index}"
                workload_names.append(name)
                return await command("docker", "exec", ENGINE, "docker", "run", "-d", "--name", name,
                                     IMAGE, "sh", "-c", "trap 'exit 0' TERM; while :; do sleep 1 & wait $!; done")

            async def stop(ids):
                await api("/api/v1/containers/stop", ids, "PATCH")
                # Verify once after the command, outside any hot observation loop.
                literal = ",".join("'" + value + "'" for value in ids)
                count = await sql(f"SELECT count(*) FROM containers WHERE dockercontainerid IN ({literal}) AND state='Exited' AND controlstate='Idle'")
                if int(count) != len(ids):
                    raise RuntimeError("stop did not reach committed Exited/Idle for every container")

            async def burst():
                for index in range(20):
                    name = f"{database}-burst-{index}"
                    workload_names.append(name)
                    await command("docker", "exec", ENGINE, "docker", "create", "--name", name, IMAGE, "true")
                    await command("docker", "exec", ENGINE, "docker", "rm", name)
                    workload_names.remove(name)

            try:
                for _ in range(120):
                    if proc.returncode is not None:
                        raise RuntimeError("server exited; see server.log")
                    try:
                        await api("/health")
                        break
                    except Exception:
                        await asyncio.sleep(.5)
                else:
                    raise RuntimeError("server readiness timed out")
                auth = await api("/api/v1/setup/initialize", {"name": "cpuadmin", "email": "cpu@example.test",
                                                           "password": "Fixture!Measure-42-Only"})
                token = auth["accessToken"]
                await api("/api/v1/platforms", {"name": "cpu-local", "connectorType": "Local", "type": "Docker",
                                                "pruneHistoricalSwarmTaskContainers": False})
                await asyncio.sleep(30)
                await window("idle", no_action, args.idle_seconds)
                one = await run_container("single")
                await asyncio.sleep(15)
                await window("single-stop", lambda: stop([one]), 20)
                many = [await run_container(index) for index in range(20)]
                await asyncio.sleep(15)
                await window("stats-20-running", no_action, args.stats_seconds)
                await window("20-stop", lambda: stop(many), 30)
                await window("event-burst", burst, 30)
                await window("recovery-idle", no_action, 60)
                (args.output / "summary.json").write_text(json.dumps(results, indent=2) + "\n")
            finally:
                sampler.cancel()
                await asyncio.gather(sampler, return_exceptions=True)
                if proc.returncode is None:
                    proc.send_signal(signal.SIGTERM)
                    try:
                        await asyncio.wait_for(proc.wait(), 20)
                    except asyncio.TimeoutError:
                        proc.kill()
                        await proc.wait()
                proxy_server.close()
                await proxy_server.wait_closed()
                for task in list(active_proxy_tasks):
                    task.cancel()
                await asyncio.gather(*active_proxy_tasks, return_exceptions=True)
                for filename, rows in (("docker-requests.jsonl", requests), ("cpu.jsonl", samples)):
                    (args.output / filename).write_text("".join(json.dumps(row) + "\n" for row in rows))
                if workload_names:
                    await command("docker", "exec", ENGINE, "docker", "rm", "-f", *workload_names)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path)
    parser.add_argument("--binary", type=Path, default=ROOT / "target/release/citadel-server")
    parser.add_argument("--idle-seconds", type=int, default=300)
    parser.add_argument("--stats-seconds", type=int, default=60)
    args = parser.parse_args()
    if args.idle_seconds < 1 or args.stats_seconds < 1:
        parser.error("window durations must be positive")
    asyncio.run(capture(args))


if __name__ == "__main__":
    main()
