import { test, expect } from '../../support/test';
import { copyApi, createCopyFixture, CopyProgress } from '../../support/copy-api';
import { clearCopyFixtureHistory, controlRuntimeService, runInDockerDaemon } from '../../support/docker';
import { applyRuntimeStack, getRuntimeStack } from '../../support/runtime-api';

test('snapshot copy works through the browser and a Core crash cannot finalize an interrupted copy', async ({ page, request }) => {
  test.setTimeout(360_000);
  const fixture = await createCopyFixture(request);
  const targets = new Set<string>();
  const destinations: string[] = [];
  let daemonPaused = false;
  let coreStopped = false;
  try {
    await page.goto(`/stacks/edit/${fixture.stackId}/`);
    await page.getByRole('button', { name: 'Duplicate', exact: true }).click();
    await page.getByRole('menuitem', { name: /Configuration \+ data/ }).click();
    const dialog = page.getByRole('dialog', { name: 'Duplicate with data' });
    await dialog.getByLabel('Name', { exact: true }).fill(`${fixture.name}-copy`);
    await dialog.getByRole('combobox', { name: 'Data source' }).click();
    await page.getByRole('option', { name: 'Use an existing backup', exact: true }).click();
    await dialog.getByRole('button', { name: 'Choose backups' }).click();
    await dialog.getByRole('combobox').click();
    await page.getByRole('option', { name: new RegExp(fixture.name) }).click();
    await dialog.getByRole('button', { name: 'Review copy' }).click();
    await expect(dialog.getByRole('button', { name: 'Create copy' })).toBeDisabled();
    await dialog.getByRole('checkbox').check();
    const submitted = page.waitForResponse(response => response.request().method() === 'POST' && /\/resource-copies\/[^/]+\/start$/.test(new URL(response.url()).pathname));
    await dialog.getByRole('button', { name: 'Create copy' }).click();
    expect((await submitted).ok()).toBe(true);
    await expect(page).toHaveURL(/copyOperation=/);
    const id = new URL(page.url()).searchParams.get('copyOperation')!;
    await page.reload();
    let completed: CopyProgress;
    await expect.poll(async () => {
      completed = await copyApi(request, 'get', `resource-copies/${id}`);
      completed.restores.forEach(r => targets.add(r.target));
      return completed.status;
    }, { timeout: 120_000 }).toBe('Succeeded');
    destinations.push(completed!.destinationId!);
    await expect(dialog.getByRole('link', { name: 'Open stack' })).toBeVisible();
    const destination = await getRuntimeStack(request, completed!.destinationId!);
    expect(destination.spec?.composeFile).toContain(completed!.restores[0].target);
    expect(await runInDockerDaemon('ps', '-aq', '--filter', `label=com.citadel.stack-id=${destination.id}`)).toBe('');
    await applyRuntimeStack(request, destination.id);
    expect(await runInDockerDaemon('run', '--rm', '-v', `${completed!.restores[0].target}:/data`, 'busybox:1.36.1', 'sh', '-ec', 'cat /data/payload; printf independent > /data/copy-only')).toBe('copy-fixture');
    expect(await runInDockerDaemon('run', '--rm', '-v', `${fixture.source}:/data:ro`, 'busybox:1.36.1', 'sh', '-ec', 'test ! -e /data/copy-only; cat /data/payload')).toBe('copy-fixture');

    const draft = { kind: 'Stack', sourceId: fixture.stackId, name: `${fixture.name}-crash`, selections: [] };
    const choices = await copyApi(request, 'post', 'resource-copies/preview', draft);
    const selection = choices.volumes[0].snapshots[0];
    const review = await copyApi(request, 'post', 'resource-copies/preview', { ...draft, selections: [{ volume: fixture.source, backupRunId: selection.backupRunId, backupRunItemId: selection.backupRunItemId }] });
    // Pause only the nested daemon, after review. Core can durably claim the
    // restore but cannot complete its runtime work before it is killed.
    await controlRuntimeService('pause', 'docker'); daemonPaused = true;
    const started: CopyProgress = await copyApi(request, 'post', `resource-copies/${review.id}/start`);
    started.restores.forEach(r => targets.add(r.target));
    await expect.poll(async () => (await copyApi(request, 'get', `resource-copies/${review.id}`)).restores[0].status, { timeout: 30_000 }).toMatch(/Preparing|Running/);
    await controlRuntimeService('kill', 'citadel'); coreStopped = true;
    await controlRuntimeService('unpause', 'docker'); daemonPaused = false;
    await controlRuntimeService('start', 'citadel'); coreStopped = false;
    await expect.poll(async () => {
      try { return (await request.get('/health', { maxRetries: 2 })).ok(); } catch { return false; }
    }, { timeout: 60_000 }).toBe(true);
    await expect.poll(async () => (await copyApi(request, 'get', `resource-copies/${review.id}`)).status, { timeout: 60_000 }).toBe('Failed');
    const stopped: CopyProgress = await copyApi(request, 'post', `resource-copies/${review.id}/start`);
    expect(stopped.destinationId).toBeNull();
    expect(stopped.restores.map(r => r.id)).toEqual(started.restores.map(r => r.id));
    expect(stopped.restores[0].status).toBe('Interrupted');
    await copyApi(request, 'post', `resource-copies/${review.id}/cleanup`);
    await expect.poll(async () => (await copyApi(request, 'get', `resource-copies/${review.id}`)).status, { timeout: 30_000 }).toBe('Cleaned');
    expect(await runInDockerDaemon('run', '--rm', '-v', `${fixture.source}:/data:ro`, 'busybox:1.36.1', 'cat', '/data/payload')).toBe('copy-fixture');
  } finally {
    if (daemonPaused) await controlRuntimeService('unpause', 'docker');
    if (coreStopped) await controlRuntimeService('start', 'citadel');
    await expect.poll(async () => {
      try { return (await request.get('/health', { maxRetries: 2 })).ok(); } catch { return false; }
    }, { timeout: 60_000 }).toBe(true);
    await copyApi(request, 'delete', 'stacks', [...destinations, fixture.stackId]);
    await copyApi(request, 'delete', `backupPolicies/${fixture.policyId}`);
    await copyApi(request, 'delete', `backupRepositories/${fixture.repositoryId}`);
    await clearCopyFixtureHistory(fixture.stackId, fixture.policyId);
    await copyApi(request, 'delete', 'platforms', { ids: [fixture.platformId] });
    // All names are recorded from this test's fixture or operation receipts.
    const existing = (await runInDockerDaemon('volume', 'ls', '-q')).split(/\r?\n/);
    for (const volume of [...targets, fixture.source, fixture.repositoryVolume]) {
      if (existing.includes(volume)) await runInDockerDaemon('volume', 'rm', volume);
    }
  }
});
