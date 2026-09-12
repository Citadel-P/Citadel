import { act, screen, waitFor } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import { ActivityEventType, ActivityResourceType, ActivityStatus } from '@/api/generated/api.types';
import { renderCitadel } from '@/test/render-citadel';
import { server } from '@/test/server';
import { FakeRealtimeConnection } from '@/test/fakes/realtime';
import { createStack } from '@/test/factories/resources';
import { useStackGroup } from './useStackGroup';

const initial = createStack();
const activity = {
  id: '00000000-0000-0000-0000-000000000002',
  resourceType: ActivityResourceType.Stack,
  eventType: ActivityEventType.StackDegraded,
  status: ActivityStatus.Warning,
  createdAt: '2026-09-09T12:00:00Z',
};
const info = { $type: 'StackDegraded', reason: 'Service is unavailable' };

function Probe() {
  const { stack } = useStackGroup(initial.id);
  return <output data-testid="activity">{JSON.stringify(stack?.latestActivityView ?? null)}</output>;
}

describe('useStackGroup activity updates', () => {
  it.each([
    ['Rust JSON', info],
    ['.NET MessagePack', ['StackDegraded', { reason: info.reason }]],
    ['no activity', null],
  ])('preserves %s activity details', async (_format, wireInfo) => {
    const fake = new FakeRealtimeConnection();
    server.use(http.get('http://localhost/api/v1/stacks/:id', () => HttpResponse.json(initial)));
    renderCitadel(<Probe />, {
      groups: {
        connectionFactory: () => fake.asRealtimeConnection(),
        startConnection: (connection) => connection.start(),
      },
    });
    await waitFor(() => expect(fake.listenerCount('StackInfoUpdated')).toBe(1));
    const event = {
      ...initial,
      latestActivityView: wireInfo ? { ...activity, info: structuredClone(wireInfo) } : null,
    };
    const originalEvent = structuredClone(event);
    act(() => fake.emit('StackInfoUpdated', event));
    await waitFor(() =>
      expect(JSON.parse(screen.getByTestId('activity').textContent!)).toEqual(wireInfo ? { ...activity, info } : null),
    );
    expect(event).toEqual(originalEvent);
  });
});
