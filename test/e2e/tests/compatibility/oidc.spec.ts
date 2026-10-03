import type { OidcProviderFixture } from '../../support/admin-api';
import { createOidcProvider, deleteOidcProvider } from '../../support/admin-api';
import { test, expect } from '../../support/test';

const emptyStorageState = { cookies: [], origins: [] };
const keycloakPassword = 'KeycloakE2E123!';

test.use({ storageState: emptyStorageState });

test('Keycloak login creates a Citadel session that survives reload and is cleared by logout', async ({
  page,
  context,
  request,
}) => {
  let provider: OidcProviderFixture | undefined;

  try {
    provider = await createOidcProvider(request, uniqueProviderName('login'));

    await page.goto('/profile');
    await expect(page.getByRole('heading', { name: 'Sign in to your account' })).toBeVisible();
    await page.getByRole('button', { name: `Continue with ${provider.displayName}` }).click();

    await expect(page).toHaveURL(/keycloak:18080\/realms\/citadel-e2e\/protocol\/openid-connect\/auth/);
    await page.locator('#username').fill('oidc-admin');
    await page.locator('#password').fill(keycloakPassword);
    await page.locator('#kc-login').click();

    await expect(page).toHaveURL(/\/profile$/);
    await expect(page.getByRole('heading', { name: 'admin' })).toBeVisible();
    await expect(page.getByText('Managed by Keycloak E2E', { exact: true }).first()).toBeVisible();
    await expect(page.getByText('admin@citadel.local', { exact: true }).first()).toBeVisible();

    const refreshCookie = (await context.cookies()).find((cookie) => cookie.name === 'refresh_token');
    expect(refreshCookie).toBeDefined();
    expect(refreshCookie?.httpOnly).toBe(true);

    await page.reload();
    await expect(page).toHaveURL(/\/profile$/);
    await expect(page.getByText('Managed by Keycloak E2E', { exact: true }).first()).toBeVisible();

    await page.getByRole('button', { name: 'Open account menu' }).click();
    const logoutResponse = page.waitForResponse(
      (response) =>
        response.request().method() === 'POST' && response.url().endsWith('/api/v1/authentication/logout'),
    );
    await page.getByRole('menuitem', { name: 'Log out' }).click();
    expect((await logoutResponse).ok()).toBe(true);

    await expect(page.getByRole('heading', { name: 'Sign in to your account' })).toBeVisible();

    await page.reload();
    await expect(page.getByRole('heading', { name: 'Sign in to your account' })).toBeVisible();
  } finally {
    if (provider) {
      await deleteOidcProvider(request, provider);
    }
  }
});

test('Keycloak identity missing a required claim is rejected without creating a session', async ({
  page,
  context,
  request,
}) => {
  let provider: OidcProviderFixture | undefined;

  try {
    provider = await createOidcProvider(request, uniqueProviderName('claim'));

    await page.goto('/login');
    await page.getByRole('button', { name: `Continue with ${provider.displayName}` }).click();
    await page.locator('#username').fill('oidc-missing-claim');
    await page.locator('#password').fill(keycloakPassword);
    await page.locator('#kc-login').click();

    await expect(page).toHaveURL(new RegExp(`/api/v1/authentication/oidc/${provider.id}/callback`));
    await expect(page.locator('body')).toContainText('OIDC account is missing the required claim.');
    expect((await context.cookies()).some((cookie) => cookie.name === 'refresh_token')).toBe(false);
  } finally {
    if (provider) {
      await deleteOidcProvider(request, provider);
    }
  }
});

function uniqueProviderName(scenario: string) {
  return `oidc-e2e-${scenario}-${Date.now()}-${Math.random().toString(16).slice(2, 10)}`;
}
