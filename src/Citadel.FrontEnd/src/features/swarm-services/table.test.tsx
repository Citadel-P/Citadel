import { ManagedSwarmServiceView, SwarmTaskView } from '@/api/generated/api.types';
import { renderCitadel } from '@/test/render-citadel';
import { screen } from '@testing-library/react';
import { SwarmServicesTable } from './table';

const platformId = '00000000-0000-0000-0000-000000000200';

describe('SwarmServicesTable', () => {
  it('expands the runtime tasks beneath a managed Service', async () => {
    const { user } = renderCitadel(<SwarmServicesTable items={[service()]} actions={{}} isLoading={false} />);

    expect(screen.queryByRole('link', { name: 'web.1' })).not.toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: 'Expand service web' }));

    expect(screen.getByRole('link', { name: 'web.1' })).toHaveAttribute(
      'href',
      `/platforms/${platformId}/tasks/task-1`,
    );
    expect(screen.getByRole('link', { name: 'manager-1' })).toHaveAttribute(
      'href',
      `/platforms/${platformId}/nodes/node-1`,
    );
    expect(screen.getByText(/running/i)).toBeVisible();
    expect(screen.getAllByRole('checkbox', { name: 'Select Service' })).toHaveLength(1);
  });
});

const service = (): ManagedSwarmServiceView =>
  ({
    id: 'managed-service-1',
    platformId,
    name: 'web',
    dockerName: 'web-managed-service-1',
    dockerServiceId: 'service-1',
    spec: {
      image: { $type: 'External', registryId: 'registry-1', imageTag: 'nginx:latest' },
      schedulingMode: 'Replicated',
      replicas: 1,
    },
    health: 'Healthy',
    controlState: 'Idle',
    platformName: 'Swarm',
    platformStatus: 'Online',
    runningTaskCount: 1,
    desiredTaskCount: 1,
    tags: [],
    tasks: [task()],
  }) as ManagedSwarmServiceView;

const task = (): SwarmTaskView =>
  ({
    id: 'task-1',
    name: 'web.1',
    serviceId: 'service-1',
    serviceName: 'web',
    nodeId: 'node-1',
    nodeHostname: 'manager-1',
    desiredState: 'running',
    state: 'running',
    image: 'nginx:latest',
    isStale: false,
  }) as SwarmTaskView;
