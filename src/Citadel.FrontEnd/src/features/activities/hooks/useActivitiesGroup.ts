import { useCallback, useEffect, useState } from 'react';
import { ActivityView, PagedResultViewOfActivityView } from '@/api/generated/api.types';
import { useRead } from '@/lib/hooks';
import { useActivityQuery } from '@/lib/atoms';
import { ResourceType } from '@/api/types';
import { useSignalRGroup } from '@/hooks/useSignalRGroup';
import { HubConnection } from '@microsoft/signalr';

export const useActivitiesGroup = (
  resourceId?: string | undefined,
  resourceType?: ResourceType | undefined,
  pageSize?: number | undefined,
) => {
  const [query] = useActivityQuery();

  const { data, isLoading } = useRead('listActivities', {
    query: {
      Page: query.page,
      PageSize: pageSize ?? query.pageSize,
      ResourceType: (resourceType ?? query.resourceType === 'All') ? undefined : query.resourceType,
      EventType: query.eventType === 'All' ? undefined : query.eventType,
      ResourceId: resourceId ?? query.resourceId,
    },
  });

  const [pagedActivities, setPagedActivities] = useState<PagedResultViewOfActivityView | undefined>();

  useEffect(() => {
    if (!data) return;
    const nextPaged = data.data.pagedResult;
    setPagedActivities(nextPaged);
  }, [data]);

  const handleActivityEventReceived = useCallback((activity: ActivityView) => {
    setPagedActivities((prev) => {
      if (!prev) return prev;

      const alreadyExists = prev.items?.some((item) => item.id === activity.id);
      if (alreadyExists) return prev;

      const pageSize = Number(prev.pageSize ?? 0) || prev.items.length || 1;
      const nextItems = [activity, ...prev.items].slice(0, pageSize);
      const nextTotalCount = Number(prev.totalCount ?? 0) + 1;

      return {
        ...prev,
        items: nextItems,
        totalCount: nextTotalCount,
      };
    });
  }, []);

  const setupEventListeners = useCallback(
    (hubConnection: HubConnection) => {
      hubConnection.on('ActivityEventReceived', handleActivityEventReceived);
    },
    [handleActivityEventReceived],
  );

  const removeEventListeners = useCallback(
    (hubConnection: HubConnection) => {
      hubConnection.off('ActivityEventReceived', handleActivityEventReceived);
    },
    [handleActivityEventReceived],
  );

  useSignalRGroup({
    groupName: `activity:${resourceId}`,
    setupEventListeners,
    removeEventListeners,
    skip: !resourceId,
  });

  return { pagedActivities, isLoading };
};
