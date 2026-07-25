import {
  applyRuntimeStack,
  cleanupRuntimeFixture,
  createRuntimeFixture,
  getRuntimeStack,
  getRuntimeStackReleases,
  RuntimeFixture,
  updateRuntimeStack,
} from '../../support/runtime-api';
import { getRuntimeContainer, getRuntimeContainerLogs } from '../../support/docker';
import { expect, test } from '../../support/test';

test('a stack can deploy a second release and roll back to the running first release', async ({
  page,
  request,
}) => {
  test.setTimeout(180_000);
  let fixture: RuntimeFixture | undefined;

  try {
    fixture = await createRuntimeFixture(request);
    await applyRuntimeStack(request, fixture.stackId);

    const firstStack = await getRuntimeStack(request, fixture.stackId);
    expect(firstStack).toMatchObject({
      status: 'Healthy',
      version: '1',
    });
    expect(firstStack.spec?.composeFile).toContain('citadel-e2e-release-one');

    const firstContainer = await getRuntimeContainer(fixture.stackId);
    expect(firstContainer.running).toBe(true);
    expect(firstContainer.image).toBe('busybox:1.36.1');
    expect(firstContainer.labels).toMatchObject({
      'citadel.e2e.release': 'one',
      'com.citadel.stack-id': fixture.stackId,
      'com.citadel.release-id': firstStack.currentStackReleaseId,
    });
    expect(await getRuntimeContainerLogs(firstContainer.id)).toContain(
      'citadel-e2e-release-one',
    );

    await updateRuntimeStack(request, fixture, 'two');
    await applyRuntimeStack(request, fixture.stackId);

    const secondStack = await getRuntimeStack(request, fixture.stackId);
    expect(secondStack).toMatchObject({
      status: 'Healthy',
      version: '2',
    });
    expect(secondStack.spec?.composeFile).toContain('citadel-e2e-release-two');

    const releases = await getRuntimeStackReleases(request, fixture.stackId);
    const firstRelease = releases.find((release) => release.version === '1');
    expect(firstRelease).toMatchObject({ status: 'Healthy' });
    expect(firstRelease?.spec.composeFile).toContain('citadel-e2e-release-one');

    const secondContainer = await getRuntimeContainer(fixture.stackId);
    expect(secondContainer.running).toBe(true);
    expect(secondContainer.labels).toMatchObject({
      'citadel.e2e.release': 'two',
      'com.citadel.release-id': secondStack.currentStackReleaseId,
    });
    expect(await getRuntimeContainerLogs(secondContainer.id)).toContain(
      'citadel-e2e-release-two',
    );

    await page.goto(`/stacks/edit/${fixture.stackId}`);
    await page.getByRole('tab', { name: 'Releases' }).click();

    const releaseRow = page.getByRole('row').filter({ hasText: 'v1' });
    await expect(releaseRow).toBeVisible();
    await releaseRow.getByRole('button', { name: 'Rollback' }).click();

    const confirmDialog = page.getByRole('dialog', { name: 'Confirm Rollback' });
    await confirmDialog.getByRole('textbox').fill(fixture.name);
    const rollbackResponse = page.waitForResponse(
      (response) =>
        response.url().endsWith('/api/v1/stacks/rollback') &&
        response.request().method() === 'POST',
    );
    await confirmDialog.getByRole('button', { name: 'Rollback' }).click();
    const response = await rollbackResponse;
    expect(response.status()).toBe(200);
    await response.finished();

    await expect
      .poll(async () => getRuntimeStack(request, fixture!.stackId), {
        timeout: 60_000,
      })
      .toMatchObject({
        status: 'Healthy',
        version: '3',
      });

    const rolledBackStack = await getRuntimeStack(request, fixture.stackId);
    expect(rolledBackStack.spec?.composeFile).toContain('citadel-e2e-release-one');
    expect((await getRuntimeStackReleases(request, fixture.stackId)).map((release) => release.version)).toEqual(
      expect.arrayContaining(['1', '2']),
    );

    const rolledBackContainer = await getRuntimeContainer(fixture.stackId);
    expect(rolledBackContainer.running).toBe(true);
    expect(rolledBackContainer.labels).toMatchObject({
      'citadel.e2e.release': 'one',
      'com.citadel.release-id': rolledBackStack.currentStackReleaseId,
    });
    expect(await getRuntimeContainerLogs(rolledBackContainer.id)).toContain(
      'citadel-e2e-release-one',
    );
  } finally {
    if (fixture) {
      await cleanupRuntimeFixture(request, fixture);
    }
  }
});
