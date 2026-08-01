import { describe, expect, it } from 'vitest';
import {
  ContainerDataView,
  ContainerStateStatus,
  ContainerSystemRole,
  ContainerView,
  ResourceControlState,
} from '@/api/generated/api.types';
import { mergeContainerRuntimeUpdate, toContainerDetailsView } from './useContainerInfoGroup';

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
      lastStats: null,
      ports: {},
      deploymentId: null,
      stackId: null,
    } satisfies ContainerView);

    expect(details.resourceId).toBe('019f0000-0000-7000-8000-000000000001');
    expect(details.platformId).toBe('019f0000-0000-7000-8000-000000000002');
    expect(details.id).toBe('ea5f935b4706');
  });
});
