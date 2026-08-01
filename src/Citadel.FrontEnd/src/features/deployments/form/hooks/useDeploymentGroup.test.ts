import { describe, expect, it } from 'vitest';
import { createElement } from 'react';
import { act, screen, waitFor } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import {
  AutoUpdateStatus,
  DeploymentStatus,
  DeploymentView,
  PlatformStatus,
  ResourceControlState,
  UpdateBehavior,
} from '@/api/generated/api.types';
import { FakeHubConnection } from '@/test/fakes/signalr';
import { renderCitadel } from '@/test/render-citadel';
import { server } from '@/test/server';
import { getDeploymentUpdateCheckDisabledReason } from '../../update-status';
import { DeploymentActions } from '../actions';
import { mergeDeploymentInfo, useDeploymentGroup } from './useDeploymentGroup';

const deploymentId = '019f0000-0000-7000-8000-000000000010';

const DeploymentUpdateCheckProbe = () => {
  const { deployment } = useDeploymentGroup(deploymentId);
  return deployment ? createElement(DeploymentActions.checkUpdates, { resource: deployment }) : null;
};

describe('useDeploymentGroup', () => {
  it('enables update checks from the successful redeploy notification without refetching', async () => {
    const fake = new FakeHubConnection();
    const beforeRedeploy = createExternalDeployment(null);
    const afterRedeploy = withMessagePackImage(createExternalDeployment('sha256:applied'));
    let requestCount = 0;

    server.use(
      http.get(`http://localhost/api/v1/deployments/${deploymentId}`, () => {
        requestCount += 1;
        return HttpResponse.json(beforeRedeploy);
      }),
    );

    const { queryClient } = renderCitadel(createElement(DeploymentUpdateCheckProbe), {
      signalR: {
        connectionFactory: () => fake.asHubConnection(),
        startConnection: (connection) => connection.start(),
      },
    });

    const button = await screen.findByRole('button', { name: /check for updates/i });
    expect(button).toBeDisabled();
    await waitFor(() => expect(fake.listenerCount('DeploymentInfoUpdated')).toBe(1));

    act(() => {
      fake.emit('DeploymentInfoUpdated', afterRedeploy, 'update');
    });

    await waitFor(() => expect(screen.getByRole('button', { name: /check for updates/i })).toBeEnabled());
    await waitFor(() => expect(queryClient.isFetching({ queryKey: ['getDeployment', { deploymentId }] })).toBe(0));
    expect(requestCount).toBe(1);
  });

  it('keeps update checks enabled when a later container status notification omits the spec', async () => {
    const fake = new FakeHubConnection();
    const beforeRedeploy = createExternalDeployment(null);
    const afterRedeploy = withMessagePackImage(createExternalDeployment('sha256:applied'));
    const containerStatusUpdate = {
      ...createExternalDeployment(null),
      spec: null,
    } as unknown as DeploymentView;

    server.use(
      http.get(`http://localhost/api/v1/deployments/${deploymentId}`, () => HttpResponse.json(beforeRedeploy)),
    );

    renderCitadel(createElement(DeploymentUpdateCheckProbe), {
      signalR: {
        connectionFactory: () => fake.asHubConnection(),
        startConnection: (connection) => connection.start(),
      },
    });

    const button = await screen.findByRole('button', { name: /check for updates/i });
    expect(button).toBeDisabled();
    await waitFor(() => expect(fake.listenerCount('DeploymentInfoUpdated')).toBe(1));

    act(() => {
      fake.emit('DeploymentInfoUpdated', afterRedeploy, 'update');
    });
    await waitFor(() => expect(screen.getByRole('button', { name: /check for updates/i })).toBeEnabled());

    act(() => {
      fake.emit('DeploymentInfoUpdated', containerStatusUpdate, 'update');
    });

    await waitFor(() => expect(screen.getByRole('button', { name: /check for updates/i })).toBeEnabled());
  });
});

describe('mergeDeploymentInfo', () => {
  it('includes image provenance from a successful redeploy notification', () => {
    const current = createExternalDeployment(null);
    const deployment = withMessagePackImage({
      ...current,
      spec: { ...current.spec, image: { ...current.spec.image, resolvedDigest: 'sha256:applied' } },
    } as DeploymentView);

    const update = mergeDeploymentInfo(current, deployment);

    expect(update.spec?.image).toMatchObject({
      $type: 'External',
      resolvedDigest: 'sha256:applied',
    });
    expect(Array.isArray(deployment.spec.image)).toBe(true);
    expect(getDeploymentUpdateCheckDisabledReason(update as DeploymentView)).toBeUndefined();
  });

  it('replaces stale cached image provenance after a successful redeploy', () => {
    const current = {
      spec: {
        image: {
          $type: 'External',
          registryId: '019f0000-0000-7000-8000-000000000001',
          imageTag: 'nginx:latest',
          resolvedDigest: null,
        },
      },
      latestActivityView: null,
    } as DeploymentView;
    const notification = {
      ...current,
      spec: {
        ...current.spec,
        image: {
          ...current.spec.image,
          resolvedDigest: 'sha256:applied',
        },
      },
    } as DeploymentView;

    const merged = mergeDeploymentInfo(current, notification);

    expect(getDeploymentUpdateCheckDisabledReason(merged)).toBeUndefined();
  });

  it('preserves the complete spec when a status notification omits it', () => {
    const current = createExternalDeployment('sha256:applied');
    const notification = {
      ...current,
      status: DeploymentStatus.Stopped,
      spec: null,
    } as unknown as DeploymentView;

    const merged = mergeDeploymentInfo(current, notification);

    expect(merged.status).toBe(DeploymentStatus.Stopped);
    expect(merged.spec).toBe(current.spec);
    expect(merged.spec.image).toMatchObject({
      $type: 'External',
      resolvedDigest: 'sha256:applied',
    });
  });

  it('normalizes activity info without mutating the SignalR payload', () => {
    const serializedInfo = ['DeploymentApplied', { result: { containerIds: ['container-id'] } }];
    const deployment = {
      ...createExternalDeployment(null),
      latestActivityView: { info: serializedInfo },
    } as unknown as DeploymentView;

    const update = mergeDeploymentInfo(createExternalDeployment(null), deployment);

    expect(update.latestActivityView?.info).toMatchObject({ $type: 'DeploymentApplied' });
    expect(serializedInfo[1]).not.toHaveProperty('$type');
  });
});

const createExternalDeployment = (resolvedDigest: string | null): DeploymentView =>
  ({
    id: deploymentId,
    name: 'nginx',
    description: null,
    platformId: '019f0000-0000-7000-8000-000000000020',
    createdAt: '2026-07-31T00:00:00Z',
    createdByActorId: '019f0000-0000-7000-8000-000000000030',
    status: DeploymentStatus.Healthy,
    controlState: ResourceControlState.Idle,
    autoUpdateState: {
      lastCheckedAt: '0001-01-01T00:00:00Z',
      status: AutoUpdateStatus.Unknown,
      currentDigest: null,
      remoteDigest: null,
      lastError: null,
    },
    spec: {
      image: {
        $type: 'External',
        registryId: '019f0000-0000-7000-8000-000000000001',
        imageTag: 'nginx:latest',
        resolvedDigest,
      },
      updateBehavior: UpdateBehavior.Disabled,
    },
    platformStatus: PlatformStatus.Online,
    platformName: 'local',
    imageName: 'nginx',
    imageId: null,
    containerId: null,
    dockerContainerId: null,
    dockerImageId: null,
    tags: [],
    latestActivityView: null,
    capabilities: null,
  }) as DeploymentView;

const withMessagePackImage = (deployment: DeploymentView): DeploymentView => {
  const image = deployment.spec.image;
  const { $type, ...payload } = image;

  return {
    ...deployment,
    spec: {
      ...deployment.spec,
      image: [$type, payload] as unknown as DeploymentView['spec']['image'],
    },
  };
};
