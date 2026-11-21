import { act, screen, waitFor } from '@testing-library/react';
import { http, HttpResponse } from 'msw';
import {
  AlertEventStatus,
  AlertResourceType,
  AlertSeverity,
  AlertType,
  type AlertEventView,
} from '@/api/generated/api.types';
import { FakeRealtimeConnection } from '@/test/fakes/realtime';
import { renderCitadel } from '@/test/render-citadel';
import { AppContext } from '@/lib/context/app-context';
import { useAlertEventsList } from './useAlertEventsList';
import { useRead } from '@/lib/hooks';
import { server } from '@/test/server';
import { useAlertEventsGroup } from './useAlertEventsGroup';

function AlertEventsProbe() {
  const { liveAlertEvents, receivedAlertEventIds } = useAlertEventsGroup();
  const ids = Object.values(liveAlertEvents)
    .filter((event) => event.status !== AlertEventStatus.Resolved)
    .sort((a, b) => new Date(b.createdAt).getTime() - new Date(a.createdAt).getTime())
    .slice(0, 5)
    .map((event) => event.id);

  return (
    <>
      <span data-testid="alert-ids">{ids.join(',')}</span>
      <span data-testid="size">
        {Object.keys(liveAlertEvents).length}/{receivedAlertEventIds.length}
      </span>
    </>
  );
}

describe('useAlertEventsGroup', () => {
  it('bounds payloads and IDs, replaces replay windows, and keeps an open detail current after eviction', async () => {
    const fake = new FakeRealtimeConnection();
    const first = createAlert(1);
    let detail = first;
    const older = Array.from({ length: 5 }, (_, i) => createAlert(10001 + i));
    server.use(
      http.get('http://localhost/api/v1/alertEvents', () =>
        HttpResponse.json({ pagedResult: { items: older, totalCount: 5 } }),
      ),
      http.get('http://localhost/api/v1/alertEvents/:id', () => HttpResponse.json(detail)),
    );
    function Detail() {
      const { data } = useRead('getAlertEvent', { id: first.id });
      return <span>{data?.data?.status}</span>;
    }
    renderCitadel(
      <>
        <AlertEventsProbe />
        <Detail />
      </>,
      {
        groups: {
          connectionFactory: () => fake.asRealtimeConnection(),
          startConnection: (c) => c.start(),
        },
      },
    );
    await screen.findByText(AlertEventStatus.Active);
    await waitFor(() => expect(screen.getByTestId('size')).toHaveTextContent('5/0'));
    await waitFor(() => expect(fake.listenerCount('AlertEventsUpdated')).toBe(1));
    act(() => {
      for (let i = 1; i <= 5000; i++) fake.emit('AlertEventReceived', createAlert(i));
    });
    expect(screen.getByTestId('size')).toHaveTextContent('105/100');
    detail = { ...first, status: AlertEventStatus.Resolved };
    act(() => fake.emit('AlertEventsUpdated', [detail]));
    expect(screen.getByTestId('size')).toHaveTextContent('6/0');
    await screen.findByText(AlertEventStatus.Resolved);
    act(() => fake.emit('AlertEventsUpdated', []));
    expect(screen.getByTestId('size')).toHaveTextContent('0/0');
    await screen.findByText(AlertEventStatus.Resolved);
  });
  it('preserves acknowledgment in a filtered page and notification seed after live eviction', async () => {
    const fake = new FakeRealtimeConnection();
    const first = createAlert(1);
    const notification = { ...createAlert(102), resourceId: 'notification-resource' };
    const unrelated = Array.from({ length: 100 }, (_, i) => ({ ...createAlert(2 + i), resourceId: 'other' }));
    let requests = 0;
    server.use(
      http.get('http://localhost/api/v1/alertEvents', ({ request }) => {
        requests++;
        const items = new URL(request.url).searchParams.has('ResourceId') ? [first] : [notification];
        return HttpResponse.json({ pagedResult: { items, totalCount: 1, page: 1, pageSize: 5 } });
      }),
    );
    function FilteredList() {
      const { pagedAlertEvents } = useAlertEventsList(first.resourceId!);
      return <span data-testid="filtered">{pagedAlertEvents?.items[0]?.status}</span>;
    }
    function MountedAlerts() {
      const live = useAlertEventsGroup();
      return (
        <AppContext.Provider
          value={{
            ...live,
            isLoading: false,
            currentPlatform: undefined,
            platforms: undefined,
            applicationInfo: undefined,
          }}>
          <FilteredList />
          <span data-testid="notification">{live.liveAlertEvents[notification.id]?.status}</span>
        </AppContext.Provider>
      );
    }
    renderCitadel(<MountedAlerts />, {
      groups: { connectionFactory: () => fake.asRealtimeConnection(), startConnection: (c) => c.start() },
    });
    const expectStatus = (status: AlertEventStatus) => {
      expect(screen.getByTestId('filtered')).toHaveTextContent(status);
      expect(screen.getByTestId('notification')).toHaveTextContent(status);
    };
    await waitFor(() => expectStatus(AlertEventStatus.Active));
    act(() =>
      fake.emit(
        'AlertEventsUpdated',
        [first, notification].map((event) => ({ ...event, status: AlertEventStatus.Acknowledged })),
      ),
    );
    await waitFor(() => expectStatus(AlertEventStatus.Acknowledged));
    act(() => unrelated.forEach((event) => fake.emit('AlertEventReceived', event)));
    expectStatus(AlertEventStatus.Acknowledged);
    expect(requests).toBe(2);
  });

  it('refills the unresolved alert list after a visible alert is resolved', async () => {
    const fake = new FakeRealtimeConnection();
    const alerts = Array.from({ length: 6 }, (_, index) => createAlert(index + 1));
    let requestCount = 0;

    server.use(
      http.get('http://localhost/api/v1/alertEvents', () => {
        requestCount += 1;
        const items = requestCount === 1 ? alerts.slice(0, 5) : alerts.slice(1, 6);
        return HttpResponse.json({
          pagedResult: {
            items,
            totalCount: requestCount === 1 ? 6 : 5,
            page: 1,
            pageSize: 5,
          },
        });
      }),
    );

    renderCitadel(<AlertEventsProbe />, {
      groups: {
        connectionFactory: () => fake.asRealtimeConnection(),
        startConnection: (connection) => connection.start(),
      },
    });

    expect(await screen.findByTestId('alert-ids')).toHaveTextContent(
      alerts
        .slice(0, 5)
        .map((alert) => alert.id)
        .join(','),
    );
    await waitFor(() => expect(fake.listenerCount('AlertEventsUpdated')).toBe(1));

    act(() => {
      fake.emit('AlertEventsUpdated', [{ ...alerts[0], status: AlertEventStatus.Resolved }]);
    });

    await waitFor(() => expect(requestCount).toBe(2));
    await waitFor(() =>
      expect(screen.getByTestId('alert-ids')).toHaveTextContent(
        alerts
          .slice(1, 6)
          .map((alert) => alert.id)
          .join(','),
      ),
    );
    expect(screen.getByTestId('alert-ids')).not.toHaveTextContent(alerts[0].id);
  });
});

function createAlert(index: number): AlertEventView {
  return {
    id: `00000000-0000-0000-0000-${index.toString().padStart(12, '0')}`,
    alertRuleId: '10000000-0000-0000-0000-000000000001',
    type: AlertType.PlatformUnreachable,
    severity: AlertSeverity.Warning,
    status: AlertEventStatus.Active,
    message: `Alert ${index}`,
    info: {
      $type: AlertType.PlatformUnreachable,
      platformName: 'Platform',
      id: 'platform-id',
      address: 'http://localhost',
      humanMessage: `Alert ${index}`,
    },
    resourceId: '20000000-0000-0000-0000-000000000001',
    resourceName: 'Platform',
    resourceType: AlertResourceType.Platform,
    resourcePath: null,
    acknowledgedByActorId: null,
    acknowledgedAt: null,
    resolvedByActorId: null,
    resolvedAt: null,
    actorId: null,
    actorName: null,
    actorType: null,
    resolutionNote: null,
    createdAt: new Date(Date.UTC(2026, 0, 1, 0, 0, 10 - index)).toISOString(),
    updatedAt: new Date(Date.UTC(2026, 0, 1, 0, 0, 10 - index)).toISOString(),
  };
}
