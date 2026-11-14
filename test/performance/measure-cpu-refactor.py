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


async def postgres_resources(container=POSTGRES):
    """Read cgroup counters; file includes shmem, which is not reclaimable cache."""
    raw = await command("docker", "exec", container, "sh", "-c", """
if test -f /sys/fs/cgroup/memory.current; then
    echo v2
    cat /sys/fs/cgroup/memory.current /sys/fs/cgroup/cpu.stat /sys/fs/cgroup/memory.stat
else
    echo v1
    cat /sys/fs/cgroup/memory/memory.usage_in_bytes /sys/fs/cgroup/cpuacct/cpuacct.usage /sys/fs/cgroup/memory/memory.stat
fi
""")
    lines = raw.splitlines()
    total = int(lines[1])
    v2 = lines[0] == "v2"
    values = dict((key, int(value)) for key, value in (line.split() for line in lines[2 if v2 else 3:]))
    file_bytes = values["file"] if v2 else values.get("total_cache", values["cache"])
    shmem = values.get("shmem", 0) if v2 else values.get("total_shmem", values.get("shmem", 0))
    inactive = values["inactive_file"] if v2 else values.get("total_inactive_file", values["inactive_file"])
    active = values["active_file"] if v2 else values.get("total_active_file", values["active_file"])
    file_cache = max(0, file_bytes - shmem)
    return {"total_bytes": total,
            "anon_bytes": values["anon"] if v2 else values.get("total_rss", values["rss"]),
            "file_bytes": file_bytes, "shmem_bytes": shmem,
            "kernel_bytes": values.get("kernel"), "active_file_bytes": active,
            "inactive_file_bytes": inactive,
            "non_cache_bytes": max(0, total - file_cache),
            "working_set_bytes": max(0, total - inactive),
            "cpu_usage_usec": values["usage_usec"] if v2 else int(lines[2]) / 1000}


async def postgres_io():
    # Preserve per-device counters: summing stacked devices can double-count I/O.
    raw = await command("docker", "exec", POSTGRES, "sh", "-c",
                        "test ! -f /sys/fs/cgroup/io.stat || cat /sys/fs/cgroup/io.stat")
    return {fields[0]: {key: int(value) for key, value in
                       (item.split("=", 1) for item in fields[1:])}
            for line in raw.splitlines() if (fields := line.split())}



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
        "commit": await command("git", "-C", str(args.source), "rev-parse", "HEAD"),
        "dirty": bool(await command("git", "-C", str(args.source), "status", "--porcelain")),
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
        "capture_version": 3, "stack_burst": args.stack_burst,
        "maintenance_seconds": args.maintenance_seconds,
        "maintenance_start_seconds": args.maintenance_start_seconds,
        "allocator_environment": {"MALLOC_MMAP_THRESHOLD_": "131072"},
        "postgres_settings": json.loads(await sql("""SELECT json_object_agg(name, setting)
            FROM pg_settings WHERE name IN ('server_version','shared_buffers','work_mem',
                'maintenance_work_mem','max_connections','shared_preload_libraries','track_io_timing')""")),
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
            with urllib.request.urlopen(req, timeout=180) as response:
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
 SELECT queryid::text, calls, rows, total_exec_time, query,
        shared_blks_hit, shared_blks_read, shared_blks_dirtied, temp_blks_written, wal_bytes::float8
 FROM pg_stat_statements
 WHERE dbid=(SELECT oid FROM pg_database WHERE datname=current_database())
 AND query NOT LIKE '%pg_stat_statements%'
) s
"""

    async def snapshot():
        runtime, statements, postgres_net, docker_net, postgres, io, pg_io = await asyncio.gather(
            api("/metrics"), sql(statement_query),
            command("docker", "exec", POSTGRES, "cat", "/proc/net/dev"),
            command("docker", "exec", ENGINE, "cat", "/proc/net/dev"),
            postgres_resources(),
            postgres_io(),
            sql("""SELECT row_to_json(s) FROM (
                SELECT sum(read_bytes) AS read_bytes, sum(write_bytes) AS write_bytes,
                       sum(extend_bytes) AS extend_bytes, sum(read_time) AS read_ms,
                       sum(write_time) AS write_ms, sum(fsyncs) AS fsyncs
                FROM pg_stat_io) s"""),
        )
        return {"metrics": metrics(runtime), "statements": json.loads(statements), "postgres": postgres,
                "io": io, "pg_io": json.loads(pg_io),
                "network": {"postgres": network_bytes(postgres_net),
                            "docker": network_bytes(docker_net)}}

    with tempfile.TemporaryDirectory(prefix="citadel-cpu-") as temp:
        sock = Path(temp) / "docker.sock"
        proxy_server = await asyncio.start_unix_server(proxy, path=sock)
        env = {"PATH": os.environ["PATH"], "LANG": "C.UTF-8", "MALLOC_MMAP_THRESHOLD_": "131072",
               "HOME": temp,
               "CITADEL_DATABASE_URL": f"postgres://citadel_cpu:citadel_cpu_fixture@127.0.0.1:{args.postgres_port}/{database}",
               "CITADEL_TRANSPORT_MODE": "Disabled", "CITADEL_HTTP_LISTEN_PORT": "58049",
               "CITADEL_JWT_KEY": "cpu-measurement-disposable-signing-key-32bytes",
               "CITADEL_SECRETS_ENCRYPTION_KEY": "AQIDBAUGBwgJCgsMDQ4PEBESExQVFhcYGRobHB0eHyA=",
               "CITADEL_DATA_ROOT": str(Path(temp) / "state"),
               "CITADEL_DOCKER_SOCKET": str(sock),
               "CITADEL_DATABASE_MAX_CONNECTIONS": "5",
               "CITADEL_EVENT_QUEUE_CAPACITY": "256",
               "CITADEL_STATS_POLL_INTERVAL_SECONDS": "10",
               "RUST_LOG": "citadel_server=info,citadel_adapters=info"}
        with (args.output / "server.log").open("w") as log:
            proc = await asyncio.create_subprocess_exec(str(binary), "serve", env=env,
                                                        cwd=temp, stdout=log, stderr=log)
            process_started = time.monotonic()
            samples = []

            async def sample_cpu():
                tick_hz = os.sysconf("SC_CLK_TCK")
                previous, at = process_ticks(proc.pid), time.monotonic()
                postgres = None
                sampled_pg_at = 0.0
                postgres_cpu = None
                while True:
                    await asyncio.sleep(1)
                    current, now = process_ticks(proc.pid), time.monotonic()
                    if now - sampled_pg_at >= 5:
                        measured_pg = await postgres_resources()
                        if postgres is not None:
                            postgres_cpu = (measured_pg["cpu_usage_usec"] - postgres["cpu_usage_usec"]) / (now - sampled_pg_at) / 10000
                        postgres = measured_pg
                        sampled_pg_at = now
                    samples.append({"time": time.time(), "postgres": postgres, "postgres_cpu_percent": postgres_cpu, "cpu_percent":
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
                    delta = {k: row[k] - prior.get(k, 0) for k in ("calls", "rows", "total_exec_time", "shared_blks_hit", "shared_blks_read",
                                                                           "shared_blks_dirtied", "temp_blks_written", "wal_bytes")}
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
                          "postgres_before": before["postgres"], "postgres_after": after["postgres"],
                          "postgres_cpu_average": (after["postgres"]["cpu_usage_usec"] - before["postgres"]["cpu_usage_usec"]) / duration / 10000,
                          "postgres_cpu_max": max((sample["postgres_cpu_percent"] for sample in inside
                                                   if sample["postgres_cpu_percent"] is not None), default=None),
                          "postgres_memory_peak": {key: max((sample["postgres"][key] for sample in inside), default=None)
                                                   for key in ("total_bytes", "working_set_bytes", "non_cache_bytes")},
                          "db_calls": sum(s["calls"] for s in statements), "statements": statements,
                          "postgres_io_by_device": {device: {key: value - before["io"].get(device, {}).get(key, 0)
                                                              for key, value in counters.items()}
                                                    for device, counters in after["io"].items()},
                          "postgres_io_operations": {key: (value or 0) - (before["pg_io"][key] or 0)
                                                     for key, value in after["pg_io"].items()},
                          "metrics_delta": {k: v - before["metrics"].get(k, 0) for k, v in after["metrics"].items()
                                            if k.endswith("_total") or "_total{" in k},
                          "gauges_at_end": {k: v for k, v in after["metrics"].items()
                                            if not (k.endswith("_total") or "_total{" in k)},
                          "residency_at_end": process_residency(proc.pid),
                          "residency_peak": {key: max((s[key] for s in inside), default=None)
                                             for key in ("rss_bytes", "pss_bytes", "threads")},
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

            async def seed_history():
                await sql("""
INSERT INTO statisticssources(platformid,containerid)
 SELECT (SELECT id FROM platforms LIMIT 1),gen_random_uuid() FROM generate_series(1,2000);
INSERT INTO statisticssamples(sourceid,created,cpuusage,memoryactive,memorycache,memorylimit,rxbytes,txbytes)
 SELECT s.id,(extract(epoch FROM now())::bigint/86400-3)*86400+n,1,100,5,200,n,n
 FROM (SELECT id FROM statisticssources ORDER BY id DESC LIMIT 4) s CROSS JOIN generate_series(1,80000)n;
-- Populated default partition exercises expiry across the seven-day boundary.
INSERT INTO statisticssamples(sourceid,created,cpuusage,memoryactive,memorycache,memorylimit,rxbytes,txbytes)
 SELECT (SELECT max(id) FROM statisticssources),(extract(epoch FROM now())::bigint/86400-8)*86400+n,1,100,5,200,n,n
 FROM generate_series(1,10001)n;
INSERT INTO activityevents(id,createdat,createdbyactorid,eventtype,info,resourcename,resourcetype,status)
 SELECT gen_random_uuid(),now()-interval '100 days','00000000-0000-0000-0000-000000000001','Measurement','{}','fixture','Test','Succeeded'
 FROM generate_series(1,100001);
ANALYZE statisticssources; ANALYZE statisticssamples; ANALYZE activityevents;
""")

            async def history_counts():
                return json.loads(await sql("""SELECT json_build_object(
                    'raw', (SELECT count(*) FROM statisticssamples),
                    'rollups', (SELECT count(*) FROM statisticsrollups),
                    'expired_raw', (SELECT count(*) FROM statisticssamples WHERE created<extract(epoch FROM now())::bigint-604800),
                    'expired_activities', (SELECT count(*) FROM activityevents WHERE createdat<now()-interval '90 days'))"""))

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

            async def stack_burst(platform_id):
                stacks = []
                for index in range(4):
                    name = f"{database}-stack-{index}"
                    workload_names.append(name)
                    compose = (f"services:\n  app:\n    image: {IMAGE}\n"
                               f"    container_name: {name}\n    command: [sleep, '86400']\n")
                    stack = await api("/api/v1/stacks", {
                        "name": name, "platformId": platform_id, "stackSource": "WebEditor",
                        "spec": {"$type": "WebEditor", "composeFile": compose,
                                 "updateBehavior": "Disabled", "destroyBeforeDeploy": False}})
                    stacks.append(stack["id"])

                async def apply(stack_id, recreate):
                    progress = await api("/api/v1/stacks/apply", {"id": stack_id, "recreate": recreate})
                    if not isinstance(progress, list) or progress[-1].get("stackStatus") != "Healthy":
                        raise RuntimeError(f"stack deployment failed: {progress}")

                await window("four-stack-deploy", lambda: asyncio.gather(*(apply(s, False) for s in stacks)), 60)
                await window("four-stack-redeploy", lambda: asyncio.gather(*(apply(s, True) for s in stacks)), 60)

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
                platform_created = await api("/api/v1/platforms", {"name": "cpu-local", "connectorType": "Local", "type": "Docker",
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
                if args.stack_burst:
                    await stack_burst(platform_created["id"])
                await window("recovery-idle", no_action, 60)
                if args.maintenance_seconds:
                    if args.maintenance_start_seconds:
                        remaining = args.maintenance_start_seconds - (time.monotonic() - process_started)
                        if remaining < 0:
                            raise RuntimeError("ordinary scenarios overran the requested maintenance start")
                        await asyncio.sleep(remaining)
                    await seed_history()
                    counts_before = await history_counts()
                    await window("aged-maintenance", no_action, args.maintenance_seconds)
                    (args.output / "maintenance-progress.json").write_text(json.dumps(
                        {"before": counts_before, "after": await history_counts()}, indent=2) + "\n")
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
    parser.add_argument("--source", type=Path, default=ROOT, help="source checkout used to build --binary")
    parser.add_argument("--postgres-port", type=int, default=55449)
    parser.add_argument("--stack-burst", action="store_true", help="deploy and recreate four Compose stacks through Core")
    parser.add_argument("--idle-seconds", type=int, default=300)
    parser.add_argument("--stats-seconds", type=int, default=60)
    parser.add_argument("--maintenance-seconds", type=int, default=0,
                        help="opt-in aged telemetry/Activity backlog window (use at least 900s for retention cadence)")
    parser.add_argument("--maintenance-start-seconds", type=int, default=0,
                        help="align backlog seeding to this offset after Core starts; fail if earlier scenarios overrun")
    args = parser.parse_args()
    if args.idle_seconds < 1 or args.stats_seconds < 1 or min(args.maintenance_seconds, args.maintenance_start_seconds) < 0:
        parser.error("window durations must be positive")
    asyncio.run(capture(args))


if __name__ == "__main__":
    main()
