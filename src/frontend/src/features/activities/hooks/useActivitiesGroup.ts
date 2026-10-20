import { useCallback, useMemo, useState } from 'react';
import {
  ActivityResourceType,
  ActivityEventType,
  ActivityStatus,
  ActivityView,
  PagedActivityView,
} from '@/api/generated/api.types';
import { useRead } from '@/lib/hooks';
import { useActivityQuery } from '@/lib/atoms';
import { useRealtimeGroup } from '@/hooks/useRealtimeGroup';
import { RealtimeConnection } from '@/lib/realtime-connection';

export const useActivitiesGroup = (
  resourceId?: string | undefined,
  resourceType?: `${ActivityResourceType}` | undefined,
  pageSize?: number | undefined,
) => {
  const [query] = useActivityQuery();
  const eventType =
    query.eventType === 'BackupRunFailed'
      ? ActivityEventType.BackupRunCompleted
      : query.eventType === 'All'
        ? undefined
        : query.eventType;
  const status = query.eventType === 'BackupRunFailed' ? ActivityStatus.Failure : undefined;
  const selectedResourceType = resourceType ?? (query.resourceType === 'All' ? undefined : query.resourceType);
  const selectedResourceId = resourceId ?? query.resourceId;

  const { data, isLoading, error, refetch, isFetching } = useRead('listActivities', {
    query: {
      Page: query.page,
      PageSize: pageSize ?? query.pageSize,
      ResourceType: Object.values(ActivityResourceType).find((type) => type === selectedResourceType),
      EventType: eventType,
      Status: status,
      ResourceId: selectedResourceId,
    },
  });

  const [liveActivities, setLiveActivities] = useState<ActivityView[]>([]);

  const pagedActivities = useMemo<PagedActivityView | undefined>(() => {
    const base = data?.data?.pagedResult;
    if (!base) return undefined;

    const matchingLiveActivities = liveActivities.filter(
      (activity) =>
        (!eventType || activity.eventType === eventType) &&
        (!status || activity.status === status) &&
        (!selectedResourceType || activity.resourceType === selectedResourceType) &&
        (!selectedResourceId || activity.resourceId === selectedResourceId),
    );
    if (matchingLiveActivities.length === 0) return base;

    const pageSize = Number(base.pageSize) || base.items?.length || 1;
    const baseItems = base.items || [];
    const liveIds = new Set(baseItems.map((item) => item.id));

    const mergedItems = [...matchingLiveActivities.filter((act) => !liveIds.has(act.id)), ...baseItems].slice(
      0,
      pageSize,
    );

    return {
      ...base,
      items: mergedItems,
      totalCount: Number(base.totalCount) + matchingLiveActivities.filter((act) => !liveIds.has(act.id)).length,
    };
  }, [data, liveActivities, eventType, status, selectedResourceType, selectedResourceId]);

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
