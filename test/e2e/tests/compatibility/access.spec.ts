import type { APIRequestContext } from '@playwright/test';
import {
  createOidcProvider,
  deleteOidcProvider,
  getAdminAccessToken,
  type OidcProviderFixture,
} from '../../support/admin-api';
import { expect, test } from '../../support/test';

test('administrator Access screens use the Rust identity and license contracts', async ({
  page,
  request,
}) => {
  const marker = `${Date.now()}-${Math.random().toString(16).slice(2, 8)}`;
  const userName = `access-user-${marker}`;
  const userEmail = `${userName}@citadel.local`;
  const password = 'access-e2e-password';
  const teamName = `access-team-${marker}`;
  let userId: string | undefined;
  let teamId: string | undefined;

  try {
    await page.goto('/access/users/add');
    await expect(page.getByRole('heading', { name: 'Add User' })).toBeVisible();
    await page.getByLabel('Username').fill(userName);
    await page.getByLabel('Email').fill(userEmail);
    await page.getByLabel('Password', { exact: true }).fill(password);
    await page.getByLabel('Repeat Password').fill(password);

    const createUserResponse = page.waitForResponse(
      (response) =>
        response.request().method() === 'POST' && response.url().endsWith('/api/v1/users'),
    );
    await page.getByRole('button', { name: 'Save' }).click();
    expect((await createUserResponse).ok()).toBe(true);
    await expect(page).toHaveURL(/\/access\/users\/edit\/[^/]+$/);
    userId = page.url().split('/').at(-1);
    expect(userId).toBeTruthy();
    await expect(page.getByText(userName, { exact: true })).toBeVisible();
    await expect(page.getByLabel('Email')).toHaveValue(userEmail);

    await page.goto('/access/users');
    await expect(page.getByRole('link', { name: userName })).toBeVisible();
    await expect(page.getByText(userEmail, { exact: true })).toBeVisible();

    await page.goto('/access/teams/add');
    await expect(page.getByRole('heading', { name: 'Add Team' })).toBeVisible();
    await page.getByLabel('Team Name').fill(teamName);

    const createTeamResponse = page.waitForResponse(
      (response) =>
        response.request().method() === 'POST' && response.url().endsWith('/api/v1/teams'),
    );
    await page.getByRole('button', { name: 'Save' }).click();
    expect((await createTeamResponse).ok()).toBe(true);
    await expect(page).toHaveURL(/\/access\/teams\/edit\/[^/]+$/);
    teamId = page.url().split('/').at(-1);
    expect(teamId).toBeTruthy();
    await expect(page.getByText(teamName, { exact: true })).toBeVisible();

    await page.goto('/access/teams');
    await expect(page.getByRole('link', { name: teamName })).toBeVisible();

    await page.goto('/access/roles');
    await expect(page.getByText('Admin', { exact: true })).toBeVisible();
    await expect(page.getByText('Operator', { exact: true })).toBeVisible();
    await expect(page.getByText('Viewer', { exact: true })).toBeVisible();
    await expect(page.getByRole('button', { name: 'Add Role' })).toBeDisabled();

    await page.goto('/access/service-accounts');
    await expect(page.getByText('Service Accounts require a Team license')).toBeVisible();
    await expect(page.getByRole('button', { name: 'Add Service Account' })).toBeDisabled();

    await page.goto('/license');
    await expect(page.getByRole('heading', { name: 'License', exact: true })).toBeVisible();
    await expect(page.getByText('Community', { exact: true }).first()).toBeVisible();
    await expect(page.getByRole('heading', { name: 'Capabilities' })).toBeVisible();
  } finally {
    await deleteAccessFixtures(request, { userId, teamId });
  }
});

test('administrator can open a configured OIDC provider through the production UI', async ({
  page,
  request,
}) => {
  let provider: OidcProviderFixture | undefined;

  try {
    provider = await createOidcProvider(request, `access-oidc-${Date.now()}`);

    await page.goto('/oidc-providers');
    await expect(page.getByRole('link', { name: provider.displayName })).toBeVisible();
    await page.getByRole('link', { name: provider.displayName }).click();
    await expect(page).toHaveURL(new RegExp(`/oidc-providers/edit/${provider.id}$`));
    await expect(page.getByLabel('Display Name')).toHaveValue(provider.displayName);
    await expect(page.getByRole('tab', { name: 'Config' })).toBeVisible();
    await expect(page.getByRole('tab', { name: 'Activities' })).toBeVisible();
  } finally {
    if (provider) {
      await deleteOidcProvider(request, provider);
    }
  }
});

async function deleteAccessFixtures(
  request: APIRequestContext,
  fixtures: { userId?: string; teamId?: string },
) {
  const accessToken = await getAdminAccessToken(request);
  const headers = { Authorization: `Bearer ${accessToken}` };

  if (fixtures.teamId) {
    const response = await request.delete('/api/v1/teams', {
      headers,
      data: { ids: [fixtures.teamId] },
    });
    if (!response.ok() && response.status() !== 404) {
      throw new Error(`Unable to delete Access E2E Team: HTTP ${response.status()}`);
    }
  }

  if (fixtures.userId) {
    const response = await request.delete('/api/v1/users', {
      headers,
      data: { ids: [fixtures.userId] },
    });
    if (!response.ok() && response.status() !== 404) {
      throw new Error(`Unable to delete Access E2E User: HTTP ${response.status()}`);
    }
  }
}
