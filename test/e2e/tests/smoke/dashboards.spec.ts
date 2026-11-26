import { randomUUID } from 'node:crypto';
import { getAdminAccessToken } from '../../support/admin-api';
import { expect, test } from '../../support/test';

test('dropping a widget creates one draft tile and saves once', async ({ page, request }) => {
  const id = randomUUID();
  const path = '/api/v1/dashboards/' + id;
  const headers = {
    Authorization: 'Bearer ' + (await getAdminAccessToken(request)),
  };
  const created = await request.post('/api/v1/dashboards', {
    headers,
    data: {
      id,
      name: 'Builder ' + id,
      visibility: 'Private',
      configuration: {
        schemaVersion: 2,
        tags: [],
        rangeMinutes: 60,
        widgets: [],
      },
    },
  });
  expect(created.ok()).toBeTruthy();
  try {
    let saves = 0;
    page.on('request', (r) => {
      if (r.method() === 'PUT' && new URL(r.url()).pathname === path) saves++;
    });
    await page.goto('/dashboards/' + id);
    await page.getByRole('button', { name: 'Edit dashboard' }).click();
    await page.getByRole('textbox', { name: 'Find widget' }).fill('Notes');
    const add = page.getByRole('button', { name: 'Add Notes and links' });
    const grid = page.locator('.dashboard-editor-grid');
    await add.locator('..').dragTo(grid, { targetPosition: { x: 45, y: 45 } });
    await expect(page.getByLabel('Widget title')).toHaveValue('Notes and links');
    await expect(grid.locator('.react-grid-item')).toHaveCount(1);
    await page.getByRole('button', { name: 'Cancel settings' }).click();
    await expect(grid.locator('.react-grid-item')).toHaveCount(0);
    await add.click();
    await page.getByLabel('Widget title').fill('Operator contacts');
    expect(saves).toBe(0);
    await page.getByRole('button', { name: 'Save changes' }).click();
    await expect(page.getByRole('region', { name: 'Operator contacts' })).toBeVisible();
    expect(saves).toBe(1);
    await page.reload();
    await expect(page.getByRole('region', { name: 'Operator contacts' })).toBeVisible();
  } finally {
    const saved = await request.get(path, { headers });
    if (saved.ok()) {
      const { summary } = await saved.json();
      expect(
        (
          await request.delete(path, {
            headers,
            params: { revision: summary.revision },
          })
        ).ok(),
      ).toBeTruthy();
    }
  }
});
