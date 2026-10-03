import {
  applyRuntimeStack,
  cleanupRuntimeFixture,
  createRuntimeFixture,
  RuntimeFixture,
} from '../../support/runtime-api';
import {
  emitRuntimeContainerLog,
  getRuntimeContainer,
} from '../../support/docker';
import { expect, test } from '../../support/test';

test('stack logs survive runtime tab changes and reconnect after a network interruption', async ({
  context,
  page,
  request,
}) => {
  test.setTimeout(180_000);
  let fixture: RuntimeFixture | undefined;

  try {
    fixture = await createRuntimeFixture(request);
    await applyRuntimeStack(request, fixture.stackId);
    const container = await getRuntimeContainer(fixture.stackId);

    await page.goto(`/stacks/edit/${fixture.stackId}`);
    await page.getByRole('tab', { name: 'Services' }).click();

    const logsTab = page.getByRole('tab', { name: 'Logs', exact: true });
    const inspectTab = page.getByRole('tab', { name: 'Inspect', exact: true });
    const terminalTab = page.getByRole('tab', { name: 'Terminal', exact: true });
    const initialLog = page.getByText('citadel-e2e-release-one', { exact: false }).first();
    await expect(initialLog).toBeVisible();

    const inspectResponse = page.waitForResponse(
      (response) =>
        response.url().includes(`/api/v1/stacks/${fixture!.stackId}/containers/`) &&
        response.url().endsWith('/inspect'),
    );
    await inspectTab.click();
    expect((await inspectResponse).status()).toBe(200);
    await expect(page.locator('.monaco-editor')).toBeVisible();

    await logsTab.click();
    await expect(initialLog).toBeVisible();

    await terminalTab.click();
    const shellSelector = page
      .getByRole('combobox')
      .filter({ hasText: 'bash' });
    await shellSelector.click();
    await page.getByRole('option', { name: 'sh', exact: true }).click();
    await page.getByRole('button', { name: 'Connect' }).click();
    await expect(page.getByRole('button', { name: 'Disconnect' })).toBeVisible();

    const terminalMarker = `terminal-${Date.now()}`;
    const terminalInput = page.locator('.xterm-helper-textarea');
    await terminalInput.fill(`echo ${terminalMarker}`);
    await terminalInput.press('Enter');
    await expect(page.locator('.xterm-rows')).toContainText(terminalMarker);
    await page.getByRole('button', { name: 'Disconnect' }).click();

    await logsTab.click();
    await expect(initialLog).toBeVisible();

    await context.setOffline(true);
    await page.waitForTimeout(1_500);
    await context.setOffline(false);

    const reconnectMarker = `reconnected-${Date.now()}`;
    await expect
      .poll(
        async () => {
          await emitRuntimeContainerLog(container.id, reconnectMarker);
          return page.locator('body').innerText();
        },
        {
          intervals: [1_000, 2_000, 3_000],
          timeout: 30_000,
        },
      )
      .toContain(reconnectMarker);
  } finally {
    if (fixture) {
      await cleanupRuntimeFixture(request, fixture);
    }
  }
});
