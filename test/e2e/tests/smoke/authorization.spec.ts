import {
  createRestrictedPersona,
  deleteRestrictedPersona,
  getAdminAccessToken,
} from '../../support/admin-api';
import { test, expect } from '../../support/test';

test('a restricted user cannot list, read, or create unauthorized build pools', async ({
  browser,
  request,
}) => {
  const prefix = `e2e-restricted-${Date.now()}`;
  const persona = await createRestrictedPersona(request, prefix);
  const headers = { Authorization: `Bearer ${await getAdminAccessToken(request)}` };
  const poolInput = {
    name: `${prefix}-pool`,
    enabled: false,
    providerSpec: { $type: 'SelfManagedVm', connectionMode: 'EdgeAgent' },
  };
  let poolId: string | undefined;
  const restrictedContext = await browser.newContext({
    storageState: { cookies: [], origins: [] },
  });

  try {
    const created = await request.post('/api/v1/buildAgentPools', { headers, data: poolInput });
    expect(created.ok()).toBe(true);
    poolId = ((await created.json()) as { id: string }).id;

    const page = await restrictedContext.newPage();
    await page.goto('/build-pools');
    await page.getByLabel('Email address or username').fill(persona.email);
    await page.getByLabel('Password', { exact: true }).fill(persona.password);

    const listResponse = page.waitForResponse(
      (response) =>
        new URL(response.url()).pathname === '/api/v1/buildAgentPools' &&
        response.request().method() === 'GET',
    );
    const loginResponse = page.waitForResponse(
      (response) =>
        new URL(response.url()).pathname === '/api/v1/authentication/login' &&
        response.request().method() === 'POST',
    );
    await page.getByRole('button', { name: 'Sign in' }).click();
    const login = await loginResponse;
    expect(login.ok()).toBe(true);
    const { accessToken } = (await login.json()) as { accessToken: string };
    const restrictedHeaders = { Authorization: `Bearer ${accessToken}` };

    const response = await listResponse;
    // Lists return only authorized resources; individual access and writes are denied.
    expect(response.status()).toBe(200);
    expect(await response.json()).toMatchObject({
      pools: [],
      capabilities: { canWrite: false },
    });
    await expect(page).toHaveURL(/\/build-pools$/);
    await expect(page.getByRole('button', { name: 'Add Pool' })).toBeDisabled();

    const detail = await restrictedContext.request.get(`/api/v1/buildAgentPools/${poolId}`, {
      headers: restrictedHeaders,
    });
    expect(detail.status()).toBe(403);
    const create = await restrictedContext.request.post('/api/v1/buildAgentPools', {
      headers: restrictedHeaders,
      data: { ...poolInput, name: `${prefix}-denied` },
    });
    expect(create.status()).toBe(403);
  } finally {
    await restrictedContext.close();
    try {
      if (poolId) {
        const deleted = await request.delete(`/api/v1/buildAgentPools/${poolId}`, { headers });
        expect(deleted.status()).toBe(204);
      }
    } finally {
      await deleteRestrictedPersona(request, persona);
    }
  }
});
