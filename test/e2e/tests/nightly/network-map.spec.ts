import { getAdminAccessToken } from '../../support/admin-api';
import {
  applyRuntimeStack,
  cleanupRuntimeFixture,
  createRuntimeFixture,
  type RuntimeFixture,
} from '../../support/runtime-api';
import { getRuntimeContainer, runInDockerDaemon } from '../../support/docker';
import { expect, test } from '../../support/test';

test('network sheet supports inline exploration, maximize and realtime detach without canvas mutations', async ({
  page,
  request,
}, testInfo) => {
  test.setTimeout(240_000);
  let fixture: RuntimeFixture | undefined;
  let proxy: string | undefined;
  let database: string | undefined;
  const headers = {
    Authorization: `Bearer ${await getAdminAccessToken(request)}`,
  };
  const errors: string[] = [];
  page.on('pageerror', (error) => errors.push(error.message));
  try {
    fixture = await createRuntimeFixture(request);
    await applyRuntimeStack(request, fixture.stackId);
    const app = await getRuntimeContainer(fixture.stackId);
    proxy = await runInDockerDaemon('network', 'create', `${fixture.name}-proxy`);
    await runInDockerDaemon('network', 'connect', '--alias', 'application', proxy, app.id);
    database = await runInDockerDaemon(
      'run',
      '-d',
      '--network',
      `${fixture.projectName}_default`,
      '--name',
      `${fixture.name}-db`,
      'busybox:1.36.1',
      'sleep',
      '1200',
    );
    let containerId = '';
    await expect
      .poll(
        async () => {
          const response = await request.get(`/api/v1/platforms/${fixture!.platformId}/network-containers`, {
            headers,
          });
          expect(response.ok()).toBeTruthy();
          containerId = (await response.json()).containers.find(
            (item: { dockerContainerId: string }) => item.dockerContainerId === app.id,
          )?.containerId;
          return containerId;
        },
        { timeout: 90_000 },
      )
      .toBeTruthy();
    const loaded: string[] = [];
    page.on('request', (req) => loaded.push(req.url()));
    await page.goto(`/platforms/${fixture.platformId}/networks?view=list`);
    await expect(page.getByRole('table')).toBeVisible();
    expect(loaded.some((url) => /network-map-.*\.(js|css)|elk-worker/.test(url))).toBe(false);
    const trigger = page.getByRole('button', {
      name: `Show network details for ${fixture.name}-proxy`,
      exact: true,
    });
    await trigger.click();
    await expect(page.getByRole('dialog')).toBeVisible();
    await expect(page.getByRole('button', { name: 'View on map' })).toHaveCount(0);
    await page.getByRole('button', { name: `Expand ${fixture.name}`, exact: true }).click();
    await expect(page.locator('.react-flow__node')).toHaveCount(3);
    await expect
      .poll(() =>
        page.locator('.network-map').evaluate((host) => {
          const viewport = host.getBoundingClientRect();
          return [...host.querySelectorAll('.react-flow__node')].every((node) => {
            const rect = node.getBoundingClientRect();
            return rect.top >= viewport.top - 1 && rect.bottom <= viewport.bottom + 1;
          });
        }),
      )
      .toBe(true);
    await page
      .getByRole('button', {
        name: `Show details for ${app.name}`,
        exact: true,
      })
      .click();
    await expect(page.getByText('Aliases: application', { exact: false })).toBeVisible();
    await page.screenshot({
      path: testInfo.outputPath('network-sheet-light.png'),
      fullPage: true,
      animations: 'disabled',
    });
    await page.getByRole('button', { name: 'Explore connections', exact: true }).click();
    const canvas = page.locator('.react-flow');
    await expect(canvas.locator('.react-flow__node')).toHaveCount(3);
    await expect(canvas.locator('.react-flow__edge')).toHaveCount(2);
    await expect(page.getByRole('link', { name: fixture.name, exact: true })).toBeVisible();
    await page.getByRole('button', { name: 'Maximize network sheet', exact: true }).click();
    await expect(page.getByRole('button', { name: 'Restore network sheet' })).toBeVisible();
    await expect(canvas.locator('.react-flow__node')).toHaveCount(3);
    await expect(page.getByText('Aliases: application', { exact: false })).toBeVisible();
    await page.getByRole('button', { name: 'Fit View', exact: true }).click();
    await page.screenshot({
      path: testInfo.outputPath('network-map-light.png'),
      fullPage: true,
      animations: 'disabled',
    });
    await page.evaluate(() => document.documentElement.classList.add('dark'));
    await expect(page.getByRole('dialog')).toHaveCSS('background-color', 'rgb(10, 10, 10)');
    await page.screenshot({
      path: testInfo.outputPath('network-map-dark.png'),
      fullPage: true,
      animations: 'disabled',
    });
    await page.getByRole('button', { name: 'Restore network sheet' }).click();
    await expect(canvas.locator('.react-flow__node')).toHaveCount(3);
    await page.setViewportSize({ width: 390, height: 844 });
    await expect.poll(() => page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
    await page.screenshot({
      path: testInfo.outputPath('network-map-mobile.png'),
      fullPage: true,
      animations: 'disabled',
    });
    await page.keyboard.press('Delete');
    await expect(canvas.locator('.react-flow__node')).toHaveCount(3);
    await runInDockerDaemon('network', 'disconnect', proxy, app.id);
    await expect(canvas.locator('.react-flow__node')).toHaveCount(2, {
      timeout: 90_000,
    });
    await expect(canvas.locator('.react-flow__edge')).toHaveCount(1);
    await runInDockerDaemon('stop', '-t', '0', app.id);
    await expect(canvas.locator('.react-flow__node')).toHaveCount(1, {
      timeout: 90_000,
    });
    await expect(canvas.locator('.react-flow__edge')).toHaveCount(0);
    await expect(page.getByText('Configured network · Not attached', { exact: true })).toBeVisible();
    await page.setViewportSize({ width: 1280, height: 720 });
    await page.getByRole('button', { name: 'Maximize network sheet' }).click();
    await page.keyboard.press('Escape');
    await expect(page.getByRole('dialog')).toBeVisible();
    await expect(page.getByRole('button', { name: 'Maximize network sheet' })).toBeVisible();
    await page.keyboard.press('Escape');
    await expect(page.getByRole('dialog')).toHaveCount(0);
    await expect(trigger).toBeFocused();
    expect(errors).toEqual([]);
  } finally {
    if (database) await runInDockerDaemon('rm', '-f', database);
    if (fixture) await cleanupRuntimeFixture(request, fixture);
    if (proxy) await runInDockerDaemon('network', 'rm', proxy);
  }
});
