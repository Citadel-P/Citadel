import {
  ContainerStateStatus,
  ResourceControlState,
  StackReleaseStatus,
  type ContainerView,
} from '@/api/generated/api.types';
import type { ContainerStackGroupResource } from './actions';
import { normalizeContainerSelection } from './selection';

const container = (containerId: string): ContainerView =>
  ({
    id: `resource-${containerId}`,
    platformId: 'platform-id',
    containerId,
    name: containerId,
    dockerImageId: 'image-id',
    created: 1,
    updated: 2,
    state: ContainerStateStatus.Running,
    controlState: ResourceControlState.Idle,
    stack: 'demo-project',
    isSystem: false,
    systemRole: null,
    lastStats: null,
    ports: {},
    deploymentId: null,
    stackId: null,
  }) as ContainerView;

describe('normalizeContainerSelection', () => {
  const first = container('container-1');
  const second = container('container-2');
  const group: ContainerStackGroupResource = {
    id: 'stack:demo-project',
    platformId: 'platform-id',
    name: 'demo-project',
    containerId: 'stack:demo-project',
    state: ContainerStateStatus.Running,
    controlState: ResourceControlState.Idle,
    stackId: null,
    stack: 'demo-project',
    lastStats: null,
    ports: {},
    deploymentId: null,
    isSystem: false,
    systemRole: null,
    imageView: null,
    displayStatus: StackReleaseStatus.Healthy,
    containers: [first, second],
    isStackGroup: true,
  };

  it('preserves a selected root stack and removes its automatically selected children', () => {
    expect(normalizeContainerSelection([group, first, second])).toEqual([group]);
  });

  it('preserves individually selected child containers when the root is not selected', () => {
    expect(normalizeContainerSelection([first, second])).toEqual([first, second]);
  });
});
