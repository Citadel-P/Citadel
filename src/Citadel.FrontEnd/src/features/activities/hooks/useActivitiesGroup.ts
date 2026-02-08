import { useEffect, useState } from 'react';
import { PagedResultViewOfActivityView } from '@/api/generated/api.types';
import { useRead } from '@/lib/hooks';
import { useActivityQuery } from '@/lib/atoms';

export const useActivitiesGroup = () => {
  const [query] = useActivityQuery();

  const { data, isLoading } = useRead('listActivities', {
    query: {
      Page: query.page,
      PageSize: query.pageSize,
      ResourceType: query.resourceType === 'All' ? undefined : query.resourceType,
      EventType: query.eventType === 'All' ? undefined : query.eventType,
      ResourceId: query.resourceId,
    },
  });

  const [pagedActivities, setPagedActivities] = useState<PagedResultViewOfActivityView | undefined>();

  useEffect(() => {
    if (!data) return;
    const nextPaged = data.data.pagedResult;
    setPagedActivities(nextPaged);
  }, [data]);

  //const handleActivityInfoUpdated = useCallback((activity: ActivityView) => {}, []);

  // const setupEventListeners = useCallback(
  //   (hubConnection: HubConnection) => {
  //     hubConnection.on('ActivityInfoUpdated', handleActivityInfoUpdated);
  //   },
  //   [handleActivityInfoUpdated],
  // );

  // const removeEventListeners = useCallback(
  //   (hubConnection: HubConnection) => {
  //     hubConnection.off('ActivityInfoUpdated', handleActivityInfoUpdated);
  //   },
  //   [handleActivityInfoUpdated],
  // );

  //   useSignalRGroup({
  //     groupName: 'activities',
  //     setupEventListeners,
  //     removeEventListeners,
  //   });

  return { pagedActivities, isLoading };
};
