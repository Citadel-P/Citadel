import { deleteTagsByPrefix } from '../../support/admin-api';
import { test, expect } from '../../support/test';

test('an administrator can create, edit, and delete a tag', async ({ page, request }) => {
  const prefix = `e2e-${Date.now()}`;
  const initialName = `${prefix}-initial`;
  const updatedName = `${prefix}-updated`;

  await deleteTagsByPrefix(request, prefix);

  try {
    await page.goto('/tags');
    await page.getByRole('button', { name: 'Add Tag' }).click();

    const createDialog = page.getByRole('dialog', { name: 'Create Tag' });
    await createDialog.getByLabel('Name').fill(initialName);
    await createDialog.getByRole('button', { name: 'Save' }).click();
    await expect(page.getByText(initialName, { exact: true })).toBeVisible();

    let row = page.getByRole('row').filter({ hasText: initialName });
    await row.getByRole('button', { name: 'Open menu' }).click();
    await page.getByRole('menuitem', { name: 'Edit' }).click();

    const editDialog = page.getByRole('dialog', { name: 'Edit Tag' });
    await editDialog.getByLabel('Name').fill(updatedName);
    await editDialog.getByRole('button', { name: 'Save' }).click();
    await expect(page.getByText(updatedName, { exact: true })).toBeVisible();
    await expect(page.getByText(initialName, { exact: true })).not.toBeVisible();

    row = page.getByRole('row').filter({ hasText: updatedName });
    await row.getByRole('button', { name: 'Open menu' }).click();
    await page.getByRole('menuitem', { name: 'Delete' }).click();

    const confirmDialog = page.getByRole('dialog', { name: 'Confirm Delete' });
    await confirmDialog.getByRole('textbox').fill(updatedName);
    await confirmDialog.getByRole('button', { name: 'Delete' }).click();
    await expect(confirmDialog).not.toBeVisible();
    await expect(row).toHaveCount(0);
  } finally {
    await deleteTagsByPrefix(request, prefix);
  }
});
