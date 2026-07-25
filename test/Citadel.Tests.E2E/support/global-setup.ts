import { expect, request } from '@playwright/test';
import fs from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

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

  const login = await api.post('/api/v1/authentication/login', {
    data: {
      emailOrName: process.env.CITADEL_E2E_ADMIN_EMAIL ?? 'admin@citadel.local',
      password: process.env.CITADEL_E2E_ADMIN_PASSWORD ?? 'admin123',
    },
  });

  if (!login.ok()) {
    throw new Error(`E2E admin login failed with HTTP ${login.status()}`);
  }

  const loginBody = (await login.json()) as { accessToken?: string };
  if (!loginBody.accessToken) {
    throw new Error('E2E admin login did not return an access token');
  }

  await fs.mkdir(path.dirname(authFile), { recursive: true });
  await api.storageState({ path: authFile });
  await fs.writeFile(tokenFile, loginBody.accessToken, { encoding: 'utf8', mode: 0o600 });
  await api.dispose();
}
