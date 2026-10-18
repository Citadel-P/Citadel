// Capture current resource interfaces with deterministic demonstration data.
export function prepareFeatures({
  routes,
  fixture,
  id,
  caps,
  capturedAt,
  snapshots,
  data,
}) {
  const platformId = data.platforms[0].id;
  const containers = routes[
    `/platforms/${platformId}/containers`
  ].containers.slice(0, 2);
  const networks = ["frontend", "backend"].map((suffix, index) => {
    const attached = index === 0 ? containers.slice(0, 1) : containers;
    return fixture("NetworkView", {
      id: id(80 + index),
      name: `storefront_${suffix}`,
      driver: "bridge",
      scope: "local",
      inUse: true,
      enableIPv4: true,
      internal: index === 1,
      created: "2026-10-12T10:00:00Z",
      capabilities: { ...caps, canInspect: true },
      ipam: {
        driver: "default",
        options: {},
        config: [
          {
            subnet: `172.${20 + index}.0.0/24`,
            gateway: `172.${20 + index}.0.1`,
          },
        ],
      },
      containers: Object.fromEntries(
        attached.map((container, i) => [
          container.containerId,
          {
            name: container.name,
            endpointId: `endpoint-${index}-${i}`,
            macAddress: `02:42:ac:15:00:0${i + 2}`,
            ipV4Address: `172.${20 + index}.0.${i + 2}/24`,
            ipv6Address: "",
          },
        ]),
      ),
      usage: {
        isPartial: false,
        truncated: false,
        entries: attached.map((container) =>
          fixture("NetworkUsageEntry", {
            containerId: container.id,
            dockerContainerId: container.containerId,
            containerName: container.name,
            containerState: "Running",
            relationship: "Attachment",
            workloadKind: "Stack",
            workloadId: data.stack.id,
            workloadName: data.stack.name,
            workloadStatus: "Healthy",
            observedAt: capturedAt,
          }),
        ),
      },
    });
  });
  routes[`/networks/${platformId}`] = { networks, capabilities: caps };
  snapshots[`container-health:${containers[0].id}`] = {
    target: "ReceiveContainerHealthHistory",
    arguments: [
      {
        resourceId: containers[0].id,
        entries: Array.from({ length: 60 }, (_, i) => {
          const observedAt = capturedAt / 1000 - (59 - i) * 60;
          const status = i >= 22 && i < 30 ? "Unhealthy" : "Healthy";
          return fixture("ContainerHealthTransitionView", {
            observedAt,
            firstObservedAt: observedAt,
            sampleCount: 1,
            state: "Running",
            origin: "Inspection",
            health: fixture("ContainerHealthView", {
              status,
              reason:
                status === "Healthy"
                  ? "Health check is passing."
                  : "Health check failed.",
              observedAt: observedAt * 1000,
              lastCheckAt: new Date(observedAt * 1000).toISOString(),
              failingStreak: status === "Unhealthy" ? i - 21 : 0,
            }),
          });
        }),
      },
    ],
  };
  const volumeName = "storefront_uploads";
  routes[`/volumes/${platformId}/${volumeName}`] = fixture("VolumeView", {
    id: volumeName,
    name: volumeName,
    driver: "local",
    scope: "local",
    inUse: true,
    mountpoint: `/var/lib/docker/volumes/${volumeName}/_data`,
    createdAt: "2026-10-12T10:00:00Z",
    contentAccessAllowed: true,
    containers: [
      fixture("VolumeContainerView", {
        id: containers[0].containerId,
        name: containers[0].name,
        state: "Running",
        image: "nginx:alpine",
        imageId: containers[0].imageView.id,
      }),
    ],
    capabilities: {
      ...caps,
      canBrowse: true,
      canDownload: true,
      canInspect: true,
    },
  });
  const directory = (path, entries) => ({
    platformId,
    volumeName,
    path,
    isTruncated: false,
    entries: entries.map(([name, type, size]) => ({
      name,
      type,
      size: size ?? null,
      path: `${path === "/" ? "" : path}/${name}`,
      modifiedAt: "2026-10-18T09:30:00Z",
    })),
  });
  const directories = {
    "/": directory("/", [
      ["images", "Directory"],
      ["documents", "Directory"],
      ["exports", "Directory"],
      ["README.txt", "File", 1240],
      ["manifest.json", "File", 2864],
    ]),
    "/images": directory("/images", [
      ["banner.webp", "File", 128460],
      ["logo.svg", "File", 3480],
      ["product-card.webp", "File", 64200],
    ]),
    "/documents": directory("/documents", [
      ["getting-started.pdf", "File", 245760],
      ["release-notes.txt", "File", 4820],
    ]),
  };
  routes[`/platforms/${platformId}/volumes/${volumeName}/files`] = (url) =>
    directories[url.searchParams.get("path") ?? "/"];
  return {
    platformId,
    container: containers[0],
    volumeName,
    stack: data.stack,
  };
}

export async function captureFeatures({
  page,
  origin,
  root,
  expect,
  data,
  modes,
}) {
  await page.setViewportSize({ width: 1440, height: 900 });
  async function save(name, panel) {
    await page.evaluate(() => document.fonts.ready);
    await expect(
      page.getByText("Live updates unavailable", { exact: false }),
    ).toHaveCount(0);
    await expect(
      page
        .locator('div[style*="position: fixed"][style*="height: 2px"] > div')
        .first(),
    ).toHaveCSS("width", "0px");
    await panel.screenshot({
      path: `${root}/docs/public/screenshots/${name}.png`,
      animations: "disabled",
    });
    console.log(`Captured ${name}`);
  }
  if (modes.includes("network")) {
    await page.goto(`${origin}/stacks/edit/${data.stack.id}/`, {
      waitUntil: "networkidle",
    });
    await page
      .getByRole("button", { name: "View connections", exact: true })
      .click();
    const sheet = page.getByRole("dialog");
    await sheet.getByRole("button", { name: "Maximize network sheet" }).click();
    await sheet
      .getByRole("button", { name: "Expand Storefront", exact: true })
      .click();
    await expect(
      sheet.getByRole("button", {
        name: "Show details for storefront-cache",
        exact: true,
      }),
    ).toBeVisible();
    await expect(sheet.locator(".react-flow__edge")).toHaveCount(3);
    await sheet
      .getByRole("button", {
        name: "Show details for storefront_backend",
        exact: true,
      })
      .click();
    await expect(sheet.getByText("172.21.0.1", { exact: true })).toBeVisible();
    await page.mouse.move(10, 10);
    await save("network-map", sheet);
  }
  if (modes.includes("health")) {
    await page.goto(`${origin}/platforms/${data.platformId}/containers`, {
      waitUntil: "networkidle",
    });
    const row = page
      .getByRole("row")
      .filter({ hasText: data.container.name })
      .last();
    await row
      .getByRole("button", { name: "Health: Healthy", exact: true })
      .click();
    await page
      .getByRole("button", { name: "View details", exact: true })
      .click();
    const dialog = page.getByRole("dialog", {
      name: "Health details",
      exact: true,
    });
    await expect(
      dialog.getByText("60 recorded observations", { exact: false }),
    ).toBeVisible();
    await expect(
      dialog.locator(".recharts-scatter-symbol").first(),
    ).toBeVisible();
    await page.mouse.move(10, 10);
    await save("container-health-details", dialog);
  }
  if (modes.includes("volume")) {
    await page.setViewportSize({ width: 1440, height: 540 });
    await page.goto(
      `${origin}/platforms/${data.platformId}/volumes/${data.volumeName}`,
      { waitUntil: "networkidle" },
    );
    await page.getByRole("button", { name: "Browse", exact: true }).click();
    const sheet = page.getByRole("dialog", {
      name: data.volumeName,
      exact: true,
    });
    await sheet
      .getByRole("button", { name: "Expand images", exact: true })
      .click();
    await sheet
      .getByRole("button", { name: "Expand documents", exact: true })
      .click();
    await expect(
      sheet.getByText("getting-started.pdf", { exact: true }),
    ).toBeVisible();
    await sheet.getByText("banner.webp", { exact: true }).hover();
    await save("volume-browser", sheet);
  }
}
