import { deleteSecretProvidersByPrefix } from '../../support/admin-api';
import { test, expect } from '../../support/test';

const providerPrefix = 'vault-e2e-';
const vaultAddress = process.env.CITADEL_E2E_VAULT_ADDRESS ?? 'http://vault:8200';
const vaultToken =
  process.env.CITADEL_E2E_VAULT_TOKEN ?? 'citadel-vault-acceptance-root-token';

test('Vault provider can be tested, saved, and edited without replacing its token', async ({
  page,
  request,
}) => {
  const providerName =
    `${providerPrefix}${Date.now()}-${Math.random().toString(16).slice(2, 10)}`;

  try {
    await page.goto('/bindings');
    const createProviderButton = page
      .getByRole('button', { name: 'Create Vault Provider' })
      .first();
    await expect(createProviderButton).toBeVisible();
    await createProviderButton.click();
    const createDialog = page.getByRole('dialog', { name: 'Create Vault Provider' });
    await createDialog.getByPlaceholder('Production Vault').fill(providerName);
    await createDialog.getByPlaceholder('https://vault.example.com').fill(vaultAddress);
    await createDialog.getByPlaceholder('secret').fill('secret');
    await createDialog.getByPlaceholder('Vault token').fill(vaultToken);

    await createDialog.getByRole('button', { name: 'Test Connection' }).click();
    await expect(
      page.getByText(
        'Connection successful. Vault is reachable and the token is valid.',
        { exact: true },
      ),
    ).toBeVisible();

    await createDialog.getByRole('button', { name: 'Save' }).click();
    await expect(page.getByText('Secret provider created', { exact: true })).toBeVisible();

    const providerCard = page.getByRole('button').filter({ hasText: providerName });
    await expect(providerCard).toBeVisible();
    await providerCard.click();

    const editDialog = page.getByRole('dialog', { name: 'Edit Vault Provider' });
    await expect(editDialog.getByPlaceholder('Vault token')).toHaveValue('');
    await editDialog.getByRole('button', { name: 'Save' }).click();
    await expect(page.getByText('Secret provider updated', { exact: true })).toBeVisible();

    await providerCard.click();
    await editDialog.getByRole('button', { name: 'Test Stored Token' }).click();
    await expect(
      page.getByText(
        'Connection successful using the stored token. Vault is reachable and the token is valid.',
        { exact: true },
      ),
    ).toBeVisible();
  } finally {
    await deleteSecretProvidersByPrefix(request, providerName);
  }
});
