import { test, expect } from '../../support/test';
import { adminCredentials } from '../../support/admin-credentials';

test.use({ storageState: { cookies: [], origins: [] } });

test('local login, reload, and tab refocus preserve the requested route', async ({ page, context }) => {
  await page.goto('/stacks?source=e2e#active');

  await expect(page.getByRole('heading', { name: 'Sign in', exact: true })).toBeVisible();
  await page.getByLabel('Email address or username').fill(adminCredentials.email);
  await page.getByLabel('Password', { exact: true }).fill(adminCredentials.password);
  await page.getByRole('button', { name: 'Sign in' }).click();

  await expect(page).toHaveURL(/\/stacks\?source=e2e#active$/);
  await expect(page.getByRole('button', { name: 'Add Stack' })).toBeVisible();

  await page.reload();
  await expect(page).toHaveURL(/\/stacks\?source=e2e#active$/);
  await expect(page.getByRole('button', { name: 'Add Stack' })).toBeVisible();

  const otherTab = await context.newPage();
  await otherTab.goto('about:blank');
  await page.bringToFront();

  await expect(page).toHaveURL(/\/stacks\?source=e2e#active$/);
  await expect(page.getByRole('button', { name: 'Add Stack' })).toBeVisible();
  await otherTab.close();
});
