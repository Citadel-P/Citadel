import { renderHook } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import {
  ContainerDataView,
  ContainerStateStatus,
  ContainerSystemRole,
  ContainerView,
  ResourceControlState,
} from '@/api/generated/api.types';
import { mergeContainerRuntimeUpdate, toContainerDetailsView, useContainerInfoGroup } from './useContainerInfoGroup';

const { useReadMock } = vi.hoisted(() => ({ useReadMock: vi.fn() }));

vi.mock('@/lib/hooks', () => ({ useRead: useReadMock }));
vi.mock('@/lib/context/app-context', () => ({ useAppContext: () => ({ currentPlatform: undefined }) }));
vi.mock('@/features/platforms/hooks/useDockerDaemonGroup', () => ({ useDockerDaemonGroup: vi.fn() }));
vi.mock('@/hooks/useRealtimeGroup', () => ({ useRealtimeGroup: vi.fn() }));

beforeEach(() => {
  useReadMock.mockReset();
  useReadMock.mockReturnValue({ data: undefined, isLoading: false });
});

describe('mergeContainerRuntimeUpdate', () => {
  const runtimeUpdate: ContainerDataView = {
    id: 'ea5f935b4706',
    name: '/nginx',
    image: 'nginx:latest',
    imageId: 'sha256:nginx',
    state: ContainerStateStatus.Running,
    controlState: ResourceControlState.Idle,
    isSystem: false,
    systemRole: null,
    hasCitadelOwnershipLabels: false,
    isSwarmTask: true,
    dockerNodeId: 'worker-node',
    deploymentId: null,
    stackId: null,
    containerStat: {
      cpuUsage: 2,
      memoryActive: 1024,
      memoryCache: 0,
      memoryLimit: 4096,
      rxBytes: 10,
      txBytes: 20,
    },
  };

  it('preserves deployment ownership when runtime stats contain null ownership', () => {
    const deploymentId = '019faff7-7f5b-7e8d-9cd6-4bd5efb3cc80';
    const result = mergeContainerRuntimeUpdate({ deploymentId }, runtimeUpdate);

    expect(result.deploymentId).toBe(deploymentId);
    expect(result).not.toHaveProperty('stackId');
    expect(result.state).toBe(ContainerStateStatus.Running);
    expect(result.containerStat).toBe(runtimeUpdate.containerStat);
  });

  it('preserves system classification and operation state', () => {
    const result = mergeContainerRuntimeUpdate(
      {
        controlState: ResourceControlState.Processing,
        isSystem: true,
        systemRole: ContainerSystemRole.Agent,
      },
      runtimeUpdate,
    );

    expect(result.controlState).toBe(ResourceControlState.Processing);
    expect(result.isSystem).toBe(true);
    expect(result.systemRole).toBe(ContainerSystemRole.Agent);
    expect(result).not.toHaveProperty('deploymentId');
  });
});

describe('toContainerDetailsView', () => {
  it('keeps the persisted resource id separate from the Docker container id', () => {
    const details = toContainerDetailsView({
      id: '019f0000-0000-7000-8000-000000000001',
      platformId: '019f0000-0000-7000-8000-000000000002',
      containerId: 'ea5f935b4706',
      name: 'nginx',
      dockerImageId: 'nginx-image-id',
      created: 1,
      updated: 2,
      state: ContainerStateStatus.Running,
      controlState: ResourceControlState.Idle,
      stack: null,
      isSystem: false,
      systemRole: null,
      hasCitadelOwnershipLabels: false,
      isSwarmTask: true,
      dockerNodeId: 'worker-node',
      nodeHostname: 'worker',
      projectionObservedAt: 1,
      projectionStaleSince: null,
      projectionStaleReason: null,
      lastStats: null,
      ports: {},
      deploymentId: null,
      stackId: null,
    } satisfies ContainerView);

    expect(details.resourceId).toBe('019f0000-0000-7000-8000-000000000001');
    expect(details.platformId).toBe('019f0000-0000-7000-8000-000000000002');
    expect(details.id).toBe('ea5f935b4706');
    expect(details.dockerNodeId).toBe('worker-node');
  });
});

describe('useContainerInfoGroup', () => {
  it('preserves a persisted container id when loading container details', () => {
    const persistedId = '019fed15-5340-7000-8000-000000000001';

    renderHook(() => useContainerInfoGroup(persistedId, '019f0000-0000-7000-8000-000000000002'));

    expect(useReadMock).toHaveBeenCalledWith('getContainer', { id: persistedId });
  });
});
