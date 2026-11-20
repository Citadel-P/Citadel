import { useCallback, useMemo, useState } from 'react';
import { AlertEventStatus, AlertEventView, Count } from '@/api/generated/api.types';
import { useRead } from '@/lib/hooks';
import { useRealtimeGroup } from '@/hooks/useRealtimeGroup';
import { RealtimeConnection } from '@/lib/realtime-connection';
import { useQueryClient } from '@tanstack/react-query';

export function useAlertEventsGroup() {
  const queryClient = useQueryClient();
  const { data: unresolvedEventsData } = useRead('listAlertEvents', {
    query: { UnresolvedOnly: true, Page: 1, PageSize: 5 },
  });

  const [liveUnresolvedAlertCount, setLiveUnresolvedAlertCount] = useState<number | null>(null);
  // The server sends its latest 100 events. Five older unresolved events can
  // additionally come from the notification query; paginated history stays in React Query.
  const [recent, setRecent] = useState<{ items: AlertEventView[] | null; received: string[] }>({
    items: null,
    received: [],
  });

  const unresolvedAlertCount = useMemo(() => {
    if (liveUnresolvedAlertCount !== null) return liveUnresolvedAlertCount;

    const initialCount = Number(unresolvedEventsData?.data?.pagedResult.totalCount ?? 0);
    return Number.isNaN(initialCount) ? 0 : initialCount;
  }, [unresolvedEventsData, liveUnresolvedAlertCount]);

  const alertEvents = useMemo(() => {
    const seededEvents = unresolvedEventsData?.data?.pagedResult.items ?? [];
    const next: Record<string, AlertEventView> = {};

    if (recent.items?.length === 0) return next;
    seededEvents.slice(0, 5).forEach((alertEvent) => {
      next[alertEvent.id] = alertEvent;
    });

    for (const item of recent.items ?? []) next[item.id] = item;
    return next;
  }, [unresolvedEventsData, recent.items]);

  const handleAlertEventReceived = useCallback((alertEvent: AlertEventView) => {
    setRecent((prev) => {
      const items = [alertEvent, ...(prev.items ?? []).filter((item) => item.id !== alertEvent.id)].slice(0, 100);
      const ids = new Set(items.map((item) => item.id));
      return { items, received: [alertEvent.id, ...prev.received.filter((id) => id !== alertEvent.id && ids.has(id))] };
    });
  }, []);

  const handleAlertEventsUpdated = useCallback(
    (alertEvents: AlertEventView[]) => {
      const items = alertEvents.slice(0, 100);
      const ids = new Set(items.map((event) => event.id));
      setRecent((prev) => ({ items, received: prev.received.filter((id) => ids.has(id)) }));
      const byId = new Map(items.map((event) => [event.id, event]));
      const details = queryClient.getQueryCache().findAll({ queryKey: ['getAlertEvent'], type: 'active' });
      for (const detail of details) {
        const id = (detail.queryKey[1] as { id: string }).id;
        const event = byId.get(id);
        if (event) {
          queryClient.setQueryData<{ data: AlertEventView }>(
            detail.queryKey,
            (cached) => cached && { ...cached, data: event },
          );
        } else {
          // An older open detail can change outside the latest-event window.
          void queryClient.invalidateQueries({ queryKey: detail.queryKey, exact: true });
        }
      }

      if (alertEvents.some((alertEvent) => alertEvent.status === AlertEventStatus.Resolved)) {
        void queryClient.invalidateQueries({ queryKey: ['listAlertEvents'] });
      }
    },
    [queryClient],
  );

  const handleUnresolvedAlertCount = useCallback((unresolvedCounts: Count) => {
    setLiveUnresolvedAlertCount(Number(unresolvedCounts.count ?? 0));
  }, []);

  const setupEventListeners = useCallback(
    (hubConnection: RealtimeConnection) => {
      hubConnection.on('AlertEventReceived', handleAlertEventReceived);
      hubConnection.on('AlertEventsUpdated', handleAlertEventsUpdated);
      hubConnection.on('UnresolvedAlertCount', handleUnresolvedAlertCount);
    },
    [handleAlertEventReceived, handleAlertEventsUpdated, handleUnresolvedAlertCount],
  );

  const removeEventListeners = useCallback(
    (hubConnection: RealtimeConnection) => {
      hubConnection.off('AlertEventReceived', handleAlertEventReceived);
      hubConnection.off('AlertEventsUpdated', handleAlertEventsUpdated);
      hubConnection.off('UnresolvedAlertCount', handleUnresolvedAlertCount);
    },
    [handleAlertEventReceived, handleAlertEventsUpdated, handleUnresolvedAlertCount],
  );

  useRealtimeGroup({
    groupName: 'alert-events',
    setupEventListeners,
    removeEventListeners,
  });

  return useMemo(
    () => ({
      unresolvedAlertCount,
      liveAlertEvents: alertEvents,
      receivedAlertEventIds: recent.received,
    }),
    [unresolvedAlertCount, alertEvents, recent.received],
  );
}
