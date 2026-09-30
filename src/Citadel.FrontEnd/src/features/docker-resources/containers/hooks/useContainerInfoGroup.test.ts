import { act, renderHook } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import {
  ContainerDataView,
  ContainerStateStatus,
  ContainerSystemRole,
  ContainerView,
  ResourceControlState,
} from '@/api/generated/api.types';
import { mergeContainerRuntimeUpdate, toContainerDetailsView, useContainerInfoGroup } from './useContainerInfoGroup';

const { useReadMock, useRealtimeGroupMock } = vi.hoisted(() => ({
  useReadMock: vi.fn(),
  useRealtimeGroupMock: vi.fn(),
}));

vi.mock('@/lib/hooks', () => ({ useRead: useReadMock }));
vi.mock('@/lib/context/app-context', () => ({ useAppContext: () => ({ currentPlatform: undefined }) }));
vi.mock('@/features/platforms/hooks/useDockerDaemonGroup', () => ({ useDockerDaemonGroup: vi.fn() }));
vi.mock('@/hooks/useRealtimeGroup', () => ({ useRealtimeGroup: useRealtimeGroupMock }));

beforeEach(() => {
  useReadMock.mockReset();
  useRealtimeGroupMock.mockReset();
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
    capabilities: null,
    containerStat: {
      containerId: 'container-1',
      created: 0,
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
      capabilities: null,
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

it('merges detail patches without clearing metadata or accepting another resource', () => {
  const resourceId = '019f0000-0000-7000-8000-000000000001';
  useReadMock.mockReturnValue({
    data: {
      data: {
        id: resourceId,
        containerId: 'docker-id',
        platformId: 'platform',
        name: 'nginx',
        state: ContainerStateStatus.Running,
        ports: { '80': '8080' },
        controlState: ResourceControlState.Processing,
      },
    },
    isLoading: false,
  });
  const { result } = renderHook(() => useContainerInfoGroup(resourceId, 'platform'));
  const handlers = new Map<string, (value: unknown) => void>();
  const hub = { on: (name: string, callback: (value: unknown) => void) => handlers.set(name, callback), off: vi.fn() };
  useRealtimeGroupMock.mock.lastCall![0].setupEventListeners(hub);
  act(() => handlers.get('ReceiveContainerInfoPatch')!({ resourceId: 'other', state: ContainerStateStatus.Exited }));
  expect(result.current.containerInfo?.state).toBe(ContainerStateStatus.Running);
  act(() =>
    handlers.get('ReceiveContainerInfoPatch')!({
      resourceId,
      state: ContainerStateStatus.Exited,
      controlState: ResourceControlState.Idle,
    }),
  );
  expect(result.current.containerInfo).toMatchObject({
    name: 'nginx',
    state: ContainerStateStatus.Exited,
    controlState: ResourceControlState.Idle,
    ports: { '80': '8080' },
  });
  act(() =>
    handlers.get('ReceiveContainerInfoPatch')!({
      resourceId,
      containerStat: {
        containerId: 'container-1',
        created: 0,
        cpuUsage: 12,
      },
    }),
  );
  expect(result.current.containerInfo).toMatchObject({
    state: ContainerStateStatus.Exited,
    name: 'nginx',
    containerStat: {
      containerId: 'container-1',
      created: 0,
      cpuUsage: 12,
    },
  });
  useRealtimeGroupMock.mock.lastCall![0].removeEventListeners(hub);
  expect(hub.off).toHaveBeenCalledWith('ReceiveContainerInfoPatch', expect.any(Function));
});

it('switches identity and subscription and rejects late callbacks after navigation', () => {
  const a = '019f0000-0000-7000-8000-000000000001';
  const b = '019f0000-0000-7000-8000-000000000002';
  const view = (id: string) => ({
    data: {
      data: {
        id,
        platformId: 'platform',
        containerId: id === a ? 'docker-a' : 'docker-b',
        name: id === a ? 'A' : 'B',
        state: ContainerStateStatus.Running,
      },
    },
    isLoading: false,
  });
  useReadMock.mockReturnValue(view(a));
  const { result, rerender } = renderHook(({ id }) => useContainerInfoGroup(id, 'platform'), {
    initialProps: { id: a },
  });
  const handlers = new Map<string, (value: unknown) => void>();
  useRealtimeGroupMock.mock.lastCall![0].setupEventListeners({
    on: (name: string, callback: (value: unknown) => void) => handlers.set(name, callback),
    off: vi.fn(),
  });
  act(() => handlers.get('ReceiveContainerInfoPatch')!({ resourceId: a, state: ContainerStateStatus.Exited }));
  useReadMock.mockReturnValue(view(b));
  rerender({ id: b });
  expect.soft(result.current.containerInfo?.resourceId).toBe(b);
  expect.soft(result.current.containerInfo?.state).toBe(ContainerStateStatus.Running);
  expect.soft(useRealtimeGroupMock.mock.lastCall![0].groupName).toBe(`container-info:${b}`);
  // These callbacks still belong to the old subscription, including a full snapshot.
  act(() => {
    handlers.get('ReceiveContainerInfoPatch')!({ resourceId: a, state: ContainerStateStatus.Exited });
    handlers.get('ReceiveContainerInfo')!({ id: 'docker-a', state: ContainerStateStatus.Exited });
  });
  expect(result.current.containerInfo).toMatchObject({
    resourceId: b,
    id: 'docker-b',
    state: ContainerStateStatus.Running,
  });
});

it('rejects full snapshots for another node and resets live state across platforms sharing a Docker id', () => {
  const resourceId = '019f0000-0000-7000-8000-000000000003';
  const view = (platformId: string, dockerNodeId: string) => ({
    data: {
      data: {
        id: resourceId,
        platformId,
        dockerNodeId,
        containerId: 'same-docker-id',
        name: platformId,
        state: ContainerStateStatus.Running,
      },
    },
    isLoading: false,
  });
  useReadMock.mockReturnValue(view('first', 'node-a'));
  const { result, rerender } = renderHook(({ platform }) => useContainerInfoGroup(resourceId, platform), {
    initialProps: { platform: 'first' },
  });
  const handlers = new Map<string, (value: unknown) => void>();
  const hub = { on: (name: string, callback: (value: unknown) => void) => handlers.set(name, callback), off: vi.fn() };
  useRealtimeGroupMock.mock.lastCall![0].setupEventListeners(hub);
  act(() =>
    handlers.get('ReceiveContainerInfo')!({
      id: 'same-docker-id',
      dockerNodeId: 'node-b',
      state: ContainerStateStatus.Exited,
    }),
  );
  expect(result.current.containerInfo?.state).toBe(ContainerStateStatus.Running);
  act(() => handlers.get('ReceiveContainerInfoPatch')!({ resourceId, state: ContainerStateStatus.Exited }));
  const oldPatch = handlers.get('ReceiveContainerInfoPatch')!;
  useReadMock.mockReturnValue(view('second', 'node-b'));
  rerender({ platform: 'second' });
  act(() => oldPatch({ resourceId, state: ContainerStateStatus.Exited }));
  expect(result.current.containerInfo).toMatchObject({
    platformId: 'second',
    dockerNodeId: 'node-b',
    state: ContainerStateStatus.Running,
  });
  useRealtimeGroupMock.mock.lastCall![0].setupEventListeners(hub);
  act(() =>
    handlers.get('ReceiveContainerInfo')!({
      id: 'same-docker-id',
      dockerNodeId: 'node-b',
      name: 'fresh snapshot',
      state: ContainerStateStatus.Exited,
    }),
  );
  expect(result.current.containerInfo).toMatchObject({
    platformId: 'second',
    name: 'fresh snapshot',
    state: ContainerStateStatus.Exited,
  });
});
