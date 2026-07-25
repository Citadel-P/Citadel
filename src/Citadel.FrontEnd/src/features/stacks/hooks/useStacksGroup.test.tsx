import { act, screen, waitFor } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { renderCitadel } from '@/test/render-citadel';
import { server } from '@/test/server';
import { FakeHubConnection } from '@/test/fakes/signalr';
import { createResourceCapabilities, createStack } from '@/test/factories/resources';
import { useNavigate } from 'react-router';
import { useStacksGroup } from './useStacksGroup';

function StacksProbe() {
  const { stacks } = useStacksGroup();
  const navigate = useNavigate();

  return (
    <>
      <button onClick={() => navigate('/?platformId=00000000-0000-0000-0000-000000000300')}>
        change platform
      </button>
      <ul>
        {(stacks ?? []).map((stack) => (
          <li key={stack.id}>{stack.name}</li>
        ))}
      </ul>
    </>
  );
}

describe('useStacksGroup', () => {
  it('applies create, update, and delete events without duplicating list items', async () => {
    const fake = new FakeHubConnection();
    const initialStack = createStack();
    server.use(
      http.get('http://localhost/api/v1/stacks', () =>
        HttpResponse.json({
          stacks: [initialStack],
          capabilities: createResourceCapabilities(),
        }),
      ),
    );

    renderCitadel(<StacksProbe />, {
      signalR: {
        connectionFactory: () => fake.asHubConnection(),
        startConnection: (connection) => connection.start(),
      },
    });

    expect(await screen.findByText('api')).toBeVisible();
    await waitFor(() => {
      expect(fake.listenerCount('StackInfoUpdated')).toBe(1);
    });

    const worker = createStack({
      id: '00000000-0000-0000-0000-000000000002',
      name: 'worker',
    });
    act(() => {
      fake.emit('StackInfoUpdated', worker, 'create');
      fake.emit('StackInfoUpdated', { ...worker, name: 'worker-updated' }, 'create');
    });

    expect(screen.queryByText('worker')).not.toBeInTheDocument();
    expect(screen.getAllByText('worker-updated')).toHaveLength(1);

    act(() => {
      fake.emit('StackInfoUpdated', { ...initialStack, name: 'api-updated' }, 'update');
    });
    expect(screen.getByText('api-updated')).toBeVisible();

    act(() => {
      fake.emit('StackInfoUpdated', worker, 'delete');
    });
    expect(screen.queryByText('worker-updated')).not.toBeInTheDocument();
  });

  it('ignores events outside the active platform filter and removes listeners on unmount', async () => {
    const fake = new FakeHubConnection();
    const platformId = '00000000-0000-0000-0000-000000000200';
    const nextPlatformId = '00000000-0000-0000-0000-000000000300';
    server.use(
      http.get('http://localhost/api/v1/stacks', ({ request }) => {
        const requestedPlatformId = new URL(request.url).searchParams.get('platformId');
        expect([platformId, nextPlatformId]).toContain(requestedPlatformId);
        return HttpResponse.json({
          stacks: [
            createStack({
              platformId: requestedPlatformId,
              name: requestedPlatformId === platformId ? 'api' : 'next-api',
            }),
          ],
          capabilities: createResourceCapabilities(),
        });
      }),
    );

    const view = renderCitadel(<StacksProbe />, {
      route: `/?platformId=${platformId}`,
      signalR: {
        connectionFactory: () => fake.asHubConnection(),
        startConnection: (connection) => connection.start(),
      },
    });

    expect(await screen.findByText('api')).toBeVisible();
    await waitFor(() => {
      expect(fake.listenerCount('StackInfoUpdated')).toBe(1);
    });

    act(() => {
      fake.emit(
        'StackInfoUpdated',
        createStack({
          id: '00000000-0000-0000-0000-000000000003',
          name: 'other-platform',
          platformId: '00000000-0000-0000-0000-000000000999',
        }),
        'create',
      );
    });
    expect(screen.queryByText('other-platform')).not.toBeInTheDocument();

    await view.user.click(screen.getByRole('button', { name: 'change platform' }));
    expect(await screen.findByText('next-api')).toBeVisible();
    await waitFor(() => {
      expect(fake.listenerCount('StackInfoUpdated')).toBe(1);
    });

    act(() => {
      fake.emit(
        'StackInfoUpdated',
        createStack({
          id: '00000000-0000-0000-0000-000000000004',
          name: 'stale-platform-event',
          platformId,
        }),
        'create',
      );
    });
    expect(screen.queryByText('stale-platform-event')).not.toBeInTheDocument();

    view.unmount();
    expect(fake.listenerCount('StackInfoUpdated')).toBe(0);
  });
});
