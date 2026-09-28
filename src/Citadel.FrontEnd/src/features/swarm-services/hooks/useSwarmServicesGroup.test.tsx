import { createManagedSwarmService } from '@/test/factories/resources';
import { ManagedSwarmServiceView, SwarmTaskView } from '@/api/generated/api.types';
import { FakeRealtimeConnection } from '@/test/fakes/realtime';
import { renderCitadel } from '@/test/render-citadel';
import { server } from '@/test/server';
import { act, screen, waitFor } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { useSwarmServicesGroup } from './useSwarmServicesGroup';

const platformId = '00000000-0000-0000-0000-000000000200';

describe('useSwarmServicesGroup', () => {
  it('keeps managed Service tasks synchronized from the Swarm inventory stream', async () => {
    const fake = new FakeRealtimeConnection();
    server.use(
      http.get('http://localhost/api/v1/swarmServices', () =>
        HttpResponse.json({
          swarmServices: [service([task('task-1', 'web.1')])],
          capabilities: { canRead: true, canWrite: true, canExecute: true },
        }),
      ),
    );

    renderCitadel(<TaskNames />, {
      groups: {
        connectionFactory: () => fake.asRealtimeConnection(),
        startConnection: (connection) => connection.start(),
      },
    });

    expect(await screen.findByText('web.1')).toBeVisible();
    await waitFor(() => expect(fake.listenerCount('SwarmInventoryUpdated')).toBe(1));

    act(() => {
      fake.emit('SwarmInventoryUpdated', {
        platformId,
        services: {
          items: [{ id: 'docker-service-1', runningTaskCount: 1, desiredTaskCount: 1, updateState: 'completed' }],
        },
        tasks: { items: [task('task-2', 'web.2')] },
      });
    });

    expect(await screen.findByText('web.2')).toBeVisible();
    expect(screen.queryByText('web.1')).not.toBeInTheDocument();
  });
});

const TaskNames = () => {
  const { services } = useSwarmServicesGroup();
  return (
    <>
      {services
        ?.flatMap((service) => service.tasks ?? [])
        .map((task) => (
          <span key={task.id}>{task.name}</span>
        ))}
    </>
  );
};

const service = (tasks: SwarmTaskView[]): ManagedSwarmServiceView =>
  ({
    ...createManagedSwarmService(),
    id: 'managed-service-1',
    platformId,
    name: 'web',
    dockerName: 'web-managed-service-1',
    dockerServiceId: 'docker-service-1',
    spec: { image: { $type: 'External', registryId: 'registry-1', imageTag: 'nginx:latest' } },
    tags: [],
    tasks,
  }) as ManagedSwarmServiceView;

const task = (id: string, name: string): SwarmTaskView =>
  ({
    id,
    name,
    serviceId: 'docker-service-1',
    serviceName: 'web',
    nodeId: 'node-1',
    nodeHostname: 'manager-1',
    desiredState: 'running',
    state: 'running',
    image: 'nginx:latest',
    isStale: false,
  }) as SwarmTaskView;
