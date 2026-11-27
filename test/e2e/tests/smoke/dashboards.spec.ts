import { randomUUID } from 'node:crypto';
import { getAdminAccessToken } from '../../support/admin-api';
import { expect, test } from '../../support/test';

test('adds one draft widget and moves the saved widget from its card body', async ({ page, request }) => {
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
    await page.getByRole('button', { name: 'Undo' }).click();
    await expect(grid.locator('.react-grid-item')).toHaveCount(0);
    await add.click();
    await page.getByLabel('Widget title').fill('Operator contacts');
    expect(saves).toBe(0);
    await page.getByRole('button', { name: 'Save changes' }).click();
    await expect(page.getByRole('button', { name: 'Edit dashboard' })).toBeVisible();
    await expect(page.getByRole('region', { name: 'Operator contacts', exact: true })).toBeVisible();
    expect(saves).toBe(1);
    await page.reload();
    await expect(page.getByRole('region', { name: 'Operator contacts', exact: true })).toBeVisible();

    await page.getByRole('button', { name: 'Edit dashboard' }).click();
    await expect(page.getByRole('heading', { name: 'Widget library' })).toBeVisible();
    await page.getByRole('button', { name: 'Builder ' + id }).click();
    const title = page.getByRole('textbox', { name: 'Dashboard name' });
    await title.fill('Cancelled title');
    await title.press('Escape');
    await expect(page.getByRole('button', { name: 'Builder ' + id })).toBeVisible();
    await expect(page.getByRole('button', { name: 'Save changes' })).toBeDisabled();
    const tile = grid.locator('.dashboard-edit-tile');
    const card = tile.locator('[data-widget="Notes"]');
    await expect(card).toBeInViewport();
    const body = grid.getByRole('button', { name: 'Operator contacts' });
    await expect(body).toHaveCSS('cursor', 'grab');
    const box = (await body.boundingBox())!;
    const start = (await tile.boundingBox())!;
    const step = await grid.evaluate((el) => parseFloat(getComputedStyle(el).getPropertyValue('--grid-column-step')));
    await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
    await page.mouse.down();
    await page.mouse.move(box.x + box.width / 2 + 2 * step, box.y + box.height / 2, { steps: 12 });
    // The highlighted background cells must be the cells the saved widget actually occupies.
    const destination = await grid.evaluate((el) => {
      const canvas = el.getBoundingClientRect();
      const target = getComputedStyle(el, '::after');
      return {
        x: canvas.x + parseFloat(target.left),
        y: canvas.y + parseFloat(target.top),
        width: parseFloat(target.width),
      };
    });
    expect(Math.abs(destination.x - (start.x + 2 * step))).toBeLessThanOrEqual(1);
    await page.mouse.up();
    await expect.poll(async () => Math.round((await tile.boundingBox())!.x)).toBe(Math.round(destination.x));
    const dropped = (await tile.boundingBox())!;
    expect(dropped.y).toBeCloseTo(destination.y, 0);
    expect(dropped.width).toBeCloseTo(destination.width, 0);
    await expect(page.getByRole('heading', { name: 'Widget library' })).toBeVisible();
    await page.getByRole('button', { name: 'Builder ' + id }).click();
    await title.fill('Operations ' + id);
    await page.getByRole('button', { name: 'Save changes' }).click();
    await expect(page.getByRole('button', { name: 'Edit dashboard' })).toBeVisible();
    const moved = await request.get(path, { headers });
    const updated = await moved.json();
    expect(updated.configuration.widgets[0].layout.x).toBe(2);
    expect(updated.summary.name).toBe('Operations ' + id);
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
