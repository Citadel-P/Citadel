import { useCallback, useEffect, useMemo, useState } from 'react';
import { AlertEventView, UnresolvedAlertsCountView } from '@/api/generated/api.types';
import { useRead } from '@/lib/hooks';
import { useSignalRGroup } from '@/hooks/useSignalRGroup';
import { HubConnection } from '@microsoft/signalr';

export function useAlertEventsGroup() {
  const { data } = useRead('getUnresolvedAlertEventsCount');
  const [unresolvedAlertCount, setUnresolvedAlertCount] = useState(0);
  const [liveAlertEvents, setLiveAlertEvents] = useState<Record<string, AlertEventView>>({});
  const [receivedAlertEventIds, setReceivedAlertEventIds] = useState<string[]>([]);

  useEffect(() => {
    const initialCount = Number(data?.data?.count ?? 0);
    if (!Number.isNaN(initialCount)) {
      setUnresolvedAlertCount(initialCount);
    }
  }, [data]);

  const handleAlertEventReceived = useCallback((alertEvent: AlertEventView) => {
    setLiveAlertEvents((prev) => ({ ...prev, [alertEvent.id]: alertEvent }));
    setReceivedAlertEventIds((prev) => [alertEvent.id, ...prev.filter((id) => id !== alertEvent.id)].slice(0, 200));
  }, []);

  const handleAlertEventsUpdated = useCallback((alertEvents: AlertEventView[]) => {
    if (!alertEvents.length) return;

    setLiveAlertEvents((prev) => {
      const next = { ...prev };
      alertEvents.forEach((alertEvent) => {
        next[alertEvent.id] = alertEvent;
      });
      return next;
    });
  }, []);

  const handleUnresolvedAlertCount = useCallback((unresolvedCounts: UnresolvedAlertsCountView) => {
    setUnresolvedAlertCount(Number(unresolvedCounts.count ?? 0));
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
      liveAlertEvents,
      receivedAlertEventIds,
    }),
    [unresolvedAlertCount, liveAlertEvents, receivedAlertEventIds],
  );
}
