import { SwarmTaskView } from '@/api/generated/api.types';
import { RegularResourceView } from '@/pages/regular-resource';
import { FakeRealtimeConnection } from '@/test/fakes/realtime';
import { renderCitadel } from '@/test/render-citadel';
import { server } from '@/test/server';
import { act, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { http, HttpResponse } from 'msw';
import { Route, Routes } from 'react-router';
import { TaskComponents } from '.';

vi.mock('@/components/custom/task-sheet', () => ({ default: () => null }));

const platformId = '00000000-0000-0000-0000-000000000200';

describe('TaskComponents', () => {
  it('filters tasks by their displayed name', async () => {
    const fake = new FakeRealtimeConnection();
    server.use(
      http.get(`http://localhost/api/v1/platforms/${platformId}/swarm/tasks`, () =>
        HttpResponse.json({
          items: [createTask({ id: 'task-1', name: 'frontend.1' }), createTask({ id: 'task-2', name: '', slot: 7 })],
        }),
      ),
    );

    renderCitadel(
      <Routes>
        <Route
          path="/platforms/:platformId/tasks"
          element={<RegularResourceView Components={TaskComponents} type="Task" showTaskSheet={false} />}
        />
      </Routes>,
      {
        route: `/platforms/${platformId}/tasks`,
        groups: {
          connectionFactory: () => fake.asRealtimeConnection(),
          startConnection: (connection) => connection.start(),
        },
      },
    );

    expect(await screen.findByRole('link', { name: 'frontend.1' })).toBeVisible();
    expect(screen.getByRole('link', { name: 'web.7' })).toBeVisible();

    await userEvent.type(screen.getByRole('searchbox', { name: 'Search resources by name' }), 'web.7');

    await waitFor(() => expect(screen.queryByRole('link', { name: 'frontend.1' })).not.toBeInTheDocument());
    expect(screen.getByRole('link', { name: 'web.7' })).toBeVisible();
  });

  it('applies realtime additions, updates, and removals without refetching', async () => {
    const fake = new FakeRealtimeConnection();
    let requestCount = 0;
    server.use(
      http.get(`http://localhost/api/v1/platforms/${platformId}/swarm/tasks`, () => {
        requestCount++;
        return HttpResponse.json({
          items: [createTask({ id: 'task-1', name: 'web.1' }), createTask({ id: 'task-2', name: 'web.2' })],
        });
      }),
    );

    renderCitadel(
      <Routes>
        <Route
          path="/platforms/:platformId/tasks"
          element={<RegularResourceView Components={TaskComponents} type="Task" showTaskSheet={false} />}
        />
      </Routes>,
      {
        route: `/platforms/${platformId}/tasks`,
        groups: {
          connectionFactory: () => fake.asRealtimeConnection(),
          startConnection: (connection) => connection.start(),
        },
      },
    );

    expect(await screen.findByRole('link', { name: 'web.1' })).toHaveAttribute(
      'href',
      `/platforms/${platformId}/tasks/task-1`,
    );
    expect(screen.getAllByRole('link', { name: 'web' })[0]).toHaveAttribute(
      'href',
      `/platforms/${platformId}/services/service-1`,
    );
    expect(screen.getAllByRole('link', { name: 'manager-1' })[0]).toHaveAttribute(
      'href',
      `/platforms/${platformId}/nodes/node-1`,
    );
    await waitFor(() => expect(fake.listenerCount('SwarmInventoryUpdated')).toBe(1));

    act(() => {
      fake.emit('SwarmInventoryUpdated', {
        platformId,
        nodes: { items: [] },
        services: { items: [] },
        tasks: {
          items: [
            createTask({ id: 'task-2', name: 'web.2', state: 'Failed', desiredState: 'Shutdown' }),
            createTask({ id: 'task-3', name: 'web.3' }),
          ],
        },
        networks: { items: [] },
        secrets: { items: [] },
        configs: { items: [] },
      });
    });

    await waitFor(() => expect(screen.getByRole('link', { name: 'web.3' })).toBeVisible());
    expect(screen.queryByRole('link', { name: 'web.1' })).not.toBeInTheDocument();
    expect(screen.getByRole('link', { name: 'web.2' })).toBeVisible();
    expect(screen.getByText('Shutdown')).toHaveClass('border-border', 'text-muted-foreground');
    expect(requestCount).toBe(1);
  });
});

function createTask(overrides: Partial<SwarmTaskView> = {}): SwarmTaskView {
  return {
    id: 'task-1',
    versionIndex: 1,
    name: 'web.1',
    serviceId: 'service-1',
    serviceName: 'web',
    slot: 1,
    nodeId: 'node-1',
    nodeHostname: 'manager-1',
    desiredState: 'Running',
    state: 'Running',
    statusMessage: null,
    error: null,
    image: 'nginx:latest',
    ports: ['80/tcp'],
    statusTimestamp: '2026-08-04T12:00:00Z',
    createdAt: '2026-08-04T11:59:00Z',
    updatedAt: '2026-08-04T12:00:00Z',
    observedAt: '2026-08-04T12:00:00Z',
    isStale: false,
    capabilities: null,
    ...overrides,
  };
}
