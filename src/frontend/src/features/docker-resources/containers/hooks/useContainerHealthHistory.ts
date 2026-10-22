import { useCallback } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { useRealtimeGroup } from '@/hooks/useRealtimeGroup';
import { useRead } from '@/lib/hooks';
import type { RealtimeConnection } from '@/lib/realtime-connection';

export function useContainerHealthHistory(resourceId: string, enabled = true) {
  const queryClient = useQueryClient();
  const query = useRead(
    'getContainerHealthHistory',
    { id: resourceId },
    {
      enabled: !!resourceId && enabled,
      meta: { suppressErrorToast: true },
    },
  );
  const receive = useCallback(
    async (notification: { resourceId: string }) => {
      if (notification.resourceId !== resourceId || !enabled) return;
      const filter = {
        queryKey: ['getContainerHealthHistory', { id: resourceId }],
        exact: true,
        type: 'active' as const,
      };
      // An in-flight response can predate this notification. Let it finish, then
      // fetch again. Concurrent notifications share both requests instead of
      // repeatedly cancelling them. Inactive/disabled queries never refetch.
      if (queryClient.isFetching(filter)) {
        await queryClient.refetchQueries(filter, { cancelRefetch: false });
      }
      await queryClient.invalidateQueries(filter, { cancelRefetch: false });
    },
    [queryClient, resourceId, enabled],
  );
  const setupEventListeners = useCallback(
    (connection: RealtimeConnection) => {
      connection.on('ContainerHealthHistoryChanged', receive);
    },
    [receive],
  );
  const removeEventListeners = useCallback(
    (connection: RealtimeConnection) => {
      connection.off('ContainerHealthHistoryChanged', receive);
    },
    [receive],
  );
  const { isConnected } = useRealtimeGroup({
    groupName: `container-health:${resourceId}`,
    setupEventListeners,
    removeEventListeners,
    skip: !resourceId || !enabled,
  });
  return {
    entries: query.data?.data,
    isConnected,
    isLoading: query.isLoading,
    isError: query.isError,
    refetch: query.refetch,
  };
}
