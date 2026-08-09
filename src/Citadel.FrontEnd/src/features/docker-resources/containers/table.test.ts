vi.hoisted(() => {
  document.queryCommandSupported ??= () => false;
});

import {
  ContainerStateStatus,
  ResourceControlState,
  StackReleaseStatus,
  type ContainerView,
} from '@/api/generated/api.types';
import type { ContainerStackGroupResource } from './actions';
import { countSelectedContainers, filterVisibleContainers, normalizeContainerSelection } from './selection';
import { buildContainerRows } from './table';

vi.mock('./actions', () => ({
  isContainerStackGroup: (resource: { isStackGroup?: boolean }) => resource.isStackGroup === true,
}));
vi.mock('./container-info/', () => ({ ImageName: () => null }));

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
    isSwarmTask: false,
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
    isSwarmTask: false,
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

  it('counts the containers represented by a selected stack root', () => {
    expect(countSelectedContainers([group])).toBe(2);
  });
});

describe('buildContainerRows', () => {
  it('groups Swarm task containers under their Docker Stack namespace', () => {
    const first = { ...container('task-container-1'), stack: 'redis-test', isSwarmTask: true };
    const second = { ...container('task-container-2'), stack: 'redis-test', isSwarmTask: true };
    const stoppedTasks = Array.from({ length: 4 }, (_, index) => ({
      ...container(`old-task-container-${index + 1}`),
      stack: 'redis-test',
      isSwarmTask: true,
      state: ContainerStateStatus.Exited,
    }));

    const rows = buildContainerRows([first, second, ...stoppedTasks], { memoryTotal: 0, cpuCount: 0 });
    const visibleContainers = filterVisibleContainers([first, second, ...stoppedTasks]);

    expect(visibleContainers).toEqual([first, second]);
    expect(rows).toHaveLength(1);
    expect(rows[0]).toMatchObject({
      name: 'redis-test',
      stack: 'redis-test',
      isStackGroup: true,
      isSwarmTask: true,
    });
    expect((rows[0] as ContainerStackGroupResource).containers).toEqual([first, second]);
    expect((rows[0] as ContainerStackGroupResource).displayStatus).toBe(StackReleaseStatus.Healthy);
  });

  it('keeps stopped Docker Compose containers in standalone stack groups', () => {
    const running = container('compose-container-1');
    const stopped = { ...container('compose-container-2'), state: ContainerStateStatus.Exited };

    const rows = buildContainerRows([running, stopped], { memoryTotal: 0, cpuCount: 0 });

    expect(filterVisibleContainers([running, stopped])).toEqual([running, stopped]);
    expect(rows).toHaveLength(1);
    expect((rows[0] as ContainerStackGroupResource).containers).toEqual([running, stopped]);
    expect((rows[0] as ContainerStackGroupResource).displayStatus).toBe(StackReleaseStatus.Degraded);
  });
});
