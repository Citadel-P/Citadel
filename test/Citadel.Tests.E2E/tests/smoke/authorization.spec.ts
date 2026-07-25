import {
  createRestrictedPersona,
  deleteRestrictedPersona,
} from '../../support/admin-api';
import { test, expect } from '../../support/test';

test('a restricted user cannot directly access an unauthorized resource list', async ({
  browser,
  request,
}) => {
  const prefix = `e2e-restricted-${Date.now()}`;
  const persona = await createRestrictedPersona(request, prefix);
  const restrictedContext = await browser.newContext({
    storageState: { cookies: [], origins: [] },
  });

  try {
    const page = await restrictedContext.newPage();
    await page.goto('/build-pools');
    await page.getByLabel('Email address or username').fill(persona.email);
    await page.getByLabel('Password').fill(persona.password);

    const listResponse = page.waitForResponse(
      (response) =>
        new URL(response.url()).pathname === '/api/v1/buildAgentPools' &&
        response.request().method() === 'GET',
    );
    await page.getByRole('button', { name: 'Sign in' }).click();

    const response = await listResponse;
    expect(response.status()).toBe(403);
    await expect(page).toHaveURL(/\/build-pools$/);
    await expect(page.getByRole('button', { name: 'Add Pool' })).toBeDisabled();
  } finally {
    await restrictedContext.close();
    await deleteRestrictedPersona(request, persona);
  }
});
