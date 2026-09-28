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
import { server } from '@/test/server';
import { useAlertEventsGroup } from './useAlertEventsGroup';

function AlertEventsProbe() {
  const { liveAlertEvents } = useAlertEventsGroup();
  const ids = Object.values(liveAlertEvents)
    .filter((event) => event.status !== AlertEventStatus.Resolved)
    .sort((a, b) => new Date(b.createdAt).getTime() - new Date(a.createdAt).getTime())
    .slice(0, 5)
    .map((event) => event.id);

  return <span data-testid="alert-ids">{ids.join(',')}</span>;
}

describe('useAlertEventsGroup', () => {
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
