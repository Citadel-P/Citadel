import { APIRequestContext } from '@playwright/test';
import fs from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const adminCredentials = {
  emailOrName: process.env.CITADEL_E2E_ADMIN_EMAIL ?? 'admin@citadel.local',
  password: process.env.CITADEL_E2E_ADMIN_PASSWORD ?? 'admin123',
};

const tokenFile = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  '../.auth/admin-token',
);

export const getAdminAccessToken = async (request: APIRequestContext) => {
  try {
    const token = (await fs.readFile(tokenFile, 'utf8')).trim();
    if (token) {
      return token;
    }
  } catch {
    // Fall back to a login for helpers used without Playwright global setup.
  }

  const response = await request.post('/api/v1/authentication/login', {
    data: adminCredentials,
  });
  if (!response.ok()) {
    throw new Error(`E2E admin API login failed with HTTP ${response.status()}`);
  }

  const body = (await response.json()) as { accessToken?: string };
  if (!body.accessToken) {
    throw new Error('E2E admin API login did not return an access token');
  }

  return body.accessToken;
};

export type RestrictedPersona = {
  userId: string;
  email: string;
  password: string;
};

export const createRestrictedPersona = async (
  request: APIRequestContext,
  prefix: string,
): Promise<RestrictedPersona> => {
  const accessToken = await getAdminAccessToken(request);
  const headers = { Authorization: `Bearer ${accessToken}` };
  const email = `${prefix}@citadel.local`;
  const password = 'restricted123';

  const userResponse = await request.post('/api/v1/users', {
    headers,
    data: {
      name: prefix,
      email,
      password,
      isEnabled: true,
    },
  });
  if (!userResponse.ok()) {
    throw new Error(`Unable to create restricted E2E user: HTTP ${userResponse.status()}`);
  }
  const user = (await userResponse.json()) as { id: string };

  return {
    userId: user.id,
    email,
    password,
  };
};

export const deleteRestrictedPersona = async (
  request: APIRequestContext,
  persona: RestrictedPersona,
) => {
  const accessToken = await getAdminAccessToken(request);
  const headers = { Authorization: `Bearer ${accessToken}` };

  const userDeletion = await request.delete('/api/v1/users', {
    headers,
    data: { ids: [persona.userId] },
  });
  if (!userDeletion.ok() && userDeletion.status() !== 404) {
    throw new Error(`Unable to delete restricted E2E user: HTTP ${userDeletion.status()}`);
  }
};

export const deleteTagsByPrefix = async (request: APIRequestContext, prefix: string) => {
  const accessToken = await getAdminAccessToken(request);
  const headers = { Authorization: `Bearer ${accessToken}` };
  const response = await request.get('/api/v1/tags', { headers });
  if (!response.ok()) {
    throw new Error(`Unable to list tags during E2E cleanup: HTTP ${response.status()}`);
  }

  const body = (await response.json()) as { tags?: Array<{ id: string; name: string }> };
  const tags = (body.tags ?? []).filter((tag) => tag.name.startsWith(prefix));

  await Promise.all(
    tags.map(async (tag) => {
      const deletion = await request.delete(`/api/v1/tags/${tag.id}`, { headers });
      if (!deletion.ok() && deletion.status() !== 404) {
        throw new Error(`Unable to delete E2E tag '${tag.name}': HTTP ${deletion.status()}`);
      }
    }),
  );
};
