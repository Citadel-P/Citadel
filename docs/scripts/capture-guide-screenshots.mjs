import { createRequire } from "node:module";
import { fileURLToPath } from "node:url";
const require = createRequire(
  new URL("../../test/e2e/package.json", import.meta.url),
);
const { chromium, expect } = require("@playwright/test");
import fs from "node:fs/promises";
const root = fileURLToPath(new URL("../../", import.meta.url));
const origin =
  process.env.CITADEL_SCREENSHOT_BASE_URL ?? "http://localhost:5173";
// Fresh browser context; all application API requests use demonstration fixtures.
// Never forward an unknown request or a mutation to a running backend.
const schemas = JSON.parse(await fs.readFile(`${root}/schema/v1.json`, "utf8"))
  .components.schemas;
const id = (n) => `00000000-0000-4000-8000-${String(n).padStart(12, "0")}`;
const caps = { canRead: true, canWrite: true, canExecute: true };
function sample(s) {
  if (s.$ref) return sample(schemas[s.$ref.split("/").at(-1)]);
  if (s.oneOf) return sample(s.oneOf[0]);
  if (s.allOf) return Object.assign({}, ...s.allOf.map(sample));
  if (s.enum) return s.enum[0];
  const t = Array.isArray(s.type)
    ? s.type.includes("null")
      ? "null"
      : s.type[0]
    : s.type;
  if (t === "object")
    return Object.fromEntries(
      Object.entries(s.properties ?? {}).map(([k, v]) => [k, sample(v)]),
    );
  if (t === "array") return [];
  if (t === "boolean") return false;
  if (t === "integer" || t === "number") return 0;
  if (t === "null") return null;
  return s.format === "uuid"
    ? id(99)
    : s.format === "date-time"
      ? "2026-01-01T12:00:00Z"
      : "";
}
const fixture = (name, overrides) => ({
  ...sample(schemas[name]),
  ...overrides,
});
const platform = fixture("PlatformView", {
  id: id(1),
  name: "Production Swarm",
  type: "DockerSwarm",
  status: "Running",
  connectorType: "Agent",
  address: "https://manager.example.com:7443",
  clusterId: "demo-cluster",
  capabilities: caps,
  stats: [],
});
const stack = fixture("StackView", {
  id: id(2),
  name: "Storefront",
  stackSource: "Git",
  platformId: id(1),
  platformName: platform.name,
  platformType: "DockerSwarm",
  platformStatus: "Running",
  capabilities: caps,
  spec: {
    $type: "Git",
    gitRepoId: id(3),
    branch: "main",
    composePaths: [
      "stacks/storefront/compose.yaml",
      "stacks/storefront/compose.production.yaml",
    ],
    composeEnvFilesFromRepo: ["stacks/storefront/production.env"],
    workingDirectory: "stacks/storefront",
    watchPaths: ["stacks/storefront/**", "shared/**"],
    registryId: id(4),
    updateBehavior: "Notify",
    buildImageBindings: [],
  },
  driftPolicy: { mode: "Disabled" },
  tags: [],
});
const repo = fixture("GitRepositoryView", {
  id: id(3),
  name: "Application infrastructure",
  url: "https://git.example.com/team/infrastructure.git",
  branch: "main",
  capabilities: caps,
  status: "Synced",
});
const backupRepo = fixture("BackupRepositoryView", {
  id: id(5),
  name: "Operations backups",
  type: "S3Compatible",
  status: "Ready",
  capabilities: caps,
  spec: {
    $type: "S3Compatible",
    bucket: "example-backups",
    endpoint: "https://s3.example.com",
    prefix: "production",
    region: "us-east-1",
  },
});
const policy = fixture("BackupPolicyView", {
  id: id(6),
  name: "Storefront volumes",
  enabled: true,
  backupRepositoryId: id(5),
  runAsActorId: id(8),
  source: { $type: "Stack", stackId: id(2) },
  keepLastSuccessful: 14,
  timeoutSeconds: 1800,
  alertOnFailure: true,
  capabilities: caps,
});
const action = fixture("AutomationActionView", {
  id: id(7),
  name: "Scheduled inventory check",
  code: 'console.log("Inventory check", args.environment);',
  defaultArgsJson: '{"environment":"production"}',
  enabled: true,
  runAsActorId: id(8),
  timeoutSeconds: 120,
  alertOnFailure: true,
  scheduleEnabled: true,
  scheduleCron: "0 8 * * 1-5",
  scheduleTimeZone: "UTC",
  capabilities: caps,
});
const role = fixture("RoleView", {
  id: id(9),
  name: "Operations reader",
  roleType: "Custom",
  permissions: ["Platform", "Deployment", "Stack"].map((resourceType) => ({
    resourceType,
    permissionLevel: "Read",
    specificPermissions: [],
  })),
});
const coverage = fixture("SwarmNodeAgentCoverage", {
  state: "Partial",
  isInstalled: true,
  canManageNodeAgents: true,
  totalNodes: 3,
  eligibleNodes: 3,
  coveredNodes: 2,
  connectedNodes: 1,
  offlineNodes: 1,
  nodes: [
    ["manager-01", "manager", "ManagerConnector", true],
    ["worker-01", "worker", "Satellite", true],
    ["worker-02", "worker", "Satellite", false],
  ].map(([hostname, role, dataSource, online], i) =>
    fixture("NodeAgentCoverage", {
      dockerNodeId: `demo-node-${i}`,
      hostname,
      role,
      dataSource,
      eligible: true,
      dockerReachable: online,
      projectionStale: false,
      architecture: "amd64",
      agentConnectionState: online ? "Connected" : "Offline",
      reasons: online ? [] : ["AgentOffline"],
    }),
  ),
});
const lookups = {
  Platform: [platform],
  Stack: [stack],
  GitRepository: [repo],
  Registry: [{ id: id(4), name: "Docker Hub" }],
  BackupRepository: [backupRepo],
  RunAsActor: [{ id: id(8), name: "Operations runner" }],
};
const routes = {
  "/setup/status": {
    requiresSetup: false,
    passwordMinimumLength: 15,
    passwordMaximumLength: 128,
  },
  "/profile/preferences": { theme: "light" },
  "/profile": {
    id: id(10),
    displayName: "Demo Administrator",
    email: "admin@example.com",
    teams: [],
    directRoles: [],
    authentication: { type: "Local", label: "Local" },
    authorization: {
      isAdministrator: true,
      alertRules: caps,
      bindings: caps,
      tags: caps,
    },
  },
  "/application/info": {
    name: "Citadel",
    version: "1.0.0",
    informationalVersion: "1.0.0",
    realtimeTransport: "Disabled",
  },
  "/platforms": { platforms: [platform], capabilities: caps },
  [`/platforms/${id(1)}`]: platform,
  [`/platforms/${id(1)}/node-agent-coverage`]: coverage,
  [`/platforms/${id(1)}/swarm`]: fixture("SwarmOverviewView", {
    nodeCount: 3,
    managerCount: 1,
    health: "Healthy",
  }),
  "/alertEvents": { pagedResult: { items: [], totalCount: 0 } },
  "/tags": { tags: [], capabilities: caps },
  "/license/entitlements": {
    capabilities: [
      "AdvancedAlerting",
      "AutomatedOperations",
      "CustomAccessControl",
    ].map((capability) => ({ capability, enabled: true })),
  },
  [`/stacks/${id(2)}`]: stack,
  [`/stacks/${id(2)}/_cfg`]: stack,
  [`/stacks/${id(2)}/drift`]: fixture("StackDrift", { items: [] }),
  [`/gitRepositories/${id(3)}`]: repo,
  [`/gitRepositories/${id(3)}/refs`]: {
    refs: [
      fixture("GitRepositoryRefView", {
        id: id(11),
        gitRepositoryId: id(3),
        branch: "main",
        status: "Synced",
        resolvedCommitSha: "a1b2c3d4e5f6789012345678901234567890abcdef",
      }),
    ],
  },
  "/buildProjects": { projects: [], capabilities: caps },
  "/backupRepositories": { repositories: [backupRepo], capabilities: caps },
  [`/backupPolicies/${id(6)}`]: policy,
  "/backupPolicies/platform-summaries": { summaries: [] },
  [`/stacks/${id(2)}/backup-source-preview`]: {
    stackName: "Storefront",
    platformId: id(1),
    platformName: platform.name,
    volumes: [
      {
        name: "storefront_data",
        nodeHostname: "worker-01",
        dockerNodeId: "demo-node-1",
        kind: "Named",
        isExternal: false,
        isShared: false,
        hasBackupCoverage: true,
      },
    ],
    warnings: [],
  },
  [`/automation/actions/${id(7)}`]: action,
  "/roles": { roles: [role], capabilities: caps },
};
routes["/roles/permissions/matrix"] = JSON.parse(
  await fs.readFile(
    new URL("./fixtures/guide-permissions.json", import.meta.url),
    "utf8",
  ),
);
routes[`/gitRepositories/${id(3)}/compose-projects`] = {
  projects: [
    {
      composePaths: stack.spec.composePaths,
      envFilePaths: stack.spec.composeEnvFilesFromRepo,
      workingDirectory: stack.spec.workingDirectory,
      suggestedWatchPaths: stack.spec.watchPaths,
    },
  ],
};
routes["/platforms/agent/setup"] = {};
const browser = await chromium.launch();
const context = await browser.newContext({
  viewport: { width: 1360, height: 2000 },
  deviceScaleFactor: 2,
  colorScheme: "light",
  reducedMotion: "reduce",
});
await context.addInitScript(() =>
  localStorage.setItem(
    "theme",
    JSON.stringify({ color: "blue", mode: "light" }),
  ),
);
const unknown = new Set(),
  errors = [];
await context.route("**/api/v1/**", async (route) => {
  const req = route.request();
  const url = new URL(req.url());
  const path = url.pathname.replace("/api/v1", "");
  let data = routes[path];
  if (req.method() !== "GET") {
    unknown.add(`${req.method()} ${path}`);
    return route.abort();
  }
  if (path === "/authentication/refresh")
    data = {
      accessToken: `e30.${Buffer.from(JSON.stringify({ sub: id(10), exp: 2000000000 })).toString("base64url")}.demo`,
    };
  else if (path === "/lookup")
    data = (lookups[url.searchParams.get("TargetResourceType")] ?? []).map(
      (x) => ({ id: x.id, name: x.name }),
    );
  else if (data === undefined) {
    unknown.add(path);
    return route.abort();
  }
  await route.fulfill({
    status: 200,
    contentType: "application/json",
    body: JSON.stringify(data),
  });
});
const page = await context.newPage();
page.on("pageerror", (e) => errors.push(e.message));
async function capture(name, start, end = start) {
  await start.evaluate((el) =>
    el.scrollIntoView({ block: "start", behavior: "instant" }),
  );
  await page.evaluate(() => document.fonts.ready);
  const first = await start.boundingBox(),
    last = await end.boundingBox();
  const clip = {
    x: first.x,
    y: first.y,
    width: first.width,
    height: last.y + last.height - first.y,
  };
  if (start === end)
    await start.screenshot({
      path: `${root}/docs/public/screenshots/${name}.png`,
      animations: "disabled",
    });
  else {
    expect(clip.y + clip.height).toBeLessThanOrEqual(2000);
    await page.screenshot({
      path: `${root}/docs/public/screenshots/${name}.png`,
      clip,
      animations: "disabled",
    });
  }
}
try {
  for (const [name, path] of [
    ["git", `stacks/edit/${id(2)}#config`],
    ["swarm", `platforms/edit/${id(1)}#config`],
    ["backup", `backup-policies/edit/${id(6)}#config`],
    ["automation", `automation/edit/${id(7)}#config`],
    ["roles", "access/roles"],
  ]) {
    await page.goto(`${origin}/${path}`, { waitUntil: "networkidle" });
    if (name === "git") {
      await expect(page.locator("#git_stack_source")).toContainText(
        "Application infrastructure",
      );
      await expect(
        page.locator("#git_stack_source .monaco-editor").first(),
      ).toBeVisible();
      await expect(
        page.locator("#git_stack_source .squiggly-error"),
      ).toHaveCount(0);
      await capture("git-stack-source", page.locator("#git_stack_source"));
      await capture("git-stack-watch-paths", page.locator("#git_stack_paths"));
    }
    if (name === "swarm")
      await capture(
        "swarm-node-coverage",
        page.getByRole("region", {
          name: "Cluster node coverage",
          exact: true,
        }),
      );
    if (name === "backup")
      await capture(
        "backup-policy-source-destination",
        page.locator("#source"),
        page.locator("#destination"),
      );
    if (name === "automation") {
      await expect(page.locator("#runtime")).toContainText("Operations runner");
      await capture(
        "automation-runtime-schedule",
        page.locator("#runtime"),
        page.locator("#schedule"),
      );
    }
    if (name === "roles") {
      await page.getByText("Operations reader", { exact: true }).click();
      const heading = page.getByRole("heading", {
        name: "Operations reader",
        exact: true,
      });
      const detail = heading.locator("xpath=../../..");
      await capture("role-permissions", detail, detail.getByRole("row").nth(4));
    }
    console.log(`Captured ${name}`);
  }
  expect([...unknown]).toEqual([]);
  expect(errors).toEqual([]);
  console.log(
    "All guide screenshots captured without browser errors or unmatched API requests.",
  );
} finally {
  await browser.close();
}
