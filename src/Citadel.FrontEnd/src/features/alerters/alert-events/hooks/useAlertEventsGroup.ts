import { useCallback, useMemo, useState } from 'react';
import { AlertEventStatus, AlertEventView, UnresolvedAlertsCountView } from '@/api/generated/api.types';
import { useRead } from '@/lib/hooks';
import { useSignalRGroup } from '@/hooks/useSignalRGroup';
import { HubConnection } from '@microsoft/signalr';
import { useQueryClient } from '@tanstack/react-query';

export function useAlertEventsGroup() {
  const queryClient = useQueryClient();
  const { data: unresolvedEventsData } = useRead('listAlertEvents', {
    query: { UnresolvedOnly: true, Page: 1, PageSize: 5 },
  });

  const [liveUnresolvedAlertCount, setLiveUnresolvedAlertCount] = useState<number | null>(null);
  const [liveAlertEvents, setLiveAlertEvents] = useState<Record<string, AlertEventView>>({});
  const [receivedAlertEventIds, setReceivedAlertEventIds] = useState<string[]>([]);

  const unresolvedAlertCount = useMemo(() => {
    if (liveUnresolvedAlertCount !== null) return liveUnresolvedAlertCount;

    const initialCount = Number(unresolvedEventsData?.data?.pagedResult.totalCount ?? 0);
    return Number.isNaN(initialCount) ? 0 : initialCount;
  }, [unresolvedEventsData, liveUnresolvedAlertCount]);

  const alertEvents = useMemo(() => {
    const seededEvents = unresolvedEventsData?.data?.pagedResult.items ?? [];
    const next: Record<string, AlertEventView> = {};

    seededEvents.forEach((alertEvent) => {
      next[alertEvent.id] = alertEvent;
    });

    return { ...next, ...liveAlertEvents };
  }, [unresolvedEventsData, liveAlertEvents]);

  const handleAlertEventReceived = useCallback((alertEvent: AlertEventView) => {
    setLiveAlertEvents((prev) => ({ ...prev, [alertEvent.id]: alertEvent }));
    setReceivedAlertEventIds((prev) => [alertEvent.id, ...prev.filter((id) => id !== alertEvent.id)].slice(0, 200));
  }, []);

  const handleAlertEventsUpdated = useCallback(
    (alertEvents: AlertEventView[]) => {
      if (!alertEvents.length) return;

      setLiveAlertEvents((prev) => {
        const next = { ...prev };
        alertEvents.forEach((alertEvent) => {
          next[alertEvent.id] = alertEvent;
        });
        return next;
      });

      if (alertEvents.some((alertEvent) => alertEvent.status === AlertEventStatus.Resolved)) {
        void queryClient.invalidateQueries({ queryKey: ['listAlertEvents'] });
      }
    },
    [queryClient],
  );

  const handleUnresolvedAlertCount = useCallback((unresolvedCounts: UnresolvedAlertsCountView) => {
    setLiveUnresolvedAlertCount(Number(unresolvedCounts.count ?? 0));
  }, []);

  const setupEventListeners = useCallback(
    (hubConnection: HubConnection) => {
      hubConnection.on('AlertEventReceived', handleAlertEventReceived);
      hubConnection.on('AlertEventsUpdated', handleAlertEventsUpdated);
      hubConnection.on('UnresolvedAlertCount', handleUnresolvedAlertCount);
    },
    [handleAlertEventReceived, handleAlertEventsUpdated, handleUnresolvedAlertCount],
  );

  const removeEventListeners = useCallback(
    (hubConnection: HubConnection) => {
      hubConnection.off('AlertEventReceived', handleAlertEventReceived);
      hubConnection.off('AlertEventsUpdated', handleAlertEventsUpdated);
      hubConnection.off('UnresolvedAlertCount', handleUnresolvedAlertCount);
    },
    [handleAlertEventReceived, handleAlertEventsUpdated, handleUnresolvedAlertCount],
  );

  useSignalRGroup({
    groupName: 'alert-events',
    setupEventListeners,
    removeEventListeners,
  });

  return useMemo(
    () => ({
      unresolvedAlertCount,
      liveAlertEvents: alertEvents,
      receivedAlertEventIds,
    }),
    [unresolvedAlertCount, alertEvents, receivedAlertEventIds],
  );
}
