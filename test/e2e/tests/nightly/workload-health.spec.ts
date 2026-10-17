import { getAdminAccessToken } from '../../support/admin-api';
import {
  applyRuntimeStack,
  cleanupRuntimeFixture,
  createRuntimeFixture,
  type RuntimeFixture,
} from '../../support/runtime-api';
import { runInDockerDaemon } from '../../support/docker';
import { expect, test } from '../../support/test';

test('a deployment without a probe opens disabled health details from its container table', async ({
  page,
  request,
}) => {
  test.setTimeout(180_000);
  let fixture: RuntimeFixture | undefined;
  let deploymentId: string | undefined;
  const headers = {
    Authorization: `Bearer ${await getAdminAccessToken(request)}`,
  };
  try {
    fixture = await createRuntimeFixture(request);
    await runInDockerDaemon('pull', 'busybox:1.36.1');
    const dockerImageId = await runInDockerDaemon(
      'image',
      'inspect',
      '--format',
      '{{.Id}}',
      'busybox:1.36.1',
    );
    let localImageId: string | undefined;
    await expect
      .poll(
        async () => {
          const response = await request.get(
            `/api/v1/images/${fixture!.platformId}`,
            { headers },
          );
          expect(response.ok()).toBeTruthy();
          const inventory = await response.json();
          localImageId = inventory.images.find(
            (image: { dockerImageId: string }) =>
              image.dockerImageId === dockerImageId,
          )?.id;
          return localImageId;
        },
        { timeout: 90_000 },
      )
      .toBeTruthy();
    const created = await request.post('/api/v1/deployments', {
      headers,
      data: {
        name: `${fixture.name}-deployment`,
        platformId: fixture.platformId,
        spec: {
          image: { $type: 'Local', imageId: localImageId },
          command: ['sleep', '1200'],
          networks: ['bridge'],
          updateBehavior: 'Disabled',
        },
      },
    });
    expect(created.ok()).toBeTruthy();
    deploymentId = (await created.json()).id;
    const applied = await request.post('/api/v1/deployments/apply', {
      headers,
      data: { id: deploymentId, recreate: false },
      timeout: 120_000,
    });
    expect(applied.ok()).toBeTruthy();
    const progress = await applied.json();
    expect(progress.length).toBeGreaterThan(0);
    expect(
      progress.filter(
        (event: { error?: unknown; errorMessage?: string }) =>
          event.error || event.errorMessage,
      ),
    ).toEqual([]);
    await expect
      .poll(
        async () => {
          const response = await request.get(
            `/api/v1/deployments/${deploymentId}`,
            { headers },
          );
          return (await response.json()).probeHealth.status;
        },
        { timeout: 90_000 },
      )
      .toBe('NoCheck');
    await page.goto(`/deployments/edit/${deploymentId}/#container`);
    await page.getByRole('tab', { name: 'Container', exact: true }).click();
    await page
      .getByRole('table')
      .getByRole('button', { name: 'Health: No check', exact: true })
      .click();
    await page.getByRole('button', { name: 'View details' }).click();
    const dialog = page.getByRole('dialog', { name: 'Health details' });
    await expect(dialog).toBeVisible();
    await expect(
      dialog.getByRole('figure', { name: 'Health history' }),
    ).toHaveAttribute('aria-disabled', 'true');
    await expect(
      dialog.getByRole('button', { name: 'Probe details' }),
    ).toHaveCount(0);
  } finally {
    if (deploymentId) {
      const removed = await request.delete('/api/v1/deployments', {
        headers,
        data: [deploymentId],
      });
      expect(removed.ok()).toBeTruthy();
    }
    if (fixture) await cleanupRuntimeFixture(request, fixture);
  }
});

test('workload health details follow real probe changes without restarting or fetching chart ranges', async ({
  page,
  request,
}) => {
  test.setTimeout(240_000);
  let fixture: RuntimeFixture | undefined;
  try {
    fixture = await createRuntimeFixture(request);
    const headers = {
      Authorization: `Bearer ${await getAdminAccessToken(request)}`,
    };
    const composeFile = `services:
  checked:
    image: busybox:1.36.1
    command: ['sleep', '1200']
    healthcheck:
      test: ['CMD-SHELL', 'test ! -e /tmp/unhealthy']
      interval: 1s
      timeout: 1s
      retries: 1
    stop_grace_period: 1s
  unchecked:
    image: busybox:1.36.1
    command: ['sleep', '1200']
    stop_grace_period: 1s
`;
    const updated = await request.patch(`/api/v1/stacks/${fixture.stackId}`, {
      headers,
      data: {
        spec: {
          $type: 'WebEditor',
          composeFile,
          projectName: fixture.projectName,
          updateBehavior: 'Disabled',
          destroyBeforeDeploy: true,
          buildImageBindings: [],
        },
      },
    });
    expect(updated.ok()).toBeTruthy();
    await applyRuntimeStack(request, fixture.stackId);
    const assessment = async () => {
      const response = await request.get(`/api/v1/stacks/${fixture!.stackId}`, {
        headers,
      });
      expect(response.ok()).toBeTruthy();
      return (await response.json()).probeHealth;
    };
    await expect
      .poll(async () => (await assessment()).counts, { timeout: 90_000 })
      .toMatchObject({ healthy: 1, noCheck: 1 });
    const dockerId = await runInDockerDaemon(
      'ps',
      '-q',
      '--filter',
      `label=com.citadel.stack-id=${fixture.stackId}`,
      '--filter',
      'label=com.docker.compose.service=checked',
    );
    expect(dockerId).not.toContain('\n');
    const initial = JSON.parse(await runInDockerDaemon('inspect', dockerId))[0];
    const healthRequests: string[] = [];
    page.on('request', (req) => {
      if (/health-history|\/inspect(?:\?|$)|\/exec(?:\?|$)/.test(req.url()))
        healthRequests.push(req.url());
    });
    await page.goto(`/stacks/edit/${fixture.stackId}`);
    await page
      .getByRole('button', { name: /^Health:/ })
      .first()
      .click();
    await page.getByRole('button', { name: 'View details' }).click();
    const dialog = page.getByRole('dialog', { name: 'Health details' });
    await expect(dialog).toBeVisible();
    const selectContainer = async (name: string) => {
      await dialog
        .getByRole('combobox', { name: 'Container', exact: true })
        .click();
      await page.getByRole('option', { name: new RegExp(name) }).click();
    };
    await selectContainer('-unchecked-');
    await expect(
      dialog.getByRole('figure', { name: 'Health history' }),
    ).toHaveAttribute('aria-disabled', 'true');
    await expect(
      dialog.getByRole('button', { name: 'Probe details' }),
    ).toHaveCount(0);
    await selectContainer('-checked-');
    await expect(
      dialog.getByRole('combobox', { name: 'Time range' }),
    ).toBeVisible();
    await dialog.getByRole('combobox', { name: 'Time range' }).click();
    await page.getByRole('option', { name: '6 hours', exact: true }).click();
    await runInDockerDaemon('exec', dockerId, 'touch', '/tmp/unhealthy');
    await expect
      .poll(async () => (await assessment()).status, { timeout: 90_000 })
      .toBe('Unhealthy');
    await expect(
      dialog
        .getByRole('region', { name: 'Current health' })
        .getByText('Unhealthy', { exact: true }),
    ).toBeVisible({ timeout: 30_000 });
    await runInDockerDaemon('exec', dockerId, 'rm', '/tmp/unhealthy');
    await expect(
      dialog
        .getByRole('region', { name: 'Current health' })
        .getByText('Healthy', { exact: true }),
    ).toBeVisible({ timeout: 90_000 });
    await page.setViewportSize({ width: 390, height: 844 });
    const bounds = await dialog.boundingBox();
    expect(bounds).not.toBeNull();
    expect(bounds!.x).toBeGreaterThanOrEqual(0);
    expect(bounds!.width).toBeLessThanOrEqual(390);
    expect(healthRequests).toEqual([]);
    const after = JSON.parse(await runInDockerDaemon('inspect', dockerId))[0];
    expect(after.State.StartedAt).toBe(initial.State.StartedAt);
    expect(after.RestartCount).toBe(initial.RestartCount);
    expect(after.State.Running).toBe(true);
  } finally {
    if (fixture) await cleanupRuntimeFixture(request, fixture);
  }
});
