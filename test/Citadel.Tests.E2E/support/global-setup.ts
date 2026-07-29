import { chromium, expect, request } from '@playwright/test';
import fs from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { adminCredentials } from './admin-credentials';

const rootDirectory = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const authFile = path.join(rootDirectory, '.auth/admin.json');
const tokenFile = path.join(rootDirectory, '.auth/admin-token');

export default async function globalSetup() {
  const baseURL = process.env.CITADEL_E2E_BASE_URL ?? 'http://127.0.0.1:18000';
  const api = await request.newContext({ baseURL });

  await expect
    .poll(
      async () => {
        try {
          return (await api.get('/health')).ok();
        } catch {
          return false;
        }
      },
      { timeout: 60_000, message: `Citadel did not become healthy at ${baseURL}` },
    )
    .toBe(true);

  const setupStatus = await api.get('/api/v1/setup/status');
  if (!setupStatus.ok()) {
    throw new Error(`E2E setup status failed with HTTP ${setupStatus.status()}`);
  }

  const { requiresSetup } = (await setupStatus.json()) as { requiresSetup: boolean };
  await fs.mkdir(path.dirname(authFile), { recursive: true });

  let accessToken: string | undefined;
  if (requiresSetup) {
    const browser = await chromium.launch();
    try {
      const context = await browser.newContext({ baseURL });
      const page = await context.newPage();
      await page.goto('/stacks?source=first-run');
      await expect(page.getByRole('heading', { name: 'Set up Citadel' })).toBeVisible();
      await page.getByLabel('Username').fill(adminCredentials.name);
      await page.getByLabel('Email address').fill(adminCredentials.email);
      await page.getByLabel('Password', { exact: true }).fill(adminCredentials.password);
      await page.getByLabel('Confirm password', { exact: true }).fill(adminCredentials.password);

      const initializeResponsePromise = page.waitForResponse(
        (response) =>
          response.url().endsWith('/api/v1/setup/initialize') &&
          response.request().method() === 'POST',
      );
      await page.getByRole('button', { name: 'Create administrator' }).click();
      const initializeResponse = await initializeResponsePromise;
      if (!initializeResponse.ok()) {
        throw new Error(
          `E2E admin initialization failed with HTTP ${initializeResponse.status()}`,
        );
      }

      const body = (await initializeResponse.json()) as { accessToken?: string };
      accessToken = body.accessToken;
      await expect(page).toHaveURL(/\/stacks\?source=first-run$/);
      expect(page.url()).not.toContain(adminCredentials.password);
      await context.storageState({ path: authFile });
    } finally {
      await browser.close();
    }
  } else {
    const login = await api.post('/api/v1/authentication/login', {
      data: {
        emailOrName: adminCredentials.email,
        password: adminCredentials.password,
      },
    });

    if (!login.ok()) {
      throw new Error(`E2E admin login failed with HTTP ${login.status()}`);
    }

    const loginBody = (await login.json()) as { accessToken?: string };
    accessToken = loginBody.accessToken;
    await api.storageState({ path: authFile });
  }

  if (!accessToken) {
    throw new Error('E2E admin authentication did not return an access token');
  }

  await fs.writeFile(tokenFile, accessToken, { encoding: 'utf8', mode: 0o600 });
  await api.dispose();
}
