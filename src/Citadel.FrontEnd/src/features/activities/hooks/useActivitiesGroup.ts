import { useEffect, useState, useCallback } from 'react';
import { HubConnection } from '@microsoft/signalr';
import { ActivityView, PagedResultViewOfActivityView } from '@/api/generated/api.types';
import { useSignalRGroup } from '@/hooks/useSignalRGroup';
import { useRead } from '@/lib/hooks';

export const useActivitiesGroup = () => {
  const { data, isLoading } = useRead('listActivities');
  const [pagedActivities, setPagedActivities] = useState<PagedResultViewOfActivityView | undefined>();

  useEffect(() => {
    if (!data) return;
    setPagedActivities(data.data.pagedResult);
  }, [data]);

  const handleActivityInfoUpdated = useCallback((activity: ActivityView) => {}, []);

  const setupEventListeners = useCallback(
    (hubConnection: HubConnection) => {
      hubConnection.on('ActivityInfoUpdated', handleActivityInfoUpdated);
    },
    [handleActivityInfoUpdated],
  );

  const removeEventListeners = useCallback(
    (hubConnection: HubConnection) => {
      hubConnection.off('ActivityInfoUpdated', handleActivityInfoUpdated);
    },
    [handleActivityInfoUpdated],
  );

  //   useSignalRGroup({
  //     groupName: 'activities',
  //     setupEventListeners,
  //     removeEventListeners,
  //   });

  return { pagedActivities, isLoading };
};
