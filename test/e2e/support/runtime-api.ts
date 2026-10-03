import { APIRequestContext, APIResponse } from '@playwright/test';

import { getAdminAccessToken } from './admin-api';

const runtimePrefix = 'e2e-runtime-';

export type RuntimeRelease = 'one' | 'two';

export type RuntimeFixture = {
  name: string;
  platformId: string;
  projectName: string;
  stackId: string;
};

export type RuntimeStack = {
  id: string;
  name: string;
  status: string;
  currentStackReleaseId: string;
  version?: string | null;
  spec?: {
    $type?: string;
    composeFile?: string;
  } | null;
};

export type RuntimeStackRelease = {
  id: string;
  status: string;
  version: string;
  spec: {
    $type?: string;
    composeFile?: string;
  };
};

const authorizationHeaders = async (request: APIRequestContext) => ({
  Authorization: `Bearer ${await getAdminAccessToken(request)}`,
});

const responseFailure = async (operation: string, response: APIResponse) => {
  const body = await response.text().catch(() => '');
  return new Error(
    `${operation} failed with HTTP ${response.status()}${body ? `: ${body}` : ''}`,
  );
};

const expectOk = async (operation: string, response: APIResponse) => {
  if (!response.ok()) {
    throw await responseFailure(operation, response);
  }
};

const composeFile = (release: RuntimeRelease) => `services:
  runtime:
    image: busybox:1.36.1
    command: ["sh", "-c", "echo citadel-e2e-release-${release}; exec tail -f /dev/null"]
    stop_grace_period: 1s
    labels:
      citadel.e2e.release: ${release}
`;

const stackSpec = (fixture: Pick<RuntimeFixture, 'projectName'>, release: RuntimeRelease) => ({
  $type: 'WebEditor',
  composeFile: composeFile(release),
  updateBehavior: 'Disabled',
  projectName: fixture.projectName,
  destroyBeforeDeploy: true,
  buildImageBindings: [],
});

const deleteStacks = async (request: APIRequestContext, ids: string[]) => {
  if (ids.length === 0) return;

  const response = await request.delete('/api/v1/stacks', {
    headers: await authorizationHeaders(request),
    data: ids,
  });
  if (!response.ok() && response.status() !== 404) {
    throw await responseFailure('Deleting E2E runtime stacks', response);
  }
};

const deletePlatforms = async (request: APIRequestContext, ids: string[]) => {
  if (ids.length === 0) return;

  const response = await request.delete('/api/v1/platforms', {
    headers: await authorizationHeaders(request),
    data: { ids },
  });
  if (!response.ok() && response.status() !== 404) {
    throw await responseFailure('Deleting E2E runtime platforms', response);
  }
};

export const cleanupStaleRuntimeFixtures = async (request: APIRequestContext) => {
  const headers = await authorizationHeaders(request);
  const [stacksResponse, platformsResponse] = await Promise.all([
    request.get('/api/v1/stacks', { headers }),
    request.get('/api/v1/platforms', { headers }),
  ]);

  await expectOk('Listing stacks for E2E cleanup', stacksResponse);
  await expectOk('Listing platforms for E2E cleanup', platformsResponse);

  const stacksBody = (await stacksResponse.json()) as {
    stacks?: Array<{ id: string; name: string }>;
  };
  const platformsBody = (await platformsResponse.json()) as {
    platforms?: Array<{ id: string; name: string }>;
  };

  await deleteStacks(
    request,
    (stacksBody.stacks ?? [])
      .filter((stack) => stack.name.startsWith(runtimePrefix))
      .map((stack) => stack.id),
  );
  await deletePlatforms(
    request,
    (platformsBody.platforms ?? [])
      .filter((platform) => platform.name.startsWith(runtimePrefix))
      .map((platform) => platform.id),
  );
};

export const createRuntimeFixture = async (
  request: APIRequestContext,
  fixedSuffix?: string,
  connectorType: 'Local' | 'EdgeAgent' = 'Local',
): Promise<RuntimeFixture> => {
  await cleanupStaleRuntimeFixtures(request);

  const suffix = fixedSuffix ?? `${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
  const name = `${runtimePrefix}${suffix}`;
  const projectName = name.replace(/[^a-z0-9_-]/g, '-');
  const headers = await authorizationHeaders(request);

  const platformResponse = await request.post('/api/v1/platforms', {
    headers,
    data: {
      name: `${name}-platform`,
      address: null,
      description: 'Disposable Playwright runtime platform',
      type: 'Docker',
      connectorType,
      tagIds: [],
    },
  });
  await expectOk('Creating the E2E runtime platform', platformResponse);
  const platform = (await platformResponse.json()) as { id: string };

  const fixture: RuntimeFixture = {
    name,
    platformId: platform.id,
    projectName,
    stackId: '',
  };

  try {
    const stackResponse = await request.post('/api/v1/stacks', {
      headers,
      data: {
        name,
        platformId: platform.id,
        description: 'Disposable Playwright release and rollback stack',
        stackSource: 'WebEditor',
        spec: stackSpec(fixture, 'one'),
        tagIds: [],
      },
    });
    await expectOk('Creating the E2E runtime stack', stackResponse);
    fixture.stackId = ((await stackResponse.json()) as { id: string }).id;
    return fixture;
  } catch (error) {
    await deletePlatforms(request, [platform.id]);
    throw error;
  }
};

export const cleanupRuntimeFixture = async (
  request: APIRequestContext,
  fixture: RuntimeFixture,
) => {
  if (fixture.stackId) {
    await deleteStacks(request, [fixture.stackId]);
  }
  await deletePlatforms(request, [fixture.platformId]);
};

export const updateRuntimeStack = async (
  request: APIRequestContext,
  fixture: RuntimeFixture,
  release: RuntimeRelease,
) => {
  const response = await request.patch(`/api/v1/stacks/${fixture.stackId}`, {
    headers: await authorizationHeaders(request),
    data: {
      platformId: fixture.platformId,
      spec: stackSpec(fixture, release),
    },
  });
  await expectOk(`Updating the E2E runtime stack to release ${release}`, response);
};

export const applyRuntimeStack = async (
  request: APIRequestContext,
  stackId: string,
) => {
  const response = await request.post('/api/v1/stacks/apply', {
    headers: await authorizationHeaders(request),
    data: { id: stackId, recreate: false },
    timeout: 120_000,
  });
  await expectOk('Applying the E2E runtime stack', response);

  const events = (await response.json()) as Array<{
    exitCode?: number;
    severity?: string;
    stackStatus?: string;
  }>;
  const completion = events.at(-1);
  if (
    completion?.exitCode !== 0 ||
    completion.severity !== 'success' ||
    completion.stackStatus !== 'Healthy'
  ) {
    throw new Error(
      `Applying the E2E runtime stack did not finish healthy: ${JSON.stringify(completion)}`,
    );
  }
};

export const getRuntimeStack = async (
  request: APIRequestContext,
  stackId: string,
): Promise<RuntimeStack> => {
  const response = await request.get(`/api/v1/stacks/${stackId}`, {
    headers: await authorizationHeaders(request),
  });
  await expectOk('Reading the E2E runtime stack', response);
  return (await response.json()) as RuntimeStack;
};

export const getRuntimeStackReleases = async (
  request: APIRequestContext,
  stackId: string,
): Promise<RuntimeStackRelease[]> => {
  const response = await request.get(`/api/v1/stacks/${stackId}/releases`, {
    headers: await authorizationHeaders(request),
  });
  await expectOk('Reading E2E runtime stack releases', response);
  const body = (await response.json()) as { releases: RuntimeStackRelease[] };
  return body.releases;
};
