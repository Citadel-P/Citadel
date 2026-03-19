import { useEffect, useMemo, useState } from 'react';
import { AlertEventStatus, AlertEventView, PagedResultViewOfAlertEventView } from '@/api/generated/api.types';
import { useRead } from '@/lib/hooks';
import { useAlertEventQuery } from '@/lib/atoms';
import { ResourceType } from '@/api/types';
import { useAppContext } from '@/lib/context/app-context';

export const useAlertEventsList = (
  resourceId?: string | undefined,
  resourceType?: ResourceType | undefined,
  pageSize?: number | undefined,
) => {
  const [query] = useAlertEventQuery();
  const { liveAlertEvents, receivedAlertEventIds } = useAppContext();

  const { data, isLoading } = useRead('listAlertEvents', {
    query: {
      Page: query.page,
      PageSize: pageSize ?? query.pageSize,
      ResourceType: (resourceType ?? query.resourceType === 'All') ? undefined : query.resourceType,
      AlertType: query.alertType === 'All' ? undefined : query.alertType,
      ResourceId: resourceId ?? query.resourceId,
      UnresolvedOnly: query.unresolvedOnly,
    },
  });

  const [serverPagedAlertEvents, setServerPagedAlertEvents] = useState<PagedResultViewOfAlertEventView | undefined>();

  useEffect(() => {
    if (!data) return;
    setServerPagedAlertEvents(data.data.pagedResult);
  }, [data]);

  const pagedAlertEvents = useMemo(() => {
    if (!serverPagedAlertEvents) return undefined;

    const pageSizeValue = Number(serverPagedAlertEvents.pageSize ?? 0) || serverPagedAlertEvents.items.length || 1;
    const seen = new Set<string>();

    const matchesFilters = (alertEvent: AlertEventView) => {
      const targetResourceType = resourceType ?? query.resourceType;
      const targetResourceId = resourceId ?? query.resourceId;

      if (targetResourceType !== 'All' && alertEvent.resourceType !== targetResourceType) return false;
      if (query.alertType !== 'All' && alertEvent.type !== query.alertType) return false;
      if (targetResourceId && alertEvent.resourceId !== targetResourceId) return false;
      if (query.unresolvedOnly && alertEvent.status === AlertEventStatus.Resolved) return false;

      return true;
    };

    const receivedItems = receivedAlertEventIds
      .map((id) => liveAlertEvents[id])
      .filter((alertEvent): alertEvent is AlertEventView => !!alertEvent)
      .filter(matchesFilters)
      .filter((alertEvent) => {
        if (seen.has(alertEvent.id)) return false;
        seen.add(alertEvent.id);
        return true;
      });

    const baseItems = (serverPagedAlertEvents.items ?? [])
      .map((item) => liveAlertEvents[item.id] ?? item)
      .filter(matchesFilters)
      .filter((alertEvent) => {
        if (seen.has(alertEvent.id)) return false;
        seen.add(alertEvent.id);
        return true;
      });

    const removedVisibleItems = (serverPagedAlertEvents.items ?? []).filter((item) => {
      const alertEvent = liveAlertEvents[item.id] ?? item;
      return !matchesFilters(alertEvent);
    }).length;

    const addedReceivedItems = receivedItems.filter(
      (alertEvent) => !(serverPagedAlertEvents.items ?? []).some((item) => item.id === alertEvent.id),
    ).length;

    return {
      ...serverPagedAlertEvents,
      items: [...receivedItems, ...baseItems].slice(0, pageSizeValue),
      totalCount: Math.max(
        0,
        Number(serverPagedAlertEvents.totalCount ?? 0) - removedVisibleItems + addedReceivedItems,
      ),
    };
  }, [serverPagedAlertEvents, receivedAlertEventIds, liveAlertEvents, resourceType, resourceId, query]);

  return { pagedAlertEvents, isLoading };
};
