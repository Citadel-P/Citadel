import { useMemo } from 'react';
import { AlertEventStatus, AlertEventView, type AlertResourceType } from '@/api/generated/api.types';
import { useRead } from '@/lib/hooks';
import { useAlertEventQuery, type AlertEventQueryState } from '@/lib/atoms';
import { useAppContext } from '@/lib/context/app-context';

export const useAlertEventsList = (resourceId?: string, resourceType?: AlertResourceType, pageSize?: number) => {
  const [query] = useAlertEventQuery();
  const filters = useMemo(
    () => ({
      ...query,
      resourceId: resourceId ?? query.resourceId,
      resourceType: resourceType ?? query.resourceType,
      pageSize: pageSize ?? query.pageSize,
    }),
    [query, resourceId, resourceType, pageSize],
  );
  return useAlertPage(filters);
};

export const useAlertPage = (
  query: AlertEventQueryState,
  options?: { enabled: boolean; staleTime: number; refetchInterval?: number | false },
  tags?: string[],
) => {
  const { liveAlertEvents, receivedAlertEventIds, alertFeedConnected } = useAppContext();

  const { data, isLoading, error, refetch, isFetching } = useRead(
    'listAlertEvents',
    {
      query: {
        tags,
        Page: query.page,
        PageSize: query.pageSize,
        ResourceType: query.resourceType === 'All' ? undefined : query.resourceType,
        AlertType: query.alertType === 'All' ? undefined : query.alertType,
        ResourceId: query.resourceId,
        UnresolvedOnly: query.unresolvedOnly,
      },
    },
    options,
  );

  const serverPagedAlertEvents = data?.data?.pagedResult;

  const pagedAlertEvents = useMemo(() => {
    // Global notifications do not establish membership in a tagged page.
    if (tags?.length || alertFeedConnected === false) return serverPagedAlertEvents;
    if (!serverPagedAlertEvents) return undefined;

    const pageSizeValue = Number(serverPagedAlertEvents.pageSize ?? 0) || serverPagedAlertEvents.items.length || 1;

    const seen = new Set<string>();

    const matchesFilters = (alertEvent: AlertEventView) => {
      const targetResourceType = query.resourceType;
      const targetResourceId = query.resourceId;

      if (targetResourceType !== 'All' && alertEvent.resourceType !== targetResourceType) return false;
      if (query.alertType !== 'All' && alertEvent.type !== query.alertType) return false;
      if (targetResourceId && alertEvent.resourceId !== targetResourceId) return false;
      if (query.unresolvedOnly && alertEvent.status === AlertEventStatus.Resolved) return false;

      return true;
    };

    const receivedItems = receivedAlertEventIds
      .map((id) => liveAlertEvents[id])
      .filter((a): a is AlertEventView => !!a)
      .filter(matchesFilters)
      .filter((a) => {
        if (seen.has(a.id)) return false;
        seen.add(a.id);
        return true;
      });

    const baseItems = (serverPagedAlertEvents.items ?? [])
      .map((item) => liveAlertEvents[item.id] ?? item)
      .filter(matchesFilters)
      .filter((a) => {
        if (seen.has(a.id)) return false;
        seen.add(a.id);
        return true;
      });

    const removedVisibleItems = (serverPagedAlertEvents.items ?? []).filter((item) => {
      const alertEvent = liveAlertEvents[item.id] ?? item;
      return !matchesFilters(alertEvent);
    }).length;

    const addedReceivedItems = receivedItems.filter(
      (a) => !(serverPagedAlertEvents.items ?? []).some((item) => item.id === a.id),
    ).length;

    return {
      ...serverPagedAlertEvents,
      items: [...receivedItems, ...baseItems].slice(0, pageSizeValue),
      totalCount: Math.max(
        0,
        Number(serverPagedAlertEvents.totalCount ?? 0) - removedVisibleItems + addedReceivedItems,
      ),
    };
  }, [serverPagedAlertEvents, receivedAlertEventIds, liveAlertEvents, query, tags, alertFeedConnected]);

  return { error, refetch, isFetching, pagedAlertEvents, isLoading };
};
