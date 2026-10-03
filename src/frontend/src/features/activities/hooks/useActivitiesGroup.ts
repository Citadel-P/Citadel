import { useCallback, useMemo, useState } from 'react';
import { ActivityView, PagedActivityView } from '@/api/generated/api.types';
import { useRead } from '@/lib/hooks';
import { useActivityQuery } from '@/lib/atoms';
import { ResourceType } from '@/api/types';
import { useRealtimeGroup } from '@/hooks/useRealtimeGroup';
import { RealtimeConnection } from '@/lib/realtime-connection';

export const useActivitiesGroup = (
  resourceId?: string | undefined,
  resourceType?: ResourceType | undefined,
  pageSize?: number | undefined,
) => {
  const [query] = useActivityQuery();

  const { data, isLoading, error, refetch, isFetching } = useRead('listActivities', {
    query: {
      Page: query.page,
      PageSize: pageSize ?? query.pageSize,
      ResourceType: resourceType ?? (query.resourceType === 'All' ? undefined : query.resourceType),
      EventType: query.eventType === 'All' ? undefined : query.eventType,
      ResourceId: resourceId ?? query.resourceId,
    },
  });

  const [liveActivities, setLiveActivities] = useState<ActivityView[]>([]);

  const pagedActivities = useMemo<PagedActivityView | undefined>(() => {
    const base = data?.data?.pagedResult;
    if (!base) return undefined;

    if (liveActivities.length === 0) return base;

    const pageSize = Number(base.pageSize) || base.items?.length || 1;
    const baseItems = base.items || [];
    const liveIds = new Set(baseItems.map((item) => item.id));

    const mergedItems = [...liveActivities.filter((act) => !liveIds.has(act.id)), ...baseItems].slice(0, pageSize);

    return {
      ...base,
      items: mergedItems,
      totalCount: Number(base.totalCount) + liveActivities.filter((act) => !liveIds.has(act.id)).length,
    };
  }, [data, liveActivities]);

  const handleActivityEventReceived = useCallback((activity: ActivityView) => {
    setLiveActivities((prev) => {
      if (prev.some((item) => item.id === activity.id)) return prev;
      return [activity, ...prev];
    });
  }, []);

  const setupEventListeners = useCallback(
    (hubConnection: RealtimeConnection) => {
      hubConnection.on('ActivityEventReceived', handleActivityEventReceived);
    },
    [handleActivityEventReceived],
  );

  const removeEventListeners = useCallback(
    (hubConnection: RealtimeConnection) => {
      hubConnection.off('ActivityEventReceived', handleActivityEventReceived);
    },
    [handleActivityEventReceived],
  );

  useRealtimeGroup({
    groupName: resourceId && resourceType ? `activity:${resourceType}:${resourceId}` : undefined,
    setupEventListeners,
    removeEventListeners,
    skip: !resourceId || !resourceType,
  });

  return { error, refetch, isFetching, pagedActivities, isLoading };
};
