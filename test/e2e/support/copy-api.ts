import { APIRequestContext, expect } from '@playwright/test';
import { randomUUID } from 'node:crypto';
import { getAdminAccessToken } from './admin-api';
import { runInDockerDaemon } from './docker';

export type CopyProgress = {
  id: string; status: string; destinationId: string | null;
  restores: Array<{ id: string; status: string; target: string }>;
};
export const copyApi = async (request: APIRequestContext, method: 'get' | 'post' | 'delete', path: string, data?: unknown) => {
  const response = await request[method](`/api/v1/${path}`, {
    headers: { Authorization: `Bearer ${await getAdminAccessToken(request)}` }, data,
    timeout: 120_000, maxRetries: method === 'get' ? 2 : 0,
  });
  if (!response.ok()) throw new Error(`${method} ${path}: ${response.status()} ${await response.text()}`);
  const body = await response.text();
  return body ? JSON.parse(body) : undefined;
};

export async function createCopyFixture(request: APIRequestContext) {
  const name = `e2e-copy-${randomUUID().slice(0, 8)}`;
  const source = `${name}-data`;
  const repositoryVolume = `${name}-repository`;
  // Reuse this suite's Platform only if an earlier fixture failed during setup.
  const platforms = await copyApi(request, 'get', 'platforms');
  let platform = platforms.platforms.find((p: { name: string }) => p.name === 'e2e-copy-platform');
  if (!platform) platform = await copyApi(request, 'post', 'platforms', {
    name: 'e2e-copy-platform', type: 'Docker', connectorType: 'Local', tagIds: [],
  });
  await runInDockerDaemon('volume', 'create', source);
  await runInDockerDaemon('volume', 'create', repositoryVolume);
  await runInDockerDaemon('run', '--rm', '-v', `${source}:/data`, 'busybox:1.36.1',
    'sh', '-ec', 'printf copy-fixture > /data/payload; chmod 640 /data/payload');
  const secret = await copyApi(request, 'post', 'resourceBindings/secrets', { name, value: 'disposable-copy-password' });
  const repository = await copyApi(request, 'post', 'backupRepositories', {
    name, passwordSecretId: secret.id,
    spec: { $type: 'FileSystem', location: 'Platform', platformId: platform.id, path: repositoryVolume },
  });
  await copyApi(request, 'post', `backupRepositories/${repository.id}/initialize`, { location: 'Platform', platformId: platform.id });
  const policy = await copyApi(request, 'post', 'backupPolicies', {
    name, backupRepositoryId: repository.id, enabled: true, alertOnFailure: false,
    source: { $type: 'DockerVolume', platformId: platform.id, volumeName: source },
    tagIds: [],
  });
  await copyApi(request, 'post', `backupPolicies/${policy.id}/run`, { trigger: 'Manual' });
  await expect.poll(async () => {
    const body = await copyApi(request, 'get', `backupRuns?policyId=${policy.id}`);
    return body.runs[0]?.status;
  }, { timeout: 120_000 }).toBe('Succeeded');
  const stack = await copyApi(request, 'post', 'stacks', {
    name, platformId: platform.id, stackSource: 'WebEditor', tagIds: [],
    spec: { $type: 'WebEditor', updateBehavior: 'Disabled', composeFile: `services:\n  app:\n    image: busybox:1.36.1\n    command: ["tail", "-f", "/dev/null"]\n    volumes: ["data:/data"]\nvolumes:\n  data:\n    external: true\n    name: ${source}\n` },
  });
  return { name, source, repositoryVolume, platformId: platform.id, stackId: stack.id, policyId: policy.id, repositoryId: repository.id };
}
