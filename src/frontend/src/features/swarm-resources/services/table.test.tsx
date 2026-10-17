import { createSwarmTask, createWorkloadHealth } from '@/test/factories/resources';
import {
  CheckConfiguration,
  HealthStatus,
  PlatformStatus,
  ContainerRuntimeView,
  ContainerStateStatus,
  SwarmServiceOwnership,
  SwarmTaskView,
} from '@/api/generated/api.types';
import { renderCitadel } from '@/test/render-citadel';
import { screen, within } from '@testing-library/react';
import { SwarmServiceListView } from './hooks/useServicesGroup';
import { ServicesTable } from './table';

describe('ServicesTable', () => {
  it('shows streamed task CPU and memory when runtime containers are supplied', async () => {
    const task = {
      ...createSwarmTask(),
      id: 'task-1',
      name: 'web.1',
      serviceId: 'service-1',
      serviceName: 'web',
      nodeId: 'node-1',
      nodeHostname: 'worker-1',
      desiredState: 'Running',
      state: 'Running',
      image: 'nginx:latest',
      ports: [],
      isStale: false,
    } as SwarmTaskView;
    const service = {
      id: 'service-1',
      name: 'web',
      mode: 'Replicated',
      image: 'nginx:latest',
      runningTaskCount: 1,
      desiredTaskCount: 1,
      updateState: 'Completed',
      labels: {},
      ownership: SwarmServiceOwnership.CitadelStack,
      ownershipDiagnostic: null,
      dockerStackNamespace: 'demo',
      stackId: null,
      isStale: false,
      tasks: [task],
    } as SwarmServiceListView;
    const container = {
      id: 'abcdef012345',
      name: '/web.1.task-1',
      state: ContainerStateStatus.Running,
      isSwarmTask: true,
      containerStat: {
        cpuUsage: 12.5,
        memoryActive: 128,
        memoryLimit: 512,
      },
    } as ContainerRuntimeView;

    const { user } = renderCitadel(
      <ServicesTable
        items={[service]}
        isLoading={false}
        actions={{}}
        platformId="platform-1"
        selectable={false}
        showActions={false}
        taskContainers={[container]}
        healthResource={{
          id: 'stack',
          name: 'demo',
          platformStatus: PlatformStatus.Online,
          probeHealth: createWorkloadHealth({
            evaluatedAt: Date.now(),
            members: [
              {
                platformId: 'platform-1',
                service: 'service-1',
                taskId: 'task-1',
                nodeId: 'node-1',
                name: 'web.1',
                available: true,
                state: 'Running',
                status: HealthStatus.Healthy,
                observation: { observedAt: Date.now(), configuration: CheckConfiguration.Enabled },
              },
            ],
          }),
        }}
      />,
    );

    expect(screen.getByRole('columnheader', { name: 'Health' })).toBeVisible();
    expect(screen.getByRole('columnheader', { name: 'CPU' })).toBeVisible();
    expect(screen.getByRole('columnheader', { name: 'Memory' })).toBeVisible();
    const serviceRow = screen.getByRole('link', { name: 'web' }).closest('tr');
    expect(serviceRow).not.toBeNull();
    expect(within(serviceRow!).getByRole('button', { name: 'Health: Healthy' })).toBeVisible();
    expect(within(serviceRow!).getByText(/12.*50/)).toBeVisible();
    expect(within(serviceRow!).getByText(/128.*512/)).toBeVisible();
    await user.click(screen.getByRole('button', { name: 'Expand service web' }));
    const taskRow = screen.getByRole('link', { name: 'web.1' }).closest('tr');
    expect(taskRow).not.toBeNull();
    expect(within(taskRow!).getByRole('button', { name: 'Health: Healthy' })).toBeVisible();
    expect(within(taskRow!).getByText(/12.*50/)).toBeVisible();
    expect(within(taskRow!).getByText(/128.*512/)).toBeVisible();
  });
});
