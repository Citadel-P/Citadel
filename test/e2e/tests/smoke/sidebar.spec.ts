import { test, expect } from '../../support/test';

test('sidebar navigation reaches resource pages and nested settings', async ({ page }) => {
  await page.goto('/');

  await page.getByRole('link', { name: 'Stacks' }).click();
  await expect(page).toHaveURL(/\/stacks$/);
  await expect(page.getByRole('button', { name: 'Add Stack' })).toBeVisible();

  await page.getByRole('button', { name: 'Settings' }).click();
  await page.getByRole('link', { name: 'Tags' }).click();
  await expect(page).toHaveURL(/\/tags$/);
  await expect(page.getByRole('button', { name: 'Add Tag' })).toBeVisible();
});
