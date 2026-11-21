import { resourceQueryKey } from '@/api/dispatch';
import type { ResourceResponse } from '@/api/types';
import { act, screen, waitFor } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { renderCitadel } from '@/test/render-citadel';
import { server } from '@/test/server';
import { FakeRealtimeConnection } from '@/test/fakes/realtime';
import { createResourceCapabilities, createStack } from '@/test/factories/resources';
import { useNavigate } from 'react-router';
import { useStacksGroup } from './useStacksGroup';

function StacksProbe() {
  const { stacks, capabilities, refetch } = useStacksGroup();
  const navigate = useNavigate();

  return (
    <>
      <button onClick={() => navigate('/?platformId=00000000-0000-0000-0000-000000000300')}>change platform</button>
      <output>{capabilities?.canWrite ? 'editable' : 'read only'}</output>
      <button onClick={() => void refetch()}>Load permissions</button>
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
    const fake = new FakeRealtimeConnection();
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
      groups: {
        connectionFactory: () => fake.asRealtimeConnection(),
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

  it('updates collection capabilities independently of an unchanged HTTP row array', async () => {
    const fake = new FakeRealtimeConnection();
    const initial = createStack();
    let serverRow = initial;
    server.use(
      http.get('http://localhost/api/v1/stacks', () =>
        HttpResponse.json({
          stacks: [serverRow],
          capabilities: createResourceCapabilities(),
        }),
      ),
    );
    const view = renderCitadel(<StacksProbe />, {
      groups: {
        connectionFactory: () => fake.asRealtimeConnection(),
        startConnection: (connection) => connection.start(),
      },
    });
    await screen.findByText('editable');
    await waitFor(() => expect(fake.listenerCount('StackInfoUpdated')).toBe(1));
    act(() => fake.emit('StackInfoUpdated', { ...initial, name: 'updated live' }, 'update'));
    const key = resourceQueryKey('listStacks');
    const cached = view.queryClient.getQueryData<ResourceResponse<'listStacks'>>(key)!;
    act(() =>
      view.queryClient.setQueryData(key, {
        ...cached,
        data: { ...cached.data, capabilities: { ...cached.data.capabilities, canWrite: false } },
      }),
    );
    expect(view.queryClient.getQueryData<ResourceResponse<'listStacks'>>(key)?.data.stacks).toBe(cached.data.stacks);
    expect(await screen.findByText('read only')).toBeVisible();
    expect(screen.getByText('updated live')).toBeVisible();
    serverRow = { ...initial, name: 'next snapshot' };
    await view.user.click(screen.getByRole('button', { name: 'Load permissions' }));
    expect(await screen.findByText('next snapshot')).toBeVisible();
  });

  it('ignores events outside the active platform filter and removes listeners on unmount', async () => {
    const fake = new FakeRealtimeConnection();
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
      groups: {
        connectionFactory: () => fake.asRealtimeConnection(),
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
