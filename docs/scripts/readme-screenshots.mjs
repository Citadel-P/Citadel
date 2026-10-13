// Demonstration data rendered by the real frontend, never a live installation.
export function prepareReadme({ routes, lookups, fixture, id, caps, version }) {
  routes["/application/info"].version = version;
  routes["/application/info"].realtimeTransport = "WebSocketV1";
  routes["/license/entitlements"].effectiveEdition = "Team";
  routes["/license/entitlements"].status = "Valid";
  routes["/application/info"].informationalVersion = version;
  const preferences = (routes["/profile/preferences"] = {
    theme: "Light",
    themeColor: "Violet",
  });
  const counts = (healthy) =>
    fixture("WorkloadStatusCounts", { total: healthy, healthy });
  const platforms = [false, true].map((swarm, index) =>
    fixture("PlatformView", {
      id: id(index + 1),
      name: swarm ? "Production Swarm" : "Production Docker",
      type: swarm ? "DockerSwarm" : "Docker",
      status: "Online",
      connectorType: swarm ? "Agent" : "Local",
      agentVersion: swarm ? version : null,
      address: swarm
        ? "https://manager.example.com:7443"
        : "unix:///var/run/docker.sock",
      clusterId: swarm ? "production-cluster" : null,
      serverVersion: "28.5.1",
      cpuCount: swarm ? 16 : 8,
      memTotal: 16 * 1024 ** 3,
      imageCount: 8,
      networkCount: 4,
      volumeCount: 5,
      deploymentCount: swarm ? 0 : 3,
      stackCount: 2,
      deploymentStatusCounts: counts(swarm ? 0 : 3),
      stackStatusCounts: counts(2),
      swarmServiceStatusCounts: counts(swarm ? 3 : 0),
      capabilities: { ...caps, canViewLogs: true, canViewStats: true },
      platformDescriptor: {
        $type: swarm ? "DockerSwarm" : "Docker",
        daemonId: `demo-${index}`,
        operatingSystem: "Ubuntu 24.04 LTS",
        architecture: "x86_64",
        containerCount: swarm ? 6 : 5,
        containersRunning: swarm ? 6 : 5,
        containersStopped: 0,
        containersPaused: 0,
        ...(swarm
          ? {
              nodes: 3,
              managers: 3,
              serviceCount: 3,
              runningTaskCount: 6,
              controlAvailable: true,
              localNodeState: "active",
              remoteManagers: [],
            }
          : {}),
      },
      stats: [
        fixture("PlatformStatView", {
          cpuUsage: swarm ? 8.4 : 4.2,
          memoryUsage: swarm ? 23.7 : 18.2,
          diskUsage: 31.4,
          diskTotalBytes: 512 * 1024 ** 3,
          diskUsedBytes: 161 * 1024 ** 3,
          created: 1791892800000,
        }),
      ],
    }),
  );
  routes["/platforms"] = { platforms, capabilities: caps };
  for (const p of platforms) routes[`/platforms/${p.id}`] = p;
  lookups.Platform = platforms;
  routes[`/platforms/${id(2)}/swarm`] = fixture("SwarmOverviewView", {
    platformId: id(2),
    health: "Healthy",
    nodeCount: 3,
    managerCount: 3,
    serviceCount: 3,
    imageCount: 8,
    networkCount: 4,
    volumeCount: 5,
    serviceStatusCounts: counts(3),
    runningTaskCount: 6,
    desiredTaskCount: 6,
    capabilities: caps,
    quorum: {
      state: "Healthy",
      hasLeader: true,
      reachableManagers: 3,
      requiredManagers: 2,
    },
  });
  const backup = (status, platformId = id(1)) =>
    fixture("ResourceBackupSummaryView", {
      status,
      platformId,
      canConfigure: true,
      canConfigureBindMount: true,
      hasResourcePolicy: status === "Protected",
    });
  const containers = [
    "storefront-web",
    "storefront-cache",
    "forgejo",
    "metrics",
    "worker",
  ].map((name, i) =>
    fixture("ContainerView", {
      id: id(30 + i),
      containerId: `${["a78c9e41bf05", "8a917bcc73d2", "734cf90a2e1b", "451bf809ead3", "ea938abd210f"][i]}e89011223`,
      name,
      stackId: i < 2 ? id(2) : null,
      deploymentId: i === 2 ? id(70) : null,
      hasCitadelOwnershipLabels: i < 3,
      state: "Running",
      platformId: id(1),
      controlState: "Idle",
      created: 1791705600000,
      updated: 1791892800000,
      capabilities: caps,
      imageView: fixture("ImageView", {
        name: [
          "nginx",
          "redis",
          "codeberg.org/forgejo/forgejo",
          "prom/prometheus",
          "ghcr.io/example/worker",
        ][i],
        tags: ["latest"],
        id: id(50 + i),
      }),
      ports:
        i === 0 ? { "80/tcp": [{ hostIp: "0.0.0.0", hostPort: "8080" }] } : {},
      backupCoverage: backup(
        ["NotApplicable", "Protected", "Protected", "Warning", "Unprotected"][
          i
        ],
      ),
      lastStats: fixture("ContainerStatView", {
        containerId: id(30 + i),
        cpuUsage: [0.8, 0.2, 1.6, 0.9, 0.4][i],
        memoryActive: [24, 18, 128, 96, 42][i] * 1024 ** 2,
        memoryLimit: 16 * 1024 ** 3,
      }),
    }),
  );
  routes[`/platforms/${id(1)}/containers`] = { containers, capabilities: caps };
  routes["/registries"] = {
    registries: [
      fixture("RegistryView", {
        id: id(4),
        name: "Docker Hub",
        registryHost: "docker.io",
        capabilities: caps,
      }),
    ],
    capabilities: caps,
  };
  routes["/buildProjects"] = { buildProjects: [], capabilities: caps };
  lookups.Registry = routes["/registries"].registries;
  const services = [
    "storefront-web",
    "storefront-cache",
    "background-worker",
  ].map((name, i) =>
    fixture("ManagedSwarmServiceView", {
      id: id(60 + i),
      name,
      dockerName: name,
      dockerServiceId: `service-${i}`,
      health: "Healthy",
      platformId: id(2),
      platformName: platforms[1].name,
      platformStatus: "Online",
      controlState: "Idle",
      synchronizationState: "InSync",
      runningTaskCount: [3, 1, 2][i],
      desiredTaskCount: [3, 1, 2][i],
      capabilities: {
        ...caps,
        canApply: true,
        canViewLogs: true,
        canViewStats: true,
      },
      backupCoverage: backup(i === 1 ? "Protected" : "NotApplicable", id(2)),
      spec: fixture("SwarmServiceSpec", {
        image: {
          $type: "External",
          registryId: id(4),
          imageTag: ["nginx:stable", "redis:alpine", "example/worker:latest"][
            i
          ],
        },
        schedulingMode: "Replicated",
        replicas: [3, 1, 2][i],
        updateBehavior: "Notify",
      }),
    }),
  );
  routes["/swarmServices"] = { swarmServices: services, capabilities: caps };
  for (const service of services)
    routes[`/swarmServices/${service.id}`] = service;
  for (const type of ["networks", "secrets", "configs"])
    routes[`/platforms/${id(2)}/swarm/${type}`] = {
      items: [],
      capabilities: caps,
    };
  const stack = routes[`/stacks/${id(2)}`];
  Object.assign(stack, {
    name: "Storefront",
    description: "Web application and persistent session cache",
    status: "Healthy",
    controlState: "Idle",
    platformId: id(1),
    platformName: platforms[0].name,
    platformStatus: "Online",
    platformType: "Docker",
    stackSource: "WebEditor",
    capabilities: {
      ...caps,
      canApply: true,
      canViewLogs: true,
      canViewStats: true,
      canViewReleases: true,
      canViewResourceBindings: true,
    },
    backupCoverage: backup("Protected"),
    spec: {
      $type: "WebEditor",
      composeFile:
        'services:\n  web:\n    image: nginx:stable\n    ports:\n      - "8080:80"\n    restart: unless-stopped\n\n  cache:\n    image: redis:alpine\n    volumes:\n      - cache_data:/data\n    restart: unless-stopped\n\nvolumes:\n  cache_data:\n',
      updateBehavior: "Notify",
      environmentVariables: [],
      registryId: null,
    },
  });
  routes[`/stacks/${stack.id}/data`] = {
    containers: containers.slice(0, 2).map((container) =>
      fixture("ContainerRuntimeView", {
        ...container,
        id: container.containerId,
        image: container.imageView.name + ":latest",
        imageId: container.imageView.id,
        stack: stack.name,
        stackId: stack.id,
        hasCitadelOwnershipLabels: true,
        containerStat: container.lastStats,
      }),
    ),
  };
  return { platforms, services, stack, preferences };
}

export async function captureReadme({
  page,
  context,
  origin,
  root,
  expect,
  data,
}) {
  // Fulfil the current realtime protocol locally; never connect to the live Core.
  await context.routeWebSocket("**/api/v1/realtime", (socket) => {
    const send = (message) =>
      socket.send(JSON.stringify({ protocolVersion: 1, ...message }));
    socket.onMessage((raw) => {
      const message = JSON.parse(String(raw));
      if (message.kind === "subscribe") send({ kind: "subscribed" });
      if (message.kind === "invoke") {
        send({
          kind: "completion",
          invocationId: message.invocationId,
          result: null,
        });
        if (message.target === "StartStackLogs") {
          const logs = [
            "2026-10-13T12:00:00Z [storefront-cache] Ready to accept connections tcp",
            "2026-10-13T12:00:01Z [storefront-web] Configuration complete; ready for start up",
            "2026-10-13T12:00:01Z [storefront-web] Starting worker processes",
            "2026-10-13T12:00:08Z [storefront-web] GET / HTTP/1.1 200",
            "2026-10-13T12:00:12Z [storefront-web] GET /assets/app.css HTTP/1.1 200",
            "2026-10-13T12:00:18Z [storefront-web] GET /health HTTP/1.1 200",
          ].join("\n");
          send({
            kind: "event",
            target: "SendStackLogs",
            arguments: [Array.from(Buffer.from(logs))],
          });
        }
      }
    });
  });
  await page.setViewportSize({ width: 1440, height: 1000 });
  for (const [name, path, ready] of [
    ["platforms", "/platforms", "Production Docker"],
    [
      "platform-containers",
      `/platforms/${data.platforms[0].id}/containers`,
      "storefront-web",
    ],
    ["swarm-services", "/swarm-services", "storefront-web"],
    [
      "swarm-service-config",
      `/swarm-services/edit/${data.services[0].id}/`,
      "storefront-web",
    ],
    ["stacks", `/stacks/edit/${data.stack.id}/`, "Storefront"],
    ["platforms-dark", "/platforms", "Production Docker"],
  ]) {
    if (page.url() !== "about:blank")
      await page.evaluate(() => localStorage.removeItem("sidebar-platform-id"));
    data.preferences.theme = name === "platforms-dark" ? "Dark" : "Light";
    await page.goto(origin + path, { waitUntil: "networkidle" });
    await expect(page.getByText(ready, { exact: true }).first()).toBeVisible();
    if (name === "stacks") {
      await page.getByRole("tab", { name: "Services", exact: true }).click();
      await expect(
        page.getByText("Ready to accept connections tcp", { exact: false }),
      ).toBeVisible();
    }
    await expect(
      page.getByText("Live updates unavailable", { exact: false }),
    ).toHaveCount(0);
    await expect(page.locator("html")).toHaveClass(
      name === "platforms-dark" ? /dark/ : /light/,
    );
    await page.evaluate(() => document.fonts.ready);
    await page.screenshot({
      path: `${root}/docs/public/screenshots/${name}.png`,
      animations: "disabled",
    });
    console.log(`Captured ${name}`);
  }
}
