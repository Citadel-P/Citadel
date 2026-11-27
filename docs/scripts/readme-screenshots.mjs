// Demonstration data rendered by the real frontend, never a live installation.
import { MIN_SCREENSHOT_WIDTH, saveScreenshot } from "./screenshot-output.mjs";

function prepareDashboard({ routes, fixture, id, platform, capturedAt }) {
  const name = "Production overview";
  const tags = [
    ["Production", "#dbeafe"],
    ["Storefront", "#ede9fe"],
  ].map(([name, color], index) => fixture("AuthorizedTagView", {
    id: id(130 + index),
    name,
    normalizedName: name.toLowerCase(),
    color,
  }));
  routes["/tags"] = { ...routes["/tags"], tags };
  const tagNames = tags.map((tag) => tag.normalizedName);
  const requestedTags = (url) => url.searchParams.getAll("tags");
  const matchesTags = (url, names) =>
    requestedTags(url).every((tag) => names.includes(tag.toLowerCase()));
  routes["/tags/_filter"] = (url) =>
    Object.fromEntries(requestedTags(url).map((tag) => [tag, tag.toLowerCase()]));
  // Staging belongs to Storefront as well, but lacks the Production tag.
  const scoped = (url, production, staging) => [
    ...(matchesTags(url, tagNames) ? production : []),
    ...(matchesTags(url, ["storefront"]) ? staging : []),
  ];
  const summary = fixture("DashboardSummaryView", {
    id: id(100),
    name,
    visibility: "InstanceShared",
    revision: 1,
    canEdit: true,
    canDelete: true,
  });
  const metric = (metric, color) => ({
    kind: "Metrics",
    source: { kind: "Platform", id: platform.id },
    metric,
    color,
    presentation: "Gradient",
    legend: false,
  });
  const widgets = [
    ["Fleet summary", 0, 0, 12, 2, { kind: "FleetSummary", measures: ["Platforms", "Workloads", "Alerts"] }],
    ["CPU usage", 0, 2, 6, 5, metric("Cpu", "Blue")],
    ["Memory usage", 6, 2, 6, 5, metric("Memory", "Violet")],
    ["Deployment state", 0, 7, 4, 5, { kind: "ResourceState", resourceKind: "Deployment" }],
    ["Needs attention", 4, 7, 4, 5, { kind: "NeedsAttention", kinds: [], tab: "All", limit: 3 }],
    ["Recent activity", 8, 7, 4, 5, { kind: "RecentActivity", limit: 4 }],
  ].map(([title, x, y, w, h, settings], index) => ({
    id: id(101 + index),
    title,
    layout: { x, y, w, h },
    settings,
  }));
  routes["/profile/dashboard-default"] = { dashboardId: summary.id };
  routes[`/dashboards/${summary.id}`] = {
    summary,
    configuration: {
      schemaVersion: 2,
      description: "Storefront, background workers, and their Docker hosts.",
      rangeMinutes: 360,
      tags: tags.map((tag) => tag.name),
      widgets,
    },
  };
  routes["/dashboards/sources"] = (url) => [
    fixture("DashboardSourceView", {
      id: platform.id,
      name: platform.name,
      platformId: platform.id,
      status: "Online",
      matchesTags: matchesTags(url, tagNames),
    }),
  ];

  const statsPath = `/platforms/${platform.id}/stats`;
  const defaultStats = routes[statsPath];
  const samples = Array.from({ length: 361 }, (_, minute) => {
    const pulse = (center, width) => Math.exp(-(((minute - center) / width) ** 2));
    return fixture("PlatformStatView", {
      created: capturedAt / 1000 - (360 - minute) * 60,
      cpuUsage:
        9 + minute * 0.022 + 1.6 * Math.sin(minute / 8) + Math.sin(minute / 3) +
        10 * pulse(44, 9) + 24 * pulse(124, 11) + 15 * pulse(230, 16) + 22 * pulse(296, 12),
      memoryUsage:
        42 + minute * 0.026 + 0.55 * Math.sin(minute / 11) +
        (minute >= 71 ? 5 : 0) + (minute >= 158 ? 6 : 0) - (minute >= 277 ? 7 : 0),
    });
  });
  routes[statsPath] = (url) =>
    url.searchParams.get("minutes") === "360" ? { stats: samples } : defaultStats;

  const states = [
    ["Platform", "Online", 2],
    ["Deployment", "Healthy", 19],
    ["Deployment", "Stopped", 2],
    ["Deployment", "Degraded", 1],
    ["Stack", "Healthy", 4],
    ["SwarmService", "Healthy", 6],
  ].map(([kind, state, count]) => ({ kind, state, count }));
  const rows = [
    ["Deployment", "checkout-api", "Degraded", "Degraded"],
    ["Stack", "Storefront", "Healthy", "New Git commit"],
    ["SwarmService", "background-worker", "Healthy", "Image update"],
  ].map(([kind, name, state, code], index) => ({
    id: id(110 + index),
    kind,
    name,
    state,
    reasons: [{ code, occurrences: 1 }],
  }));
  const stagingRow = {
    id: id(113),
    kind: "Deployment",
    name: "checkout-staging",
    state: "Failed",
    reasons: [{ code: "Failed", occurrences: 1 }],
  };
  routes["/dashboards/fleet"] = (url) => {
    const { searchParams } = url;
    const read = searchParams.get("read");
    const tab = searchParams.get("tab");
    const kinds = searchParams.get("kinds");
    const scopeStates = scoped(url, states, [{ kind: "Deployment", state: "Failed", count: 1 }]);
    const scopeRows = scoped(url, rows, [stagingRow]);
    const problems = scopeRows.filter(({ state }) => state !== "Healthy");
    const updates = scopeRows.filter(({ state }) => state === "Healthy");
    const results = (tab === "Problems" ? problems : tab === "Updates" ? updates : scopeRows)
      .filter((row) => !kinds || kinds.split(",").includes(row.kind));
    const offset = Number(searchParams.get("offset") ?? 0);
    const limit = Number(searchParams.get("limit") ?? 25);
    return {
      alerts: (matchesTags(url, tagNames) ? 2 : 0) + (matchesTags(url, ["storefront"]) ? 1 : 0),
      problems: problems.length,
      updates: updates.length,
      total: scopeRows.length,
      rows: read === "Attention" ? results.slice(offset, offset + limit) : [],
      states: read === "Summary" ? scopeStates : scopeStates.filter((row) => row.kind === kinds),
    };
  };
  const recent = [
    ["Deployment", "orders-api", "DeploymentUpdated", 7],
    ["BackupPolicy", "storefront volumes", "BackupRunCompleted", 18],
    ["Stack", "Storefront", "StackApplied", 32],
    ["SwarmService", "background-worker", "SwarmServiceUpdated", 46],
  ].map(([resourceType, resourceName, eventType, ageMinutes], index) => ({
    id: id(120 + index),
    resourceType,
    resourceName,
    eventType,
    status: "Success",
    createdAt: new Date(capturedAt - ageMinutes * 60_000).toISOString(),
  }));
  const stagingActivity = {
    id: id(124),
    resourceType: "Deployment",
    resourceName: stagingRow.name,
    eventType: "DeploymentApplied",
    status: "Failure",
    createdAt: new Date(capturedAt - 4 * 60_000).toISOString(),
  };
  routes["/activities/recent"] = (url) =>
    scoped(url, recent, [stagingActivity])
      .sort((left, right) => right.createdAt.localeCompare(left.createdAt))
      .slice(0, Number(url.searchParams.get("limit") ?? 5));

  return name;
}

export function prepareReadme({ routes, lookups, fixture, id, caps, version, capturedAt }) {
  routes["/application/info"].version = version;
  routes["/application/info"].realtimeTransport = "WebSocketV1";
  routes["/license/entitlements"].effectiveEdition = "Team";
  routes["/license/entitlements"].status = "Valid";
  routes["/application/info"].informationalVersion = version;
  const preferences = (routes["/profile/preferences"] = {
    theme: "Light",
    themeColor: "Blue",
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
          created: capturedAt / 1000,
        }),
      ],
    }),
  );
  routes["/platforms"] = { platforms, capabilities: caps };
  for (const p of platforms) {
    routes[`/platforms/${p.id}`] = p;
    const stats = Array.from({ length: 361 }, (_, step) =>
      fixture("PlatformStatView", {
        created: capturedAt / 1000 - (360 - step) * 10,
        cpuUsage: 12 + 4 * Math.sin(step / 30) + 2 * Math.cos(step / 18),
        memoryUsage: 23 + step / 300 + Math.sin(step / 54),
        diskUsage: 31.4,
        diskTotalBytes: 512 * 1024 ** 3,
        diskUsedBytes: 161 * 1024 ** 3,
        rxBytes: 250_000 + 80_000 * Math.sin(step / 24),
        txBytes: 130_000 + 40_000 * Math.cos(step / 36),
      }),
    );
    routes[`/platforms/${p.id}/stats`] = { stats };
  }
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
      health: fixture("ContainerHealthView", {
        status: i < 3 ? "Healthy" : "NoCheck",
        observedAt: capturedAt,
        lastCheckAt: i < 3 ? new Date(capturedAt).toISOString() : null,
        failingStreak: i < 3 ? 0 : null,
        reason: i < 3 ? "The configured health check is passing." : "No health check configured.",
      }),
      platformId: id(1),
      controlState: "Idle",
      created: 1791705600000,
      updated: capturedAt,
      capabilities: caps,
      imageView: fixture("ImageView", {
        name: [
          "nginx",
          "redis",
          "codeberg.org/forgejo/forgejo",
          "prom/prometheus",
          "ghcr.io/example/worker",
        ][i],
        tags: [i < 2 ? "alpine" : "latest"],
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
      probeHealth: fixture("WorkloadHealth", {
        status: "Healthy", coverageComplete: true,
        counts: fixture("HealthCounts", { healthy: [3, 1, 2][i] }),
        observed: [3, 1, 2][i], expected: [3, 1, 2][i], missing: 0,
        observedAt: capturedAt, evaluatedAt: capturedAt,
        reason: "All configured checks are passing.",
      }),
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
          imageTag: ["nginx:alpine", "redis:alpine", "example/worker:latest"][
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
    probeHealth: fixture("WorkloadHealth", {
      status: "Healthy", coverageComplete: true,
      counts: fixture("HealthCounts", { healthy: 2 }),
      observed: 2, expected: 2, missing: 0,
      observedAt: capturedAt, evaluatedAt: capturedAt,
      reason: "All configured checks are passing.",
      members: containers.slice(0, 2).map((container) => fixture("HealthMember", {
        status: "Healthy", available: true,
        containerId: container.id, dockerContainerId: container.containerId,
        platformId: container.platformId, name: container.name,
        service: container.name === "storefront-web" ? "web" : "cache",
        state: container.state,
        observation: fixture("ContainerHealthObservation", {
          configuration: "Enabled", result: "Healthy",
          observedAt: capturedAt, lastCheckAt: new Date(capturedAt).toISOString(),
          failingStreak: 0,
        }),
      })),
    }),
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
        'services:\n  web:\n    image: nginx:alpine\n    healthcheck:\n      test: ["CMD", "wget", "-q", "-O", "/dev/null", "http://127.0.0.1/"]\n    ports:\n      - "8080:80"\n    restart: unless-stopped\n\n  cache:\n    image: redis:alpine\n    healthcheck:\n      test: ["CMD", "redis-cli", "ping"]\n    volumes:\n      - cache_data:/data\n    restart: unless-stopped\n\nvolumes:\n  cache_data:\n',
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
        image: container.imageView.name + ":" + container.imageView.tags[0],
        imageId: container.imageView.id,
        stack: stack.name,
        stackId: stack.id,
        hasCitadelOwnershipLabels: true,
        containerStat: container.lastStats,
      }),
    ),
  };
  const dashboard = prepareDashboard({ routes, fixture, id, platform: platforms[0], capturedAt });
  return { platforms, services, stack, preferences, dashboard };
}

export async function mockScreenshotRealtime(context, snapshots = {}) {
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
        if (message.target === "JoinGroup" && snapshots[message.arguments?.[0]])
          send({ kind: "event", ...snapshots[message.arguments[0]] });
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
}

export async function captureReadme({
  page,
  origin,
  root,
  expect,
  data,
}) {
  await page.setViewportSize({ width: MIN_SCREENSHOT_WIDTH, height: 1000 });
  await page.addInitScript((platformId) => {
    if (window.location.pathname === "/") {
      localStorage.setItem("sidebar-platform-id", JSON.stringify(platformId));
      localStorage.setItem("sidebar-platform-expanded", "false");
    }
  }, data.platforms[0].id);
  for (const [name, path, ready] of [
    ["dashboard", "/", data.dashboard],
    ["dashboard-dark", "/", data.dashboard],
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
    ["platform-stats", `/platforms/edit/${data.platforms[0].id}/#stats`, "Production Docker"],
    ["platform-stats-dark", `/platforms/edit/${data.platforms[0].id}/#stats`, "Production Docker"],
  ]) {
    if (page.url() !== "about:blank")
      await page.evaluate(() => localStorage.removeItem("sidebar-platform-id"));
    data.preferences.theme = name.endsWith("-dark") ? "Dark" : "Light";
    if (page.url() === origin + path)
      await page.reload({ waitUntil: "networkidle" });
    else
      await page.goto(origin + path, { waitUntil: "networkidle" });
    await expect(page.getByText(ready, { exact: true }).first()).toBeVisible({ timeout: 15000 });
    if (name === "dashboard" || name === "dashboard-dark") {
      for (const title of ["CPU usage", "Memory usage"]) {
        await expect(page.getByRole("region", { name: title }).locator(".recharts-area-curve")).toBeVisible();
      }
      await expect(page.getByRole("region", { name: "Deployment state" }).locator(".dashboard-state-ring")).toBeVisible();
      const attention = page.getByRole("region", { name: "Needs attention" });
      const recentActivity = page.getByRole("region", { name: "Recent activity" });
      await expect(attention).toContainText("checkout-api");
      await expect(attention.getByRole("button", { name: "View all 3 resources" })).toHaveClass(/dashboard-widget-footer/);
      await expect(recentActivity).toContainText("orders-api");
      await expect(recentActivity.getByText("checkout-staging", { exact: true })).toHaveCount(0);
      await expect(page.getByRole("button", { name: "Remove Production tag filter" })).toBeVisible();
      await expect(page.getByRole("button", { name: "Remove Storefront tag filter" })).toBeVisible();
      const activity = await recentActivity.boundingBox();
      expect(activity.y + activity.height).toBeLessThanOrEqual(1000);
    }
    if (name === "stacks") {
      await page.getByRole("tab", { name: "Services", exact: true }).click();
      await expect(
        page.getByText("Ready to accept connections tcp", { exact: false }),
      ).toBeVisible();
    }
    if (name.startsWith("platform-stats")) {
      await expect(page.locator(".recharts-area-curve").first()).toBeVisible();
      await page.getByRole("tablist").scrollIntoViewIfNeeded();
      await page.getByRole("tablist").evaluate((element) =>
        element.scrollIntoView({ block: "start", behavior: "instant" }),
      );
    }
    await expect(
      page.getByText("Live updates unavailable", { exact: false }),
    ).toHaveCount(0);
    await expect(page.locator("html")).toHaveClass(
      name.endsWith("-dark") ? /dark/ : /light/,
    );
    await expect(page.locator("html")).toHaveAttribute("data-theme-color", "blue");
    await page.evaluate(() => document.fonts.ready);
    await expect(page.locator('div[style*="position: fixed"][style*="height: 2px"] > div').first()).toHaveCSS("width", "0px");
    await saveScreenshot(page, {
      path: `${root}/docs/public/screenshots/${name}.png`,
      animations: "disabled",
    });
    if (name === "dashboard") {
      await page.getByRole("button", { name: "Remove Production tag filter" }).click();
      await expect(page.getByRole("region", { name: "Recent activity" })).toContainText("checkout-staging");
    }
    console.log(`Captured ${name}`);
  }
}
