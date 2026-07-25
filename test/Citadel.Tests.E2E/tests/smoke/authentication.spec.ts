import { test, expect } from '../../support/test';

test.use({ storageState: { cookies: [], origins: [] } });

test('local login, reload, and tab refocus preserve the requested route', async ({ page, context }) => {
  await page.goto('/stacks?source=e2e#active');

  await expect(page.getByRole('heading', { name: 'Sign in to your account' })).toBeVisible();
  await page.getByLabel('Email address or username').fill('admin@citadel.local');
  await page.getByLabel('Password').fill('admin123');
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
